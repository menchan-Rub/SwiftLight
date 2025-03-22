//! # 型制約ソルバー
//! 
//! SwiftLight言語の型システムにおける型制約を表現し、解決するためのモジュールです。
//! 型変数の統合、サブタイプ関係の検証、依存型の制約解決などを担当します。

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::sync::Arc;

use crate::frontend::error::{CompilerError, ErrorKind, Result, SourceLocation};
use crate::typesystem::types::{Type, TypeVariable, TypeKind};
use crate::typesystem::traits::{Trait, TraitBound};

/// 型制約の種類
#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintKind {
    /// 等価制約: 2つの型が等しいことを要求
    Equality(Type, Type),
    /// サブタイプ制約: 1つ目の型が2つ目の型のサブタイプであることを要求
    Subtype(Type, Type),
    /// トレイト境界制約: 型が特定のトレイトを実装していることを要求
    TraitBound(Type, Vec<TraitBound>),
    /// 条件付き制約: 条件が満たされた場合に適用される制約
    Conditional(Box<Constraint>, Box<Constraint>),
    /// 論理和制約: 少なくとも1つの制約が満たされる必要がある
    Disjunction(Vec<Constraint>),
    /// 論理積制約: すべての制約が満たされる必要がある
    Conjunction(Vec<Constraint>),
}

/// 型制約
#[derive(Debug, Clone, PartialEq)]
pub struct Constraint {
    /// 制約の種類
    pub kind: ConstraintKind,
    /// 制約の発生位置
    pub location: Option<SourceLocation>,
    /// 制約に関連するメッセージ（エラー時に使用）
    pub message: Option<String>,
}

impl Constraint {
    /// 新しい等価制約を作成
    pub fn equality(t1: Type, t2: Type) -> Self {
        Self {
            kind: ConstraintKind::Equality(t1, t2),
            location: None,
            message: None,
        }
    }

    /// 新しいサブタイプ制約を作成
    pub fn subtype(sub: Type, sup: Type) -> Self {
        Self {
            kind: ConstraintKind::Subtype(sub, sup),
            location: None,
            message: None,
        }
    }

    /// 新しいトレイト境界制約を作成
    pub fn trait_bound(ty: Type, bounds: Vec<TraitBound>) -> Self {
        Self {
            kind: ConstraintKind::TraitBound(ty, bounds),
            location: None,
            message: None,
        }
    }

    /// 位置情報を設定
    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = Some(location);
        self
    }

    /// メッセージを設定
    pub fn with_message(mut self, message: &str) -> Self {
        self.message = Some(message.to_string());
        self
    }
}

/// 型制約ソルバー
#[derive(Debug, Default)]
pub struct ConstraintSolver {
    /// 制約リスト
    constraints: Vec<Constraint>,
    /// 型変数の置換マップ
    substitutions: HashMap<TypeVariable, Type>,
    /// 処理済みの制約セット（ループ防止）
    processed: HashSet<Constraint>,
    /// サブタイプ関係キャッシュ
    subtype_cache: HashMap<(Type, Type), bool>,
}

impl ConstraintSolver {
    /// 新しい型制約ソルバーを作成
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
            substitutions: HashMap::new(),
            processed: HashSet::new(),
            subtype_cache: HashMap::new(),
        }
    }

    /// 制約を追加
    pub fn add_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }

    /// 等価制約を追加
    pub fn add_equality_constraint(&mut self, t1: Type, t2: Type) -> Result<()> {
        self.add_constraint(Constraint::equality(t1, t2));
        Ok(())
    }

    /// サブタイプ制約を追加
    pub fn add_subtype_constraint(&mut self, sub: Type, sup: Type) -> Result<()> {
        self.add_constraint(Constraint::subtype(sub, sup));
        Ok(())
    }

    /// トレイト境界制約を追加
    pub fn add_trait_bound_constraint(&mut self, ty: Type, bounds: Vec<TraitBound>) -> Result<()> {
        self.add_constraint(Constraint::trait_bound(ty, bounds));
        Ok(())
    }

    /// 制約を解決
    pub fn solve(&mut self) -> Result<()> {
        // 制約解決のワークリスト
        let mut worklist = VecDeque::new();
        worklist.extend(self.constraints.clone());
        
        // 既に処理した制約を記録
        self.processed.clear();
        
        // ワークリストが空になるまで処理
        while let Some(constraint) = worklist.pop_front() {
            // 既に処理済みならスキップ
            if self.processed.contains(&constraint) {
                continue;
            }
            
            // 制約を解決
            match self.solve_constraint(&constraint)? {
                SolveResult::Solved => {
                    // 制約が解決された
                    self.processed.insert(constraint);
                },
                SolveResult::Deferred => {
                    // 制約を後回し
                    worklist.push_back(constraint);
                },
                SolveResult::NewConstraints(new_constraints) => {
                    // 新しい制約が生成された
                    self.processed.insert(constraint);
                    worklist.extend(new_constraints);
                }
            }
            
            // 無限ループ防止（一定回数で強制終了）
            if self.processed.len() > 10000 {
                return Err(CompilerError::new(
                    ErrorKind::TypeSystem,
                    "制約解決の上限を超過しました。循環参照または過度に複雑な型制約が存在する可能性があります。",
                    constraint.location.unwrap_or_default()
                ));
            }
        }
        
        Ok(())
    }

    /// 制約解決の結果
    enum SolveResult {
        /// 制約が解決された
        Solved,
        /// 制約を後回しにする
        Deferred,
        /// 新しい制約が生成された
        NewConstraints(Vec<Constraint>),
    }

    /// 単一の制約を解決
    fn solve_constraint(&mut self, constraint: &Constraint) -> Result<SolveResult> {
        match &constraint.kind {
            ConstraintKind::Equality(t1, t2) => {
                self.solve_equality(t1, t2, constraint.location)
            },
            ConstraintKind::Subtype(sub, sup) => {
                self.solve_subtype(sub, sup, constraint.location)
            },
            ConstraintKind::TraitBound(ty, bounds) => {
                self.solve_trait_bound(ty, bounds, constraint.location)
            },
            ConstraintKind::Conditional(cond, then) => {
                // まずは条件を解決
                match self.solve_constraint(cond)? {
                    SolveResult::Solved => {
                        // 条件が解決されたら、thenを解決
                        self.solve_constraint(then)
                    },
                    other => Ok(other),
                }
            },
            ConstraintKind::Disjunction(constraints) => {
                // いずれかの制約が解決できればOK
                let mut deferred = true;
                
                for c in constraints {
                    match self.solve_constraint(c)? {
                        SolveResult::Solved => {
                            return Ok(SolveResult::Solved);
                        },
                        SolveResult::NewConstraints(_) => {
                            deferred = false;
                        },
                        SolveResult::Deferred => {}
                    }
                }
                
                if deferred {
                    Ok(SolveResult::Deferred)
                } else {
                    Ok(SolveResult::Solved)
                }
            },
            ConstraintKind::Conjunction(constraints) => {
                // すべての制約が解決できる必要がある
                let mut all_solved = true;
                let mut deferred = false;
                let mut new_constraints = Vec::new();
                
                for c in constraints {
                    match self.solve_constraint(c)? {
                        SolveResult::Solved => {},
                        SolveResult::Deferred => {
                            all_solved = false;
                            deferred = true;
                        },
                        SolveResult::NewConstraints(new_c) => {
                            all_solved = false;
                            new_constraints.extend(new_c);
                        }
                    }
                }
                
                if all_solved {
                    Ok(SolveResult::Solved)
                } else if !new_constraints.is_empty() {
                    Ok(SolveResult::NewConstraints(new_constraints))
                } else if deferred {
                    Ok(SolveResult::Deferred)
                } else {
                    Ok(SolveResult::Solved)
                }
            }
        }
    }

    /// 等価制約を解決
    fn solve_equality(&mut self, t1: &Type, t2: &Type, location: Option<SourceLocation>) -> Result<SolveResult> {
        // 両方の型を解決
        let t1 = self.resolve_type(t1);
        let t2 = self.resolve_type(t2);
        
        // 同じ型なら何もしない
        if t1 == t2 {
            return Ok(SolveResult::Solved);
        }
        
        match (&t1.kind, &t2.kind) {
            // 型変数と他の型
            (TypeKind::Variable(var), _) => {
                // 型変数を他の型に置換
                self.substitutions.insert(var.clone(), t2.clone());
                Ok(SolveResult::Solved)
            },
            (_, TypeKind::Variable(var)) => {
                // 型変数を他の型に置換
                self.substitutions.insert(var.clone(), t1.clone());
                Ok(SolveResult::Solved)
            },
            
            // ジェネリック型（例: Vec<T>とVec<U>）
            (TypeKind::Generic(base1, args1), TypeKind::Generic(base2, args2)) => {
                if base1 != base2 {
                    // 基本型が異なる
                    return Err(CompilerError::new(
                        ErrorKind::TypeMismatch,
                        &format!("型の不一致: {} != {}", t1, t2),
                        location.unwrap_or_default()
                    ));
                }
                
                if args1.len() != args2.len() {
                    // 型引数の数が異なる
                    return Err(CompilerError::new(
                        ErrorKind::TypeMismatch,
                        &format!("ジェネリック型の引数数不一致: {}個 != {}個", args1.len(), args2.len()),
                        location.unwrap_or_default()
                    ));
                }
                
                // 型引数ごとに等価制約を追加
                let mut new_constraints = Vec::new();
                for (a1, a2) in args1.iter().zip(args2.iter()) {
                    new_constraints.push(Constraint::equality(a1.clone(), a2.clone())
                        .with_location(location.unwrap_or_default()));
                }
                
                Ok(SolveResult::NewConstraints(new_constraints))
            },
            
            // 関数型
            (TypeKind::Function(params1, ret1), TypeKind::Function(params2, ret2)) => {
                if params1.len() != params2.len() {
                    // パラメータ数が異なる
                    return Err(CompilerError::new(
                        ErrorKind::TypeMismatch,
                        &format!("関数型のパラメータ数不一致: {}個 != {}個", params1.len(), params2.len()),
                        location.unwrap_or_default()
                    ));
                }
                
                // パラメータと戻り値の型に対して等価制約を追加
                let mut new_constraints = Vec::new();
                
                // 戻り値の型
                new_constraints.push(Constraint::equality(ret1.as_ref().clone(), ret2.as_ref().clone())
                    .with_location(location.unwrap_or_default()));
                
                // パラメータの型
                for (p1, p2) in params1.iter().zip(params2.iter()) {
                    new_constraints.push(Constraint::equality(p1.clone(), p2.clone())
                        .with_location(location.unwrap_or_default()));
                }
                
                Ok(SolveResult::NewConstraints(new_constraints))
            },
            
            // その他の型の組み合わせ
            _ => {
                Err(CompilerError::new(
                    ErrorKind::TypeMismatch,
                    &format!("型の不一致: {} != {}", t1, t2),
                    location.unwrap_or_default()
                ))
            }
        }
    }

    /// サブタイプ制約を解決
    fn solve_subtype(&mut self, sub: &Type, sup: &Type, location: Option<SourceLocation>) -> Result<SolveResult> {
        // 両方の型を解決
        let sub = self.resolve_type(sub);
        let sup = self.resolve_type(sup);
        
        // キャッシュをチェック
        let cache_key = (sub.clone(), sup.clone());
        if let Some(&result) = self.subtype_cache.get(&cache_key) {
            return if result {
                Ok(SolveResult::Solved)
            } else {
                Err(CompilerError::new(
                    ErrorKind::TypeMismatch,
                    &format!("{} は {} のサブタイプではありません", sub, sup),
                    location.unwrap_or_default()
                ))
            };
        }
        
        // 同じ型なら何もしない
        if sub == sup {
            self.subtype_cache.insert(cache_key, true);
            return Ok(SolveResult::Solved);
        }
        
        match (&sub.kind, &sup.kind) {
            // 型変数と他の型
            (TypeKind::Variable(var), _) => {
                // 型変数を他の型に置換
                self.substitutions.insert(var.clone(), sup.clone());
                self.subtype_cache.insert(cache_key, true);
                Ok(SolveResult::Solved)
            },
            (_, TypeKind::Variable(var)) => {
                // 型変数を他の型に置換
                self.substitutions.insert(var.clone(), sub.clone());
                self.subtype_cache.insert(cache_key, true);
                Ok(SolveResult::Solved)
            },
            
            // ジェネリック型（例: Vec<T>とVec<U>）
            (TypeKind::Generic(base1, args1), TypeKind::Generic(base2, args2)) => {
                if base1 != base2 {
                    // 基本型が異なる
                    self.subtype_cache.insert(cache_key, false);
                    return Err(CompilerError::new(
                        ErrorKind::TypeMismatch,
                        &format!("型の不一致: {} != {}", sub, sup),
                        location.unwrap_or_default()
                    ));
                }
                
                if args1.len() != args2.len() {
                    // 型引数の数が異なる
                    self.subtype_cache.insert(cache_key, false);
                    return Err(CompilerError::new(
                        ErrorKind::TypeMismatch,
                        &format!("ジェネリック型の引数数不一致: {}個 != {}個", args1.len(), args2.len()),
                        location.unwrap_or_default()
                    ));
                }
                
                // 型引数ごとにサブタイプ制約を追加
                let mut new_constraints = Vec::new();
                for (a1, a2) in args1.iter().zip(args2.iter()) {
                    // ジェネリックパラメータの変性に応じて制約を追加
                    // 共変: サブタイプ関係を保持
                    // 反変: サブタイプ関係が逆転
                    // 不変: 型が一致する必要がある
                    // ここでは簡略化のため、すべてを不変として扱う
                    new_constraints.push(Constraint::equality(a1.clone(), a2.clone())
                        .with_location(location.unwrap_or_default()));
                }
                
                self.subtype_cache.insert(cache_key, true);
                Ok(SolveResult::NewConstraints(new_constraints))
            },
            
            // 関数型
            (TypeKind::Function(params1, ret1), TypeKind::Function(params2, ret2)) => {
                if params1.len() != params2.len() {
                    // パラメータ数が異なる
                    self.subtype_cache.insert(cache_key, false);
                    return Err(CompilerError::new(
                        ErrorKind::TypeMismatch,
                        &format!("関数型のパラメータ数不一致: {}個 != {}個", params1.len(), params2.len()),
                        location.unwrap_or_default()
                    ));
                }
                
                // 関数型のサブタイプ関係:
                // 1. 引数の型: 逆順のサブタイプ関係（反変）
                // 2. 戻り値の型: 同順のサブタイプ関係（共変）
                let mut new_constraints = Vec::new();
                
                // 戻り値の型（共変）
                new_constraints.push(Constraint::subtype(ret1.as_ref().clone(), ret2.as_ref().clone())
                    .with_location(location.unwrap_or_default()));
                
                // パラメータの型（反変）
                for (p1, p2) in params1.iter().zip(params2.iter()) {
                    new_constraints.push(Constraint::subtype(p2.clone(), p1.clone())
                        .with_location(location.unwrap_or_default()));
                }
                
                self.subtype_cache.insert(cache_key, true);
                Ok(SolveResult::NewConstraints(new_constraints))
            },
            
            // その他の型の組み合わせ
            _ => {
                // ここで本来は型の階層関係をチェックする（例: IntはNumのサブタイプ）
                // 簡略化のため、ここでは型の等価性だけをチェック
                self.subtype_cache.insert(cache_key, false);
                Err(CompilerError::new(
                    ErrorKind::TypeMismatch,
                    &format!("{} は {} のサブタイプではありません", sub, sup),
                    location.unwrap_or_default()
                ))
            }
        }
    }

    /// トレイト境界制約を解決
    fn solve_trait_bound(&mut self, ty: &Type, bounds: &Vec<TraitBound>, location: Option<SourceLocation>) -> Result<SolveResult> {
        // 型を解決
        let ty = self.resolve_type(ty);
        
        // 型変数の場合は後回し
        if let TypeKind::Variable(_) = ty.kind {
            return Ok(SolveResult::Deferred);
        }
        
        // TODO: トレイト境界の検証
        // ここでは簡略化のため、常に成功するとする
        Ok(SolveResult::Solved)
    }

    /// 型変数を解決（現在の置換マップに基づく）
    pub fn resolve_type_var(&self, var: &TypeVariable) -> Option<Type> {
        self.substitutions.get(var).cloned()
    }

    /// 型を解決（型変数を置換）
    pub fn resolve_type(&self, ty: &Type) -> Type {
        match &ty.kind {
            TypeKind::Variable(var) => {
                // 型変数があれば置換
                if let Some(subst) = self.substitutions.get(var) {
                    // 置換後の型も再帰的に解決
                    self.resolve_type(subst)
                } else {
                    // 未解決の型変数はそのまま
                    ty.clone()
                }
            },
            TypeKind::Generic(base, args) => {
                // ジェネリック型の引数を再帰的に解決
                let resolved_args = args.iter()
                    .map(|arg| self.resolve_type(arg))
                    .collect();
                
                Type::generic(base.clone(), resolved_args)
            },
            TypeKind::Function(params, ret) => {
                // 関数型のパラメータと戻り値を再帰的に解決
                let resolved_params = params.iter()
                    .map(|param| self.resolve_type(param))
                    .collect();
                
                let resolved_ret = Box::new(self.resolve_type(ret));
                
                Type::function(resolved_params, resolved_ret)
            },
            // 他の型はそのまま
            _ => ty.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_equality() {
        let mut solver = ConstraintSolver::new();
        
        // 型変数を作成
        let var_a = Type::variable("a");
        let int_type = Type::primitive("Int");
        
        // 制約を追加: a = Int
        solver.add_equality_constraint(var_a.clone(), int_type.clone()).unwrap();
        
        // 制約を解決
        solver.solve().unwrap();
        
        // 型変数aがIntに解決されることを確認
        if let TypeKind::Variable(var) = &var_a.kind {
            let resolved = solver.resolve_type_var(var).unwrap();
            assert_eq!(resolved, int_type);
        }
    }
    
    #[test]
    fn test_transitive_equality() {
        let mut solver = ConstraintSolver::new();
        
        // 型変数を作成
        let var_a = Type::variable("a");
        let var_b = Type::variable("b");
        let int_type = Type::primitive("Int");
        
        // 制約を追加: a = b, b = Int
        solver.add_equality_constraint(var_a.clone(), var_b.clone()).unwrap();
        solver.add_equality_constraint(var_b.clone(), int_type.clone()).unwrap();
        
        // 制約を解決
        solver.solve().unwrap();
        
        // 型変数aとbがIntに解決されることを確認
        if let TypeKind::Variable(var) = &var_a.kind {
            let resolved = solver.resolve_type_var(var).unwrap();
            assert_eq!(resolved, int_type);
        }
        
        if let TypeKind::Variable(var) = &var_b.kind {
            let resolved = solver.resolve_type_var(var).unwrap();
            assert_eq!(resolved, int_type);
        }
    }
    
    #[test]
    fn test_generic_types() {
        let mut solver = ConstraintSolver::new();
        
        // 型変数を作成
        let var_a = Type::variable("a");
        
        // ジェネリック型を作成
        let vec_a = Type::generic("Vec".to_string(), vec![var_a.clone()]);
        let vec_int = Type::generic("Vec".to_string(), vec![Type::primitive("Int")]);
        
        // 制約を追加: Vec<a> = Vec<Int>
        solver.add_equality_constraint(vec_a, vec_int).unwrap();
        
        // 制約を解決
        solver.solve().unwrap();
        
        // 型変数aがIntに解決されることを確認
        if let TypeKind::Variable(var) = &var_a.kind {
            let resolved = solver.resolve_type_var(var).unwrap();
            assert_eq!(resolved, Type::primitive("Int"));
        }
    }
    
    #[test]
    fn test_function_types() {
        let mut solver = ConstraintSolver::new();
        
        // 型変数を作成
        let var_a = Type::variable("a");
        let var_b = Type::variable("b");
        
        // 関数型を作成
        let func1 = Type::function(
            vec![var_a.clone()],
            Box::new(var_b.clone())
        );
        
        let func2 = Type::function(
            vec![Type::primitive("Int")],
            Box::new(Type::primitive("String"))
        );
        
        // 制約を追加: (a) -> b = (Int) -> String
        solver.add_equality_constraint(func1, func2).unwrap();
        
        // 制約を解決
        solver.solve().unwrap();
        
        // 型変数aがIntに、bがStringに解決されることを確認
        if let TypeKind::Variable(var) = &var_a.kind {
            let resolved = solver.resolve_type_var(var).unwrap();
            assert_eq!(resolved, Type::primitive("Int"));
        }
        
        if let TypeKind::Variable(var) = &var_b.kind {
            let resolved = solver.resolve_type_var(var).unwrap();
            assert_eq!(resolved, Type::primitive("String"));
        }
    }
} 
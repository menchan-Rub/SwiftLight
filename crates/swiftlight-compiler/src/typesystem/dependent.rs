// SwiftLight Type System - Dependent Types
// 依存型システムの実装

//! # 依存型システム
//! 
//! SwiftLight言語における依存型(Dependent Types)のサポートを実装します。
//! 依存型とは、値に依存する型のことで、これにより型レベルでより強力な制約を表現できます。
//! 例えば、長さNの配列の型や、非負の整数の型などを表現できます。

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;

use crate::frontend::error::{CompilerError, ErrorKind, Result, SourceLocation};
use crate::typesystem::{
    Type, TypeId, TypeRegistry, Symbol, Kind, 
    TypeError, RefinementPredicate, TypeManager
};
use crate::utils::StrId;

/// 型レベルの式
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeLevelExpr {
    /// 変数参照
    Var(Symbol),
    /// リテラル値
    Literal(TypeLevelLiteralValue),
    /// 二項演算
    BinaryOp {
        op: TypeLevelBinaryOp,
        left: Box<TypeLevelExpr>,
        right: Box<TypeLevelExpr>,
    },
    /// 関数適用
    Apply {
        func: Box<TypeLevelExpr>,
        args: Vec<TypeLevelExpr>,
    },
    /// 条件式
    If {
        condition: Box<TypeLevelExpr>,
        then_expr: Box<TypeLevelExpr>,
        else_expr: Box<TypeLevelExpr>,
    },
    /// 型レベル関数
    Lambda {
        param: Symbol,
        param_kind: Kind,
        body: Box<TypeLevelExpr>,
    },
    /// 型レベルLet束縛
    Let {
        name: Symbol,
        value: Box<TypeLevelExpr>,
        body: Box<TypeLevelExpr>,
    },
}

/// 型レベルのリテラル値
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeLevelLiteralValue {
    /// 整数値
    Int(i64),
    /// 真偽値
    Bool(bool),
    /// 文字列
    String(String),
    /// 型
    Type(TypeId),
    /// リスト
    List(Vec<TypeLevelLiteralValue>),
}

/// 型レベルの二項演算子
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeLevelBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

/// 型レベル式の評価結果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeLevelValue {
    Literal(TypeLevelLiteralValue),
    Function(TypeLevelFunction),
    Neutral(TypeLevelNeutral),
}

/// 型レベル関数値
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeLevelFunction {
    pub param: Symbol,
    pub param_kind: Kind,
    pub body: Box<TypeLevelExpr>,
    pub env: Arc<HashMap<Symbol, TypeLevelValue>>,
}

/// 正規形に評価できない型レベル式
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeLevelNeutral {
    Var(Symbol),
    Apply {
        func: Box<TypeLevelNeutral>,
        args: Vec<TypeLevelValue>,
    },
}

/// 依存型制約ソルバー
pub struct DependentTypeSolver {
    /// 型レベル式の環境
    environment: HashMap<Symbol, TypeLevelValue>,
    /// 評価中の式スタック（無限再帰検出用）
    evaluation_stack: Vec<TypeLevelExpr>,
}

impl DependentTypeSolver {
    /// 新しい依存型制約ソルバーを作成
    pub fn new() -> Self {
        Self {
            environment: HashMap::new(),
            evaluation_stack: Vec::new(),
        }
    }

    /// 型レベル式を評価
    pub fn evaluate(&mut self, expr: &TypeLevelExpr) -> Result<TypeLevelValue> {
        // 無限ループ検出
        if self.evaluation_stack.contains(expr) {
            return Err(CompilerError::new(
                ErrorKind::TypeSystem,
                "型レベル式の評価で無限再帰が検出されました".to_owned(),
                SourceLocation::default(),
            ));
        }

        self.evaluation_stack.push(expr.clone());
        let result = self.evaluate_impl(expr);
        self.evaluation_stack.pop();
        result
    }

    /// 型レベル式の実際の評価処理
    fn evaluate_impl(&mut self, expr: &TypeLevelExpr) -> Result<TypeLevelValue> {
        match expr {
            TypeLevelExpr::Var(name) => {
                if let Some(value) = self.environment.get(name) {
                    Ok(value.clone())
                } else {
                    Ok(TypeLevelValue::Neutral(TypeLevelNeutral::Var(*name)))
                }
            }
            TypeLevelExpr::Literal(lit) => {
                Ok(TypeLevelValue::Literal(lit.clone()))
            }
            TypeLevelExpr::BinaryOp { op, left, right } => {
                let left_val = self.evaluate(left)?;
                let right_val = self.evaluate(right)?;
                self.evaluate_binary_op(*op, &left_val, &right_val)
            }
            TypeLevelExpr::Apply { func, args } => {
                let func_val = self.evaluate(func)?;
                let mut arg_vals = Vec::new();
                for arg in args {
                    arg_vals.push(self.evaluate(arg)?);
                }
                self.apply_function(&func_val, &arg_vals)
            }
            TypeLevelExpr::If { condition, then_expr, else_expr } => {
                let cond_val = self.evaluate(condition)?;
                match cond_val {
                    TypeLevelValue::Literal(TypeLevelLiteralValue::Bool(true)) => {
                        self.evaluate(then_expr)
                    }
                    TypeLevelValue::Literal(TypeLevelLiteralValue::Bool(false)) => {
                        self.evaluate(else_expr)
                    }
                    _ => {
                        Err(CompilerError::new(
                            ErrorKind::TypeSystem,
                            "型レベルのif式の条件は真偽値でなければなりません".to_owned(),
                            SourceLocation::default(),
                        ))
                    }
                }
            }
            TypeLevelExpr::Lambda { param, param_kind, body } => {
                Ok(TypeLevelValue::Function(TypeLevelFunction {
                    param: *param,
                    param_kind: param_kind.clone(),
                    body: body.clone(),
                    env: Arc::new(self.environment.clone()),
                }))
            }
            TypeLevelExpr::Let { name, value, body } => {
                let value_val = self.evaluate(value)?;
                self.environment.insert(*name, value_val);
                let result = self.evaluate(body);
                self.environment.remove(name);
                result
            }
        }
    }

    /// 二項演算を評価
    fn evaluate_binary_op(
        &self,
        op: TypeLevelBinaryOp,
        left: &TypeLevelValue,
        right: &TypeLevelValue,
    ) -> Result<TypeLevelValue> {
        match (left, right) {
            (TypeLevelValue::Literal(TypeLevelLiteralValue::Int(l)), 
             TypeLevelValue::Literal(TypeLevelLiteralValue::Int(r))) => {
                let result = match op {
                    TypeLevelBinaryOp::Add => TypeLevelLiteralValue::Int(l + r),
                    TypeLevelBinaryOp::Sub => TypeLevelLiteralValue::Int(l - r),
                    TypeLevelBinaryOp::Mul => TypeLevelLiteralValue::Int(l * r),
                    TypeLevelBinaryOp::Div => {
                        if *r == 0 {
                            return Err(CompilerError::new(
                                ErrorKind::TypeSystem,
                                "型レベル計算でゼロ除算が発生しました".to_owned(),
                                SourceLocation::default(),
                            ));
                        }
                        TypeLevelLiteralValue::Int(l / r)
                    },
                    TypeLevelBinaryOp::Mod => {
                        if *r == 0 {
                            return Err(CompilerError::new(
                                ErrorKind::TypeSystem,
                                "型レベル計算でゼロ除算（剰余）が発生しました".to_owned(),
                                SourceLocation::default(),
                            ));
                        }
                        TypeLevelLiteralValue::Int(l % r)
                    },
                    TypeLevelBinaryOp::Eq => TypeLevelLiteralValue::Bool(l == r),
                    TypeLevelBinaryOp::Ne => TypeLevelLiteralValue::Bool(l != r),
                    TypeLevelBinaryOp::Lt => TypeLevelLiteralValue::Bool(l < r),
                    TypeLevelBinaryOp::Le => TypeLevelLiteralValue::Bool(l <= r),
                    TypeLevelBinaryOp::Gt => TypeLevelLiteralValue::Bool(l > r),
                    TypeLevelBinaryOp::Ge => TypeLevelLiteralValue::Bool(l >= r),
                    _ => {
                        return Err(CompilerError::new(
                            ErrorKind::TypeSystem,
                            &format!("整数値に対して演算子 {:?} は適用できません", op),
                            SourceLocation::default(),
                        ));
                    }
                };
                Ok(TypeLevelValue::Literal(result))
            }
            (TypeLevelValue::Literal(TypeLevelLiteralValue::Bool(l)),
             TypeLevelValue::Literal(TypeLevelLiteralValue::Bool(r))) => {
                let result = match op {
                    TypeLevelBinaryOp::And => TypeLevelLiteralValue::Bool(*l && *r),
                    TypeLevelBinaryOp::Or => TypeLevelLiteralValue::Bool(*l || *r),
                    TypeLevelBinaryOp::Eq => TypeLevelLiteralValue::Bool(l == r),
                    TypeLevelBinaryOp::Ne => TypeLevelLiteralValue::Bool(l != r),
                    _ => {
                        return Err(CompilerError::new(
                            ErrorKind::TypeSystem,
                            &format!("真偽値に対して演算子 {:?} は適用できません", op),
                            SourceLocation::default(),
                        ));
                    }
                };
                Ok(TypeLevelValue::Literal(result))
            }
            _ => {
                // 非正規形の式に対する演算は保留
                Err(CompilerError::new(
                    ErrorKind::TypeSystem,
                    "型レベル式が評価できません：非リテラル値に対する演算です".to_owned(),
                    SourceLocation::default(),
                ))
            }
        }
    }

    /// 関数適用を評価
    fn apply_function(
        &mut self,
        func: &TypeLevelValue,
        args: &[TypeLevelValue],
    ) -> Result<TypeLevelValue> {
        match func {
            TypeLevelValue::Function(f) => {
                if args.len() != 1 {
                    return Err(CompilerError::new(
                        ErrorKind::TypeSystem,
                        &format!("型レベル関数は単一の引数を取りますが、{}個の引数が与えられました", args.len()),
                        SourceLocation::default(),
                    ));
                }

                // 関数の環境を保存
                let old_env = self.environment.clone();
                
                // 関数の環境を設定
                for (name, value) in f.env.iter() {
                    self.environment.insert(*name, value.clone());
                }
                
                // 引数を束縛
                self.environment.insert(f.param, args[0].clone());
                
                // 関数本体を評価
                let result = self.evaluate(&f.body);
                
                // 環境を復元
                self.environment = old_env;
                
                result
            }
            TypeLevelValue::Neutral(n) => {
                // 中性値への適用は評価できないので、新しい中性値を作成
                let mut app_args = Vec::new();
                for arg in args {
                    app_args.push(arg.clone());
                }
                
                Ok(TypeLevelValue::Neutral(TypeLevelNeutral::Apply {
                    func: Box::new(n.clone()),
                    args: app_args,
                }))
            }
            _ => {
                Err(CompilerError::new(
                    ErrorKind::TypeSystem,
                    "型レベルの関数適用が無効です：適用対象が関数ではありません".to_owned(),
                    SourceLocation::default(),
                ))
            }
        }
    }

    /// 2つの型が等価かどうかをチェック（より詳細なバージョン）
    pub fn check_equivalence(
        &mut self,
        t1: &Type,
        t2: &Type,
        registry: &TypeRegistry,
    ) -> Result<bool> {
        match (t1, t2) {
            // 両方とも依存関数型の場合
            (Type::DependentFunction { param: p1, param_ty: pt1, return_ty: rt1 },
             Type::DependentFunction { param: p2, param_ty: pt2, return_ty: rt2 }) => {
                // パラメータ型が等価かチェック
                let param_type_eq = registry.is_equivalent_basic(
                    &registry.resolve(*pt1),
                    &registry.resolve(*pt2)
                )?;
                
                if !param_type_eq {
                    return Ok(false);
                }
                
                // 戻り値型の比較には、パラメータ名を統一する必要がある
                // p2の名前をp1に置き換えた戻り値型を取得
                let p2_var_expr = TypeLevelExpr::Var(*p2);
                let p1_var_expr = TypeLevelExpr::Var(*p1);
                
                // p2の実際の変数参照をp1に置き換える（α変換）
                let substituted_rt2 = registry.substitute_in_type(*rt2, *p2, &p1_var_expr)?;
                
                // 置き換え後の戻り値型を比較
                registry.is_equivalent(*rt1, substituted_rt2)
            },
            
            // 両方とも精製型の場合
            (Type::Refinement { base: b1, predicate: pred1 },
             Type::Refinement { base: b2, predicate: pred2 }) => {
                // 基本型が等価かチェック
                let base_eq = registry.is_equivalent_basic(
                    &registry.resolve(*b1),
                    &registry.resolve(*b2)
                )?;
                
                if !base_eq {
                    return Ok(false);
                }
                
                // 述語が等価かチェック
                self.check_predicate_equivalence(pred1, pred2)
            },
            
            // 一方が精製型で他方が基本型の場合
            (Type::Refinement { base, .. }, other) | (other, Type::Refinement { base, .. }) => {
                // 精製型の基本型とother型が等価かチェック
                registry.is_equivalent_basic(
                    &registry.resolve(*base),
                    other
                )
            },
            
            // その他の型のベーシックな比較
            _ => registry.is_equivalent_basic(t1, t2),
        }
    }

    /// 述語の論理的等価性をチェック
    fn check_predicate_equivalence(
        &mut self,
        p1: &RefinementPredicate,
        p2: &RefinementPredicate,
    ) -> Result<bool> {
        // 述語の論理的等価性チェックの実装
        // 一般的な論理式の等価性証明は複雑なため、シンプルなケースのみ処理
        match (p1, p2) {
            (RefinementPredicate::BoolLiteral(b1), RefinementPredicate::BoolLiteral(b2)) => {
                Ok(b1 == b2)
            }
            (RefinementPredicate::IntComparison { op: op1, lhs: lhs1, rhs: rhs1 },
             RefinementPredicate::IntComparison { op: op2, lhs: lhs2, rhs: rhs2 }) => {
                if op1 == op2 && lhs1 == lhs2 && rhs1 == rhs2 {
                    Ok(true)
                } else {
                    // 複雑な式の等価性は未実装
                    Ok(false)
                }
            }
            // その他の組み合わせは等価でないと判断
            _ => Ok(false),
        }
    }

    /// 依存型の推論を実行
    pub fn infer_dependent_type(
        &mut self,
        expr: &TypeLevelExpr,
        context: &HashMap<Symbol, TypeId>,
        registry: &TypeRegistry,
    ) -> Result<TypeId> {
        match expr {
            // 変数参照
            TypeLevelExpr::Var(name) => {
                if let Some(&type_id) = context.get(name) {
                    Ok(type_id)
                } else {
                    Err(CompilerError::new(
                        ErrorKind::TypeSystem,
                        format!("未定義の型レベル変数 '{}'", name),
                        SourceLocation::default(),
                    ))
                }
            },
            
            // リテラル値
            TypeLevelExpr::Literal(lit) => {
                match lit {
                    TypeLevelLiteralValue::Int(_) => {
                        registry.lookup_builtin(BuiltinType::Int64)
                    },
                    TypeLevelLiteralValue::Bool(_) => {
                        registry.lookup_builtin(BuiltinType::Bool)
                    },
                    TypeLevelLiteralValue::String(_) => {
                        registry.lookup_builtin(BuiltinType::String)
                    },
                    TypeLevelLiteralValue::Type(type_id) => {
                        // 型を表す型（メタタイプ）
                        let metatype = Type::MetaType(*type_id);
                        let id = TypeId(registry.next_id.fetch_add(1, Ordering::Relaxed));
                        let type_arc = Arc::new(metatype);
                        {
                            let mut types = registry.types.write();
                            types.insert(id, type_arc);
                        }
                        Ok(id)
                    },
                    TypeLevelLiteralValue::List(items) => {
                        if items.is_empty() {
                            // 空リストの場合は汎用リスト型を返す
                            registry.lookup_builtin(BuiltinType::EmptyList)
                        } else {
                            // 最初の要素の型を推論して、その型のリストと判断
                            let first_expr = TypeLevelExpr::Literal(items[0].clone());
                            let element_type = self.infer_dependent_type(
                                &first_expr, context, registry
                            )?;
                            
                            // 配列型を生成
                            let array_type = Type::Array {
                                element: element_type,
                                size: Some(items.len()),
                            };
                            
                            let id = TypeId(registry.next_id.fetch_add(1, Ordering::Relaxed));
                            let type_arc = Arc::new(array_type);
                            {
                                let mut types = registry.types.write();
                                types.insert(id, type_arc);
                            }
                            Ok(id)
                        }
                    },
                }
            },
            
            // 二項演算
            TypeLevelExpr::BinaryOp { op, left, right } => {
                match op {
                    // 比較演算子は常に真偽値を返す
                    TypeLevelBinaryOp::Eq | TypeLevelBinaryOp::Ne |
                    TypeLevelBinaryOp::Lt | TypeLevelBinaryOp::Le |
                    TypeLevelBinaryOp::Gt | TypeLevelBinaryOp::Ge => {
                        registry.lookup_builtin(BuiltinType::Bool)
                    },
                    
                    // 論理演算子も真偽値を返す
                    TypeLevelBinaryOp::And | TypeLevelBinaryOp::Or => {
                        registry.lookup_builtin(BuiltinType::Bool)
                    },
                    
                    // 算術演算子は整数型を返す
                    TypeLevelBinaryOp::Add | TypeLevelBinaryOp::Sub |
                    TypeLevelBinaryOp::Mul | TypeLevelBinaryOp::Div |
                    TypeLevelBinaryOp::Mod => {
                        registry.lookup_builtin(BuiltinType::Int64)
                    },
                }
            },
            
            // 型レベル関数適用
            TypeLevelExpr::Apply { func, args } => {
                let func_type = self.infer_dependent_type(func, context, registry)?;
                let func_type_resolved = registry.resolve(func_type);
                
                match &*func_type_resolved {
                    Type::DependentFunction { param, param_ty, return_ty } => {
                        if args.len() != 1 {
                            return Err(CompilerError::new(
                                ErrorKind::TypeSystem,
                                format!("型レベル関数は1つの引数を期待していますが、{}個が与えられました", args.len()),
                                SourceLocation::default(),
                            ));
                        }
                        
                        // 引数の型を推論
                        let arg_type = self.infer_dependent_type(&args[0], context, registry)?;
                        
                        // 引数の型がパラメータの型と一致するか確認
                        if !registry.is_equivalent(arg_type, *param_ty)? {
                            return Err(CompilerError::new(
                                ErrorKind::TypeSystem,
                                "型レベル関数の引数の型が一致しません".to_string(),
                                SourceLocation::default(),
                            ));
                        }
                        
                        // 引数の値で戻り値の型を置き換える
                        registry.substitute_in_type(*return_ty, *param, &args[0])
                    },
                    _ => {
                        Err(CompilerError::new(
                            ErrorKind::TypeSystem,
                            "関数型ではない値への適用です".to_string(),
                            SourceLocation::default(),
                        ))
                    }
                }
            },
            
            // 条件式
            TypeLevelExpr::If { condition, then_expr, else_expr } => {
                let cond_type = self.infer_dependent_type(condition, context, registry)?;
                let then_type = self.infer_dependent_type(then_expr, context, registry)?;
                let else_type = self.infer_dependent_type(else_expr, context, registry)?;
                
                // 条件が真偽値型であることを確認
                let bool_type = registry.lookup_builtin(BuiltinType::Bool)?;
                if !registry.is_equivalent(cond_type, bool_type)? {
                    return Err(CompilerError::new(
                        ErrorKind::TypeSystem,
                        "if式の条件部は真偽値型である必要があります".to_string(),
                        SourceLocation::default(),
                    ));
                }
                
                // then部とelse部の型が一致することを確認
                if registry.is_equivalent(then_type, else_type)? {
                    Ok(then_type)
                } else {
                    Err(CompilerError::new(
                        ErrorKind::TypeSystem,
                        "if式のthen部とelse部の型が一致しません".to_string(),
                        SourceLocation::default(),
                    ))
                }
            },
            
            // ラムダ式
            TypeLevelExpr::Lambda { param, param_kind, body } => {
                // パラメータの型を推測（簡略化のためとりあえずInt64と仮定）
                let param_type = registry.lookup_builtin(BuiltinType::Int64)?;
                
                // 拡張されたコンテキストを作成
                let mut extended_context = context.clone();
                extended_context.insert(*param, param_type);
                
                // ボディの型を推論
                let body_type = self.infer_dependent_type(body, &extended_context, registry)?;
                
                // 依存関数型を作成
                let dependent_func_type = Type::DependentFunction {
                    param: *param,
                    param_ty: param_type,
                    return_ty: body_type,
                };
                
                let id = TypeId(registry.next_id.fetch_add(1, Ordering::Relaxed));
                let type_arc = Arc::new(dependent_func_type);
                {
                    let mut types = registry.types.write();
                    types.insert(id, type_arc);
                }
                Ok(id)
            },
            
            // Let束縛
            TypeLevelExpr::Let { name, value, body } => {
                // 値の型を推論
                let value_type = self.infer_dependent_type(value, context, registry)?;
                
                // 拡張されたコンテキストを作成
                let mut extended_context = context.clone();
                extended_context.insert(*name, value_type);
                
                // ボディの型を推論
                self.infer_dependent_type(body, &extended_context, registry)
            },
        }
    }

    /// 精製型の述語がサブタイプ関係にあるかチェック
    pub fn check_subtype_predicate(
        &mut self,
        sub_pred: &RefinementPredicate,
        super_pred: &RefinementPredicate,
    ) -> Result<bool> {
        // 簡単なケース: 同じ述語ならサブタイプ
        if sub_pred == super_pred {
            return Ok(true);
        }
        
        // サブタイプチェックの実装はとても複雑になり得る
        // ここでは単純な実装のみ提供し、将来的にはSMTソルバーとの連携なども検討
        
        match (sub_pred, super_pred) {
            // 論理積(AND)は各部分述語がサブタイプであることを確認
            (RefinementPredicate::And(sub_preds), RefinementPredicate::And(super_preds)) => {
                // 簡略化: subの各述語がsuperのいずれかの述語のサブタイプであること
                for sub_p in sub_preds {
                    let mut found = false;
                    for super_p in super_preds {
                        if self.check_subtype_predicate(sub_p, super_p)? {
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        return Ok(false);
                    }
                }
                Ok(true)
            },
            
            // 整数比較のサブタイプ関係
            (
                RefinementPredicate::IntComparison { 
                    op: op1, 
                    lhs: lhs1, 
                    rhs: rhs1 
                },
                RefinementPredicate::IntComparison { 
                    op: op2, 
                    lhs: lhs2, 
                    rhs: rhs2 
                }
            ) => {
                // 同じ変数についての制約の場合のみ簡略化した判定を行う
                if lhs1 == lhs2 {
                    match (op1, op2, rhs1, rhs2) {
                        // x >= a はx >= b のサブタイプ（a >= b の場合）
                        (ComparisonOp::GreaterEqual, ComparisonOp::GreaterEqual, 
                         TypeLevelLiteralValue::Int(a), TypeLevelLiteralValue::Int(b)) => {
                            Ok(a >= b)
                        },
                        
                        // x > a はx > b のサブタイプ（a >= b の場合）
                        (ComparisonOp::Greater, ComparisonOp::Greater, 
                         TypeLevelLiteralValue::Int(a), TypeLevelLiteralValue::Int(b)) => {
                            Ok(a >= b)
                        },
                        
                        // x <= a はx <= b のサブタイプ（a <= b の場合）
                        (ComparisonOp::LessEqual, ComparisonOp::LessEqual, 
                         TypeLevelLiteralValue::Int(a), TypeLevelLiteralValue::Int(b)) => {
                            Ok(a <= b)
                        },
                        
                        // x < a はx < b のサブタイプ（a <= b の場合）
                        (ComparisonOp::Less, ComparisonOp::Less, 
                         TypeLevelLiteralValue::Int(a), TypeLevelLiteralValue::Int(b)) => {
                            Ok(a <= b)
                        },
                        
                        // x > a はx >= a のサブタイプ
                        (ComparisonOp::Greater, ComparisonOp::GreaterEqual, 
                         TypeLevelLiteralValue::Int(a), TypeLevelLiteralValue::Int(b)) => {
                            Ok(a >= b)
                        },
                        
                        // x < a はx <= a のサブタイプ
                        (ComparisonOp::Less, ComparisonOp::LessEqual, 
                         TypeLevelLiteralValue::Int(a), TypeLevelLiteralValue::Int(b)) => {
                            Ok(a <= b)
                        },
                        
                        // それ以外の場合は簡単な判定ができないので、保守的にfalseを返す
                        _ => Ok(false),
                    }
                } else {
                    // 異なる変数間の制約は現時点では判定できない
                    Ok(false)
                }
            },
            
            // その他のケースは現時点では簡単に判定できないのでfalseを返す
            _ => Ok(false),
        }
    }
}

/// 比較演算子
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComparisonOp {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

/// 依存型のメタタイプ
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct MetaType;

/// 依存型のテストモジュール
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_level_expression_evaluation() {
        let mut solver = DependentTypeSolver::new();
        
        // 単純な整数リテラル
        let expr = TypeLevelExpr::Literal(TypeLevelLiteralValue::Int(42));
        let result = solver.evaluate(&expr).unwrap();
        assert_eq!(result, TypeLevelValue::Literal(TypeLevelLiteralValue::Int(42)));
        
        // 二項演算
        let expr = TypeLevelExpr::BinaryOp {
            op: TypeLevelBinaryOp::Add,
            left: Box::new(TypeLevelExpr::Literal(TypeLevelLiteralValue::Int(10))),
            right: Box::new(TypeLevelExpr::Literal(TypeLevelLiteralValue::Int(20))),
        };
        let result = solver.evaluate(&expr).unwrap();
        assert_eq!(result, TypeLevelValue::Literal(TypeLevelLiteralValue::Int(30)));
    }

    #[test]
    fn test_dependent_function() {
        let mut solver = DependentTypeSolver::new();
        
        // n + 1 を計算する関数
        let inc_func = TypeLevelExpr::Lambda {
            param: Symbol::intern("n"),
            param_kind: Kind::Natural,
            body: Box::new(TypeLevelExpr::BinaryOp {
                op: TypeLevelBinaryOp::Add,
                left: Box::new(TypeLevelExpr::Var(Symbol::intern("n"))),
                right: Box::new(TypeLevelExpr::Literal(TypeLevelLiteralValue::Int(1))),
            }),
        };
        
        // 関数適用: inc(5)
        let apply_expr = TypeLevelExpr::Apply {
            func: Box::new(inc_func),
            args: vec![TypeLevelExpr::Literal(TypeLevelLiteralValue::Int(5))],
        };
        
        let result = solver.evaluate(&apply_expr).unwrap();
        assert_eq!(result, TypeLevelValue::Literal(TypeLevelLiteralValue::Int(6)));
    }
} 
// SwiftLight Type System - Ownership
// 所有権システムの実装

//! # 所有権システム
//! 
//! SwiftLight言語の高度な所有権システムを実装します。
//! このモジュールは、安全なメモリ管理のための所有権、借用、ライフタイムの自動検証を提供します。
//! 
//! - 所有権の追跡と検証
//! - 借用チェッカー
//! - ライフタイム推論
//! - 線形型と消費型の管理
//! - リージョンベース所有権管理

use std::collections::{HashMap, HashSet, BTreeSet, VecDeque};
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};

use crate::frontend::ast::{Expr, ExprKind, Statement, StatementKind};
use crate::frontend::error::{CompilerError, ErrorKind, Result, SourceLocation};
use crate::typesystem::{
    Type, TypeId, TypeRegistry, Symbol, Kind, 
    TypeError, TypeManager,
    effects::{EffectSet, EffectKind},
};

/// ライフタイム識別子
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Lifetime {
    /// ライフタイムのID
    pub id: usize,
    /// ライフタイムの名前（デバッグ用）
    pub name: Option<Symbol>,
    /// 親スコープのライフタイム（ネスト関係）
    pub parent: Option<usize>,
}

/// 所有権の種類
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipKind {
    /// 所有（値のライフサイクルを管理）
    Owned,
    /// 共有借用（読み取り専用）
    SharedBorrow(Lifetime),
    /// 可変借用（読み書き可能）
    MutableBorrow(Lifetime),
    /// 弱参照（所有権なし）
    Weak,
    /// 所有権移動済み
    Moved,
    /// 線形型（一度だけ使用可能）
    Linear,
    /// 使用済み線形型
    Consumed,
}

/// 変数の所有権状態
#[derive(Debug, Clone)]
pub struct OwnershipState {
    /// 現在の所有権状態
    pub kind: OwnershipKind,
    /// 借用情報（変数が借用されている場合）
    pub borrows: Vec<(Symbol, OwnershipKind)>,
    /// 最後のアクセス位置
    pub last_access: Option<SourceLocation>,
}

impl OwnershipState {
    /// 新しい所有状態を作成
    pub fn new(kind: OwnershipKind) -> Self {
        Self {
            kind,
            borrows: Vec::new(),
            last_access: None,
        }
    }
    
    /// 所有状態を作成
    pub fn owned() -> Self {
        Self::new(OwnershipKind::Owned)
    }
    
    /// 共有借用状態を作成
    pub fn shared_borrow(lifetime: Lifetime) -> Self {
        Self::new(OwnershipKind::SharedBorrow(lifetime))
    }
    
    /// 可変借用状態を作成
    pub fn mutable_borrow(lifetime: Lifetime) -> Self {
        Self::new(OwnershipKind::MutableBorrow(lifetime))
    }
    
    /// 線形型状態を作成
    pub fn linear() -> Self {
        Self::new(OwnershipKind::Linear)
    }
    
    /// 借用を追加
    pub fn add_borrow(&mut self, var: Symbol, kind: OwnershipKind) {
        self.borrows.push((var, kind));
    }
    
    /// 借用を解除
    pub fn remove_borrow(&mut self, var: Symbol) {
        self.borrows.retain(|(v, _)| *v != var);
    }
    
    /// 移動済みとしてマーク
    pub fn mark_moved(&mut self) {
        self.kind = OwnershipKind::Moved;
    }
    
    /// 消費済みとしてマーク（線形型用）
    pub fn mark_consumed(&mut self) {
        self.kind = OwnershipKind::Consumed;
    }
    
    /// アクセス位置を更新
    pub fn update_access(&mut self, location: SourceLocation) {
        self.last_access = Some(location);
    }
}

/// 所有権チェッカー
pub struct OwnershipChecker {
    /// 変数の所有権状態
    variables: HashMap<Symbol, OwnershipState>,
    /// ライフタイム情報
    lifetimes: HashMap<usize, Lifetime>,
    /// 次のライフタイムID
    next_lifetime_id: usize,
    /// 現在のスコープスタック
    scope_stack: Vec<usize>,
    /// エラーリスト
    errors: Vec<String>,
    /// 型レジストリへの参照
    type_registry: Arc<TypeRegistry>,
}

impl OwnershipChecker {
    /// 新しい所有権チェッカーを作成
    pub fn new(type_registry: Arc<TypeRegistry>) -> Self {
        Self {
            variables: HashMap::new(),
            lifetimes: HashMap::new(),
            next_lifetime_id: 0,
            scope_stack: Vec::new(),
            errors: Vec::new(),
            type_registry,
        }
    }
    
    /// 新しいライフタイムを作成
    pub fn fresh_lifetime(&mut self, name: Option<Symbol>) -> Lifetime {
        let id = self.next_lifetime_id;
        self.next_lifetime_id += 1;
        
        let parent = self.scope_stack.last().copied();
        
        let lifetime = Lifetime { id, name, parent };
        self.lifetimes.insert(id, lifetime.clone());
        
        lifetime
    }
    
    /// スコープを開始
    pub fn enter_scope(&mut self) {
        let lifetime = self.fresh_lifetime(None);
        self.scope_stack.push(lifetime.id);
    }
    
    /// スコープを終了
    pub fn exit_scope(&mut self) -> Result<()> {
        if let Some(scope_id) = self.scope_stack.pop() {
            // スコープを抜ける際に、そのスコープのライフタイムを持つ借用が残っていないかチェック
            for (var_name, state) in &self.variables {
                match &state.kind {
                    OwnershipKind::SharedBorrow(lt) | OwnershipKind::MutableBorrow(lt) => {
                        if lt.id == scope_id {
                            self.errors.push(format!(
                                "変数'{}'の借用がスコープ外で使用されています",
                                var_name
                            ));
                        }
                    },
                    _ => {}
                }
                
                // 借用リストもチェック
                for (borrowed_var, borrow_kind) in &state.borrows {
                    match borrow_kind {
                        OwnershipKind::SharedBorrow(lt) | OwnershipKind::MutableBorrow(lt) => {
                            if lt.id == scope_id {
                                self.errors.push(format!(
                                    "変数'{}'から'{}'への借用がスコープ外で使用されています",
                                    var_name, borrowed_var
                                ));
                            }
                        },
                        _ => {}
                    }
                }
            }
            
            Ok(())
        } else {
            Err(CompilerError::new(
                ErrorKind::TypeSystem,
                "スコープスタックが空です".to_string(),
                SourceLocation::default(),
            ))
        }
    }
    
    /// 変数を登録
    pub fn register_variable(&mut self, name: Symbol, type_id: TypeId) {
        let ty = self.type_registry.resolve(type_id);
        
        let state = match &*ty {
            Type::Reference { is_mutable, .. } => {
                let lifetime = self.fresh_lifetime(None);
                if *is_mutable {
                    OwnershipState::mutable_borrow(lifetime)
                } else {
                    OwnershipState::shared_borrow(lifetime)
                }
            },
            Type::Linear(_) => OwnershipState::linear(),
            _ => OwnershipState::owned(),
        };
        
        self.variables.insert(name, state);
    }
    
    /// 変数の使用をチェック
    pub fn check_variable_use(&mut self, name: Symbol, location: SourceLocation, is_move_context: bool) -> Result<()> {
        if let Some(state) = self.variables.get_mut(&name) {
            // 使用位置を更新
            state.update_access(location);
            
            match &state.kind {
                OwnershipKind::Moved => {
                    return Err(CompilerError::new(
                        ErrorKind::TypeSystem,
                        format!("変数'{}'は既に移動されています", name),
                        location,
                    ));
                },
                OwnershipKind::Consumed => {
                    return Err(CompilerError::new(
                        ErrorKind::TypeSystem,
                        format!("線形型変数'{}'は既に消費されています", name),
                        location,
                    ));
                },
                OwnershipKind::Linear => {
                    if is_move_context {
                        // 線形型が移動コンテキストで使用された場合、消費済みとマーク
                        state.mark_consumed();
                    }
                },
                OwnershipKind::Owned => {
                    if is_move_context {
                        // 所有型が移動コンテキストで使用された場合、移動済みとマーク
                        state.mark_moved();
                    }
                },
                _ => {}
            }
            
            Ok(())
        } else {
            Err(CompilerError::new(
                ErrorKind::TypeSystem,
                format!("未定義の変数: '{}'", name),
                location,
            ))
        }
    }
    
    /// 借用をチェック
    pub fn check_borrow(&mut self, target: Symbol, is_mutable: bool, location: SourceLocation) -> Result<Lifetime> {
        if let Some(state) = self.variables.get(&target) {
            match &state.kind {
                OwnershipKind::Moved => {
                    return Err(CompilerError::new(
                        ErrorKind::TypeSystem,
                        format!("移動済みの変数'{}'を借用しようとしています", target),
                        location,
                    ));
                },
                OwnershipKind::Consumed => {
                    return Err(CompilerError::new(
                        ErrorKind::TypeSystem,
                        format!("消費済みの線形型変数'{}'を借用しようとしています", target),
                        location,
                    ));
                },
                OwnershipKind::MutableBorrow(_) => {
                    // 可変借用がある場合、他の借用は不可
                    return Err(CompilerError::new(
                        ErrorKind::TypeSystem,
                        format!("変数'{}'は既に可変借用されています", target),
                        location,
                    ));
                },
                OwnershipKind::SharedBorrow(_) => {
                    if is_mutable {
                        // 共有借用がある場合、可変借用は不可
                        return Err(CompilerError::new(
                            ErrorKind::TypeSystem,
                            format!("共有借用されている変数'{}'を可変借用しようとしています", target),
                            location,
                        ));
                    }
                },
                _ => {}
            }
            
            // 新しいライフタイムを作成
            let lifetime = self.fresh_lifetime(None);
            
            // 借用を記録
            if let Some(state) = self.variables.get_mut(&target) {
                let borrower = Symbol::intern(&format!("anonymous_{}", lifetime.id));
                if is_mutable {
                    state.add_borrow(borrower, OwnershipKind::MutableBorrow(lifetime.clone()));
                } else {
                    state.add_borrow(borrower, OwnershipKind::SharedBorrow(lifetime.clone()));
                }
            }
            
            Ok(lifetime)
        } else {
            Err(CompilerError::new(
                ErrorKind::TypeSystem,
                format!("未定義の変数: '{}'", target),
                location,
            ))
        }
    }
    
    /// 式の所有権をチェック
    pub fn check_expr(&mut self, expr: &Expr, is_move_context: bool) -> Result<()> {
        match &expr.kind {
            ExprKind::Variable(name) => {
                self.check_variable_use(*name, expr.location, is_move_context)?;
            },
            
            ExprKind::Binary { op, left, right } => {
                // 二項演算子の左右の式をチェック
                self.check_expr(left, false)?;
                self.check_expr(right, false)?;
            },
            
            ExprKind::Unary { op, expr: inner_expr } => {
                // 単項演算子の式をチェック
                self.check_expr(inner_expr, false)?;
            },
            
            ExprKind::Call { function, args } => {
                // 関数呼び出しをチェック
                self.check_expr(function, false)?;
                
                // 引数は移動コンテキストとしてチェック（保守的）
                for arg in args {
                    self.check_expr(arg, true)?;
                }
            },
            
            ExprKind::Assignment { target, value } => {
                // 代入のターゲットをチェック
                if let ExprKind::Variable(name) = &target.kind {
                    // 代入先が変数の場合、その所有権状態をチェック
                    if let Some(state) = self.variables.get(name) {
                        match &state.kind {
                            OwnershipKind::Moved => {
                                return Err(CompilerError::new(
                                    ErrorKind::TypeSystem,
                                    format!("移動済みの変数'{}'に代入しようとしています", name),
                                    target.location,
                                ));
                            },
                            OwnershipKind::Consumed => {
                                return Err(CompilerError::new(
                                    ErrorKind::TypeSystem,
                                    format!("消費済みの線形型変数'{}'に代入しようとしています", name),
                                    target.location,
                                ));
                            },
                            OwnershipKind::SharedBorrow(_) => {
                                return Err(CompilerError::new(
                                    ErrorKind::TypeSystem,
                                    format!("共有借用されている変数'{}'に代入しようとしています", name),
                                    target.location,
                                ));
                            },
                            _ => {}
                        }
                    }
                }
                
                // 代入値をチェック（移動コンテキスト）
                self.check_expr(value, true)?;
            },
            
            ExprKind::Block { statements, result } => {
                // ブロックスコープを開始
                self.enter_scope();
                
                // 各文をチェック
                for stmt in statements {
                    self.check_statement(stmt)?;
                }
                
                // 結果式があればチェック
                if let Some(result_expr) = result {
                    self.check_expr(result_expr, is_move_context)?;
                }
                
                // ブロックスコープを終了
                self.exit_scope()?;
            },
            
            ExprKind::If { condition, then_branch, else_branch } => {
                // 条件式をチェック
                self.check_expr(condition, false)?;
                
                // then節をチェック
                self.check_expr(then_branch, is_move_context)?;
                
                // else節があればチェック
                if let Some(else_expr) = else_branch {
                    self.check_expr(else_expr, is_move_context)?;
                }
            },
            
            ExprKind::Reference { expr: inner_expr, is_mutable } => {
                // 参照式をチェック
                if let ExprKind::Variable(name) = &inner_expr.kind {
                    // 変数の借用をチェック
                    self.check_borrow(*name, *is_mutable, expr.location)?;
                } else {
                    // 変数以外の参照は現時点では単純にチェック
                    self.check_expr(inner_expr, false)?;
                }
            },
            
            // リテラルなど、所有権に影響しない式
            ExprKind::IntLiteral(_) | ExprKind::FloatLiteral(_) |
            ExprKind::BoolLiteral(_) | ExprKind::CharLiteral(_) |
            ExprKind::StringLiteral(_) => {
                // 何もしない
            },
            
            // TODO: その他の式タイプの所有権チェック
            
            _ => {
                // 未対応の式タイプは保守的に処理
                // エラーは出さず、今後実装を進める
            }
        }
        
        Ok(())
    }
    
    /// 文の所有権をチェック
    pub fn check_statement(&mut self, stmt: &Statement) -> Result<()> {
        match &stmt.kind {
            StatementKind::Let { pattern, type_annotation, initializer } => {
                // 初期化式をチェック（移動コンテキスト）
                self.check_expr(initializer, true)?;
                
                // パターンで導入される変数を登録
                // TODO: 複雑なパターンのサポート
                // 現時点では単純な変数パターンのみ処理
                
                // TODO: 型注釈に基づいて所有権状態を設定
            },
            
            StatementKind::Expression(expr) => {
                // 式をチェック
                self.check_expr(expr, false)?;
            },
            
            StatementKind::Return(expr) => {
                if let Some(e) = expr {
                    // 戻り値式をチェック（移動コンテキスト）
                    self.check_expr(e, true)?;
                }
            },
            
            // TODO: その他の文タイプの所有権チェック
            
            _ => {
                // 未対応の文タイプは保守的に処理
            }
        }
        
        Ok(())
    }
    
    /// スコープ終了時の線形型チェック
    pub fn check_linear_types_consumed(&self) -> Result<()> {
        let mut errors = Vec::new();
        
        for (name, state) in &self.variables {
            if let OwnershipKind::Linear = state.kind {
                errors.push(format!("線形型変数'{}'が消費されていません", name));
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(CompilerError::new(
                ErrorKind::TypeSystem,
                errors.join("\n"),
                SourceLocation::default(),
            ))
        }
    }
    
    /// エラーのリストを取得
    pub fn get_errors(&self) -> &[String] {
        &self.errors
    }
}

/// リージョンベースのメモリ管理
pub struct RegionManager {
    /// リージョンマップ（ID -> リージョン）
    regions: HashMap<usize, Region>,
    /// 次のリージョンID
    next_region_id: usize,
    /// 変数とリージョンの対応
    var_regions: HashMap<Symbol, usize>,
    /// 型レジストリへの参照
    type_registry: Arc<TypeRegistry>,
}

/// メモリリージョン
#[derive(Debug, Clone)]
pub struct Region {
    /// リージョンのID
    pub id: usize,
    /// リージョンの名前
    pub name: Symbol,
    /// 親リージョン
    pub parent: Option<usize>,
    /// リージョン内の変数
    pub variables: HashSet<Symbol>,
    /// リージョンのライフタイム
    pub lifetime: Lifetime,
}

impl RegionManager {
    /// 新しいリージョンマネージャーを作成
    pub fn new(type_registry: Arc<TypeRegistry>) -> Self {
        Self {
            regions: HashMap::new(),
            next_region_id: 0,
            var_regions: HashMap::new(),
            type_registry,
        }
    }
    
    /// 新しいリージョンを作成
    pub fn create_region(&mut self, name: Symbol, parent: Option<usize>, lifetime: Lifetime) -> usize {
        let id = self.next_region_id;
        self.next_region_id += 1;
        
        let region = Region {
            id,
            name,
            parent,
            variables: HashSet::new(),
            lifetime,
        };
        
        self.regions.insert(id, region);
        id
    }
    
    /// 変数をリージョンに追加
    pub fn add_variable_to_region(&mut self, var: Symbol, region_id: usize) -> Result<()> {
        if let Some(region) = self.regions.get_mut(&region_id) {
            region.variables.insert(var);
            self.var_regions.insert(var, region_id);
            Ok(())
        } else {
            Err(CompilerError::new(
                ErrorKind::TypeSystem,
                format!("存在しないリージョンID: {}", region_id),
                SourceLocation::default(),
            ))
        }
    }
    
    /// 変数のリージョンを取得
    pub fn get_variable_region(&self, var: Symbol) -> Option<usize> {
        self.var_regions.get(&var).copied()
    }
    
    /// リージョンを取得
    pub fn get_region(&self, region_id: usize) -> Option<&Region> {
        self.regions.get(&region_id)
    }
    
    /// リージョン間の包含関係をチェック
    pub fn is_region_contained_in(&self, inner: usize, outer: usize) -> bool {
        if inner == outer {
            return true;
        }
        
        let mut current = inner;
        while let Some(region) = self.regions.get(&current) {
            if let Some(parent) = region.parent {
                if parent == outer {
                    return true;
                }
                current = parent;
            } else {
                break;
            }
        }
        
        false
    }
    
    /// 変数アクセスのリージョン安全性チェック
    pub fn check_access_safety(&self, var: Symbol, accessing_region: usize) -> Result<()> {
        if let Some(var_region) = self.get_variable_region(var) {
            if self.is_region_contained_in(var_region, accessing_region) {
                Ok(())
            } else {
                Err(CompilerError::new(
                    ErrorKind::TypeSystem,
                    format!("変数'{}'への安全でないリージョンアクセス", var),
                    SourceLocation::default(),
                ))
            }
        } else {
            Err(CompilerError::new(
                ErrorKind::TypeSystem,
                format!("リージョンに割り当てられていない変数: '{}'", var),
                SourceLocation::default(),
            ))
        }
    }
} 
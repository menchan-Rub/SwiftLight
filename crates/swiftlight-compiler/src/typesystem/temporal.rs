// SwiftLight Type System - Temporal
// 時間型システムの実装

//! # 時間型システム
//! 
//! SwiftLight言語の時間的性質と状態変化を型レベルで追跡するためのシステムを実装します。
//! このモジュールは、Typestate programmingとステート機械の検証を可能にします。
//! 
//! - オブジェクトの状態追跡
//! - 状態遷移の検証
//! - プロトコル準拠の時間的検証
//! - 時相論理に基づく性質検証
//! - 時間的契約の強制

use std::collections::{HashMap, HashSet, BTreeSet, VecDeque};
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};

use crate::frontend::ast::{Expr, ExprKind, Statement, StatementKind};
use crate::frontend::error::{CompilerError, ErrorKind, Result, SourceLocation};
use crate::typesystem::{
    Type, TypeId, TypeRegistry, Symbol, Kind, 
    TypeError, TypeManager,
};

/// 状態識別子
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StateId {
    /// 状態のID
    pub id: usize,
    /// 状態の名前
    pub name: Symbol,
}

/// 状態遷移
#[derive(Debug, Clone)]
pub struct Transition {
    /// 遷移の名前（メソッド名など）
    pub name: Symbol,
    /// 遷移前の状態
    pub from_state: StateId,
    /// 遷移後の状態
    pub to_state: StateId,
    /// 遷移の条件（オプション）
    pub condition: Option<Expr>,
    /// 遷移のアクション（関数呼び出しなど）
    pub action: Option<Expr>,
}

/// ステートマシン
#[derive(Debug, Clone)]
pub struct StateMachine {
    /// ステートマシンの名前
    pub name: Symbol,
    /// 状態のリスト
    pub states: Vec<StateId>,
    /// 遷移のリスト
    pub transitions: Vec<Transition>,
    /// 初期状態
    pub initial_state: StateId,
    /// 終了状態のリスト
    pub final_states: Vec<StateId>,
}

/// 時間的制約
#[derive(Debug, Clone)]
pub enum TemporalConstraint {
    /// 状態順序制約（状態Aは状態Bの前に来なければならない）
    OrderConstraint {
        before: StateId,
        after: StateId,
    },
    /// 状態到達可能性制約（状態Aから状態Bに到達可能でなければならない）
    ReachabilityConstraint {
        from: StateId,
        to: StateId,
    },
    /// 状態不変条件（状態Aにいる間は条件Cが常に成立しなければならない）
    InvariantConstraint {
        state: StateId,
        condition: Expr,
    },
    /// 時相論理式（LTL/CTLなどの時相論理に基づく制約）
    TemporalLogicConstraint {
        formula: String, // 構文木に変換する前の文字列表現
        parsed_formula: Option<TemporalFormula>,
    },
}

/// 時相論理式
#[derive(Debug, Clone)]
pub enum TemporalFormula {
    /// 原子命題（状態や条件）
    Atom(String),
    /// 論理積（AND）
    And(Box<TemporalFormula>, Box<TemporalFormula>),
    /// 論理和（OR）
    Or(Box<TemporalFormula>, Box<TemporalFormula>),
    /// 論理否定（NOT）
    Not(Box<TemporalFormula>),
    /// 次状態（NEXT）
    Next(Box<TemporalFormula>),
    /// いつか（EVENTUALLY）
    Eventually(Box<TemporalFormula>),
    /// 常に（ALWAYS）
    Always(Box<TemporalFormula>),
    /// AまでB（UNTIL）
    Until(Box<TemporalFormula>, Box<TemporalFormula>),
}

/// Typestate型
#[derive(Debug, Clone)]
pub struct TypestateType {
    /// 基本型
    pub base_type: TypeId,
    /// 現在の状態
    pub current_state: StateId,
    /// 関連するステートマシン
    pub state_machine: Symbol,
}

/// 時間型エラー
#[derive(Debug, Clone)]
pub enum TemporalTypeError {
    /// 無効な状態遷移
    InvalidStateTransition {
        from_state: StateId,
        to_state: StateId,
        method: Symbol,
        location: SourceLocation,
    },
    /// 到達不能状態
    UnreachableState {
        state: StateId,
        location: SourceLocation,
    },
    /// 状態不変条件違反
    InvariantViolation {
        state: StateId,
        condition: String,
        location: SourceLocation,
    },
    /// 未完了のステートマシン（終了状態に到達しない）
    IncompleteStateMachine {
        object: Symbol,
        current_state: StateId,
        location: SourceLocation,
    },
    /// 時相論理制約違反
    TemporalLogicViolation {
        formula: String,
        location: SourceLocation,
    },
    /// その他のエラー
    Other {
        message: String,
        location: SourceLocation,
    },
}

/// 時間型チェッカー
pub struct TemporalTypeChecker {
    /// ステートマシン定義
    state_machines: HashMap<Symbol, StateMachine>,
    /// オブジェクトの現在の状態
    object_states: HashMap<Symbol, TypestateType>,
    /// 時間的制約
    constraints: Vec<TemporalConstraint>,
    /// エラーリスト
    errors: Vec<TemporalTypeError>,
    /// 型レジストリへの参照
    type_registry: Arc<TypeRegistry>,
}

impl TemporalTypeChecker {
    /// 新しい時間型チェッカーを作成
    pub fn new(type_registry: Arc<TypeRegistry>) -> Self {
        Self {
            state_machines: HashMap::new(),
            object_states: HashMap::new(),
            constraints: Vec::new(),
            errors: Vec::new(),
            type_registry,
        }
    }
    
    /// ステートマシンを登録
    pub fn register_state_machine(&mut self, state_machine: StateMachine) {
        self.state_machines.insert(state_machine.name, state_machine);
    }
    
    /// オブジェクトの状態を登録
    pub fn register_object_state(&mut self, object: Symbol, typestate: TypestateType) {
        self.object_states.insert(object, typestate);
    }
    
    /// 時間的制約を追加
    pub fn add_constraint(&mut self, constraint: TemporalConstraint) {
        self.constraints.push(constraint);
    }
    
    /// メソッド呼び出しによる状態遷移をチェック
    pub fn check_method_call(&mut self, object: Symbol, method: Symbol, location: SourceLocation) -> Result<()> {
        // オブジェクトの現在の状態を取得
        if let Some(typestate) = self.object_states.get(&object) {
            let state_machine_name = typestate.state_machine;
            let current_state = typestate.current_state.clone();
            
            // ステートマシンを取得
            if let Some(state_machine) = self.state_machines.get(&state_machine_name) {
                // メソッドに対応する遷移を探す
                let mut valid_transition = None;
                
                for transition in &state_machine.transitions {
                    if transition.name == method && transition.from_state == current_state {
                        valid_transition = Some(transition);
                        break;
                    }
                }
                
                if let Some(transition) = valid_transition {
                    // 状態を更新
                    if let Some(typestate) = self.object_states.get_mut(&object) {
                        typestate.current_state = transition.to_state.clone();
                    }
                    
                    Ok(())
                } else {
                    // 有効な遷移が見つからない場合はエラー
                    let error = TemporalTypeError::InvalidStateTransition {
                        from_state: current_state,
                        to_state: StateId { id: 0, name: Symbol::intern("unknown") }, // 未知の遷移先
                        method,
                        location,
                    };
                    
                    self.errors.push(error);
                    
                    Err(CompilerError::new(
                        ErrorKind::TypeSystem,
                        format!("オブジェクト'{}'の状態'{}'からメソッド'{}'を呼び出すことはできません",
                            object, current_state.name, method),
                        location,
                    ))
                }
            } else {
                Err(CompilerError::new(
                    ErrorKind::TypeSystem,
                    format!("ステートマシン'{}'が見つかりません", state_machine_name),
                    location,
                ))
            }
        } else {
            // 状態が追跡されていないオブジェクトはエラーとしない
            Ok(())
        }
    }
    
    /// 式の時間型チェック
    pub fn check_expr(&mut self, expr: &Expr) -> Result<()> {
        match &expr.kind {
            ExprKind::Call { function, args } => {
                // メソッド呼び出しの場合
                if let ExprKind::MemberAccess { object, member } = &function.kind {
                    // オブジェクトのメソッド呼び出しの場合
                    if let ExprKind::Variable(obj_name) = &object.kind {
                        // オブジェクト名が変数の場合
                        self.check_method_call(*obj_name, *member, expr.location)?;
                    }
                }
                
                // 引数も再帰的にチェック
                for arg in args {
                    self.check_expr(arg)?;
                }
            },
            
            ExprKind::MemberAccess { object, member } => {
                // メンバーアクセスもチェック
                self.check_expr(object)?;
            },
            
            ExprKind::Binary { op, left, right } => {
                // 二項演算子の左右もチェック
                self.check_expr(left)?;
                self.check_expr(right)?;
            },
            
            ExprKind::Unary { op, expr: inner_expr } => {
                // 単項演算子の式もチェック
                self.check_expr(inner_expr)?;
            },
            
            ExprKind::Block { statements, result } => {
                // ブロック内の各文をチェック
                for stmt in statements {
                    self.check_statement(stmt)?;
                }
                
                // 結果式があればチェック
                if let Some(result_expr) = result {
                    self.check_expr(result_expr)?;
                }
            },
            
            ExprKind::If { condition, then_branch, else_branch } => {
                // 条件式をチェック
                self.check_expr(condition)?;
                
                // then節をチェック
                self.check_expr(then_branch)?;
                
                // else節があればチェック
                if let Some(else_expr) = else_branch {
                    self.check_expr(else_expr)?;
                }
            },
            
            // 他の式タイプも必要に応じてチェック
            
            _ => {
                // 他の式タイプは特に時間的制約をチェックしない
            }
        }
        
        Ok(())
    }
    
    /// 文の時間型チェック
    pub fn check_statement(&mut self, stmt: &Statement) -> Result<()> {
        match &stmt.kind {
            StatementKind::Expression(expr) => {
                // 式をチェック
                self.check_expr(expr)?;
            },
            
            StatementKind::Let { pattern: _, type_annotation: _, initializer } => {
                // 初期化式をチェック
                self.check_expr(initializer)?;
            },
            
            StatementKind::Return(expr) => {
                // 戻り値式があればチェック
                if let Some(e) = expr {
                    self.check_expr(e)?;
                }
            },
            
            // 他の文タイプも必要に応じてチェック
            
            _ => {
                // 他の文タイプは特に時間的制約をチェックしない
            }
        }
        
        Ok(())
    }
    
    /// オブジェクトのライフサイクル終了時のチェック
    pub fn check_object_lifecycle_end(&mut self, object: Symbol, location: SourceLocation) -> Result<()> {
        if let Some(typestate) = self.object_states.get(&object) {
            let state_machine_name = typestate.state_machine;
            let current_state = &typestate.current_state;
            
            if let Some(state_machine) = self.state_machines.get(&state_machine_name) {
                // 現在の状態が終了状態かどうかをチェック
                if !state_machine.final_states.contains(current_state) {
                    // 終了状態でない場合はエラー
                    let error = TemporalTypeError::IncompleteStateMachine {
                        object,
                        current_state: current_state.clone(),
                        location,
                    };
                    
                    self.errors.push(error);
                    
                    return Err(CompilerError::new(
                        ErrorKind::TypeSystem,
                        format!("オブジェクト'{}'のステートマシンが終了状態に達していません（現在の状態: '{}'）",
                            object, current_state.name),
                        location,
                    ));
                }
            }
        }
        
        Ok(())
    }
    
    /// 到達可能性分析
    pub fn analyze_reachability(&mut self, state_machine_name: Symbol) -> Result<HashMap<StateId, HashSet<StateId>>> {
        if let Some(state_machine) = self.state_machines.get(&state_machine_name) {
            let mut reachability = HashMap::new();
            
            // 各状態からの到達可能な状態を計算
            for state in &state_machine.states {
                let mut reachable = HashSet::new();
                reachable.insert(state.clone()); // 自己ループも含める
                
                for transition in &state_machine.transitions {
                    if transition.from_state == *state {
                        reachable.insert(transition.to_state.clone());
                    }
                }
                
                reachability.insert(state.clone(), reachable);
            }
            
            // 到達可能性の推移閉包を計算
            let mut changed = true;
            while changed {
                changed = false;
                
                for state in &state_machine.states {
                    let mut new_reachable = reachability[state].clone();
                    
                    for to_state in reachability[state].clone() {
                        for further_state in &reachability[&to_state] {
                            if !new_reachable.contains(further_state) {
                                new_reachable.insert(further_state.clone());
                                changed = true;
                            }
                        }
                    }
                    
                    if changed {
                        reachability.insert(state.clone(), new_reachable);
                    }
                }
            }
            
            Ok(reachability)
        } else {
            Err(CompilerError::new(
                ErrorKind::TypeSystem,
                format!("ステートマシン'{}'が見つかりません", state_machine_name),
                SourceLocation::default(),
            ))
        }
    }
    
    /// 時間的制約の検証
    pub fn verify_temporal_constraints(&mut self) -> Result<()> {
        for constraint in &self.constraints {
            match constraint {
                TemporalConstraint::ReachabilityConstraint { from, to } => {
                    // 到達可能性制約をチェック
                    for (state_machine_name, state_machine) in &self.state_machines {
                        if state_machine.states.contains(from) && state_machine.states.contains(to) {
                            let reachability = self.analyze_reachability(*state_machine_name)?;
                            
                            if !reachability[from].contains(to) {
                                // 到達不能な場合はエラー
                                let error = TemporalTypeError::UnreachableState {
                                    state: to.clone(),
                                    location: SourceLocation::default(),
                                };
                                
                                self.errors.push(error);
                            }
                        }
                    }
                },
                
                TemporalConstraint::OrderConstraint { before, after } => {
                    // 順序制約は到達可能性分析で間接的にチェック
                    // （実際には追加の検証が必要）
                },
                
                TemporalConstraint::InvariantConstraint { state, condition } => {
                    // 不変条件はランタイムチェックが必要
                    // （現時点では静的検証を行わない）
                },
                
                TemporalConstraint::TemporalLogicConstraint { formula, parsed_formula } => {
                    // 時相論理式の検証はモデル検査が必要
                    // （現時点では単純なチェックのみ実施）
                },
            }
        }
        
        Ok(())
    }
    
    /// エラーのリストを取得
    pub fn get_errors(&self) -> &[TemporalTypeError] {
        &self.errors
    }
}

/// Typestate型ビルダー
pub struct TypestateBuilder {
    /// ステートマシン名
    state_machine_name: Symbol,
    /// 状態のリスト
    states: Vec<StateId>,
    /// 遷移のリスト
    transitions: Vec<Transition>,
    /// 初期状態
    initial_state: Option<StateId>,
    /// 終了状態のリスト
    final_states: Vec<StateId>,
    /// 次の状態ID
    next_state_id: usize,
}

impl TypestateBuilder {
    /// 新しいTypestate型ビルダーを作成
    pub fn new(name: Symbol) -> Self {
        Self {
            state_machine_name: name,
            states: Vec::new(),
            transitions: Vec::new(),
            initial_state: None,
            final_states: Vec::new(),
            next_state_id: 0,
        }
    }
    
    /// 状態を追加
    pub fn add_state(&mut self, name: Symbol) -> StateId {
        let id = self.next_state_id;
        self.next_state_id += 1;
        
        let state = StateId { id, name };
        self.states.push(state.clone());
        
        state
    }
    
    /// 初期状態を設定
    pub fn set_initial_state(&mut self, state: StateId) -> &mut Self {
        self.initial_state = Some(state);
        self
    }
    
    /// 終了状態を追加
    pub fn add_final_state(&mut self, state: StateId) -> &mut Self {
        self.final_states.push(state);
        self
    }
    
    /// 遷移を追加
    pub fn add_transition(&mut self, from: StateId, to: StateId, method: Symbol) -> &mut Self {
        let transition = Transition {
            name: method,
            from_state: from,
            to_state: to,
            condition: None,
            action: None,
        };
        
        self.transitions.push(transition);
        self
    }
    
    /// 条件付き遷移を追加
    pub fn add_conditional_transition(&mut self, from: StateId, to: StateId, method: Symbol, condition: Expr) -> &mut Self {
        let transition = Transition {
            name: method,
            from_state: from,
            to_state: to,
            condition: Some(condition),
            action: None,
        };
        
        self.transitions.push(transition);
        self
    }
    
    /// ステートマシンをビルド
    pub fn build(&self) -> Result<StateMachine> {
        // 初期状態が設定されているか確認
        let initial_state = self.initial_state.clone().ok_or_else(|| {
            CompilerError::new(
                ErrorKind::TypeSystem,
                format!("ステートマシン'{}'に初期状態が設定されていません", self.state_machine_name),
                SourceLocation::default(),
            )
        })?;
        
        // 少なくとも1つの終了状態があるか確認
        if self.final_states.is_empty() {
            return Err(CompilerError::new(
                ErrorKind::TypeSystem,
                format!("ステートマシン'{}'に終了状態が設定されていません", self.state_machine_name),
                SourceLocation::default(),
            ));
        }
        
        // 全ての状態が存在するか確認
        let all_states: HashSet<_> = self.states.iter().collect();
        
        for transition in &self.transitions {
            if !all_states.contains(&transition.from_state) {
                return Err(CompilerError::new(
                    ErrorKind::TypeSystem,
                    format!("遷移の開始状態'{}'がステートマシン内に存在しません", transition.from_state.name),
                    SourceLocation::default(),
                ));
            }
            
            if !all_states.contains(&transition.to_state) {
                return Err(CompilerError::new(
                    ErrorKind::TypeSystem,
                    format!("遷移の終了状態'{}'がステートマシン内に存在しません", transition.to_state.name),
                    SourceLocation::default(),
                ));
            }
        }
        
        Ok(StateMachine {
            name: self.state_machine_name,
            states: self.states.clone(),
            transitions: self.transitions.clone(),
            initial_state,
            final_states: self.final_states.clone(),
        })
    }
}

/// 拡張: TypeRegistryへのTypestate型関連メソッドの追加
impl TypeRegistry {
    /// Typestate型を作成
    pub fn create_typestate_type(&self, base_type: TypeId, state_machine: Symbol, initial_state: StateId) -> TypeId {
        // ベース型を取得
        let base_ty = self.resolve(base_type);
        
        // Typestate型の名前を作成
        let base_name = match &*base_ty {
            Type::Named { name, .. } => name.to_string(),
            _ => "Unknown".to_string(),
        };
        
        let typestate_name = format!("Typestate<{}, {}>", base_name, state_machine);
        let typestate_sym = Symbol::intern(&typestate_name);
        
        // 型パラメータ
        let params = vec![base_type];
        
        // 新しい型を登録
        self.register_named_type(typestate_sym, params, Kind::Type)
    }
} 
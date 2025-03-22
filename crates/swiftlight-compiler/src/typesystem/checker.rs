// SwiftLight Type System - Type Checker
// 型チェッカーの実装

//! # 型チェッカー
//! 
//! SwiftLight言語の高度な型チェックシステムを実装します。
//! このモジュールは依存型を含む様々な型制約を検証します。
//! 
//! - 精製型のサブタイプ関係検証
//! - 依存型のプロパティ検証
//! - 線形型の所有権検証
//! - エフェクト型の副作用検証
//! - 量子型の量子リソース検証

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};

use crate::frontend::ast::{Expr, ExprKind, Pattern, PatternKind, Statement, StatementKind};
use crate::frontend::error::{CompilerError, ErrorKind, Result, SourceLocation};
use crate::typesystem::{
    Type, TypeId, TypeRegistry, Symbol, Kind, 
    TypeError, RefinementPredicate, TypeManager,
    TypeConstraint, TypeConstraintSet, TypeLevelExpr,
    TypeLevelLiteralValue, DependentTypeSolver,
    ComparisonOp, OrderingOp, LogicalOp, TemporalOperator, QuantumGate,
    inference::{
        InferenceContext, TypeInferencer, InferenceOptimizationLevel,
        QuantumTypeInference, TemporalTypeInference, QuantumConstraint,
        TemporalConstraint, UnifiedConstraintSolver
    },
};

/// 型チェックの厳格さレベル
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeCheckStrictness {
    /// 基本的な型チェックのみ
    Basic,
    /// 標準的な型チェック（依存型の基本チェック含む）
    Standard,
    /// 厳格な型チェック（全ての高度な型機能を検証）
    Strict,
    /// 量子型チェック（量子操作の整合性を検証）
    Quantum,
    /// 時相型チェック（時間的性質を検証）
    Temporal,
    /// 完全型チェック（全ての型システム機能を検証）
    Complete,
}

/// 型チェックエラー
#[derive(Debug, Clone)]
pub struct TypeCheckError {
    /// エラーメッセージ
    pub message: String,
    /// エラーの発生位置
    pub location: SourceLocation,
    /// エラーの種類
    pub kind: TypeCheckErrorKind,
}

/// 型チェックエラーの種類
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeCheckErrorKind {
    /// 型の不一致
    TypeMismatch,
    /// 未定義の変数
    UndefinedVariable,
    /// 線形型の使用エラー
    LinearityViolation,
    /// エフェクト型の制約違反
    EffectViolation,
    /// 依存型の述語違反
    DependentTypeViolation,
    /// リソース使用エラー
    ResourceUsageError,
    /// 量子型エラー
    QuantumTypeError,
    /// 量子ゲート適用エラー
    QuantumGateError,
    /// 量子状態エラー
    QuantumStateError,
    /// 時相型エラー
    TemporalTypeError,
    /// 状態遷移エラー
    StateTransitionError,
    /// 時相論理違反
    TemporalLogicViolation,
    /// その他のエラー
    Other,
}

/// 型チェックコンテキスト
pub struct TypeCheckContext {
    /// 型推論コンテキスト
    inference_context: InferenceContext,
    
    /// 検証済み型の環境
    checked_types: HashMap<Symbol, TypeId>,
    
    /// 線形型の使用状態管理
    linear_vars: HashMap<Symbol, bool>, // true=使用済み
    
    /// 型チェックの厳格さレベル
    strictness: TypeCheckStrictness,
    
    /// 型チェックエラーのリスト
    errors: Vec<TypeCheckError>,
    
    /// 依存型ソルバー
    dependent_solver: DependentTypeSolver,
    
    /// 型レジストリへの参照
    type_registry: Arc<TypeRegistry>,
    
    /// 量子型チェッカー
    quantum_checker: Option<QuantumTypeChecker>,
    
    /// 時相型チェッカー
    temporal_checker: Option<TemporalTypeChecker>,
}

/// 量子型チェッカー
pub struct QuantumTypeChecker {
    /// 量子ビットの追跡
    qubits: HashMap<Symbol, u32>,
    
    /// 量子回路の状態
    circuits: HashMap<Symbol, QuantumCircuitState>,
    
    /// 量子操作の履歴
    operations: Vec<QuantumOperation>,
    
    /// 量子ゲート検証機構
    gate_validator: QuantumGateValidator,
}

/// 量子回路の状態
#[derive(Debug, Clone)]
pub struct QuantumCircuitState {
    /// 量子ビット数
    pub qubits: u32,
    
    /// 操作リスト
    pub operations: Vec<QuantumGate>,
    
    /// エンタングルメント情報
    pub entanglement: HashMap<u32, HashSet<u32>>,
    
    /// 測定済みビット
    pub measured_qubits: HashSet<u32>,
}

/// 量子操作
#[derive(Debug, Clone)]
pub struct QuantumOperation {
    /// ゲート種別
    pub gate: Symbol,
    
    /// 対象量子ビット
    pub target_qubits: Vec<u32>,
    
    /// コントロールビット
    pub control_qubits: Vec<u32>,
    
    /// パラメータ
    pub parameters: Vec<f64>,
    
    /// 操作位置
    pub location: SourceLocation,
}

/// 量子ゲート検証機構
pub struct QuantumGateValidator {
    /// 基本ゲート
    pub basic_gates: HashSet<Symbol>,
    
    /// 複合ゲート
    pub composite_gates: HashMap<Symbol, Vec<QuantumGate>>,
}

/// 時相型チェッカー
pub struct TemporalTypeChecker {
    /// 状態の追跡
    states: HashMap<Symbol, StateInfo>,
    
    /// 状態遷移の追跡
    transitions: HashMap<Symbol, Vec<StateTransition>>,
    
    /// 時相論理式の検証
    temporal_assertions: Vec<TemporalAssertion>,
    
    /// 状態履歴の追跡
    state_history: Vec<StateHistoryEntry>,
}

/// 状態情報
#[derive(Debug, Clone)]
pub struct StateInfo {
    /// 状態名
    pub name: Symbol,
    
    /// 状態の述語
    pub predicate: Option<RefinementPredicate>,
    
    /// 状態変数セット
    pub variables: HashSet<Symbol>,
    
    /// 状態が有効か
    pub is_active: bool,
}

/// 状態遷移
#[derive(Debug, Clone)]
pub struct StateTransition {
    /// 遷移元状態
    pub from_state: Symbol,
    
    /// 遷移先状態
    pub to_state: Symbol,
    
    /// 遷移条件
    pub condition: Option<RefinementPredicate>,
    
    /// 遷移アクション
    pub action: Option<Symbol>,
}

/// 時相論理アサーション
#[derive(Debug, Clone)]
pub struct TemporalAssertion {
    /// 論理式
    pub formula: TemporalFormula,
    
    /// アサーション位置
    pub location: SourceLocation,
    
    /// 検証結果
    pub verified: Option<bool>,
}

/// 時相論理式
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TemporalFormula {
    /// 状態述語
    State(Symbol),
    
    /// 論理演算
    Logic {
        op: LogicalOp,
        operands: Vec<TemporalFormula>,
    },
    
    /// 時相演算子
    Temporal {
        op: TemporalOperator,
        formula: Box<TemporalFormula>,
    },
    
    /// 時相バイナリ演算子
    TemporalBinary {
        op: TemporalBinaryOp,
        left: Box<TemporalFormula>,
        right: Box<TemporalFormula>,
    },
}

/// 時相バイナリ演算子
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TemporalBinaryOp {
    /// Until
    Until,
    /// Release
    Release,
    /// Since
    Since,
    /// Triggered
    Triggered,
}

/// 状態履歴エントリ
#[derive(Debug, Clone)]
pub struct StateHistoryEntry {
    /// 現在状態
    pub current_state: Symbol,
    
    /// アクティブな述語
    pub active_predicates: Vec<RefinementPredicate>,
    
    /// 位置情報
    pub location: SourceLocation,
}

impl TypeCheckContext {
    /// 新しい型チェックコンテキストを作成
    pub fn new(
        type_registry: Arc<TypeRegistry>,
        strictness: TypeCheckStrictness,
    ) -> Self {
        let inference_level = match strictness {
            TypeCheckStrictness::Basic => InferenceOptimizationLevel::Basic,
            TypeCheckStrictness::Standard => InferenceOptimizationLevel::Standard,
            TypeCheckStrictness::Strict => InferenceOptimizationLevel::Advanced,
            TypeCheckStrictness::Quantum => InferenceOptimizationLevel::Quantum,
            TypeCheckStrictness::Temporal => InferenceOptimizationLevel::Temporal,
            TypeCheckStrictness::Complete => InferenceOptimizationLevel::Full,
        };
        
        let mut inference_context = InferenceContext::new(type_registry.clone());
        inference_context.set_optimization_level(inference_level);
        
        let mut ctx = Self {
            inference_context,
            checked_types: HashMap::new(),
            linear_vars: HashMap::new(),
            strictness,
            errors: Vec::new(),
            dependent_solver: DependentTypeSolver::new(),
            type_registry,
            quantum_checker: None,
            temporal_checker: None,
        };
        
        // 厳格さレベルに応じて特殊型チェッカーを初期化
        match strictness {
            TypeCheckStrictness::Quantum | TypeCheckStrictness::Complete => {
                ctx.initialize_quantum_checker();
            },
            _ => {}
        }
        
        match strictness {
            TypeCheckStrictness::Temporal | TypeCheckStrictness::Complete => {
                ctx.initialize_temporal_checker();
            },
            _ => {}
        }
        
        ctx
    }
    
    /// 量子型チェッカーを初期化
    fn initialize_quantum_checker(&mut self) {
        self.quantum_checker = Some(QuantumTypeChecker {
            qubits: HashMap::new(),
            circuits: HashMap::new(),
            operations: Vec::new(),
            gate_validator: QuantumGateValidator {
                basic_gates: self.initialize_basic_quantum_gates(),
                composite_gates: HashMap::new(),
            },
        });
    }
    
    /// 基本量子ゲートを初期化
    fn initialize_basic_quantum_gates(&self) -> HashSet<Symbol> {
        let mut gates = HashSet::new();
        
        // 基本量子ゲートを登録
        gates.insert(Symbol::intern("H"));  // Hadamard
        gates.insert(Symbol::intern("X"));  // Pauli-X
        gates.insert(Symbol::intern("Y"));  // Pauli-Y
        gates.insert(Symbol::intern("Z"));  // Pauli-Z
        gates.insert(Symbol::intern("CNOT"));  // Controlled-NOT
        gates.insert(Symbol::intern("CZ"));  // Controlled-Z
        gates.insert(Symbol::intern("S"));  // S phase
        gates.insert(Symbol::intern("T"));  // T phase
        gates.insert(Symbol::intern("RX"));  // X-rotation
        gates.insert(Symbol::intern("RY"));  // Y-rotation
        gates.insert(Symbol::intern("RZ"));  // Z-rotation
        gates.insert(Symbol::intern("SWAP"));  // SWAP
        gates.insert(Symbol::intern("MEASURE"));  // 測定
        
        gates
    }
    
    /// 時相型チェッカーを初期化
    fn initialize_temporal_checker(&mut self) {
        self.temporal_checker = Some(TemporalTypeChecker {
            states: HashMap::new(),
            transitions: HashMap::new(),
            temporal_assertions: Vec::new(),
            state_history: Vec::new(),
        });
    }
    
    /// 量子式を型チェック
    pub fn check_quantum_expr(&mut self, expr: &Expr, expected_type: Option<TypeId>) -> Result<TypeId> {
        if let Some(ref mut quantum_checker) = self.quantum_checker {
            // 量子式の型チェック処理
            // TODO: 実際の量子型チェックロジックを実装
            
            // 型推論コンテキストを使って量子型を推論
            let inferred_type = self.inference_context.infer_quantum_expr(expr)?;
            
            // 期待される型がある場合は、サブタイプ関係をチェック
            if let Some(expected) = expected_type {
                self.check_quantum_subtype(inferred_type, expected, expr.span)?;
            }
            
            // 推論された型を返す
            Ok(inferred_type)
        } else {
            // 量子型チェッカーが有効でない場合は通常の型チェックにフォールバック
            self.check_expr(expr, expected_type)
        }
    }
    
    /// 量子型のサブタイプ関係をチェック
    fn check_quantum_subtype(&mut self, sub_type: TypeId, super_type: TypeId, location: SourceLocation) -> Result<()> {
        // 量子型の互換性をチェック
        // TODO: 量子型のサブタイプ関係の詳細なチェックロジックを実装
        
        // 一時的に、単純な型一致チェックを実行
        if sub_type != super_type {
            self.add_error(
                format!("量子型の不一致: {:?} は {:?} のサブタイプではありません", 
                    self.type_registry.resolve(sub_type),
                    self.type_registry.resolve(super_type)),
                location,
                TypeCheckErrorKind::QuantumTypeError,
            );
            return Err(CompilerError::new(
                ErrorKind::TypeError("量子型の不一致".to_string()),
                location,
            ));
        }
        
        Ok(())
    }
    
    /// 時相式を型チェック
    pub fn check_temporal_expr(&mut self, expr: &Expr, expected_type: Option<TypeId>) -> Result<TypeId> {
        if let Some(ref mut temporal_checker) = self.temporal_checker {
            // 時相式の型チェック処理
            // TODO: 実際の時相型チェックロジックを実装
            
            // 型推論コンテキストを使って時相型を推論
            let inferred_type = self.inference_context.infer_temporal_expr(expr)?;
            
            // 期待される型がある場合は、サブタイプ関係をチェック
            if let Some(expected) = expected_type {
                self.check_temporal_subtype(inferred_type, expected, expr.span)?;
            }
            
            // 推論された型を返す
            Ok(inferred_type)
        } else {
            // 時相型チェッカーが有効でない場合は通常の型チェックにフォールバック
            self.check_expr(expr, expected_type)
        }
    }
    
    /// 時相型のサブタイプ関係をチェック
    fn check_temporal_subtype(&mut self, sub_type: TypeId, super_type: TypeId, location: SourceLocation) -> Result<()> {
        // 時相型の互換性をチェック
        // TODO: 時相型のサブタイプ関係の詳細なチェックロジックを実装
        
        // 一時的に、単純な型一致チェックを実行
        if sub_type != super_type {
            self.add_error(
                format!("時相型の不一致: {:?} は {:?} のサブタイプではありません", 
                    self.type_registry.resolve(sub_type),
                    self.type_registry.resolve(super_type)),
                location,
                TypeCheckErrorKind::TemporalTypeError,
            );
            return Err(CompilerError::new(
                ErrorKind::TypeError("時相型の不一致".to_string()),
                location,
            ));
        }
        
        Ok(())
    }
    
    /// 式の型チェック（拡張版：量子・時相対応）
    pub fn check_expr(&mut self, expr: &Expr, expected_type: Option<TypeId>) -> Result<TypeId> {
        // 式の種類に応じて適切な型チェックを実行
        match expr.kind {
            // 量子式の場合
            ExprKind::QuantumExpr { .. } => {
                return self.check_quantum_expr(expr, expected_type);
            },
            
            // 時相式の場合
            ExprKind::TemporalExpr { .. } => {
                return self.check_temporal_expr(expr, expected_type);
            },
            
            // その他の式は既存の型チェックを実行
            _ => {
                let type_id = self.inference_context.infer_expr(expr)?;
                
                if let Some(expected) = expected_type {
                    self.check_subtype(type_id, expected, expr.span)?;
                }
                
                Ok(type_id)
            }
        }
    }
    
    /// 変数を環境に追加
    pub fn add_variable(&mut self, name: Symbol, type_id: TypeId) {
        self.checked_types.insert(name, type_id);
        
        // 線形型の場合は使用状態を初期化
        let ty = self.type_registry.resolve(type_id);
        if let Type::Linear(_) = &*ty {
            self.linear_vars.insert(name, false); // 未使用
        }
    }
    
    /// 変数を環境から取得
    pub fn lookup_variable(&self, name: Symbol) -> Option<TypeId> {
        self.checked_types.get(&name).copied()
    }
    
    /// 線形型の変数の使用をマーク
    pub fn mark_linear_var_used(&mut self, name: Symbol) -> Result<()> {
        if let Some(used) = self.linear_vars.get_mut(&name) {
            if *used {
                // 既に使用済みの線形型変数の再利用はエラー
                return Err(CompilerError::new(
                    ErrorKind::TypeSystem,
                    format!("線形型変数'{}'が複数回使用されています", name),
                    SourceLocation::default(),
                ));
            }
            *used = true;
        }
        Ok(())
    }
    
    /// エラーを追加
    pub fn add_error(&mut self, message: String, location: SourceLocation, kind: TypeCheckErrorKind) {
        self.errors.push(TypeCheckError {
            message,
            location,
            kind,
        });
    }
    
    /// パターンの型チェック
    pub fn check_pattern(&mut self, pattern: &Pattern, expected_type: TypeId) -> Result<HashMap<Symbol, TypeId>> {
        let bindings = self.inference_context.check_pattern(pattern, expected_type)?;
        
        // 新しく束縛された変数を環境に追加
        for (name, type_id) in &bindings {
            self.add_variable(*name, *type_id);
        }
        
        Ok(bindings)
    }
    
    /// サブタイプ関係のチェック
    pub fn check_subtype(&mut self, sub_type: TypeId, super_type: TypeId, location: SourceLocation) -> Result<()> {
        // 型制約を追加
        self.inference_context.add_constraint(TypeConstraint::Subtype {
            sub: sub_type,
            sup: super_type,
            location,
        });
        
        // 制約が解決できればOK
        // （解決は後で一括して行うので、ここでは何もしない）
        Ok(())
    }
    
    /// 文の型チェック
    pub fn check_statement(&mut self, stmt: &Statement) -> Result<()> {
        match &stmt.kind {
            StatementKind::Let { pattern, type_annotation, initializer } => {
                // 初期化式の型を推論
                let init_type = self.check_expr(initializer, None)?;
                
                // 型注釈がある場合は一致を確認
                let expected_type = if let Some(type_expr) = type_annotation {
                    let annotated_type = self.resolve_type_expr(type_expr)?;
                    self.check_subtype(init_type, annotated_type, initializer.location)?;
                    annotated_type
                } else {
                    init_type
                };
                
                // パターンの型チェック
                self.check_pattern(pattern, expected_type)?;
                
                Ok(())
            },
            
            StatementKind::Expression(expr) => {
                // 式の型チェック（期待型なし）
                self.check_expr(expr, None)?;
                Ok(())
            },
            
            StatementKind::Return(expr) => {
                // TODO: 関数の戻り値型との一致を確認
                if let Some(e) = expr {
                    self.check_expr(e, None)?;
                }
                Ok(())
            },
            
            // 他の文のチェックも必要に応じて実装
            
            _ => {
                // 未対応の文型は一旦無視
                Ok(())
            }
        }
    }
    
    /// 型式を解決して型IDを取得
    pub fn resolve_type_expr(&mut self, type_expr: &Expr) -> Result<TypeId> {
        // TODO: 型式の評価（現在は仮実装）
        Err(CompilerError::new(
            ErrorKind::TypeSystem,
            format!("型式の解決はまだ実装されていません"),
            type_expr.location,
        ))
    }
    
    /// コード終了時の線形型変数使用確認
    pub fn check_linear_vars_consumed(&mut self) -> Result<()> {
        for (name, used) in &self.linear_vars {
            if !used {
                // 未使用の線形型変数があればエラー
                self.add_error(
                    format!("線形型変数'{}'が使用されていません", name),
                    SourceLocation::default(),
                    TypeCheckErrorKind::LinearityViolation,
                );
            }
        }
        
        // エラーがなければOK
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(CompilerError::new(
                ErrorKind::TypeSystem,
                format!("型チェックエラーが発生しました"),
                SourceLocation::default(),
            ))
        }
    }
    
    /// 制約解決を実行
    pub fn solve_constraints(&mut self) -> Result<()> {
        self.inference_context.solve_constraints()?;
        
        // 線形型の使用確認
        self.check_linear_vars_consumed()?;
        
        Ok(())
    }
    
    /// エラーのリストを取得
    pub fn get_errors(&self) -> &[TypeCheckError] {
        &self.errors
    }
}

/// 型チェッカー
pub struct TypeChecker {
    /// 型チェックコンテキスト
    context: TypeCheckContext,
}

impl TypeChecker {
    /// 新しい型チェッカーを作成
    pub fn new(
        type_registry: Arc<TypeRegistry>,
        strictness: TypeCheckStrictness,
    ) -> Self {
        Self {
            context: TypeCheckContext::new(type_registry, strictness),
        }
    }
    
    /// 式の型チェック
    pub fn check_expr(&mut self, expr: &Expr, expected_type: Option<TypeId>) -> Result<TypeId> {
        self.context.check_expr(expr, expected_type)
    }
    
    /// 文の型チェック
    pub fn check_statement(&mut self, stmt: &Statement) -> Result<()> {
        self.context.check_statement(stmt)
    }
    
    /// 文のシーケンスを型チェック
    pub fn check_statements(&mut self, stmts: &[Statement]) -> Result<()> {
        for stmt in stmts {
            self.check_statement(stmt)?;
        }
        Ok(())
    }
    
    /// 制約解決
    pub fn solve_constraints(&mut self) -> Result<()> {
        self.context.solve_constraints()
    }
    
    /// エラーのリストを取得
    pub fn get_errors(&self) -> &[TypeCheckError] {
        self.context.get_errors()
    }
} 
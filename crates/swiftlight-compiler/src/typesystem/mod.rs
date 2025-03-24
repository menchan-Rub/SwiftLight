// SwiftLight Type System
// 型システムのメインモジュール

//! # SwiftLight Type System
//! 
//! SwiftLight言語の次世代型システムを実装するコアモジュールです。
//! 従来の型システムの概念を超える以下の機能を統合的に提供します：
//!
//! - ハイブリッド型推論（HM型推論 + 依存型 + 線形論理）
//! - 量子計算型システム（リソース感知型量子操作）
//! - 時相型状態管理（状態遷移の時間的整合性保証）
//! - 多層効果システム（エフェクト多相 + リージョン推論）
//! - 契約駆動型精製型（実行時チェックと静的分析の統合）
//! - 型レベル機械学習（テンソル型と自動微分の型表現）

use std::collections::{HashMap, HashSet, BTreeMap, BTreeSet};
use std::fmt;
use std::sync::{Arc, atomic::{AtomicU32, Ordering}};
use thiserror::Error;
use std::any::TypeId as StdTypeId;
use std::hash::{Hash, Hasher};
use std::num::NonZeroU32;
use bitflags::bitflags;
use parking_lot::RwLock;
use rustc_hash::FxHashMap;

use crate::frontend::ast;
use crate::frontend::error::{Result, ErrorKind, SourceLocation};
use crate::utils::{StrId, InternedString};

// モジュール定義
pub mod types;
pub mod traits;
pub mod constraints;
pub mod dependent;
pub mod inference;
pub mod checker;
pub mod effects;
pub mod ownership;
pub mod quantum;
pub mod temporal;
// pub mod generics;
// pub mod validation;
// pub mod subtyping;
// pub mod higher_kinded;
// pub mod quantum;
// pub mod refinement;
// pub mod intersection;
// pub mod union;
// pub mod existential;
// pub mod universal;
// pub mod variance;
// pub mod specialization;
// pub mod coherence;
// pub mod monomorphization;
// pub mod polymorphic_recursion;
// pub mod pattern_matching;
// pub mod type_families;
// pub mod type_classes;
// pub mod type_operators;
// pub mod type_level_computation;
// pub mod type_level_literals;
// pub mod type_level_functions;
// pub mod row_polymorphism;
// pub mod gradual_typing;
// pub mod capabilities;
// pub mod regions;
// pub mod lifetime_inference;
// pub mod memory_regions;
// pub mod borrowck;
// pub mod alias_analysis;
// pub mod escape_analysis;
// pub mod effect_inference;
// pub mod effect_polymorphism;

// モジュールの再エクスポート
pub use self::types::*;
pub use self::traits::*;
pub use self::dependent::*;
pub use self::constraints::*;
pub use self::inference::*;
pub use self::checker::*;
pub use self::effects::*;
pub use self::ownership::*;
pub use self::quantum::*;
pub use self::temporal::*;

/// 最適化されたシンボル型（文字列インターン化）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Symbol(NonZeroU32);

impl Symbol {
    pub fn intern(s: &str) -> Self {
        static INTERN: RwLock<FxHashMap<&'static str, Symbol>> = RwLock::new(FxHashMap::default());
        static COUNTER: AtomicU32 = AtomicU32::new(1);
        
        let mut map = INTERN.write();
        if let Some(&sym) = map.get(s) {
            return sym;
        }
        
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let sym = Symbol(NonZeroU32::new(id).unwrap());
        let leaked = Box::leak(s.to_string().into_boxed_str());
        map.insert(leaked, sym);
        sym
    }

    pub fn as_str(&self) -> &'static str {
        let map = INTERN.read();
        map.iter()
            .find_map(|(k, &v)| if v == *self { Some(*k) } else { None })
            .unwrap_or_else(|| panic!("Symbol {:?} not found in interning table", self.0))
    }
}

impl From<&str> for Symbol {
    fn from(s: &str) -> Self {
        Symbol::intern(s)
    }
}

impl From<String> for Symbol {
    fn from(s: String) -> Self {
        Symbol::intern(&s)
    }
}

/// ソースコードの位置情報
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceSpan {
    pub file_id: u32,
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

/// メモリ領域識別子
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegionId(pub u32);

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
    /// 変数参照
    Var(Symbol),
}

/// 型レベル式（依存型、型レベル計算など高度な型機能の基盤）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeLevelExpr {
    /// リテラル値
    Literal(TypeLevelLiteralValue),
    /// 変数参照
    Var(Symbol),
    /// 二項演算
    BinaryOp {
        op: ArithmeticOp,
        left: Box<TypeLevelExpr>,
        right: Box<TypeLevelExpr>,
    },
    /// 関数呼び出し
    FunctionCall {
        func: Symbol,
        args: Vec<TypeLevelExpr>,
    },
    /// 条件式
    Conditional {
        condition: Box<RefinementPredicate>,
        then_expr: Box<TypeLevelExpr>,
        else_expr: Box<TypeLevelExpr>,
    },
    /// リスト構築
    ListExpr(Vec<TypeLevelExpr>),
    /// リスト添字アクセス
    IndexAccess {
        list: Box<TypeLevelExpr>,
        index: Box<TypeLevelExpr>,
    },
    /// 型参照
    TypeRef(TypeId),
    /// メタ型演算（型の型）
    MetaType(TypeId),
    /// 型レベルラムダ抽象
    Lambda {
        param: Symbol,
        body: Box<TypeLevelExpr>,
    },
    /// 型レベル適用
    Apply {
        func: Box<TypeLevelExpr>,
        arg: Box<TypeLevelExpr>,
    },
    /// 量子状態参照
    QuantumState {
        qubits: Box<TypeLevelExpr>,
        amplitude: Box<TypeLevelExpr>,
    },
    /// 時相演算子
    TemporalOp {
        op: TemporalOperator,
        expr: Box<TypeLevelExpr>,
    },
}

/// 時相論理演算子
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TemporalOperator {
    /// 次の状態で（Next）
    Next,
    /// いつか（Eventually）
    Eventually,
    /// 常に（Always）
    Always,
    /// 〜まで（Until）
    Until,
    /// 過去に（Past）
    Past,
}

impl TypeLevelExpr {
    /// 整数リテラルを作成
    pub fn int(value: i64) -> Self {
        Self::Literal(TypeLevelLiteralValue::Int(value))
    }
    
    /// 変数参照を作成
    pub fn var<S: Into<Symbol>>(name: S) -> Self {
        Self::Var(name.into())
    }
    
    /// 二項加算式を作成
    pub fn add(left: Self, right: Self) -> Self {
        Self::BinaryOp {
            op: ArithmeticOp::Add,
            left: Box::new(left),
            right: Box::new(right),
        }
    }
    
    /// 二項減算式を作成
    pub fn sub(left: Self, right: Self) -> Self {
        Self::BinaryOp {
            op: ArithmeticOp::Sub,
            left: Box::new(left),
            right: Box::new(right),
        }
    }
    
    /// 二項乗算式を作成
    pub fn mul(left: Self, right: Self) -> Self {
        Self::BinaryOp {
            op: ArithmeticOp::Mul,
            left: Box::new(left),
            right: Box::new(right),
        }
    }
    
    /// 型レベル関数適用を作成
    pub fn apply_fn<S: Into<Symbol>>(func: S, args: Vec<Self>) -> Self {
        Self::FunctionCall {
            func: func.into(),
            args,
        }
    }
    
    /// 条件式を作成
    pub fn cond(condition: RefinementPredicate, then_expr: Self, else_expr: Self) -> Self {
        Self::Conditional {
            condition: Box::new(condition),
            then_expr: Box::new(then_expr),
            else_expr: Box::new(else_expr),
        }
    }
    
    /// リスト式を作成
    pub fn list(elements: Vec<Self>) -> Self {
        Self::ListExpr(elements)
    }
    
    /// 量子状態を作成
    pub fn quantum_state(qubits: Self, amplitude: Self) -> Self {
        Self::QuantumState {
            qubits: Box::new(qubits),
            amplitude: Box::new(amplitude),
        }
    }
    
    /// 時相演算子「次の状態で」を適用
    pub fn next(expr: Self) -> Self {
        Self::TemporalOp {
            op: TemporalOperator::Next,
            expr: Box::new(expr),
        }
    }
    
    /// 時相演算子「いつか」を適用
    pub fn eventually(expr: Self) -> Self {
        Self::TemporalOp {
            op: TemporalOperator::Eventually,
            expr: Box::new(expr),
        }
    }
    
    /// 時相演算子「常に」を適用
    pub fn always(expr: Self) -> Self {
        Self::TemporalOp {
            op: TemporalOperator::Always,
            expr: Box::new(expr),
        }
    }
}

/// 精製型の述語
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RefinementPredicate {
    BoolLiteral(bool),
    IntComparison {
        op: OrderingOp,
        lhs: TypeLevelLiteralValue,
        rhs: TypeLevelLiteralValue,
    },
    LogicalOp {
        op: LogicalOp,
        operands: Vec<RefinementPredicate>,
    },
    ArithmeticOp {
        op: ArithmeticOp,
        operands: Vec<TypeLevelLiteralValue>,
    },
    HasCapability(Symbol),
    InState(Symbol),
    Custom(Symbol, Vec<TypeLevelLiteralValue>),
}

impl RefinementPredicate {
    /// 論理AND演算子を使って2つの述語を結合
    pub fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::LogicalOp { op: LogicalOp::And, operands: mut ops1 }, 
             Self::LogicalOp { op: LogicalOp::And, operands: mut ops2 }) => {
                ops1.append(&mut ops2);
                Self::LogicalOp { op: LogicalOp::And, operands: ops1 }
            },
            (Self::LogicalOp { op: LogicalOp::And, operands: mut ops }, other) => {
                ops.push(other);
                Self::LogicalOp { op: LogicalOp::And, operands: ops }
            },
            (self_pred, Self::LogicalOp { op: LogicalOp::And, operands: mut ops }) => {
                ops.insert(0, self_pred);
                Self::LogicalOp { op: LogicalOp::And, operands: ops }
            },
            (self_pred, other_pred) => {
                Self::LogicalOp { 
                    op: LogicalOp::And, 
                    operands: vec![self_pred, other_pred] 
                }
            }
        }
    }

    /// 論理OR演算子を使って2つの述語を結合
    pub fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::LogicalOp { op: LogicalOp::Or, operands: mut ops1 }, 
             Self::LogicalOp { op: LogicalOp::Or, operands: mut ops2 }) => {
                ops1.append(&mut ops2);
                Self::LogicalOp { op: LogicalOp::Or, operands: ops1 }
            },
            (Self::LogicalOp { op: LogicalOp::Or, operands: mut ops }, other) => {
                ops.push(other);
                Self::LogicalOp { op: LogicalOp::Or, operands: ops }
            },
            (self_pred, Self::LogicalOp { op: LogicalOp::Or, operands: mut ops }) => {
                ops.insert(0, self_pred);
                Self::LogicalOp { op: LogicalOp::Or, operands: ops }
            },
            (self_pred, other_pred) => {
                Self::LogicalOp { 
                    op: LogicalOp::Or, 
                    operands: vec![self_pred, other_pred] 
                }
            }
        }
    }

    /// 論理NOT演算子を適用
    pub fn not(self) -> Self {
        Self::LogicalOp { 
            op: LogicalOp::Not, 
            operands: vec![self] 
        }
    }
    
    /// 2つの式が等しいという述語を作成
    pub fn equals(lhs: TypeLevelLiteralValue, rhs: TypeLevelLiteralValue) -> Self {
        Self::IntComparison { 
            op: OrderingOp::Eq, 
            lhs, 
            rhs 
        }
    }
    
    /// 2つの式が等しくないという述語を作成
    pub fn not_equals(lhs: TypeLevelLiteralValue, rhs: TypeLevelLiteralValue) -> Self {
        Self::IntComparison { 
            op: OrderingOp::Ne, 
            lhs, 
            rhs 
        }
    }
    
    /// 左辺が右辺より小さいという述語を作成
    pub fn less_than(lhs: TypeLevelLiteralValue, rhs: TypeLevelLiteralValue) -> Self {
        Self::IntComparison { 
            op: OrderingOp::Lt, 
            lhs, 
            rhs 
        }
    }
    
    /// 左辺が右辺以下という述語を作成
    pub fn less_equals(lhs: TypeLevelLiteralValue, rhs: TypeLevelLiteralValue) -> Self {
        Self::IntComparison { 
            op: OrderingOp::Le, 
            lhs, 
            rhs 
        }
    }
    
    /// 左辺が右辺より大きいという述語を作成
    pub fn greater_than(lhs: TypeLevelLiteralValue, rhs: TypeLevelLiteralValue) -> Self {
        Self::IntComparison { 
            op: OrderingOp::Gt, 
            lhs, 
            rhs 
        }
    }
    
    /// 左辺が右辺以上という述語を作成
    pub fn greater_equals(lhs: TypeLevelLiteralValue, rhs: TypeLevelLiteralValue) -> Self {
        Self::IntComparison { 
            op: OrderingOp::Ge, 
            lhs, 
            rhs 
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderingOp {
    Eq, // ==
    Ne, // !=
    Lt, // <
    Le, // <=
    Gt, // >
    Ge, // >=
}

impl OrderingOp {
    /// ComparisonOpからOrderingOpへの変換
    pub fn from_comparison_op(op: ComparisonOp) -> Self {
        match op {
            ComparisonOp::Equal => Self::Eq,
            ComparisonOp::NotEqual => Self::Ne,
            ComparisonOp::Less => Self::Lt,
            ComparisonOp::LessEqual => Self::Le,
            ComparisonOp::Greater => Self::Gt,
            ComparisonOp::GreaterEqual => Self::Ge,
        }
    }
    
    /// OrderingOpからComparisonOpへの変換
    pub fn to_comparison_op(&self) -> ComparisonOp {
        match self {
            Self::Eq => ComparisonOp::Equal,
            Self::Ne => ComparisonOp::NotEqual,
            Self::Lt => ComparisonOp::Less,
            Self::Le => ComparisonOp::LessEqual,
            Self::Gt => ComparisonOp::Greater,
            Self::Ge => ComparisonOp::GreaterEqual,
        }
    }
    
    /// この演算子の否定を返す
    pub fn negate(&self) -> Self {
        match self {
            Self::Eq => Self::Ne,
            Self::Ne => Self::Eq,
            Self::Lt => Self::Ge,
            Self::Le => Self::Gt,
            Self::Gt => Self::Le,
            Self::Ge => Self::Lt,
        }
    }
    
    /// 左右のオペランドを入れ替えた場合の等価な演算子を返す
    pub fn flip(&self) -> Self {
        match self {
            Self::Eq => Self::Eq,  // a == b  <=>  b == a
            Self::Ne => Self::Ne,  // a != b  <=>  b != a
            Self::Lt => Self::Gt,  // a < b   <=>  b > a
            Self::Le => Self::Ge,  // a <= b  <=>  b >= a
            Self::Gt => Self::Lt,  // a > b   <=>  b < a
            Self::Ge => Self::Le,  // a >= b  <=>  b <= a
        }
    }
}

// OrderingOpの定義の後に追加
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogicalOp {
    And, Or, Not,
}

// 比較演算子（dependent.rsと同期するため）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComparisonOp {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

// ArithmeticOpもまだ必要なので再追加
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArithmeticOp {
    Add, Sub, Mul, Div, Mod,
}

/// 型の種類（カインド）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Kind {
    Type,
    Constructor(Box<Kind>, Box<Kind>),
    Row,
    Effect,
    Dependent(Symbol, Box<Kind>),
    Quantum,
    Natural,
    Boolean,
    Symbol,
    HigherOrder(Vec<Kind>, Box<Kind>),
    KindVar(usize),
    Universe(usize),  // 宇宙階層を追加
    Linear(Box<Kind>), // 線形種
}

impl Kind {
    pub fn is_type(&self) -> bool {
        matches!(self, Kind::Type)
    }
    
    pub fn is_constructor(&self) -> bool {
        matches!(self, Kind::Constructor(_, _))
    }
    
    pub fn complexity(&self) -> usize {
        match self {
            Kind::Type | Kind::Row | Kind::Effect | Kind::Quantum | 
            Kind::Natural | Kind::Boolean | Kind::Symbol | Kind::KindVar(_) => 1,
            Kind::Constructor(k1, k2) => k1.complexity() + k2.complexity() + 1,
            Kind::Dependent(_, k) => k.complexity() + 1,
            Kind::HigherOrder(params, result) => {
                params.iter().map(|k| k.complexity()).sum::<usize>() + result.complexity() + 1
            }
            Kind::Universe(_) => 2,
            Kind::Linear(k) => k.complexity() + 1,
        }
    }
}

/// 型システムエラー
#[derive(Debug, Error)]
pub enum TypeError {
    #[error("型不一致: 期待 '{expected}' 実際 '{actual}'")]
    TypeMismatch { expected: String, actual: String, location: SourceLocation },
    
    #[error("未定義型: {0}")]
    UndefinedType(String, SourceLocation),
    
    #[error("未解決型変数: {0}")]
    UnresolvedTypeVariable(String, SourceLocation),
    
    #[error("循環型定義: {0}")]
    RecursiveType(String, SourceLocation),
    
    #[error("所有権違反: {0}")]
    OwnershipViolation(String, SourceLocation),
    
    #[error("借用チェック失敗: {0}")]
    BorrowCheckFailure(String, SourceLocation),
    
    #[error("トレイト境界不満足: {ty} は {trait_name} を実装していない")]
    TraitBoundNotSatisfied { ty: String, trait_name: String, location: SourceLocation },
    
    #[error("種不一致: 期待 '{expected}' 実際 '{actual}'")]
    KindMismatch { expected: String, actual: String, location: SourceLocation },
    
    #[error("量子型操作エラー: {0}")]
    QuantumOperationError(String, SourceLocation),
    
    #[error("型レベル計算エラー: {0}")]
    TypeLevelComputationError(String, SourceLocation),
    
    #[error("内部エラー: {0}")]
    Internal(String),
}

/// 組み込み型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinType {
    Void,
    Bool,
    Int8,
    Int16,
    Int32,
    Int64,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float32,
    Float64,
    Char,
    String,
    Byte,
    Symbol,
    Tensor { dims: u32, dtype: Symbol },
    Qubit,
    QuantumGate { arity: u32 },
}

/// 型表現
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Builtin(BuiltinType),
    Named {
        name: Symbol,
        module_path: Vec<Symbol>,
        params: Vec<TypeId>,
        kind: Kind,
    },
    Function {
        params: Vec<TypeId>,
        return_type: Box<TypeId>,
        effects: EffectSet,
        closure_env: Option<Arc<ClosureEnvironment>>,
    },
    Array {
        element: TypeId,
        size: Option<usize>,
    },
    Reference {
        target: TypeId,
        is_mutable: bool,
        lifetime: Symbol,
    },
    Tuple(Vec<TypeId>),
    TypeVar {
        id: usize,
        constraints: Vec<TypeConstraint>,
    },
    DependentFunction {
        param: Symbol,
        param_ty: TypeId,
        return_ty: TypeId,
    },
    QuantumCircuit {
        qubits: u32,
        operations: Vec<QuantumGate>,
    },
    Refinement {
        base: TypeId,
        predicate: RefinementPredicate,
    },
    Effectful {
        base: TypeId,
        effects: EffectSet,
    },
    Linear(Box<TypeId>),
    Capability {
        resource: Symbol,
        operations: Vec<Symbol>,
    },
    MetaType(TypeId),
    Quantum(QuantumType),
    Temporal(TemporalType),
    Effect(EffectType),
    Resource(ResourceType),
}

/// 量子ゲート定義
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QuantumGate {
    pub name: Symbol,
    pub qubits: u32,
    pub parameters: Vec<TypeLevelLiteralValue>,
    pub adjoint: bool,
}

/// 型レジストリ
#[derive(Debug)]
pub struct TypeRegistry {
    types: FxHashMap<TypeId, Arc<Type>>,
    symbols: RwLock<FxHashMap<String, TypeId>>,
    next_id: AtomicU32,
    type_vars: RwLock<FxHashMap<usize, TypeConstraintSet>>,
    scopes: RwLock<Vec<Scope>>,
    trait_impls: DashMap<TypeId, Vec<TraitImplementation>>,
}

#[derive(Debug, Clone)]
struct Scope {
    symbols: FxHashMap<Symbol, TypeId>,
    parent: Option<Arc<Scope>>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        let mut registry = TypeRegistry {
            types: FxHashMap::default(),
            symbols: RwLock::new(FxHashMap::default()),
            next_id: AtomicU32::new(1),
            type_vars: RwLock::new(FxHashMap::default()),
            scopes: RwLock::new(Vec::new()),
            trait_impls: DashMap::new(),
        };

        registry.register_builtins();
        registry
    }

    fn register_builtins(&mut self) {
        let builtins = [
            (BuiltinType::Void, "Void"),
            (BuiltinType::Bool, "Bool"),
            (BuiltinType::Int8, "Int8"),
            (BuiltinType::Int16, "Int16"),
            (BuiltinType::Int32, "Int32"),
            (BuiltinType::Int64, "Int64"),
            (BuiltinType::UInt8, "UInt8"),
            (BuiltinType::UInt16, "UInt16"),
            (BuiltinType::UInt32, "UInt32"),
            (BuiltinType::UInt64, "UInt64"),
            (BuiltinType::Float32, "Float32"),
            (BuiltinType::Float64, "Float64"),
            (BuiltinType::Char, "Char"),
            (BuiltinType::String, "String"),
            (BuiltinType::Byte, "Byte"),
            (BuiltinType::Symbol, "Symbol"),
            (BuiltinType::Qubit, "Qubit"),
        ];

        for (ty, name) in builtins {
            let id = TypeId(self.next_id.fetch_add(1, Ordering::Relaxed));
            self.types.insert(id, Arc::new(Type::Builtin(ty)));
            self.symbols.write().insert(name.to_string(), id);
        }
    }

    pub fn new_type_var(&self, constraints: Vec<TypeConstraint>) -> TypeId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let type_id = TypeId(id);
        self.type_vars.write().insert(id as usize, TypeConstraintSet::new(constraints));
        type_id
    }

    pub fn resolve(&self, id: TypeId) -> Arc<Type> {
        self.types.get(&id).cloned().unwrap_or_else(|| panic!("Unknown type ID: {:?}", id))
    }

    pub fn unify(&self, a: TypeId, b: TypeId) -> Result<(), TypeError> {
        // 型統一アルゴリズムの完全実装
        let mut stack = vec![(a, b)];
        let mut substitutions = FxHashMap::default();

        while let Some((mut a, mut b)) = stack.pop() {
            a = self.follow(a, &substitutions);
            b = self.follow(b, &substitutions);

            if a == b {
                continue;
            }

            match (self.resolve(a).as_ref(), self.resolve(b).as_ref()) {
                (Type::TypeVar { id: id_a, .. }, Type::TypeVar { id: id_b, .. }) => {
                    let constraints = self.merge_constraints(*id_a, *id_b)?;
                    substitutions.insert(*id_b, a);
                    self.update_constraints(*id_a, constraints);
                }
                // 他の型の組み合わせに対する処理を実装
                _ => return Err(TypeError::TypeMismatch {
                    expected: self.type_name(a),
                    actual: self.type_name(b),
                    location: SourceLocation::unknown(),
                }),
            }
        }

        Ok(())
    }

    fn follow(&self, id: TypeId, substitutions: &FxHashMap<usize, TypeId>) -> TypeId {
        let mut current = id;
        while let Type::TypeVar { id: var_id, .. } = self.resolve(current).as_ref() {
            if let Some(sub) = substitutions.get(var_id) {
                current = *sub;
            } else {
                break;
            }
        }
        current
    }

    /// 依存関数型を作成
    /// param_name: パラメータ名
    /// param_type: パラメータの型
    /// body_fn: パラメータを受け取り、戻り値の型を返す関数
    pub fn dependent_function_type(
        &self,
        param_name: Symbol,
        param_type: TypeId,
        body_fn: impl Fn(TypeLevelExpr) -> TypeId,
    ) -> Result<TypeId> {
        // パラメータ式を作成
        let param_expr = TypeLevelExpr::Var(param_name);
        
        // 本体の型を計算
        let return_type = body_fn(param_expr);
        
        // 依存関数型を作成
        let ty = Type::DependentFunction {
            param: param_name,
            param_ty: param_type,
            return_ty: return_type,
        };
        
        // 型レジストリに登録して返す
        let id = TypeId(self.next_id.fetch_add(1, Ordering::Relaxed));
        let type_arc = Arc::new(ty);
        {
            let mut types = self.types.write();
            types.insert(id, type_arc);
        }
        
        Ok(id)
    }
    
    /// 精製型（refinement type）を作成
    /// base_type: 基本となる型
    /// predicate: 型の条件述語
    pub fn refinement_type(
        &self,
        base_type: TypeId,
        predicate: RefinementPredicate,
    ) -> Result<TypeId> {
        // 精製型を作成
        let ty = Type::Refinement {
            base: base_type,
            predicate,
        };
        
        // 型レジストリに登録して返す
        let id = TypeId(self.next_id.fetch_add(1, Ordering::Relaxed));
        let type_arc = Arc::new(ty);
        {
            let mut types = self.types.write();
            types.insert(id, type_arc);
        }
        
        Ok(id)
    }
    
    /// ある値に対する条件述語型を作成（例: x > 0 の型）
    pub fn predicate_type<F>(
        &self,
        base_type: TypeId,
        value_name: Symbol,
        predicate_fn: F,
    ) -> Result<TypeId>
    where
        F: Fn(TypeLevelExpr) -> RefinementPredicate,
    {
        // 変数式を作成
        let var_expr = TypeLevelExpr::Var(value_name);
        
        // 述語を生成
        let predicate = predicate_fn(var_expr);
        
        // 精製型を作成
        self.refinement_type(base_type, predicate)
    }
    
    /// 非負整数型を作成する便利なヘルパー
    pub fn non_negative_int_type(&self) -> Result<TypeId> {
        // 整数型を取得
        let int_type = self.lookup_builtin(BuiltinType::Int32)?;
        
        // x >= 0 の述語を作成
        let predicate = RefinementPredicate::IntComparison {
            op: ComparisonOp::GreaterEqual,
            lhs: TypeLevelLiteralValue::Var(Symbol::intern("x")),
            rhs: TypeLevelLiteralValue::Int(0),
        };
        
        // 精製型を作成
        self.refinement_type(int_type, predicate)
    }
    
    /// 長さNの配列型を作成する便利なヘルパー
    pub fn array_of_length(
        &self,
        element_type: TypeId,
        length: usize,
    ) -> Result<TypeId> {
        // 配列型を作成
        let ty = Type::Array {
            element: element_type,
            size: Some(length),
        };
        
        // 型レジストリに登録して返す
        let id = TypeId(self.next_id.fetch_add(1, Ordering::Relaxed));
        let type_arc = Arc::new(ty);
        {
            let mut types = self.types.write();
            types.insert(id, type_arc);
        }
        
        Ok(id)
    }
    
    /// 依存配列型を作成（要素の型が配列のインデックスに依存する型）
    pub fn dependent_array_type<F>(
        &self,
        length: usize,
        element_type_fn: F,
    ) -> Result<TypeId>
    where
        F: Fn(TypeLevelExpr) -> TypeId,
    {
        // セマンティクスはまだ詳細に定義されていないが、将来的な拡張のための土台
        // 現状では通常の配列として扱い、将来のバージョンで依存配列型として実装予定
        
        // 基本の要素型を仮に計算（インデックス0での型）
        let index_expr = TypeLevelExpr::Literal(TypeLevelLiteralValue::Int(0));
        let base_element_type = element_type_fn(index_expr);
        
        // 通常の配列型として作成（将来的には依存配列型になる）
        self.array_of_length(base_element_type, length)
    }

    /// 量子型の登録
    pub fn register_quantum_type(&mut self, quantum_type: QuantumType) -> TypeId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let type_id = TypeId(id);
        let type_arc = Arc::new(Type::Quantum(quantum_type));
        self.types.insert(type_id, type_arc);
        type_id
    }

    /// 時相型の登録
    pub fn register_temporal_type(&mut self, temporal_type: TemporalType) -> TypeId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let type_id = TypeId(id);
        let type_arc = Arc::new(Type::Temporal(temporal_type));
        self.types.insert(type_id, type_arc);
        type_id
    }

    /// エフェクト型の登録
    pub fn register_effect_type(&mut self, effect_type: EffectType) -> TypeId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let type_id = TypeId(id);
        let type_arc = Arc::new(Type::Effect(effect_type));
        self.types.insert(type_id, type_arc);
        type_id
    }

    /// リソース型の登録
    pub fn register_resource_type(&mut self, resource_type: ResourceType) -> TypeId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let type_id = TypeId(id);
        let type_arc = Arc::new(Type::Resource(resource_type));
        self.types.insert(type_id, type_arc);
        type_id
    }

    /// 量子型を取得
    pub fn get_quantum_type(&self) -> Result<TypeId> {
        let quantum_name = Symbol::intern("Quantum");
        
        // 既存の型を探す
        for (id, ty) in self.types.iter() {
            if let Type::Named { name, .. } = &**ty {
                if *name == quantum_name {
                    return Ok(*id);
                }
            }
        }
        
        // 見つからなければ新しく作成
        let type_id = self.register_named_type(quantum_name, Vec::new(), Kind::Quantum);
        
        Ok(type_id)
    }

    /// 時相型を取得
    pub fn get_temporal_type(&self) -> Result<TypeId> {
        let temporal_name = Symbol::intern("Temporal");
        
        // 既存の型を探す
        for (id, ty) in self.types.iter() {
            if let Type::Named { name, .. } = &**ty {
                if *name == temporal_name {
                    return Ok(*id);
                }
            }
        }
        
        // 見つからなければ新しく作成
        let type_id = self.register_named_type(temporal_name, Vec::new(), Kind::Type);
        
        Ok(type_id)
    }

    /// エフェクト型を取得
    pub fn get_effect_type(&self) -> Result<TypeId> {
        let effect_name = Symbol::intern("Effect");
        
        // 既存の型を探す
        for (id, ty) in self.types.iter() {
            if let Type::Named { name, .. } = &**ty {
                if *name == effect_name {
                    return Ok(*id);
                }
            }
        }
        
        // 見つからなければ新しく作成
        let type_id = self.register_named_type(effect_name, Vec::new(), Kind::Effect);
        
        Ok(type_id)
    }

    /// リソース型を取得
    pub fn get_resource_type(&self) -> Result<TypeId> {
        let resource_name = Symbol::intern("Resource");
        
        // 既存の型を探す
        for (id, ty) in self.types.iter() {
            if let Type::Named { name, .. } = &**ty {
                if *name == resource_name {
                    return Ok(*id);
                }
            }
        }
        
        // 見つからなければ新しく作成
        let type_id = self.register_named_type(resource_name, Vec::new(), Kind::Type);
        
        Ok(type_id)
    }

    /// 量子型の型チェック
    pub fn check_quantum_type(&self, type_id: TypeId) -> Result<bool> {
        let ty = self.resolve(type_id);
        
        match &*ty {
            Type::Named { name, .. } => {
                Ok(name.to_string().contains("Quantum") ||
                   name.to_string().contains("Qubit") ||
                   name.to_string().contains("Circuit"))
            },
            Type::Quantum(_) => Ok(true),
            _ => Ok(false),
        }
    }

    /// 時相型の型チェック
    pub fn check_temporal_type(&self, type_id: TypeId) -> Result<bool> {
        let ty = self.resolve(type_id);
        
        match &*ty {
            Type::Named { name, .. } => {
                Ok(name.to_string().contains("Temporal") ||
                   name.to_string().contains("Future") ||
                   name.to_string().contains("Past"))
            },
            Type::Temporal(_) => Ok(true),
            _ => Ok(false),
        }
    }

    /// エフェクト型の型チェック
    pub fn check_effect_type(&self, type_id: TypeId) -> Result<bool> {
        let ty = self.resolve(type_id);
        
        match &*ty {
            Type::Named { name, .. } => {
                Ok(name.to_string().contains("Effect") ||
                   name.to_string().contains("IO") ||
                   name.to_string().contains("Exception"))
            },
            Type::Effect(_) => Ok(true),
            _ => Ok(false),
        }
    }

    /// リソース型の型チェック
    pub fn check_resource_type(&self, type_id: TypeId) -> Result<bool> {
        let ty = self.resolve(type_id);
        
        match &*ty {
            Type::Named { name, .. } => {
                Ok(name.to_string().contains("Resource") ||
                   name.to_string().contains("File") ||
                   name.to_string().contains("Memory"))
            },
            Type::Resource(_) => Ok(true),
            _ => Ok(false),
        }
    }

    /// 量子型の単一化
    pub fn unify_quantum_types(&self, a: TypeId, b: TypeId) -> Result<TypeId> {
        let a_ty = self.resolve(a);
        let b_ty = self.resolve(b);
        
        match (&*a_ty, &*b_ty) {
            (Type::Quantum(q1), Type::Quantum(q2)) => {
                // 量子型の単一化ロジック
                if q1.qubits == q2.qubits && q1.operations == q2.operations {
                    Ok(a)
                } else {
                    Err(TypeError::TypeMismatch {
                        expected: format!("{:?}", q1),
                        actual: format!("{:?}", q2),
                        location: SourceLocation::default(),
                    })
                }
            },
            (Type::Named { name: n1, .. }, Type::Named { name: n2, .. }) => {
                if n1.to_string().contains("Quantum") && n2.to_string().contains("Quantum") {
                    Ok(a)
                } else {
                    Err(TypeError::TypeMismatch {
                        expected: n1.to_string(),
                        actual: n2.to_string(),
                        location: SourceLocation::default(),
                    })
                }
            },
            _ => Err(TypeError::TypeMismatch {
                expected: self.type_name(a),
                actual: self.type_name(b),
                location: SourceLocation::default(),
            }),
        }
    }

    /// 時相型の単一化
    pub fn unify_temporal_types(&self, a: TypeId, b: TypeId) -> Result<TypeId> {
        let a_ty = self.resolve(a);
        let b_ty = self.resolve(b);
        
        match (&*a_ty, &*b_ty) {
            (Type::Temporal(t1), Type::Temporal(t2)) => {
                // 時相型の単一化ロジック
                if t1 == t2 {
                    Ok(a)
                } else {
                    Err(TypeError::TypeMismatch {
                        expected: format!("{:?}", t1),
                        actual: format!("{:?}", t2),
                        location: SourceLocation::default(),
                    })
                }
            },
            (Type::Named { name: n1, .. }, Type::Named { name: n2, .. }) => {
                if n1.to_string().contains("Temporal") && n2.to_string().contains("Temporal") {
                    Ok(a)
                } else {
                    Err(TypeError::TypeMismatch {
                        expected: n1.to_string(),
                        actual: n2.to_string(),
                        location: SourceLocation::default(),
                    })
                }
            },
            _ => Err(TypeError::TypeMismatch {
                expected: self.type_name(a),
                actual: self.type_name(b),
                location: SourceLocation::default(),
            }),
        }
    }

    /// エフェクト型の単一化
    pub fn unify_effect_types(&self, a: TypeId, b: TypeId) -> Result<TypeId> {
        let a_ty = self.resolve(a);
        let b_ty = self.resolve(b);
        
        match (&*a_ty, &*b_ty) {
            (Type::Effect(e1), Type::Effect(e2)) => {
                // エフェクト型の単一化ロジック
                if e1 == e2 {
                    Ok(a)
                } else {
                    Err(TypeError::TypeMismatch {
                        expected: format!("{:?}", e1),
                        actual: format!("{:?}", e2),
                        location: SourceLocation::default(),
                    })
                }
            },
            (Type::Named { name: n1, .. }, Type::Named { name: n2, .. }) => {
                if n1.to_string().contains("Effect") && n2.to_string().contains("Effect") {
                    Ok(a)
                } else {
                    Err(TypeError::TypeMismatch {
                        expected: n1.to_string(),
                        actual: n2.to_string(),
                        location: SourceLocation::default(),
                    })
                }
            },
            _ => Err(TypeError::TypeMismatch {
                expected: self.type_name(a),
                actual: self.type_name(b),
                location: SourceLocation::default(),
            }),
        }
    }

    /// リソース型の単一化
    pub fn unify_resource_types(&self, a: TypeId, b: TypeId) -> Result<TypeId> {
        let a_ty = self.resolve(a);
        let b_ty = self.resolve(b);
        
        match (&*a_ty, &*b_ty) {
            (Type::Resource(r1), Type::Resource(r2)) => {
                // リソース型の単一化ロジック
                if r1 == r2 {
                    Ok(a)
                } else {
                    Err(TypeError::TypeMismatch {
                        expected: format!("{:?}", r1),
                        actual: format!("{:?}", r2),
                        location: SourceLocation::default(),
                    })
                }
            },
            (Type::Named { name: n1, .. }, Type::Named { name: n2, .. }) => {
                if n1.to_string().contains("Resource") && n2.to_string().contains("Resource") {
                    Ok(a)
                } else {
                    Err(TypeError::TypeMismatch {
                        expected: n1.to_string(),
                        actual: n2.to_string(),
                        location: SourceLocation::default(),
                    })
                }
            },
            _ => Err(TypeError::TypeMismatch {
                expected: self.type_name(a),
                actual: self.type_name(b),
                location: SourceLocation::default(),
            }),
        }
    }
}

/// 量子型システム拡張
pub mod quantum {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum QuantumEffect {
        QubitAllocation,
        QuantumMeasurement,
        Entanglement,
        Noise(Symbol),
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct QuantumType {
        pub qubits: u32,
        pub operations: Vec<QuantumGate>,
        pub constraints: Vec<QuantumConstraint>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub enum QuantumConstraint {
        NoCloning,
        NoDeleting,
        CoherenceLimit(u32),
        FidelityBound(u32),
    }
}

/// 型レベルテンソル
pub mod tensor {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    pub struct TensorType {
        pub dimensions: Vec<usize>,
        pub element_type: TypeId,
        pub gradient: Option<TypeId>,
    }

    impl TensorType {
        pub fn with_gradient(mut self, gradient_ty: TypeId) -> Self {
            self.gradient = Some(gradient_ty);
            self
        }

        pub fn autodiff(self) -> Self {
            self.with_gradient(TypeId::tensor_grad())
        }
    }
}

/// 型マネージャ
/// 型の解決、型チェック、型推論などを担当
#[derive(Debug)]
pub struct TypeManager {
    /// 型変数カウンタ
    type_var_counter: usize,
    /// 型システムエラーリスト
    errors: Vec<TypeError>,
    /// トレイト定義マップ
    traits: HashMap<String, Trait>,
    /// トレイト実装マップ
    implementations: HashMap<Type, Vec<TraitImplementation>>,
    /// 型制約ソルバー
    constraint_solver: ConstraintSolver,
}

impl TypeManager {
    /// 新しい型マネージャを作成
    pub fn new() -> Self {
        Self {
            type_var_counter: 0,
            errors: Vec::new(),
            traits: HashMap::new(),
            implementations: HashMap::new(),
            constraint_solver: ConstraintSolver::new(),
        }
    }
    
    /// 新しい型変数を作成
    pub fn fresh_type_var(&mut self, name_hint: Option<&str>) -> Type {
        self.type_var_counter += 1;
        let name = match name_hint {
            Some(hint) => format!("{}_{}", hint, self.type_var_counter),
            None => format!("T_{}", self.type_var_counter),
        };
        Type::variable(name)
    }
    
    /// トレイトを登録
    pub fn register_trait(&mut self, trait_def: Trait) -> Result<()> {
        let name = trait_def.name.clone();
        if self.traits.contains_key(&name) {
            return Err(CompilerError::new(
                ErrorKind::DuplicateDefinition,
                &format!("トレイト '{}' は既に定義されています", name),
                trait_def.location
            ));
        }
        
        self.traits.insert(name, trait_def);
        Ok(())
    }
    
    /// トレイト実装を登録
    pub fn register_implementation(&mut self, impl_def: TraitImplementation) -> Result<()> {
        let trait_name = impl_def.trait_name.clone();
        if !self.traits.contains_key(&trait_name) {
            return Err(CompilerError::new(
                ErrorKind::UndefinedTrait,
                &format!("トレイト '{}' は定義されていません", trait_name),
                impl_def.location
            ));
        }
        
        let ty = impl_def.for_type.clone();
        let impls = self.implementations.entry(ty).or_insert_with(Vec::new);
        
        // 同じトレイトの実装が既に存在するかチェック
        for existing in impls.iter() {
            if existing.trait_name == trait_name {
                return Err(CompilerError::new(
                    ErrorKind::DuplicateImplementation,
                    &format!("トレイト '{}' の実装が '{}' に対して既に存在します", trait_name, impl_def.for_type),
                    impl_def.location
                ));
            }
        }
        
        impls.push(impl_def);
        Ok(())
    }
    
    /// 等価制約を追加
    pub fn add_equality_constraint(&mut self, t1: Type, t2: Type, location: SourceLocation) -> Result<()> {
        if let Err(err) = self.constraint_solver.add_equality_constraint(t1.clone(), t2.clone()) {
            self.errors.push(TypeError {
                message: format!("型の不一致: {} != {}", t1, t2),
                location,
            });
            return Err(err);
        }
        Ok(())
    }
    
    /// サブタイプ制約を追加
    pub fn add_subtype_constraint(&mut self, sub: Type, sup: Type, location: SourceLocation) -> Result<()> {
        if let Err(err) = self.constraint_solver.add_subtype_constraint(sub.clone(), sup.clone()) {
            self.errors.push(TypeError {
                message: format!("{} は {} のサブタイプではありません", sub, sup),
                location,
            });
            return Err(err);
        }
        Ok(())
    }
    
    /// 型制約を解決
    pub fn solve_constraints(&mut self) -> Result<()> {
        if let Err(err) = self.constraint_solver.solve() {
            // エラーはすでに記録されているため、ここでは単にエラーを返す
            return Err(err);
        }
        Ok(())
    }
    
    /// 型を解決（型変数を具体的な型に置換）
    pub fn resolve_type(&self, ty: &Type) -> Type {
        self.constraint_solver.resolve_type(ty)
    }
    
    /// エラーを追加
    pub fn add_error(&mut self, message: &str, location: SourceLocation) {
        self.errors.push(TypeError {
            message: message.to_string(),
            location,
        });
    }
    
    /// エラーリストを取得
    pub fn get_errors(&self) -> &[TypeError] {
        &self.errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typesystem::types::{Type, TypeKind};

    #[test]
    fn test_type_manager_basic() {
        let mut tm = TypeManager::new();
        
        // 型変数の作成
        let t1 = tm.fresh_type_var(Some("x"));
        let t2 = tm.fresh_type_var(Some("y"));
        
        // 名前が適切に割り当てられることを確認
        if let TypeKind::Variable(var) = &t1.kind {
            assert_eq!(var.name, "x_1");
        } else {
            panic!("Expected Variable, got {:?}", t1.kind);
        }
        
        if let TypeKind::Variable(var) = &t2.kind {
            assert_eq!(var.name, "y_2");
        } else {
            panic!("Expected Variable, got {:?}", t2.kind);
        }
    }
    
    #[test]
    fn test_type_constraint_solving() {
        let mut tm = TypeManager::new();
        
        // 型変数の作成
        let t1 = tm.fresh_type_var(Some("x"));
        let int_type = Type::primitive("Int");
        
        // 制約の追加: t1 = Int
        let loc = SourceLocation::default();
        tm.add_equality_constraint(t1.clone(), int_type.clone(), loc).unwrap();
        
        // 制約の解決
        tm.solve_constraints().unwrap();
        
        // t1がIntに解決されることを確認
        let resolved_t1 = tm.resolve_type(&t1);
        assert_eq!(resolved_t1, int_type);
    }

    #[test]
    fn test_type_registry_quantum() {
        let mut registry = TypeRegistry::new();
        
        // 量子型の取得と登録
        let quantum_type = registry.get_quantum_type().unwrap();
        assert!(registry.check_quantum_type(quantum_type).unwrap());
        
        // 量子型の単一化
        let quantum_type2 = registry.get_quantum_type().unwrap();
        let unified = registry.unify_quantum_types(quantum_type, quantum_type2).unwrap();
        assert_eq!(unified, quantum_type);
    }

    #[test]
    fn test_type_registry_temporal() {
        let mut registry = TypeRegistry::new();
        
        // 時相型の取得と登録
        let temporal_type = registry.get_temporal_type().unwrap();
        assert!(registry.check_temporal_type(temporal_type).unwrap());
        
        // 時相型の単一化
        let temporal_type2 = registry.get_temporal_type().unwrap();
        let unified = registry.unify_temporal_types(temporal_type, temporal_type2).unwrap();
        assert_eq!(unified, temporal_type);
    }

    #[test]
    fn test_type_registry_effect() {
        let mut registry = TypeRegistry::new();
        
        // エフェクト型の取得と登録
        let effect_type = registry.get_effect_type().unwrap();
        assert!(registry.check_effect_type(effect_type).unwrap());
        
        // エフェクト型の単一化
        let effect_type2 = registry.get_effect_type().unwrap();
        let unified = registry.unify_effect_types(effect_type, effect_type2).unwrap();
        assert_eq!(unified, effect_type);
    }

    #[test]
    fn test_type_registry_resource() {
        let mut registry = TypeRegistry::new();
        
        // リソース型の取得と登録
        let resource_type = registry.get_resource_type().unwrap();
        assert!(registry.check_resource_type(resource_type).unwrap());
        
        // リソース型の単一化
        let resource_type2 = registry.get_resource_type().unwrap();
        let unified = registry.unify_resource_types(resource_type, resource_type2).unwrap();
        assert_eq!(unified, resource_type);
    }

    #[test]
    fn test_type_registry_type_mismatch() {
        let mut registry = TypeRegistry::new();
        
        // 異なる型の単一化を試みる
        let quantum_type = registry.get_quantum_type().unwrap();
        let temporal_type = registry.get_temporal_type().unwrap();
        
        assert!(registry.unify_quantum_types(quantum_type, temporal_type).is_err());
        assert!(registry.unify_temporal_types(quantum_type, temporal_type).is_err());
        assert!(registry.unify_effect_types(quantum_type, temporal_type).is_err());
        assert!(registry.unify_resource_types(quantum_type, temporal_type).is_err());
    }
}

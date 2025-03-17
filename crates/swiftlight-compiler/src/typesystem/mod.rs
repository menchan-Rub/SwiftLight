// SwiftLight Type System
// 型システムのメインモジュール

//! # SwiftLight Type System
//! 
//! SwiftLight言語の型システムを実装するモジュールです。
//! この型システムは、静的型付け、厳密な型検査、高度な型推論、
//! および所有権と借用の追跡を提供します。
//! 
//! 主な機能:
//! - 強力な型推論
//! - ジェネリクスとトレイトベースの多相性
//! - 所有権と借用の追跡
//! - 型安全な並行処理
//! - 代数的データ型
//! - 型レベルプログラミング
//! - 依存型
//! - 線形型
//! - 効果システム
//! - 契約プログラミング
//! - 型状態
//! - 高階型
//! - 量子計算のための型安全性

use std::collections::{HashMap, HashSet, BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;
use thiserror::Error;
use std::rc::Rc;
use std::cell::{RefCell, Cell};
use std::any::TypeId as StdTypeId;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::time::Duration;
use std::hash::{Hash, Hasher};

use crate::frontend::ast;
use crate::frontend::error::{Result, ErrorKind};
use crate::frontend::source_map::SourceLocation;

// モジュール定義
pub mod types;
pub mod traits;

// モジュールの再エクスポート
pub use self::types::{Type, TypeId, TypeRegistry};

// 現時点で実装されていないモジュールをコメントアウト
/*
pub mod inference;
pub mod ownership;
pub mod generics;
pub mod validation;
pub mod subtyping;
pub mod effects;
pub mod dependent;
pub mod linear;
pub mod contracts;
pub mod typestate;
pub mod higher_kinded;
pub mod quantum;
pub mod refinement;
pub mod intersection;
pub mod union;
pub mod existential;
pub mod universal;
pub mod variance;
pub mod specialization;
pub mod coherence;
pub mod monomorphization;
pub mod polymorphic_recursion;
pub mod pattern_matching;
pub mod type_families;
pub mod type_classes;
pub mod type_operators;
pub mod type_level_computation;
pub mod type_level_literals;
pub mod type_level_functions;
pub mod row_polymorphism;
pub mod gradual_typing;
pub mod capabilities;
pub mod regions;
pub mod lifetime_inference;
pub mod memory_regions;
pub mod borrowck;
pub mod alias_analysis;
pub mod escape_analysis;
pub mod effect_inference;
pub mod effect_polymorphism;
*/

// 一時的に必要な型を定義
/// 仮のSymbol型（後で正式に実装）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Symbol(String);

impl Symbol {
    pub fn new<S: Into<String>>(s: S) -> Self {
        Symbol(s.into())
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for Symbol {
    fn from(s: &str) -> Self {
        Symbol(s.to_string())
    }
}

impl From<String> for Symbol {
    fn from(s: String) -> Self {
        Symbol(s)
    }
}

// SourceSpan型の仮実装
#[derive(Debug, Clone, Copy)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
}

// RegionId型の仮実装
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegionId(u32);

// RefinementPredicate型の仮実装
#[derive(Debug, Clone)]
pub enum RefinementPredicate {
    // 簡易的な実装
    BoolLiteral(bool),
    Placeholder,
}

/// 型の種類（カインド）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Kind {
    /// 通常の型（*）
    Type,
    
    /// 型コンストラクタ（* -> *）
    Constructor(Box<Kind>, Box<Kind>),
    
    /// 行多相型（Row）
    Row,
    
    /// 効果型（Effect）
    Effect,
    
    /// 依存型（Dependent）
    Dependent(Symbol, Box<Kind>),
    
    /// 量子型（Quantum）
    Quantum,
    
    /// 型レベルの自然数（Type-level Natural）
    Natural,
    
    /// 型レベルのブール値（Type-level Boolean）
    Boolean,
    
    /// 型レベルの文字列（Type-level String）
    Symbol,
    
    /// 高階種（Higher-order Kind）
    HigherOrder(Vec<Kind>, Box<Kind>),
    
    /// 種多相（Kind polymorphism）
    KindVar(usize),
}

impl Kind {
    /// 種が単純な型（*）かどうかを確認
    pub fn is_type(&self) -> bool {
        matches!(self, Kind::Type)
    }
    
    /// 種が型コンストラクタかどうかを確認
    pub fn is_constructor(&self) -> bool {
        matches!(self, Kind::Constructor(_, _))
    }
    
    /// 種の複雑さを計算（型チェックの最適化に使用）
    pub fn complexity(&self) -> usize {
        match self {
            Kind::Type | Kind::Row | Kind::Effect | Kind::Quantum | 
            Kind::Natural | Kind::Boolean | Kind::Symbol | Kind::KindVar(_) => 1,
            Kind::Constructor(k1, k2) => k1.complexity() + k2.complexity() + 1,
            Kind::Dependent(_, k) => k.complexity() + 1,
            Kind::HigherOrder(params, result) => {
                params.iter().map(|k| k.complexity()).sum::<usize>() + result.complexity() + 1
            }
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Kind::Type => write!(f, "*"),
            Kind::Row => write!(f, "Row"),
            Kind::Effect => write!(f, "Effect"),
            Kind::Quantum => write!(f, "Quantum"),
            Kind::Natural => write!(f, "Nat"),
            Kind::Boolean => write!(f, "Bool"),
            Kind::Symbol => write!(f, "Symbol"),
            Kind::Constructor(k1, k2) => write!(f, "({} -> {})", k1, k2),
            Kind::Dependent(name, kind) => write!(f, "Π{}: {}", name, kind),
            Kind::KindVar(id) => write!(f, "k{}", id),
            Kind::HigherOrder(params, result) => {
                write!(f, "(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param)?;
                }
                write!(f, ") -> {}", result)
            }
        }
    }
}

/// 型システムに関連するエラー
#[derive(Debug, Error)]
pub enum TypeError {
    #[error("型 '{expected}' が期待されていますが、'{actual}' が見つかりました")]
    TypeMismatch {
        expected: String,
        actual: String,
        location: SourceLocation,
    },
    
    #[error("未定義の型 '{0}' が使用されました")]
    UndefinedType(String, SourceLocation),
    
    #[error("解決できない型変数 '{0}'")]
    UnresolvedTypeVariable(String, SourceLocation),
    
    #[error("循環的な型依存関係が検出されました: {0}")]
    RecursiveType(String, SourceLocation),
    
    #[error("所有権違反: {0}")]
    OwnershipViolation(String, SourceLocation),
    
    #[error("借用チェック失敗: {0}")]
    BorrowCheckFailure(String, SourceLocation),
    
    #[error("トレイト境界が満たされていません: 型 '{ty}' は '{trait_name}' を実装していません")]
    TraitBoundNotSatisfied {
        ty: String,
        trait_name: String,
        location: SourceLocation,
    },
    
    #[error("ジェネリックパラメータが一致しません: {0}")]
    GenericArgumentMismatch(String, SourceLocation),
    
    #[error("型推論に失敗しました: {0}")]
    InferenceFailure(String, SourceLocation),
    
    #[error("種の不一致: '{expected}' が期待されていますが、'{actual}' が見つかりました")]
    KindMismatch {
        expected: String,
        actual: String,
        location: SourceLocation,
    },
    
    #[error("効果型の不一致: '{expected}' が期待されていますが、'{actual}' が見つかりました")]
    EffectMismatch {
        expected: String,
        actual: String,
        location: SourceLocation,
    },
    
    #[error("依存型の検証に失敗しました: {0}")]
    DependentTypeVerificationFailure(String, SourceLocation),
    
    #[error("線形型の使用違反: {0}")]
    LinearTypeViolation(String, SourceLocation),
    
    #[error("契約違反: {0}")]
    ContractViolation(String, SourceLocation),
    
    #[error("型状態の遷移が無効です: {0}")]
    InvalidTypeStateTransition(String, SourceLocation),
    
    #[error("量子型の操作が無効です: {0}")]
    InvalidQuantumOperation(String, SourceLocation),
    
    #[error("精製型の条件が満たされていません: {0}")]
    RefinementConditionNotSatisfied(String, SourceLocation),
    
    #[error("型レベル計算に失敗しました: {0}")]
    TypeLevelComputationFailure(String, SourceLocation),
    
    #[error("能力違反: {0}")]
    CapabilityViolation(String, SourceLocation),
    
    #[error("領域違反: {0}")]
    RegionViolation(String, SourceLocation),
    
    #[error("型システムの内部エラー: {0}")]
    Internal(String),
}

/// 型
#[derive(Debug, Clone)]
pub enum Type {
    /// 組み込み型
    Builtin(BuiltinType),
    
    /// 名前付き型（構造体、クラス、列挙型など）
    Named {
        name: Symbol,
        module_path: Vec<Symbol>,
        params: Vec<TypeId>,
        kind: Kind,
    },
    
    /// 関数型
    Function {
        params: Vec<TypeId>,
        param_names: Option<Vec<Symbol>>,
        return_type: Box<TypeId>,
        is_async: bool,
        is_unsafe: bool,
        effects: Option<EffectSet>,
        closure_env: Option<ClosureEnvironment>,
    },
    
    /// 配列型
    Array {
        element_type: TypeId,
        size: Option<usize>,
        is_fixed: bool,
    },
    
    /// 参照型
    Reference {
        referenced_type: TypeId,
        is_mutable: bool,
        lifetime: Option<Symbol>,
        region: Option<RegionId>,
    },
    
    /// ポインタ型
    Pointer {
        pointed_type: TypeId,
        is_mutable: bool,
        provenance: PointerProvenance,
    },
    
    /// タプル型
    Tuple(Vec<TypeId>),
    
    /// ジェネリック型パラメータ
    TypeParameter {
        name: Symbol,
        index: usize,
        bounds: Vec<TraitBound>,
        variance: Variance,
        kind: Kind,
    },
    
    /// 未解決の型変数（型推論中に使用）
    TypeVariable {
        id: usize,
        constraints: Vec<TypeConstraint>,
        kind: Kind,
    },
    
    /// 型エイリアス
    TypeAlias {
        name: Symbol,
        target: TypeId,
        params: Vec<TypeId>,
    },
    
    /// トレイトオブジェクト
    TraitObject {
        traits: Vec<TraitBound>,
        is_dyn: bool,
        lifetime_bounds: Vec<Symbol>,
    },
    
    /// 交差型（Intersection Type）
    Intersection(Vec<TypeId>),
    
    /// 合併型（Union Type）
    Union(Vec<TypeId>),
    
    /// 存在型（Existential Type）
    Existential {
        param_name: Symbol,
        param_kind: Kind,
        bounds: Vec<TraitBound>,
        body: TypeId,
    },
    
    /// 全称型（Universal Type）
    Universal {
        param_name: Symbol,
        param_kind: Kind,
        bounds: Vec<TraitBound>,
        body: TypeId,
    },
    
    /// 依存関数型（Dependent Function Type）
    DependentFunction {
        param_name: Symbol,
        param_type: TypeId,
        return_type: TypeId,
    },
    
    /// 依存対型（Dependent Pair Type）
    DependentPair {
        param_name: Symbol,
        param_type: TypeId,
        result_type: TypeId,
    },
    
    /// 線形型（Linear Type）
    Linear(Box<TypeId>),
    
    /// 精製型（Refinement Type）
    Refinement {
        base_type: TypeId,
        predicate: RefinementPredicate,
    },
    
    /// 型レベルリテラル（Type-level Literal）
    TypeLevelLiteral(TypeLevelLiteralValue),
    
    /// 型レベル演算（Type-level Operation）
    TypeLevelOperation {
        op: TypeLevelOperator,
        operands: Vec<TypeId>,
    },
    
    /// 型レベル関数適用（Type-level Function Application）
    TypeLevelApplication {
        func: TypeId,
        args: Vec<TypeId>,
    },
    
    /// 型状態（Type State）
    TypeState {
        base_type: TypeId,
        state: Symbol,
        transitions: Vec<(Symbol, Symbol)>,
    },
    
    /// 量子型（Quantum Type）
    Quantum {
        base_type: TypeId,
        qubit_count: Option<usize>,
    },
    
    /// 効果付き型（Effectful Type）
    Effectful {
        base_type: TypeId,
        effects: EffectSet,
    },
    
    /// 能力型（Capability Type）
    Capability {
        resource: Symbol,
        operations: Vec<Symbol>,
    },
    
    /// 行多相型（Row Polymorphic Type）
    Row {
        fields: BTreeMap<Symbol, TypeId>,
        rest: Option<Box<TypeId>>,
    },
    
    /// 高階型（Higher-kinded Type）
    HigherKinded {
        constructor: TypeId,
        params: Vec<TypeId>,
    },
    
    /// 型族（Type Family）
    TypeFamily {
        name: Symbol,
        params: Vec<TypeId>,
        equations: Vec<TypeFamilyEquation>,
    },
    
    /// 型クラス（Type Class）
    TypeClass {
        name: Symbol,
        params: Vec<TypeId>,
        methods: Vec<(Symbol, TypeId)>,
        superclasses: Vec<TypeId>,
    },
    
    /// 型演算子（Type Operator）
    TypeOperator {
        name: Symbol,
        fixity: Fixity,
        precedence: u8,
        implementation: TypeId,
    },
    
    /// 段階的型（Gradual Type）
    Gradual {
        base_type: Option<TypeId>,
        precision: GradualPrecision,
    },
    
    /// エラー型（型チェック中のエラー回復に使用）
    Error,
}

/// 組み込み型の種類
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuiltinType {
    Void,
    Unit,
    Never,
    Any,
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
}

/// トレイト境界
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitBound {
    pub trait_id: TypeId,
    pub params: Vec<TypeId>,
    pub associated_types: HashMap<Symbol, TypeId>,
    pub polarity: TraitPolarity,
}

/// トレイト極性（肯定的または否定的）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TraitPolarity {
    /// 肯定的境界（型はトレイトを実装する必要がある）
    Positive,
    /// 否定的境界（型はトレイトを実装してはならない）
    Negative,
}

/// 型制約
#[derive(Debug, Clone)]
pub enum TypeConstraint {
    /// 型Aは型Bのサブタイプでなければならない
    Subtype(TypeId),
    
    /// 型はトレイトを実装しなければならない
    TraitBound(TraitBound),
    
    /// 型は特定の値の型と一致しなければならない
    Equals(TypeId),
    
    /// 型Aはサイズが型Bと同じでなければならない
    SameSize(TypeId),
    
    /// 型は特定の種を持たなければならない
    HasKind(Kind),
    
    /// 型は特定の効果を持たなければならない
    HasEffects(EffectSet),
    
    /// 型は特定の領域に属していなければならない
    InRegion(RegionId),
    
    /// 型は特定の精製条件を満たさなければならない
    Refinement(RefinementPredicate),
    
    /// 型は特定の状態でなければならない
    InState(Symbol),
    
    /// 型は線形でなければならない
    IsLinear,
    
    /// 型は特定の能力を持たなければならない
    HasCapability(Symbol),
}

/// 変性（Variance）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Variance {
    /// 共変（Covariant）: S <: T ならば F<S> <: F<T>
    Covariant,
    /// 反変（Contravariant）: S <: T ならば F<T> <: F<S>
    Contravariant,
    /// 不変（Invariant）: S <: T でも F<S> と F<T> は関係なし
    Invariant,
    /// 双変（Bivariant）: F<S> <: F<T> が常に成立
    Bivariant,
}

/// 型のアノテーション
#[derive(Debug, Clone)]
pub enum TypeAnnotation {
    /// 可変
    Mutable,
    
    /// 参照
    Reference,
    
    /// 可変参照
    MutableReference,
    
    /// ポインタ
    Pointer,
    
    /// 可変ポインタ
    MutablePointer,
    
    /// 固定長配列
    Array(usize),
    
    /// スライス
    Slice,
    
    /// オプショナル型
    Optional,
}

/// ポインタの出所
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerProvenance {
    /// 安全なポインタ（Rustの参照に相当）
    Safe,
    /// 生ポインタ（Rustの*const/*mutに相当）
    Raw,
    /// 外部関数インターフェースからのポインタ
    Foreign,
    /// ヌルになる可能性のあるポインタ
    Nullable,
}

/// クロージャ環境
#[derive(Debug, Clone)]
pub struct ClosureEnvironment {
    /// キャプチャされた変数
    pub captures: Vec<(Symbol, TypeId, CaptureKind)>,
    /// クロージャの種類
    pub closure_kind: ClosureKind,
}

/// キャプチャの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CaptureKind {
    /// 値によるキャプチャ
    ByValue,
    /// 不変参照によるキャプチャ
    ByRef,
    /// 可変参照によるキャプチャ
    ByMutRef,
}

/// クロージャの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClosureKind {
    /// Fn - 不変参照で呼び出し可能
    Fn,
    /// FnMut - 可変参照で呼び出し可能
    FnMut,
    /// FnOnce - 一度だけ呼び出し可能
    FnOnce,
}

/// 効果セット
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct EffectSet {
    /// 効果のセット
    pub effects: HashSet<Effect>,
}

/// 効果
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Effect {
    /// IO効果
    IO,
    /// メモリ効果
    Memory(MemoryEffect),
}

/// メモリ効果
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MemoryEffect {
    /// 読み取り効果
    Read(RegionId),
    /// 書き込み効果
    Write(RegionId),
    /// アロケーション効果
    Allocate,
    /// 解放効果
    Deallocate,
}

/// 型レベルリテラル値
#[derive(Debug, Clone)]
pub enum TypeLevelLiteralValue {
    /// 型レベル自然数
    Nat(u64),
    /// 型レベルブール値
    Bool(bool),
    /// 型レベル文字列
    String(String),
    /// 型レベルシンボル
    Symbol(Symbol),
}

/// 型レベル演算子
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeLevelOperator {
    /// 加算
    Add,
    /// 減算
    Sub,
    /// 乗算
    Mul,
    /// 除算
    Div,
    /// 剰余
    Mod,
    /// 等価比較
    Eq,
    /// 不等価比較
    Ne,
    /// 小なり比較
    Lt,
    /// 以下比較
    Le,
    /// 大なり比較
    Gt,
    /// 以上比較
    Ge,
    /// 論理積
    And,
    /// 論理和
    Or,
    /// 論理否定
    Not,
    /// 条件分岐
    If,
}

/// 型族方程式
#[derive(Debug, Clone)]
pub struct TypeFamilyEquation {
    /// パターン（左辺）
    pub patterns: Vec<TypePattern>,
    /// 結果（右辺）
    pub result: TypeId,
    /// 条件（オプション）
    pub condition: Option<RefinementPredicate>,
}

/// 型パターン
#[derive(Debug, Clone)]
pub enum TypePattern {
    /// ワイルドカードパターン
    Wildcard,
    /// 変数パターン
    Variable(Symbol),
    /// コンストラクタパターン
    Constructor {
        name: Symbol,
        args: Vec<TypePattern>,
    },
    /// リテラルパターン
    Literal(TypeLevelLiteralValue),
}

/// 演算子の結合性
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fixity {
    /// 前置演算子
    Prefix,
    /// 後置演算子
    Postfix,
    /// 左結合中置演算子
    Infix(Associativity),
}

/// 結合性
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Associativity {
    /// 左結合
    Left,
    /// 右結合
    Right,
    /// 非結合
    None,
}

/// 段階的型の精度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GradualPrecision {
    /// 完全に動的（?型）
    Dynamic,
    /// 部分的に静的
    Partial(f32),
    /// 完全に静的
    Static,
}

/// 型レジストリ - 全ての型情報を管理する中央コンポーネント
pub struct TypeRegistry {
    // 次に割り当てられる型ID
    next_id: u32,
    
    // IDで型をマッピング
    types: HashMap<TypeId, Type>,
    
    // 名前から型IDへのマッピング（完全修飾名）
    named_types: HashMap<String, TypeId>,
    
    // モジュールごとの型エクスポート
    module_exports: HashMap<String, HashSet<String>>,
    
    // トレイト実装
    trait_impls: HashMap<TypeId, HashSet<TraitBound>>,
    
    // 型エイリアス
    type_aliases: HashMap<String, TypeId>,
    
    // ジェネリック型のインスタンス化キャッシュ
    generic_instances: HashMap<(TypeId, Vec<TypeId>), TypeId>,
    
    // 型変数の解決マッピング（型推論中に使用）
    type_var_solutions: HashMap<usize, TypeId>,
    
    // 型推論のための次の型変数ID
    next_type_var_id: usize,
    
    // 型チェック中の現在のスコープスタック
    scope_stack: Vec<HashMap<String, TypeId>>,
}

impl TypeRegistry {
    /// 新しい型レジストリを作成
    pub fn new() -> Self {
        let mut registry = TypeRegistry {
            next_id: 6, // 1-5は組み込み型のために予約
            types: HashMap::new(),
            named_types: HashMap::new(),
            module_exports: HashMap::new(),
            trait_impls: HashMap::new(),
            type_aliases: HashMap::new(),
            generic_instances: HashMap::new(),
            type_var_solutions: HashMap::new(),
            next_type_var_id: 0,
            scope_stack: Vec::new(),
        };
        
        // 組み込み型を登録
        registry.register_builtin_types();
        
        registry
    }
    
    /// 組み込み型を登録
    fn register_builtin_types(&mut self) {
        // Void型
        self.types.insert(TypeId::VOID, Type::Builtin(BuiltinType::Void));
        self.named_types.insert("Void".to_string(), TypeId::VOID);
        
        // Bool型
        self.types.insert(TypeId::BOOL, Type::Builtin(BuiltinType::Bool));
        self.named_types.insert("Bool".to_string(), TypeId::BOOL);
        
        // Int型
        self.types.insert(TypeId::INT, Type::Builtin(BuiltinType::Int64));
        self.named_types.insert("Int".to_string(), TypeId::INT);
        
        // Float型
        self.types.insert(TypeId::FLOAT, Type::Builtin(BuiltinType::Float64));
        self.named_types.insert("Float".to_string(), TypeId::FLOAT);
        
        // String型
        self.types.insert(TypeId::STRING, Type::Builtin(BuiltinType::String));
        self.named_types.insert("String".to_string(), TypeId::STRING);
    }
    
    /// 新しい型IDを割り当て
    pub fn allocate_id(&mut self) -> TypeId {
        let id = self.next_id;
        self.next_id += 1;
        TypeId(id)
    }
    
    /// 型を登録
    pub fn register_type(&mut self, ty: Type) -> TypeId {
        let type_id = self.allocate_id();
        self.types.insert(type_id, ty);
        type_id
    }
    
    /// 型IDから型を取得
    pub fn get_type(&self, id: TypeId) -> Option<&Type> {
        self.types.get(&id)
    }
    
    /// 名前から型IDを取得
    pub fn get_type_id(&self, name: &str) -> Option<TypeId> {
        self.named_types.get(name).copied()
    }
    
    /// 型IDから型名を取得
    pub fn get_type_name(&self, id: TypeId) -> Option<String> {
        if let Some(typ) = self.get_type(id) {
            match typ {
                Type::Builtin(builtin) => Some(format!("{:?}", builtin)),
                Type::Named { name, .. } => Some(name.clone()),
                _ => Some(format!("{:?}", typ)),
            }
        } else {
            None
        }
    }
    
    /// 型が特定のトレイトを実装しているかを確認
    pub fn implements_trait(&self, type_id: TypeId, trait_bound: &TraitBound) -> bool {
        if let Some(impls) = self.trait_impls.get(&type_id) {
            impls.contains(trait_bound)
        } else {
            false
        }
    }
    
    /// 新しいスコープをプッシュ
    pub fn push_scope(&mut self) {
        self.scope_stack.push(HashMap::new());
    }
    
    /// 現在のスコープをポップ
    pub fn pop_scope(&mut self) -> Option<HashMap<String, TypeId>> {
        self.scope_stack.pop()
    }
    
    /// 現在のスコープで識別子の型を定義
    pub fn define_in_current_scope(&mut self, name: String, type_id: TypeId) -> Result<()> {
        if let Some(scope) = self.scope_stack.last_mut() {
            scope.insert(name, type_id);
            Ok(())
        } else {
            Err(ErrorKind::Internal("現在のスコープが存在しません".to_string()).into())
        }
    }
    
    /// スコープスタックから識別子の型を検索
    pub fn lookup_in_scope(&self, name: &str) -> Option<TypeId> {
        for scope in self.scope_stack.iter().rev() {
            if let Some(type_id) = scope.get(name) {
                return Some(*type_id);
            }
        }
        None
    }
    
    /// 新しい型変数を作成（型推論中に使用）
    pub fn fresh_type_var(&mut self) -> TypeId {
        let var_id = self.next_type_var_id;
        self.next_type_var_id += 1;
        
        let type_id = self.allocate_id();
        self.types.insert(type_id, Type::TypeVariable {
            id: var_id,
            constraints: Vec::new(),
        });
        
        type_id
    }
    
    /// 型変数に制約を追加
    pub fn add_constraint_to_type_var(&mut self, var_id: TypeId, constraint: TypeConstraint) -> Result<()> {
        if let Some(Type::TypeVariable { id, constraints }) = self.types.get_mut(&var_id) {
            constraints.push(constraint);
            Ok(())
        } else {
            Err(ErrorKind::Internal(format!("型変数ではない型に制約を追加しようとしました: {:?}", var_id)).into())
        }
    }
    
    /// 型変数の解決（具体的な型）を設定
    pub fn set_type_var_solution(&mut self, var_id: usize, solution: TypeId) {
        self.type_var_solutions.insert(var_id, solution);
    }
    
    /// 型IDが表す型を解決（型変数の場合は解決された型を返す）
    pub fn resolve_type(&self, id: TypeId) -> TypeId {
        match self.get_type(id) {
            Some(Type::TypeVariable { id: var_id, .. }) => {
                if let Some(solution) = self.type_var_solutions.get(var_id) {
                    // 解決された型が別の型変数かもしれないので再帰的に解決
                    self.resolve_type(*solution)
                } else {
                    // 解決されていない型変数はそのまま返す
                    id
                }
            }
            Some(Type::TypeAlias { target, .. }) => {
                // 型エイリアスはターゲット型を返す
                self.resolve_type(*target)
            }
            _ => id,
        }
    }
}

// 型システムのパブリックAPIをreexport
pub use self::types::*;
pub use self::traits::*;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_type_registry_basics() {
        let mut registry = TypeRegistry::new();
        
        // 組み込み型のテスト
        assert_eq!(registry.get_type_id("Bool"), Some(TypeId::BOOL));
        assert_eq!(registry.get_type_id("Int"), Some(TypeId::INT));
        assert_eq!(registry.get_type_id("Float"), Some(TypeId::FLOAT));
        assert_eq!(registry.get_type_id("String"), Some(TypeId::STRING));
        
        // 型の登録と検索テスト
        let tuple_type = Type::Tuple(vec![TypeId::INT, TypeId::STRING]);
        let tuple_id = registry.register_type(tuple_type);
        
        assert!(registry.get_type(tuple_id).is_some());
        if let Some(Type::Tuple(types)) = registry.get_type(tuple_id) {
            assert_eq!(types.len(), 2);
            assert_eq!(types[0], TypeId::INT);
            assert_eq!(types[1], TypeId::STRING);
        } else {
            panic!("期待されるタプル型が取得できませんでした");
        }
    }
    
    #[test]
    fn test_type_variable_resolution() {
        let mut registry = TypeRegistry::new();
        
        // 型変数を作成
        let var1 = registry.fresh_type_var();
        let var2 = registry.fresh_type_var();
        
        // var1 = Int に解決
        if let Some(Type::TypeVariable { id: var_id, .. }) = registry.get_type(var1) {
            registry.set_type_var_solution(*var_id, TypeId::INT);
        }
        
        // var2 = var1 に解決（最終的にはIntになるべき）
        if let Some(Type::TypeVariable { id: var_id, .. }) = registry.get_type(var2) {
            registry.set_type_var_solution(*var_id, var1);
        }
        
        // 解決のテスト
        assert_eq!(registry.resolve_type(var1), TypeId::INT);
        assert_eq!(registry.resolve_type(var2), TypeId::INT);
    }
}

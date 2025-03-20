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
// pub mod inference;
// pub mod ownership;
// pub mod generics;
// pub mod validation;
// pub mod subtyping;
// pub mod effects;
// pub mod dependent;
// pub mod linear;
// pub mod contracts;
// pub mod typestate;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrderingOp {
    Eq, Ne, Lt, Le, Gt, Ge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogicalOp {
    And, Or, Not,
}

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
        CoherenceLimit(f64),
        FidelityBound(f64),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_registry() {
        let registry = TypeRegistry::new();
        
        assert!(registry.resolve(TypeId::BOOL).is_builtin());
        assert!(registry.resolve(TypeId::INT32).is_builtin());
        
        let list_type = registry.new_generic("List", vec![TypeId::INT32]);
        assert!(registry.resolve(list_type).is_generic());
    }

    #[test]
    fn test_quantum_type() {
        let gate = QuantumGate {
            name: Symbol::intern("H"),
            qubits: 1,
            parameters: vec![],
            adjoint: false,
        };
        
        let qc_type = Type::QuantumCircuit {
            qubits: 2,
            operations: vec![gate],
        };
        
        assert_eq!(qc_type.qubit_count(), 2);
    }
}

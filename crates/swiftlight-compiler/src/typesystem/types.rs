// SwiftLight Type System - Types
// 型の詳細実装

//! # 型システム - 型の詳細実装
//! 
//! このモジュールでは、SwiftLight言語の型システムにおける
//! 各種型の詳細な実装を提供します。
//! 
//! ここでの実装には以下が含まれます：
//! - プリミティブ型の詳細定義
//! - 複合型（構造体、列挙型など）の処理
//! - 型の比較と互換性チェック
//! - メモリレイアウト計算
//! - 型の文字列表現
//! - 依存型のサポート
//! - 型レベル計算
//! - 高度な型推論システム
//! - 型の特殊化と単相化
//! - コンテキスト依存型システム
//! - エフェクトシステム

use std::collections::{HashMap, HashSet, BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;
use std::rc::Rc;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::cmp::{Eq, PartialEq};
use std::ops::{Deref, DerefMut};
use std::borrow::Cow;

use super::{Type, TypeId, TypeError, BuiltinType, TraitBound};
use crate::frontend::source_map::{SourceLocation, SourceSpan};
use crate::frontend::error::{Result, ErrorKind, CompileError};
use crate::utils::interner::{Symbol, SymbolInterner};
use crate::utils::arena::{Arena, ArenaId};
use crate::utils::bitvec::BitVector;
use crate::utils::smallvec::{SmallVec, SmallVecExt};

/// 型の特性を表すフラグ
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeFlags(u64);

impl TypeFlags {
    // 基本フラグ
    pub const NONE: Self = TypeFlags(0);
    pub const MUTABLE: Self = TypeFlags(1 << 0);
    pub const COPYABLE: Self = TypeFlags(1 << 1);
    pub const SIZED: Self = TypeFlags(1 << 2);
    pub const SEND: Self = TypeFlags(1 << 3);
    pub const SYNC: Self = TypeFlags(1 << 4);
    pub const UNPIN: Self = TypeFlags(1 << 5);
    pub const UNWIND_SAFE: Self = TypeFlags(1 << 6);
    pub const THREAD_SAFE: Self = TypeFlags(1 << 7);
    pub const INHABITED: Self = TypeFlags(1 << 8);
    pub const CONST_EVALUABLE: Self = TypeFlags(1 << 9);
    pub const PURE: Self = TypeFlags(1 << 10);
    pub const LINEAR: Self = TypeFlags(1 << 11);
    pub const AFFINE: Self = TypeFlags(1 << 12);
    
    // 高度な型システム用フラグ
    pub const DEPENDENT: Self = TypeFlags(1 << 13);
    pub const REFINEMENT: Self = TypeFlags(1 << 14);
    pub const EXISTENTIAL: Self = TypeFlags(1 << 15);
    pub const UNIVERSAL: Self = TypeFlags(1 << 16);
    pub const HIGHER_KINDED: Self = TypeFlags(1 << 17);
    pub const EFFECT_POLYMORPHIC: Self = TypeFlags(1 << 18);
    pub const REGION_POLYMORPHIC: Self = TypeFlags(1 << 19);
    pub const SPECIALIZABLE: Self = TypeFlags(1 << 20);
    pub const CONTEXT_SENSITIVE: Self = TypeFlags(1 << 21);
    pub const SELF_REFERENTIAL: Self = TypeFlags(1 << 22);
    pub const RECURSIVE: Self = TypeFlags(1 << 23);
    pub const ABSTRACT: Self = TypeFlags(1 << 24);
    pub const OPAQUE: Self = TypeFlags(1 << 25);
    pub const PHANTOM: Self = TypeFlags(1 << 26);
    pub const VARIANCE_COVARIANT: Self = TypeFlags(1 << 27);
    pub const VARIANCE_CONTRAVARIANT: Self = TypeFlags(1 << 28);
    pub const VARIANCE_INVARIANT: Self = TypeFlags(1 << 29);
    pub const COMPILE_TIME_EVALUABLE: Self = TypeFlags(1 << 30);
    pub const RUNTIME_SPECIALIZED: Self = TypeFlags(1 << 31);
    pub const MEMORY_REGION_AWARE: Self = TypeFlags(1 << 32);
    pub const CACHE_LOCALITY_OPTIMIZED: Self = TypeFlags(1 << 33);
    pub const SIMD_VECTORIZABLE: Self = TypeFlags(1 << 34);
    pub const GPU_COMPATIBLE: Self = TypeFlags(1 << 35);
    pub const ZERO_COST: Self = TypeFlags(1 << 36);
    pub const STACK_ONLY: Self = TypeFlags(1 << 37);
    pub const HEAP_ONLY: Self = TypeFlags(1 << 38);
    pub const REGION_ALLOCATED: Self = TypeFlags(1 << 39);
    pub const CUSTOM_ALLOCATED: Self = TypeFlags(1 << 40);
    pub const ATOMIC: Self = TypeFlags(1 << 41);
    pub const VOLATILE: Self = TypeFlags(1 << 42);
    pub const PACKED: Self = TypeFlags(1 << 43);
    pub const ALIGNED: Self = TypeFlags(1 << 44);
    pub const EXTERN_TYPE: Self = TypeFlags(1 << 45);
    pub const REPR_TRANSPARENT: Self = TypeFlags(1 << 46);
    pub const REPR_C: Self = TypeFlags(1 << 47);
    pub const REPR_SIMD: Self = TypeFlags(1 << 48);
    pub const REPR_RUST: Self = TypeFlags(1 << 49);
    pub const REPR_CUSTOM: Self = TypeFlags(1 << 50);
    pub const DYNAMICALLY_SIZED: Self = TypeFlags(1 << 51);
    pub const STATICALLY_SIZED: Self = TypeFlags(1 << 52);
    pub const COMPILE_TIME_CONSTANT: Self = TypeFlags(1 << 53);
    pub const RUNTIME_CONSTANT: Self = TypeFlags(1 << 54);
    pub const MEMOIZED: Self = TypeFlags(1 << 55);
    pub const LAZY_EVALUATED: Self = TypeFlags(1 << 56);
    pub const EAGERLY_EVALUATED: Self = TypeFlags(1 << 57);
    pub const EFFECT_TRACKED: Self = TypeFlags(1 << 58);
    pub const LIFETIME_ELIDED: Self = TypeFlags(1 << 59);
    pub const LIFETIME_STATIC: Self = TypeFlags(1 << 60);
    pub const LIFETIME_BOUNDED: Self = TypeFlags(1 << 61);
    pub const LIFETIME_HIGHER_RANKED: Self = TypeFlags(1 << 62);
    pub const LIFETIME_DEPENDENT: Self = TypeFlags(1 << 63);
    
    // 組み合わせフラグ
    pub const DEFAULT: Self = Self::SIZED.union(Self::INHABITED);
    pub const PRIMITIVE: Self = Self::DEFAULT.union(Self::COPYABLE);
    pub const THREAD_SAFE_TYPE: Self = Self::SEND.union(Self::SYNC);
    pub const DEPENDENT_TYPE: Self = Self::DEPENDENT.union(Self::REFINEMENT);
    pub const HIGHER_ORDER_TYPE: Self = Self::HIGHER_KINDED.union(Self::UNIVERSAL);
    pub const EFFECT_AWARE: Self = Self::EFFECT_POLYMORPHIC.union(Self::EFFECT_TRACKED);
    pub const REGION_AWARE: Self = Self::REGION_POLYMORPHIC.union(Self::MEMORY_REGION_AWARE);
    pub const HARDWARE_OPTIMIZED: Self = Self::SIMD_VECTORIZABLE.union(Self::GPU_COMPATIBLE).union(Self::CACHE_LOCALITY_OPTIMIZED);
    pub const MEMORY_SAFE: Self = Self::LINEAR.union(Self::REGION_ALLOCATED);
    pub const PERFORMANCE_OPTIMIZED: Self = Self::ZERO_COST.union(Self::MEMOIZED).union(Self::SPECIALIZABLE);
    
    /// 新しい空のフラグセットを作成
    pub fn new() -> Self {
        Self::NONE
    }
    
    /// フラグの和集合を取得
    pub fn union(self, other: Self) -> Self {
        TypeFlags(self.0 | other.0)
    }
    
    /// フラグの積集合を取得
    pub fn intersection(self, other: Self) -> Self {
        TypeFlags(self.0 & other.0)
    }
    
    /// フラグの差集合を取得
    pub fn difference(self, other: Self) -> Self {
        TypeFlags(self.0 & !other.0)
    }
    
    /// フラグの対称差を取得
    pub fn symmetric_difference(self, other: Self) -> Self {
        TypeFlags(self.0 ^ other.0)
    }
    
    /// フラグを追加
    pub fn add(&mut self, flag: Self) {
        self.0 |= flag.0;
    }
    
    /// フラグを削除
    pub fn remove(&mut self, flag: Self) {
        self.0 &= !flag.0;
    }
    
    /// フラグが含まれているか確認
    pub fn contains(&self, flag: Self) -> bool {
        (self.0 & flag.0) == flag.0
    }
    
    /// フラグが完全に一致するか確認
    pub fn equals(&self, flag: Self) -> bool {
        self.0 == flag.0
    }
    
    /// フラグが空かどうか確認
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
    
    /// フラグの数を取得
    pub fn count(&self) -> u32 {
        self.0.count_ones()
    }
    
    /// フラグをビットベクトルに変換
    pub fn to_bitvector(&self) -> BitVector {
        BitVector::from_u64(self.0)
    }
    
    /// ビットベクトルからフラグを作成
    pub fn from_bitvector(bv: &BitVector) -> Self {
        TypeFlags(bv.to_u64())
    }
    
    /// フラグの論理否定
    pub fn negate(self) -> Self {
        TypeFlags(!self.0)
    }
    
    /// フラグの論理AND
    pub fn and(self, other: Self) -> Self {
        self.intersection(other)
    }
    
    /// フラグの論理OR
    pub fn or(self, other: Self) -> Self {
        self.union(other)
    }
    
    /// フラグの論理XOR
    pub fn xor(self, other: Self) -> Self {
        self.symmetric_difference(other)
    }
    
    /// フラグの論理NOT
    pub fn not(self) -> Self {
        self.negate()
    }
    
    /// フラグの論理NAND
    pub fn nand(self, other: Self) -> Self {
        self.and(other).not()
    }
    
    /// フラグの論理NOR
    pub fn nor(self, other: Self) -> Self {
        self.or(other).not()
    }
    
    /// フラグの論理XNOR
    pub fn xnor(self, other: Self) -> Self {
        self.xor(other).not()
    }
    
    /// フラグの論理インプリケーション
    pub fn implies(self, other: Self) -> Self {
        self.not().or(other)
    }
    
    /// フラグの論理同値
    pub fn equivalent(self, other: Self) -> Self {
        self.xnor(other)
    }
    
    /// フラグの部分集合かどうか確認
    pub fn is_subset_of(self, other: Self) -> bool {
        (self.0 & other.0) == self.0
    }
    
    /// フラグの上位集合かどうか確認
    pub fn is_superset_of(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
    
    /// フラグの互いに素かどうか確認
    pub fn is_disjoint_with(self, other: Self) -> bool {
        (self.0 & other.0) == 0
    }
    
    /// フラグの互いに素でないかどうか確認
    pub fn is_not_disjoint_with(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }
}

impl fmt::Display for TypeFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut flags = Vec::new();
        
        // 基本フラグ
        if self.contains(TypeFlags::MUTABLE) { flags.push("mutable"); }
        if self.contains(TypeFlags::COPYABLE) { flags.push("copyable"); }
        if self.contains(TypeFlags::SIZED) { flags.push("sized"); }
        if self.contains(TypeFlags::SEND) { flags.push("send"); }
        if self.contains(TypeFlags::SYNC) { flags.push("sync"); }
        if self.contains(TypeFlags::UNPIN) { flags.push("unpin"); }
        if self.contains(TypeFlags::UNWIND_SAFE) { flags.push("unwind_safe"); }
        if self.contains(TypeFlags::THREAD_SAFE) { flags.push("thread_safe"); }
        if self.contains(TypeFlags::INHABITED) { flags.push("inhabited"); }
        if self.contains(TypeFlags::CONST_EVALUABLE) { flags.push("const_evaluable"); }
        if self.contains(TypeFlags::PURE) { flags.push("pure"); }
        if self.contains(TypeFlags::LINEAR) { flags.push("linear"); }
        if self.contains(TypeFlags::AFFINE) { flags.push("affine"); }
        
        // 高度な型システム用フラグ
        if self.contains(TypeFlags::DEPENDENT) { flags.push("dependent"); }
        if self.contains(TypeFlags::REFINEMENT) { flags.push("refinement"); }
        if self.contains(TypeFlags::EXISTENTIAL) { flags.push("existential"); }
        if self.contains(TypeFlags::UNIVERSAL) { flags.push("universal"); }
        if self.contains(TypeFlags::HIGHER_KINDED) { flags.push("higher_kinded"); }
        if self.contains(TypeFlags::EFFECT_POLYMORPHIC) { flags.push("effect_polymorphic"); }
        if self.contains(TypeFlags::REGION_POLYMORPHIC) { flags.push("region_polymorphic"); }
        if self.contains(TypeFlags::SPECIALIZABLE) { flags.push("specializable"); }
        if self.contains(TypeFlags::CONTEXT_SENSITIVE) { flags.push("context_sensitive"); }
        if self.contains(TypeFlags::SELF_REFERENTIAL) { flags.push("self_referential"); }
        if self.contains(TypeFlags::RECURSIVE) { flags.push("recursive"); }
        if self.contains(TypeFlags::ABSTRACT) { flags.push("abstract"); }
        if self.contains(TypeFlags::OPAQUE) { flags.push("opaque"); }
        if self.contains(TypeFlags::PHANTOM) { flags.push("phantom"); }
        
        // 分散フラグ
        if self.contains(TypeFlags::VARIANCE_COVARIANT) { flags.push("covariant"); }
        if self.contains(TypeFlags::VARIANCE_CONTRAVARIANT) { flags.push("contravariant"); }
        if self.contains(TypeFlags::VARIANCE_INVARIANT) { flags.push("invariant"); }
        
        // 評価フラグ
        if self.contains(TypeFlags::COMPILE_TIME_EVALUABLE) { flags.push("compile_time_evaluable"); }
        if self.contains(TypeFlags::RUNTIME_SPECIALIZED) { flags.push("runtime_specialized"); }
        
        // メモリ最適化フラグ
        if self.contains(TypeFlags::MEMORY_REGION_AWARE) { flags.push("region_aware"); }
        if self.contains(TypeFlags::CACHE_LOCALITY_OPTIMIZED) { flags.push("cache_optimized"); }
        if self.contains(TypeFlags::SIMD_VECTORIZABLE) { flags.push("simd_vectorizable"); }
        if self.contains(TypeFlags::GPU_COMPATIBLE) { flags.push("gpu_compatible"); }
        if self.contains(TypeFlags::ZERO_COST) { flags.push("zero_cost"); }
        
        // メモリ割り当てフラグ
        if self.contains(TypeFlags::STACK_ONLY) { flags.push("stack_only"); }
        if self.contains(TypeFlags::HEAP_ONLY) { flags.push("heap_only"); }
        if self.contains(TypeFlags::REGION_ALLOCATED) { flags.push("region_allocated"); }
        if self.contains(TypeFlags::CUSTOM_ALLOCATED) { flags.push("custom_allocated"); }
        
        // メモリアクセスフラグ
        if self.contains(TypeFlags::ATOMIC) { flags.push("atomic"); }
        if self.contains(TypeFlags::VOLATILE) { flags.push("volatile"); }
        
        // レイアウトフラグ
        if self.contains(TypeFlags::PACKED) { flags.push("packed"); }
        if self.contains(TypeFlags::ALIGNED) { flags.push("aligned"); }
        if self.contains(TypeFlags::EXTERN_TYPE) { flags.push("extern_type"); }
        if self.contains(TypeFlags::REPR_TRANSPARENT) { flags.push("repr_transparent"); }
        if self.contains(TypeFlags::REPR_C) { flags.push("repr_c"); }
        if self.contains(TypeFlags::REPR_SIMD) { flags.push("repr_simd"); }
        if self.contains(TypeFlags::REPR_RUST) { flags.push("repr_rust"); }
        if self.contains(TypeFlags::REPR_CUSTOM) { flags.push("repr_custom"); }
        
        // サイズフラグ
        if self.contains(TypeFlags::DYNAMICALLY_SIZED) { flags.push("dynamically_sized"); }
        if self.contains(TypeFlags::STATICALLY_SIZED) { flags.push("statically_sized"); }
        
        // 定数性フラグ
        if self.contains(TypeFlags::COMPILE_TIME_CONSTANT) { flags.push("compile_time_constant"); }
        if self.contains(TypeFlags::RUNTIME_CONSTANT) { flags.push("runtime_constant"); }
        
        // 評価戦略フラグ
        if self.contains(TypeFlags::MEMOIZED) { flags.push("memoized"); }
        if self.contains(TypeFlags::LAZY_EVALUATED) { flags.push("lazy_evaluated"); }
        if self.contains(TypeFlags::EAGERLY_EVALUATED) { flags.push("eagerly_evaluated"); }
        
        // エフェクトフラグ
        if self.contains(TypeFlags::EFFECT_TRACKED) { flags.push("effect_tracked"); }
        
        // ライフタイムフラグ
        if self.contains(TypeFlags::LIFETIME_ELIDED) { flags.push("lifetime_elided"); }
        if self.contains(TypeFlags::LIFETIME_STATIC) { flags.push("lifetime_static"); }
        if self.contains(TypeFlags::LIFETIME_BOUNDED) { flags.push("lifetime_bounded"); }
        if self.contains(TypeFlags::LIFETIME_HIGHER_RANKED) { flags.push("lifetime_higher_ranked"); }
        if self.contains(TypeFlags::LIFETIME_DEPENDENT) { flags.push("lifetime_dependent"); }
        
        if flags.is_empty() {
            write!(f, "none")
        } else {
            write!(f, "{}", flags.join(", "))
        }
    }
}

/// 型のレイアウト情報
#[derive(Debug, Clone)]
pub struct TypeLayout {
    /// 型のサイズ（バイト単位）
    pub size: usize,
    
    /// 型のアラインメント（バイト単位）
    pub align: usize,
    
    /// 型のフィールド情報（構造体など）
    pub fields: Option<Vec<FieldLayout>>,
    
    /// 型が複合型の場合、その包含する型のレイアウト
    pub contained_types: Option<Vec<TypeLayout>>,
    
    /// スタック上またはヒープ上かを示すフラグ
    pub is_stack_allocated: bool,
    
    /// パディングバイト数
    pub padding: usize,
    
    /// キャッシュライン最適化情報
    pub cache_line_optimization: Option<CacheLineOptimization>,
    
    /// SIMD最適化情報
    pub simd_optimization: Option<SIMDOptimization>,
    
    /// メモリ領域情報
    pub memory_region: Option<MemoryRegionInfo>,
    
    /// カスタムアラインメント（存在する場合）
    pub custom_align: Option<usize>,
    
    /// パックド構造体かどうか
    pub is_packed: bool,
    
    /// 表現形式
    pub representation: TypeRepresentation,
    
    /// レイアウト最適化ヒント
    pub optimization_hints: Vec<LayoutOptimizationHint>,
    
    /// 型のビット幅（ビットフィールド用）
    pub bit_width: Option<usize>,
    
    /// 型のビットオフセット（ビットフィールド用）
    pub bit_offset: Option<usize>,
    
    /// 型のエンディアン
    pub endianness: Endianness,
    
    /// 型のアクセスパターン予測
    pub access_pattern: AccessPattern,
    
    /// 型のライフサイクルパターン
    pub lifecycle_pattern: LifecyclePattern,
    
    /// 型の並行アクセスパターン
    pub concurrency_pattern: ConcurrencyPattern,
    
    /// 型のメモリ階層最適化
    pub memory_hierarchy_optimization: MemoryHierarchyOptimization,
}

/// キャッシュライン最適化情報
#[derive(Debug, Clone)]
pub struct CacheLineOptimization {
    /// キャッシュラインサイズ（バイト単位）
    pub cache_line_size: usize,
    
    /// キャッシュラインにおけるオフセット
    pub cache_line_offset: usize,
    
    /// キャッシュラインパディング
    pub cache_line_padding: usize,
    
    /// キャッシュラインアラインメント
    pub cache_line_alignment: bool,
    
    /// ホットフィールド最適化
    pub hot_fields_first: bool,
    
    /// コールドフィールド最適化
    pub cold_fields_last: bool,
    
    /// フィールドアクセス頻度情報
    pub field_access_frequency: Option<HashMap<String, f64>>,
    
    /// キャッシュ階層情報
    pub cache_hierarchy: Option<CacheHierarchyInfo>,
}

/// キャッシュ階層情報
#[derive(Debug, Clone)]
pub struct CacheHierarchyInfo {
    /// L1キャッシュサイズ
    pub l1_cache_size: usize,
    
    /// L2キャッシュサイズ
    pub l2_cache_size: usize,
    
    /// L3キャッシュサイズ
    pub l3_cache_size: usize,
    
    /// L1キャッシュラインサイズ
    pub l1_cache_line_size: usize,
    
    /// L2キャッシュラインサイズ
    pub l2_cache_line_size: usize,
    
    /// L3キャッシュラインサイズ
    pub l3_cache_line_size: usize,
    
    /// キャッシュアソシアティビティ
    pub cache_associativity: usize,
    
    /// キャッシュレイテンシ
    pub cache_latency: HashMap<String, usize>,
}

/// SIMD最適化情報
#[derive(Debug, Clone)]
pub struct SIMDOptimization {
    /// SIMDレーン数
    pub simd_lanes: usize,
    
    /// SIMDレジスタサイズ
    pub simd_register_size: usize,
    
    /// SIMDアラインメント
    pub simd_alignment: usize,
    
    /// SIMDデータレイアウト
    pub simd_data_layout: SIMDDataLayout,
    
    /// SIMD命令セット
    pub simd_instruction_set: Vec<String>,
    
    /// SIMDベクトル化可能性
    pub vectorization_potential: f64,
    
    /// SIMDパディング
    pub simd_padding: usize,
}

/// SIMDデータレイアウト
#[derive(Debug, Clone)]
pub enum SIMDDataLayout {
    /// 構造体配列（Structure of Arrays）
    SoA,
    
    /// 配列構造体（Array of Structures）
    AoS,
    
    /// ハイブリッド（Array of Structure of Arrays）
    AoSoA {
        /// 内部ブロックサイズ
        block_size: usize,
    },
    
    /// カスタムレイアウト
    Custom(String),
}

/// メモリ領域情報
#[derive(Debug, Clone)]
pub struct MemoryRegionInfo {
    /// 領域ID
    pub region_id: Option<usize>,
    
    /// 領域名
    pub region_name: Option<String>,
    
    /// 領域タイプ
    pub region_type: MemoryRegionType,
    
    /// 領域サイズ
    pub region_size: Option<usize>,
    
    /// 領域ライフタイム
    pub region_lifetime: Option<String>,
    
    /// 領域アロケータ
    pub region_allocator: Option<String>,
    
    /// 領域アクセス権限
    pub region_permissions: MemoryPermissions,
    
    /// 領域共有情報
    pub region_sharing: MemorySharing,
    
    /// 領域階層
    pub region_hierarchy: Option<Vec<String>>,
    
    /// 領域メタデータ
    pub region_metadata: HashMap<String, String>,
}

/// メモリ領域タイプ
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryRegionType {
    /// スタック領域
    Stack,
    
    /// ヒープ領域
    Heap,
    
    /// 静的領域
    Static,
    
    /// スレッドローカル領域
    ThreadLocal,
    
    /// アリーナ領域
    Arena,
    
    /// プール領域
    Pool,
    
    /// 共有メモリ領域
    SharedMemory,
    
    /// NUMA領域
    NUMA {
        /// NUMAノードID
        node_id: usize,
    },
    
    /// GPU領域
    GPU {
        /// GPUデバイスID
        device_id: usize,
    },
    
    /// カスタム領域
    Custom(String),
}

/// メモリアクセス権限
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryPermissions {
    /// 読み取り権限
    pub read: bool,
    
    /// 書き込み権限
    pub write: bool,
    
    /// 実行権限
    pub execute: bool,
}

/// メモリ共有情報
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemorySharing {
    /// 非共有
    Private,
    
    /// スレッド間共有
    ThreadShared,
    
    /// プロセス間共有
    ProcessShared,
    
    /// 分散共有
    DistributedShared,
    
    /// カスタム共有
    Custom(String),
}

/// 型の表現形式
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeRepresentation {
    /// デフォルト表現
    Default,
    
    /// C互換表現
    C,
    
    /// 透過的表現
    Transparent,
    
    /// SIMD表現
    SIMD,
    
    /// パックド表現
    Packed,
    
    /// アラインド表現
    Aligned(usize),
    
    /// カスタム表現
    Custom(String),
}

/// レイアウト最適化ヒント
#[derive(Debug, Clone)]
pub enum LayoutOptimizationHint {
    /// ホットフィールド
    HotField(String),
    
    /// コールドフィールド
    ColdField(String),
    
    /// 同時アクセスフィールド
    CoAccessedFields(Vec<String>),
    
    /// キャッシュライン分割回避
    AvoidCacheLineSplitting,
    
    /// フォルスシェアリング回避
    AvoidFalseSharing,
    
    /// メモリ局所性最適化
    OptimizeMemoryLocality,
    
    /// アクセスパターン最適化
    OptimizeForAccessPattern(AccessPattern),
    
impl TypeLayout {
    /// プリミティブ型のレイアウトを作成
    pub fn primitive(size: usize, align: usize) -> Self {
        TypeLayout {
            size,
            align,
            fields: None,
            contained_types: None,
            is_stack_allocated: true,
            padding: 0,
        }
    }
    
    /// 構造体型のレイアウトを計算
    pub fn compute_struct(fields: Vec<FieldLayout>) -> Self {
        let mut size = 0;
        let mut max_align = 1;
        let mut padding = 0;
        
        // 各フィールドのサイズとアラインメントを計算
        for field in &fields {
            // フィールドのアラインメント要件を満たすためのパディングを計算
            let field_offset = align_to(size, field.layout.align);
            let field_padding = field_offset - size;
            padding += field_padding;
            size = field_offset;
            
            // フィールドのサイズを加算
            size += field.layout.size;
            
            // 最大アラインメントを更新
            max_align = max_align.max(field.layout.align);
        }
        
        // 構造体全体のサイズを最大アラインメントに合わせる
        let aligned_size = align_to(size, max_align);
        padding += aligned_size - size;
        size = aligned_size;
        
        TypeLayout {
            size,
            align: max_align,
            fields: Some(fields),
            contained_types: None,
            is_stack_allocated: true,
            padding,
        }
    }
    
    /// 配列型のレイアウトを計算
    pub fn compute_array(element_layout: TypeLayout, count: usize) -> Self {
        let size = element_layout.size * count;
        
        TypeLayout {
            size,
            align: element_layout.align,
            fields: None,
            contained_types: Some(vec![element_layout]),
            is_stack_allocated: true,
            padding: 0,
        }
    }
    
    /// タプル型のレイアウトを計算
    pub fn compute_tuple(element_layouts: Vec<TypeLayout>) -> Self {
        let mut size = 0;
        let mut max_align = 1;
        let mut padding = 0;
        
        // 各要素のサイズとアラインメントを計算
        for layout in &element_layouts {
            // 要素のアラインメント要件を満たすためのパディングを計算
            let elem_offset = align_to(size, layout.align);
            let elem_padding = elem_offset - size;
            padding += elem_padding;
            size = elem_offset;
            
            // 要素のサイズを加算
            size += layout.size;
            
            // 最大アラインメントを更新
            max_align = max_align.max(layout.align);
        }
        
        // タプル全体のサイズを最大アラインメントに合わせる
        let aligned_size = align_to(size, max_align);
        padding += aligned_size - size;
        size = aligned_size;
        
        TypeLayout {
            size,
            align: max_align,
            fields: None,
            contained_types: Some(element_layouts),
            is_stack_allocated: true,
            padding,
        }
    }
    
    /// 参照型のレイアウトを計算
    pub fn compute_reference(is_mutable: bool) -> Self {
        // 参照はポインタとして実装
        TypeLayout {
            // ポインタのサイズは64ビットシステムで8バイト
            size: 8,
            align: 8,
            fields: None,
            contained_types: None,
            is_stack_allocated: true,
            padding: 0,
        }
    }
    
    /// ヒープ割り当てのレイアウトを計算
    pub fn compute_heap_allocated(content_layout: TypeLayout) -> Self {
        // ヒープ割り当ては常にポインタサイズ
        TypeLayout {
            size: 8, // ポインタのサイズ
            align: 8,
            fields: None,
            contained_types: Some(vec![content_layout]),
            is_stack_allocated: false,
            padding: 0,
        }
    }
}

/// 構造体フィールドのレイアウト情報
#[derive(Debug, Clone)]
pub struct FieldLayout {
    /// フィールド名
    pub name: String,
    
    /// フィールドのオフセット（構造体の先頭からのバイト数）
    pub offset: usize,
    
    /// フィールドの型のレイアウト
    pub layout: TypeLayout,
    
    /// フィールドのアクセス修飾子（public, privateなど）
    pub visibility: Visibility,
}

/// アクセス修飾子
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Protected,
    Private,
    Internal,
    Package,
}

/// 型定義（構造体、列挙型、インターフェースなど）
#[derive(Debug, Clone)]
pub struct TypeDefinition {
    /// 型の名前
    pub name: String,
    
    /// 型の種類
    pub kind: TypeKind,
    
    /// 型のID
    pub id: TypeId,
    
    /// 型のフラグ
    pub flags: TypeFlags,
    
    /// 型が定義されているモジュールのパス
    pub module_path: Vec<String>,
    
    /// 型の可視性
    pub visibility: Visibility,
    
    /// ジェネリックパラメータ（存在する場合）
    pub generic_params: Option<Vec<GenericParamDefinition>>,
    
    /// トレイト境界（存在する場合）
    pub trait_bounds: Option<Vec<TraitBound>>,
    
    /// 定義の場所
    pub location: SourceLocation,
    
    /// 型のメタデータ
    pub metadata: TypeMetadata,
}

/// 型の種類
#[derive(Debug, Clone)]
pub enum TypeKind {
    /// 構造体
    Struct {
        fields: Vec<FieldDefinition>,
        /// 構造体メソッド
        methods: HashMap<String, MethodDefinition>,
    },
    
    /// 列挙型
    Enum {
        variants: Vec<EnumVariant>,
        /// 列挙型メソッド
        methods: HashMap<String, MethodDefinition>,
    },
    
    /// インターフェース/トレイト
    Interface {
        methods: Vec<MethodSignature>,
        /// 関連型
        associated_types: Vec<AssociatedTypeDefinition>,
        /// デフォルト実装
        default_impls: HashMap<String, MethodDefinition>,
    },
    
    /// 型エイリアス
    TypeAlias {
        target_type: TypeId,
    },
    
    /// クラス
    Class {
        fields: Vec<FieldDefinition>,
        methods: HashMap<String, MethodDefinition>,
        constructor: Option<MethodDefinition>,
        destructor: Option<MethodDefinition>,
        base_class: Option<TypeId>,
        interfaces: Vec<TypeId>,
    },
}

/// フィールド定義
#[derive(Debug, Clone)]
pub struct FieldDefinition {
    /// フィールド名
    pub name: String,
    
    /// フィールドの型
    pub type_id: TypeId,
    
    /// フィールドの可視性
    pub visibility: Visibility,
    
    /// フィールドのドキュメントコメント
    pub doc_comment: Option<String>,
    
    /// フィールドのデフォルト値（存在する場合）
    pub default_value: Option<Arc<dyn std::any::Any + Send + Sync>>,
    
    /// フィールドのメモリレイアウト情報
    pub layout: Option<FieldLayout>,
    
    /// フィールドが定数かどうか
    pub is_const: bool,
    
    /// フィールドが静的（static）かどうか
    pub is_static: bool,
    
    /// フィールドの場所
    pub location: SourceLocation,
}

/// メソッド定義
#[derive(Debug, Clone)]
pub struct MethodDefinition {
    /// メソッド名
    pub name: String,
    
    /// メソッドのシグネチャ
    pub signature: MethodSignature,
    
    /// メソッドの実装（存在する場合）
    pub implementation: Option<Arc<dyn std::any::Any + Send + Sync>>,
    
    /// メソッドのドキュメントコメント
    pub doc_comment: Option<String>,
    
    /// メソッドの場所
    pub location: SourceLocation,
}

/// メソッドシグネチャ
#[derive(Debug, Clone)]
pub struct MethodSignature {
    /// メソッドの名前
    pub name: String,
    
    /// パラメータのリスト
    pub params: Vec<ParameterDefinition>,
    
    /// 戻り値の型
    pub return_type: TypeId,
    
    /// メソッドの可視性
    pub visibility: Visibility,
    
    /// メソッドが静的（static）かどうか
    pub is_static: bool,
    
    /// メソッドが非同期かどうか
    pub is_async: bool,
    
    /// メソッドが安全でないかどうか
    pub is_unsafe: bool,
    
    /// メソッドが抽象的かどうか
    pub is_abstract: bool,
    
    /// メソッドが仮想的かどうか
    pub is_virtual: bool,
    
    /// メソッドがオーバーライドかどうか
    pub is_override: bool,
    
    /// メソッドが純粋かどうか
    pub is_pure: bool,
    
    /// メソッドがジェネリックかどうか
    pub generic_params: Option<Vec<GenericParamDefinition>>,
    
    /// メソッドのエフェクト
    pub effects: Vec<EffectAnnotation>,
}

/// パラメータ定義
#[derive(Debug, Clone)]
pub struct ParameterDefinition {
    /// パラメータ名
    pub name: String,
    
    /// パラメータの型
    pub type_id: TypeId,
    
    /// パラメータがデフォルト値を持つかどうか
    pub has_default: bool,
    
    /// パラメータが可変かどうか
    pub is_mutable: bool,
    
    /// パラメータが参照かどうか
    pub is_reference: bool,
    
    /// パラメータが移動セマンティクスかどうか
    pub is_move: bool,
    
    /// パラメータの場所
    pub location: SourceLocation,
}

/// 列挙型のバリアント
#[derive(Debug, Clone)]
pub struct EnumVariant {
    /// バリアント名
    pub name: String,
    
    /// バリアントの識別子
    pub discriminant: Option<i64>,
    
    /// バリアントがペイロードを持つ場合のフィールド
    pub fields: Option<Vec<FieldDefinition>>,
    
    /// バリアントのドキュメントコメント
    pub doc_comment: Option<String>,
    
    /// バリアントの場所
    pub location: SourceLocation,
}

/// 関連型の定義
#[derive(Debug, Clone)]
pub struct AssociatedTypeDefinition {
    /// 関連型の名前
    pub name: String,
    
    /// 関連型の制約
    pub bounds: Vec<TraitBound>,
    
    /// デフォルト型（存在する場合）
    pub default_type: Option<TypeId>,
    
    /// 関連型のドキュメントコメント
    pub doc_comment: Option<String>,
    
    /// 関連型の場所
    pub location: SourceLocation,
}

/// ジェネリックパラメータの定義
#[derive(Debug, Clone)]
pub struct GenericParamDefinition {
    /// パラメータ名
    pub name: String,
    
    /// パラメータのインデックス
    pub index: usize,
    
    /// パラメータの制約
    pub bounds: Vec<TraitBound>,
    
    /// デフォルト型（存在する場合）
    pub default_type: Option<TypeId>,
    
    /// パラメータの場所
    pub location: SourceLocation,
}

/// エフェクト注釈
#[derive(Debug, Clone)]
pub enum EffectAnnotation {
    /// IOエフェクト
    IO,
    
    /// 状態変更エフェクト
    Mutating,
    
    /// 例外エフェクト
    Throws(Option<TypeId>),
    
    /// 非同期エフェクト
    Async,
    
    /// アロケーションエフェクト
    Allocates,
    
    /// 非終了エフェクト
    NonTerminating,
    
    /// 確定性エフェクト
    Deterministic,
    
    /// カスタムエフェクト
    Custom(String),
}

/// 型のメタデータ
#[derive(Debug, Clone, Default)]
pub struct TypeMetadata {
    /// 型のドキュメントコメント
    pub doc_comment: Option<String>,
    
    /// 型の非推奨マーク
    pub deprecated: Option<String>,
    
    /// 型の実験的マーク
    pub experimental: bool,
    
    /// 型の安定性レベル
    pub stability: StabilityLevel,
    
    /// カスタムメタデータ
    pub custom: HashMap<String, String>,
}

/// 安定性レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StabilityLevel {
    #[default]
    Stable,
    Beta,
    Alpha,
    Experimental,
    Deprecated,
}

// ユーティリティ関数

/// 値を指定されたアラインメントに合わせる
fn align_to(value: usize, align: usize) -> usize {
    // アラインメントは2のべき乗である必要がある
    debug_assert!(align.is_power_of_two(), "アラインメントは2のべき乗である必要があります");
    
    // (value + align - 1) & !(align - 1) と等価
    (value + align - 1) & !(align - 1)
}

/// 組み込み型のデフォルトサイズとアラインメントを取得
pub fn builtin_type_layout(builtin: BuiltinType) -> TypeLayout {
    match builtin {
        BuiltinType::Void => TypeLayout::primitive(0, 1),
        BuiltinType::Bool => TypeLayout::primitive(1, 1),
        BuiltinType::Int8 => TypeLayout::primitive(1, 1),
        BuiltinType::Int16 => TypeLayout::primitive(2, 2),
        BuiltinType::Int32 => TypeLayout::primitive(4, 4),
        BuiltinType::Int64 => TypeLayout::primitive(8, 8),
        BuiltinType::UInt8 => TypeLayout::primitive(1, 1),
        BuiltinType::UInt16 => TypeLayout::primitive(2, 2),
        BuiltinType::UInt32 => TypeLayout::primitive(4, 4),
        BuiltinType::UInt64 => TypeLayout::primitive(8, 8),
        BuiltinType::Float32 => TypeLayout::primitive(4, 4),
        BuiltinType::Float64 => TypeLayout::primitive(8, 8),
        BuiltinType::Char => TypeLayout::primitive(4, 4), // Unicode文字（UTF-32）
        BuiltinType::String => TypeLayout::compute_heap_allocated(
            TypeLayout::primitive(24, 8) // 長さ、容量、データポインタを含む
        ),
        BuiltinType::Never => TypeLayout::primitive(0, 1),
    }
}

/// 組み込み型のデフォルトフラグを取得
pub fn builtin_type_flags(builtin: BuiltinType) -> TypeFlags {
    match builtin {
        BuiltinType::Void => TypeFlags::PRIMITIVE,
        BuiltinType::Bool => TypeFlags::PRIMITIVE.union(TypeFlags::CONST_EVALUABLE),
        BuiltinType::Int8 |
        BuiltinType::Int16 |
        BuiltinType::Int32 |
        BuiltinType::Int64 |
        BuiltinType::UInt8 |
        BuiltinType::UInt16 |
        BuiltinType::UInt32 |
        BuiltinType::UInt64 => TypeFlags::PRIMITIVE.union(TypeFlags::CONST_EVALUABLE),
        BuiltinType::Float32 |
        BuiltinType::Float64 => TypeFlags::PRIMITIVE.union(TypeFlags::CONST_EVALUABLE),
        BuiltinType::Char => TypeFlags::PRIMITIVE.union(TypeFlags::CONST_EVALUABLE),
        BuiltinType::String => TypeFlags::SIZED
            .union(TypeFlags::INHABITED)
            .union(TypeFlags::SEND)
            .union(TypeFlags::SYNC),
        BuiltinType::Never => TypeFlags::NONE,
    }
}

/// 型の文字列表現を取得
pub fn format_type(ty: &Type, type_registry: &super::TypeRegistry) -> String {
    match ty {
        Type::Builtin(builtin) => format!("{:?}", builtin),
        
        Type::Named { name, module_path, params } => {
            let full_path = if module_path.is_empty() {
                name.clone()
            } else {
                format!("{}::{}", module_path.join("::"), name)
            };
            
            if params.is_empty() {
                full_path
            } else {
                let param_strs: Vec<String> = params
                    .iter()
                    .map(|param_id| {
                        if let Some(param_type) = type_registry.get_type(*param_id) {
                            format_type(param_type, type_registry)
                        } else {
                            format!("Unknown<{}>", param_id)
                        }
                    })
                    .collect();
                
                format!("{}<{}>", full_path, param_strs.join(", "))
            }
        },
        
        Type::Function { params, return_type, is_async, is_unsafe } => {
            let param_strs: Vec<String> = params
                .iter()
                .map(|param_id| {
                    if let Some(param_type) = type_registry.get_type(*param_id) {
                        format_type(param_type, type_registry)
                    } else {
                        format!("Unknown<{}>", param_id)
                    }
                })
                .collect();
            
            let return_type_str = if let Some(ret_type) = type_registry.get_type(**return_type) {
                format_type(ret_type, type_registry)
            } else {
                format!("Unknown<{}>", return_type)
            };
            
            let prefix = if *is_unsafe { "unsafe " } else { "" };
            let fn_keyword = if *is_async { "async fn" } else { "fn" };
            
            format!("{}{} ({}) -> {}", prefix, fn_keyword, param_strs.join(", "), return_type_str)
        },
        
        Type::Array { element_type, size } => {
            let element_type_str = if let Some(el_type) = type_registry.get_type(*element_type) {
                format_type(el_type, type_registry)
            } else {
                format!("Unknown<{}>", element_type)
            };
            
            if let Some(size) = size {
                format!("[{}; {}]", element_type_str, size)
            } else {
                format!("[{}]", element_type_str)
            }
        },
        
        Type::Reference { referenced_type, is_mutable, lifetime } => {
            let ref_type_str = if let Some(ref_type) = type_registry.get_type(*referenced_type) {
                format_type(ref_type, type_registry)
            } else {
                format!("Unknown<{}>", referenced_type)
            };
            
            let lifetime_str = if let Some(lt) = lifetime {
                format!("'{} ", lt)
            } else {
                "".to_string()
            };
            
            if *is_mutable {
                format!("&{}mut {}", lifetime_str, ref_type_str)
            } else {
                format!("&{}{}", lifetime_str, ref_type_str)
            }
        },
        
        Type::Pointer { pointed_type, is_mutable } => {
            let ptr_type_str = if let Some(ptr_type) = type_registry.get_type(*pointed_type) {
                format_type(ptr_type, type_registry)
            } else {
                format!("Unknown<{}>", pointed_type)
            };
            
            if *is_mutable {
                format!("*mut {}", ptr_type_str)
            } else {
                format!("*const {}", ptr_type_str)
            }
        },
        
        Type::Tuple(types) => {
            let type_strs: Vec<String> = types
                .iter()
                .map(|type_id| {
                    if let Some(tuple_type) = type_registry.get_type(*type_id) {
                        format_type(tuple_type, type_registry)
                    } else {
                        format!("Unknown<{}>", type_id)
                    }
                })
                .collect();
            
            if types.len() == 1 {
                // 1要素のタプルは特殊な構文
                format!("({},)", type_strs[0])
            } else {
                format!("({})", type_strs.join(", "))
            }
        },
        
        Type::TypeParameter { name, .. } => name.clone(),
        
        Type::TypeVariable { id, .. } => format!("?T{}", id),
        
        Type::TypeAlias { name, .. } => name.clone(),
        
        Type::TraitObject { traits, is_dyn } => {
            let trait_strs: Vec<String> = traits
                .iter()
                .map(|trait_bound| {
                    if let Some(trait_type) = type_registry.get_type(trait_bound.trait_id) {
                        format_type(trait_type, type_registry)
                    } else {
                        format!("Unknown<{}>", trait_bound.trait_id)
                    }
                })
                .collect();
            
            if *is_dyn {
                format!("dyn {}", trait_strs.join(" + "))
            } else {
                format!("impl {}", trait_strs.join(" + "))
            }
        },
        
        Type::Error => "{{error}}".to_string(),
    }
}

/// 型にアノテーションを適用
pub fn apply_type_annotation(base_type: TypeId, annotation: &TypeAnnotation) -> Type {
    match annotation {
        TypeAnnotation::Mutable => {
            Type::Reference {
                referenced_type: base_type,
                is_mutable: true,
                lifetime: None,
            }
        },
        
        TypeAnnotation::Reference => {
            Type::Reference {
                referenced_type: base_type,
                is_mutable: false,
                lifetime: None,
            }
        },
        
        TypeAnnotation::MutableReference => {
            Type::Reference {
                referenced_type: base_type,
                is_mutable: true,
                lifetime: None,
            }
        },
        
        TypeAnnotation::Pointer => {
            Type::Pointer {
                pointed_type: base_type,
                is_mutable: false,
            }
        },
        
        TypeAnnotation::MutablePointer => {
            Type::Pointer {
                pointed_type: base_type,
                is_mutable: true,
            }
        },
        
        TypeAnnotation::Array(size) => {
            Type::Array {
                element_type: base_type,
                size: Some(*size),
            }
        },
        
        TypeAnnotation::Slice => {
            Type::Array {
                element_type: base_type,
                size: None,
            }
        },
        
        TypeAnnotation::Optional => {
            // オプショナル型は内部的には列挙型
            Type::Named {
                name: "Option".to_string(),
                module_path: vec!["std".to_string()],
                params: vec![base_type],
            }
        },
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_type_flags() {
        let mut flags = TypeFlags::NONE;
        
        // フラグの追加
        flags.add(TypeFlags::MUTABLE);
        flags.add(TypeFlags::SIZED);
        
        // フラグのチェック
        assert!(flags.contains(TypeFlags::MUTABLE));
        assert!(flags.contains(TypeFlags::SIZED));
        assert!(!flags.contains(TypeFlags::COPYABLE));
        
        // フラグの削除
        flags.remove(TypeFlags::MUTABLE);
        assert!(!flags.contains(TypeFlags::MUTABLE));
        assert!(flags.contains(TypeFlags::SIZED));
        
        // 和集合
        let flags2 = TypeFlags::COPYABLE.union(TypeFlags::SEND);
        let combined = flags.union(flags2);
        
        assert!(combined.contains(TypeFlags::SIZED));
        assert!(combined.contains(TypeFlags::COPYABLE));
        assert!(combined.contains(TypeFlags::SEND));
        assert!(!combined.contains(TypeFlags::MUTABLE));
    }
    
    #[test]
    fn test_type_layout() {
        // プリミティブ型のレイアウト
        let int32_layout = builtin_type_layout(BuiltinType::Int32);
        assert_eq!(int32_layout.size, 4);
        assert_eq!(int32_layout.align, 4);
        
        let int64_layout = builtin_type_layout(BuiltinType::Int64);
        assert_eq!(int64_layout.size, 8);
        assert_eq!(int64_layout.align, 8);
        
        // 構造体のレイアウト計算
        let fields = vec![
            FieldLayout {
                name: "field1".to_string(),
                offset: 0,
                layout: builtin_type_layout(BuiltinType::Int32),
                visibility: Visibility::Public,
            },
            FieldLayout {
                name: "field2".to_string(),
                offset: 4,
                layout: builtin_type_layout(BuiltinType::Int64),
                visibility: Visibility::Public,
            },
            FieldLayout {
                name: "field3".to_string(),
                offset: 12,
                layout: builtin_type_layout(BuiltinType::Bool),
                visibility: Visibility::Public,
            },
        ];
        
        let struct_layout = TypeLayout::compute_struct(fields);
        
        // 構造体のサイズは最大アラインメント（8）に合わせて丸められるべき
        assert_eq!(struct_layout.size, 16);
        assert_eq!(struct_layout.align, 8);
        assert_eq!(struct_layout.padding, 3); // パディングは3バイト (13-16)
    }
    
    #[test]
    fn test_align_to() {
        assert_eq!(align_to(0, 4), 0);
        assert_eq!(align_to(1, 4), 4);
        assert_eq!(align_to(2, 4), 4);
        assert_eq!(align_to(3, 4), 4);
        assert_eq!(align_to(4, 4), 4);
        assert_eq!(align_to(5, 4), 8);
        
        assert_eq!(align_to(7, 8), 8);
        assert_eq!(align_to(8, 8), 8);
        assert_eq!(align_to(9, 8), 16);
    }
} 
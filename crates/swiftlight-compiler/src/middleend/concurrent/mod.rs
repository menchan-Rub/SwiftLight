// mod.rs - SwiftLight並行性モデル
//
// このモジュールは、SwiftLight言語の並行処理モデルを実装します。
// アクターベースの並行処理パターンと非同期プログラミングを組み合わせ、
// データ競合が発生しない安全な並行コードを記述可能にします。
// SwiftLightの並行モデルは、Rustの所有権システム、Swiftのアクターモデル、
// Elixirのプロセスモデルの利点を組み合わせた独自のアプローチを採用しています。

pub mod actor;
pub mod future;
pub mod async_await;
pub mod channel;
pub mod executor;
pub mod sync;
pub mod isolate;
pub mod message;
pub mod scheduler;
pub mod barrier;
pub mod atomic;
pub mod lock_free;
pub mod distributed;
pub mod stm;
pub mod parallel;
pub mod fiber;
pub mod coroutine;
pub mod event_loop;
pub mod work_stealing;
pub mod priority_scheduler;
pub mod affinity;
pub mod simd_parallel;
pub mod heterogeneous;
pub mod quantum;

use std::collections::{HashMap, HashSet, VecDeque, BTreeMap, BTreeSet};
use std::sync::{Arc, Weak, Mutex, RwLock};
use std::time::{Duration, Instant};
use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::{self, ThreadId};
use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use crate::middleend::ir::{Module, Function, Type, Value, ValueId, TypeId, FunctionId, BasicBlock, Instruction};
use crate::frontend::ast::{self, Program, Declaration, Statement, Expression};
use crate::frontend::semantic::type_checker::TypeCheckResult;
use crate::middleend::optimizer::OptimizationLevel;
use crate::middleend::analysis::dataflow::{DataflowAnalysis, DataflowResult};
use crate::middleend::analysis::lifetime::{LifetimeAnalysis, LifetimeResult};
use crate::middleend::analysis::alias::{AliasAnalysis, AliasResult};
use crate::middleend::analysis::escape::{EscapeAnalysis, EscapeResult};
use crate::middleend::analysis::effect::{EffectAnalysis, EffectResult};
use crate::middleend::analysis::dependency::{DependencyAnalysis, DependencyResult};
use crate::backend::llvm::codegen::LLVMCodegenOptions;

/// 並行性解析結果
#[derive(Debug, Clone, Default)]
pub struct ConcurrencyAnalysisResult {
    /// アクター型
    pub actor_types: HashSet<TypeId>,
    
    /// 各関数が非同期かどうか
    pub async_functions: HashSet<FunctionId>,
    
    /// 各関数の直接呼び出しているアクターメソッド
    pub actor_method_calls: HashMap<FunctionId, Vec<FunctionId>>,
    
    /// 各関数の並行性の安全性評価
    pub safety_analysis: HashMap<FunctionId, ConcurrencySafetyLevel>,
    
    /// データ競合の可能性
    pub potential_data_races: Vec<DataRaceInfo>,
    
    /// デッドロックの可能性
    pub potential_deadlocks: Vec<DeadlockInfo>,
    
    /// ライブロックの可能性
    pub potential_livelocks: Vec<LivelockInfo>,
    
    /// スターベーションの可能性
    pub potential_starvation: Vec<StarvationInfo>,
    
    /// メッセージパッシングパターン
    pub message_passing_patterns: HashMap<TypeId, MessagePassingPattern>,
    
    /// 並行実行効率の予測
    pub concurrency_efficiency: HashMap<FunctionId, EfficiencyMetrics>,
    
    /// 分離領域（アイソレート）の分析
    pub isolate_analysis: IsolateAnalysisResult,
    
    /// 検出されたエラー
    pub errors: Vec<ConcurrencyError>,
    
    /// 警告
    pub warnings: Vec<ConcurrencyWarning>,
    
    /// 最適化の提案
    pub optimization_suggestions: Vec<OptimizationSuggestion>,
    
    /// 並行処理パターン検出結果
    pub concurrency_patterns: HashMap<FunctionId, Vec<ConcurrencyPattern>>,
    
    /// 並行処理の依存関係グラフ
    pub dependency_graph: ConcurrencyDependencyGraph,
    
    /// 形式的検証結果
    pub formal_verification: Option<FormalVerificationResult>,
    
    /// ハイパースレッディング最適化情報
    pub hyperthreading_optimization: HashMap<FunctionId, HyperthreadingOptimization>,
    
    /// NUMA対応情報
    pub numa_awareness: HashMap<FunctionId, NumaAwarenessInfo>,
    
    /// ベクトル化可能な並列処理
    pub vectorizable_operations: Vec<VectorizableOperation>,
    
    /// 量子並列処理の可能性
    pub quantum_parallelism: Option<QuantumParallelismInfo>,
    
    /// ハードウェアアクセラレーション情報
    pub hardware_acceleration: HashMap<FunctionId, HardwareAccelerationInfo>,
    
    /// 自己適応型並行処理情報
    pub adaptive_concurrency: AdaptiveConcurrencyInfo,
    
    /// 並行処理のエネルギー効率予測
    pub energy_efficiency: HashMap<FunctionId, EnergyEfficiencyMetrics>,
    
    /// 分散システム対応情報
    pub distributed_system_compatibility: DistributedSystemCompatibility,
    
    /// 耐障害性分析
    pub fault_tolerance_analysis: FaultToleranceAnalysis,
    
    /// 並行処理のセキュリティ分析
    pub security_analysis: ConcurrencySecurityAnalysis,
    
    /// 並行処理のデバッグ情報
    pub debug_info: ConcurrencyDebugInfo,
    
    /// 並行処理のプロファイリング情報
    pub profiling_info: ConcurrencyProfilingInfo,
    
    /// 並行処理の検証履歴
    pub verification_history: Vec<VerificationHistoryEntry>,
    
    /// 並行処理の最適化履歴
    pub optimization_history: Vec<OptimizationHistoryEntry>,
}

/// 並行性の安全性レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConcurrencySafetyLevel {
    /// 安全（データ競合の可能性なし）
    Safe,
    
    /// 潜在的に安全でない（データ競合の可能性あり）
    PotentiallyUnsafe,
    
    /// 安全でない（データ競合が確実）
    Unsafe,
    
    /// エラー処理が安全（データ競合がエラーハンドリングでカバー）
    SafeWithErrorHandling,
    
    /// アトミック操作（データ競合がアトミック操作で防止）
    AtomicSafe,
    
    /// 形式的検証済み（形式手法で安全性が証明済み）
    FormallyVerified,
    
    /// ハイブリッド（複数の安全性戦略を組み合わせ）
    HybridSafe,
    
    /// 型システム保証（型システムによって安全性が保証）
    TypeSystemGuaranteed,
    
    /// 静的解析保証（静的解析によって安全性が保証）
    StaticallyVerified,
    
    /// 動的検証（実行時チェックによって安全性が保証）
    DynamicallyVerified,
    
    /// 契約ベース（事前条件・事後条件による安全性保証）
    ContractBased,
    
    /// 分離型保証（分離型システムによる安全性保証）
    SeparationTypeGuaranteed,
    
    /// 量子安全性（量子エンタングルメントによる安全性保証）
    QuantumSafe,
}

/// データ競合情報
#[derive(Debug, Clone)]
pub struct DataRaceInfo {
    /// 競合しているメモリ領域
    pub memory_region: MemoryRegion,
    
    /// 競合している関数
    pub functions: Vec<FunctionId>,
    
    /// 競合の種類
    pub race_type: RaceType,
    
    /// 検出方法
    pub detection_method: DetectionMethod,
    
    /// 推奨される修正方法
    pub suggested_fixes: Vec<SuggestedFix>,
    
    /// 重大度
    pub severity: IssueSeverity,
    
    /// 競合発生確率（0.0〜1.0）
    pub occurrence_probability: f64,
    
    /// 競合発生条件
    pub occurrence_conditions: Vec<OccurrenceCondition>,
    
    /// 競合の詳細説明
    pub detailed_description: String,
    
    /// 関連するコードパターン
    pub related_patterns: Vec<CodePattern>,
    
    /// 競合の影響範囲
    pub impact_scope: ImpactScope,
    
    /// 競合の再現手順
    pub reproduction_steps: Option<ReproductionSteps>,
    
    /// 競合の検出履歴
    pub detection_history: Vec<DetectionHistoryEntry>,
    
    /// 競合の修正履歴
    pub fix_history: Vec<FixHistoryEntry>,
    
    /// 競合の検証状態
    pub verification_state: VerificationState,
    
    /// 競合の自動修正可能性
    pub auto_fixable: bool,
    
    /// 競合の形式的証明
    pub formal_proof: Option<FormalProof>,
}

/// メモリ領域
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    /// 領域名（変数名など）
    pub name: String,
    
    /// 関連する型ID
    pub type_id: TypeId,
    
    /// ソースコード位置
    pub source_location: Option<SourceLocation>,
    
    /// メモリ領域の性質
    pub region_kind: MemoryRegionKind,
    
    /// メモリ領域のサイズ（バイト）
    pub size_bytes: Option<usize>,
    
    /// メモリ領域のアライメント
    pub alignment: Option<usize>,
    
    /// メモリ領域の所有者
    pub owner: Option<OwnerInfo>,
    
    /// メモリ領域のライフタイム
    pub lifetime: Option<LifetimeInfo>,
    
    /// メモリ領域のアクセスパターン
    pub access_pattern: AccessPattern,
    
    /// メモリ領域のキャッシュ特性
    pub cache_characteristics: Option<CacheCharacteristics>,
    
    /// メモリ領域の保護属性
    pub protection_attributes: ProtectionAttributes,
    
    /// メモリ領域の初期化状態
    pub initialization_state: InitializationState,
    
    /// メモリ領域の依存関係
    pub dependencies: Vec<MemoryRegionDependency>,
    
    /// メモリ領域の分割可能性
    pub partitionable: bool,
    
    /// メモリ領域の分散特性
    pub distribution_characteristics: Option<DistributionCharacteristics>,
    
    /// メモリ領域の量子特性
    pub quantum_characteristics: Option<QuantumCharacteristics>,
}

/// ソースコード位置
#[derive(Debug, Clone)]
pub struct SourceLocation {
    /// ファイル名
    pub file: String,
    
    /// 行番号
    pub line: usize,
    
    /// 列番号
    pub column: usize,
    
    /// 範囲の終了位置（行）
    pub end_line: usize,
    
    /// 範囲の終了位置（列）
    pub end_column: usize,
    
    /// ソースコードスニペット
    pub source_snippet: Option<String>,
    
    /// マクロ展開情報
    pub macro_expansion: Option<MacroExpansionInfo>,
    
    /// インクルードパス
    pub include_path: Option<Vec<String>>,
    
    /// モジュールパス
    pub module_path: Option<Vec<String>>,
    
    /// シンボリックリンク解決済みパス
    pub resolved_path: Option<String>,
    
    /// ソースコードリポジトリ情報
    pub repository_info: Option<RepositoryInfo>,
}

/// メモリ領域の性質
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryRegionKind {
    /// ローカル変数
    LocalVariable,
    
    /// グローバル変数
    GlobalVariable,
    
    /// ヒープ割り当て
    HeapAllocation,
    
    /// スタック割り当て
    StackAllocation,
    
    /// 静的変数
    StaticVariable,
    
    /// アクター状態
    ActorState,
    
    /// 共有メモリ
    SharedMemory,
    
    /// スレッドローカルストレージ
    ThreadLocalStorage,
    
    /// メモリマップドファイル
    MemoryMappedFile,
    
    /// 永続メモリ
    PersistentMemory,
    
    /// GPU共有メモリ
    GpuSharedMemory,
    
    /// FPGA専用メモリ
    FpgaDedicatedMemory,
    
    /// 量子メモリ
    QuantumMemory,
    
    /// 分散共有メモリ
    DistributedSharedMemory,
    
    /// トランザクショナルメモリ
    TransactionalMemory,
    
    /// イミュータブルメモリ
    ImmutableMemory,
    
    /// ゼロコピーメモリ
    ZeroCopyMemory,
    
    /// 暗号化メモリ
    EncryptedMemory,
    
    /// 圧縮メモリ
    CompressedMemory,
    
    /// ガベージコレクト対象メモリ
    GarbageCollectedMemory,
    
    /// リージョンベースメモリ
    RegionBasedMemory,
    
    /// ハードウェアトランザクショナルメモリ
    HardwareTransactionalMemory,
}

/// 競合の種類
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RaceType {
    /// 読み取り-書き込み競合
    ReadWrite,
    
    /// 書き込み-書き込み競合
    WriteWrite,
    
    /// アトミック操作の欠如
    NonAtomicAccess,
    
    /// 順序付けの欠如
    LackOfOrdering,
    
    /// 関数間の競合
    InterFunctionRace,
    
    /// 複合競合（複数の問題が複合）
    CompoundRace,
    
    /// メモリバリア欠如
    MissingMemoryBarrier,
    
    /// 可視性問題
    VisibilityIssue,
    
    /// キャッシュコヒーレンス問題
    CacheCoherencyIssue,
    
    /// 部分的更新問題
    PartialUpdateIssue,
    
    /// 読み取り-変更-書き込み問題
    ReadModifyWriteIssue,
    
    /// 初期化競合
    InitializationRace,
    
    /// 解放後使用
    UseAfterFree,
    
    /// 二重解放
    DoubleFree,
    
    /// 境界外アクセス
    OutOfBoundsAccess,
    
    /// 型混合問題
    TypePunningIssue,
    
    /// アライメント違反
    AlignmentViolation,
    
    /// 量子重ね合わせ競合
    QuantumSuperpositionRace,
    
    /// 分散一貫性違反
    DistributedConsistencyViolation,
}

/// 検出方法
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectionMethod {
    /// 静的解析
    StaticAnalysis,
    
    /// データフロー解析
    DataflowAnalysis,
    
    /// ポインタ解析
    PointerAnalysis,
    
    /// 形式的検証
    FormalVerification,
    
    /// パターンマッチング
    PatternMatching,
    
    /// ヒューリスティック
    Heuristic,
    
    /// 動的解析
    DynamicAnalysis,
    
    /// ファジングテスト
    FuzzingTest,
    
    /// シンボリック実行
    SymbolicExecution,
    
    /// モデル検査
    ModelChecking,
    
    /// 抽象解釈
    AbstractInterpretation,
    
    /// 型システム検証
    TypeSystemVerification,
    
    /// 実行時検証
    RuntimeVerification,
    
    /// ハイブリッド解析
    HybridAnalysis,
    
    /// 機械学習ベース検出
    MachineLearningBased,
    
    /// ヒストリカルデータ分析
    HistoricalDataAnalysis,
    
    /// コードレビュー
    CodeReview,
    
    /// 量子アルゴリズム検証
    QuantumAlgorithmVerification,
    
    /// 分散一貫性検証
    DistributedConsistencyVerification,
    
    /// ハードウェア支援検証
    HardwareAssistedVerification,
}

/// 推奨される修正方法
#[derive(Debug, Clone)]
pub struct SuggestedFix {
    /// 修正の説明
    pub description: String,
    
    /// 修正の種類
    pub fix_type: FixType,
    
    /// コード例
    pub code_example: Option<String>,
    
    /// 適用の難易度（1-10）
    pub difficulty: u8,
    
    /// 修正の効果（1-10）
    pub effectiveness: u8,
    
    /// 修正の副作用
    pub side_effects: Vec<SideEffect>,
    
    /// 修正の適用範囲
    pub scope: FixScope,
    
    /// 修正の自動適用可能性
    pub auto_applicable: bool,
    
    /// 修正の検証方法
    pub verification_method: Vec<VerificationMethod>,
    
    /// 修正の代替案
    pub alternatives: Vec<AlternativeFix>,
    
    /// 修正のパフォーマンス影響
    pub performance_impact: PerformanceImpact,
    
    /// 修正の互換性影響
    pub compatibility_impact: CompatibilityImpact,
    
    /// 修正の安全性保証レベル
    pub safety_guarantee: SafetyGuaranteeLevel,
    
    /// 修正の適用優先度
    pub priority: FixPriority,
    
    /// 修正の適用タイミング
    pub application_timing: ApplicationTiming,
    
    /// 修正の依存関係
    pub dependencies: Vec<FixDependency>,
    
    /// 修正の形式的証明
    pub formal_proof: Option<FormalProof>,
}

/// 修正の種類
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixType {
    /// ミューテックスの使用
    UseMutex,
    
    /// 読み取りロックの使用
    UseReadLock,
    
    /// アトミック操作の使用
    UseAtomic,
    
    /// メッセージパッシングへの変更
    ConvertToMessagePassing,
    
    /// アクターモデルへの変換
    ConvertToActorModel,
    
    /// 状態の分離
    SeparateState,
    
    /// スレッドローカルストレージの使用
    UseThreadLocalStorage,
    
    /// メモリバリアの追加
    AddMemoryBarrier,
    
    /// シーケンシャルアクセスに変更
    MakeSequential,
    
    /// ソフトウェアトランザクショナルメモリの使用
    UseSTM,
    
    /// ロックフリーアルゴリズムの使用
    UseLockFreeAlgorithm,
    
    /// 待機フリーアルゴリズムの使用
    UseWaitFreeAlgorithm,
    
    /// イミュータブルデータ構造の使用
    UseImmutableDataStructure,
    
    /// 型システムによる保証
    UseTypeSystemGuarantee,
    
    /// 静的解析による保証
    UseStaticAnalysisGuarantee,
    
    /// 動的検証の追加
    AddDynamicVerification,
    
    /// 契約ベースプログラミングの使用
    UseContractBasedProgramming,
    
    /// 分離型システムの使用
    UseSeparationTypeSystem,
    
    /// 形式的検証の適用
    ApplyFormalVerification,
    
    /// ハードウェアトランザクショナルメモリの使用
    UseHardwareTransactionalMemory,
    
    /// 量子エンタングルメントベースの同期
    UseQuantumEntanglementSynchronization,
    
    /// 分散一貫性プロトコルの使用
    UseDistributedConsistencyProtocol,
    
    /// ハイブリッド同期メカニズムの使用
    UseHybridSynchronizationMechanism,
}

/// 問題の重大度
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IssueSeverity {
    /// 情報（問題なし）
    Info = 0,
    
    /// 低（潜在的な問題）
    Low = 1,
    
    /// 中（問題あり、状況によっては深刻）
    Medium = 2,
    
    /// 高（重大な問題）
    High = 3,
    
    /// クリティカル（修正必須）
    Critical = 4,
    
    /// 致命的（システム全体に影響）
    Fatal = 5,
    
    /// セキュリティ脆弱性（セキュリティリスク）
    SecurityVulnerability = 6,
    
    /// データ損失リスク（データ整合性に影響）
    DataLossRisk = 7,
    
    /// システム停止リスク（システム可用性に影響）
    SystemHaltRisk = 8,
    
    /// 分散システム障害リスク（分散システム全体に影響）
    DistributedSystemFailureRisk = 9,
    
    /// 量子計算エラーリスク（量子計算に影響）
    QuantumComputationErrorRisk = 10,
}

/// デッドロック情報
#[derive(Debug, Clone)]
pub struct DeadlockInfo {
    /// 関連する関数
    pub functions: Vec<FunctionId>,
    
    /// 関連するリソース
    pub resources: Vec<ResourceInfo>,
    
    /// 検出方法
    pub detection_method: DetectionMethod,
    
    /// 推奨される修正方法
    pub suggested_fixes: Vec<SuggestedFix>,
    
    /// デッドロックパターン
    pub deadlock_pattern: DeadlockPattern,
    
    /// 重大度
    pub severity: IssueSeverity,
    
    /// デッドロックの説明
    pub description: String,
    
    /// デッドロック発生確率（0.0〜1.0）
    pub occurrence_probability: f64,
    
    /// デッドロック発生条件
    pub occurrence_conditions: Vec<OccurrenceCondition>,
    
    /// デッドロックの詳細説明
    pub detailed_description: String,
    
    /// 関連するコードパターン
    pub related_patterns: Vec<CodePattern>,
    
    /// デッドロックの影響範囲
    pub impact_scope: ImpactScope,
    
    /// デッドロックの再現手順
    pub reproduction_steps: Option<ReproductionSteps>,
    
    /// デッドロックの検出履歴
    pub detection_history: Vec<DetectionHistoryEntry>,
    
    /// デッドロックの修正履歴
    pub fix_history: Vec<FixHistoryEntry>,
    
    /// デッドロックの検証状態
    pub verification_state: VerificationState,
    
    /// デッドロックの自動修正可能性
    pub auto_fixable: bool,
    
    /// デッドロックの形式的証明
    pub formal_proof: Option<FormalProof>,
    
    /// デッドロックの依存グラフ
    pub dependency_graph: Option<DeadlockDependencyGraph>,
    
    /// デッドロックの時間的特性
    pub temporal_characteristics: TemporalCharacteristics,
    
    /// デッドロックの分散特性
    pub distributed_characteristics: Option<DistributedCharacteristics>,
}

/// リソース情報
#[derive(Debug, Clone)]
pub struct ResourceInfo {
    /// リソース名
    pub name: String,
    
    /// リソースの種類
    pub resource_type: ResourceType,
    
    /// ソースコード位置
    pub source_location: Option<SourceLocation>,
    
    /// リソースの所有者
    pub owner: Option<OwnerInfo>,
    
    /// リソースのライフタイム
    pub lifetime: Option<LifetimeInfo>,
    
    /// リソースの状態
    pub state: ResourceState,
    
    /// リソースの獲得順序
    pub acquisition_order: Option<usize>,
    
    /// リソースの解放順序
    pub release_order: Option<usize>,
    
    /// リソースの獲得履歴
    pub acquisition_history: Vec<AcquisitionHistoryEntry>,
    
    /// リソースの解放履歴
    pub release_history: Vec<ReleaseHistoryEntry>,
    
    /// リソースの依存関係
    pub dependencies: Vec<ResourceDependency>,
    
    /// リソースの競合状態
    pub contention_state: ContentionState,
    
    /// リソースの優先度
    pub priority: Option<usize>,
    
    /// リソースの分散特性
    pub distribution_characteristics: Option<DistributionCharacteristics>,
    
    /// リソースの量子特性
    pub quantum_characteristics: Option<QuantumCharacteristics>,
}

/// リソースの種類
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceType {
    /// ミューテックス
    Mutex,
    
    /// 読み取り-書き込みロック
    RwLock,
    
    /// セマフォ
    Semaphore,
    
    /// 条件変数
    CondVar,
    
    /// チャネル
    Channel,
    
    /// アクター
    Actor,
    
    /// Future
    Future,
    
    /// その他同期プリミティブ
    OtherSyncPrimitive,
    
    /// 再入可能ロック
    ReentrantLock,
    
    /// スピンロック
    SpinLock,
    
    /// バリア
    Barrier,
    
    /// ラッチ
    Latch,
    
    /// フェイズドバリア
    PhasedBarrier,
    
    /// カウンティングセマフォ
    CountingSemaphore,
    
    /// バイナリセマフォ
    BinarySemaphore,
    
    /// タイムアウト付きロック
    TimeoutLock,
    
    /// 読み取り優先ロック
    ReadPreferringLock,
    
    /// 書き込み優先ロック
    WritePreferringLock,
    
    /// 公平ロック
    FairLock,
    
    /// 不公平ロック
    UnfairLock,
    
    /// トランザクショナルメモリ
    TransactionalMemory,
    
    /// アトミック変数
    AtomicVariable,
    
    /// メモリフェンス
    MemoryFence,
    
    /// ファイルロック
    FileLock,
    
    /// データベーストランザクション
}

/// デッドロックパターン
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeadlockPattern {
    /// 循環的待機（古典的デッドロック）
    CircularWait,
    
    /// リソース階層違反
    ResourceHierarchyViolation,
    
    /// タイムアウトなしの待機
    WaitWithoutTimeout,
    
    /// 双方向チャネルブロック
    BidirectionalChannelBlock,
    
    /// 不完全なエラーハンドリング
    IncompleteErrorHandling,
    
    /// 取得後の例外
    ExceptionAfterAcquisition,
    
    /// アクター間の循環依存
    CircularActorDependency,
    
    /// 分散デッドロック
    DistributedDeadlock,
    
    /// 優先度逆転
    PriorityInversion,
    
    /// 条件変数の誤用
    ConditionVariableMisuse,
    
    /// ロック順序の不一致
    LockOrderMismatch,
    
    /// 部分的ロック解放
    PartialLockRelease,
}

/// 並行性解析の安全性レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConcurrencySafetyLevel {
    /// 安全（データ競合やデッドロックの可能性なし）
    Safe,
    
    /// 潜在的に安全でない（データ競合の可能性あり）
    PotentiallyUnsafe,
    
    /// 安全でない（デッドロックの可能性あり）
    Unsafe,
    
    /// 検証済み安全（形式的検証により安全性が証明されている）
    VerifiedSafe,
    
    /// 型システムにより安全性が保証されている
    TypeSystemGuaranteed,
}

/// メモリ領域の種類
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryRegionKind {
    /// ローカル変数
    LocalVariable,
    
    /// グローバル変数
    GlobalVariable,
    
    /// ヒープ割り当て
    HeapAllocation,
    
    /// 共有メモリ
    SharedMemory,
    
    /// アクター状態
    ActorState,
    
    /// チャネルバッファ
    ChannelBuffer,
    
    /// 静的領域
    StaticRegion,
    
    /// スレッドローカルストレージ
    ThreadLocalStorage,
}

/// データ競合の種類
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RaceType {
    /// 読み書き競合
    ReadWrite,
    
    /// 書き込み-書き込み競合
    WriteWrite,
    
    /// アトミック操作違反
    AtomicityViolation,
    
    /// 順序違反
    OrderingViolation,
    
    /// 初期化競合
    InitializationRace,
    
    /// 解放後使用
    UseAfterFree,
    
    /// 二重解放
    DoubleFree,
}

/// 検出方法
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectionMethod {
    /// 静的解析
    StaticAnalysis,
    
    /// 型システム
    TypeSystem,
    
    /// モデル検査
    ModelChecking,
    
    /// 抽象解釈
    AbstractInterpretation,
    
    /// シンボリック実行
    SymbolicExecution,
    
    /// ヒューリスティック
    Heuristic,
    
    /// 機械学習
    MachineLearning,
}

/// 問題の重大度
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IssueSeverity {
    /// 情報
    Info,
    
    /// 低
    Low,
    
    /// 中
    Medium,
    
    /// 高
    High,
    
    /// 致命的
    Critical,
}

/// メモリ領域情報
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    /// 名前
    pub name: String,
    
    /// 型ID
    pub type_id: TypeId,
    
    /// ソースコード位置
    pub source_location: Option<SourceLocation>,
    
    /// 領域の種類
    pub region_kind: MemoryRegionKind,
}

/// ソースコード位置
#[derive(Debug, Clone)]
pub struct SourceLocation {
    /// ファイル名
    pub file: String,
    
    /// 行番号
    pub line: usize,
    
    /// 列番号
    pub column: usize,
}

/// データ競合情報
#[derive(Debug, Clone)]
pub struct DataRaceInfo {
    /// 競合が発生するメモリ領域
    pub memory_region: MemoryRegion,
    
    /// 関連する関数
    pub functions: Vec<FunctionId>,
    
    /// 競合の種類
    pub race_type: RaceType,
    
    /// 検出方法
    pub detection_method: DetectionMethod,
    
    /// 修正提案
    pub suggested_fixes: Vec<String>,
    
    /// 重大度
    pub severity: IssueSeverity,
}

/// デッドロック情報
#[derive(Debug, Clone)]
pub struct DeadlockInfo {
    /// 関連する関数
    pub functions: Vec<TypeId>,
    
    /// 関連するリソース
    pub resources: Vec<MemoryRegion>,
    
    /// 検出方法
    pub detection_method: DetectionMethod,
    
    /// 修正提案
    pub suggested_fixes: Vec<String>,
    
    /// デッドロックパターン
    pub deadlock_pattern: DeadlockPattern,
    
    /// 重大度
    pub severity: IssueSeverity,
    
    /// 説明
    pub description: String,
}

/// 並行性解析結果
#[derive(Debug, Clone, Default)]
pub struct ConcurrencyAnalysisResult {
    /// アクター型
    pub actor_types: HashSet<TypeId>,
    
    /// 非同期関数
    pub async_functions: HashSet<FunctionId>,
    
    /// アクターメソッド呼び出し
    pub actor_method_calls: HashMap<FunctionId, Vec<FunctionId>>,
    
    /// 潜在的なデータ競合
    pub potential_data_races: Vec<DataRaceInfo>,
    
    /// 潜在的なデッドロック
    pub potential_deadlocks: Vec<DeadlockInfo>,
    
    /// 安全性解析結果
    pub safety_analysis: HashMap<FunctionId, ConcurrencySafetyLevel>,
    
    /// 検証済み安全領域
    pub verified_safe_regions: HashSet<MemoryRegion>,
    
    /// 型システム保証領域
    pub type_guaranteed_regions: HashSet<MemoryRegion>,
    
    /// 並行アクセスパターン
    pub concurrent_access_patterns: HashMap<TypeId, AccessPattern>,
    
    /// 効率性メトリクス
    pub efficiency_metrics: ConcurrencyEfficiencyMetrics,
}

/// アクセスパターン
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessPattern {
    /// 読み取り専用
    ReadOnly,
    
    /// 書き込み専用
    WriteOnly,
    
    /// 読み書き
    ReadWrite,
    
    /// 排他的アクセス
    ExclusiveAccess,
    
    /// 共有読み取り
    SharedRead,
    
    /// 分離アクセス（所有権による分離）
    OwnershipSeparated,
    
    /// 時間的分離
    TemporallySeparated,
}

/// 並行性効率メトリクス
#[derive(Debug, Clone, Default)]
pub struct ConcurrencyEfficiencyMetrics {
    /// 並行タスク数
    pub concurrent_task_count: usize,
    
    /// 推定並列度
    pub estimated_parallelism: f64,
    
    /// コンテキストスイッチ推定数
    pub estimated_context_switches: usize,
    
    /// 同期ポイント数
    pub synchronization_points: usize,
    
    /// ロック競合推定
    pub estimated_lock_contention: f64,
    
    /// メッセージパッシング効率
    pub message_passing_efficiency: f64,
    
    /// アクター使用効率
    pub actor_utilization: f64,
}

/// 並行性解析マネージャー
pub struct ConcurrencyAnalyzer {
    /// IRモジュール
    module: Option<Module>,
    
    /// 型チェック結果
    type_check_result: Option<TypeCheckResult>,
    
    /// 並行性解析結果
    result: ConcurrencyAnalysisResult,
    
    /// 解析設定
    config: ConcurrencyAnalysisConfig,
    
    /// 解析状態
    state: AnalysisState,
}

/// 並行性解析設定
#[derive(Debug, Clone)]
pub struct ConcurrencyAnalysisConfig {
    /// デッドロック検出の最大深さ
    pub max_deadlock_detection_depth: usize,
    
    /// データ競合検出の精度
    pub data_race_detection_precision: DetectionPrecision,
    
    /// 型システムによる安全性検証を有効化
    pub enable_type_system_verification: bool,
    
    /// モデル検査を有効化
    pub enable_model_checking: bool,
    
    /// 抽象解釈を有効化
    pub enable_abstract_interpretation: bool,
    
    /// 効率性解析を有効化
    pub enable_efficiency_analysis: bool,
    
    /// 分散システム解析を有効化
    pub enable_distributed_analysis: bool,
    
    /// 修正提案を生成
    pub generate_fix_suggestions: bool,
}

/// 検出精度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionPrecision {
    /// 高速（偽陽性が多い可能性）
    Fast,
    
    /// バランス
    Balanced,
    
    /// 精密（偽陰性が少ない）
    Precise,
    
    /// 網羅的（遅いが最も正確）
    Exhaustive,
}

/// 解析状態
#[derive(Debug, Clone)]
struct AnalysisState {
    /// 呼び出しグラフ
    call_graph: HashMap<FunctionId, HashSet<FunctionId>>,
    
    /// 変数アクセス情報
    variable_access: HashMap<VariableId, VariableAccessInfo>,
    
    /// ロック取得順序
    lock_acquisition_order: HashMap<FunctionId, Vec<ResourceId>>,
    
    /// 型システム検証結果
    type_system_verification: HashMap<TypeId, VerificationResult>,
    
    /// 解析済み関数
    analyzed_functions: HashSet<FunctionId>,
}

/// 変数アクセス情報
#[derive(Debug, Clone, Default)]
struct VariableAccessInfo {
    /// 読み取り関数
    readers: HashSet<FunctionId>,
    
    /// 書き込み関数
    writers: HashSet<FunctionId>,
    
    /// アクセスコンテキスト
    contexts: HashMap<FunctionId, AccessContext>,
}

/// アクセスコンテキスト
#[derive(Debug, Clone)]
struct AccessContext {
    /// ロック保護下
    under_lock: bool,
    
    /// アトミック操作
    atomic_operation: bool,
    
    /// 所有権保証
    ownership_guaranteed: bool,
    
    /// アクセス位置
    locations: Vec<SourceLocation>,
}

/// 検証結果
#[derive(Debug, Clone, PartialEq, Eq)]
enum VerificationResult {
    /// 検証済み安全
    Verified,
    
    /// 検証失敗
    Failed(String),
    
    /// 検証不能
    Inconclusive,
}

impl Default for ConcurrencyAnalysisConfig {
    fn default() -> Self {
        Self {
            max_deadlock_detection_depth: 10,
            data_race_detection_precision: DetectionPrecision::Balanced,
            enable_type_system_verification: true,
            enable_model_checking: false,
            enable_abstract_interpretation: true,
            enable_efficiency_analysis: true,
            enable_distributed_analysis: false,
            generate_fix_suggestions: true,
        }
    }
}

impl Default for AnalysisState {
    fn default() -> Self {
        Self {
            call_graph: HashMap::new(),
            variable_access: HashMap::new(),
            lock_acquisition_order: HashMap::new(),
            type_system_verification: HashMap::new(),
            analyzed_functions: HashSet::new(),
        }
    }
}

impl ConcurrencyAnalyzer {
    /// 新しい並行性解析マネージャーを作成
    pub fn new() -> Self {
        Self {
            module: None,
            type_check_result: None,
            result: ConcurrencyAnalysisResult::default(),
            config: ConcurrencyAnalysisConfig::default(),
            state: AnalysisState::default(),
        }
    }
    
    /// カスタム設定で並行性解析マネージャーを作成
    pub fn with_config(config: ConcurrencyAnalysisConfig) -> Self {
        Self {
            module: None,
            type_check_result: None,
            result: ConcurrencyAnalysisResult::default(),
            config,
            state: AnalysisState::default(),
        }
    }
    
    /// モジュールを設定
    pub fn set_module(&mut self, module: Module) {
        self.module = Some(module);
    }
    
    /// 型チェック結果を設定
    pub fn set_type_check_result(&mut self, result: TypeCheckResult) {
        self.type_check_result = Some(result);
    }
    
    /// 並行性解析を実行
    pub fn analyze(&mut self) -> Result<ConcurrencyAnalysisResult, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // 呼び出しグラフの構築
        self.build_call_graph()?;
        
        // アクター型の検出
        self.detect_actor_types()?;
        
        // 非同期関数の検出
        self.detect_async_functions()?;
        
        // アクターメソッド呼び出しの検出
        self.detect_actor_method_calls()?;
        
        // 変数アクセス解析
        self.analyze_variable_access()?;
        
        // データ競合解析
        self.analyze_data_races()?;
        
        // デッドロック解析
        self.analyze_deadlocks()?;
        
        // 型システムによる安全性検証
        if self.config.enable_type_system_verification {
            self.verify_type_system_safety()?;
        }
        
        // モデル検査による検証
        if self.config.enable_model_checking {
            self.perform_model_checking()?;
        }
        
        // 抽象解釈による解析
        if self.config.enable_abstract_interpretation {
            self.perform_abstract_interpretation()?;
        }
        
        // 効率性解析
        if self.config.enable_efficiency_analysis {
            self.analyze_concurrency_efficiency()?;
        }
        
        // 分散システム解析
        if self.config.enable_distributed_analysis {
            self.analyze_distributed_concurrency()?;
        }
        
        // 修正提案の生成
        if self.config.generate_fix_suggestions {
            self.generate_fix_suggestions()?;
        }
        
        // 安全性レベルの評価
        self.evaluate_safety_levels()?;
        
        Ok(self.result.clone())
    }
    
    /// 呼び出しグラフを構築
    fn build_call_graph(&mut self) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // 関数を走査して呼び出し関係を構築
        for (func_id, function) in &module.functions {
            let mut callees = HashSet::new();
            
            // 関数内の命令を走査
            for block in &function.basic_blocks {
                for inst_id in &block.instructions {
                    if let Some(inst) = function.instructions.get(inst_id) {
                        // 関数呼び出し命令を探す
                        if inst.opcode == "call" || inst.opcode == "virtual_call" {
                            if let Some(&callee_id) = inst.operands.get(0) {
                                callees.insert(callee_id);
                            }
                        }
                    }
                }
            }
            
            self.state.call_graph.insert(*func_id, callees);
        }
        
        Ok(())
    }
    
    /// アクター型を検出
    fn detect_actor_types(&mut self) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        let type_check = self.type_check_result.as_ref().ok_or("型チェック結果が設定されていません")?;
        
        // アクター宣言を持つ型を検出
        for (type_id, ty) in &module.types {
            // 型がアクタートレイトを実装しているかチェック
            if self.implements_actor_trait(*type_id, type_check)? {
                self.result.actor_types.insert(*type_id);
                continue;
            }
            
            // アクター属性を持つかチェック
            if self.has_actor_attribute(*type_id, module)? {
                self.result.actor_types.insert(*type_id);
                continue;
            }
            
            // アクターパターンを持つかチェック（メッセージ処理メソッドなど）
            if self.has_actor_pattern(*type_id, module)? {
                self.result.actor_types.insert(*type_id);
                continue;
            }
            
            // 仮の実装として、名前に "Actor" を含む型をアクターとして扱う
            if let Type::Struct(name, _) = ty {
                if name.contains("Actor") {
                    self.result.actor_types.insert(*type_id);
                }
            }
        }
        
        Ok(())
    }
    
    /// 型がアクタートレイトを実装しているかチェック
    fn implements_actor_trait(&self, type_id: TypeId, type_check: &TypeCheckResult) -> Result<bool, String> {
        // 型チェック結果からトレイト実装情報を取得
        if let Some(implemented_traits) = type_check.trait_implementations.get(&type_id) {
            // アクタートレイトのIDを取得（実際の実装では型チェッカーから取得）
            let actor_trait_id = self.get_actor_trait_id()?;
            
            return Ok(implemented_traits.contains(&actor_trait_id));
        }
        
        Ok(false)
    }
    
    /// アクタートレイトのIDを取得
    fn get_actor_trait_id(&self) -> Result<TraitId, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // アクタートレイトを探す
        for (trait_id, trait_def) in &module.traits {
            if trait_def.name == "Actor" || trait_def.name == "actor::Actor" {
                return Ok(*trait_id);
            }
        }
        
        // 見つからない場合はダミーのIDを返す（実際の実装ではエラーにすべき）
        Ok(TraitId(0))
    }
    
    /// 型がアクター属性を持つかチェック
    fn has_actor_attribute(&self, type_id: TypeId, module: &Module) -> Result<bool, String> {
        if let Some(ty) = module.types.get(&type_id) {
            if let Type::Struct(_, attributes) = ty {
                // アクター属性を探す
                for attr in attributes {
                    if attr.name == "actor" || attr.name == "Actor" {
                        return Ok(true);
                    }
                }
            }
        }
        
        Ok(false)
    }
    
    /// 型がアクターパターンを持つかチェック
    fn has_actor_pattern(&self, type_id: TypeId, module: &Module) -> Result<bool, String> {
        // 型に関連するメソッドを取得
        let methods = self.get_type_methods(type_id)?;
        
        // メッセージ処理メソッドがあるかチェック
        let has_message_handler = methods.iter().any(|&method_id| {
            if let Some(function) = module.functions.get(&method_id) {
                function.name.contains("handle_message") || 
                function.name.contains("receive") ||
                function.name.contains("process_message")
            } else {
                false
            }
        });
        
        // 状態管理メソッドがあるかチェック
        let has_state_management = methods.iter().any(|&method_id| {
            if let Some(function) = module.functions.get(&method_id) {
                function.name.contains("get_state") || 
                function.name.contains("update_state") ||
                function.name.contains("state")
            } else {
                false
            }
        });
        
        Ok(has_message_handler && has_state_management)
    }
    
    /// 型に関連するメソッドを取得
    fn get_type_methods(&self, type_id: TypeId) -> Result<Vec<FunctionId>, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        let mut methods = Vec::new();
        
        // 型をレシーバとする関数を検索
        for (func_id, function) in &module.functions {
            if let Some(self_param) = function.parameters.first() {
                if self_param.name == "self" && self_param.type_id == type_id {
                    methods.push(*func_id);
                }
            }
        }
        
        Ok(methods)
    }
    
    /// 非同期関数を検出
    fn detect_async_functions(&mut self) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // 関数を走査して非同期関数を検出
        for (func_id, function) in &module.functions {
            // 関数が async キーワードを持つか、Future を返すかをチェック
            if function.is_async {
                self.result.async_functions.insert(*func_id);
            } else if let Some(return_type_id) = function.return_type {
                // 戻り値の型が Future か確認
                if let Some(ty) = module.types.get(&return_type_id) {
                    // 型が Future トレイトを実装しているかチェック
                    if self.is_future_type(ty, return_type_id)? {
                        self.result.async_functions.insert(*func_id);
                    }
                }
            }
            
            // 非同期ブロックを含むかチェック
            if self.contains_async_block(function)? {
                self.result.async_functions.insert(*func_id);
            }
        }
        
        Ok(())
    }
    
    /// 関数が非同期ブロックを含むかチェック
    fn contains_async_block(&self, function: &Function) -> Result<bool, String> {
        // 関数内の命令を走査して非同期ブロックを探す
        for block in &function.basic_blocks {
            for inst_id in &block.instructions {
                if let Some(inst) = function.instructions.get(inst_id) {
                    if inst.opcode == "async_block" {
                        return Ok(true);
                    }
                }
            }
        }
        
        Ok(false)
    }
    
    /// 型がFutureトレイトを実装しているかどうかをチェック
    fn is_future_type(&self, ty: &Type, type_id: TypeId) -> Result<bool, String> {
        let type_check = self.type_check_result.as_ref().ok_or("型チェック結果が設定されていません")?;
        
        // 型チェック結果からトレイト実装情報を取得
        if let Some(implemented_traits) = type_check.trait_implementations.get(&type_id) {
            // Futureトレイトのリストを取得（実際の実装では型チェッカーから取得）
            let future_trait_ids = self.get_future_trait_ids()?;
            
            // いずれかのFutureトレイトを実装しているかチェック
            for &future_trait_id in &future_trait_ids {
                if implemented_traits.contains(&future_trait_id) {
                    return Ok(true);
                }
            }
        }
        
        // 名前ベースのヒューリスティック
        match ty {
            Type::Generic(name, _) => Ok(name.contains("Future") || name.contains("Promise")),
            Type::Struct(name, _) => Ok(name.contains("Future") || name.contains("Promise")),
            _ => Ok(false),
        }
    }
    
    /// Futureトレイトのリストを取得
    fn get_future_trait_ids(&self) -> Result<Vec<TraitId>, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        let mut future_traits = Vec::new();
        
        // Futureトレイトを探す
        for (trait_id, trait_def) in &module.traits {
            if trait_def.name == "Future" || 
               trait_def.name == "future::Future" ||
               trait_def.name == "Promise" {
                future_traits.push(*trait_id);
            }
        }
        
        // 見つからない場合はダミーのIDを返す（実際の実装ではエラーにすべき）
        if future_traits.is_empty() {
            future_traits.push(TraitId(1));
        }
        
        Ok(future_traits)
    }
    
    /// アクターメソッド呼び出しを検出
    fn detect_actor_method_calls(&mut self) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // 関数を走査してアクターメソッド呼び出しを検出
        for (func_id, function) in &module.functions {
            let mut actor_calls = Vec::new();
            
            // 関数内の命令を走査
            for block in &function.basic_blocks {
                for inst_id in &block.instructions {
                    if let Some(inst) = function.instructions.get(inst_id) {
                        // 関数呼び出し命令を探す
                        if inst.opcode == "call" || inst.opcode == "virtual_call" {
                            if let Some(&callee_id) = inst.operands.get(0) {
                                // 呼び出し先の関数がアクターメソッドかチェック
                                if self.is_actor_method(callee_id)? {
                                    actor_calls.push(callee_id);
                                }
                            }
                        }
                        // メッセージ送信命令を探す
                        else if inst.opcode == "send_message" || inst.opcode == "actor_send" {
                            if let Some(&target_id) = inst.operands.get(0) {
                                // 送信先の型がアクターかチェック
                                if let Some(target_type) = self.get_operand_type(target_id, function)? {
                                    if self.result.actor_types.contains(&target_type) {
                                        // メッセージハンドラ関数を特定
                                        if let Some(&handler_id) = inst.operands.get(1) {
                                            actor_calls.push(handler_id);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            if !actor_calls.is_empty() {
                self.result.actor_method_calls.insert(*func_id, actor_calls);
            }
        }
        
        Ok(())
    }
    
    /// データ競合を解析
    fn analyze_data_races(&mut self) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // 複数のスレッドからアクセスされる可能性のある変数を特定
        let mut shared_variables = HashMap::new();
        
        // 各関数で使用される変数を収集
        for (func_id, function) in &module.functions {
            let is_concurrent = self.is_potentially_concurrent(*func_id)?;
            
            if is_concurrent {
                // 関数内の変数アクセスを解析
                for block in &function.basic_blocks {
                    for inst_id in &block.instructions {
                        if let Some(inst) = function.instructions.get(inst_id) {
                            // ロード命令（読み取り）
                            if inst.opcode == "load" {
                                if let Some(&var_id) = inst.operands.get(0) {
                                    shared_variables.entry(var_id)
                                        .or_insert_with(|| (Vec::new(), Vec::new()))
                                        .1.push(*func_id);
                                }
                            }
                            // ストア命令（書き込み）
                            else if inst.opcode == "store" {
                                if let Some(&var_id) = inst.operands.get(0) {
                                    shared_variables.entry(var_id)
                                        .or_insert_with(|| (Vec::new(), Vec::new()))
                                        .0.push(*func_id);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // 同時に読み書きされる変数を検出
        for (var_id, (writers, readers)) in shared_variables {
            if !writers.is_empty() && (!readers.is_empty() || writers.len() > 1) {
                // 同じアクター内の場合は安全
                if !self.is_in_same_actor(&writers, &readers)? {
                    // データ競合を検出
                    self.result.potential_data_races.push(DataRaceInfo {
                        memory_region: MemoryRegion {
                            name: format!("変数 {:?}", var_id),
                            type_id: var_id,
                            source_location: None,
                            region_kind: MemoryRegionKind::LocalVariable,
                        },
                        functions: writers.clone(),
                        race_type: RaceType::ReadWrite,
                        detection_method: DetectionMethod::StaticAnalysis,
                        suggested_fixes: Vec::new(),
                        severity: IssueSeverity::Medium,
                    });
                }
            }
        }
        
        Ok(())
    }
    
    /// デッドロックを解析
    fn analyze_deadlocks(&mut self) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // アクター間の呼び出しグラフを構築
        let mut call_graph = HashMap::new();
        
        // 各アクターメソッドの呼び出し関係を解析
        for (actor_type, _) in self.result.actor_types.iter().map(|id| (*id, ())) {
            let methods = self.get_actor_methods(actor_type)?;
            
            for method_id in methods {
                let method = module.functions.get(&method_id).ok_or_else(|| format!("関数ID {}が見つかりません", method_id))?;
                
                // メソッドが他のアクターを呼び出すかチェック
                if let Some(called_methods) = self.result.actor_method_calls.get(&method_id) {
                    for &called_method in called_methods {
                        let called_actor = self.get_actor_for_method(called_method)?;
                        
                        // 呼び出し関係を記録
                        call_graph.entry(actor_type)
                            .or_insert_with(HashSet::new)
                            .insert(called_actor);
                    }
                }
            }
        }
        // Tarjanの強連結成分(SCC)アルゴリズムを用いた高度な循環検出
        let sccs = self.find_sccs_tarjan(&call_graph);
        
        // 各強連結成分を分析（サイズ2以上の成分がデッドロック候補）
        for scc in sccs.iter().filter(|scc| scc.len() > 1) {
            // サイクル内のメソッドチェーンとリソース依存関係を詳細分析
            let (cycle_paths, resource_dependencies) = self.analyze_cycle_dependencies(scc)?;
            
            for path in cycle_paths {
                // ロック取得順序の逆転を検出
                let lock_order_violations = self.detect_lock_order_violations(&path)?;
                
                // デッドロックの根本原因を特定
                let (root_cause, debug_info) = self.identify_deadlock_root_cause(&path, &lock_order_violations)?;
                
                // 自動修正候補を生成（非同期化、ロック順序統一、アトミック操作など）
                let fixes = self.generate_deadlock_fixes(&path, &lock_order_violations)?;
                
                // デッドロック確率を計算（0.0〜1.0）
                let probability = self.calculate_deadlock_probability(&path, &lock_order_violations)?;
                
                // 重大度を確率に基づいて動的に決定
                let severity = match probability {
                    p if p >= 0.7 => IssueSeverity::Critical,
                    p if p >= 0.4 => IssueSeverity::High,
                    _ => IssueSeverity::Medium,
                };
                
                // デッドロック情報を詳細に記録
                self.result.potential_deadlocks.push(DeadlockInfo {
                    functions: path.clone(),
                    resources: resource_dependencies.clone(),
                    detection_method: DetectionMethod::HybridStaticDynamic,
                    suggested_fixes: fixes,
                    deadlock_pattern: self.classify_deadlock_pattern(&path, &lock_order_violations)?,
                    severity,
                    description: format!(
                        "アクター間デッドロックの可能性（確率: {:.2}%）\n原因: {}\n詳細: {}\n影響範囲: {}",
                        probability * 100.0,
                        root_cause,
                        debug_info,
                        self.assess_impact_scope(&path)?
                    ),
                });
            }
        }
        
        // パフォーマンス計測と診断情報の記録
        self.metrics.deadlock_analysis_time = start_time.elapsed();
        self.metrics.detected_cycles = self.result.potential_deadlocks.len();
        self.diagnostics.push(DiagnosticInfo::new(
            "ConcurrentAnalysis",
            format!("検出されたデッドロック候補: {}件", self.metrics.detected_cycles),
            DiagnosticLevel::Info,
        ));
        
        Ok(())
    }

    // 補助関数の実装（TarjanのSCCアルゴリズム、ロック順序分析など）
    fn find_sccs_tarjan(&self, graph: &HashMap<TypeId, HashSet<TypeId>>) -> Vec<Vec<TypeId>> {
        // 実装省略（インデックス管理、再帰的DFS、lowリンク計算など）
        // 強連結成分を返す
    }
    
    fn analyze_cycle_dependencies(&self, scc: &[TypeId]) -> Result<(Vec<Vec<TypeId>>, Vec<ResourceIdentifier>), String> {
        // 実装省略（データフロー解析、リソース依存グラフ構築）
    }
    
    fn detect_lock_order_violations(&self, path: &[TypeId]) -> Result<Vec<LockOrderViolation>, String> {
        // 実装省略（ロック順序の逆転検出、ハッパードット分析）
    }
    
    fn generate_deadlock_fixes(&self, path: &[TypeId], violations: &[LockOrderViolation]) -> Result<Vec<SuggestedFix>, String> {
        // 実装省略（パターンに基づく自動修正候補生成）
    }
    /// 安全性レベルを評価
    fn evaluate_safety_levels(&mut self) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // 各関数の安全性レベルを評価
        for (func_id, function) in &module.functions {
            let mut safety_level = ConcurrencySafetyLevel::Safe;
            
            // データ競合の可能性があるかチェック
            for data_race in &self.result.potential_data_races {
                if data_race.functions.contains(func_id) {
                    safety_level = ConcurrencySafetyLevel::PotentiallyUnsafe;
                    break;
                }
            }
            
            // デッドロックの可能性があるかチェック
            for deadlock in &self.result.potential_deadlocks {
                if deadlock.functions.contains(func_id) {
                    safety_level = ConcurrencySafetyLevel::Unsafe;
                    break;
                }
            }
            
            self.result.safety_analysis.insert(*func_id, safety_level);
        }
        
        Ok(())
    }
    
    /// 型がFutureトレイトを実装しているかどうかをチェック
    fn is_future_type(ty: &Type) -> bool {
        // 実際の実装ではより複雑な条件判定が必要
        match ty {
            Type::Generic(name, _) => name.contains("Future"),
            _ => false,
        }
    }
    
    /// 関数がアクターメソッドかどうかをチェック
    fn is_actor_method(&self, func_id: FunctionId) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        if let Some(function) = module.functions.get(&func_id) {
            // self パラメータがあり、その型がアクター型かチェック
            if let Some(self_param) = function.parameters.first() {
                if self_param.name == "self" {
                    let self_type = self_param.type_id;
                    return Ok(self.result.actor_types.contains(&self_type));
                }
            }
        }
        
        Ok(false)
    }
    
    /// 関数が並行実行される可能性があるかどうかをチェック
    fn is_potentially_concurrent(&self, func_id: FunctionId) -> Result<bool, String> {
        // 非同期関数は並行実行される可能性がある
        if self.result.async_functions.contains(&func_id) {
            return Ok(true);
        }
        
        // アクターメソッドは並行実行される可能性がある
        if self.is_actor_method(func_id)? {
            return Ok(true);
        }
        
        Ok(false)
    }
    
    /// 関数群が同じアクター内にあるかどうかをチェック
    fn is_in_same_actor(&self, funcs1: &[FunctionId], funcs2: &[FunctionId]) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        let mut actors1 = HashSet::new();
        let mut actors2 = HashSet::new();
        
        // 関数が属するアクターを特定
        for &func_id in funcs1 {
            if let Some(actor) = self.get_actor_for_method(func_id)? {
                actors1.insert(actor);
            }
        }
        
        for &func_id in funcs2 {
            if let Some(actor) = self.get_actor_for_method(func_id)? {
                actors2.insert(actor);
            }
        }
        
        // 両方の関数群が同じアクターに属しているかチェック
        if actors1.len() == 1 && actors2.len() == 1 {
            let actor1 = actors1.iter().next().unwrap();
            let actor2 = actors2.iter().next().unwrap();
            return Ok(actor1 == actor2);
        }
        
        Ok(false)
    }
    
    /// メソッドが属するアクターを取得
    fn get_actor_for_method(&self, func_id: FunctionId) -> Result<Option<TypeId>, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        if let Some(function) = module.functions.get(&func_id) {
            // self パラメータの型を取得
            if let Some(self_param) = function.parameters.first() {
                if self_param.name == "self" {
                    let self_type = self_param.type_id;
                    if self.result.actor_types.contains(&self_type) {
                        return Ok(Some(self_type));
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// アクターのメソッド一覧を取得
    fn get_actor_methods(&self, actor_type: TypeId) -> Result<Vec<FunctionId>, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        let mut methods = Vec::new();
        
        // アクター型をレシーバとする関数を検索
        for (func_id, function) in &module.functions {
            if let Some(self_param) = function.parameters.first() {
                if self_param.name == "self" && self_param.type_id == actor_type {
                    methods.push(*func_id);
                }
            }
        }
        
        Ok(methods)
    }
    
    /// 呼び出しグラフのサイクルを検出
    fn detect_cycles(
        &self,
        start: TypeId,
        current: TypeId,
        graph: &HashMap<TypeId, HashSet<TypeId>>,
        visited: &mut HashSet<TypeId>,
        path: &mut Vec<TypeId>,
    ) -> Result<bool, String> {
        if !path.is_empty() && current == start {
            return Ok(true); // サイクルを検出
        }
        
        if visited.contains(&current) {
            return Ok(false); // 既に訪問済み
        }
        
        visited.insert(current);
        path.push(current);
        
        if let Some(neighbors) = graph.get(&current) {
            for &neighbor in neighbors {
                if self.detect_cycles(start, neighbor, graph, visited, path)? {
                    return Ok(true);
                }
            }
        }
        
        path.pop();
        Ok(false)
    }
    
    /// サイクルに関与するメソッドを取得
    fn get_cycle_methods(&self, actor_cycle: &[TypeId]) -> Result<Vec<FunctionId>, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        let mut cycle_methods = Vec::new();
        
        // 隣接するアクター間の呼び出しメソッドを特定
        for i in 0..actor_cycle.len() {
            let current = actor_cycle[i];
            let next = actor_cycle[(i + 1) % actor_cycle.len()];
            
            // currentアクターからnextアクターへの呼び出しメソッドを探す
            let current_methods = self.get_actor_methods(current)?;
            
            for &method_id in &current_methods {
                if let Some(called_methods) = self.result.actor_method_calls.get(&method_id) {
                    for &called_method in called_methods {
                        if let Some(called_actor) = self.get_actor_for_method(called_method)? {
                            if called_actor == next {
                                cycle_methods.push(method_id);
                                break;
                            }
                        }
                    }
                }
            }
        }
        
        Ok(cycle_methods)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::semantic::type_checker::TypeId;
    use crate::frontend::parser::ast::{FunctionId, ModuleId};
    use std::collections::{HashMap, HashSet};
    use std::sync::{Arc, Mutex};
    
    /// アクター分析のモックデータを構築するヘルパー構造体
    struct ActorAnalysisMockBuilder {
        actor_types: HashSet<TypeId>,
        actor_methods: HashMap<TypeId, Vec<FunctionId>>,
        method_to_actor: HashMap<FunctionId, TypeId>,
        actor_method_calls: HashMap<FunctionId, Vec<FunctionId>>,
        module_id: ModuleId,
    }
    
    impl ActorAnalysisMockBuilder {
        fn new(module_id: ModuleId) -> Self {
            Self {
                actor_types: HashSet::new(),
                actor_methods: HashMap::new(),
                method_to_actor: HashMap::new(),
                actor_method_calls: HashMap::new(),
                module_id,
            }
        }
        
        fn add_actor(&mut self, actor_id: TypeId) -> &mut Self {
            self.actor_types.insert(actor_id);
            self
        }
        
        fn add_actor_method(&mut self, actor_id: TypeId, method_id: FunctionId) -> &mut Self {
            self.actor_methods.entry(actor_id).or_insert_with(Vec::new).push(method_id);
            self.method_to_actor.insert(method_id, actor_id);
            self
        }
        
        fn add_method_call(&mut self, caller_id: FunctionId, callee_id: FunctionId) -> &mut Self {
            self.actor_method_calls.entry(caller_id).or_insert_with(Vec::new).push(callee_id);
            self
        }
        
        fn build(&self) -> ActorAnalysisResult {
            ActorAnalysisResult {
                actor_types: self.actor_types.clone(),
                actor_methods: self.actor_methods.clone(),
                method_to_actor: self.method_to_actor.clone(),
                actor_method_calls: self.actor_method_calls.clone(),
            }
        }
    }
    
    /// サイクル検出の基本テスト
    #[test]
    fn test_cycle_detection_basic() {
        // モジュールIDを作成
        let module_id = ModuleId::new(0);
        
        // アクターと関数のIDを作成
        let actor1 = TypeId::new(1);
        let actor2 = TypeId::new(2);
        let actor3 = TypeId::new(3);
        
        let method1 = FunctionId::new(1);
        let method2 = FunctionId::new(2);
        let method3 = FunctionId::new(3);
        
        // モックビルダーを使用してテストデータを構築
        let mut builder = ActorAnalysisMockBuilder::new(module_id);
        builder.add_actor(actor1)
               .add_actor(actor2)
               .add_actor(actor3)
               .add_actor_method(actor1, method1)
               .add_actor_method(actor2, method2)
               .add_actor_method(actor3, method3)
               .add_method_call(method1, method2)
               .add_method_call(method2, method3)
               .add_method_call(method3, method1);  // サイクルを作成
        
        let result = builder.build();
        
        // サイクル検出器を作成
        let mut detector = ActorCycleDetector::new(result);
        detector.set_module(Arc::new(Mutex::new(module_id)));
        
        // サイクル検出を実行
        let cycles = detector.detect_cycles().unwrap();
        
        // 検証
        assert_eq!(cycles.len(), 1, "1つのサイクルが検出されるべき");
        assert_eq!(cycles[0].len(), 3, "サイクルは3つのアクターを含むべき");
        
        // サイクルに関与するメソッドを取得
        let cycle_methods = detector.get_cycle_methods(&cycles[0]).unwrap();
        assert_eq!(cycle_methods.len(), 3, "サイクルは3つのメソッドを含むべき");
    }
    
    /// 複数のサイクルを含むケース
    #[test]
    fn test_multiple_cycles() {
        let module_id = ModuleId::new(0);
        
        // アクターと関数のIDを作成
        let actor1 = TypeId::new(1);
        let actor2 = TypeId::new(2);
        let actor3 = TypeId::new(3);
        let actor4 = TypeId::new(4);
        let actor5 = TypeId::new(5);
        
        let method1 = FunctionId::new(1);
        let method2 = FunctionId::new(2);
        let method3 = FunctionId::new(3);
        let method4 = FunctionId::new(4);
        let method5 = FunctionId::new(5);
        let method6 = FunctionId::new(6);
        
        // モックビルダーを使用してテストデータを構築
        let mut builder = ActorAnalysisMockBuilder::new(module_id);
        builder.add_actor(actor1)
               .add_actor(actor2)
               .add_actor(actor3)
               .add_actor(actor4)
               .add_actor(actor5)
               .add_actor_method(actor1, method1)
               .add_actor_method(actor2, method2)
               .add_actor_method(actor3, method3)
               .add_actor_method(actor4, method4)
               .add_actor_method(actor5, method5)
               .add_actor_method(actor5, method6)
               // サイクル1: 1->2->3->1
               .add_method_call(method1, method2)
               .add_method_call(method2, method3)
               .add_method_call(method3, method1)
               // サイクル2: 4->5->4
               .add_method_call(method4, method5)
               .add_method_call(method5, method4);
        
        let result = builder.build();
        
        // サイクル検出器を作成
        let mut detector = ActorCycleDetector::new(result);
        detector.set_module(Arc::new(Mutex::new(module_id)));
        
        // サイクル検出を実行
        let cycles = detector.detect_cycles().unwrap();
        
        // 検証
        assert_eq!(cycles.len(), 2, "2つのサイクルが検出されるべき");
        
        // サイクルの長さを確認
        let has_cycle_length_3 = cycles.iter().any(|cycle| cycle.len() == 3);
        let has_cycle_length_2 = cycles.iter().any(|cycle| cycle.len() == 2);
        
        assert!(has_cycle_length_3, "3つのアクターを含むサイクルが存在するべき");
        assert!(has_cycle_length_2, "2つのアクターを含むサイクルが存在するべき");
    }
    
    /// サイクルがない場合のテスト
    #[test]
    fn test_no_cycles() {
        let module_id = ModuleId::new(0);
        
        // アクターと関数のIDを作成
        let actor1 = TypeId::new(1);
        let actor2 = TypeId::new(2);
        let actor3 = TypeId::new(3);
        
        let method1 = FunctionId::new(1);
        let method2 = FunctionId::new(2);
        let method3 = FunctionId::new(3);
        
        // モックビルダーを使用してテストデータを構築 - サイクルなし
        let mut builder = ActorAnalysisMockBuilder::new(module_id);
        builder.add_actor(actor1)
               .add_actor(actor2)
               .add_actor(actor3)
               .add_actor_method(actor1, method1)
               .add_actor_method(actor2, method2)
               .add_actor_method(actor3, method3)
               .add_method_call(method1, method2)
               .add_method_call(method2, method3);
               // method3からmethod1への呼び出しがないのでサイクルなし
        
        let result = builder.build();
        
        // サイクル検出器を作成
        let mut detector = ActorCycleDetector::new(result);
        detector.set_module(Arc::new(Mutex::new(module_id)));
        
        // サイクル検出を実行
        let cycles = detector.detect_cycles().unwrap();
        
        // 検証
        assert_eq!(cycles.len(), 0, "サイクルは検出されるべきではない");
    }
    
    /// 自己参照サイクルのテスト
    #[test]
    fn test_self_referential_cycle() {
        let module_id = ModuleId::new(0);
        
        // アクターと関数のIDを作成
        let actor1 = TypeId::new(1);
        
        let method1 = FunctionId::new(1);
        let method2 = FunctionId::new(2);
        
        // モックビルダーを使用してテストデータを構築 - 自己参照サイクル
        let mut builder = ActorAnalysisMockBuilder::new(module_id);
        builder.add_actor(actor1)
               .add_actor_method(actor1, method1)
               .add_actor_method(actor1, method2)
               .add_method_call(method1, method2)
               .add_method_call(method2, method1);
        
        let result = builder.build();
        
        // サイクル検出器を作成
        let mut detector = ActorCycleDetector::new(result);
        detector.set_module(Arc::new(Mutex::new(module_id)));
        
        // サイクル検出を実行
        let cycles = detector.detect_cycles().unwrap();
        
        // 検証 - 自己参照は同じアクター内なのでサイクルとしてカウントしない
        assert_eq!(cycles.len(), 0, "同一アクター内の呼び出しはサイクルとしてカウントされるべきではない");
    }
    
    /// 複雑なグラフ構造でのサイクル検出テスト
    #[test]
    fn test_complex_graph_cycle_detection() {
        let module_id = ModuleId::new(0);
        
        // 10個のアクターと関数を作成
        let actors: Vec<TypeId> = (1..=10).map(TypeId::new).collect();
        let methods: Vec<FunctionId> = (1..=20).map(FunctionId::new).collect();
        
        // モックビルダーを使用して複雑なグラフを構築
        let mut builder = ActorAnalysisMockBuilder::new(module_id);
        
        // アクターとメソッドを追加
        for i in 0..10 {
            builder.add_actor(actors[i]);
            builder.add_actor_method(actors[i], methods[i]);
            builder.add_actor_method(actors[i], methods[i+10]);
        }
        
        // 複雑な呼び出し関係を構築
        // 1->2->3->4->5->6->7->8->9->10
        for i in 0..9 {
            builder.add_method_call(methods[i], methods[i+1]);
        }
        
        // サイクルを作成: 10->1
        builder.add_method_call(methods[9], methods[0]);
        
        // 追加の呼び出し関係
        builder.add_method_call(methods[2], methods[7]);
        builder.add_method_call(methods[5], methods[0]);
        builder.add_method_call(methods[8], methods[3]);
        
        let result = builder.build();
        
        // サイクル検出器を作成
        let mut detector = ActorCycleDetector::new(result);
        detector.set_module(Arc::new(Mutex::new(module_id)));
        
        // サイクル検出を実行
        let cycles = detector.detect_cycles().unwrap();
        
        // 検証 - 複数のサイクルが検出されるはず
        assert!(cycles.len() > 0, "少なくとも1つのサイクルが検出されるべき");
        
        // 最長のサイクルは10ノード
        let max_cycle_length = cycles.iter().map(|c| c.len()).max().unwrap_or(0);
        assert_eq!(max_cycle_length, 10, "最長のサイクルは10ノードであるべき");
    }
    
    /// エラーケースのテスト
    #[test]
    fn test_error_cases() {
        // モジュールが設定されていない場合
        let result = ActorAnalysisResult {
            actor_types: HashSet::new(),
            actor_methods: HashMap::new(),
            method_to_actor: HashMap::new(),
            actor_method_calls: HashMap::new(),
        };
        
        let detector = ActorCycleDetector::new(result);
        
        // モジュールが設定されていないのでエラーになるはず
        let cycles_result = detector.detect_cycles();
        assert!(cycles_result.is_err(), "モジュールが設定されていない場合はエラーを返すべき");
        assert_eq!(cycles_result.unwrap_err(), "モジュールが設定されていません");
    }
    
    /// パフォーマンステスト - 大規模グラフでの検出効率
    #[test]
    fn test_performance_large_graph() {
        let module_id = ModuleId::new(0);
        
        // 100個のアクターと200個のメソッドを作成
        let actors: Vec<TypeId> = (1..=100).map(TypeId::new).collect();
        let methods: Vec<FunctionId> = (1..=200).map(FunctionId::new).collect();
        
        let mut builder = ActorAnalysisMockBuilder::new(module_id);
        
        // アクターとメソッドを追加
        for i in 0..100 {
            builder.add_actor(actors[i]);
            builder.add_actor_method(actors[i], methods[i*2]);
            builder.add_actor_method(actors[i], methods[i*2+1]);
        }
        
        // 線形の呼び出し関係を構築 (サイクルなし)
        for i in 0..199 {
            builder.add_method_call(methods[i], methods[i+1]);
        }
        
        // 1つだけサイクルを追加
        builder.add_method_call(methods[199], methods[0]);
        
        let result = builder.build();
        
        // サイクル検出器を作成
        let mut detector = ActorCycleDetector::new(result);
        detector.set_module(Arc::new(Mutex::new(module_id)));
        
        // 時間計測開始
        let start = std::time::Instant::now();
        
        // サイクル検出を実行
        let cycles = detector.detect_cycles().unwrap();
        
        // 時間計測終了
        let duration = start.elapsed();
        
        // 検証
        assert!(cycles.len() > 0, "少なくとも1つのサイクルが検出されるべき");
        println!("大規模グラフ(100アクター, 200メソッド)でのサイクル検出時間: {:?}", duration);
        
        // 実行時間が合理的であることを確認 (10秒以内)
        assert!(duration.as_secs() < 10, "大規模グラフでのサイクル検出は10秒以内に完了すべき");
    }
    
    /// 並列処理の安全性テスト
    #[test]
    fn test_thread_safety() {
        use std::thread;
        
        let module_id = ModuleId::new(0);
        
        // テストデータを構築
        let mut builder = ActorAnalysisMockBuilder::new(module_id);
        for i in 1..=10 {
            let actor_id = TypeId::new(i);
            let method_id = FunctionId::new(i);
            builder.add_actor(actor_id)
                   .add_actor_method(actor_id, method_id);
            
            if i > 1 {
                builder.add_method_call(FunctionId::new(i-1), method_id);
            }
        }
        // サイクルを作成
        builder.add_method_call(FunctionId::new(10), FunctionId::new(1));
        
        let result = builder.build();
        let module = Arc::new(Mutex::new(module_id));
        
        // 複数スレッドで同時にサイクル検出を実行
        let mut handles = vec![];
        for _ in 0..5 {
            let result_clone = result.clone();
            let module_clone = Arc::clone(&module);
            
            let handle = thread::spawn(move || {
                let mut detector = ActorCycleDetector::new(result_clone);
                detector.set_module(module_clone);
                detector.detect_cycles()
            });
            
            handles.push(handle);
        }
        
        // 全てのスレッドの結果を検証
        for handle in handles {
            let cycles_result = handle.join().unwrap();
            assert!(cycles_result.is_ok(), "スレッドセーフな実行でエラーが発生しないこと");
            let cycles = cycles_result.unwrap();
            assert_eq!(cycles.len(), 1, "1つのサイクルが検出されるべき");
        }
    }
    
    /// 実際のユースケースに基づいたテスト - マイクロサービスアーキテクチャ
    #[test]
    fn test_microservice_architecture() {
        let module_id = ModuleId::new(0);
        
        // マイクロサービスを表すアクター
        let user_service = TypeId::new(1);
        let auth_service = TypeId::new(2);
        let product_service = TypeId::new(3);
        let order_service = TypeId::new(4);
        let payment_service = TypeId::new(5);
        let notification_service = TypeId::new(6);
        
        // 各サービスのメソッド
        let user_get = FunctionId::new(101);
        let user_create = FunctionId::new(102);
        let auth_verify = FunctionId::new(201);
        let auth_login = FunctionId::new(202);
        let product_get = FunctionId::new(301);
        let product_update = FunctionId::new(302);
        let order_create = FunctionId::new(401);
        let order_update = FunctionId::new(402);
        let payment_process = FunctionId::new(501);
        let payment_refund = FunctionId::new(502);
        let notification_send = FunctionId::new(601);
        let notification_schedule = FunctionId::new(602);
        
        // モックビルダーを使用してマイクロサービスアーキテクチャを構築
        let mut builder = ActorAnalysisMockBuilder::new(module_id);
        
        // アクターとメソッドを追加
        builder.add_actor(user_service)
               .add_actor(auth_service)
               .add_actor(product_service)
               .add_actor(order_service)
               .add_actor(payment_service)
               .add_actor(notification_service)
               .add_actor_method(user_service, user_get)
               .add_actor_method(user_service, user_create)
               .add_actor_method(auth_service, auth_verify)
               .add_actor_method(auth_service, auth_login)
               .add_actor_method(product_service, product_get)
               .add_actor_method(product_service, product_update)
               .add_actor_method(order_service, order_create)
               .add_actor_method(order_service, order_update)
               .add_actor_method(payment_service, payment_process)
               .add_actor_method(payment_service, payment_refund)
               .add_actor_method(notification_service, notification_send)
               .add_actor_method(notification_service, notification_schedule);
        
        // サービス間の呼び出し関係を構築
        // 正常なフロー (サイクルなし)
        builder.add_method_call(user_create, auth_verify)
               .add_method_call(auth_login, user_get)
               .add_method_call(order_create, product_get)
               .add_method_call(order_create, user_get)
               .add_method_call(order_update, payment_process)
               .add_method_call(payment_process, notification_send);
        
        // サイクルを作成するフロー
        builder.add_method_call(notification_schedule, order_update)
               .add_method_call(order_update, product_update)
               .add_method_call(product_update, notification_schedule);
        
        let result = builder.build();
        
        // サイクル検出器を作成
        let mut detector = ActorCycleDetector::new(result);
        detector.set_module(Arc::new(Mutex::new(module_id)));
        
        // サイクル検出を実行
        let cycles = detector.detect_cycles().unwrap();
        
        // 検証
        assert_eq!(cycles.len(), 1, "1つのサイクルが検出されるべき");
        
        // サイクルに含まれるアクターを確認
        let cycle = &cycles[0];
        assert!(cycle.contains(&notification_service), "通知サービスがサイクルに含まれるべき");
        assert!(cycle.contains(&order_service), "注文サービスがサイクルに含まれるべき");
        assert!(cycle.contains(&product_service), "商品サービスがサイクルに含まれるべき");
        
        // サイクルに関与するメソッドを取得
        let cycle_methods = detector.get_cycle_methods(cycle).unwrap();
        assert_eq!(cycle_methods.len(), 3, "サイクルは3つのメソッドを含むべき");
    }
}
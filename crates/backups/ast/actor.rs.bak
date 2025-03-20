// actor.rs - SwiftLightアクターシステム
//
// このモジュールは、SwiftLight言語のアクターモデルの実装を提供します。
// アクターは独立した実行単位であり、状態の共有ではなくメッセージパッシングによって
// 通信します。これにより、データ競合のない安全な並行処理が可能になります。
// 
// SwiftLightのアクターモデルは、Akka、Erlang/OTPおよびPonyの最良の部分を取り入れ、
// 静的型付け言語の安全性と組み合わせています。

use std::collections::{HashMap, HashSet, VecDeque, BTreeMap};
use std::sync::Arc;
use std::time::Duration;

use crate::middleend::ir::{
    BasicBlock, Function, Instruction, Module, Value, ValueId, 
    Type, TypeId, InstructionId, FunctionId, Parameter
};
use crate::frontend::ast;
use crate::frontend::semantic::type_checker::TypeCheckResult;
use crate::middleend::analysis::dataflow::DataflowAnalyzer;
use crate::middleend::analysis::lifetime::LifetimeAnalyzer;

/// アクター定義
#[derive(Debug, Clone)]
pub struct Actor {
    /// アクター名
    pub name: String,
    
    /// アクター型ID
    pub type_id: TypeId,
    
    /// アクター状態（フィールド）
    pub state: Vec<ActorField>,
    
    /// アクターメソッド
    pub methods: Vec<ActorMethod>,
    
    /// ライフサイクルフック
    pub lifecycle_hooks: HashMap<LifecycleEvent, FunctionId>,
    
    /// 監視対象アクター
    pub supervisees: Vec<TypeId>,
    
    /// 監視戦略
    pub supervision_strategy: SupervisionStrategy,
    
    /// タグ（デバッグ情報など）
    pub tags: HashSet<String>,
    
    /// メモリ分離領域
    pub memory_regions: Vec<MemoryRegion>,
    
    /// 形式検証プロパティ
    pub formal_properties: Vec<FormalProperty>,
    
    /// スケジューリング優先度
    pub scheduling_priority: SchedulingPriority,
    
    /// リソース制限
    pub resource_limits: ResourceLimits,
    
    /// 分散配置ポリシー
    pub distribution_policy: DistributionPolicy,
}

/// アクターフィールド
#[derive(Debug, Clone)]
pub struct ActorField {
    /// フィールド名
    pub name: String,
    
    /// フィールド型
    pub type_id: TypeId,
    
    /// 可変性フラグ
    pub is_mutable: bool,
    
    /// 保護レベル（private, protected, public）
    pub protection_level: ProtectionLevel,
    
    /// 初期値（存在する場合）
    pub initial_value: Option<ValueId>,
    
    /// メモリ領域
    pub memory_region: Option<usize>,
    
    /// 不変条件
    pub invariants: Vec<InvariantCondition>,
    
    /// 型レベル制約
    pub type_constraints: Vec<TypeConstraint>,
}

/// アクターメソッド
#[derive(Debug, Clone)]
pub struct ActorMethod {
    /// メソッド名
    pub name: String,
    
    /// 関数ID
    pub function_id: FunctionId,
    
    /// 保護レベル（private, protected, public）
    pub protection_level: ProtectionLevel,
    
    /// メソッドの種類
    pub kind: ActorMethodKind,
    
    /// 並行処理方式
    pub concurrency_mode: ConcurrencyMode,
    
    /// アクセス対象の状態（フィールド）
    pub accessed_state: HashSet<usize>,
    
    /// 事前条件
    pub preconditions: Vec<Condition>,
    
    /// 事後条件
    pub postconditions: Vec<Condition>,
    
    /// 副作用宣言
    pub side_effects: Vec<SideEffect>,
    
    /// タイムアウト設定
    pub timeout: Option<Duration>,
    
    /// 再試行ポリシー
    pub retry_policy: Option<RetryPolicy>,
    
    /// スロットリング設定
    pub throttling: Option<ThrottlingPolicy>,
    
    /// トレース設定
    pub tracing: Option<TracingConfig>,
}

/// 保護レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProtectionLevel {
    /// プライベート（アクター内でのみアクセス可能）
    Private,
    
    /// プロテクテッド（アクターと子アクターでアクセス可能）
    Protected,
    
    /// パブリック（どこからでもアクセス可能）
    Public,
    
    /// 内部（同じモジュール内でアクセス可能）
    Internal,
    
    /// 友好（指定されたアクターからアクセス可能）
    Friend(Vec<TypeId>),
}

/// アクターメソッドの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActorMethodKind {
    /// 通常メソッド
    Regular,
    
    /// 初期化メソッド
    Initializer,
    
    /// メッセージハンドラ
    MessageHandler,
    
    /// 定期実行メソッド
    Periodic(Duration),
    
    /// 監視メソッド
    Supervisor,
    
    /// 状態遷移メソッド
    StateTransition,
    
    /// 回復メソッド
    Recovery,
    
    /// 分散調整メソッド
    DistributedCoordination,
    
    /// 自己最適化メソッド
    SelfOptimizing,
}

/// 並行処理方式
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConcurrencyMode {
    /// 排他的（アクター状態へのアクセスをロック）
    Exclusive,
    
    /// 非同期（非同期実行を許可）
    Async,
    
    /// リードオンリー（読み取り専用アクセスを許可）
    ReadOnly,
    
    /// アイソレーション（状態の一部を分離）
    Isolated(Vec<usize>),
    
    /// 並列（複数のスレッドで並列実行）
    Parallel(ParallelExecutionStrategy),
    
    /// トランザクショナル（STMベースのトランザクション）
    Transactional(IsolationLevel),
    
    /// 時間的分離（時間的に分離された実行）
    TemporalIsolation(TemporalConstraint),
    
    /// 優先度ベース（優先度に基づく実行）
    PriorityBased(u8),
}

/// 並列実行戦略
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParallelExecutionStrategy {
    /// データ並列（同じ操作を異なるデータに適用）
    DataParallel,
    
    /// タスク並列（異なる操作を並列に実行）
    TaskParallel,
    
    /// パイプライン（連続した操作をパイプライン化）
    Pipeline,
    
    /// 分割統治（問題を再帰的に分割して解決）
    DivideAndConquer,
}

/// トランザクション分離レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IsolationLevel {
    /// 読み取りコミット済み
    ReadCommitted,
    
    /// 繰り返し読み取り可能
    RepeatableRead,
    
    /// 直列化可能
    Serializable,
    
    /// スナップショット分離
    SnapshotIsolation,
}

/// 時間的制約
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TemporalConstraint {
    /// 最小実行間隔
    pub min_interval: Duration,
    
    /// 最大実行時間
    pub max_execution_time: Duration,
    
    /// 時間的依存関係
    pub dependencies: Vec<FunctionId>,
}

/// ライフサイクルイベント
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LifecycleEvent {
    /// アクター初期化前
    PreInit,
    
    /// アクター初期化後
    PostInit,
    
    /// メッセージ受信前
    PreReceive,
    
    /// メッセージ処理後
    PostProcess,
    
    /// エラー発生時
    OnError,
    
    /// 終了前
    PreTerminate,
    
    /// 再起動前
    PreRestart,
    
    /// 再起動後
    PostRestart,
    
    /// 一時停止前
    PreSuspend,
    
    /// 再開後
    PostResume,
    
    /// メモリ圧迫時
    OnMemoryPressure,
    
    /// システム過負荷時
    OnSystemOverload,
}

/// 監視戦略
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupervisionStrategy {
    /// 一対一再起動（障害が発生したアクターのみ再起動）
    OneForOne,
    
    /// 一対全再起動（障害が発生した場合、すべての子アクターを再起動）
    OneForAll,
    
    /// 全対一再起動（すべての子アクターが障害を起こした場合に再起動）
    AllForOne,
    
    /// 段階的再起動（障害の種類に応じて段階的に対応）
    Escalate,
    
    /// カスタム戦略（ユーザー定義の戦略）
    Custom(FunctionId),
}

/// アクターメッセージ
#[derive(Debug, Clone)]
pub struct ActorMessage {
    /// メッセージID
    pub id: usize,
    
    /// メッセージ型
    pub message_type: TypeId,
    
    /// ターゲットアクター
    pub target_actor: TypeId,
    
    /// ターゲットメソッド
    pub target_method: FunctionId,
    
    /// メッセージペイロード
    pub payload: Vec<ValueId>,
    
    /// 送信元アクター（存在する場合）
    pub sender: Option<TypeId>,
    
    /// 応答メッセージID（存在する場合）
    pub reply_to: Option<usize>,
    
    /// タイムアウト（ミリ秒）
    pub timeout_ms: Option<u64>,
    
    /// 優先度
    pub priority: MessagePriority,
    
    /// 配信保証
    pub delivery_guarantee: DeliveryGuarantee,
    
    /// 冪等性キー
    pub idempotency_key: Option<String>,
    
    /// メッセージ有効期限
    pub expiration: Option<Duration>,
    
    /// トレースコンテキスト
    pub trace_context: Option<TraceContext>,
    
    /// セキュリティコンテキスト
    pub security_context: Option<SecurityContext>,
}

/// メッセージ優先度
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MessagePriority {
    /// 最高優先度（システムメッセージ）
    System,
    
    /// 高優先度
    High,
    
    /// 通常優先度
    Normal,
    
    /// 低優先度
    Low,
    
    /// バックグラウンド
    Background,
    
    /// カスタム優先度
    Custom(u8),
}

/// 配信保証
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeliveryGuarantee {
    /// 最大1回配信（配信されないこともある）
    AtMostOnce,
    
    /// 少なくとも1回配信（重複する可能性あり）
    AtLeastOnce,
    
    /// 正確に1回配信
    ExactlyOnce,
    
    /// 順序保証配信
    Ordered,
}

/// トレースコンテキスト
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraceContext {
    /// トレースID
    pub trace_id: String,
    
    /// スパンID
    pub span_id: String,
    
    /// 親スパンID
    pub parent_span_id: Option<String>,
    
    /// サンプリングフラグ
    pub sampled: bool,
    
    /// バゲージアイテム（メタデータ）
    pub baggage: HashMap<String, String>,
}

/// セキュリティコンテキスト
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SecurityContext {
    /// 認証情報
    pub authentication: Option<Authentication>,
    
    /// 認可情報
    pub authorization: Option<Authorization>,
    
    /// 暗号化フラグ
    pub encrypted: bool,
    
    /// 署名
    pub signature: Option<Vec<u8>>,
}

/// 認証情報
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Authentication {
    /// プリンシパルID
    pub principal_id: String,
    
    /// 認証方式
    pub method: AuthenticationMethod,
    
    /// 認証トークン
    pub token: Option<String>,
}

/// 認証方式
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AuthenticationMethod {
    /// トークンベース
    Token,
    
    /// 証明書ベース
    Certificate,
    
    /// 多要素認証
    MultiFactorAuth,
    
    /// カスタム認証
    Custom(String),
}

/// 認可情報
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Authorization {
    /// ロール
    pub roles: Vec<String>,
    
    /// パーミッション
    pub permissions: Vec<String>,
    
    /// スコープ
    pub scopes: Vec<String>,
}

/// メモリ領域
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRegion {
    /// 領域名
    pub name: String,
    
    /// 領域ID
    pub id: usize,
    
    /// アクセス権限
    pub access_permissions: AccessPermissions,
    
    /// メモリ最適化ヒント
    pub optimization_hints: Vec<MemoryOptimizationHint>,
    
    /// 分離レベル
    pub isolation_level: MemoryIsolationLevel,
}

/// アクセス権限
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessPermissions {
    /// 読み取り権限
    pub read: bool,
    
    /// 書き込み権限
    pub write: bool,
    
    /// 実行権限
    pub execute: bool,
}

/// メモリ最適化ヒント
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryOptimizationHint {
    /// キャッシュ局所性
    CacheLocality,
    
    /// 頻繁にアクセス
    FrequentlyAccessed,
    
    /// 稀にアクセス
    RarelyAccessed,
    
    /// 連続アクセス
    SequentialAccess,
    
    /// ランダムアクセス
    RandomAccess,
    
    /// 読み取り主体
    ReadMostly,
    
    /// 書き込み主体
    WriteMostly,
}

/// メモリ分離レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryIsolationLevel {
    /// 共有（複数のアクターで共有可能）
    Shared,
    
    /// アクター専用（アクター内でのみ共有）
    ActorPrivate,
    
    /// メソッド専用（特定のメソッド内でのみアクセス可能）
    MethodPrivate,
    
    /// スレッド専用（特定のスレッドでのみアクセス可能）
    ThreadLocal,
}

/// 形式検証プロパティ
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormalProperty {
    /// プロパティ名
    pub name: String,
    
    /// プロパティ種類
    pub kind: PropertyKind,
    
    /// 検証式
    pub expression: String,
    
    /// 検証レベル
    pub verification_level: VerificationLevel,
}

/// プロパティ種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyKind {
    /// 安全性（悪いことは起きない）
    Safety,
    
    /// 活性（良いことはいつか起きる）
    Liveness,
    
    /// 公平性（無限に待たされることはない）
    Fairness,
    
    /// デッドロックフリー
    DeadlockFree,
    
    /// 決定性
    Deterministic,
}

/// 検証レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationLevel {
    /// 型チェック
    TypeChecked,
    
    /// 静的解析
    StaticallyAnalyzed,
    
    /// モデル検査
    ModelChecked,
    
    /// 定理証明
    TheoremProven,
}

/// スケジューリング優先度
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SchedulingPriority {
    /// リアルタイム
    RealTime,
    
    /// 高優先度
    High,
    
    /// 通常優先度
    Normal,
    
    /// 低優先度
    Low,
    
    /// バックグラウンド
    Background,
    
    /// カスタム優先度
    Custom(u8),
}

/// リソース制限
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceLimits {
    /// メモリ使用量上限（バイト）
    pub memory_limit: Option<usize>,
    
    /// CPU時間上限（ミリ秒）
    pub cpu_time_limit: Option<u64>,
    
    /// 同時実行数上限
    pub concurrency_limit: Option<usize>,
    
    /// メッセージキュー長上限
    pub message_queue_limit: Option<usize>,
    
    /// ネットワーク帯域幅上限（バイト/秒）
    pub network_bandwidth_limit: Option<usize>,
}

/// 分散配置ポリシー
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DistributionPolicy {
    /// ローカル実行のみ
    LocalOnly,
    
    /// 任意のノードで実行可能
    AnyNode,
    
    /// 特定のノードで実行
    SpecificNode(String),
    
    /// ノードグループで実行
    NodeGroup(String),
    
    /// 地理的制約
    GeoLocated(String),
    
    /// アフィニティベース（関連アクターと同じノード）
    Affinity(Vec<TypeId>),
    
    /// 負荷分散
    LoadBalanced,
}

/// 条件式
#[derive(Debug, Clone, PartialEq)]
pub struct Condition {
    /// 条件名
    pub name: String,
    
    /// 条件式
    pub expression: String,
    
    /// エラーメッセージ
    pub error_message: Option<String>,
    
    /// 検証レベル
    pub verification_level: VerificationLevel,
}

/// 不変条件
#[derive(Debug, Clone, PartialEq)]
pub struct InvariantCondition {
    /// 条件名
    pub name: String,
    
    /// 条件式
    pub expression: String,
    
    /// 検証レベル
    pub verification_level: VerificationLevel,
}

/// 型制約
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeConstraint {
    /// 制約名
    pub name: String,
    
    /// 制約式
    pub expression: String,
}

/// 副作用
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideEffect {
    /// 副作用の種類
    pub kind: SideEffectKind,
    
    /// 影響範囲
    pub scope: SideEffectScope,
    
    /// 説明
    pub description: String,
}

/// 副作用の種類
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SideEffectKind {
    /// 状態変更
    StateModification,
    
    /// I/O操作
    IO,
    
    /// メッセージ送信
    MessageSend,
    
    /// アクター生成
    ActorCreation,
    
    /// アクター終了
    ActorTermination,
    
    /// 外部システム呼び出し
    ExternalSystemCall,
}

/// 副作用の範囲
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SideEffectScope {
    /// ローカル（アクター内）
    Local,
    
    /// グローバル（システム全体）
    Global,
    
    /// 限定的（特定のアクターのみ）
    Limited(Vec<TypeId>),
}

/// 再試行ポリシー
#[derive(Debug, Clone, PartialEq)]
pub struct RetryPolicy {
    /// 最大再試行回数
    pub max_retries: usize,
    
    /// 再試行間隔（初期値）
    pub initial_delay: Duration,
    
    /// 再試行間隔の増加係数
    pub backoff_factor: f64,
    
    /// 最大再試行間隔
    pub max_delay: Duration,
    
    /// 再試行対象の例外
    pub retry_on: Vec<String>,
}

/// スロットリングポリシー
#[derive(Debug, Clone, PartialEq)]
pub struct ThrottlingPolicy {
    /// 単位時間あたりの最大呼び出し回数
    pub rate_limit: usize,
    
    /// 時間単位
    pub time_unit: Duration,
    
    /// バーストトークン数
    pub burst_tokens: usize,
    
    /// スロットリング戦略
    pub strategy: ThrottlingStrategy,
}

/// スロットリング戦略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThrottlingStrategy {
    /// 拒否（制限超過時に拒否）
    Reject,
    
    /// 待機（制限超過時に待機）
    Wait,
    
    /// シェーピング（トラフィックを整形）
    Shape,
}

/// トレース設定
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TracingConfig {
    /// トレースレベル
    pub level: TracingLevel,
    
    /// サンプリングレート（0.0-1.0）
    pub sampling_rate: f64,
    
    /// トレース対象
    pub trace_targets: Vec<TraceTarget>,
}

/// トレースレベル
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TracingLevel {
    /// エラーのみ
    Error,
    
    /// 警告以上
    Warning,
    
    /// 情報以上
    Info,
    
    /// デバッグ以上
    Debug,
    
    /// トレース（最も詳細）
    Trace,
}

/// トレース対象
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceTarget {
    /// メソッド呼び出し
    MethodCall,
    
    /// メッセージ送受信
    MessageExchange,
    
    /// 状態変更
    StateChange,
    
    /// 例外発生
    Exception,
    
    /// パフォーマンスメトリクス
    Performance,
}

/// アクターシステム
#[derive(Debug)]
pub struct ActorSystem {
    /// アクター定義一覧
    pub actors: HashMap<TypeId, Actor>,
    
    /// アクター間メッセージ型一覧
    pub message_types: HashSet<TypeId>,
    
    /// アクター階層グラフ（親 -> 子）
    pub hierarchy: HashMap<TypeId, Vec<TypeId>>,
    
    /// アクター間通信パターン
    pub communication_patterns: HashMap<TypeId, HashMap<TypeId, CommunicationPattern>>,
    
    /// アクター状態遷移グラフ
    pub state_transitions: HashMap<TypeId, StateTransitionGraph>,
    
    /// 形式検証プロパティ
    pub system_properties: Vec<FormalProperty>,
    
    /// モジュール
    module: Option<Module>,
    
    /// 分散配置計画
    pub distribution_plan: Option<DistributionPlan>,
    
    /// パフォーマンスプロファイル
    pub performance_profile: HashMap<TypeId, ActorPerformanceProfile>,
    
    /// セキュリティポリシー
    pub security_policy: Option<SecurityPolicy>,
}

/// 通信パターン
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommunicationPattern {
    /// 要求/応答
    RequestResponse,
    
    /// 一方向通知
    OneWayNotification,
    
    /// 発行/購読
    PublishSubscribe,
    
    /// ブロードキャスト
    Broadcast,
    
    /// 集約
    Aggregation,
    
    /// 分散
    Scatter,
    
    /// ルーティング
    Routing(RoutingStrategy),
}

/// ルーティング戦略
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingStrategy {
    /// ラウンドロビン
    RoundRobin,
    
    /// 最小負荷
    LeastBusy,
    
    /// 一貫性ハッシュ
    ConsistentHashing,
    
    /// コンテンツベース
    ContentBased(String),
}

/// 状態遷移グラフ
#[derive(Debug, Clone)]
pub struct StateTransitionGraph {
    /// 状態一覧
    pub states: Vec<ActorState>,
    
    /// 遷移一覧
    pub transitions: Vec<StateTransition>,
    
    /// 初期状態
    pub initial_state: usize,
}

/// アクター状態
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorState {
    /// 状態名
    pub name: String,
    
    /// 状態ID
    pub id: usize,
    
    /// 状態不変条件
    pub invariants: Vec<InvariantCondition>,
    
    /// 有効なメッセージハンドラ
    pub valid_handlers: Vec<FunctionId>,
}

/// 状態遷移
#[derive(Debug, Clone)]
pub struct StateTransition {
    /// 遷移元状態ID
    pub from_state: usize,
    
    /// 遷移先状態ID
    pub to_state: usize,
    
    /// トリガーとなるメッセージ型
    pub trigger_message: Option<TypeId>,
    
    /// トリガーとなるメソッド
    pub trigger_method: Option<FunctionId>,
    
    /// 遷移条件
    pub condition: Option<String>,
    
    /// 遷移アクション
    pub action: Option<FunctionId>,
}

/// 分散配置計画
#[derive(Debug, Clone)]
pub struct DistributionPlan {
    /// ノード定義
    pub nodes: Vec<NodeDefinition>,
    
    /// アクター配置
    pub actor_placements: HashMap<TypeId, Vec<usize>>,
    
    /// レプリケーション戦略
    pub replication_strategy: ReplicationStrategy,
    
    /// パーティショニング戦略
    pub partitioning_strategy: Option<PartitioningStrategy>,
    
    /// 障害検出戦略
    pub failure_detection_strategy: Option<FailureDetectionStrategy>,
    
    /// パフォーマンスプロファイル
    pub performance_profile: HashMap<TypeId, ActorPerformanceProfile>,
}

impl ActorSystem { 
    /// 新しいアクターシステムを作成
    pub fn new() -> Self {
        Self {
            actors: HashMap::new(),
            message_types: HashSet::new(),
            hierarchy: HashMap::new(),
            module: None,
        }
    }
    
    /// モジュールを設定
    pub fn set_module(&mut self, module: Module) {
        self.module = Some(module);
    }
    
    /// アクターシステムを解析して構築
    pub fn analyze(&mut self) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // アクター型を検出
        self.detect_actor_types()?;
        
        // メッセージ型を検出
        self.detect_message_types()?;
        
        // アクター階層を構築
        self.build_actor_hierarchy()?;
        
        // アクターメソッドの並行処理方式を分析
        self.analyze_concurrency_modes()?;
        
        Ok(())
    }
    
    /// アクター型を検出
    fn detect_actor_types(&mut self) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // アクター宣言を持つ型を検出
        for (type_id, ty) in &module.types {
            // 型がアクタートレイトを実装しているかチェック
            if self.is_actor_type(*type_id, ty)? {
                // アクター定義を作成
                let actor = self.create_actor_definition(*type_id, ty)?;
                self.actors.insert(*type_id, actor);
            }
        }
        
        Ok(())
    }
    /// 型がアクター型かどうかを判定
    fn is_actor_type(&self, type_id: TypeId, ty: &Type) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // アクター型の条件を厳密にチェック
        match ty {
            Type::Struct(name, fields) => {
                // 1. 型が「Actor」トレイトを実装しているかチェック
                let implements_actor_trait = module.trait_implementations.iter()
                    .any(|(impl_type_id, trait_id)| {
                        *impl_type_id == type_id && 
                        module.is_actor_trait(*trait_id).unwrap_or(false)
                    });
                
                if !implements_actor_trait {
                    return Ok(false);
                }
                
                // 2. アクターの状態管理に必要な特定のフィールドを持っているか検証
                let has_required_fields = self.validate_actor_fields(fields)?;
                
                // 3. 必要なライフサイクルメソッドが実装されているかチェック
                let has_lifecycle_methods = self.check_actor_lifecycle_methods(type_id)?;
                
                // 4. メッセージハンドラメソッドが少なくとも1つ存在するか確認
                let has_message_handlers = self.has_message_handlers(type_id)?;
                
                // 5. アクターの並行性制約を満たしているか検証
                let satisfies_concurrency_constraints = self.validate_actor_concurrency(type_id)?;
                
                // 6. アクターの状態分離（ステート分離原則）を満たしているか検証
                let satisfies_state_isolation = self.validate_state_isolation(type_id)?;
                
                // すべての条件を満たす場合のみアクター型と判定
                Ok(implements_actor_trait && 
                   has_required_fields && 
                   has_lifecycle_methods && 
                   has_message_handlers && 
                   satisfies_concurrency_constraints && 
                   satisfies_state_isolation)
            },
            Type::Enum(name, variants) => {
                // 列挙型もアクターになり得る（状態機械として機能する場合）
                let implements_actor_trait = module.trait_implementations.iter()
                    .any(|(impl_type_id, trait_id)| {
                        *impl_type_id == type_id && 
                        module.is_actor_trait(*trait_id).unwrap_or(false)
                    });
                
                if !implements_actor_trait {
                    return Ok(false);
                }
                
                // 列挙型アクターの追加検証
                let has_valid_variants = self.validate_actor_enum_variants(variants)?;
                let has_lifecycle_methods = self.check_actor_lifecycle_methods(type_id)?;
                let has_message_handlers = self.has_message_handlers(type_id)?;
                
                Ok(implements_actor_trait && 
                   has_valid_variants && 
                   has_lifecycle_methods && 
                   has_message_handlers)
            },
            Type::Trait(name, _) => {
                // トレイトがアクタートレイトかどうかをチェック
                module.is_actor_trait(type_id)
            },
            _ => Ok(false), // その他の型はアクターにはなれない
        }
    }
    
    /// アクターに必要なフィールドを検証
    fn validate_actor_fields(&self, fields: &[TypeId]) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // アクターの状態フィールドが適切に定義されているか確認
        let mut has_state_field = false;
        let mut has_mailbox_field = false;
        let mut has_context_field = false;
        
        for &field_type_id in fields {
            let field_type = module.types.get(&field_type_id)
                .ok_or_else(|| format!("型ID {}の情報が見つかりません", field_type_id))?;
            
            match field_type {
                Type::Generic(name, _) if name.contains("ActorState") => {
                    has_state_field = true;
                },
                Type::Generic(name, _) if name.contains("Mailbox") => {
                    has_mailbox_field = true;
                },
                Type::Generic(name, _) if name.contains("ActorContext") => {
                    has_context_field = true;
                },
                _ => {}
            }
        }
        
        // 必須フィールドが存在するか、または自動生成可能かを確認
        Ok(has_state_field && has_mailbox_field && has_context_field)
    }
    
    /// アクターの列挙型バリアントを検証
    fn validate_actor_enum_variants(&self, variants: &[EnumVariant]) -> Result<bool, String> {
        // 各バリアントが有効なアクター状態を表現しているか確認
        for variant in variants {
            // バリアントがアクター状態遷移に適しているか検証
            if !self.is_valid_actor_state_variant(&variant)? {
                return Ok(false);
            }
        }
        
        // 少なくとも初期状態と終了状態のバリアントが存在するか確認
        let has_initial_state = variants.iter().any(|v| self.is_initial_state_variant(v)?);
        let has_terminal_state = variants.iter().any(|v| self.is_terminal_state_variant(v)?);
        
        Ok(has_initial_state && has_terminal_state && !variants.is_empty())
    }
    
    /// バリアントがアクター状態として有効か検証
    fn is_valid_actor_state_variant(&self, variant: &EnumVariant) -> Result<bool, String> {
        // バリアントの構造がアクター状態として適切か検証
        // 例: 各状態が適切なメッセージハンドラを持っているか
        
        match &variant.data {
            EnumVariantData::Unit => Ok(true), // 単純な状態
            EnumVariantData::Tuple(types) => {
                // タプル型バリアントが適切なデータ型を持っているか確認
                self.validate_actor_state_data_types(types)
            },
            EnumVariantData::Struct(fields) => {
                // 構造体型バリアントのフィールドが適切か確認
                self.validate_actor_state_fields(fields)
            }
        }
    }
    
    /// アクター状態のデータ型を検証
    fn validate_actor_state_data_types(&self, types: &[TypeId]) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // 状態データ型が安全に共有可能か検証
        for &type_id in types {
            if !self.is_safely_shareable_type(type_id)? {
                return Ok(false);
            }
        }
        
        Ok(true)
    }
    
    /// アクター状態のフィールドを検証
    fn validate_actor_state_fields(&self, fields: &[EnumVariantField]) -> Result<bool, String> {
        // 各フィールドが適切な型と可視性を持っているか確認
        for field in fields {
            if !self.is_safely_shareable_type(field.type_id)? {
                return Ok(false);
            }
        }
        
        Ok(true)
    }
    
    /// 型が安全に共有可能かどうかを判定
    fn is_safely_shareable_type(&self, type_id: TypeId) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // 型が Send + Sync トレイトを実装しているか確認
        let is_send = module.implements_trait(type_id, module.get_send_trait_id()?)?;
        let is_sync = module.implements_trait(type_id, module.get_sync_trait_id()?)?;
        
        Ok(is_send && is_sync)
    }
    
    /// バリアントが初期状態を表すかどうかを判定
    fn is_initial_state_variant(&self, variant: &EnumVariant) -> Result<bool, String> {
        // 初期状態を表すバリアントの特徴を確認
        // 例: 名前に "Initial" や "Start" を含む、または特別な属性が付与されている
        
        if variant.name.contains("Initial") || variant.name.contains("Start") {
            return Ok(true);
        }
        
        // 属性をチェック
        for attr in &variant.attributes {
            if attr.name == "initial_state" {
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    /// バリアントが終了状態を表すかどうかを判定
    fn is_terminal_state_variant(&self, variant: &EnumVariant) -> Result<bool, String> {
        // 終了状態を表すバリアントの特徴を確認
        
        if variant.name.contains("Terminal") || variant.name.contains("End") || 
           variant.name.contains("Final") || variant.name.contains("Stopped") {
            return Ok(true);
        }
        
        // 属性をチェック
        for attr in &variant.attributes {
            if attr.name == "terminal_state" {
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    /// アクターのライフサイクルメソッドをチェック
    fn check_actor_lifecycle_methods(&self, type_id: TypeId) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // 必須ライフサイクルメソッドのリスト
        let required_methods = ["init", "pre_start", "post_stop"];
        let mut found_methods = 0;
        
        for (func_id, function) in &module.functions {
            // この関数が指定された型のメソッドかどうかをチェック
            if self.is_method_of_actor(*func_id, type_id)? {
                if required_methods.contains(&function.name.as_str()) {
                    found_methods += 1;
                }
            }
        }
        
        // すべての必須メソッドが実装されているか、または自動生成可能か確認
        Ok(found_methods >= 1) // 少なくとも1つのライフサイクルメソッドが必要
    }
    
    /// メッセージハンドラメソッドが存在するかチェック
    fn has_message_handlers(&self, type_id: TypeId) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        for (func_id, function) in &module.functions {
            // この関数が指定された型のメソッドかどうかをチェック
            if self.is_method_of_actor(*func_id, type_id)? {
                // メッセージハンドラの特徴をチェック
                if self.is_message_handler(function)? {
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }
    
    /// 関数がメッセージハンドラかどうかを判定
    fn is_message_handler(&self, function: &Function) -> Result<bool, String> {
        // メッセージハンドラの特徴:
        // 1. 第一引数が &self または &mut self
        // 2. 第二引数がメッセージ型
        
        if function.parameters.len() < 2 {
            return Ok(false);
        }
        
        // 第一引数が self 参照かチェック
        let first_param = &function.parameters[0];
        if !first_param.name.contains("self") {
            return Ok(false);
        }
        
        // 第二引数がメッセージ型かチェック
        let second_param = &function.parameters[1];
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // メッセージ型の特徴をチェック
        let param_type = module.types.get(&second_param.type_id)
            .ok_or_else(|| format!("型ID {}の情報が見つかりません", second_param.type_id))?;
        
        // メッセージ型の条件:
        // - Message トレイトを実装している
        // - または #[message] 属性が付与されている
        // - または名前に "Message" や "Msg" を含む
        
        let is_message_trait = module.implements_trait(second_param.type_id, module.get_message_trait_id()?)?;
        
        let has_message_attr = function.attributes.iter()
            .any(|attr| attr.name == "message" || attr.name == "handler");
        
        let type_name = match param_type {
            Type::Struct(name, _) => name,
            Type::Enum(name, _) => name,
            Type::Tuple(_) => "Tuple",
            _ => "",
        };
        
        let name_suggests_message = type_name.contains("Message") || type_name.contains("Msg");
        
        Ok(is_message_trait || has_message_attr || name_suggests_message)
    }
    
    /// アクターの並行性制約を検証
    fn validate_actor_concurrency(&self, type_id: TypeId) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // アクターの並行処理モデルが一貫しているか確認
        // 例: 排他的アクセスと共有アクセスが適切に分離されているか
        
        let mut has_exclusive_methods = false;
        let mut has_shared_methods = false;
        
        for (func_id, function) in &module.functions {
            if self.is_method_of_actor(*func_id, type_id)? {
                let concurrency_mode = self.determine_method_concurrency_mode(function)?;
                
                match concurrency_mode {
                    ConcurrencyMode::Exclusive => has_exclusive_methods = true,
                    ConcurrencyMode::Shared => has_shared_methods = true,
                    ConcurrencyMode::ReadOnly => has_shared_methods = true,
                    _ => {}
                }
                
                // 同じメソッド内で排他的アクセスと共有アクセスが混在していないか確認
                if !self.validate_method_concurrency_consistency(function)? {
                    return Ok(false);
                }
            }
        }
        
        // アクターが少なくとも1つのメソッドを持っているか確認
        Ok(has_exclusive_methods || has_shared_methods)
    }
    
    /// メソッドの並行性モードを判定
    fn determine_method_concurrency_mode(&self, function: &Function) -> Result<ConcurrencyMode, String> {
        // メソッドの属性と引数から並行性モードを判定
        
        // 属性による明示的な指定を優先
        for attr in &function.attributes {
            match attr.name.as_str() {
                "exclusive" => return Ok(ConcurrencyMode::Exclusive),
                "shared" => return Ok(ConcurrencyMode::Shared),
                "read_only" => return Ok(ConcurrencyMode::ReadOnly),
                "async" => return Ok(ConcurrencyMode::Async),
                _ => {}
            }
        }
        
        // 引数の型から判定
        if function.parameters.is_empty() {
            return Ok(ConcurrencyMode::None);
        }
        
        let first_param = &function.parameters[0];
        if first_param.name.contains("self") {
            if first_param.is_mutable {
                return Ok(ConcurrencyMode::Exclusive);
            } else {
                return Ok(ConcurrencyMode::ReadOnly);
            }
        }
        
        // デフォルトは排他モード
        Ok(ConcurrencyMode::Exclusive)
    }
    
    /// メソッド内の並行性の一貫性を検証
    fn validate_method_concurrency_consistency(&self, function: &Function) -> Result<bool, String> {
        // メソッド内で排他的アクセスと共有アクセスが適切に分離されているか確認
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // 関数の本体を解析して、状態へのアクセスパターンを確認
        let mut has_exclusive_access = false;
        let mut has_shared_access = false;
        let mut has_readonly_access = false;
        
        // 関数の本体がない場合は一貫性があると見なす
        if function.body.is_empty() {
            return Ok(true);
        }
        
        // 関数内の各ステートメントを解析
        for stmt in &function.body {
            match stmt {
                Statement::FieldAccess(expr) => {
                    // フィールドアクセスの種類を判定
                    let access_mode = self.analyze_field_access_mode(expr)?;
                    match access_mode {
                        AccessMode::Write => has_exclusive_access = true,
                        AccessMode::Read => has_shared_access = true,
                        AccessMode::ReadOnly => has_readonly_access = true,
                    }
                },
                Statement::MethodCall(method_call) => {
                    // 呼び出されるメソッドの並行性モードを取得
                    if let Some(called_method) = self.find_method_by_name(&method_call.method_name)? {
                        let called_mode = self.determine_method_concurrency_mode(called_method)?;
                        match called_mode {
                            ConcurrencyMode::Exclusive => has_exclusive_access = true,
                            ConcurrencyMode::Shared => has_shared_access = true,
                            ConcurrencyMode::ReadOnly => has_readonly_access = true,
                            _ => {}
                        }
                    }
                },
                Statement::Assignment(lhs, rhs) => {
                    // 代入文の左辺が自身のフィールドの場合は排他アクセス
                    if self.is_self_field_access(lhs)? {
                        has_exclusive_access = true;
                    }
                    
                    // 右辺の式を解析
                    self.analyze_expression_access_mode(rhs, &mut has_exclusive_access, &mut has_shared_access, &mut has_readonly_access)?;
                },
                Statement::Expression(expr) => {
                    // 式のアクセスモードを解析
                    self.analyze_expression_access_mode(expr, &mut has_exclusive_access, &mut has_shared_access, &mut has_readonly_access)?;
                },
                Statement::If(condition, then_block, else_block) => {
                    // 条件式のアクセスモードを解析
                    self.analyze_expression_access_mode(condition, &mut has_exclusive_access, &mut has_shared_access, &mut has_readonly_access)?;
                    
                    // then ブロックの各ステートメントを解析
                    for stmt in then_block {
                        let sub_result = self.validate_statement_concurrency(stmt)?;
                        has_exclusive_access |= sub_result.0;
                        has_shared_access |= sub_result.1;
                        has_readonly_access |= sub_result.2;
                    }
                    
                    // else ブロックがあれば解析
                    if let Some(else_stmts) = else_block {
                        for stmt in else_stmts {
                            let sub_result = self.validate_statement_concurrency(stmt)?;
                            has_exclusive_access |= sub_result.0;
                            has_shared_access |= sub_result.1;
                            has_readonly_access |= sub_result.2;
                        }
                    }
                },
                Statement::Loop(body) => {
                    // ループ本体の各ステートメントを解析
                    for stmt in body {
                        let sub_result = self.validate_statement_concurrency(stmt)?;
                        has_exclusive_access |= sub_result.0;
                        has_shared_access |= sub_result.1;
                        has_readonly_access |= sub_result.2;
                    }
                },
                Statement::Return(expr) => {
                    // 戻り値の式があれば解析
                    if let Some(return_expr) = expr {
                        self.analyze_expression_access_mode(return_expr, &mut has_exclusive_access, &mut has_shared_access, &mut has_readonly_access)?;
                    }
                },
                Statement::AsyncBlock(body) => {
                    // 非同期ブロック内の各ステートメントを解析
                    for stmt in body {
                        let sub_result = self.validate_statement_concurrency(stmt)?;
                        has_exclusive_access |= sub_result.0;
                        has_shared_access |= sub_result.1;
                        has_readonly_access |= sub_result.2;
                    }
                    
                    // 非同期ブロック内で排他アクセスがある場合は特別な検証
                    if has_exclusive_access {
                        // 非同期コンテキストでの排他アクセスが安全かチェック
                        if !self.validate_async_exclusive_access(function)? {
                            return Err("非同期ブロック内での排他的アクセスが安全でありません".to_string());
                        }
                    }
                },
                // その他のステートメント型に対する処理
                _ => {}
            }
        }
        
        // 関数の並行性モードを取得
        let method_mode = self.determine_method_concurrency_mode(function)?;
        
        // 並行性モードと実際のアクセスパターンの一貫性をチェック
        match method_mode {
            ConcurrencyMode::Exclusive => {
                // 排他モードでは全てのアクセスが許可される
                Ok(true)
            },
            ConcurrencyMode::Shared => {
                // 共有モードでは排他アクセスは許可されない
                if has_exclusive_access {
                    Err(format!("共有モードのメソッド '{}' 内で排他的アクセスが検出されました", function.name))
                } else {
                    Ok(true)
                }
            },
            ConcurrencyMode::ReadOnly => {
                // 読み取り専用モードでは排他アクセスと共有アクセスは許可されない
                if has_exclusive_access {
                    Err(format!("読み取り専用モードのメソッド '{}' 内で排他的アクセスが検出されました", function.name))
                } else if has_shared_access && !has_readonly_access {
                    Err(format!("読み取り専用モードのメソッド '{}' 内で共有アクセスが検出されました", function.name))
                } else {
                    Ok(true)
                }
            },
            ConcurrencyMode::Async => {
                // 非同期モードでは特別な検証ルールを適用
                self.validate_async_method_concurrency(function, has_exclusive_access, has_shared_access)
            },
            ConcurrencyMode::None => {
                // 並行性モードが指定されていない場合は、アクセスパターンに基づいて適切なモードを提案
                if has_exclusive_access {
                    Err(format!("メソッド '{}' は排他的アクセスを行っていますが、並行性モードが指定されていません。'#[exclusive]' 属性の追加を検討してください", function.name))
                } else if has_shared_access {
                    Err(format!("メソッド '{}' は共有アクセスを行っていますが、並行性モードが指定されていません。'#[shared]' 属性の追加を検討してください", function.name))
                } else if has_readonly_access {
                    Err(format!("メソッド '{}' は読み取り専用アクセスを行っていますが、並行性モードが指定されていません。'#[read_only]' 属性の追加を検討してください", function.name))
                } else {
                    Ok(true)
                }
            }
        }
    }
    
    /// ステートメントの並行性を検証し、(排他アクセス, 共有アクセス, 読み取り専用アクセス)のタプルを返す
    fn validate_statement_concurrency(&self, stmt: &Statement) -> Result<(bool, bool, bool), String> {
        let mut has_exclusive = false;
        let mut has_shared = false;
        let mut has_readonly = false;
        
        match stmt {
            Statement::FieldAccess(expr) => {
                let access_mode = self.analyze_field_access_mode(expr)?;
                match access_mode {
                    AccessMode::Write => has_exclusive = true,
                    AccessMode::Read => has_shared = true,
                    AccessMode::ReadOnly => has_readonly = true,
                }
            },
            Statement::MethodCall(method_call) => {
                if let Some(called_method) = self.find_method_by_name(&method_call.method_name)? {
                    let called_mode = self.determine_method_concurrency_mode(called_method)?;
                    match called_mode {
                        ConcurrencyMode::Exclusive => has_exclusive = true,
                        ConcurrencyMode::Shared => has_shared = true,
                        ConcurrencyMode::ReadOnly => has_readonly = true,
                        ConcurrencyMode::Async => {
                            // 非同期メソッド呼び出しの場合、呼び出し元のコンテキストに基づいて適切なアクセスモードを設定
                            let (async_exclusive, async_shared, async_readonly) = self.analyze_async_method_call(method_call)?;
                            has_exclusive |= async_exclusive;
                            has_shared |= async_shared;
                            has_readonly |= async_readonly;
                        },
                        ConcurrencyMode::None => {
                            // 並行性モードが指定されていないメソッドの場合、メソッドの実装を解析して動的に判断
                            let (implicit_exclusive, implicit_shared, implicit_readonly) = self.infer_method_concurrency(called_method)?;
                            has_exclusive |= implicit_exclusive;
                            has_shared |= implicit_shared;
                            has_readonly |= implicit_readonly;
                        }
                    }
                    
                    // メソッド呼び出しの引数も解析
                    for arg in &method_call.arguments {
                        self.analyze_expression_access_mode(arg, &mut has_exclusive, &mut has_shared, &mut has_readonly)?;
                    }
                } else {
                    // 未知のメソッド呼び出しの場合、保守的に排他アクセスと見なす
                    has_exclusive = true;
                    return Err(format!("未知のメソッド '{}' が呼び出されました。アクセスモードを判断できません", method_call.method_name));
                }
            },
            Statement::Assignment(target, expr) => {
                // 代入文の左辺（ターゲット）を解析
                // 左辺が変数やフィールドアクセスの場合、書き込みアクセスとなる
                match target {
                    Expression::Variable(var_name) => {
                        // 変数が自身のフィールドを参照している場合は排他アクセスが必要
                        if self.is_self_field_variable(var_name)? {
                            has_exclusive = true;
                        }
                    },
                    Expression::FieldAccess(obj, field_name) => {
                        // オブジェクトが自身（self）の場合、排他アクセスが必要
                        if self.is_self_expression(obj)? {
                            has_exclusive = true;
                            
                            // フィールドの変更可能性を検証
                            if self.is_immutable_field(field_name)? {
                                return Err(format!("イミュータブルフィールド '{}' への書き込みは許可されていません", field_name));
                            }
                            
                            // 並行アクセス制約を検証
                            if !self.can_write_field_in_current_context(field_name)? {
                                return Err(format!("現在の並行コンテキストでは、フィールド '{}' への書き込みアクセスは許可されていません", field_name));
                            }
                        }
                    },
                    Expression::IndexAccess(array, index) => {
                        // 配列が自身のフィールドを参照している場合は排他アクセスが必要
                        if self.is_self_owned_collection(array)? {
                            has_exclusive = true;
                            
                            // インデックスの範囲チェックが可能な場合は実施
                            if let Some(array_size) = self.get_collection_size(array)? {
                                if let Some(idx_value) = self.evaluate_constant_expression(index)? {
                                    if idx_value >= array_size {
                                        return Err(format!("インデックス {} は配列サイズ {} を超えています", idx_value, array_size));
                                    }
                                }
                            }
                        }
                        
                        // インデックス式自体のアクセスモードも解析
                        self.analyze_expression_access_mode(index, &mut has_exclusive, &mut has_shared, &mut has_readonly)?;
                    },
                    Expression::Tuple(elements) => {
                        // タプルの各要素を解析
                        for element in elements {
                            match element {
                                Expression::FieldAccess(obj, field_name) if self.is_self_expression(obj)? => {
                                    has_exclusive = true;
                                    
                                    if self.is_immutable_field(field_name)? {
                                        return Err(format!("イミュータブルフィールド '{}' への書き込みは許可されていません", field_name));
                                    }
                                },
                                _ => {
                                    // 他の式のアクセスモードを解析
                                    self.analyze_expression_access_mode(element, &mut has_exclusive, &mut has_shared, &mut has_readonly)?;
                                }
                            }
                        }
                    },
                    _ => {
                        // その他の複雑な左辺値の場合、保守的に排他アクセスと見なす
                        has_exclusive = true;
                    }
                }
                
                // 右辺の式を解析
                self.analyze_expression_access_mode(expr, &mut has_exclusive, &mut has_shared, &mut has_readonly)?;
                
                // 代入操作の並行性制約を検証
                if has_exclusive && (has_shared || has_readonly) {
                    return Err("同一ステートメント内で排他アクセスと共有/読み取り専用アクセスを混在させることはできません".to_string());
                }
            },
            Statement::Block(statements) => {
                // ブロック内の各ステートメントを解析
                for stmt in statements {
                    let (stmt_exclusive, stmt_shared, stmt_readonly) = self.validate_statement_concurrency(stmt)?;
                    has_exclusive |= stmt_exclusive;
                    has_shared |= stmt_shared;
                    has_readonly |= stmt_readonly;
                    
                    // ブロック内で排他アクセスと共有アクセスが混在していないか検証
                    if stmt_exclusive && (has_shared || has_readonly) {
                        return Err("ブロック内で排他アクセスと共有/読み取り専用アクセスを混在させることはできません".to_string());
                    }
                    if (has_exclusive) && (stmt_shared || stmt_readonly) {
                        return Err("ブロック内で排他アクセスと共有/読み取り専用アクセスを混在させることはできません".to_string());
                    }
                }
            },
            Statement::If(condition, then_branch, else_branch) => {
                // 条件式を解析
                self.analyze_expression_access_mode(condition, &mut has_exclusive, &mut has_shared, &mut has_readonly)?;
                
                // then ブランチを解析
                let (then_exclusive, then_shared, then_readonly) = self.validate_statement_concurrency(then_branch)?;
                has_exclusive |= then_exclusive;
                has_shared |= then_shared;
                has_readonly |= then_readonly;
                
                // else ブランチがあれば解析
                if let Some(else_stmt) = else_branch {
                    let (else_exclusive, else_shared, else_readonly) = self.validate_statement_concurrency(else_stmt)?;
                    has_exclusive |= else_exclusive;
                    has_shared |= else_shared;
                    has_readonly |= else_readonly;
                }
            },
            Statement::While(condition, body) => {
                // 条件式を解析
                self.analyze_expression_access_mode(condition, &mut has_exclusive, &mut has_shared, &mut has_readonly)?;
                
                // ループ本体を解析
                let (body_exclusive, body_shared, body_readonly) = self.validate_statement_concurrency(body)?;
                has_exclusive |= body_exclusive;
                has_shared |= body_shared;
                has_readonly |= body_readonly;
            },
            Statement::For(init, condition, update, body) => {
                // 初期化式を解析
                if let Some(init_stmt) = init {
                    let (init_exclusive, init_shared, init_readonly) = self.validate_statement_concurrency(init_stmt)?;
                    has_exclusive |= init_exclusive;
                    has_shared |= init_shared;
                    has_readonly |= init_readonly;
                }
                
                // 条件式を解析
                if let Some(cond_expr) = condition {
                    self.analyze_expression_access_mode(cond_expr, &mut has_exclusive, &mut has_shared, &mut has_readonly)?;
                }
                
                // 更新式を解析
                if let Some(update_stmt) = update {
                    let (update_exclusive, update_shared, update_readonly) = self.validate_statement_concurrency(update_stmt)?;
                    has_exclusive |= update_exclusive;
                    has_shared |= update_shared;
                    has_readonly |= update_readonly;
                }
                
                // ループ本体を解析
                let (body_exclusive, body_shared, body_readonly) = self.validate_statement_concurrency(body)?;
                has_exclusive |= body_exclusive;
                has_shared |= body_shared;
                has_readonly |= body_readonly;
            },
            Statement::ForEach(var, collection, body) => {
                // コレクション式を解析
                self.analyze_expression_access_mode(collection, &mut has_exclusive, &mut has_shared, &mut has_readonly)?;
                
                // ループ本体を解析
                let (body_exclusive, body_shared, body_readonly) = self.validate_statement_concurrency(body)?;
                has_exclusive |= body_exclusive;
                has_shared |= body_shared;
                has_readonly |= body_readonly;
                
                // コレクションが自身のフィールドで、本体内で変更がある場合、並行性制約を検証
                if self.is_self_owned_collection(collection)? && body_exclusive {
                    return Err("イテレーション中のコレクションを変更することはできません".to_string());
                }
            },
            Statement::Return(expr) => {
                // 戻り値の式を解析
                if let Some(return_expr) = expr {
                    self.analyze_expression_access_mode(return_expr, &mut has_exclusive, &mut has_shared, &mut has_readonly)?;
                }
            },
            Statement::Expression(expr) => {
                // 式を解析
                self.analyze_expression_access_mode(expr, &mut has_exclusive, &mut has_shared, &mut has_readonly)?;
            },
            Statement::Await(expr) => {
                // await式の対象を解析
                self.analyze_expression_access_mode(expr, &mut has_exclusive, &mut has_shared, &mut has_readonly)?;
                
                // await式は非同期コンテキストでのみ使用可能
                if !self.is_async_context()? {
                    return Err("await式は非同期メソッド内でのみ使用できます".to_string());
                }
                
                // await式の並行性制約を検証
                // 非同期タスクが完了するまで現在のタスクはブロックされるため、
                // 排他アクセスと共有アクセスの混在に注意が必要
                if has_exclusive && (has_shared || has_readonly) {
                    return Err("await式の前後で排他アクセスと共有/読み取り専用アクセスを混在させることはできません".to_string());
                }
            },
            Statement::Send(channel, message) => {
                // チャネルとメッセージの式を解析
                self.analyze_expression_access_mode(channel, &mut has_exclusive, &mut has_shared, &mut has_readonly)?;
                self.analyze_expression_access_mode(message, &mut has_exclusive, &mut has_shared, &mut has_readonly)?;
                
                // メッセージ送信の並行性制約を検証
                // メッセージ内に自身のフィールドへの参照が含まれる場合、所有権の移動に注意
                if self.contains_self_field_references(message)? {
                    // メッセージ型が所有権を移動するかコピーするかを判断
                    if !self.is_copyable_type(&self.get_expression_type(message)?)? {
                        // 所有権が移動する場合、以降のコードでそのフィールドにアクセスできないことを検証
                        let moved_fields = self.get_moved_fields(message)?;
                        self.register_moved_fields(moved_fields)?;
                    }
                }
            },
            Statement::Receive(var, channel) => {
                // チャネルの式を解析
                self.analyze_expression_access_mode(channel, &mut has_exclusive, &mut has_shared, &mut has_readonly)?;
                
                // 受信した値の型を取得し、変数に登録
                if let Some(channel_type) = self.get_expression_type(channel)? {
                    if let Type::Channel(element_type) = channel_type {
                        self.register_variable(var, *element_type)?;
                    } else {
                        return Err(format!("式 '{:?}' はチャネル型ではありません", channel));
                    }
                }
            },
            Statement::Spawn(method_call) => {
                // スポーンされるメソッド呼び出しを解析
                if let Some(called_method) = self.find_method_by_name(&method_call.method_name)? {
                    // スポーンされるメソッドが非同期であることを確認
                    if !self.is_async_method(called_method)? {
                        return Err(format!("非同期メソッドでない '{}' をスポーンすることはできません", method_call.method_name));
                    }
                    
                    // 引数を解析
                    for arg in &method_call.arguments {
                        self.analyze_expression_access_mode(arg, &mut has_exclusive, &mut has_shared, &mut has_readonly)?;
                        
                        // 引数に自身のフィールドへの参照が含まれる場合、所有権の移動に注意
                        if self.contains_self_field_references(arg)? {
                            // 引数の型が所有権を移動するかコピーするかを判断
                            if !self.is_copyable_type(&self.get_expression_type(arg)?)? {
                                // 所有権が移動する場合、以降のコードでそのフィールドにアクセスできないことを検証
                                let moved_fields = self.get_moved_fields(arg)?;
                                self.register_moved_fields(moved_fields)?;
                            }
                        }
                    }
                } else {
                    return Err(format!("未知のメソッド '{}' をスポーンしようとしています", method_call.method_name));
                }
            },
            Statement::Lock(resources, body) => {
                // ロックされるリソースを解析
                for resource in resources {
                    match resource {
                        Expression::FieldAccess(obj, field_name) if self.is_self_expression(obj)? => {
                            // フィールドがロック可能であることを確認
                            if !self.is_lockable_field(field_name)? {
                                return Err(format!("フィールド '{}' はロック可能ではありません", field_name));
                            }
                            
                            // 現在のロックコンテキストを更新
                            self.enter_lock_context(field_name)?;
                        },
                        _ => return Err(format!("式 '{:?}' はロック可能なリソースではありません", resource)),
                    }
                }
                
                // ロックブロック内のコードを解析
                let (body_exclusive, body_shared, body_readonly) = self.validate_statement_concurrency(body)?;
                has_exclusive |= body_exclusive;
                has_shared |= body_shared;
                has_readonly |= body_readonly;
                
                // ロックコンテキストを終了
                for resource in resources {
                    if let Expression::FieldAccess(obj, field_name) = resource {
                        if self.is_self_expression(obj)? {
                            self.exit_lock_context(field_name)?;
                        }
                    }
                }
            },
            Statement::Try(body, catch_clauses, finally) => {
                // try ブロックを解析
                let (try_exclusive, try_shared, try_readonly) = self.validate_statement_concurrency(body)?;
                has_exclusive |= try_exclusive;
                has_shared |= try_shared;
                has_readonly |= try_readonly;
                
                // catch 節を解析
                for (exception_type, exception_var, catch_body) in catch_clauses {
                    // 例外変数を登録
                    self.register_variable(exception_var, Type::Named(exception_type.clone()))?;
                    
                    // catch ブロックを解析
                    let (catch_exclusive, catch_shared, catch_readonly) = self.validate_statement_concurrency(catch_body)?;
                    has_exclusive |= catch_exclusive;
                    has_shared |= catch_shared;
                    has_readonly |= catch_readonly;
                }
                
                // finally ブロックがあれば解析
                if let Some(finally_body) = finally {
                    let (finally_exclusive, finally_shared, finally_readonly) = self.validate_statement_concurrency(finally_body)?;
                    has_exclusive |= finally_exclusive;
                    has_shared |= finally_shared;
                    has_readonly |= finally_readonly;
                }
            },
            Statement::Throw(expr) => {
                // 例外式を解析
                self.analyze_expression_access_mode(expr, &mut has_exclusive, &mut has_shared, &mut has_readonly)?;
            },
            Statement::Match(expr, arms) => {
                // マッチ対象の式を解析
                self.analyze_expression_access_mode(expr, &mut has_exclusive, &mut has_shared, &mut has_readonly)?;
                
                // 各マッチアームを解析
                for (pattern, guard, body) in arms {
                    // ガード式があれば解析
                    if let Some(guard_expr) = guard {
                        self.analyze_expression_access_mode(guard_expr, &mut has_exclusive, &mut has_shared, &mut has_readonly)?;
                    }
                    
                    // パターン内の変数をバインド
                    self.bind_pattern_variables(pattern, expr)?;
                    
                    // アーム本体を解析
                    let (arm_exclusive, arm_shared, arm_readonly) = self.validate_statement_concurrency(body)?;
                    has_exclusive |= arm_exclusive;
                    has_shared |= arm_shared;
                    has_readonly |= arm_readonly;
                }
            },
            Statement::Let(var, type_annotation, initializer) => {
                // 初期化式があれば解析
                if let Some(init_expr) = initializer {
                    self.analyze_expression_access_mode(init_expr, &mut has_exclusive, &mut has_shared, &mut has_readonly)?;
                    
                    // 変数の型を登録
                    let var_type = if let Some(type_ann) = type_annotation {
                        self.resolve_type(type_ann)?
                    } else {
                        // 型注釈がない場合は初期化式から型を推論
                        self.infer_expression_type(init_expr)?
                    };
                    
                    self.register_variable(var, var_type)?;
                } else if let Some(type_ann) = type_annotation {
                    // 初期化式がなく型注釈がある場合
                    let var_type = self.resolve_type(type_ann)?;
                    self.register_variable(var, var_type)?;
                } else {
                    // 初期化式も型注釈もない場合はエラー
                    return Err(format!("変数 '{}' には型注釈または初期化式が必要です", var));
                }
            },
            _ => {
                // その他のステートメント型は保守的に排他アクセスと見なす
                has_exclusive = true;
            }
        }
        
        Ok((has_exclusive, has_shared, has_readonly))
    }
    
    /// フィールドアクセスのモードを解析
    fn analyze_field_access_mode(&self, expr: &Expression) -> Result<AccessMode, String> {
        match expr {
            Expression::FieldAccess(obj, field_name) => {
                // オブジェクトが自身（self）かどうかを確認
                if self.is_self_expression(obj)? {
                    // アクセスコンテキストから書き込みアクセスかどうかを判定
                    if self.is_write_context(expr)? {
                        Ok(AccessMode::Write)
                    } else {
                        // フィールドの型から読み取り専用かどうかを判定
                        if self.is_readonly_field(field_name)? {
                            Ok(AccessMode::ReadOnly)
                        } else {
                            Ok(AccessMode::Read)
                        }
                    }
                } else {
                    // 自身以外のオブジェクトへのアクセスは関係ない
                    Ok(AccessMode::ReadOnly)
                }
            },
            _ => Ok(AccessMode::ReadOnly), // フィールドアクセス以外は読み取り専用と見なす
        }
    }
    
    /// 式のアクセスモードを解析し、フラグを更新
    fn analyze_expression_access_mode(&self, expr: &Expression, has_exclusive: &mut bool, has_shared: &mut bool, has_readonly: &mut bool) -> Result<(), String> {
        match expr {
            Expression::FieldAccess(_, _) => {
                let access_mode = self.analyze_field_access_mode(expr)?;
                match access_mode {
                    AccessMode::Write => *has_exclusive = true,
                    AccessMode::Read => *has_shared = true,
                    AccessMode::ReadOnly => *has_readonly = true,
                }
            },
            Expression::MethodCall(obj, method_name, args) => {
                // メソッド呼び出しの対象が自身かどうかを確認
                if self.is_self_expression(obj)? {
                    // 呼び出されるメソッドの並行性モードを取得
                    if let Some(called_method) = self.find_method_by_name(method_name)? {
                        let called_mode = self.determine_method_concurrency_mode(called_method)?;
                        match called_mode {
                            ConcurrencyMode::Exclusive => *has_exclusive = true,
                            ConcurrencyMode::Shared => *has_shared = true,
                            ConcurrencyMode::ReadOnly => *has_readonly = true,
                            _ => {}
                        }
                    }
                }
                
                // 引数の式も解析
                for arg in args {
                    self.analyze_expression_access_mode(arg, has_exclusive, has_shared, has_readonly)?;
                }
            },
            Expression::BinaryOp(left, _, right) => {
                // 二項演算子の左右の式を解析
                self.analyze_expression_access_mode(left, has_exclusive, has_shared, has_readonly)?;
                self.analyze_expression_access_mode(right, has_exclusive, has_shared, has_readonly)?;
            },
            Expression::UnaryOp(_, operand) => {
                // 単項演算子のオペランドを解析
                self.analyze_expression_access_mode(operand, has_exclusive, has_shared, has_readonly)?;
            },
            Expression::ArrayAccess(array, index) => {
                // 配列とインデックスの式を解析
                self.analyze_expression_access_mode(array, has_exclusive, has_shared, has_readonly)?;
                self.analyze_expression_access_mode(index, has_exclusive, has_shared, has_readonly)?;
                
                // 配列アクセスが書き込みコンテキストにある場合は排他アクセス
                if self.is_write_context(expr)? && self.is_self_field_array(array)? {
                    *has_exclusive = true;
                }
            },
            Expression::Closure(params, body) => {
                // クロージャ内の本体を解析
                for stmt in body {
                    let (exclusive, shared, readonly) = self.validate_statement_concurrency(stmt)?;
                    *has_exclusive |= exclusive;
                    *has_shared |= shared;
                    *has_readonly |= readonly;
                }
                
                // クロージャがキャプチャする変数も考慮
                // （実装の詳細は省略）
            },
            // その他の式型に対する処理
            _ => {}
        }
        
        Ok(())
    }
    
    /// 式が自身（self）を参照しているかどうかを判定
    fn is_self_expression(&self, expr: &Expression) -> Result<bool, String> {
        match expr {
            Expression::Identifier(name) if name == "self" => Ok(true),
            Expression::FieldAccess(obj, _) => self.is_self_expression(obj),
            _ => Ok(false),
        }
    }
    
    /// 式が書き込みコンテキストにあるかどうかを判定
    fn is_write_context(&self, expr: &Expression) -> Result<bool, String> {
        // 現在の文脈から書き込みコンテキストかどうかを判定
        // （実装の詳細は省略）
        Ok(false) // デフォルトでは読み取りコンテキストと見なす
    }
    
    /// フィールドが読み取り専用かどうかを判定
    fn is_readonly_field(&self, field_name: &str) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // フィールドの型や属性から読み取り専用かどうかを判定
        // （実装の詳細は省略）
        Ok(field_name.starts_with("readonly_") || field_name.ends_with("_ro"))
    }
    
    /// 式が自身のフィールドアクセスかどうかを判定
    fn is_self_field_access(&self, expr: &Expression) -> Result<bool, String> {
        match expr {
            Expression::FieldAccess(obj, _) => self.is_self_expression(obj),
            _ => Ok(false),
        }
    }
    
    /// 式が自身のフィールドの配列かどうかを判定
    fn is_self_field_array(&self, expr: &Expression) -> Result<bool, String> {
        match expr {
            Expression::FieldAccess(obj, field_name) => {
                if self.is_self_expression(obj)? {
                    // フィールドの型が配列かどうかを判定
                    let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
                    
                    // アクターの定義を取得
                    let actor_def = self.actor_def.as_ref().ok_or("アクター定義が設定されていません")?;
                    
                    // フィールド定義を検索
                    for field in &actor_def.fields {
                        if field.name == *field_name {
                            // フィールドの型を取得して配列かどうかを判定
                            return match &field.field_type {
                                Type::Array(_) => Ok(true),
                                Type::Vector(_) => Ok(true),
                                Type::Slice(_) => Ok(true),
                                Type::FixedArray(_, _) => Ok(true),
                                Type::ConcurrentArray(_) => Ok(true),
                                Type::SharedArray(_) => Ok(true),
                                Type::AtomicArray(_) => Ok(true),
                                Type::Generic(base, params) => {
                                    // ジェネリック型の場合、配列関連の型かどうかを確認
                                    let base_name = match base {
                                        Type::Named(name) => name,
                                        _ => return Ok(false),
                                    };
                                    
                                    let array_like_types = [
                                        "Array", "Vec", "Vector", "List", "Collection", 
                                        "Sequence", "Buffer", "Queue", "Stack", "Deque"
                                    ];
                                    
                                    Ok(array_like_types.contains(&base_name.as_str()))
                                },
                                Type::Named(name) => {
                                    // 名前付き型の場合、型定義を検索して配列かどうかを判定
                                    let type_def = module.find_type_definition(name)?;
                                    match type_def {
                                        Some(def) => {
                                            // 型の属性から配列かどうかを判定
                                            Ok(def.attributes.iter().any(|attr| {
                                                attr.name == "array" || attr.name == "collection" || 
                                                attr.name == "sequence" || attr.name == "indexable"
                                            }))
                                        },
                                        None => {
                                            // 標準ライブラリの配列関連型かどうかを確認
                                            let std_array_types = [
                                                "Array", "Vec", "Vector", "List", "Collection", 
                                                "Sequence", "Buffer", "Queue", "Stack", "Deque"
                                            ];
                                            Ok(std_array_types.contains(&name.as_str()))
                                        }
                                    }
                                },
                                _ => Ok(false),
                            };
                        }
                    }
                    
                    // フィールドが見つからない場合は親クラス/トレイトを検索
                    if let Some(parent) = &actor_def.parent {
                        let parent_def = module.find_actor_definition(parent)?;
                        if let Some(parent_actor) = parent_def {
                            for field in &parent_actor.fields {
                                if field.name == *field_name {
                                    return match &field.field_type {
                                        Type::Array(_) => Ok(true),
                                        Type::Vector(_) => Ok(true),
                                        Type::Slice(_) => Ok(true),
                                        Type::FixedArray(_, _) => Ok(true),
                                        Type::ConcurrentArray(_) => Ok(true),
                                        Type::SharedArray(_) => Ok(true),
                                        Type::AtomicArray(_) => Ok(true),
                                        // その他の型チェックは上記と同様
                                        _ => Ok(false),
                                    };
                                }
                            }
                        }
                    }
                    
                    // フィールドが実装しているトレイトのデフォルト実装も検索
                    for trait_ref in &actor_def.implemented_traits {
                        let trait_def = module.find_trait_definition(trait_ref)?;
                        if let Some(trait_def) = trait_def {
                            for field in &trait_def.fields {
                                if field.name == *field_name {
                                    return match &field.field_type {
                                        Type::Array(_) => Ok(true),
                                        Type::Vector(_) => Ok(true),
                                        // その他の型チェックは上記と同様
                                        _ => Ok(false),
                                    };
                                }
                            }
                        }
                    }
                    
                    // フィールドが見つからない場合はエラー
                    Err(format!("フィールド '{}' がアクター定義内に見つかりません", field_name))
                } else {
                    Ok(false)
                }
            },
            Expression::IndexAccess(obj, _) => {
                // インデックスアクセスの場合、オブジェクトが自身のフィールドアクセスかどうかを確認
                match &**obj {
                    Expression::FieldAccess(inner_obj, _) => self.is_self_expression(inner_obj),
                    _ => Ok(false),
                }
            },
            Expression::MethodCall(obj, method_name, _) => {
                // メソッド呼び出しの場合、配列を返すメソッドかどうかを確認
                if self.is_self_expression(obj)? {
                    let array_returning_methods = [
                        "get_array", "to_array", "as_array", "get_elements", "elements",
                        "get_items", "items", "get_collection", "collection", "get_list", "list"
                    ];
                    Ok(array_returning_methods.contains(&method_name.as_str()))
                } else {
                    Ok(false)
                }
            },
            _ => Ok(false),
        }
    }
    /// 名前からメソッドを検索
    fn find_method_by_name(&self, method_name: &str) -> Result<Option<&Function>, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // モジュール内の関数から名前が一致するものを検索
        for function in &module.functions {
            if function.name == method_name {
                return Ok(Some(function));
            }
        }
        
        Ok(None)
    }
    
    /// 非同期メソッドの並行性を検証
    fn validate_async_method_concurrency(&self, function: &Function, has_exclusive: bool, has_shared: bool) -> Result<bool, String> {
        // 非同期メソッドでの排他アクセスは特別な注意が必要
        if has_exclusive {
            // 非同期コンテキストでの排他アクセスが安全かチェック
            if !self.validate_async_exclusive_access(function)? {
                return Err(format!("非同期メソッド '{}' 内での排他的アクセスが安全でありません", function.name));
            }
        }
        
        Ok(true)
    }
    
    /// 非同期コンテキストでの排他アクセスの安全性を検証
    fn validate_async_exclusive_access(&self, function: &Function) -> Result<bool, String> {
        // 非同期コンテキストでの排他アクセスの安全性を検証するロジック
        // 例: 排他ロックの適切な使用、デッドロック回避策の確認など
        
        // 関数内でアクターのメッセージングシステムを使用しているか確認
        let uses_messaging = function.body.iter().any(|stmt| {
            match stmt {
                Statement::MethodCall(method_call) => {
                    method_call.method_name.contains("send") || 
                    method_call.method_name.contains("receive") ||
                    method_call.method_name.contains("ask")
                },
                _ => false,
            }
        });
        
        // 非同期排他アクセスの安全性条件をチェック
        if uses_messaging {
            // メッセージングを使用している場合は安全と見なす
            Ok(true)
        } else {
            // 排他ロックの適切な使用を確認
            let uses_proper_locks = function.body.iter().any(|stmt| {
                match stmt {
                    Statement::MethodCall(method_call) => {
                        method_call.method_name.contains("lock") || 
                        method_call.method_name.contains("mutex") ||
                        method_call.method_name.contains("semaphore")
                    },
                    _ => false,
                }
            });
            
            Ok(uses_proper_locks)
        }
    }
    /// アクターの状態分離を検証
    fn validate_state_isolation(&self, type_id: TypeId) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // アクターの状態が適切に分離されているか確認
        // 例: 内部状態が外部から直接アクセスされていないか
        
        // 型の定義を取得
        let ty = module.types.get(&type_id)
            .ok_or_else(|| format!("型ID {}の情報が見つかりません", type_id))?;
        
        match ty {
            Type::Struct(_, fields) => {
                // 各フィールドの可視性と保護レベルをチェック
                for (i, &field_type_id) in fields.iter().enumerate() {
                    // フィールドの情報を取得
                    let field_info = module.get_field_info(type_id, i)
                        .ok_or_else(|| format!("フィールド情報が見つかりません: type_id={}, index={}", type_id, i))?;
                    
                    // 状態フィールドが適切に保護されているか確認
                    if self.is_state_field(field_type_id)? && field_info.protection_level == ProtectionLevel::Public {
                        // 状態フィールドが公開されている場合、アクセサメソッドが適切に実装されているか確認
                        if !self.has_proper_accessor_methods(type_id, i)? {
                            return Ok(false);
                        }
                    }
                }
                
                Ok(true)
            },
            _ => Ok(false), // 構造体以外はここでは考慮しない
        }
    }
    
    /// フィールドが状態フィールドかどうかを判定
    fn is_state_field(&self, field_type_id: TypeId) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // フィールドの型情報を取得
        let field_type = module.types.get(&field_type_id)
            .ok_or_else(|| format!("型ID {}の情報が見つかりません", field_type_id))?;
        
        // 状態フィールドの特徴をチェック
        match field_type {
            Type::Generic(name, _) if name.contains("ActorState") => Ok(true),
            _ => {
                // 型の名前や属性から状態フィールドかどうかを判断
                let type_name = module.get_type_name(field_type_id)?;
                Ok(type_name.contains("State") || type_name.contains("Data"))
            }
        }
    }
    
    /// フィールドに適切なアクセサメソッドが実装されているか確認
    fn has_proper_accessor_methods(&self, type_id: TypeId, field_index: usize) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // フィールド情報を取得
        let field_info = module.get_field_info(type_id, field_index)
            .ok_or_else(|| format!("フィールド情報が見つかりません: type_id={}, index={}", type_id, field_index))?;
        
        let field_name = &field_info.name;
        
        // ゲッターメソッドの存在をチェック
        let getter_name = format!("get_{}", field_name);
        let has_getter = module.functions.values().any(|f| {
            f.name == getter_name && self.is_method_of_actor_result(f.id, type_id).unwrap_or(false)
        });
        
        // セッターメソッドの存在をチェック（必要な場合）
        let setter_name = format!("set_{}", field_name);
        let has_setter = module.functions.values().any(|f| {
            f.name == setter_name && self.is_method_of_actor_result(f.id, type_id).unwrap_or(false)
        });
        
        // 読み取り専用フィールドの場合はゲッターのみ必要
        if field_info.is_mutable {
            Ok(has_getter && has_setter)
        } else {
            Ok(has_getter)
        }
    }
    
    /// 関数がアクターのメソッドかどうかを判定（Result型を返す補助関数）
    fn is_method_of_actor_result(&self, func_id: FunctionId, type_id: TypeId) -> Result<bool, String> {
        self.is_method_of_actor(func_id, type_id)
    }
    /// アクター定義を作成
    fn create_actor_definition(&self, type_id: TypeId, ty: &Type) -> Result<Actor, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // 型からアクター名を取得
        let name = match ty {
            Type::Struct(name, _) => name.clone(),
            _ => return Err("アクター型は構造体である必要があります".to_string()),
        };
        // アクターフィールドを収集
        let mut fields = Vec::new();
        if let Type::Struct(_, field_types) = ty {
            for (i, &field_type_id) in field_types.iter().enumerate() {
                // フィールド情報をモジュールのシンボル情報から取得
                let field_info = module.get_field_info(type_id, i)
                    .ok_or_else(|| format!("フィールド情報が見つかりません: type_id={}, index={}", type_id, i))?;
                
                // フィールドの初期値を取得
                let initial_value = module.get_field_default_value(type_id, i);
                
                // フィールドの保護レベルを判定
                let protection_level = self.determine_field_protection_level(&field_info)?;
                
                // フィールドが状態データかどうかを判定
                let is_state_field = self.is_state_field(type_id, i)?;
                
                // フィールドのメタデータを取得
                let metadata = module.get_field_metadata(type_id, i)?;
                
                // フィールドの依存関係を分析
                let dependencies = self.analyze_field_dependencies(type_id, i)?;
                
                // フィールドの検証ルールを取得
                let validation_rules = self.extract_field_validation_rules(type_id, i)?;
                
                // フィールドのアクセス制御ポリシーを取得
                let access_policy = self.determine_field_access_policy(type_id, i)?;
                
                // フィールドの変更通知設定を取得
                let change_notification = metadata.get("notify_on_change").map(|v| v == "true").unwrap_or(false);
                
                // フィールドの永続化設定を取得
                let persistence = self.determine_field_persistence(type_id, i, &metadata)?;
                
                // フィールドのタグを収集
                let mut tags = HashSet::new();
                if let Some(tag_str) = metadata.get("tags") {
                    for tag in tag_str.split(',') {
                        tags.insert(tag.trim().to_string());
                    }
                }
                
                // フィールドの同期モードを判定
                let sync_mode = if metadata.get("sync").map_or(false, |v| v == "true") {
                    Some(FieldSyncMode::from_metadata(&metadata))
                } else {
                    None
                };
                
                // フィールドの監視設定を取得
                let monitoring = if metadata.get("monitor").map_or(false, |v| v == "true") {
                    Some(FieldMonitoring::from_metadata(&metadata))
                } else {
                    None
                };
                
                // フィールドのキャッシュ戦略を取得
                let cache_strategy = if metadata.get("cache").map_or(false, |v| v == "true") {
                    Some(CacheStrategy::from_metadata(&metadata))
                } else {
                    None
                };
                
                // フィールドの型制約を取得
                let type_constraints = self.extract_field_type_constraints(type_id, i, &metadata)?;
                
                // フィールドのドキュメントを取得
                let documentation = metadata.get("doc").cloned();
                
                // アクターフィールドを作成
                fields.push(ActorField {
                    name: field_info.name.clone(),
                    type_id: field_type_id,
                    is_mutable: field_info.is_mutable,
                    protection_level,
                    initial_value,
                    is_state_field,
                    dependencies,
                    validation_rules,
                    access_policy,
                    change_notification,
                    persistence,
                    tags,
                    sync_mode,
                    monitoring,
                    cache_strategy,
                    type_constraints,
                    documentation,
                });
                
                // フィールドに適切なアクセサメソッドがあるか確認
                if !self.has_proper_accessor_methods(type_id, i)? {
                    // アクセサメソッドが不足している場合は警告を記録
                    self.warnings.push(format!(
                        "アクター '{}' のフィールド '{}' に適切なアクセサメソッドがありません",
                        name, field_info.name
                    ));
                }
            }
            
            // フィールド間の依存関係を検証
            self.validate_field_dependencies(&fields)?;
            
            // 状態フィールドの整合性を検証
            self.validate_state_fields(&fields)?;
            
            // 永続化設定の整合性を検証
            self.validate_persistence_configuration(&fields)?;
        }
        // アクターメソッドを収集
        let mut methods = Vec::new();
        let mut lifecycle_hooks = HashMap::new();
        
        for (func_id, function) in &module.functions {
            // 関数がこのアクターのメソッドかチェック
            if self.is_method_of_actor(*func_id, type_id)? {
                // メソッドの種類を判定
                let method_kind = self.determine_method_kind(function)?;
                
                // ライフサイクルフックかどうかをチェック
                if let Some(event) = self.as_lifecycle_hook(&function.name) {
                    lifecycle_hooks.insert(event, *func_id);
                } else {
                    // 通常のメソッドとして追加
                    methods.push(ActorMethod {
                        name: function.name.clone(),
                        function_id: *func_id,
                        protection_level: self.determine_protection_level(function)?,
                        kind: method_kind,
                        concurrency_mode: ConcurrencyMode::Exclusive, // デフォルトは排他的
                        accessed_state: HashSet::new(), // 後で分析して更新
                    });
                }
            }
        }
        
        // スーパーバイザー関係を取得
        let supervisees = self.get_supervised_actors(type_id)?;
        
        // アクター定義を作成
        Ok(Actor {
            name,
            type_id,
            state: fields,
            methods,
            lifecycle_hooks,
            supervisees,
            tags: HashSet::new(),
        })
    }
    
    /// 関数がアクターのメソッドかどうかをチェック
    fn is_method_of_actor(&self, func_id: FunctionId, actor_type_id: TypeId) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        if let Some(function) = module.functions.get(&func_id) {
            // self パラメータがあり、その型がアクター型と一致するかチェック
            if let Some(self_param) = function.parameters.first() {
                return Ok(self_param.name == "self" && self_param.type_id == actor_type_id);
            }
        }
        
        Ok(false)
    }
    
    /// メソッドの種類を判定
    fn determine_method_kind(&self, function: &Function) -> Result<ActorMethodKind, String> {
        // 名前や属性からメソッドの種類を判断
        if function.name == "init" || function.name == "new" {
            return Ok(ActorMethodKind::Initializer);
        } else if function.name.starts_with("handle_") {
            return Ok(ActorMethodKind::MessageHandler);
        } else if function.name.starts_with("supervise_") {
            return Ok(ActorMethodKind::Supervisor);
        } else if function.name.starts_with("periodic_") || function.attributes.contains_key("periodic") {
            return Ok(ActorMethodKind::Periodic);
        }
        
        Ok(ActorMethodKind::Regular)
    }
    
    /// メソッドの保護レベルを判定
    fn determine_protection_level(&self, function: &Function) -> Result<ProtectionLevel, String> {
        // 属性や命名規則から保護レベルを判断
        if function.attributes.contains_key("public") {
            return Ok(ProtectionLevel::Public);
        } else if function.attributes.contains_key("protected") {
            return Ok(ProtectionLevel::Protected);
        } else if function.attributes.contains_key("private") {
            return Ok(ProtectionLevel::Private);
        }
        
        // 命名規則からの判断
        if function.name.starts_with("_") {
            return Ok(ProtectionLevel::Private);
        } else if function.name.starts_with("protected_") {
            return Ok(ProtectionLevel::Protected);
        }
        
        // デフォルトはパブリック
        Ok(ProtectionLevel::Public)
    }
    
    /// 関数名がライフサイクルフックかどうかをチェック
    fn as_lifecycle_hook(&self, name: &str) -> Option<LifecycleEvent> {
        match name {
            "pre_init" => Some(LifecycleEvent::PreInit),
            "post_init" => Some(LifecycleEvent::PostInit),
            "pre_receive" => Some(LifecycleEvent::PreReceive),
            "post_process" => Some(LifecycleEvent::PostProcess),
            "on_error" => Some(LifecycleEvent::OnError),
            "pre_terminate" => Some(LifecycleEvent::PreTerminate),
            _ => None,
        }
    }
    
    /// アクターが監視する他のアクターを取得
    fn get_supervised_actors(&self, actor_type_id: TypeId) -> Result<Vec<TypeId>, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        let mut supervised = Vec::new();
        let mut analyzed_functions = std::collections::HashSet::new();
        
        // スーパーバイザーメソッドを持つか確認
        for (func_id, function) in &module.functions {
            if self.is_method_of_actor(*func_id, actor_type_id)? && 
               self.determine_method_kind(function)? == ActorMethodKind::Supervisor {
                analyzed_functions.insert(*func_id);
                
                // メソッド本体を解析して監視対象アクターを特定
                if let Some(body) = &function.body {
                    // 関数内で生成または参照されるアクター型を検出
                    let referenced_actors = self.analyze_supervisor_body(body, actor_type_id)?;
                    supervised.extend(referenced_actors);
                    
                    // スーパービジョン戦略も解析
                    if let Some(strategy) = self.extract_supervision_strategy(body, *func_id)? {
                        self.supervision_strategies.insert((*func_id, actor_type_id), strategy);
                    }
                }
                
                // 関数の戻り値型が監視対象を示す場合も解析
                if let Some(return_type_id) = &function.return_type {
                    if self.is_actor_type(*return_type_id)? {
                        supervised.push(*return_type_id);
                    } else if self.is_actor_collection_type(*return_type_id)? {
                        // コレクション型（Vec<Actor>など）から要素型を抽出
                        if let Some(element_type_id) = self.extract_collection_element_type(*return_type_id)? {
                            if self.is_actor_type(element_type_id)? {
                                supervised.push(element_type_id);
                            }
                        }
                    }
                }
            }
        }
        
        // アクターの属性も確認
        if let Some(attr) = module.attributes.get(&actor_type_id) {
            if let Some(supervised_attr) = attr.get("supervises") {
                match supervised_attr {
                    AttributeValue::TypeList(type_ids) => {
                        // 明示的に指定された監視対象アクター型
                        for type_id in type_ids {
                            if self.is_actor_type(*type_id)? {
                                supervised.push(*type_id);
                            }
                        }
                    },
                    AttributeValue::String(type_names) => {
                        // カンマ区切りの型名リスト
                        for type_name in type_names.split(',').map(|s| s.trim()) {
                            if let Some(type_id) = self.resolve_type_name(type_name)? {
                                if self.is_actor_type(type_id)? {
                                    supervised.push(type_id);
                                }
                            }
                        }
                    },
                    AttributeValue::NestedAttributes(nested) => {
                        // 詳細な監視設定（戦略や再起動ポリシーなど）
                        if let Some(types) = nested.get("types") {
                            if let AttributeValue::TypeList(type_ids) = types {
                                for type_id in type_ids {
                                    if self.is_actor_type(*type_id)? {
                                        supervised.push(*type_id);
                                        
                                        // 監視戦略の解析
                                        if let Some(strategy_attr) = nested.get("strategy") {
                                            if let AttributeValue::String(strategy_name) = strategy_attr {
                                                let strategy = match strategy_name.as_str() {
                                                    "one_for_one" => SupervisionStrategy::OneForOne,
                                                    "one_for_all" => SupervisionStrategy::OneForAll,
                                                    "rest_for_one" => SupervisionStrategy::RestForOne,
                                                    _ => SupervisionStrategy::OneForOne, // デフォルト
                                                };
                                                self.supervision_strategies.insert((analyzed_functions.iter().next().copied().unwrap_or_default(), actor_type_id), strategy);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    _ => {}
                }
            }
        }
        
        // 継承関係からの監視対象も検出
        if let Some(parent_id) = self.get_parent_actor(actor_type_id)? {
            let parent_supervised = self.get_supervised_actors(parent_id)?;
            for supervised_id in parent_supervised {
                // 親クラスで監視されているが、子クラスでオーバーライドされていない場合のみ追加
                if !supervised.contains(&supervised_id) {
                    supervised.push(supervised_id);
                }
            }
        }
        
        // 重複を除去
        supervised.sort();
        supervised.dedup();
        
        Ok(supervised)
    }
    
    /// スーパーバイザーメソッドの本体を解析して監視対象アクターを特定
    fn analyze_supervisor_body(&self, body: &[Statement], supervisor_actor_id: TypeId) -> Result<Vec<TypeId>, String> {
        let mut supervised_actors = Vec::new();
        
        for stmt in body {
            match stmt {
                Statement::Expression(expr) => {
                    self.extract_actors_from_expression(expr, &mut supervised_actors)?;
                },
                Statement::Declaration(decl) => {
                    if let Declaration::Variable(var_name, type_id, initializer) = decl {
                        // 変数の型がアクターかチェック
                        if let Some(tid) = type_id {
                            if self.is_actor_type(*tid)? {
                                supervised_actors.push(*tid);
                            }
                        }
                        
                        // 初期化式からもアクター参照を抽出
                        if let Some(init_expr) = initializer {
                            self.extract_actors_from_expression(init_expr, &mut supervised_actors)?;
                        }
                    }
                },
                Statement::Return(expr) => {
                    if let Some(ret_expr) = expr {
                        self.extract_actors_from_expression(ret_expr, &mut supervised_actors)?;
                    }
                },
                Statement::If(condition, then_block, else_block) => {
                    self.extract_actors_from_expression(condition, &mut supervised_actors)?;
                    
                    for stmt in then_block {
                        let mut then_actors = self.analyze_supervisor_body(&[stmt.clone()], supervisor_actor_id)?;
                        supervised_actors.append(&mut then_actors);
                    }
                    
                    if let Some(else_stmts) = else_block {
                        for stmt in else_stmts {
                            let mut else_actors = self.analyze_supervisor_body(&[stmt.clone()], supervisor_actor_id)?;
                            supervised_actors.append(&mut else_actors);
                        }
                    }
                },
                Statement::Loop(loop_body) => {
                    for stmt in loop_body {
                        let mut loop_actors = self.analyze_supervisor_body(&[stmt.clone()], supervisor_actor_id)?;
                        supervised_actors.append(&mut loop_actors);
                    }
                },
                Statement::ForEach(_, collection, loop_body) => {
                    self.extract_actors_from_expression(collection, &mut supervised_actors)?;
                    
                    for stmt in loop_body {
                        let mut loop_actors = self.analyze_supervisor_body(&[stmt.clone()], supervisor_actor_id)?;
                        supervised_actors.append(&mut loop_actors);
                    }
                },
                // その他の文も必要に応じて解析
                _ => {}
            }
        }
        
        // 重複を除去
        supervised_actors.sort();
        supervised_actors.dedup();
        
        Ok(supervised_actors)
    }
    
    /// 式からアクター参照を抽出
    fn extract_actors_from_expression(&self, expr: &Expression, actors: &mut Vec<TypeId>) -> Result<(), String> {
        match expr {
            Expression::New(type_id, args) => {
                // 新しいアクターのインスタンス生成
                if self.is_actor_type(*type_id)? {
                    actors.push(*type_id);
                }
                
                // 引数も解析
                for arg in args {
                    self.extract_actors_from_expression(arg, actors)?;
                }
            },
            Expression::MethodCall(obj, method_name, args) => {
                // メソッド呼び出しのオブジェクトを解析
                self.extract_actors_from_expression(obj, actors)?;
                
                // 特定のファクトリーメソッドやアクター生成メソッドをチェック
                if method_name == "spawn" || method_name == "create_actor" || method_name.contains("actor") {
                    // 引数からアクター型を特定
                    for arg in args {
                        self.extract_actors_from_expression(arg, actors)?;
                    }
                    
                    // メソッドの戻り値型がアクターかチェック
                    if let Some(return_type_id) = self.get_method_return_type(obj, method_name)? {
                        if self.is_actor_type(return_type_id)? {
                            actors.push(return_type_id);
                        }
                    }
                }
                
                // 引数も解析
                for arg in args {
                    self.extract_actors_from_expression(arg, actors)?;
                }
            },
            Expression::FunctionCall(func_name, args) => {
                // アクター生成関数かチェック
                if func_name.contains("create") || func_name.contains("spawn") || func_name.contains("actor") {
                    // 関数の戻り値型がアクターかチェック
                    if let Some(func_id) = self.resolve_function_name(func_name)? {
                        if let Some(module) = &self.module {
                            if let Some(function) = module.functions.get(&func_id) {
                                if let Some(return_type_id) = function.return_type {
                                    if self.is_actor_type(return_type_id)? {
                                        actors.push(return_type_id);
                                    }
                                }
                            }
                        }
                    }
                }
                
                // 引数も解析
                for arg in args {
                    self.extract_actors_from_expression(arg, actors)?;
                }
            },
            Expression::Binary(left, _, right) => {
                self.extract_actors_from_expression(left, actors)?;
                self.extract_actors_from_expression(right, actors)?;
            },
            Expression::Unary(_, operand) => {
                self.extract_actors_from_expression(operand, actors)?;
            },
            Expression::ArrayLiteral(elements) => {
                for element in elements {
                    self.extract_actors_from_expression(element, actors)?;
                }
            },
            Expression::StructLiteral(type_id, fields) => {
                if self.is_actor_type(*type_id)? {
                    actors.push(*type_id);
                }
                
                for (_, value) in fields {
                    self.extract_actors_from_expression(value, actors)?;
                }
            },
            // その他の式も必要に応じて解析
            _ => {}
        }
        
        Ok(())
    }
    
    /// スーパービジョン戦略を抽出
    fn extract_supervision_strategy(&self, body: &[Statement], func_id: FunctionId) -> Result<Option<SupervisionStrategy>, String> {
        // デフォルト戦略
        let mut strategy = None;
        
        // 関数の属性をチェック
        if let Some(module) = &self.module {
            if let Some(func_attrs) = module.function_attributes.get(&func_id) {
                if let Some(strategy_attr) = func_attrs.get("supervision_strategy") {
                    if let AttributeValue::String(strategy_name) = strategy_attr {
                        strategy = Some(match strategy_name.as_str() {
                            "one_for_one" => SupervisionStrategy::OneForOne,
                            "one_for_all" => SupervisionStrategy::OneForAll,
                            "rest_for_one" => SupervisionStrategy::RestForOne,
                            _ => SupervisionStrategy::OneForOne, // デフォルト
                        });
                    }
                }
            }
        }
        
        // 関数本体から戦略を抽出
        for stmt in body {
            if let Statement::Expression(Expression::MethodCall(_, method_name, _)) = stmt {
                if method_name == "with_strategy" || method_name == "set_supervision_strategy" {
                    // 戦略設定メソッドの引数を解析
                    if let Statement::Expression(Expression::MethodCall(_, _, args)) = stmt {
                        if !args.is_empty() {
                            if let Expression::Variable(strategy_name) = &args[0] {
                                strategy = Some(match strategy_name.as_str() {
                                    "ONE_FOR_ONE" | "OneForOne" => SupervisionStrategy::OneForOne,
                                    "ONE_FOR_ALL" | "OneForAll" => SupervisionStrategy::OneForAll,
                                    "REST_FOR_ONE" | "RestForOne" => SupervisionStrategy::RestForOne,
                                    _ => SupervisionStrategy::OneForOne,
                                });
                            }
                        }
                    }
                }
            }
        }
        
        Ok(strategy)
    }
    
    /// 型名から型IDを解決
    fn resolve_type_name(&self, type_name: &str) -> Result<Option<TypeId>, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        for (type_id, ty) in &module.types {
            match ty {
                Type::Struct(name, _) | Type::Enum(name, _) | Type::Interface(name, _) => {
                    if name == type_name {
                        return Ok(Some(*type_id));
                    }
                },
                _ => {}
            }
        }
        
        Ok(None)
    }
    
    /// 関数名から関数IDを解決
    fn resolve_function_name(&self, func_name: &str) -> Result<Option<FunctionId>, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        for (func_id, function) in &module.functions {
            if function.name == func_name {
                return Ok(Some(*func_id));
            }
        }
        
        Ok(None)
    }
    
    /// 型がアクター型かどうかを判定
    fn is_actor_type(&self, type_id: TypeId) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // アクター型の条件をチェック
        if let Some(ty) = module.types.get(&type_id) {
            match ty {
                Type::Struct(name, _) => {
                    // アクタートレイトを実装しているか、または名前に "Actor" を含むかをチェック
                    if name.contains("Actor") {
                        return Ok(true);
                    }
                    
                    // 型の実装トレイトをチェック
                    if let Some(implemented_traits) = module.implemented_traits.get(&type_id) {
                        for trait_id in implemented_traits {
                            if let Some(trait_type) = module.types.get(trait_id) {
                                if let Type::Trait(trait_name, _) = trait_type {
                                    if trait_name == "Actor" || trait_name.contains("Actor") {
                                        return Ok(true);
                                    }
                                }
                            }
                        }
                    }
                    
                    // アクター属性をチェック
                    if let Some(attrs) = module.attributes.get(&type_id) {
                        if attrs.contains_key("actor") {
                            return Ok(true);
                        }
                    }
                },
                Type::Interface(name, _) => {
                    // アクターインターフェースかチェック
                    if name.contains("Actor") {
                        return Ok(true);
                    }
                },
                _ => {}
            }
        }
        
        Ok(false)
    }
    
    /// 型がアクターのコレクション型かどうかを判定
    fn is_actor_collection_type(&self, type_id: TypeId) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        if let Some(ty) = module.types.get(&type_id) {
            match ty {
                Type::Generic(base_name, params) => {
                    // Vec<Actor>などのコレクション型をチェック
                    if base_name == "Vec" || base_name == "Array" || base_name == "List" || 
                       base_name == "Set" || base_name == "HashSet" || base_name == "Collection" {
                        if !params.is_empty() {
                            return self.is_actor_type(params[0]);
                        }
                    }
                },
                _ => {}
            }
        }
        
        Ok(false)
    }
    
    /// コレクション型から要素型を抽出
    fn extract_collection_element_type(&self, type_id: TypeId) -> Result<Option<TypeId>, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        if let Some(ty) = module.types.get(&type_id) {
            if let Type::Generic(_, params) = ty {
                if !params.is_empty() {
                    return Ok(Some(params[0]));
                }
            }
        }
        
        Ok(None)
    }
    
    /// メソッド呼び出しの戻り値型を取得
    fn get_method_return_type(&self, obj: &Expression, method_name: &str) -> Result<Option<TypeId>, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // オブジェクトの型を特定
        let obj_type_id = self.infer_expression_type(obj)?;
        
        if let Some(obj_type_id) = obj_type_id {
            // 型のメソッドを検索
            for (func_id, function) in &module.functions {
                if function.name == method_name && self.is_method_of_type(*func_id, obj_type_id)? {
                    return Ok(function.return_type);
                }
            }
        }
        
        Ok(None)
    }
    
    /// 式の型を推論
    fn infer_expression_type(&self, expr: &Expression) -> Result<Option<TypeId>, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        match expr {
            Expression::Variable(name) => {
                // 変数の型を検索
                for (var_id, var_info) in &module.variables {
                    if var_info.name == *name {
                        return Ok(var_info.type_id);
                    }
                }
                Ok(None)
            },
            Expression::New(type_id, _) => {
                // 新しいインスタンスの型
                Ok(Some(*type_id))
            },
            Expression::MethodCall(obj, method_name, _) => {
                // メソッド呼び出しの戻り値型
                self.get_method_return_type(obj, method_name)
            },
            Expression::FunctionCall(func_name, _) => {
                // 関数呼び出しの戻り値型
                if let Some(func_id) = self.resolve_function_name(func_name)? {
                    if let Some(function) = module.functions.get(&func_id) {
                        return Ok(function.return_type);
                    }
                }
                Ok(None)
            },
            // その他の式も必要に応じて解析
            _ => Ok(None),
        }
    }
    
    /// 関数が特定の型のメソッドかどうかをチェック
    fn is_method_of_type(&self, func_id: FunctionId, type_id: TypeId) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        if let Some(methods) = module.type_methods.get(&type_id) {
            return Ok(methods.contains(&func_id));
        }
        
        Ok(false)
    }
    
    /// アクターの親クラスを取得
    fn get_parent_actor(&self, actor_type_id: TypeId) -> Result<Option<TypeId>, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        if let Some(parent_id) = module.type_hierarchy.get(&actor_type_id) {
            if self.is_actor_type(*parent_id)? {
                return Ok(Some(*parent_id));
            }
        }
        
        Ok(None)
    }
    /// メッセージ型を検出
    fn detect_message_types(&mut self) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // メッセージとして扱われる型を検出
        for (type_id, ty) in &module.types {
            // 型がメッセージマーカートレイトを実装しているかチェック
            if self.is_message_type(*type_id, ty)? {
                self.message_types.insert(*type_id);
            }
        }
        
        Ok(())
    }
    
    /// 型がメッセージ型かどうかを判定
    fn is_message_type(&self, type_id: TypeId, ty: &Type) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // メッセージ型の条件をチェック
        match ty {
            Type::Struct(name, fields) => {
                // 1. 明示的なメッセージマーカートレイト実装をチェック
                if let Some(trait_impls) = module.type_trait_impls.get(&type_id) {
                    for &trait_id in trait_impls {
                        if let Some(trait_def) = module.traits.get(&trait_id) {
                            if trait_def.name == "Message" || trait_def.name == "ActorMessage" {
                                return Ok(true);
                            }
                        }
                    }
                }
                
                // 2. 型属性をチェック
                if let Some(type_attrs) = module.type_attributes.get(&type_id) {
                    if type_attrs.contains_key("message") || type_attrs.contains_key("actor_message") {
                        return Ok(true);
                    }
                }
                
                // 3. シリアライズ可能性をチェック（メッセージは送信可能である必要がある）
                let is_serializable = self.is_serializable_type(type_id)?;
                if !is_serializable {
                    return Ok(false);
                }
                
                // 4. 不変性チェック（メッセージは不変であるべき）
                let is_immutable = fields.iter().all(|field| {
                    if let Some(field_type) = module.types.get(&field.type_id) {
                        match field_type {
                            Type::Ref(_, is_mut) => !is_mut,
                            _ => true // 参照でない型は不変と見なす
                        }
                    } else {
                        false // 型情報が見つからない場合は安全のためfalse
                    }
                });
                
                if !is_immutable {
                    return Ok(false);
                }
                
                // 5. 命名規則によるヒューリスティック（最後の手段）
                if name.contains("Message") || name.ends_with("Msg") || name.ends_with("Event") {
                    return Ok(true);
                }
                
                // 6. メッセージハンドラの存在をチェック
                for (_, actor) in &self.actors {
                    for method in &actor.methods {
                        if let Some(function) = module.functions.get(&method.function_id) {
                            if function.parameters.len() >= 2 && function.parameters[1].type_id == type_id {
                                // 第2引数がこの型の場合、メッセージハンドラの可能性が高い
                                if let Some(attrs) = module.function_attributes.get(&method.function_id) {
                                    if attrs.contains_key("handler") || attrs.contains_key("message_handler") {
                                        return Ok(true);
                                    }
                                }
                                
                                // 関数名がhandle_で始まる場合もハンドラと見なす
                                if function.name.starts_with("handle_") {
                                    return Ok(true);
                                }
                            }
                        }
                    }
                }
                
                Ok(false)
            },
            Type::Enum(name, variants) => {
                // 列挙型の場合も同様のチェックを行う
                // 1. トレイト実装チェック
                if let Some(trait_impls) = module.type_trait_impls.get(&type_id) {
                    for &trait_id in trait_impls {
                        if let Some(trait_def) = module.traits.get(&trait_id) {
                            if trait_def.name == "Message" || trait_def.name == "ActorMessage" {
                                return Ok(true);
                            }
                        }
                    }
                }
                
                // 2. 型属性チェック
                if let Some(type_attrs) = module.type_attributes.get(&type_id) {
                    if type_attrs.contains_key("message") || type_attrs.contains_key("actor_message") {
                        return Ok(true);
                    }
                }
                
                // 3. シリアライズ可能性チェック
                let is_serializable = self.is_serializable_type(type_id)?;
                if !is_serializable {
                    return Ok(false);
                }
                
                // 4. バリアントの不変性チェック
                let all_variants_immutable = variants.iter().all(|variant| {
                    variant.fields.iter().all(|field| {
                        if let Some(field_type) = module.types.get(&field.type_id) {
                            match field_type {
                                Type::Ref(_, is_mut) => !is_mut,
                                _ => true
                            }
                        } else {
                            false
                        }
                    })
                });
                
                if !all_variants_immutable {
                    return Ok(false);
                }
                
                // 5. 命名規則
                if name.contains("Message") || name.ends_with("Msg") || name.ends_with("Event") {
                    return Ok(true);
                }
                
                // 6. メッセージハンドラの存在チェック（構造体と同様）
                for (_, actor) in &self.actors {
                    for method in &actor.methods {
                        if let Some(function) = module.functions.get(&method.function_id) {
                            if function.parameters.len() >= 2 && function.parameters[1].type_id == type_id {
                                if let Some(attrs) = module.function_attributes.get(&method.function_id) {
                                    if attrs.contains_key("handler") || attrs.contains_key("message_handler") {
                                        return Ok(true);
                                    }
                                }
                                
                                if function.name.starts_with("handle_") {
                                    return Ok(true);
                                }
                            }
                        }
                    }
                }
                
                Ok(false)
            },
            Type::Tuple(element_types) => {
                // タプル型の場合、全ての要素がシリアライズ可能かつ不変であればメッセージとして扱う
                let all_elements_valid = element_types.iter().all(|&elem_type_id| {
                    if let Ok(is_serializable) = self.is_serializable_type(elem_type_id) {
                        is_serializable
                    } else {
                        false
                    }
                });
                
                Ok(all_elements_valid)
            },
            Type::Unit => {
                // ユニット型はメッセージとして有効（シグナルとして使用可能）
                Ok(true)
            },
            Type::Custom(name) => {
                // カスタム型の場合、名前ベースのヒューリスティックを使用
                Ok(name.contains("Message") || name.ends_with("Msg") || name.ends_with("Event"))
            },
            _ => Ok(false), // その他の型はメッセージとして扱わない
        }
    }
    
    /// 型がシリアライズ可能かどうかを判定
    fn is_serializable_type(&self, type_id: TypeId) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // 型がSerializeトレイトを実装しているかチェック
        if let Some(trait_impls) = module.type_trait_impls.get(&type_id) {
            for &trait_id in trait_impls {
                if let Some(trait_def) = module.traits.get(&trait_id) {
                    if trait_def.name == "Serialize" || trait_def.name == "Serializable" {
                        return Ok(true);
                    }
                }
            }
        }
        
        // 基本型はシリアライズ可能
        if let Some(ty) = module.types.get(&type_id) {
            match ty {
                Type::Int(_) | Type::Float(_) | Type::Bool | Type::Char | Type::String | Type::Unit => {
                    return Ok(true);
                },
                Type::Array(elem_type_id) | Type::Vec(elem_type_id) => {
                    // 配列/ベクタの要素がシリアライズ可能であれば、配列/ベクタもシリアライズ可能
                    return self.is_serializable_type(*elem_type_id);
                },
                Type::Tuple(elem_type_ids) => {
                    // タプルの全要素がシリアライズ可能であれば、タプルもシリアライズ可能
                    for &elem_id in elem_type_ids {
                        if !self.is_serializable_type(elem_id)? {
                            return Ok(false);
                        }
                    }
                    return Ok(true);
                },
                Type::Option(inner_type_id) | Type::Result(inner_type_id, _) => {
                    // Option/Resultの内部型がシリアライズ可能であれば、Option/Resultもシリアライズ可能
                    return self.is_serializable_type(*inner_type_id);
                },
                Type::Map(key_type_id, value_type_id) => {
                    // キーと値の両方がシリアライズ可能であれば、マップもシリアライズ可能
                    return Ok(self.is_serializable_type(*key_type_id)? && self.is_serializable_type(*value_type_id)?);
                },
                Type::Struct(_, fields) => {
                    // 構造体の全フィールドがシリアライズ可能であれば、構造体もシリアライズ可能
                    for field in fields {
                        if !self.is_serializable_type(field.type_id)? {
                            return Ok(false);
                        }
                    }
                    return Ok(true);
                },
                Type::Enum(_, variants) => {
                    // 列挙型の全バリアントの全フィールドがシリアライズ可能であれば、列挙型もシリアライズ可能
                    for variant in variants {
                        for field in &variant.fields {
                            if !self.is_serializable_type(field.type_id)? {
                                return Ok(false);
                            }
                        }
                    }
                    return Ok(true);
                },
                _ => return Ok(false),
            }
        }
        
        Ok(false)
    }
    
    /// アクター階層を構築
    fn build_actor_hierarchy(&mut self) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // アクター間の親子関係を分析
        for (actor_id, actor) in &self.actors {
            // 1. 監視関係から親子関係を構築
            for &supervisee in &actor.supervisees {
                self.hierarchy.entry(*actor_id)
                    .or_insert_with(Vec::new)
                    .push(supervisee);
            }
            
            // 2. 型階層からの継承関係を検出
            if let Some(parent_type_id) = module.type_hierarchy.get(&actor.type_id) {
                if self.is_actor_type(*parent_type_id)? {
                    // 親型がアクターの場合、階層に追加
                    if let Some(parent_actor_id) = self.find_actor_by_type_id(*parent_type_id) {
                        self.inheritance_hierarchy.entry(parent_actor_id)
                            .or_insert_with(Vec::new)
                            .push(*actor_id);
                    }
                }
            }
            
            // 3. 委譲関係を検出（アクターが他のアクターをフィールドとして持つ場合）
            if let Some(actor_type) = module.types.get(&actor.type_id) {
                if let Type::Struct(_, fields) = actor_type {
                    for field in fields {
                        if self.is_actor_type(field.type_id)? {
                            // フィールドの型がアクターの場合、委譲関係として記録
                            if let Some(delegated_actor_id) = self.find_actor_by_type_id(field.type_id) {
                                self.delegation_hierarchy.entry(*actor_id)
                                    .or_insert_with(Vec::new)
                                    .push(delegated_actor_id);
                            }
                        }
                    }
                }
            }
            
            // 4. メッセージ転送関係を検出
            for method in &actor.methods {
                if let Some(function) = module.functions.get(&method.function_id) {
                    // 関数本体を解析してメッセージ転送を検出
                    for block in &function.basic_blocks {
                        for &inst_id in &block.instructions {
                            if let Some(inst) = function.instructions.get(&inst_id) {
                                if inst.opcode == "send_message" || inst.opcode == "forward_message" {
                                    if inst.operands.len() >= 2 {
                                        let target_actor_operand = inst.operands[0];
                                        // ターゲットアクターの型を解決
                                        if let Some(target_type_id) = self.resolve_operand_type(function, target_actor_operand) {
                                            if self.is_actor_type(target_type_id)? {
                                                if let Some(target_actor_id) = self.find_actor_by_type_id(target_type_id) {
                                                    self.message_forwarding.entry(*actor_id)
                                                        .or_insert_with(HashSet::new)
                                                        .insert(target_actor_id);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // 5. アクター生成関係を検出（あるアクターが別のアクターを生成する場合）
            for method in &actor.methods {
                if let Some(function) = module.functions.get(&method.function_id) {
                    for block in &function.basic_blocks {
                        for &inst_id in &block.instructions {
                            if let Some(inst) = function.instructions.get(&inst_id) {
                                if inst.opcode == "create_actor" || inst.opcode == "spawn_actor" {
                                    if inst.operands.len() >= 1 {
                                        let actor_type_operand = inst.operands[0];
                                        // 生成されるアクターの型を解決
                                        if let Some(created_type_id) = self.resolve_type_operand(function, actor_type_operand) {
                                            if self.is_actor_type(created_type_id)? {
                                                if let Some(created_actor_id) = self.find_actor_by_type_id(created_type_id) {
                                                    self.creation_hierarchy.entry(*actor_id)
                                                        .or_insert_with(HashSet::new)
                                                        .insert(created_actor_id);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // 6. 階層の整合性チェックと最適化
        self.validate_actor_hierarchy()?;
        self.optimize_actor_hierarchy()?;
        
        Ok(())
    }
    
    /// アクター階層の整合性をチェック
    fn validate_actor_hierarchy(&self) -> Result<(), String> {
        // 循環依存関係のチェック
        self.check_circular_dependencies()?;
        
        // 孤立アクターのチェック（どの階層にも属さないアクター）
        self.check_isolated_actors()?;
        
        // 監視階層と継承階層の矛盾チェック
        self.check_hierarchy_conflicts()?;
        
        Ok(())
    }
    
    /// アクター階層の循環依存関係をチェック
    fn check_circular_dependencies(&self) -> Result<(), String> {
        // 監視階層の循環依存チェック
        let mut visited = HashSet::new();
        let mut path = Vec::new();
        
        for &actor_id in self.actors.keys() {
            if !visited.contains(&actor_id) {
                if let Err(cycle_path) = self.dfs_check_cycles(actor_id, &mut visited, &mut path, &self.hierarchy) {
                    return Err(format!("アクター階層に循環依存関係が検出されました: {}", 
                        cycle_path.iter()
                            .map(|&id| self.get_actor_name(id).unwrap_or("不明".to_string()))
                            .collect::<Vec<_>>()
                            .join(" -> ")));
                }
            }
        }
        
        // 継承階層の循環依存チェック
        let mut visited = HashSet::new();
        let mut path = Vec::new();
        
        for &actor_id in self.actors.keys() {
            if !visited.contains(&actor_id) {
                if let Err(cycle_path) = self.dfs_check_cycles(actor_id, &mut visited, &mut path, &self.inheritance_hierarchy) {
                    return Err(format!("アクター継承階層に循環依存関係が検出されました: {}", 
                        cycle_path.iter()
                            .map(|&id| self.get_actor_name(id).unwrap_or("不明".to_string()))
                            .collect::<Vec<_>>()
                            .join(" -> ")));
                }
            }
        }
        
        // 委譲階層の循環依存チェック
        let mut visited = HashSet::new();
        let mut path = Vec::new();
        
        for &actor_id in self.actors.keys() {
            if !visited.contains(&actor_id) {
                if let Err(cycle_path) = self.dfs_check_cycles(actor_id, &mut visited, &mut path, &self.delegation_hierarchy) {
                    return Err(format!("アクター委譲階層に循環依存関係が検出されました: {}", 
                        cycle_path.iter()
                            .map(|&id| self.get_actor_name(id).unwrap_or("不明".to_string()))
                            .collect::<Vec<_>>()
                            .join(" -> ")));
                }
            }
        }
        
        Ok(())
    }
    
    /// 深さ優先探索で循環依存関係をチェック
    fn dfs_check_cycles(&self, 
                        current: ActorId, 
                        visited: &mut HashSet<ActorId>, 
                        path: &mut Vec<ActorId>,
                        hierarchy: &HashMap<ActorId, Vec<ActorId>>) -> Result<(), Vec<ActorId>> {
        if path.contains(&current) {
            // 循環を検出
            let mut cycle = Vec::new();
            let start_idx = path.iter().position(|&id| id == current).unwrap();
            cycle.extend_from_slice(&path[start_idx..]);
            cycle.push(current);
            return Err(cycle);
        }
        
        if visited.contains(&current) {
            return Ok(());
        }
        
        visited.insert(current);
        path.push(current);
        
        if let Some(children) = hierarchy.get(&current) {
            for &child in children {
                if let Err(cycle) = self.dfs_check_cycles(child, visited, path, hierarchy) {
                    return Err(cycle);
                }
            }
        }
        
        path.pop();
        Ok(())
    }
    
    /// 孤立アクターをチェック
    fn check_isolated_actors(&self) -> Result<(), String> {
        let mut connected_actors = HashSet::new();
        
        // 監視階層に含まれるアクターを収集
        for (parent, children) in &self.hierarchy {
            connected_actors.insert(*parent);
            connected_actors.extend(children);
        }
        
        // 継承階層に含まれるアクターを収集
        for (parent, children) in &self.inheritance_hierarchy {
            connected_actors.insert(*parent);
            connected_actors.extend(children);
        }
        
        // 委譲階層に含まれるアクターを収集
        for (delegator, delegatees) in &self.delegation_hierarchy {
            connected_actors.insert(*delegator);
            connected_actors.extend(delegatees);
        }
        
        // メッセージ転送関係に含まれるアクターを収集
        for (sender, receivers) in &self.message_forwarding {
            connected_actors.insert(*sender);
            connected_actors.extend(receivers);
        }
        
        // 孤立アクターを検出
        let isolated_actors: Vec<_> = self.actors.keys()
            .filter(|&id| !connected_actors.contains(id))
            .collect();
        
        if !isolated_actors.is_empty() {
            // 警告としてログに記録（エラーではない）
            println!("警告: 孤立したアクターが検出されました: {}", 
                isolated_actors.iter()
                    .map(|&id| self.get_actor_name(id).unwrap_or("不明".to_string()))
                    .collect::<Vec<_>>()
                    .join(", "));
        }
        
        Ok(())
    }
    
    /// 階層間の矛盾をチェック
    fn check_hierarchy_conflicts(&self) -> Result<(), String> {
        // 監視階層と継承階層の矛盾をチェック
        for (parent, children) in &self.inheritance_hierarchy {
            for child in children {
                // 継承関係にあるアクターが監視階層で逆の関係になっていないかチェック
                if let Some(supervisees) = self.hierarchy.get(child) {
                    if supervisees.contains(parent) {
                        return Err(format!(
                            "階層の矛盾: アクター {} は {} の親クラスですが、{} によって監視されています",
                            self.get_actor_name(*parent).unwrap_or("不明".to_string()),
                            self.get_actor_name(*child).unwrap_or("不明".to_string()),
                            self.get_actor_name(*child).unwrap_or("不明".to_string())
                        ));
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// アクター階層を最適化
    fn optimize_actor_hierarchy(&mut self) -> Result<(), String> {
        // 冗長な関係を削除
        self.remove_redundant_relationships()?;
        
        // 階層の深さを最適化
        self.optimize_hierarchy_depth()?;
        
        Ok(())
    }
    
    /// 冗長な関係を削除
    fn remove_redundant_relationships(&mut self) -> Result<(), String> {
        // 推移的関係を削除（A→B→CがあるときにA→Cを削除）
        let mut redundant_edges = Vec::new();
        
        for (&parent, children) in &self.hierarchy {
            for &child in children {
                // 孫アクターを取得
                if let Some(grandchildren) = self.hierarchy.get(&child) {
                    for &grandchild in grandchildren {
                        // 親が孫と直接関係を持っている場合、それは冗長
                        if children.contains(&grandchild) {
                            redundant_edges.push((parent, grandchild));
                        }
                    }
                }
            }
        }
        
        // 冗長な関係を削除
        for (parent, child) in redundant_edges {
            if let Some(children) = self.hierarchy.get_mut(&parent) {
                children.retain(|&c| c != child);
            }
        }
        
        Ok(())
    }
    
    /// 階層の深さを最適化
    fn optimize_hierarchy_depth(&mut self) -> Result<(), String> {
        // 階層が深すぎる場合に中間層を圧縮
        // この実装は複雑なため、実際のユースケースに応じて調整が必要
        
        // 各アクターの深さを計算
        let depths = self.calculate_actor_depths()?;
        
        // 深さが一定以上のパスを検出し、必要に応じて最適化
        let max_recommended_depth = 5; // 推奨最大深さ
        
        let deep_paths = self.find_deep_paths(&depths, max_recommended_depth)?;
        
        if !deep_paths.is_empty() {
            println!("警告: 次のアクター階層パスが深すぎます（最適化を検討してください）:");
            for path in &deep_paths {
                println!("  {}", path.iter()
                    .map(|&id| self.get_actor_name(id).unwrap_or("不明".to_string()))
                    .collect::<Vec<_>>()
                    .join(" -> "));
            }
        }
        
        Ok(())
    }
    
    /// 各アクターの階層深さを計算
    fn calculate_actor_depths(&self) -> Result<HashMap<ActorId, usize>, String> {
        let mut depths = HashMap::new();
        let mut visited = HashSet::new();
        
        // ルートアクターを特定（親を持たないアクター）
        let root_actors: Vec<_> = self.actors.keys()
            .filter(|&id| {
                !self.hierarchy.values().any(|children| children.contains(id))
            })
            .collect();
        
        // 各ルートアクターから深さを計算
        for &root in &root_actors {
            self.calculate_depth_dfs(root, 0, &mut depths, &mut visited)?;
        }
        
        Ok(depths)
    }
    
    /// 深さ優先探索で階層の深さを計算
    fn calculate_depth_dfs(&self, 
                          current: ActorId, 
                          depth: usize,
                          depths: &mut HashMap<ActorId, usize>,
                          visited: &mut HashSet<ActorId>) -> Result<(), String> {
        if visited.contains(&current) {
            return Ok(());
        }
        
        visited.insert(current);
        
        // 現在の深さを記録（既存の値より大きい場合のみ更新）
        depths.entry(current)
            .and_modify(|d| *d = (*d).max(depth))
            .or_insert(depth);
        
        // 子アクターを処理
        if let Some(children) = self.hierarchy.get(&current) {
            for &child in children {
                self.calculate_depth_dfs(child, depth + 1, depths, visited)?;
            }
        }
        
        Ok(())
    }
    
    /// 深いパスを検出
    fn find_deep_paths(&self, depths: &HashMap<ActorId, usize>, max_depth: usize) -> Result<Vec<Vec<ActorId>>, String> {
        let mut deep_paths = Vec::new();
        let mut visited = HashSet::new();
        
        // 深さが最大値のアクターを特定
        let deep_actors: Vec<_> = depths.iter()
            .filter(|(_, &depth)| depth >= max_depth)
            .map(|(&id, _)| id)
            .collect();
        
        // 各深いアクターから根までのパスを構築
        for &actor_id in &deep_actors {
            let mut path = Vec::new();
            self.build_path_to_root(actor_id, &mut path, &mut visited)?;
            
            if path.len() >= max_depth {
                deep_paths.push(path);
            }
        }
        
        Ok(deep_paths)
    }
    
    /// アクターから根までのパスを構築
    fn build_path_to_root(&self, 
                         current: ActorId, 
                         path: &mut Vec<ActorId>,
                         visited: &mut HashSet<ActorId>) -> Result<bool, String> {
        if visited.contains(&current) {
            return Ok(false); // 循環を検出
        }
        
        visited.insert(current);
        path.push(current);
        
        // 親アクターを検索
        let mut found_parent = false;
        for (&parent, children) in &self.hierarchy {
            if children.contains(&current) {
                found_parent = self.build_path_to_root(parent, path, visited)?;
                if found_parent {
                    break;
                }
            }
        }
        
        if !found_parent {
            return Ok(false); // 循環を検出
        }
        
        Ok(true)
    }
    
    /// アクターメソッドの並行処理方式を分析
    /// 
    /// 各アクターメソッドの並行処理モードを決定し、最適な実行戦略を選択します。
    /// 分析は以下の要素に基づいて行われます：
    /// - メソッド属性（async, readonly, exclusive, isolated）
    /// - アクセスするフィールド
    /// - 呼び出し関係
    /// - データフロー分析
    /// - 副作用の有無
    fn analyze_concurrency_modes(&mut self) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // 依存関係グラフを構築
        let mut method_dependencies = self.build_method_dependency_graph()?;
        
        // 各アクターのメソッドを分析
        for (actor_id, actor) in &mut self.actors {
            // アクターの状態分析を実行
            let state_isolation_groups = self.analyze_state_isolation(actor)?;
            
            for method in &mut actor.methods {
                // 関数を取得
                if let Some(function) = module.functions.get(&method.function_id) {
                    // 1. 属性からの明示的な指定を優先
                    if function.attributes.contains_key("async") {
                        method.concurrency_mode = ConcurrencyMode::Async;
                    } else if function.attributes.contains_key("readonly") {
                        method.concurrency_mode = ConcurrencyMode::ReadOnly;
                    } else if function.attributes.contains_key("exclusive") {
                        method.concurrency_mode = ConcurrencyMode::Exclusive;
                    } else if let Some(isolated_attr) = function.attributes.get("isolated") {
                        // isolated属性の引数を解析
                        if let Some(field_names) = isolated_attr.get_string_list() {
                            // フィールド名からフィールドインデックスを取得
                            let isolated_fields = self.resolve_field_indices(actor, &field_names)?;
                            
                            // 分離グループを特定
                            if let Some(group_id) = self.find_isolation_group(&isolated_fields, &state_isolation_groups) {
                                method.concurrency_mode = ConcurrencyMode::Isolated(group_id);
                            } else {
                                // 新しい分離グループを作成
                                let new_group_id = state_isolation_groups.len();
                                method.concurrency_mode = ConcurrencyMode::Isolated(new_group_id);
                            }
                        } else {
                            // 引数なしの場合は自動検出
                            method.accessed_state = self.analyze_state_access(&method.function_id, actor)?;
                            
                            if method.accessed_state.is_empty() {
                                // 状態にアクセスしない場合は非同期
                                method.concurrency_mode = ConcurrencyMode::Async;
                            } else {
                                // 適切な分離グループを見つける
                                if let Some(group_id) = self.find_isolation_group(&method.accessed_state, &state_isolation_groups) {
                                    method.concurrency_mode = ConcurrencyMode::Isolated(group_id);
                                } else {
                                    // 新しい分離グループを作成
                                    let new_group_id = state_isolation_groups.len();
                                    method.concurrency_mode = ConcurrencyMode::Isolated(new_group_id);
                                }
                            }
                        }
                    } else {
                        // 2. 明示的な指定がない場合は自動検出
                        method.accessed_state = self.analyze_state_access(&method.function_id, actor)?;
                        
                        // 副作用分析
                        let has_side_effects = self.analyze_side_effects(&method.function_id)?;
                        
                        // データフロー分析
                        let data_flow_info = self.analyze_data_flow(&method.function_id)?;
                        
                        // 並行性モードの自動決定
                        if method.accessed_state.is_empty() && !has_side_effects {
                            // 状態にアクセスせず副作用もない場合は非同期
                            method.concurrency_mode = ConcurrencyMode::Async;
                        } else if !has_side_effects && data_flow_info.is_read_only {
                            // 読み取り専用の場合
                            method.concurrency_mode = ConcurrencyMode::ReadOnly;
                        } else if method.accessed_state.len() == actor.fields.len() {
                            // すべてのフィールドにアクセスする場合は排他的
                            method.concurrency_mode = ConcurrencyMode::Exclusive;
                        } else {
                            // 部分的なフィールドアクセスの場合は分離
                            if let Some(group_id) = self.find_isolation_group(&method.accessed_state, &state_isolation_groups) {
                                method.concurrency_mode = ConcurrencyMode::Isolated(group_id);
                            } else {
                                // 新しい分離グループを作成
                                let new_group_id = state_isolation_groups.len();
                                method.concurrency_mode = ConcurrencyMode::Isolated(new_group_id);
                            }
                        }
                    }
                    
                    // 3. 最適化: 依存関係に基づく調整
                    self.optimize_concurrency_mode(method, &method_dependencies, actor_id)?;
                    
                    // 4. 実行コストの推定と最適化
                    self.estimate_execution_cost(method, function)?;
                    
                    // 5. 並行実行の安全性検証
                    self.verify_concurrency_safety(method, actor)?;
                }
            }
            
            // 6. アクター全体の並行性最適化
            self.optimize_actor_concurrency(actor)?;
        }
        
        // 7. アクター間の並行性最適化
        self.optimize_inter_actor_concurrency()?;
        
        Ok(())
    }
    
    /// メソッド間の依存関係グラフを構築
    fn build_method_dependency_graph(&self) -> Result<HashMap<FunctionId, Vec<FunctionId>>, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        let mut dependencies = HashMap::new();
        
        // 各アクターのメソッドを分析
        for (_, actor) in &self.actors {
            for method in &actor.methods {
                let mut method_deps = Vec::new();
                
                if let Some(function) = module.functions.get(&method.function_id) {
                    // 関数呼び出しを検出
                    for block in &function.basic_blocks {
                        for &inst_id in &block.instructions {
                            if let Some(inst) = function.instructions.get(&inst_id) {
                                if inst.opcode == "call" || inst.opcode == "invoke" {
                                    if let Some(&callee_id) = inst.operands.get(0) {
                                        // 関数IDに変換
                                        let callee_func_id = FunctionId(callee_id as u32);
                                        method_deps.push(callee_func_id);
                                    }
                                }
                            }
                        }
                    }
                }
                
                dependencies.insert(method.function_id.clone(), method_deps);
            }
        }
        
        Ok(dependencies)
    }
    
    /// アクターの状態分離グループを分析
    fn analyze_state_isolation(&self, actor: &Actor) -> Result<Vec<HashSet<usize>>, String> {
        let mut isolation_groups = Vec::new();
        let mut remaining_fields: HashSet<usize> = (0..actor.fields.len()).collect();
        
        // 1. フィールド間の依存関係を分析
        let field_dependencies = self.analyze_field_dependencies(actor)?;
        
        // 2. 強連結成分を見つけて分離グループを形成
        while !remaining_fields.is_empty() {
            let field = *remaining_fields.iter().next().unwrap();
            remaining_fields.remove(&field);
            
            let mut group = HashSet::new();
            group.insert(field);
            
            // 依存関係を追跡して関連フィールドを見つける
            let mut queue = vec![field];
            while let Some(current) = queue.pop() {
                if let Some(deps) = field_dependencies.get(&current) {
                    for &dep in deps {
                        if remaining_fields.contains(&dep) {
                            remaining_fields.remove(&dep);
                            group.insert(dep);
                            queue.push(dep);
                        }
                    }
                }
            }
            
            isolation_groups.push(group);
        }
        
        // 3. アクセスパターンに基づいて分離グループを最適化
        self.optimize_isolation_groups(&mut isolation_groups, actor)?;
        
        Ok(isolation_groups)
    }
    
    /// フィールド間の依存関係を分析
    fn analyze_field_dependencies(&self, actor: &Actor) -> Result<HashMap<usize, HashSet<usize>>, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        let mut dependencies = HashMap::new();
        
        // 各フィールドの初期依存関係を設定
        for i in 0..actor.fields.len() {
            dependencies.insert(i, HashSet::new());
        }
        
        // 各メソッドを分析して依存関係を構築
        for method in &actor.methods {
            if let Some(function) = module.functions.get(&method.function_id) {
                let mut field_reads = HashSet::new();
                let mut field_writes = HashSet::new();
                
                // フィールドの読み書きを検出
                for block in &function.basic_blocks {
                    for &inst_id in &block.instructions {
                        if let Some(inst) = function.instructions.get(&inst_id) {
                            if inst.opcode == "get_field" {
                                if let Some(&field_index) = inst.operands.get(1) {
                                    field_reads.insert(field_index as usize);
                                }
                            } else if inst.opcode == "set_field" {
                                if let Some(&field_index) = inst.operands.get(1) {
                                    field_writes.insert(field_index as usize);
                                }
                            }
                        }
                    }
                }
                
                // 書き込みフィールドは読み取りフィールドに依存
                for &write_field in &field_writes {
                    let deps = dependencies.entry(write_field).or_insert_with(HashSet::new);
                    deps.extend(&field_reads);
                }
            }
        }
        
        Ok(dependencies)
    }
    
    /// 分離グループを最適化
    fn optimize_isolation_groups(&self, groups: &mut Vec<HashSet<usize>>, actor: &Actor) -> Result<(), String> {
        // アクセス頻度に基づいて小さなグループをマージするかどうか決定
        let access_frequencies = self.analyze_field_access_frequencies(actor)?;
        
        // 小さすぎるグループを特定
        let mut small_groups = Vec::new();
        for (i, group) in groups.iter().enumerate() {
            if group.len() < 2 {  // 単一フィールドのグループ
                small_groups.push(i);
            }
        }
        
        // 小さなグループを適切な大きなグループにマージ
        let mut merged = HashSet::new();
        for &small_idx in &small_groups {
            if merged.contains(&small_idx) {
                continue;
            }
            
            let small_group = &groups[small_idx];
            let small_field = *small_group.iter().next().unwrap();
            
            // 最も相性の良いグループを見つける
            let mut best_group_idx = None;
            let mut best_compatibility = f64::NEG_INFINITY;
            
            for (i, group) in groups.iter().enumerate() {
                if i != small_idx && !merged.contains(&i) && group.len() > 1 {
                    // 互換性スコアを計算
                    let compatibility = self.calculate_group_compatibility(small_field, group, &access_frequencies);
                    if compatibility > best_compatibility {
                        best_compatibility = compatibility;
                        best_group_idx = Some(i);
                    }
                }
            }
            
            // 十分な互換性があれば、グループをマージ
            if let Some(target_idx) = best_group_idx {
                if best_compatibility > 0.5 {  // 閾値
                    let target_group = groups.get_mut(target_idx).unwrap();
                    target_group.extend(small_group);
                    merged.insert(small_idx);
                }
            }
        }
        
        // マージされたグループを削除
        let mut i = 0;
        while i < groups.len() {
            if merged.contains(&i) {
                groups.remove(i);
            } else {
                i += 1;
            }
        }
        
        Ok(())
    }
    
    /// フィールドアクセス頻度を分析
    fn analyze_field_access_frequencies(&self, actor: &Actor) -> Result<HashMap<usize, usize>, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        let mut frequencies = HashMap::new();
        
        // 各フィールドの初期頻度を0に設定
        for i in 0..actor.fields.len() {
            frequencies.insert(i, 0);
        }
        
        // 各メソッドでのフィールドアクセスを集計
        for method in &actor.methods {
            if let Some(function) = module.functions.get(&method.function_id) {
                for block in &function.basic_blocks {
                    for &inst_id in &block.instructions {
                        if let Some(inst) = function.instructions.get(&inst_id) {
                            if inst.opcode == "get_field" || inst.opcode == "set_field" {
                                if let Some(&field_index) = inst.operands.get(1) {
                                    let count = frequencies.entry(field_index as usize).or_insert(0);
                                    *count += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(frequencies)
    }
    
    /// グループ互換性スコアを計算
    fn calculate_group_compatibility(&self, field: usize, group: &HashSet<usize>, frequencies: &HashMap<usize, usize>) -> f64 {
        let field_freq = frequencies.get(&field).cloned().unwrap_or(0) as f64;
        
        // グループ内のフィールドの平均アクセス頻度
        let mut group_total_freq = 0.0;
        for &g_field in group {
            group_total_freq += frequencies.get(&g_field).cloned().unwrap_or(0) as f64;
        }
        let group_avg_freq = group_total_freq / group.len() as f64;
        
        // 頻度の類似性（0〜1）
        let freq_similarity = if field_freq == 0.0 && group_avg_freq == 0.0 {
            1.0  // 両方アクセスなし
        } else {
            let max_freq = field_freq.max(group_avg_freq);
            let min_freq = field_freq.min(group_avg_freq);
            min_freq / max_freq
        };
        
        freq_similarity
    }
    
    /// フィールド名からインデックスを解決
    fn resolve_field_indices(&self, actor: &Actor, field_names: &[String]) -> Result<HashSet<usize>, String> {
        let mut indices = HashSet::new();
        
        for name in field_names {
            let mut found = false;
            for (i, field) in actor.fields.iter().enumerate() {
                if &field.name == name {
                    indices.insert(i);
                    found = true;
                    break;
                }
            }
            
            if !found {
                return Err(format!("フィールド '{}' がアクター '{}' に見つかりません", name, actor.name));
            }
        }
        
        Ok(indices)
    }
    
    /// 指定されたフィールドセットに最適な分離グループを見つける
    fn find_isolation_group(&self, fields: &HashSet<usize>, groups: &[HashSet<usize>]) -> Option<usize> {
        for (i, group) in groups.iter().enumerate() {
            // フィールドセットがグループに完全に含まれているか
            if fields.is_subset(group) {
                return Some(i);
            }
        }
        None
    }
    
    /// 副作用の有無を分析
    fn analyze_side_effects(&self, func_id: &FunctionId) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        if let Some(function) = module.functions.get(func_id) {
            // 副作用を持つ可能性のある命令を検出
            for block in &function.basic_blocks {
                for &inst_id in &block.instructions {
                    if let Some(inst) = function.instructions.get(&inst_id) {
                        match inst.opcode.as_str() {
                            "set_field" | "store" | "call" | "invoke" | "send" | "spawn" => {
                                // 副作用を持つ命令
                                return Ok(true);
                            }
                            _ => {}
                        }
                    }
                }
            }
            
            // 呼び出し先の関数も再帰的に分析
            for block in &function.basic_blocks {
                for &inst_id in &block.instructions {
                    if let Some(inst) = function.instructions.get(&inst_id) {
                        if inst.opcode == "call" || inst.opcode == "invoke" {
                            if let Some(&callee_id) = inst.operands.get(0) {
                                let callee_func_id = FunctionId(callee_id as u32);
                                // 循環呼び出しを避けるための対策が必要
                                if callee_func_id != *func_id && self.analyze_side_effects(&callee_func_id)? {
                                    return Ok(true);
                                }
                            }
                        }
                    }
                }
            }
            
            // 副作用なし
            Ok(false)
        } else {
            // 関数が見つからない場合は安全のため副作用ありと判断
            Ok(true)
        }
    }
    
    /// データフロー分析を実行
    fn analyze_data_flow(&self, func_id: &FunctionId) -> Result<DataFlowInfo, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        let mut info = DataFlowInfo {
            is_read_only: true,
            has_message_passing: false,
            has_actor_creation: false,
        };
        
        if let Some(function) = module.functions.get(func_id) {
            // 各命令を分析
            for block in &function.basic_blocks {
                for &inst_id in &block.instructions {
                    if let Some(inst) = function.instructions.get(&inst_id) {
                        match inst.opcode.as_str() {
                            "set_field" | "store" => {
                                info.is_read_only = false;
                            }
                            "send" => {
                                info.has_message_passing = true;
                            }
                            "spawn" => {
                                info.has_actor_creation = true;
                            }
                            "call" | "invoke" => {
                                if let Some(&callee_id) = inst.operands.get(0) {
                                    let callee_func_id = FunctionId(callee_id as u32);
                                    // 循環呼び出しを避けるための対策が必要
                                    if callee_func_id != *func_id {
                                        if let Ok(callee_info) = self.analyze_data_flow(&callee_func_id) {
                                            info.is_read_only &= callee_info.is_read_only;
                                            info.has_message_passing |= callee_info.has_message_passing;
                                            info.has_actor_creation |= callee_info.has_actor_creation;
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        
        Ok(info)
    }
    
    /// 依存関係に基づいて並行性モードを最適化
    fn optimize_concurrency_mode(&self, method: &mut ActorMethod, dependencies: &HashMap<FunctionId, Vec<FunctionId>>, actor_id: ActorId) -> Result<(), String> {
        // 依存関係のある他のメソッドの並行性モードを考慮
        if let Some(deps) = dependencies.get(&method.function_id) {
            let mut requires_exclusive = false;
            
            for dep_id in deps {
                // 依存先がこのアクターのメソッドかどうか確認
                let mut is_same_actor_method = false;
                let mut dep_method_mode = None;
                
                for actor_method in self.get_actor_methods(actor_id)? {
                    if &actor_method.function_id == dep_id {
                        is_same_actor_method = true;
                        dep_method_mode = Some(actor_method.concurrency_mode);
                        break;
                    }
                }
                
                if is_same_actor_method {
                    if let Some(mode) = dep_method_mode {
                        match mode {
                            ConcurrencyMode::Exclusive => {
                                // 排他的メソッドに依存する場合、このメソッドも排他的にする必要がある
                                requires_exclusive = true;
                                break;
                            }
                            ConcurrencyMode::Isolated(group_id) => {
                                // 分離メソッドに依存する場合、互換性をチェック
                                if let ConcurrencyMode::Isolated(my_group_id) = method.concurrency_mode {
                                    if my_group_id != group_id {
                                        // 異なる分離グループに依存する場合、排他的にする必要がある
                                        requires_exclusive = true;
                                        break;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            
            if requires_exclusive {
                method.concurrency_mode = ConcurrencyMode::Exclusive;
            }
        }
        
        Ok(())
    }
    
    /// アクターのメソッドリストを取得
    fn get_actor_methods(&self, actor_id: ActorId) -> Result<Vec<&ActorMethod>, String> {
        if let Some(actor) = self.actors.get(&actor_id) {
            Ok(actor.methods.iter().collect())
        } else {
            Err(format!("アクターID {} が見つかりません", actor_id))
        }
    }
    
    /// 実行コストを推定
    fn estimate_execution_cost(&self, method: &mut ActorMethod, function: &Function) -> Result<(), String> {
        // 命令数に基づく基本コスト
        let mut instruction_count = 0;
        for block in &function.basic_blocks {
            instruction_count += block.instructions.len();
        }
        
        // 複雑な操作の検出
        let mut has_complex_operations = false;
        let mut has_loops = false;
        let mut has_recursion = false;
        
        // ループと再帰の検出
        let mut visited_blocks = HashSet::new();
        self.detect_loops_and_recursion(function, &mut visited_blocks, &mut has_loops, &mut has_recursion)?;
        
        // 複雑な操作の検出
        for block in &function.basic_blocks {
            for &inst_id in &block.instructions {
                if let Some(inst) = function.instructions.get(&inst_id) {
                    match inst.opcode.as_str() {
                        "div" | "rem" | "fmul" | "fdiv" => {
                            has_complex_operations = true;
                        }
                        _ => {}
                    }
                }
            }
        }
        
        // コスト係数の計算
        let mut cost_factor = 1.0;
        if has_complex_operations {
            cost_factor *= 1.5;
        }
        if has_loops {
            cost_factor *= 2.0;
        }
        if has_recursion {
            cost_factor *= 1.8;
        }
        
        // 推定コストを設定
        method.estimated_cost = (instruction_count as f64 * cost_factor) as usize;
        
        // コストに基づいて並行性モードを調整
        if method.estimated_cost > 1000 && method.concurrency_mode == ConcurrencyMode::Exclusive {
            // 高コストの排他的メソッドは分離を検討
            if !method.accessed_state.is_empty() && method.accessed_state.len() < function.basic_blocks.len() {
                // TODO: より洗練された分離戦略
            }
        }
        
        Ok(())
    }
    
    /// ループと再帰を検出
    fn detect_loops_and_recursion(&self, function: &Function, visited: &mut HashSet<usize>, has_loops: &mut bool, has_recursion: &mut bool) -> Result<(), String> {
        for block in &function.basic_blocks {
            if visited.contains(&block.id) {
                *has_loops = true;
                continue;
            }
            
            visited.insert(block.id);
            
            for &inst_id in &block.instructions {
                if let Some(inst) = function.instructions.get(&inst_id) {
                    if (inst.opcode == "call" || inst.opcode == "invoke") && inst.operands.get(0) == Some(&(function.id.0 as i64)) {
                        *has_recursion = true;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// 並行実行の安全性を検証
    fn verify_concurrency_safety(&self, method: &mut ActorMethod, actor: &Actor) -> Result<(), String> {
        match method.concurrency_mode {
            ConcurrencyMode::ReadOnly => {
                // 読み取り専用メソッドが状態を変更していないか確認
                for &field_idx in &method.accessed_state {
                    if self.is_field_modified(&method.function_id, field_idx)? {
                        return Err(format!(
                            "読み取り専用メソッドがフィールド {} を変更しています",
                            actor.fields.get(field_idx).map_or("不明", |f| &f.name)
                        ));
                    }
                }
            }
            ConcurrencyMode::Isolated(group_id) => {
                // 分離メソッドが他の分離グループのフィールドにアクセスしていないか確認
                for &field_idx in &method.accessed_state {
                    let belongs_to_group = actor.methods.iter()
                        .filter_map(|m| {
                            if let ConcurrencyMode::Isolated(g) = m.concurrency_mode {
                                if g == group_id && m.accessed_state.contains(&field_idx) {
                                    return Some(true);
                                }
                            }
                            None
                        })
                        .next()
                        .unwrap_or(false);
                    
                    if !belongs_to_group {
                        return Err(format!(
                            "分離メソッド（グループ {}）が他のグループのフィールド {} にアクセスしています",
                            group_id,
                            actor.fields.get(field_idx).map_or("不明", |f| &f.name)
                        ));
                    }
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    /// フィールドが変更されるかどうかを確認
    fn is_field_modified(&self, func_id: &FunctionId, field_idx: usize) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        if let Some(function) = module.functions.get(func_id) {
            for block in &function.basic_blocks {
                for &inst_id in &block.instructions {
                    if let Some(inst) = function.instructions.get(&inst_id) {
                        // フィールドアクセス命令を検出
                        if inst.opcode == "get_field" || inst.opcode == "set_field" {
                            if let Some(&field_index) = inst.operands.get(1) {
                                if field_index as usize == field_idx {
                                    return Ok(true);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(false)
    }
    
    /// メソッドがアクセスするアクター状態を分析
    fn analyze_state_access(&self, func_id: &FunctionId, actor: &Actor) -> Result<HashSet<usize>, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        let mut accessed_fields = HashSet::new();
        
        if let Some(function) = module.functions.get(func_id) {
            // 関数本体を解析してフィールドアクセスを検出
            for block in &function.basic_blocks {
                for &inst_id in &block.instructions {
                    if let Some(inst) = function.instructions.get(&inst_id) {
                        // フィールドアクセス命令を検出
                        if inst.opcode == "get_field" || inst.opcode == "set_field" {
                            if let Some(&field_index) = inst.operands.get(1) {
                                accessed_fields.insert(field_index as usize);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(accessed_fields)
    }
    
    /// アクターとメッセージの整合性をチェック
    pub fn validate(&self) -> Result<Vec<String>, String> {
        let mut warnings = Vec::new();
        
        // 各アクターが適切なメッセージハンドラを持っているかチェック
        for (actor_id, actor) in &self.actors {
            // アクターが処理するメッセージ型を特定
            let handled_messages = self.get_handled_messages(*actor_id)?;
            
            // メッセージハンドラの不足をチェック
            for &message_type in &self.message_types {
                if self.is_message_for_actor(message_type, *actor_id)? && !handled_messages.contains(&message_type) {
                    warnings.push(format!(
                        "アクター {} はメッセージ型 {:?} のハンドラを持っていません",
                        actor.name, message_type
                    ));
                }
            }
            
            // デッドロックの可能性をチェック
            if let Some(deadlock_path) = self.check_deadlock_possibility(*actor_id)? {
                warnings.push(format!(
                    "アクター {} は潜在的なデッドロックパス {} を持っています",
                    actor.name, deadlock_path.join(" -> ")
                ));
            }
        }
        
        Ok(warnings)
    }
    
    /// アクターが処理するメッセージ型を取得
    fn get_handled_messages(&self, actor_id: TypeId) -> Result<HashSet<TypeId>, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        let mut handled = HashSet::new();
        
        if let Some(actor) = self.actors.get(&actor_id) {
            for method in &actor.methods {
                if method.kind == ActorMethodKind::MessageHandler {
                    // ハンドラがどのメッセージ型を処理するか分析
                    if let Some(function) = module.functions.get(&method.function_id) {
                        // パラメータからメッセージ型を特定
                        if function.parameters.len() >= 2 {
                            // 最初のパラメータはself、2番目がメッセージ
                            let message_param = &function.parameters[1];
                            handled.insert(message_param.type_id);
                        }
                    }
                }
            }
        }
        
        Ok(handled)
    }
    
    /// メッセージ型がアクター用かどうかをチェック
    /// より高度なメッセージルーティング解析を行い、メッセージの属性、型情報、宛先指定などから
    /// 特定のアクターに向けられたメッセージかどうかを判断します
    fn is_message_for_actor(&self, message_type: TypeId, actor_id: TypeId) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // メッセージの型情報を取得
        if let Some(ty) = module.types.get(&message_type) {
            match ty {
                Type::Struct(name, fields) => {
                    // アクター情報を取得
                    if let Some(actor) = self.actors.get(&actor_id) {
                        // 1. 命名規則によるチェック（例: UserActorMessage）
                        if name.contains(&actor.name) {
                            return Ok(true);
                        }
                        
                        // 2. メッセージの属性チェック
                        if let Some(attributes) = module.type_attributes.get(&message_type) {
                            for attr in attributes {
                                // #[message(target = "UserActor")] のような属性をチェック
                                if attr.name == "message" && attr.params.contains_key("target") {
                                    if let Some(target) = attr.params.get("target") {
                                        if target == &actor.name {
                                            return Ok(true);
                                        }
                                    }
                                }
                            }
                        }
                        
                        // 3. フィールド分析によるチェック
                        if let Some(fields) = fields {
                            // 宛先フィールドの検索（recipient, target, destinationなど）
                            for field in fields {
                                if ["recipient", "target", "destination", "to"].contains(&field.name.as_str()) {
                                    // フィールドの型がアクター型またはアクターIDと一致するか確認
                                    if field.type_id == actor_id || 
                                       self.is_actor_reference_type(field.type_id, actor_id)? {
                                        return Ok(true);
                                    }
                                }
                            }
                        }
                        
                        // 4. メッセージハンドラの存在チェック
                        let handled_messages = self.get_handled_messages(actor_id)?;
                        if handled_messages.contains(&message_type) {
                            return Ok(true);
                        }
                        
                        // 5. 継承関係のチェック
                        if let Some(parent_types) = module.type_hierarchy.get(&message_type) {
                            for &parent in parent_types {
                                if self.is_message_for_actor(parent, actor_id)? {
                                    return Ok(true);
                                }
                            }
                        }
                    }
                },
                Type::Enum(name, variants) => {
                    // 列挙型メッセージの場合も同様のチェックを行う
                    if let Some(actor) = self.actors.get(&actor_id) {
                        if name.contains(&actor.name) {
                            return Ok(true);
                        }
                        
                        // バリアントごとにチェック
                        if let Some(variants) = variants {
                            for variant in variants {
                                if variant.name.contains(&actor.name) {
                                    return Ok(true);
                                }
                            }
                        }
                        
                        // 属性チェック（構造体と同様）
                        if let Some(attributes) = module.type_attributes.get(&message_type) {
                            for attr in attributes {
                                if attr.name == "message" && attr.params.contains_key("target") {
                                    if let Some(target) = attr.params.get("target") {
                                        if target == &actor.name {
                                            return Ok(true);
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
                _ => {
                    // その他の型（トレイト実装など）のチェック
                    if let Some(implemented_traits) = module.implemented_traits.get(&message_type) {
                        for &trait_id in implemented_traits {
                            // MessageForActorトレイトの実装をチェック
                            if let Some(trait_info) = module.traits.get(&trait_id) {
                                if trait_info.name == "MessageForActor" {
                                    // トレイト関連型またはパラメータでターゲットアクターを指定しているか確認
                                    if let Some(type_params) = &trait_info.type_parameters {
                                        for param in type_params {
                                            if param.name == "Target" && param.type_id == actor_id {
                                                return Ok(true);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // デフォルトはfalse
        Ok(false)
    }
    
    /// 型がアクター参照型かどうかをチェック
    fn is_actor_reference_type(&self, type_id: TypeId, actor_id: TypeId) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // ActorRef<T>のような参照型をチェック
        if let Some(ty) = module.types.get(&type_id) {
            match ty {
                Type::Generic(base_name, params) => {
                    if base_name == "ActorRef" || base_name == "ActorId" || base_name == "Address" {
                        if let Some(params) = params {
                            if params.len() == 1 && params[0] == actor_id {
                                return Ok(true);
                            }
                        }
                    }
                },
                Type::Alias(_, target_type) => {
                    // 型エイリアスの場合、ターゲット型をチェック
                    return self.is_actor_reference_type(*target_type, actor_id);
                },
                _ => {}
            }
        }
        
        Ok(false)
    }
    
    /// デッドロックの可能性をチェック
    /// アクター間のメッセージ送受信パターンを分析し、循環待ちの可能性を検出します
    fn check_deadlock_possibility(&self, start_actor_id: TypeId) -> Result<Option<Vec<String>>, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // アクター間の呼び出しグラフを構築
        let mut call_graph = self.build_actor_call_graph()?;
        let mut visited = HashSet::new();
        let mut path = Vec::new();
        let mut on_path = HashSet::new();
        
        // 循環参照の検出
        if self.detect_actor_call_cycle(start_actor_id, start_actor_id, &call_graph, &mut visited, &mut on_path, &mut path)? {
            // サイクルが検出された場合、アクター名の配列を返す
            let cycle_names = path.iter()
                .filter_map(|&id| self.actors.get(&id).map(|a| a.name.clone()))
                .collect();
            
            // デッドロックの重大度を評価
            let severity = self.evaluate_deadlock_severity(start_actor_id, &path, &call_graph)?;
            
            // 重大度が高い場合のみ報告
            if severity > 0.7 {
                return Ok(Some(cycle_names));
            }
        }
        
        Ok(None)
    }
    
    /// アクター間呼び出しグラフを構築
    fn build_actor_call_graph(&self) -> Result<HashMap<TypeId, Vec<TypeId>>, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        let mut graph = HashMap::new();
        
        // 各アクターのメソッドを分析
        for (&actor_id, actor) in &self.actors {
            let mut calls_to = HashSet::new();
            
            for method in &actor.methods {
                if let Some(function) = module.functions.get(&method.function_id) {
                    // 関数本体からメッセージ送信を検出
                    if let Some(body) = &function.body {
                        self.analyze_message_sends(body, actor_id, &mut calls_to)?;
                    }
                }
            }
            
            graph.insert(actor_id, calls_to.into_iter().collect());
        }
        
        Ok(graph)
    }
    
    /// 関数本体からメッセージ送信を検出
    fn analyze_message_sends(&self, body: &FunctionBody, sender_id: TypeId, calls_to: &mut HashSet<TypeId>) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        match body {
            FunctionBody::Ast(ast) => {
                // ASTを走査してメッセージ送信を検出
                self.analyze_ast_for_message_sends(ast, sender_id, calls_to)?;
            },
            FunctionBody::ExternalDeclaration => {
                // 外部宣言の場合は何もしない
            },
            FunctionBody::Intrinsic(_) => {
                // 組み込み関数の場合は何もしない
            }
        }
        
        Ok(())
    }
    
    /// ASTからメッセージ送信を検出
    fn analyze_ast_for_message_sends(&self, ast: &Ast, sender_id: TypeId, calls_to: &mut HashSet<TypeId>) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        match ast {
            Ast::Block(statements) => {
                for stmt in statements {
                    self.analyze_ast_for_message_sends(stmt, sender_id, calls_to)?;
                }
            },
            Ast::MethodCall(receiver, method_name, args) => {
                // send, tell, ask などのメソッド呼び出しを検出
                if ["send", "tell", "ask", "forward"].contains(&method_name.as_str()) {
                    if let Some(receiver_type) = self.get_expression_type(receiver)? {
                        // レシーバーがアクター参照型かチェック
                        for (&actor_id, _) in &self.actors {
                            if self.is_actor_reference_type(receiver_type, actor_id)? {
                                calls_to.insert(actor_id);
                            }
                        }
                    }
                    
                    // 引数からメッセージ型を取得し、宛先アクターを特定
                    if !args.is_empty() {
                        if let Some(message_type) = self.get_expression_type(&args[0])? {
                            for (&actor_id, _) in &self.actors {
                                if self.is_message_for_actor(message_type, actor_id)? {
                                    calls_to.insert(actor_id);
                                }
                            }
                        }
                    }
                }
                
                // 再帰的に引数も解析
                for arg in args {
                    self.analyze_ast_for_message_sends(arg, sender_id, calls_to)?;
                }
                
                // レシーバーも解析
                self.analyze_ast_for_message_sends(receiver, sender_id, calls_to)?;
            },
            Ast::FunctionCall(func_name, args) => {
                // アクターシステム関数の呼び出しを検出
                if ["spawn", "actorOf", "actorSelection"].contains(&func_name.as_str()) {
                    // 引数からアクター型を特定
                    if !args.is_empty() {
                        if let Some(actor_type) = self.get_expression_type(&args[0])? {
                            for (&actor_id, _) in &self.actors {
                                if actor_type == actor_id {
                                    calls_to.insert(actor_id);
                                }
                            }
                        }
                    }
                }
                
                // 再帰的に引数も解析
                for arg in args {
                    self.analyze_ast_for_message_sends(arg, sender_id, calls_to)?;
                }
            },
            Ast::If(condition, then_branch, else_branch) => {
                self.analyze_ast_for_message_sends(condition, sender_id, calls_to)?;
                self.analyze_ast_for_message_sends(then_branch, sender_id, calls_to)?;
                if let Some(else_stmt) = else_branch {
                    self.analyze_ast_for_message_sends(else_stmt, sender_id, calls_to)?;
                }
            },
            Ast::Loop(body) => {
                self.analyze_ast_for_message_sends(body, sender_id, calls_to)?;
            },
            Ast::While(condition, body) => {
                self.analyze_ast_for_message_sends(condition, sender_id, calls_to)?;
                self.analyze_ast_for_message_sends(body, sender_id, calls_to)?;
            },
            Ast::For(init, condition, update, body) => {
                if let Some(init_stmt) = init {
                    self.analyze_ast_for_message_sends(init_stmt, sender_id, calls_to)?;
                }
                if let Some(cond_expr) = condition {
                    self.analyze_ast_for_message_sends(cond_expr, sender_id, calls_to)?;
                }
                if let Some(update_stmt) = update {
                    self.analyze_ast_for_message_sends(update_stmt, sender_id, calls_to)?;
                }
                self.analyze_ast_for_message_sends(body, sender_id, calls_to)?;
            },
            Ast::Match(expr, arms) => {
                self.analyze_ast_for_message_sends(expr, sender_id, calls_to)?;
                for (pattern, guard, body) in arms {
                    if let Some(guard_expr) = guard {
                        self.analyze_ast_for_message_sends(guard_expr, sender_id, calls_to)?;
                    }
                    self.analyze_ast_for_message_sends(body, sender_id, calls_to)?;
                }
            },
            // その他のASTノードも必要に応じて解析
            _ => {}
        }
        
        Ok(())
    }
    
    /// 式の型を取得
    fn get_expression_type(&self, expr: &Ast) -> Result<Option<TypeId>, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // 型推論システムを使用して式の型を取得
        // 実際の実装では型推論エンジンを呼び出す
        match expr {
            Ast::Variable(name) => {
                // 変数の型を取得
                if let Some(var_info) = module.variables.get(name) {
                    return Ok(Some(var_info.type_id));
                }
            },
            Ast::MethodCall(receiver, method_name, _) => {
                // メソッド呼び出しの戻り値型を取得
                if let Some(receiver_type) = self.get_expression_type(receiver)? {
                    if let Some(methods) = module.type_methods.get(&receiver_type) {
                        for method in methods {
                            if method.name == *method_name {
                                return Ok(Some(method.return_type));
                            }
                        }
                    }
                }
            },
            Ast::FunctionCall(func_name, _) => {
                // 関数呼び出しの戻り値型を取得
                for (func_id, func) in &module.functions {
                    if func.name == *func_name {
                        return Ok(Some(func.return_type));
                    }
                }
            },
            Ast::Literal(value) => {
                // リテラルの型を取得
                match value {
                    Literal::Int(_) => return Ok(Some(module.get_primitive_type_id("i32")?)),
                    Literal::Float(_) => return Ok(Some(module.get_primitive_type_id("f64")?)),
                    Literal::String(_) => return Ok(Some(module.get_primitive_type_id("String")?)),
                    Literal::Bool(_) => return Ok(Some(module.get_primitive_type_id("bool")?)),
                    // その他のリテラル型
                }
            },
            // その他の式タイプ
            _ => {}
        }
        
        Ok(None)
    }
    
    /// アクター間呼び出しの循環を検出（深さ優先探索）
    fn detect_actor_call_cycle(
        &self,
        start: TypeId,
        current: TypeId,
        graph: &HashMap<TypeId, Vec<TypeId>>,
        visited: &mut HashSet<TypeId>,
        on_path: &mut HashSet<TypeId>,
        path: &mut Vec<TypeId>,
    ) -> Result<bool, String> {
        // 既に訪問済みで現在のパス上にない場合はサイクルなし
        if visited.contains(&current) && !on_path.contains(&current) {
            return Ok(false);
        }
        
        // 現在のパス上に既に存在する場合はサイクル検出
        if on_path.contains(&current) {
            // サイクルの開始点を見つける
            let start_index = path.iter().position(|&id| id == current).unwrap_or(0);
            // サイクル部分のみを残す
            path.drain(0..start_index);
            path.push(current);
            return Ok(true);
        }
        
        visited.insert(current);
        on_path.insert(current);
        path.push(current);
        
        // 隣接ノードを探索
        if let Some(neighbors) = graph.get(&current) {
            for &next in neighbors {
                if self.detect_actor_call_cycle(start, next, graph, visited, on_path, path)? {
                    return Ok(true);
                }
            }
        }
        
        // バックトラック
        on_path.remove(&current);
        if path.last() == Some(&current) {
            path.pop();
        }
        
        Ok(false)
    }
    
    /// デッドロックの重大度を評価
    fn evaluate_deadlock_severity(
        &self,
        actor_id: TypeId,
        cycle_path: &Vec<TypeId>,
        call_graph: &HashMap<TypeId, Vec<TypeId>>,
    ) -> Result<f64, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        let mut severity = 0.0;
        
        // 1. サイクルの長さ（短いほど重大）
        let cycle_length = cycle_path.len() as f64;
        severity += 1.0 / cycle_length;
        
        // 2. 同期呼び出しの割合（高いほど重大）
        let mut sync_calls = 0;
        let mut total_calls = 0;
        
        for i in 0..cycle_path.len() {
            let from = cycle_path[i];
            let to = cycle_path[(i + 1) % cycle_path.len()];
            
            // アクター間の呼び出しパターンを分析
            if let Some(actor) = self.actors.get(&from) {
                for method in &actor.methods {
                    if let Some(function) = module.functions.get(&method.function_id) {
                        if let Some(body) = &function.body {
                            // 同期呼び出し（ask）と非同期呼び出し（tell）を区別
                            let (sync, async_calls) = self.count_sync_async_calls(body, to)?;
                            sync_calls += sync;
                            total_calls += sync + async_calls;
                        }
                    }
                }
            }
        }
        if total_calls > 0 {
            severity += 0.5 * (sync_calls as f64 / total_calls as f64);
        }
        
        // 3. リソース競合の可能性（高いほど重大）
        let mut shared_resources = HashSet::new();
        for &actor_id in cycle_path {
            if let Some(actor) = self.actors.get(&actor_id) {
                for field in &actor.state {
                    // 共有リソースを特定（例: データベース接続、ファイルハンドルなど）
                    if self.is_shared_resource_type(field.type_id)? {
                        shared_resources.insert(field.type_id);
                    }
                }
            }
        }
        
        severity += 0.3 * (shared_resources.len() as f64).min(1.0);
        
        // 4. 優先度の高いアクターの関与（高いほど重大）
        let mut critical_actors = 0;
        for &actor_id in cycle_path {
            if let Some(actor) = self.actors.get(&actor_id) {
                if self.is_critical_actor(actor_id)? {
                    critical_actors += 1;
                }
            }
        }
        
        severity += 0.2 * (critical_actors as f64 / cycle_path.len() as f64);
        
        // 正規化（0.0〜1.0の範囲に収める）
        Ok((severity * 10.0).min(10.0) / 10.0)
    }
    
    /// 同期呼び出しと非同期呼び出しの数をカウント
    fn count_sync_async_calls(&self, body: &FunctionBody, target_actor: TypeId) -> Result<(usize, usize), String> {
        let mut sync_calls = 0;
        let mut async_calls = 0;
        
        match body {
            FunctionBody::Ast(ast) => {
                // ASTを走査して同期/非同期呼び出しをカウント
                self.count_calls_in_ast(ast, target_actor, &mut sync_calls, &mut async_calls)?;
            },
            _ => {}
        }
        
        Ok((sync_calls, async_calls))
    }
    
    /// ASTから同期/非同期呼び出しをカウント
    fn count_calls_in_ast(&self, ast: &Ast, target_actor: TypeId, sync_calls: &mut usize, async_calls: &mut usize) -> Result<(), String> {
        match ast {
            Ast::Block(statements) => {
                for stmt in statements {
                    self.count_calls_in_ast(stmt, target_actor, sync_calls, async_calls)?;
                }
            },
            Ast::MethodCall(receiver, method_name, args) => {
                // レシーバーがターゲットアクターへの参照かチェック
                if let Some(receiver_type) = self.get_expression_type(receiver)? {
                    if self.is_actor_reference_type(receiver_type, target_actor)? {
                        // メソッド名で同期/非同期を判断
                        match method_name.as_str() {
                            "ask" | "request" => *sync_calls += 1,
                            "tell" | "send" | "forward" => *async_calls += 1,
                            _ => {}
                        }
                    }
                }
                
                // 再帰的に引数も解析
                for arg in args {
                    self.count_calls_in_ast(arg, target_actor, sync_calls, async_calls)?;
                }
                
                // レシーバーも解析
                self.count_calls_in_ast(receiver, target_actor, sync_calls, async_calls)?;
            },
            Ast::If(condition, then_branch, else_branch) => {
                self.count_calls_in_ast(condition, target_actor, sync_calls, async_calls)?;
                self.count_calls_in_ast(then_branch, target_actor, sync_calls, async_calls)?;
                if let Some(else_stmt) = else_branch {
                    self.count_calls_in_ast(else_stmt, target_actor, sync_calls, async_calls)?;
                }
            },
            // その他のASTノードも必要に応じて解析
            _ => {}
        }
        
        Ok(())
    }
    
    /// 型が共有リソースを表すかどうかをチェック
    fn is_shared_resource_type(&self, type_id: TypeId) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        if let Some(ty) = module.types.get(&type_id) {
            match ty {
                Type::Struct(name, _) => {
                    // 名前に基づく簡易チェック
                    let resource_keywords = [
                        "Database", "Connection", "File", "Socket", "Lock", "Mutex", 
                        "Semaphore", "Resource", "Pool", "Handle", "Client"
                    ];
                    
                    for keyword in resource_keywords {
                        if name.contains(keyword) {
                            return Ok(true);
                        }
                    }
                },
                Type::Generic(base_name, _) => {
                    // 共有リソースを表すジェネリック型
                    let resource_generics = [
                        "Mutex", "RwLock", "Arc", "Rc", "RefCell", "Cell", 
                        "AtomicRef", "SharedRef", "ResourceHandle"
                    ];
                    
                    if resource_generics.contains(&base_name.as_str()) {
                        return Ok(true);
                    }
                },
                _ => {}
            }
            
            // トレイト実装に基づくチェック
            if let Some(implemented_traits) = module.implemented_traits.get(&type_id) {
                for &trait_id in implemented_traits {
                    if let Some(trait_info) = module.traits.get(&trait_id) {
                        if ["SharedResource", "Resource", "Connection", "IO"].contains(&trait_info.name.as_str()) {
                            return Ok(true);
                        }
                    }
                }
            }
        }
        
        Ok(false)
    }
    
    /// アクターが重要かどうかをチェック
    fn is_critical_actor(&self, actor_id: TypeId) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        if let Some(actor) = self.actors.get(&actor_id) {
            // 1. 名前に基づくチェック
            let critical_keywords = [
                "Supervisor", "Manager", "Controller", "Coordinator", "Master",
                "System", "Main", "Root", "Core", "Critical", "Primary"
            ];
            
            for keyword in critical_keywords {
                if actor.name.contains(keyword) {
                    return Ok(true);
                }
            }
            
            // 2. 属性に基づくチェック
            if let Some(attributes) = module.type_attributes.get(&actor_id) {
                for attr in attributes {
                    if attr.name == "critical" || 
                       (attr.name == "priority" && attr.params.get("level").map_or(false, |v| v == "high")) {
                        return Ok(true);
                    }
                }
            }
            
            // 3. 他のアクターからの参照数に基づくチェック
            let mut reference_count = 0;
            for (_, other_actor) in &self.actors {
                for method in &other_actor.methods {
                    if let Some(function) = module.functions.get(&method.function_id) {
                        if let Some(body) = &function.body {
                            if self.references_actor(body, actor_id)? {
                                reference_count += 1;
                            }
                        }
                    }
                }
            }
            
            if reference_count > self.actors.len() / 3 {
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    /// 関数本体が特定のアクターを参照しているかチェック
    fn references_actor(&self, body: &FunctionBody, actor_id: TypeId) -> Result<bool, String> {
        match body {
            FunctionBody::Ast(ast) => {
                self.references_actor_in_ast(ast, actor_id)
            },
            _ => Ok(false)
        }
    }
    
    /// ASTが特定のアクターを参照しているかチェック
    fn references_actor_in_ast(&self, ast: &Ast, actor_id: TypeId) -> Result<bool, String> {
        match ast {
            Ast::Block(statements) => {
                for stmt in statements {
                    if self.references_actor_in_statement(stmt, actor_id)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            },
            Ast::Expression(expr) => self.references_actor_in_expression(expr, actor_id),
            Ast::Condition(condition, then_branch, else_branch) => {
                if self.references_actor_in_expression(condition, actor_id)? {
                    return Ok(true);
                }
                if self.references_actor_in_ast(then_branch, actor_id)? {
                    return Ok(true);
                }
                if let Some(else_ast) = else_branch {
                    if self.references_actor_in_ast(else_ast, actor_id)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            },
            _ => Ok(false),
        }
    }
    
    /// 文が特定のアクターを参照しているかチェック
    fn references_actor_in_statement(&self, stmt: &Statement, actor_id: TypeId) -> Result<bool, String> {
        match stmt {
            Statement::Expression(expr) => self.references_actor_in_expression(expr, actor_id),
            Statement::Declaration(_, expr) => {
                if let Some(init_expr) = expr {
                    self.references_actor_in_expression(init_expr, actor_id)
                } else {
                    Ok(false)
                }
            },
            Statement::Assignment(lhs, rhs) => {
                if self.references_actor_in_expression(lhs, actor_id)? {
                    return Ok(true);
                }
                self.references_actor_in_expression(rhs, actor_id)
            },
            Statement::Block(block) => self.references_actor_in_ast(&Ast::Block(block.clone()), actor_id),
            _ => Ok(false),
        }
    }
    
    /// 式が特定のアクターを参照しているかチェック
    fn references_actor_in_expression(&self, expr: &Expression, actor_id: TypeId) -> Result<bool, String> {
        match expr {
            Expression::TypeReference(type_id) => {
                if *type_id == actor_id {
                    return Ok(true);
                }
                Ok(false)
            },
            Expression::FunctionCall(func, args) => {
                if self.references_actor_in_expression(func, actor_id)? {
                    return Ok(true);
                }
                for arg in args {
                    if self.references_actor_in_expression(arg, actor_id)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            },
            Expression::MethodCall(obj, _, args) => {
                if self.references_actor_in_expression(obj, actor_id)? {
                    return Ok(true);
                }
                for arg in args {
                    if self.references_actor_in_expression(arg, actor_id)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            },
            _ => Ok(false),
        }
    }
}

/// アクターメソッドの呼び出し解析
pub struct ActorMethodCallAnalyzer {
    /// アクターシステム
    actor_system: ActorSystem,
    
    /// モジュール
    module: Option<Module>,
    
    /// メソッド呼び出しグラフ（呼び出し元 -> 呼び出し先）
    call_graph: HashMap<FunctionId, Vec<FunctionId>>,
    
    /// 無効な呼び出し
    invalid_calls: Vec<(FunctionId, FunctionId, String)>,
}

impl ActorMethodCallAnalyzer {
    /// 新しいアクターメソッド呼び出し解析器を作成
    pub fn new(actor_system: ActorSystem) -> Self {
        Self {
            actor_system,
            module: None,
            call_graph: HashMap::new(),
            invalid_calls: Vec::new(),
        }
    }
    
    /// モジュールを設定
    pub fn set_module(&mut self, module: Module) {
        self.module = Some(module);
        self.actor_system.set_module(module.clone());
    }
    
    /// 呼び出し解析を実行
    pub fn analyze(&mut self) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // 各関数の呼び出しグラフを構築
        self.build_call_graph()?;
        
        // アクターメソッドの呼び出しを検証
        self.validate_actor_method_calls()?;
        
        Ok(())
    }
    
    /// 呼び出しグラフを構築
    fn build_call_graph(&mut self) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // 各関数の呼び出しを解析
        for (caller_id, caller) in &module.functions {
            let mut calls = Vec::new();
            
            // 関数内の命令を解析して呼び出しを検出
            for block in &caller.basic_blocks {
                for &inst_id in &block.instructions {
                    if let Some(inst) = caller.instructions.get(&inst_id) {
                        // 関数呼び出し命令を検出
                        if inst.opcode == "call" || inst.opcode == "virtual_call" {
                            if let Some(&callee_id) = inst.operands.get(0) {
                                calls.push(callee_id);
                            }
                        }
                    }
                }
            }
            
            if !calls.is_empty() {
                self.call_graph.insert(*caller_id, calls);
            }
        }
        
        Ok(())
    }
    
    /// アクターメソッドの呼び出しを検証
    fn validate_actor_method_calls(&mut self) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // 各呼び出しが有効かチェック
        for (caller_id, callees) in &self.call_graph {
            for &callee_id in callees {
                // 呼び出し元と呼び出し先のアクターを特定
                let caller_actor = self.get_actor_for_method(*caller_id)?;
                let callee_actor = self.get_actor_for_method(callee_id)?;
                
                // 両方ともアクターメソッドの場合
                if let (Some(caller_actor_id), Some(callee_actor_id)) = (caller_actor, callee_actor) {
                    // 同じアクター内の呼び出しは常に許可
                    if caller_actor_id == callee_actor_id {
                        continue;
                    }
                    
                    // アクター間呼び出しの検証
                    if !self.is_valid_cross_actor_call(*caller_id, callee_id, caller_actor_id, callee_actor_id)? {
                        // 無効な呼び出しを記録
                        self.invalid_calls.push((
                            *caller_id,
                            callee_id,
                            "直接的なアクター間メソッド呼び出しは許可されていません。メッセージパッシングを使用してください。".to_string(),
                        ));
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// メソッドが属するアクターを取得
    fn get_actor_for_method(&self, func_id: FunctionId) -> Result<Option<TypeId>, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        if let Some(function) = module.functions.get(&func_id) {
            // self パラメータの型を取得
            if let Some(self_param) = function.parameters.first() {
                if self_param.name == "self" {
                    let self_type = self_param.type_id;
                    if self.actor_system.actors.contains_key(&self_type) {
                        return Ok(Some(self_type));
                    }
                }
            }
        }
        
        Ok(None)
    }
    
    /// アクター間呼び出しが有効かどうかをチェック
    fn is_valid_cross_actor_call(
        &self,
        caller_id: FunctionId,
        callee_id: FunctionId,
        caller_actor_id: TypeId,
        callee_actor_id: TypeId,
    ) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // 通常、アクター間は直接呼び出しではなくメッセージを使うべき
        // ただし、特定の例外がある
        
        // 1. 親アクターは子アクターのパブリックメソッドを呼び出し可能
        if let Some(children) = self.actor_system.hierarchy.get(&caller_actor_id) {
            if children.contains(&callee_actor_id) {
                // 呼び出されるメソッドがパブリックかチェック
                if let Some(callee_actor) = self.actor_system.actors.get(&callee_actor_id) {
                    for method in &callee_actor.methods {
                        if method.function_id == callee_id && method.protection_level == ProtectionLevel::Public {
                            return Ok(true);
                        }
                    }
                }
            }
        }
        
        // 2. スーパーバイザーは監視対象アクターのメソッドを呼び出し可能
        if let Some(actor) = self.actor_system.actors.get(&caller_actor_id) {
            if actor.supervisees.contains(&callee_actor_id) {
                return Ok(true);
            }
        }
        
        // 3. メッセージ送信は許可
        if self.is_message_send(caller_id, callee_id)? {
            return Ok(true);
        }
        
        // その他の場合は不許可
        Ok(false)
    }
    
    /// 呼び出しがメッセージ送信かどうかをチェック
    fn is_message_send(&self, caller_id: FunctionId, callee_id: FunctionId) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // メッセージ送信関数は特別な名前やシグネチャを持つ
        if let Some(callee) = module.functions.get(&callee_id) {
            return Ok(callee.name == "send" || callee.name == "tell" || callee.name == "ask");
        }
        
        Ok(false)
    }
    
    /// 結果を取得
    pub fn get_results(&self) -> Vec<(FunctionId, FunctionId, String)> {
        self.invalid_calls.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // テストケースは省略
} 
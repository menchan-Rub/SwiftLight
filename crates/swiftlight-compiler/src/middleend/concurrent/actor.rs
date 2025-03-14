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
        
        // アクター型の条件をチェック
        match ty {
            Type::Struct(name, _) => {
                // アクターは特別なトレイトを実装している必要がある
                // ここでは簡略化のため、名前に "Actor" を含む型をアクターとして扱う
                Ok(name.contains("Actor"))
            },
            _ => Ok(false),
        }
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
                // フィールド名はモジュールのシンボル情報から取得する必要がある
                // 簡略化のため、ここではインデックスベースの名前を使用
                let field_name = format!("field_{}", i);
                
                fields.push(ActorField {
                    name: field_name,
                    type_id: field_type_id,
                    is_mutable: true, // デフォルトで可変と仮定
                    protection_level: ProtectionLevel::Private, // デフォルトでプライベート
                    initial_value: None,
                });
            }
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
        
        // スーパーバイザーメソッドを持つか確認
        for (func_id, function) in &module.functions {
            if self.is_method_of_actor(*func_id, actor_type_id)? && 
               self.determine_method_kind(function)? == ActorMethodKind::Supervisor {
                // 監視対象を分析（ここでは簡略化）
                // 実際の実装では、メソッドの本体を解析して参照されるアクター型を特定する
            }
        }
        
        // アクターの属性も確認
        if let Some(attr) = module.attributes.get(&actor_type_id) {
            if let Some(supervised_attr) = attr.get("supervises") {
                // 監視対象アクターの型IDを解析
                // 簡略化のため、ここでは実装を省略
            }
        }
        
        Ok(supervised)
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
            Type::Struct(name, _) => {
                // メッセージは特別なトレイトを実装している必要がある
                // ここでは簡略化のため、名前に "Message" を含む型をメッセージとして扱う
                Ok(name.contains("Message"))
            },
            _ => Ok(false),
        }
    }
    
    /// アクター階層を構築
    fn build_actor_hierarchy(&mut self) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // アクター間の親子関係を分析
        for (actor_id, actor) in &self.actors {
            // 監視関係から親子関係を構築
            for &supervisee in &actor.supervisees {
                self.hierarchy.entry(*actor_id)
                    .or_insert_with(Vec::new)
                    .push(supervisee);
            }
            
            // 他の親子関係（継承や委譲など）も検出
            // 簡略化のため、ここでは省略
        }
        
        Ok(())
    }
    
    /// アクターメソッドの並行処理方式を分析
    fn analyze_concurrency_modes(&mut self) -> Result<(), String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // 各アクターのメソッドを分析
        for (actor_id, actor) in &mut self.actors {
            for method in &mut actor.methods {
                // 関数を取得
                if let Some(function) = module.functions.get(&method.function_id) {
                    // 属性からの明示的な指定
                    if function.attributes.contains_key("async") {
                        method.concurrency_mode = ConcurrencyMode::Async;
                    } else if function.attributes.contains_key("readonly") {
                        method.concurrency_mode = ConcurrencyMode::ReadOnly;
                    } else if function.attributes.contains_key("exclusive") {
                        method.concurrency_mode = ConcurrencyMode::Exclusive;
                    } else if let Some(isolated_attr) = function.attributes.get("isolated") {
                        // 分離フィールドの指定
                        // 簡略化のため、ここでは0を指定
                        method.concurrency_mode = ConcurrencyMode::Isolated(0);
                    }
                    
                    // メソッドがアクセスする状態（フィールド）を分析
                    method.accessed_state = self.analyze_state_access(&method.function_id, actor)?;
                }
            }
        }
        
        Ok(())
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
    fn is_message_for_actor(&self, message_type: TypeId, actor_id: TypeId) -> Result<bool, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // メッセージの属性やフィールドからターゲットアクターを特定
        if let Some(ty) = module.types.get(&message_type) {
            if let Type::Struct(name, _) = ty {
                // 簡略化のため、名前の一致で判断
                if let Some(actor) = self.actors.get(&actor_id) {
                    return Ok(name.contains(&actor.name));
                }
            }
        }
        
        // デフォルトはfalse
        Ok(false)
    }
    
    /// デッドロックの可能性をチェック
    fn check_deadlock_possibility(&self, start_actor_id: TypeId) -> Result<Option<Vec<String>>, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        // アクター間の呼び出しグラフを構築
        let mut call_graph = HashMap::new();
        let mut visited = HashSet::new();
        let mut path = Vec::new();
        
        // 循環参照の検出
        if self.detect_actor_call_cycle(start_actor_id, start_actor_id, &call_graph, &mut visited, &mut path)? {
            // サイクルが検出された場合、アクター名の配列を返す
            let cycle_names = path.iter()
                .filter_map(|&id| self.actors.get(&id).map(|a| a.name.clone()))
                .collect();
            return Ok(Some(cycle_names));
        }
        
        Ok(None)
    }
    
    /// アクター間呼び出しの循環を検出
    fn detect_actor_call_cycle(
        &self,
        start: TypeId,
        current: TypeId,
        graph: &HashMap<TypeId, Vec<TypeId>>,
        visited: &mut HashSet<TypeId>,
        path: &mut Vec<TypeId>,
    ) -> Result<bool, String> {
        // 簡略化のため、常にfalseを返す（循環なし）
        // 実際の実装では、アクター間の呼び出し関係を分析して循環検出を行う
        Ok(false)
    }
    
    /// アクター呼び出しのコードを生成
    pub fn generate_actor_code(&self, actor_id: TypeId) -> Result<String, String> {
        let module = self.module.as_ref().ok_or("モジュールが設定されていません")?;
        
        if let Some(actor) = self.actors.get(&actor_id) {
            // アクターの実装コードを生成
            // 簡略化のため、コード生成の詳細は省略
            let mut code = format!("// Generated code for Actor: {}\n\n", actor.name);
            
            // フィールド宣言
            code.push_str("// State fields\n");
            for field in &actor.state {
                code.push_str(&format!("private var {}: {:?};\n", field.name, field.type_id));
            }
            
            // メソッド実装
            code.push_str("\n// Methods\n");
            for method in &actor.methods {
                code.push_str(&format!("public func {}() {{\n  // TODO: Implementation\n}}\n\n", method.name));
            }
            
            return Ok(code);
        }
        
        Err(format!("アクターID {:?} が見つかりません", actor_id))
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
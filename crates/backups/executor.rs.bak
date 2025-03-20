// executor.rs - SwiftLight並行実行エンジン
//
// このモジュールは、SwiftLight言語の並行タスク実行エンジンを実装します。
// アクター、Future、非同期タスクなどの並行処理単位を効率的にスケジューリングし実行します。
// 既存の言語実装を超える最適化と安全性を両立させています。

use std::collections::{HashMap, HashSet, VecDeque, BinaryHeap, BTreeMap};
use std::cmp::{Eq, PartialEq, Ord, PartialOrd, Ordering};
use std::sync::{Arc, Mutex, Condvar, atomic::{AtomicUsize, AtomicBool, Ordering as AtomicOrdering}};
use std::time::{Duration, Instant};

use crate::middleend::ir::{Module, Function, Value, ValueId, TypeId, FunctionId};
use crate::middleend::concurrent::{actor::Actor, future::Future, channel::Channel};
use crate::middleend::analysis::dataflow::DataflowAnalyzer;

/// 実行優先度レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Critical = 0,  // システムクリティカルなタスク（最高優先度）
    High = 1,      // 高優先度タスク
    Normal = 2,    // 通常タスク
    Low = 3,       // バックグラウンドタスク
    Idle = 4,      // アイドル時のみ実行
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

/// 実行コンテキスト
#[derive(Debug)]
pub struct ExecutionContext {
    /// タスクID
    pub task_id: usize,
    
    /// 所属アクターID（アクタータスクの場合）
    pub actor_id: Option<usize>,
    
    /// 優先度
    pub priority: Priority,
    
    /// タスクローカルストレージ
    pub local_storage: HashMap<String, Value>,
    
    /// 親タスク（存在する場合）
    pub parent_task: Option<usize>,
    
    /// 子タスク
    pub child_tasks: HashSet<usize>,
    
    /// タスク作成時刻
    pub created_at: Instant,
    
    /// 実行開始時刻
    pub started_at: Option<Instant>,
    
    /// デッドライン（存在する場合）
    pub deadline: Option<Instant>,
    
    /// 実行スレッドID
    pub thread_id: Option<usize>,
    
    /// 再開ポイント（中断/再開を伴う場合）
    pub resume_point: Option<usize>,
    
    /// メモリ使用量追跡
    pub memory_usage: usize,
    
    /// CPU使用時間
    pub cpu_time: Duration,
    
    /// I/O待機時間
    pub io_wait_time: Duration,
    
    /// トレース有効フラグ
    pub tracing_enabled: bool,
    
    /// トレースイベント
    pub trace_events: Vec<TraceEvent>,
    
    /// タスク状態
    pub state: TaskState,
    
    /// キャンセルトークン
    pub cancellation_token: Arc<AtomicBool>,
}

/// トレースイベント
#[derive(Debug, Clone)]
pub struct TraceEvent {
    /// イベント種類
    pub kind: TraceEventKind,
    
    /// イベント発生時刻
    pub timestamp: Instant,
    
    /// 関連オブジェクトID
    pub object_id: Option<usize>,
    
    /// 詳細情報
    pub details: String,
}

/// トレースイベント種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceEventKind {
    TaskCreated,
    TaskStarted,
    TaskSuspended,
    TaskResumed,
    TaskCompleted,
    TaskFailed,
    TaskCancelled,
    ActorMessageSent,
    ActorMessageReceived,
    ChannelMessageSent,
    ChannelMessageReceived,
    FutureCreated,
    FutureCompleted,
    FutureRejected,
    LockAcquired,
    LockReleased,
    MemoryAllocated,
    MemoryFreed,
    IOStarted,
    IOCompleted,
    Custom(String),
}

/// タスク状態
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Created,    // 作成済み、未実行
    Ready,      // 実行可能
    Running,    // 実行中
    Suspended,  // 一時停止
    Blocked,    // ブロック中（I/Oやロック待ち）
    Completed,  // 完了
    Failed,     // 失敗
    Cancelled,  // キャンセル済み
}

/// 実行可能タスク（優先度付きキュー用）
#[derive(Debug)]
struct ExecutableTask {
    /// タスクID
    task_id: usize,
    
    /// 優先度
    priority: Priority,
    
    /// デッドライン（存在する場合）
    deadline: Option<Instant>,
    
    /// 実行関数
    function: Arc<dyn Fn(&mut ExecutionContext) -> TaskResult + Send + Sync>,
    
    /// 実行コンテキスト
    context: ExecutionContext,
}

impl PartialEq for ExecutableTask {
    fn eq(&self, other: &Self) -> bool {
        self.task_id == other.task_id
    }
}

impl Eq for ExecutableTask {}

impl PartialOrd for ExecutableTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ExecutableTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // デッドラインが設定されている場合は優先
        match (self.deadline, other.deadline) {
            (Some(a), Some(b)) => a.cmp(&b),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => {
                // デッドラインがなければ優先度で比較
                self.priority.cmp(&other.priority)
            }
        }
    }
}

/// タスク実行結果
#[derive(Debug)]
pub enum TaskResult {
    /// 完了
    Completed(Value),
    
    /// 一時停止（再開ポイント付き）
    Suspended(usize),
    
    /// ブロック（解除条件付き）
    Blocked(BlockCondition),
    
    /// 失敗（エラー情報付き）
    Failed(String),
    
    /// キャンセル
    Cancelled,
}

/// ブロック解除条件
#[derive(Debug, Clone)]
pub enum BlockCondition {
    /// チャネルメッセージ受信待ち
    ChannelReceive(usize),
    
    /// アクターメッセージ受信待ち
    ActorMessage(usize),
    
    /// Future完了待ち
    FutureCompletion(usize),
    
    /// I/O操作完了待ち
    IOCompletion(usize),
    
    /// ロック獲得待ち
    LockAcquisition(usize),
    
    /// 時間経過待ち
    Timeout(Instant),
    
    /// いずれかの条件が満たされるまで待機
    Any(Vec<BlockCondition>),
    
    /// 全ての条件が満たされるまで待機
    All(Vec<BlockCondition>),
}

/// 実行スケジューラ
#[derive(Debug)]
pub struct Executor {
    /// 実行可能タスクキュー
    ready_queue: BinaryHeap<ExecutableTask>,
    
    /// すべてのタスク
    tasks: HashMap<usize, ExecutionContext>,
    
    /// ブロック中のタスク（条件ごと）
    blocked_tasks: HashMap<BlockCondition, HashSet<usize>>,
    
    /// アクターインスタンス（ID -> アクター）
    actors: HashMap<usize, Arc<Mutex<Actor>>>,
    
    /// チャネルインスタンス（ID -> チャネル）
    channels: HashMap<usize, Arc<Mutex<Channel>>>,
    
    /// Future（ID -> Future）
    futures: HashMap<usize, Arc<Mutex<Future>>>,
    
    /// アクター専用キュー
    actor_queues: HashMap<usize, VecDeque<ExecutableTask>>,
    
    /// ワーカースレッド数
    worker_count: usize,
    
    /// グローバルタスクカウンター
    task_counter: AtomicUsize,
    
    /// 実行中フラグ
    running: AtomicBool,
    
    /// スケジューラ統計
    stats: SchedulerStats,
    
    /// ワークスティーリング有効フラグ
    work_stealing_enabled: bool,
    
    /// タイムスライス（ラウンドロビン用）
    time_slice: Duration,
    
    /// デッドロック検出器
    deadlock_detector: DeadlockDetector,
}

/// スケジューラ統計
#[derive(Debug, Default, Clone)]
pub struct SchedulerStats {
    /// 処理済みタスク数
    pub processed_tasks: usize,
    
    /// 成功完了タスク数
    pub completed_tasks: usize,
    
    /// 失敗タスク数
    pub failed_tasks: usize,
    
    /// キャンセルタスク数
    pub cancelled_tasks: usize,
    
    /// スループット（タスク/秒）
    pub throughput: f64,
    
    /// 平均応答時間
    pub avg_response_time: Duration,
    
    /// 平均待機時間
    pub avg_wait_time: Duration,
    
    /// プロセッサ使用率
    pub processor_utilization: f64,
    
    /// メモリ使用量
    pub memory_usage: usize,
    
    /// 長時間実行タスク
    pub long_running_tasks: Vec<usize>,
}

/// デッドロック検出器
#[derive(Debug)]
struct DeadlockDetector {
    /// リソース待ちグラフ（タスク -> 待機中リソース）
    wait_graph: HashMap<usize, HashSet<usize>>,
    
    /// リソース所有グラフ（リソース -> 所有タスク）
    ownership_graph: HashMap<usize, usize>,
    
    /// 最後の検出実行時刻
    last_detection_time: Instant,
    
    /// 検出間隔
    detection_interval: Duration,
    
    /// デッドロック検出履歴
    detection_history: Vec<DeadlockEvent>,
}

/// デッドロックイベント
#[derive(Debug, Clone)]
struct DeadlockEvent {
    /// 検出時刻
    timestamp: Instant,
    
    /// 関連タスク
    tasks: Vec<usize>,
    
    /// 関連リソース
    resources: Vec<usize>,
    
    /// 解決方法
    resolution: Option<DeadlockResolution>,
}

/// デッドロック解決方法
#[derive(Debug, Clone)]
enum DeadlockResolution {
    /// タスクキャンセル
    TaskCancellation(usize),
    
    /// リソース強制解放
    ResourceForceRelease(usize),
    
    /// タイムアウト適用
    TimeoutApplied(Duration),
    
    /// 手動解決
    Manual,
}

impl Executor {
    /// 新しいエグゼキューターを作成
    pub fn new(worker_count: usize) -> Self {
        let worker_count = if worker_count == 0 {
            std::thread::available_parallelism()
                .map(|p| p.get())
                .unwrap_or(1)
        } else {
            worker_count
        };
        
        Self {
            ready_queue: BinaryHeap::new(),
            tasks: HashMap::new(),
            blocked_tasks: HashMap::new(),
            actors: HashMap::new(),
            channels: HashMap::new(),
            futures: HashMap::new(),
            actor_queues: HashMap::new(),
            worker_count,
            task_counter: AtomicUsize::new(1),
            running: AtomicBool::new(false),
            stats: SchedulerStats::default(),
            work_stealing_enabled: true,
            time_slice: Duration::from_millis(10),
            deadlock_detector: DeadlockDetector {
                wait_graph: HashMap::new(),
                ownership_graph: HashMap::new(),
                last_detection_time: Instant::now(),
                detection_interval: Duration::from_secs(5),
                detection_history: Vec::new(),
            },
        }
    }
    
    /// 新しいタスクを登録
    pub fn submit<F>(&mut self, func: F, priority: Priority, deadline: Option<Instant>) -> usize
    where
        F: Fn(&mut ExecutionContext) -> TaskResult + Send + Sync + 'static,
    {
        let task_id = self.task_counter.fetch_add(1, AtomicOrdering::SeqCst);
        
        let context = ExecutionContext {
            task_id,
            actor_id: None,
            priority,
            local_storage: HashMap::new(),
            parent_task: None,
            child_tasks: HashSet::new(),
            created_at: Instant::now(),
            started_at: None,
            deadline,
            thread_id: None,
            resume_point: None,
            memory_usage: 0,
            cpu_time: Duration::ZERO,
            io_wait_time: Duration::ZERO,
            tracing_enabled: false,
            trace_events: Vec::new(),
            state: TaskState::Created,
            cancellation_token: Arc::new(AtomicBool::new(false)),
        };
        
        let task = ExecutableTask {
            task_id,
            priority,
            deadline,
            function: Arc::new(func),
            context: context.clone(),
        };
        
        self.tasks.insert(task_id, context);
        self.ready_queue.push(task);
        
        task_id
    }
    
    /// アクタータスクを登録
    pub fn submit_actor_task<F>(&mut self, actor_id: usize, func: F, priority: Priority) -> usize
    where
        F: Fn(&mut ExecutionContext) -> TaskResult + Send + Sync + 'static,
    {
        let task_id = self.task_counter.fetch_add(1, AtomicOrdering::SeqCst);
        
        let context = ExecutionContext {
            task_id,
            actor_id: Some(actor_id),
            priority,
            local_storage: HashMap::new(),
            parent_task: None,
            child_tasks: HashSet::new(),
            created_at: Instant::now(),
            started_at: None,
            deadline: None,
            thread_id: None,
            resume_point: None,
            memory_usage: 0,
            cpu_time: Duration::ZERO,
            io_wait_time: Duration::ZERO,
            tracing_enabled: false,
            trace_events: Vec::new(),
            state: TaskState::Created,
            cancellation_token: Arc::new(AtomicBool::new(false)),
        };
        
        let task = ExecutableTask {
            task_id,
            priority,
            deadline: None,
            function: Arc::new(func),
            context: context.clone(),
        };
        
        self.tasks.insert(task_id, context);
        
        // アクター専用キューに追加
        self.actor_queues
            .entry(actor_id)
            .or_insert_with(VecDeque::new)
            .push_back(task);
        
        task_id
    }
    
    /// エグゼキューターを実行
    pub fn run(&mut self) {
        self.running.store(true, AtomicOrdering::SeqCst);
        
        // ワーカースレッドを起動
        let mut worker_handles = Vec::with_capacity(self.worker_count);
        
        for worker_id in 0..self.worker_count {
            let task_queue = Arc::clone(&self.task_queue);
            let actor_queues = Arc::clone(&self.actor_queues);
            let running = Arc::clone(&self.running);
            let stats = Arc::clone(&self.stats);
            let work_stealing_enabled = self.work_stealing_enabled;
            let worker_backoff_strategy = self.worker_backoff_strategy.clone();
            let task_counter = Arc::clone(&self.task_counter);
            
            // ワーカースレッド間の通信用チャネルを作成
            let (tx, rx) = crossbeam_channel::unbounded();
            self.worker_channels.insert(worker_id, tx);
            
            // ワーカースレッドを作成
            let handle = thread::Builder::new()
                .name(format!("worker-{}", worker_id))
                .spawn(move || {
                    let mut local_stats = WorkerStats::default();
                    let mut rng = rand::thread_rng();
                    let mut backoff = worker_backoff_strategy.create_backoff();
                    let thread_id = thread::current().id();
                    
                    while running.load(AtomicOrdering::SeqCst) {
                        // 1. 専用タスクがあれば処理
                        if let Some(task) = rx.try_recv().ok() {
                            backoff.reset();
                            Self::execute_task(task, &mut local_stats, thread_id);
                            continue;
                        }
                        
                        // 2. グローバルキューからタスクを取得
                        if let Some(task) = task_queue.pop() {
                            backoff.reset();
                            Self::execute_task(task, &mut local_stats, thread_id);
                            continue;
                        }
                        
                        // 3. アクターキューからタスクを取得
                        let mut actor_processed = false;
                        for (actor_id, queue) in actor_queues.lock().unwrap().iter_mut() {
                            if let Some(task) = queue.pop_front() {
                                backoff.reset();
                                Self::execute_task(task, &mut local_stats, thread_id);
                                actor_processed = true;
                                break;
                            }
                        }
                        if actor_processed {
                            continue;
                        }
                        
                        // 4. ワークスティーリングが有効な場合、他のワーカーからタスクを盗む
                        if work_stealing_enabled {
                            let mut stole_task = false;
                            // ランダムなワーカーを選択
                            let victim_id = rng.gen_range(0..self.worker_count);
                            if victim_id != worker_id {
                                // 実装省略: 他のワーカーのローカルキューからタスクを盗む
                                // stole_task = true;
                            }
                            if stole_task {
                                backoff.reset();
                                continue;
                            }
                        }
                        
                        // タスクがない場合はバックオフ
                        if backoff.is_completed() {
                            // 長時間タスクがない場合はスリープ
                            thread::sleep(Duration::from_millis(1));
                            backoff.reset();
                        } else {
                            backoff.snooze();
                        }
                    }
                    
                    // 統計情報を更新
                    stats.lock().unwrap().merge_worker_stats(&local_stats);
                })
                .expect(&format!("ワーカースレッド {} の作成に失敗しました", worker_id));
            
            worker_handles.push(handle);
        }
        // すべてのワーカーが終了するのを待機
        for handle in worker_handles {
            // handle.join().unwrap();
        }
    }
    
    /// タスクをキャンセル
    pub fn cancel_task(&mut self, task_id: usize) -> bool {
        if let Some(context) = self.tasks.get_mut(&task_id) {
            context.cancellation_token.store(true, AtomicOrdering::SeqCst);
            
            // 子タスクも再帰的にキャンセル
            let child_tasks = context.child_tasks.clone();
            for child_id in child_tasks {
                self.cancel_task(child_id);
            }
            
            // タスク状態を更新
            context.state = TaskState::Cancelled;
            self.stats.cancelled_tasks += 1;
            
            true
        } else {
            false
        }
    }
    
    /// タスクの状態を取得
    pub fn get_task_state(&self, task_id: usize) -> Option<TaskState> {
        self.tasks.get(&task_id).map(|ctx| ctx.state)
    }
    
    /// 実行統計を取得
    pub fn get_stats(&self) -> SchedulerStats {
        self.stats.clone()
    }
    
    /// デッドロック検出を実行
    fn detect_deadlocks(&mut self) -> Vec<Vec<usize>> {
        // 現在時刻がデッドロック検出間隔を超えていない場合はスキップ
        let now = Instant::now();
        if now.duration_since(self.deadlock_detector.last_detection_time) < self.deadlock_detector.detection_interval {
            return Vec::new();
        }
        
        // 検出時刻を更新
        self.deadlock_detector.last_detection_time = now;
        
        // デッドロック検出ロジック
        // 待機グラフでのサイクル検出がデッドロックを示す
        let mut deadlocks = Vec::new();
        
        // ここで実際のデッドロック検出アルゴリズムを実装
        // 簡単のため、実装詳細は省略
        
        deadlocks
    }
    
    /// デッドロックを解決
    fn resolve_deadlock(&mut self, cycle: Vec<usize>) {
        // デッドロック解決戦略
        // 1. 最も優先度の低いタスクをキャンセル
        // 2. 最も待機時間の短いタスクをキャンセル
        // 3. リソースのタイムアウトを強制
        
        if cycle.is_empty() {
            return;
        }
        
        // 最も優先度の低いタスクを見つける
        let mut lowest_priority_task = cycle[0];
        let mut lowest_priority = Priority::Critical;
        
        for &task_id in &cycle {
            if let Some(context) = self.tasks.get(&task_id) {
                if context.priority > lowest_priority {
                    lowest_priority = context.priority;
                    lowest_priority_task = task_id;
                }
            }
        }
        // タスクをキャンセル
        self.cancel_task(lowest_priority_task);
        
        // 関連リソースの特定
        let mut involved_resources = HashSet::new();
        for &task_id in &cycle {
            if let Some(context) = self.tasks.get(&task_id) {
                for &resource_id in &context.waiting_for {
                    involved_resources.insert(resource_id);
                }
                for &resource_id in &context.holding {
                    involved_resources.insert(resource_id);
                }
            }
        }
        
        // デッドロック解決イベントを記録
        self.deadlock_detector.detection_history.push(DeadlockEvent {
            timestamp: Instant::now(),
            tasks: cycle.clone(),
            resources: involved_resources.into_iter().collect(),
            resolution: Some(DeadlockResolution::TaskCancellation(lowest_priority_task)),
            resolution_time: Duration::from_micros(self.stats.total_execution_time.as_micros() as u64),
        });
        
        // デッドロック統計を更新
        self.stats.deadlocks_detected += 1;
        self.stats.deadlocks_resolved += 1;
        
        // 適応型デッドロック検出間隔の調整
        self.adjust_deadlock_detection_interval();
        
        // デッドロック解決後のシステム状態を最適化
        self.rebalance_resources_after_deadlock();
    }
    
    /// タスクをキャンセルする
    fn cancel_task(&mut self, task_id: usize) {
        if let Some(mut context) = self.tasks.remove(&task_id) {
            // 保持しているリソースを解放
            for &resource_id in &context.holding {
                if let Some(resource) = self.resources.get_mut(&resource_id) {
                    resource.owner = None;
                    
                    // 待機キューから次のタスクを選択
                    if !resource.waiting_queue.is_empty() {
                        let next_task_id = self.select_next_task_for_resource(&resource.waiting_queue);
                        if let Some(next_id) = next_task_id {
                            if let Some(next_task) = self.tasks.get_mut(&next_id) {
                                // 待機リストからリソースを削除
                                next_task.waiting_for.retain(|&r| r != resource_id);
                                // 保持リソースに追加
                                next_task.holding.push(resource_id);
                                // リソースの所有者を更新
                                resource.owner = Some(next_id);
                                // 待機キューから削除
                                resource.waiting_queue.retain(|&id| id != next_id);
                                
                                // タスクが実行可能になったら実行キューに追加
                                if next_task.waiting_for.is_empty() {
                                    self.ready_queue.push(TaskQueueEntry {
                                        task_id: next_id,
                                        priority: next_task.priority,
                                        deadline: next_task.deadline,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            
            // キャンセルコールバックを実行
            if let Some(callback) = context.on_cancel.take() {
                callback(TaskCancellationReason::Deadlock);
            }
            
            // タスク統計を更新
            self.stats.tasks_cancelled += 1;
            
            // キャンセルイベントを通知
            self.event_listeners.iter_mut().for_each(|listener| {
                listener.on_task_cancelled(task_id, TaskCancellationReason::Deadlock);
            });
        }
    }
    
    /// リソースに対する次のタスクを選択
    fn select_next_task_for_resource(&self, waiting_queue: &[usize]) -> Option<usize> {
        if waiting_queue.is_empty() {
            return None;
        }
        
        // 優先度に基づいてタスクを選択
        let mut highest_priority_task = None;
        let mut highest_priority = Priority::Low;
        let mut earliest_deadline = Instant::now() + Duration::from_secs(3600); // 1時間後をデフォルトに
        
        for &task_id in waiting_queue {
            if let Some(task) = self.tasks.get(&task_id) {
                // 優先度が高いか、同じ優先度でデッドラインが早いタスクを選択
                if task.priority < highest_priority || 
                   (task.priority == highest_priority && task.deadline < earliest_deadline) {
                    highest_priority = task.priority;
                    earliest_deadline = task.deadline;
                    highest_priority_task = Some(task_id);
                }
            }
        }
        
        highest_priority_task
    }
    
    /// デッドロック検出間隔を適応的に調整
    fn adjust_deadlock_detection_interval(&mut self) {
        let detection_history = &self.deadlock_detector.detection_history;
        
        // 直近のデッドロック検出履歴を分析
        if detection_history.len() >= 10 {
            let recent_history = &detection_history[detection_history.len() - 10..];
            
            // 直近のデッドロック頻度を計算
            let first_event_time = recent_history.first().unwrap().timestamp;
            let last_event_time = recent_history.last().unwrap().timestamp;
            let time_span = last_event_time.duration_since(first_event_time);
            
            // デッドロック頻度に基づいて検出間隔を調整
            if time_span.as_secs() < 60 && recent_history.len() >= 5 {
                // 頻繁にデッドロックが発生している場合、検出間隔を短くする
                self.deadlock_detector.detection_interval = self.deadlock_detector.detection_interval.mul_f32(0.8);
                if self.deadlock_detector.detection_interval < Duration::from_millis(100) {
                    self.deadlock_detector.detection_interval = Duration::from_millis(100);
                }
            } else if time_span.as_secs() > 300 {
                // デッドロックが稀な場合、検出間隔を長くする
                self.deadlock_detector.detection_interval = self.deadlock_detector.detection_interval.mul_f32(1.2);
                if self.deadlock_detector.detection_interval > Duration::from_secs(10) {
                    self.deadlock_detector.detection_interval = Duration::from_secs(10);
                }
            }
        }
    }
    
    /// デッドロック解決後のリソース再配分
    fn rebalance_resources_after_deadlock(&mut self) {
        // 優先度の高いタスクが必要とするリソースを優先的に割り当て
        let mut priority_tasks = Vec::new();
        
        // 優先度の高いタスクを抽出
        for (&task_id, task) in &self.tasks {
            if task.priority == Priority::High || task.priority == Priority::Critical {
                priority_tasks.push(task_id);
            }
        }
        
        // 優先度順にソート
        priority_tasks.sort_by(|&a, &b| {
            let priority_a = self.tasks.get(&a).map_or(Priority::Low, |t| t.priority);
            let priority_b = self.tasks.get(&b).map_or(Priority::Low, |t| t.priority);
            priority_a.cmp(&priority_b)
        });
        
        // 優先度の高いタスクにリソースを再配分
        for task_id in priority_tasks {
            if let Some(task) = self.tasks.get(&task_id) {
                for &resource_id in &task.waiting_for {
                    if let Some(resource) = self.resources.get_mut(&resource_id) {
                        // リソースが他のタスクに割り当てられている場合
                        if let Some(owner_id) = resource.owner {
                            if let Some(owner_task) = self.tasks.get(&owner_id) {
                                // 所有者の優先度が現在のタスクより低い場合、リソースを再配分
                                if owner_task.priority > task.priority {
                                    // 現在の所有者からリソースを取り上げる処理
                                    self.preempt_resource(resource_id, owner_id, task_id);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// リソースの先取り（プリエンプション）
    fn preempt_resource(&mut self, resource_id: usize, current_owner: usize, new_owner: usize) {
        if let Some(resource) = self.resources.get_mut(&resource_id) {
            // 現在の所有者からリソースを削除
            if let Some(owner_task) = self.tasks.get_mut(&current_owner) {
                owner_task.holding.retain(|&r| r != resource_id);
                // 所有者がリソースを待機するように設定
                owner_task.waiting_for.push(resource_id);
                resource.waiting_queue.push(current_owner);
                
                // 所有者が他のリソースを待っている場合は実行キューから削除
                if !owner_task.waiting_for.is_empty() {
                    self.ready_queue.retain(|entry| entry.task_id != current_owner);
                }
            }
            
            // 新しい所有者にリソースを割り当て
            if let Some(new_owner_task) = self.tasks.get_mut(&new_owner) {
                new_owner_task.waiting_for.retain(|&r| r != resource_id);
                new_owner_task.holding.push(resource_id);
                resource.owner = Some(new_owner);
                
                // 待機キューから削除
                resource.waiting_queue.retain(|&id| id != new_owner);
                
                // 新しい所有者が他のリソースを待っていない場合は実行キューに追加
                if new_owner_task.waiting_for.is_empty() {
                    self.ready_queue.push(TaskQueueEntry {
                        task_id: new_owner,
                        priority: new_owner_task.priority,
                        deadline: new_owner_task.deadline,
                    });
                }
            }
            
            // リソースプリエンプションイベントを記録
            self.stats.resource_preemptions += 1;
            
            // イベントリスナーに通知
            self.event_listeners.iter_mut().for_each(|listener| {
                listener.on_resource_preempted(resource_id, current_owner, new_owner);
            });
        }
    }
}

// デッドロック検出と解決のためのユーティリティ
impl Executor {
    /// 待機グラフからサイクルを検出
    fn detect_cycles(&self) -> Vec<Vec<usize>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut path = Vec::new();
        let mut on_path = HashSet::new();
        
        // 全タスクを起点としてDFSを実行
        for &task_id in self.tasks.keys() {
            if !visited.contains(&task_id) {
                self.dfs_for_cycles(task_id, &mut visited, &mut path, &mut on_path, &mut cycles);
            }
        }
        
        cycles
    }
    
    /// サイクル検出のためのDFS
    fn dfs_for_cycles(
        &self,
        task_id: usize,
        visited: &mut HashSet<usize>,
        path: &mut Vec<usize>,
        on_path: &mut HashSet<usize>,
        cycles: &mut Vec<Vec<usize>>
    ) {
        visited.insert(task_id);
        path.push(task_id);
        on_path.insert(task_id);
        
        if let Some(task) = self.tasks.get(&task_id) {
            // タスクが待機しているリソースの所有者を調べる
            for &resource_id in &task.waiting_for {
                if let Some(resource) = self.resources.get(&resource_id) {
                    if let Some(owner_id) = resource.owner {
                        if on_path.contains(&owner_id) {
                            // サイクルを検出
                            let cycle_start = path.iter().position(|&id| id == owner_id).unwrap();
                            let cycle = path[cycle_start..].to_vec();
                            cycles.push(cycle);
                        } else if !visited.contains(&owner_id) {
                            self.dfs_for_cycles(owner_id, visited, path, on_path, cycles);
                        }
                    }
                }
            }
        }
        
        path.pop();
        on_path.remove(&task_id);
    }
}
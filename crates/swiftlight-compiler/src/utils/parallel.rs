// 並列処理を行うモジュール
// スレッドプール、タスクキュー、ワークスティーリングなどを提供します

use std::sync::{Arc, Mutex, Condvar, atomic::{AtomicUsize, Ordering}};
use std::thread::{self, JoinHandle};
use std::collections::{VecDeque, BinaryHeap};
use std::time::{Duration, Instant};
use num_cpus;
use anyhow::{Result, Context};

/// タスクの優先度（コンパイラ最適化のためrepr指定）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum TaskPriority {
    /// 低優先度 (バックグラウンド処理)
    Low = 0,
    /// 標準優先度 (通常コンパイル)
    Normal = 1,
    /// 高優先度 (UI応答処理)
    High = 2,
    /// 最高優先度 (クリティカルパス最適化)
    Critical = 3,
}

/// タスクトレイト（コンパイル時計算向け最適化）
pub trait Task: Send + Sync + Ord {
    /// タスクの名前
    fn name(&self) -> &str;
    
    /// タスクの優先度（低いほど優先）
    fn priority(&self) -> u32;
    
    /// タスクを実行
    fn execute(&self) -> Result<(), Box<dyn std::error::Error>>;
}

// Taskトレイトのデフォルト実装
impl<T: Task> PartialEq for T {
    fn eq(&self, other: &Self) -> bool {
        self.priority() == other.priority()
    }
}

impl<T: Task> Eq for T {}

impl<T: Task> PartialOrd for T {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Task> Ord for T {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // 優先度が低いほど先に実行される（BinaryHeapは最大値を取り出すため）
        other.priority().cmp(&self.priority())
    }
}
    
    /// タスク名（デバッグ用）
    fn name(&self) -> &'static str {
        "unnamed_task"
    }
    
    /// タイムアウト時間（デフォルト30秒）
    fn timeout(&self) -> Duration {
        Duration::from_secs(30)
    }

/// ワークスティーリング対応キュー
struct WorkStealingQueue<T> {
    queues: Vec<Mutex<BinaryHeap<TaskWrapper>>>,
    cond_var: Arc<Condvar>,
    steal_idx: AtomicUsize,
}

impl<T: Task> WorkStealingQueue<T> {
    fn new(num_workers: usize) -> Self {
        let mut queues = Vec::with_capacity(num_workers);
        for _ in 0..num_workers {
            queues.push(Mutex::new(BinaryHeap::new()));
        }
        
        Self {
            queues,
            cond_var: Arc::new(Condvar::new()),
            steal_idx: AtomicUsize::new(0),
        }
    }

    /// タスクを最適なキューに挿入
    fn push(&self, task: Arc<dyn Task>) {
        let mut min_load = usize::MAX;
        let mut target_idx = 0;
        
        // 負荷が最小のキューを選択
        for (i, queue) in self.queues.iter().enumerate() {
            let q = queue.lock().unwrap();
            if q.len() < min_load {
                min_load = q.len();
                target_idx = i;
            }
        }
        
        self.queues[target_idx].lock().unwrap().push(task);
        self.cond_var.notify_one();
    }

    /// 優先度付き挿入（O(log n)）
    fn push_with_priority(&self, task: Arc<dyn Task>) {
        let target_idx = self.steal_idx.fetch_add(1, Ordering::Relaxed) % self.queues.len();
        let mut queue = self.queues[target_idx].lock().unwrap();
        queue.push(TaskWrapper(task));
        self.cond_var.notify_all();
    }

    /// タスク取得（ワークスティーリング対応）
    fn pop(&self, worker_id: usize) -> Option<Arc<dyn Task>> {
        // ローカルキューから取得
        if let Some(task) = self.queues[worker_id].lock().unwrap().pop() {
            return Some(task);
        }
        
        // ワークスティーリング
        for i in 0..self.queues.len() {
            let idx = (worker_id + i) % self.queues.len();
            if idx == worker_id { continue; }
            
            if let Ok(mut queue) = self.queues[idx].try_lock() {
                if let Some(task) = queue.pop() {
                    return Some(task);
                }
            }
        }
        
        None
    }
}

/// 並列実行エンジン
pub struct ParallelEngine {
    workers: Vec<JoinHandle<()>>,
    queue: Arc<WorkStealingQueue<dyn Task>>,
    shutdown: Arc<Mutex<bool>>,
    stats: Arc<StatsCollector>,
}

#[derive(Default)]
struct StatsCollector {
    total_tasks: AtomicUsize,
    stolen_tasks: AtomicUsize,
    timeouts: AtomicUsize,
}

impl ParallelEngine {
    /// 自動スレッド数で初期化
    pub fn new() -> Self {
        let num_workers = num_cpus::get().max(1);
        Self::with_workers(num_workers)
    }
    
    /// 指定スレッド数で初期化
    pub fn with_workers(num_workers: usize) -> Self {
        let queue = Arc::new(WorkStealingQueue::new(num_workers));
        let shutdown = Arc::new(Mutex::new(false));
        let stats = Arc::new(StatsCollector::default());
        
        let mut workers = Vec::with_capacity(num_workers);
        for id in 0..num_workers {
            let queue = queue.clone();
            let shutdown = shutdown.clone();
            let stats = stats.clone();
            
            workers.push(thread::Builder::new()
                .name(format!("CompilerWorker-{}", id))
                .spawn(move || Self::worker_loop(id, queue, shutdown, stats))
                .expect("Failed to spawn worker thread"));
        }
        
        Self { workers, queue, shutdown, stats }
    }
    
    /// ワーカー処理ループ
    fn worker_loop(
        id: usize,
        queue: Arc<WorkStealingQueue<dyn Task>>,
        shutdown: Arc<Mutex<bool>>,
        stats: Arc<StatsCollector>
    ) {
        while !*shutdown.lock().unwrap() {
            let task = match queue.pop(id) {
                Some(t) => t,
                None => {
                    let guard = queue.cond_var.wait(queue.queues[id].lock().unwrap()).unwrap();
                    if let Some(t) = guard.pop() {
                        t
                    } else {
                        continue;
                    }
                }
            };
            
            let start_time = Instant::now();
            let timeout = task.timeout();
            
            let result = std::panic::catch_unwind(|| {
                let _timer = stats.start_profiling(task.name());
                task.execute()
            });
            
            match result {
                Ok(Ok(_)) => stats.record_success(start_time.elapsed()),
                Ok(Err(e)) => log::error!("Task failed: {:?}", e),
                Err(_) => stats.record_panic(),
            }
            
            if start_time.elapsed() > timeout {
                stats.record_timeout();
            }
        }
    }
    
    /// タスク実行（自動負荷分散）
    pub fn execute(&self, task: impl Task) {
        self.queue.push(Arc::new(task));
    }
    
    /// 優先タスク実行（クリティカルパス用）
    pub fn execute_critical(&self, task: impl Task) {
        self.queue.push_with_priority(Arc::new(task));
    }
    
    /// グレースフルシャットダウン
    pub fn shutdown(&self) {
        *self.shutdown.lock().unwrap() = true;
        self.queue.cond_var.notify_all();
    }
}

impl Drop for ParallelEngine {
    fn drop(&mut self) {
        self.shutdown();
        for worker in self.workers.drain(..) {
            worker.join().unwrap();
        }
    }
}

// プロファイリング＆メトリクス収集機能
impl StatsCollector {
    fn start_profiling(&self, name: &str) -> ProfilingTimer {
        ProfilingTimer::new(name.to_string())
    }
    
    fn record_success(&self, duration: Duration) {
        // メトリクス記録ロジック
    }
    
    fn record_panic(&self) {
        self.total_tasks.fetch_add(1, Ordering::Relaxed);
    }
    
    fn record_timeout(&self) {
        self.timeouts.fetch_add(1, Ordering::Relaxed);
    }
}

struct ProfilingTimer {
    name: String,
    start: Instant,
}

impl ProfilingTimer {
    fn new(name: String) -> Self {
        Self {
            name,
            start: Instant::now(),
        }
    }
}

impl Drop for ProfilingTimer {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        // プロファイルデータ記録
    }
}
// ビルドプラン管理を担当するモジュール
// コンパイル手順の計画、実行、管理を行います

use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};
use crate::frontend::error::{Result, CompilerError, ErrorKind};

/// ビルドタスクの種類
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskKind {
    /// ソースファイルの読み込み
    ReadSource,
    /// 字句解析
    Lexing,
    /// 構文解析
    Parsing,
    /// 意味解析
    SemanticAnalysis,
    /// IR生成
    IRGeneration,
    /// 最適化
    Optimization,
    /// コード生成
    CodeGeneration,
    /// リンク
    Linking,
    /// デバッグ情報生成
    DebugInfoGeneration,
    /// カスタムタスク
    Custom(String),
}

/// ビルドタスクの状態
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskState {
    /// 待機中
    Pending,
    /// 実行中
    Running,
    /// 完了
    Completed,
    /// スキップ
    Skipped,
    /// 失敗
    Failed,
}

/// ビルドタスク
#[derive(Debug, Clone)]
pub struct BuildTask {
    /// タスクID
    pub id: usize,
    /// タスクの種類
    pub kind: TaskKind,
    /// タスクの説明
    pub description: String,
    /// 入力ファイル
    pub input_files: Vec<PathBuf>,
    /// 出力ファイル
    pub output_files: Vec<PathBuf>,
    /// 依存タスク
    pub dependencies: HashSet<usize>,
    /// タスクの状態
    pub state: TaskState,
    /// 開始時間
    pub start_time: Option<Instant>,
    /// 終了時間
    pub end_time: Option<Instant>,
    /// タスク実行時間
    pub duration: Option<Duration>,
    /// タスクの優先度
    pub priority: usize,
    /// タスクのメタデータ
    pub metadata: HashMap<String, String>,
}

impl BuildTask {
    /// 新しいビルドタスクを作成
    pub fn new(id: usize, kind: TaskKind, description: String) -> Self {
        Self {
            id,
            kind,
            description,
            input_files: Vec::new(),
            output_files: Vec::new(),
            dependencies: HashSet::new(),
            state: TaskState::Pending,
            start_time: None,
            end_time: None,
            duration: None,
            priority: 0,
            metadata: HashMap::new(),
        }
    }

    /// 入力ファイルを追加
    pub fn add_input_file<P: AsRef<Path>>(&mut self, path: P) {
        self.input_files.push(path.as_ref().to_path_buf());
    }

    /// 出力ファイルを追加
    pub fn add_output_file<P: AsRef<Path>>(&mut self, path: P) {
        self.output_files.push(path.as_ref().to_path_buf());
    }

    /// 依存タスクを追加
    pub fn add_dependency(&mut self, task_id: usize) {
        self.dependencies.insert(task_id);
    }

    /// タスクの状態を設定
    pub fn set_state(&mut self, state: TaskState) {
        self.state = state;
    }

    /// タスクの実行を開始
    pub fn start(&mut self) {
        self.state = TaskState::Running;
        self.start_time = Some(Instant::now());
    }

    /// タスクの実行を終了
    pub fn complete(&mut self, success: bool) {
        self.state = if success { TaskState::Completed } else { TaskState::Failed };
        self.end_time = Some(Instant::now());
        if let Some(start) = self.start_time {
            self.duration = Some(self.end_time.unwrap().duration_since(start));
        }
    }

    /// タスクをスキップ
    pub fn skip(&mut self) {
        self.state = TaskState::Skipped;
    }

    /// タスクの実行時間を取得
    pub fn get_duration(&self) -> Option<Duration> {
        self.duration
    }

    /// メタデータを設定
    pub fn set_metadata(&mut self, key: &str, value: &str) {
        self.metadata.insert(key.to_string(), value.to_string());
    }

    /// タスクの優先度を設定
    pub fn set_priority(&mut self, priority: usize) {
        self.priority = priority;
    }
}

/// ビルドプラン
#[derive(Debug, Default)]
pub struct BuildPlan {
    /// ビルドタスク
    tasks: HashMap<usize, BuildTask>,
    /// 次のタスクID
    next_task_id: usize,
    /// ルートタスク
    root_tasks: HashSet<usize>,
    /// 並列実行レベル（同時実行可能なタスク数）
    parallelism: usize,
    /// タスク間の依存関係グラフ（タスクID -> 依存されるタスクID）
    dependents: HashMap<usize, HashSet<usize>>,
}

impl BuildPlan {
    /// 新しいビルドプランを作成
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            next_task_id: 0,
            root_tasks: HashSet::new(),
            parallelism: num_cpus::get(),
            dependents: HashMap::new(),
        }
    }

    /// タスクを追加
    pub fn add_task(&mut self, kind: TaskKind, description: &str) -> usize {
        let id = self.next_task_id;
        let task = BuildTask::new(id, kind, description.to_string());
        
        self.tasks.insert(id, task);
        self.root_tasks.insert(id);
        self.next_task_id += 1;
        
        id
    }

    /// タスクを取得
    pub fn get_task(&self, id: usize) -> Option<&BuildTask> {
        self.tasks.get(&id)
    }

    /// タスクを可変で取得
    pub fn get_task_mut(&mut self, id: usize) -> Option<&mut BuildTask> {
        self.tasks.get_mut(&id)
    }

    /// 依存関係を追加
    pub fn add_dependency(&mut self, task_id: usize, depends_on: usize) -> Result<()> {
        if !self.tasks.contains_key(&task_id) || !self.tasks.contains_key(&depends_on) {
            return Err(CompilerError::new(
                ErrorKind::Internal,
                format!("無効なタスクID: {} または {}", task_id, depends_on),
                None
            ));
        }
        
        // 循環依存のチェック
        if self.would_create_cycle(task_id, depends_on) {
            return Err(CompilerError::new(
                ErrorKind::CircularDependency,
                format!("タスク間に循環依存関係が発生します: {} -> {}", task_id, depends_on),
                None
            ));
        }
        
        // タスクに依存関係を追加
        if let Some(task) = self.tasks.get_mut(&task_id) {
            task.add_dependency(depends_on);
        }
        
        // 依存グラフを更新
        self.dependents
            .entry(depends_on)
            .or_insert_with(HashSet::new)
            .insert(task_id);
        
        // ルートタスクから削除（他のタスクに依存するため）
        self.root_tasks.remove(&task_id);
        
        Ok(())
    }

    /// 並列実行レベルを設定
    pub fn set_parallelism(&mut self, level: usize) {
        self.parallelism = level;
    }

    /// 実行可能なタスクを取得
    pub fn get_executable_tasks(&self) -> Vec<usize> {
        let mut result = Vec::new();
        
        for (id, task) in &self.tasks {
            if task.state == TaskState::Pending && self.are_dependencies_completed(*id) {
                result.push(*id);
            }
        }
        
        // 優先度に基づいて並べ替え
        result.sort_by(|a, b| {
            let a_priority = self.tasks[a].priority;
            let b_priority = self.tasks[b].priority;
            b_priority.cmp(&a_priority) // 降順（高い優先度が先）
        });
        
        result
    }

    /// 依存関係が全て完了しているかチェック
    fn are_dependencies_completed(&self, task_id: usize) -> bool {
        if let Some(task) = self.tasks.get(&task_id) {
            for dep_id in &task.dependencies {
                if let Some(dep_task) = self.tasks.get(dep_id) {
                    if dep_task.state != TaskState::Completed && dep_task.state != TaskState::Skipped {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    /// 循環依存関係が発生するかチェック
    fn would_create_cycle(&self, from: usize, to: usize) -> bool {
        // toがfromに依存しているかチェック
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        
        queue.push_back(to);
        
        while let Some(current) = queue.pop_front() {
            if current == from {
                return true;
            }
            
            if !visited.insert(current) {
                continue;
            }
            
            if let Some(task) = self.tasks.get(&current) {
                for dep in &task.dependencies {
                    queue.push_back(*dep);
                }
            }
        }
        
        false
    }

    /// ビルドプランの実行順序を取得（トポロジカルソート）
    pub fn get_execution_order(&self) -> Vec<usize> {
        let mut result = Vec::new();
        let mut in_degree = HashMap::new();
        
        // 入次数を初期化
        for (id, _) in &self.tasks {
            in_degree.insert(*id, 0);
        }
        
        // 依存関係から入次数を計算
        for (id, task) in &self.tasks {
            for dep in &task.dependencies {
                *in_degree.entry(*dep).or_insert(0) += 1;
            }
        }
        
        // 入次数が0のノードをキューに追加
        let mut queue = VecDeque::new();
        for (id, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(*id);
            }
        }
        
        // トポロジカルソート
        while let Some(id) = queue.pop_front() {
            result.push(id);
            
            if let Some(deps) = self.dependents.get(&id) {
                for &dep_id in deps {
                    if let Some(degree) = in_degree.get_mut(&dep_id) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(dep_id);
                        }
                    }
                }
            }
        }
        
        result
    }

    /// ビルドプランのダンプ（デバッグ用）
    pub fn dump_plan(&self) -> String {
        let mut result = String::new();
        
        result.push_str(&format!("ビルドプラン（並列度: {}）:\n", self.parallelism));
        result.push_str("タスク一覧:\n");
        
        for (id, task) in &self.tasks {
            result.push_str(&format!("  [{}] {}: {}\n", id, task.kind_str(), task.description));
            
            if !task.dependencies.is_empty() {
                result.push_str("    依存: ");
                for dep in &task.dependencies {
                    result.push_str(&format!("{} ", dep));
                }
                result.push('\n');
            }
            
            if !task.input_files.is_empty() {
                result.push_str("    入力: ");
                for file in &task.input_files {
                    result.push_str(&format!("{} ", file.display()));
                }
                result.push('\n');
            }
            
            if !task.output_files.is_empty() {
                result.push_str("    出力: ");
                for file in &task.output_files {
                    result.push_str(&format!("{} ", file.display()));
                }
                result.push('\n');
            }
            
            result.push_str(&format!("    状態: {:?}\n", task.state));
        }
        
        result.push_str("\n実行順序:\n");
        for (i, id) in self.get_execution_order().iter().enumerate() {
            if let Some(task) = self.get_task(*id) {
                result.push_str(&format!("  {}. [{}] {}\n", i + 1, id, task.description));
            }
        }
        
        result
    }

    /// 全てのタスクをクリア
    pub fn clear(&mut self) {
        self.tasks.clear();
        self.root_tasks.clear();
        self.dependents.clear();
        self.next_task_id = 0;
    }
}

impl BuildTask {
    /// タスクの種類を文字列で取得
    fn kind_str(&self) -> &str {
        match self.kind {
            TaskKind::ReadSource => "ソース読込",
            TaskKind::Lexing => "字句解析",
            TaskKind::Parsing => "構文解析",
            TaskKind::SemanticAnalysis => "意味解析",
            TaskKind::IRGeneration => "IR生成",
            TaskKind::Optimization => "最適化",
            TaskKind::CodeGeneration => "コード生成",
            TaskKind::Linking => "リンク",
            TaskKind::DebugInfoGeneration => "デバッグ情報",
            TaskKind::Custom(ref s) => s,
        }
    }
} 
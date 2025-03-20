#!/usr/bin/env python3
import re
import os

# 対象ファイル
target_file = "crates/swiftlight-compiler/src/utils/parallel.rs"

# バックアップディレクトリ
backup_dir = "crates/backups"
os.makedirs(backup_dir, exist_ok=True)

# バックアップ作成
backup_path = os.path.join(backup_dir, "parallel.rs.bak")
if os.path.exists(target_file):
    with open(target_file, "r") as src:
        with open(backup_path, "w") as dst:
            dst.write(src.read())
    print(f"バックアップを作成しました: {backup_path}")

# ファイルを読み込み
if os.path.exists(target_file):
    with open(target_file, "r", encoding="utf-8") as f:
        content = f.read()
    
    # 修正1: Taskトレイトを検索して修正
    task_trait_pattern = r"pub trait Task([^{]*)\{([^}]*)\}"
    task_trait_match = re.search(task_trait_pattern, content, re.DOTALL)
    
    if task_trait_match:
        # 修正したTaskトレイト
        new_task_trait = """pub trait Task: Send + Sync + Ord {
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
}"""
        
        # トレイト定義を置換
        content = content.replace(task_trait_match.group(0), new_task_trait)
        
        # 修正2: BinaryHeap<Arc<dyn Task>>の使用部分を修正
        # TaskWrapperを導入してOrdを実装
        task_wrapper_impl = """
// Arc<dyn Task>をBinaryHeapで使えるようにするためのラッパー
struct TaskWrapper(Arc<dyn Task>);

impl PartialEq for TaskWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.0.priority() == other.0.priority()
    }
}

impl Eq for TaskWrapper {}

impl PartialOrd for TaskWrapper {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TaskWrapper {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // 優先度が低いほど先に実行される（BinaryHeapは最大値を取り出すため）
        other.0.priority().cmp(&self.0.priority())
    }
}

impl std::ops::Deref for TaskWrapper {
    type Target = Arc<dyn Task>;
    
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
"""
        
        # TaskQueueの定義を修正
        task_queue_pattern = r"struct TaskQueue\s*\{([^}]*)\}"
        task_queue_match = re.search(task_queue_pattern, content, re.DOTALL)
        
        if task_queue_match:
            old_queue_def = task_queue_match.group(0)
            new_queue_def = """struct TaskQueue {
    /// タスクキュー
    queue: Mutex<BinaryHeap<TaskWrapper>>,
    
    /// 実行中のタスク数
    running_tasks: AtomicUsize,
    
    /// タスク完了通知
    task_completed: Condvar,
}"""
            content = content.replace(old_queue_def, new_queue_def)
        
        # BinaryHeap<Arc<dyn Task>>をBinaryHeap<TaskWrapper>に変更
        content = re.sub(
            r"BinaryHeap<Arc<dyn Task>>",
            r"BinaryHeap<TaskWrapper>",
            content
        )
        
        # push メソッドを修正
        content = re.sub(
            r"queue\.push\(task\)",
            r"queue.push(TaskWrapper(task))",
            content
        )
        
        # 必要なインポートを追加
        imports_pattern = r"use std::([^;]+);"
        imports_match = re.search(imports_pattern, content)
        if imports_match:
            first_import = imports_match.group(0)
            if "cmp::Ordering" not in content:
                content = content.replace(
                    first_import,
                    first_import + "\nuse std::cmp::Ordering;"
                )
        
        # TaskWrapperの実装を適切な位置に挿入
        struct_pattern = r"pub struct TaskManager\s*\{([^}]*)\}"
        struct_match = re.search(struct_pattern, content, re.DOTALL)
        if struct_match:
            content = content.replace(
                struct_match.group(0),
                task_wrapper_impl + "\n\n" + struct_match.group(0)
            )
        
        # ファイルに書き戻す
        with open(target_file, "w", encoding="utf-8") as f:
            f.write(content)
        
        print(f"ファイル {target_file} を修正しました")
    else:
        print("Taskトレイトが見つかりませんでした")
else:
    print(f"ファイル {target_file} が見つかりません") 
#!/usr/bin/env python3
import re
import os
import glob

# 対象ファイル
target_file = "crates/swiftlight-compiler/src/optimization.rs"

# バックアップディレクトリ
backup_dir = "crates/backups"
os.makedirs(backup_dir, exist_ok=True)

# バックアップ作成
backup_path = os.path.join(backup_dir, "optimization.rs.bak")
if os.path.exists(target_file):
    with open(target_file, "r") as src:
        with open(backup_path, "w") as dst:
            dst.write(src.read())
    print(f"バックアップを作成しました: {backup_path}")

# OptimizationPassトレイト実装の修正
implementations = [
    {
        "class": "ConstantFolding",
        "kind": "AST",
        "pattern": r"impl ASTOptimizationPass for ConstantFolding \{([^}]*)\}",
        "method": """
    fn execute(&self) -> OptimizationPassResult {
        // このメソッドは直接呼ばれない - run_on_astが使用される
        OptimizationPassResult::new(false, std::time::Duration::from_secs(0))
    }"""
    },
    {
        "class": "DeadCodeElimination",
        "kind": "AST",
        "pattern": r"impl ASTOptimizationPass for DeadCodeElimination \{([^}]*)\}",
        "method": """
    fn execute(&self) -> OptimizationPassResult {
        // このメソッドは直接呼ばれない - run_on_astが使用される
        OptimizationPassResult::new(false, std::time::Duration::from_secs(0))
    }"""
    },
    {
        "class": "CommonSubexpressionElimination",
        "kind": "IR",
        "pattern": r"impl IROptimizationPass for CommonSubexpressionElimination \{([^}]*)\}",
        "method": """
    fn execute(&self) -> OptimizationPassResult {
        // このメソッドは直接呼ばれない - run_on_irが使用される
        OptimizationPassResult::new(false, std::time::Duration::from_secs(0))
    }"""
    },
    {
        "class": "InstructionCombining",
        "kind": "IR",
        "pattern": r"impl IROptimizationPass for InstructionCombining \{([^}]*)\}",
        "method": """
    fn execute(&self) -> OptimizationPassResult {
        // このメソッドは直接呼ばれない - run_on_irが使用される
        OptimizationPassResult::new(false, std::time::Duration::from_secs(0))
    }"""
    },
    {
        "class": "LoopOptimization",
        "kind": "IR",
        "pattern": r"impl IROptimizationPass for LoopOptimization \{([^}]*)\}",
        "method": """
    fn execute(&self) -> OptimizationPassResult {
        // このメソッドは直接呼ばれない - run_on_irが使用される
        OptimizationPassResult::new(false, std::time::Duration::from_secs(0))
    }"""
    }
]

# 実装の追加関数
def add_execute_method(content, implementation):
    class_name = implementation["class"]
    pattern = r"impl OptimizationPass for " + class_name + r" \{(.*?)\}"
    
    match = re.search(pattern, content, re.DOTALL)
    if match:
        impl_content = match.group(1)
        # executeメソッドが既に存在するか確認
        if "fn execute" not in impl_content:
            # executeメソッドを追加
            new_impl_content = impl_content + implementation["method"]
            new_content = content.replace(match.group(0), 
                                         f"impl OptimizationPass for {class_name} {{{new_impl_content}}}")
            return new_content, True
    
    return content, False

# ファイルを処理
if os.path.exists(target_file):
    with open(target_file, "r") as f:
        content = f.read()
    
    changes_made = False
    
    # 各実装に対して修正を試みる
    for impl in implementations:
        content, changed = add_execute_method(content, impl)
        if changed:
            changes_made = True
            print(f"{impl['class']}にexecuteメソッドを追加しました")
    
    # 変更があれば更新
    if changes_made:
        with open(target_file, "w") as f:
            f.write(content)
        print(f"ファイル {target_file} を更新しました")
    else:
        print(f"ファイル {target_file} に必要な変更はありませんでした")
else:
    print(f"ファイル {target_file} が見つかりません") 
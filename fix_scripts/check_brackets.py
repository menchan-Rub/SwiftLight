#!/usr/bin/env python3
import os
import re

# 対象ファイル
file_path = "crates/swiftlight-compiler/src/middleend/ir/mod.rs"

# ファイル内容を読み込む
with open(file_path, "r", encoding="utf-8") as f:
    lines = f.readlines()

# 各行の括弧の数をチェック
stack = []
line_num = 0
problems = []

for i, line in enumerate(lines):
    line_num = i + 1
    line_text = line.rstrip('\n')
    
    # 各文字をチェック
    for j, char in enumerate(line_text):
        if char == '{':
            stack.append((line_num, j + 1))
        elif char == '}':
            if stack:
                stack.pop()
            else:
                # スタックが空なのに閉じ括弧が出現 - 過剰な閉じ括弧
                problems.append(f"過剰な閉じ括弧: 行 {line_num}, 位置 {j + 1}")
    
    # 特定のインデントパターンをチェック
    if re.match(r'^    \}', line_text):
        if "else" not in lines[i-1]:  # 前の行がelseでない場合
            problems.append(f"インデントがおかしい閉じ括弧: 行 {line_num} - '{line_text}'")
    
    # implブロックの開始と終了をチェック
    if "impl<'ctx> IRGenerator<'ctx>" in line_text:
        impl_start = line_num
    
    # matchステートメントのチェック
    if "match " in line_text and "{" in line_text:
        match_start = line_num

# スタックに残った開き括弧をチェック
for line_num, col in stack:
    problems.append(f"閉じられていない開き括弧: 行 {line_num}, 位置 {col}")

# 結果レポート
if problems:
    print(f"ファイル {file_path} に {len(problems)} 個の問題が見つかりました:")
    for problem in problems:
        print(f"- {problem}")
    
    # 最後の括弧のインデントレベルをチェック
    last_closing = None
    impl_end = None
    for i in range(len(lines) - 1, -1, -1):
        line = lines[i].rstrip('\n')
        if line.strip() == '}':
            last_closing = i + 1
            indent_level = len(line) - len(line.lstrip())
            print(f"最後の閉じ括弧: 行 {last_closing}, インデントレベル {indent_level}")
            break
    
    # implブロックの終了位置の候補を探す
    impl_candidates = []
    for i in range(len(lines) - 1, -1, -1):
        line = lines[i].rstrip('\n')
        if line.strip() == '}' and i > 100:  # 最初の100行は無視
            indent_level = len(line) - len(line.lstrip())
            if indent_level == 0:
                impl_candidates.append(i + 1)
    
    if impl_candidates:
        print(f"implブロックの終了位置の候補: {impl_candidates}")
    
    # 修正案: 最後の関数の閉じ括弧のインデントを修正
    # 行番号860〜890の範囲で "    }" のインデントを "}" に変更
    updated = False
    for i in range(860, min(890, len(lines))):
        line = lines[i]
        if line.strip() == '}' and line.startswith('    }'):
            lines[i] = '}\n'
            print(f"行 {i + 1} のインデントを修正しました")
            updated = True
    
    if updated:
        # バックアップディレクトリ
        backup_dir = "crates/backups/manual"
        os.makedirs(backup_dir, exist_ok=True)
        
        # バックアップ作成
        backup_path = os.path.join(backup_dir, "mod.rs.brackets.bak")
        with open(backup_path, "w") as f:
            f.writelines(lines)
        print(f"バックアップを作成しました: {backup_path}")
        
        # 修正したファイルを書き込む
        with open(file_path, "w") as f:
            f.writelines(lines)
        print(f"ファイル {file_path} を修正しました")
else:
    print(f"ファイル {file_path} に問題は見つかりませんでした。") 
#!/usr/bin/env python3
import os
import re

# 対象ファイル
file_path = "crates/swiftlight-compiler/src/middleend/ir/mod.rs"

# バックアップディレクトリ
backup_dir = "crates/backups/manual"
os.makedirs(backup_dir, exist_ok=True)

# バックアップ作成
backup_path = os.path.join(backup_dir, "mod.rs.impl.bak")
with open(file_path, "r") as src:
    with open(backup_path, "w") as dst:
        dst.write(src.read())
print(f"バックアップを作成しました: {backup_path}")

# ファイル内容を読み込む
with open(file_path, "r", encoding="utf-8") as f:
    content = f.read()

# 括弧の数をチェック
opening_brackets = content.count("{")
closing_brackets = content.count("}")
print(f"開き括弧: {opening_brackets}, 閉じ括弧: {closing_brackets}")

if opening_brackets > closing_brackets:
    diff = opening_brackets - closing_brackets
    print(f"閉じ括弧が{diff}個不足しています。追加します。")
    
    # ファイルの末尾に閉じ括弧を追加
    with open(file_path, "a") as f:
        for _ in range(diff):
            f.write("\n}")
    
    print(f"閉じ括弧を{diff}個追加しました。")
elif closing_brackets > opening_brackets:
    diff = closing_brackets - opening_brackets
    print(f"閉じ括弧が{diff}個過剰です。")
    
    # 最後の閉じ括弧を見つけて削除する
    lines = content.split("\n")
    removed = 0
    for i in range(len(lines) - 1, -1, -1):
        if lines[i].strip() == "}" and removed < diff:
            lines[i] = ""
            removed += 1
    
    # 修正したコンテンツを書き込む
    with open(file_path, "w") as f:
        f.write("\n".join(lines))
    
    print(f"過剰な閉じ括弧を{removed}個削除しました。")
else:
    print("括弧の数は一致しています。インデントの問題かもしれません。")
    
    # implの構造を修正
    # 最後のブロックから1000行程度を取得
    lines = content.split('\n')
    last_part = '\n'.join(lines[-1000:])
    
    # impleブロックとその終了を探す
    impl_pattern = r"impl<'ctx> IRGenerator<'ctx> \{(.*?)\n\}"
    impl_match = re.search(impl_pattern, last_part, re.DOTALL)
    
    if impl_match:
        # インデントを整える
        fixed_content = content.replace("    }", "}")
        with open(file_path, "w") as f:
            f.write(fixed_content)
        print("インデントを修正しました。")
    else:
        print("impl本体の修正が必要です。手動で確認してください。") 
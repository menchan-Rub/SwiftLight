#!/usr/bin/env python3
import re
import os
import shutil

# 修正ターゲットファイル
target_file = "crates/swiftlight-compiler/src/backend/llvm/codegen.rs"

# バックアップ作成
backup_file = target_file + ".bak-fix"
shutil.copy2(target_file, backup_file)
print(f"バックアップを作成しました: {backup_file}")

# ファイルを行ごとに読み込み
with open(target_file, "r") as f:
    lines = f.readlines()

# 修正した行を格納するリスト
fixed_lines = []

# 各行を処理して必要な修正を行う
for line in lines:
    # build_struct_gep関数の引数の修正
    # パターン: build_struct_gep(typ, ptr, idx, name) -> build_struct_gep(idx, name)
    if "build_struct_gep" in line:
        # すでに正しいパターンかチェック
        if not re.search(r"build_struct_gep\([0-9]+\s*(as\s*u32)?\s*,", line):
            # 行全体のパターンに基づいて置換
            line = re.sub(
                r"(self\.builder\.build_struct_gep\()([^,]+),\s*([^,]+),\s*([^,]+),\s*([^,\)]+)(\))",
                r"\1\4, \5\6",
                line
            )
    
    # build_load関数の引数の修正
    # パターン: build_load(typ, ptr, name) -> build_load(ptr, name)
    if "build_load" in line:
        # 正規表現パターンを慎重に作成
        line = re.sub(
            r"(self\.builder\.build_load\()([^,]+),\s*([^,]+),\s*([^,\)]+)(\))",
            r"\1\3, \4\5",
            line
        )
    
    fixed_lines.append(line)

# 修正した内容を書き込む
with open(target_file, "w") as f:
    f.writelines(fixed_lines)

print(f"ファイル {target_file} の修正が完了しました")

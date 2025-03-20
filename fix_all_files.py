#!/usr/bin/env python3
import re
import os
import glob

# 修正対象のファイルリスト
target_files = [
    "crates/swiftlight-compiler/src/middleend/ir/mod.rs",
    "crates/swiftlight-compiler/src/backend/llvm/codegen.rs"
]

# 各ファイルを修正
for target_file in target_files:
    print(f"ファイル {target_file} を処理中...")
    
    # バックアップを作成
    backup_file = target_file + ".bak-safe"
    if not os.path.exists(backup_file):
        with open(target_file, "r") as src:
            with open(backup_file, "w") as dst:
                dst.write(src.read())
        print(f"バックアップを作成しました: {backup_file}")
    
    # ファイルの内容を読み込む
    with open(target_file, "r") as f:
        lines = f.readlines()
    
    # 各行を処理
    fixed_lines = []
    for line in lines:
        # build_struct_gep の修正
        if "build_struct_gep" in line:
            # すでに正しい引数のパターン
            if re.search(r'build_struct_gep\s*\(\s*[0-9]+\s*(as\s*u32)?\s*,', line):
                # 正しいパターン（index, name）の場合は変更しない
                fixed_lines.append(line)
            else:
                # 4引数パターン -> 2引数パターン
                new_line = re.sub(
                    r'build_struct_gep\s*\(\s*([^,]+)\s*,\s*([^,]+)\s*,\s*([^,]+)\s*,\s*([^,\)]+)\s*\)',
                    r'build_struct_gep(\3, \4)',
                    line
                )
                fixed_lines.append(new_line)
        
        # build_load の修正
        elif "build_load" in line:
            # すでに正しいパターン
            if "build_load" in line and not re.search(r'build_load\s*\(\s*[^,]+\s*,\s*[^,]+\s*,', line):
                fixed_lines.append(line)
            else:
                # 3引数パターン -> 2引数パターン
                new_line = re.sub(
                    r'build_load\s*\(\s*([^,]+)\s*,\s*([^,]+)\s*,\s*([^,\)]+)\s*\)',
                    r'build_load(\2, \3)',
                    line
                )
                fixed_lines.append(new_line)
        else:
            fixed_lines.append(line)
    
    # 修正した内容を書き込む
    with open(target_file, "w") as f:
        f.writelines(fixed_lines)
    
    print(f"ファイル {target_file} の修正が完了しました")

print("すべてのファイルの処理が完了しました") 
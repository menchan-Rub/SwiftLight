import re
import os

target_file = "crates/swiftlight-compiler/src/backend/llvm/codegen.rs"
backup_file = target_file + ".final-bak"

# バックアップ作成
if not os.path.exists(backup_file):
    with open(target_file, "r") as src:
        with open(backup_file, "w") as dst:
            dst.write(src.read())
    print(f"バックアップを作成しました: {backup_file}")

# ファイル内容を読み込み
with open(target_file, "r") as f:
    lines = f.readlines()

# 修正した行を格納するリスト
fixed_lines = []

# 各行ごとに修正
for line in lines:
    # build_struct_gep の修正
    if "build_struct_gep" in line:
        # すでに正しい引数のパターン
        if re.search(r'build_struct_gep\s*\(\s*[0-9]+\s*(as\s*u32)?\s*,', line):
            # インデックス, 名前の形式は変更しない
            fixed_lines.append(line)
        else:
            # 型パラメータを削除
            new_line = re.sub(
                r'build_struct_gep\s*\(\s*([^,]+)\s*,\s*([^,]+)\s*,\s*([^,]+)\s*,\s*([^,\)]+)\s*\)',
                r'build_struct_gep(\3, \4)',
                line
            )
            fixed_lines.append(new_line)
    
    # build_load の修正
    elif "build_load" in line:
        # 3引数パターン -> 2引数パターン
        new_line = re.sub(
            r'build_load\s*\(\s*([^,]+)\s*,\s*([^,]+)\s*,\s*([^,\)]+)\s*\)',
            r'build_load(\2, \3)',
            line
        )
        # 1引数だけのパターン -> 2引数パターン
        new_line = re.sub(
            r'build_load\s*\(\s*&format!\("([^"]+)"\)\s*\)',
            r'build_load(thread_id, &format!("\1"))',
            new_line
        )
        fixed_lines.append(new_line)
    else:
        fixed_lines.append(line)

# 修正した内容を書き込む
with open(target_file, "w") as f:
    f.writelines(fixed_lines)

print(f"ファイルの修正が完了しました: {target_file}") 
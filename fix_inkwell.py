import re
import os

# ファイルのバックアップを作成
os.system('cp crates/swiftlight-compiler/src/middleend/ir/mod.rs crates/swiftlight-compiler/src/middleend/ir/mod.rs.pre_fix')

# ファイルを読み込む
with open('crates/swiftlight-compiler/src/middleend/ir/mod.rs', 'r') as f:
    content = f.read()

# build_struct_gep の修正：最初の型引数を削除
content = re.sub(r'build_struct_gep\(([^,]+),\s*([^,]+),\s*([^)]+)\)', r'build_struct_gep(\2, \3)', content)

# build_load の修正：最初の型引数を削除
content = re.sub(r'build_load\(([^,]+),\s*([^)]+)\)', r'build_load(\2)', content)

# 修正した内容をファイルに書き込む
with open('crates/swiftlight-compiler/src/middleend/ir/mod.rs', 'w') as f:
    f.write(content)

print("修正が完了しました。")

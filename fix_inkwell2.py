import re
import os

# バックアップを作成（すでに存在する場合は作成しない）
target_file = "crates/swiftlight-compiler/src/middleend/ir/mod.rs"
backup_file = target_file + ".bak4"
if not os.path.exists(backup_file):
    with open(target_file, "r") as src:
        with open(backup_file, "w") as dst:
            dst.write(src.read())
    print(f"バックアップを作成しました: {backup_file}")

# 問題のある行を手動で修正
fixes = [
    # 1. build_load関数の修正
    # 1引数だけのケース
    (r'let value = self\.builder\.build_load\("indexload"\);', 
     r'let value = self.builder.build_load(element_ptr, "indexload");'),
    
    (r'let expr_value = self\.builder\.build_load\("match_expr_val"\);', 
     r'let expr_value = self.builder.build_load(expr_ptr, "match_expr_val");'),
    
    # 3引数から2引数への修正
    (r'let value = self\.builder\.build_load\(([^,]+),\s*([^,]+),\s*([^,\)]+)\)', 
     r'let value = self.builder.build_load(\2, \3)'),
    
    # 2. build_struct_gep関数の修正
    # 先頭にtype引数がある4引数の形式を2引数に
    (r'self\.builder\.build_struct_gep\(([^,]+),\s*([^,]+),\s*([^,]+),\s*([^,\)]+)\)', 
     r'self.builder.build_struct_gep(\3, \4)'),
    
    # 大きな問題: 特定の行での間違った修正を修正
    (r'let field_ptr = self\.builder\.build_struct_gep\(([^,\n\r]+),\s*\n', 
     r'let field_ptr = self.builder.build_struct_gep(\n'),
    
    # イレギュラーなパターン
    (r'build_struct_gep\(idx as u32, &format!\("{}_ptr", field_name\.name\)', 
     r'build_struct_gep(struct_ptr, idx as u32, &format!("{}_ptr", field_name.name)'),
]

# ファイルを読み込み
with open(target_file, "r") as f:
    content = f.read()

# 各修正を適用
for pattern, replacement in fixes:
    content = re.sub(pattern, replacement, content)

# 修正したコンテンツを書き戻す
with open(target_file, "w") as f:
    f.write(content)

print(f"ファイルの修正が完了しました: {target_file}") 
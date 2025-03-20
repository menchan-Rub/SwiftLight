#!/usr/bin/env python3
import os
import re

# 対象ファイル
file_path = "crates/swiftlight-compiler/src/middleend/ir/mod.rs"

# バックアップディレクトリ
backup_dir = "crates/backups/manual"
os.makedirs(backup_dir, exist_ok=True)

# バックアップ作成
backup_path = os.path.join(backup_dir, "mod.rs.gep.bak")
with open(file_path, "r") as src:
    with open(backup_path, "w") as dst:
        dst.write(src.read())
print(f"バックアップを作成しました: {backup_path}")

# ファイル内容を読み込む
with open(file_path, "r", encoding="utf-8") as f:
    content = f.read()

# 問題箇所の特定と修正
# 1. 1286-1288行目の修正
gep_error1 = """        unsafe {
            let field_ptr = self.builder.build_struct_gep(struct_name, member.name)
            ).map_err(|e| CompilerError::code_generation_error("""

gep_fix1 = """        unsafe {
            let field_ptr = self.builder.build_struct_gep(struct_ptr, field_idx, &member.name)
                .map_err(|e| CompilerError::new(ErrorKind::CodeGeneration, """

# 2. 1998-2000行目の修正
gep_error2 = """                   let field_ptr = unsafe {
                                        self.builder.build_struct_gep(field_name.name)
                                        )?"""

gep_fix2 = """                   let field_ptr = unsafe {
                                        self.builder.build_struct_gep(struct_ptr, field_idx, &field_name.name)
                                    }?"""

# 修正を適用
modified_content = content.replace(gep_error1, gep_fix1).replace(gep_error2, gep_fix2)

# 3. その他のbuild_struct_gep関数の呼び出しを修正
# 新しいInkwell APIに合わせて、型引数を削除
gep_pattern = r'build_struct_gep\(([^,]+),\s*([^,]+),\s*([^,]+),\s*([^,\)]+)\)'
modified_content = re.sub(gep_pattern, r'build_struct_gep(\2, \3, \4)', modified_content)

# 2引数のbuild_struct_gepを修正して3引数に変更
gep_pattern2 = r'build_struct_gep\(([^,]+),\s*([^,\)]+)\)'
modified_content = re.sub(gep_pattern2, r'build_struct_gep(\1, 0, "\2")', modified_content)

# ファイルに修正を書き戻す
with open(file_path, "w", encoding="utf-8") as f:
    f.write(modified_content)

print(f"ファイル {file_path} のbuild_struct_gep関数の呼び出しエラーを修正しました") 
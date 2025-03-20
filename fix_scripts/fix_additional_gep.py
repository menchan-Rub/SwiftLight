#!/usr/bin/env python3
import os
import re

# 対象ファイル
file_path = "crates/swiftlight-compiler/src/middleend/ir/mod.rs"

# バックアップディレクトリ
backup_dir = "crates/backups/manual"
os.makedirs(backup_dir, exist_ok=True)

# バックアップ作成
backup_path = os.path.join(backup_dir, "mod.rs.gep2.bak")
with open(file_path, "r") as src:
    with open(backup_path, "w") as dst:
        dst.write(src.read())
print(f"バックアップを作成しました: {backup_path}")

# ファイル内容を読み込む
with open(file_path, "r", encoding="utf-8") as f:
    content = f.read()

# 問題箇所の特定と修正
# 1. 1476-1478行目の修正
gep_error1 = """    let element_ptr = unsafe {
        self.builder.build_struct_gep(i)
        ).map_err(|_| CompilerError::new(ErrorKind::CodeGeneration, format!("タプル要素 {} へのアクセスに失敗しました", i),"""

gep_fix1 = """    let element_ptr = unsafe {
        self.builder.build_struct_gep(tuple_ptr, i, &format!("tuple_{}", i))
    }.map_err(|_| CompilerError::new(ErrorKind::CodeGeneration, format!("タプル要素 {} へのアクセスに失敗しました", i),"""

# 2. 1996-1998行目の修正
gep_error2 = """                  let field_ptr = unsafe {
                                        self.builder.build_struct_gep(field_name.name)
                                        );"""

gep_fix2 = """                  let field_ptr = unsafe {
                                        self.builder.build_struct_gep(struct_ptr, field_idx, &field_name.name)
                                    };"""

# 修正を適用
modified_content = content.replace(gep_error1, gep_fix1).replace(gep_error2, gep_fix2)

# 3. 括弧のバランスを修正するためにファイルの末尾をチェック
# 末尾の行が `}` または `} }` 形式になっているかを確認し、適切に修正
lines = modified_content.split('\n')
last_line = lines[-1].strip()

if last_line == '}':
    # OK、変更不要
    pass
elif '}' in last_line:
    # 余分な空白や括弧を取り除く
    lines[-1] = '}'
    modified_content = '\n'.join(lines)
else:
    # 末尾に閉じ括弧がない場合、追加する
    modified_content += '\n}'

# ファイルに修正を書き戻す
with open(file_path, "w", encoding="utf-8") as f:
    f.write(modified_content)

print(f"ファイル {file_path} の追加のbuild_struct_gep関数の呼び出しエラーを修正しました") 
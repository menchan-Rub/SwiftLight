#!/usr/bin/env python3
import os

# 対象ファイル
file_path = "crates/swiftlight-compiler/src/backend/wasm/codegen.rs"

# バックアップディレクトリ
backup_dir = "crates/backups/manual"
os.makedirs(backup_dir, exist_ok=True)

# バックアップ作成
backup_path = os.path.join(backup_dir, "codegen_wasm.rs.final2.bak")
with open(file_path, "r") as src:
    content = src.read()
    with open(backup_path, "w") as dst:
        dst.write(content)
print(f"バックアップを作成しました: {backup_path}")

# 問題の行を修正
with open(file_path, "r") as f:
    lines = f.readlines()

# 177行目の問題（0インデックスでは176）を修正
if len(lines) > 176:
    # 中括弧が閉じられていない問題を修正
    lines[176] = '            TypeKind::Primitive(PrimitiveType::Integer { bits, signed: _ }) => {\n'

# 修正した内容を書き込む
with open(file_path, "w") as f:
    f.writelines(lines)

print(f"ファイル {file_path} の括弧問題を修正しました") 
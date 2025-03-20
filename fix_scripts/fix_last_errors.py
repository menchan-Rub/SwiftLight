#!/usr/bin/env python3
import os

# 対象ファイル
file_path = "crates/swiftlight-compiler/src/backend/llvm/codegen.rs"

# バックアップディレクトリ
backup_dir = "crates/backups/manual"
os.makedirs(backup_dir, exist_ok=True)

# バックアップ作成
backup_path = os.path.join(backup_dir, "codegen.rs.final2.bak")
with open(file_path, "r") as src:
    with open(backup_path, "w") as dst:
        dst.write(src.read())
print(f"バックアップを作成しました: {backup_path}")

# ファイルを行ごとに読み込む
with open(file_path, "r", encoding="utf-8") as f:
    lines = f.readlines()

# 修正: Boolean)に余分な)がある
lines[125] = '            TypeKind::Primitive(PrimitiveType::Boolean) => self.context.bool_type().into(),\n'

# ファイルに書き戻す
with open(file_path, "w", encoding="utf-8") as f:
    f.writelines(lines)

print(f"ファイル {file_path} の最後のエラーを修正しました") 
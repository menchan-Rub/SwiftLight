#!/usr/bin/env python3
import os

# バックアップディレクトリ
backup_dir = "crates/backups/manual"
os.makedirs(backup_dir, exist_ok=True)

# 修正1: wasm/codegen.rsの修正
wasm_codegen_path = "crates/swiftlight-compiler/src/backend/wasm/codegen.rs"
backup_path1 = os.path.join(backup_dir, "codegen_wasm.rs.final.bak")
with open(wasm_codegen_path, "r") as src:
    content = src.read()
    with open(backup_path1, "w") as dst:
        dst.write(content)
print(f"バックアップを作成しました: {backup_path1}")

# PrimitiveType::Integerを修正
wasm_fixed_content = content.replace("PrimitiveType::Int)eger", "PrimitiveType::Integer")

with open(wasm_codegen_path, "w") as f:
    f.write(wasm_fixed_content)
print(f"ファイル {wasm_codegen_path} を修正しました")

# 修正2: utils/logger.rsの修正
logger_path = "crates/swiftlight-compiler/src/utils/logger.rs"
backup_path2 = os.path.join(backup_dir, "logger.rs.final2.bak")
with open(logger_path, "r") as src:
    lines = src.readlines()
    with open(backup_path2, "w") as dst:
        dst.writelines(lines)
print(f"バックアップを作成しました: {backup_path2}")

# 余分な閉じ括弧を削除
if len(lines) > 200 and lines[200].strip() == "}":
    lines[200] = ""  # 余分な行を削除

with open(logger_path, "w") as f:
    f.writelines(lines)
print(f"ファイル {logger_path} を修正しました")

print("すべてのエラーの修正が完了しました！") 
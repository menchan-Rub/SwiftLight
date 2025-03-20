#!/usr/bin/env python3
import os

# 対象ファイル
file_path = "crates/swiftlight-compiler/src/backend/llvm/codegen.rs"

# バックアップディレクトリ
backup_dir = "crates/backups/manual"
os.makedirs(backup_dir, exist_ok=True)

# バックアップ作成
backup_path = os.path.join(backup_dir, "codegen.rs.bak3")
with open(file_path, "r") as src:
    with open(backup_path, "w") as dst:
        dst.write(src.read())
print(f"バックアップを作成しました: {backup_path}")

# ファイルを行ごとに読み込む
with open(file_path, "r", encoding="utf-8") as f:
    lines = f.readlines()

# 特定の行番号の問題を修正
# 1. 126行目のBooleanの閉じ括弧問題
if len(lines) > 126:
    lines[125] = lines[125].replace('TypeKind::Primitive(PrimitiveType::Boolean', 'TypeKind::Primitive(PrimitiveType::Boolean)')

# 2. 974-979行目のデフォルト値の括弧問題
if len(lines) > 979:
    lines[974] = '            let default_value = match global_var.as_pointer_value().get_type().get_element_type() {\n'
    lines[975] = '                AnyTypeEnum::IntType(int_ty) => int_ty.const_zero(),\n'
    lines[976] = '                AnyTypeEnum::FloatType(float_ty) => float_ty.const_zero(),\n'
    lines[977] = '                AnyTypeEnum::PointerType(ptr_ty) => ptr_ty.const_null(),\n'
    lines[978] = '                AnyTypeEnum::StructType(struct_ty) => struct_ty.const_zero(),\n'
    lines[979] = '                AnyTypeEnum::ArrayType(array_ty) => array_ty.const_zero(),\n'

# 3. global.as_pointer_valueの修正（3531行目）
if len(lines) > 3531:
    lines[3530] = lines[3530].replace('global.as_pointer_value(, "load")', 'global.as_pointer_value(), "load"')

# 修正した内容をファイルに書き戻す
with open(file_path, "w", encoding="utf-8") as f:
    f.writelines(lines)

print(f"ファイル {file_path} の構文エラーを直接修正しました") 
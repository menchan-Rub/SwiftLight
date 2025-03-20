#!/usr/bin/env python3
import os
import re

# 対象ファイル
file_path = "crates/swiftlight-compiler/src/backend/llvm/codegen.rs"

# バックアップディレクトリ
backup_dir = "crates/backups/manual"
os.makedirs(backup_dir, exist_ok=True)

# バックアップ作成
backup_path = os.path.join(backup_dir, "codegen.rs.bak2")
with open(file_path, "r") as src:
    with open(backup_path, "w") as dst:
        dst.write(src.read())
print(f"バックアップを作成しました: {backup_path}")

# ファイル内容を読み込む
with open(file_path, "r", encoding="utf-8") as f:
    content = f.read()

# 1. PrimitiveType::Integerの修正（127行目の括弧問題）
pattern1 = r'TypeKind::Primitive\(PrimitiveType::Integer \{ bits, signed \} =>'
replacement1 = r'TypeKind::Primitive(PrimitiveType::Integer { bits, signed }) =>'

# 2. default_valueのmatchの括弧問題（974-979行目）
# まず問題のある部分を特定
match_pattern = r"""            let default_value = match global_var\.as_pointer_value\(\)\.get_type\(\)\.get_element_type\(\) \{
                AnyTypeEnum::IntType\(int_ty\) => int_ty\.const_zero\(\)\),
                AnyTypeEnum::FloatType\(float_ty\) => float_ty\.const_zero\(\)\),
                AnyTypeEnum::PointerType\(ptr_ty\) => ptr_ty\.const_null\(\)\),
                AnyTypeEnum::StructType\(struct_ty\) => struct_ty\.const_zero\(\)\),
                AnyTypeEnum::ArrayType\(array_ty\) => array_ty\.const_zero\(\)\),"""

# 修正版
match_replacement = r"""            let default_value = match global_var.as_pointer_value().get_type().get_element_type() {
                AnyTypeEnum::IntType(int_ty) => int_ty.const_zero(),
                AnyTypeEnum::FloatType(float_ty) => float_ty.const_zero(),
                AnyTypeEnum::PointerType(ptr_ty) => ptr_ty.const_null(),
                AnyTypeEnum::StructType(struct_ty) => struct_ty.const_zero(),
                AnyTypeEnum::ArrayType(array_ty) => array_ty.const_zero(),"""

# 3. global.as_pointer_valueの修正（3531行目）
pattern3 = r'global\.as_pointer_value\((, "load"\))'
replacement3 = r'global.as_pointer_value()\1'

# すべてのパターンを適用
modified_content = content
modified_content = re.sub(pattern1, replacement1, modified_content)
modified_content = modified_content.replace(match_pattern, match_replacement)
modified_content = re.sub(pattern3, replacement3, modified_content)

# ファイルに修正を書き戻す
with open(file_path, "w", encoding="utf-8") as f:
    f.write(modified_content)

print(f"ファイル {file_path} の残りの構文エラーを修正しました") 
#!/usr/bin/env python3
import os
import re

# 対象ファイル
file_path = "crates/swiftlight-compiler/src/backend/llvm/codegen.rs"

# バックアップディレクトリ
backup_dir = "crates/backups/manual"
os.makedirs(backup_dir, exist_ok=True)

# バックアップ作成
backup_path = os.path.join(backup_dir, "codegen.rs.bak")
with open(file_path, "r") as src:
    with open(backup_path, "w") as dst:
        dst.write(src.read())
print(f"バックアップを作成しました: {backup_path}")

# ファイル内容を読み込む
with open(file_path, "r", encoding="utf-8") as f:
    content = f.read()

# 1. PrimitiveType::Booleanの修正
pattern1 = r'PrimitiveType::Bool\)ean'
replacement1 = r'PrimitiveType::Boolean'

# 2. PrimitiveType::Integerの修正 
pattern2 = r'PrimitiveType::Int\)eger'
replacement2 = r'PrimitiveType::Integer'

# 3. AnyTypeEnum::fromの修正
pattern3 = r'AnyTypeEnum::from\(BasicTypeEnum::IntType\(([^)]+)\) =>'
replacement3 = r'AnyTypeEnum::IntType(\1) =>'

pattern4 = r'AnyTypeEnum::from\(BasicTypeEnum::FloatType\(([^)]+)\) =>'
replacement4 = r'AnyTypeEnum::FloatType(\1) =>'

pattern5 = r'AnyTypeEnum::from\(BasicTypeEnum::PointerType\(([^)]+)\) =>'
replacement5 = r'AnyTypeEnum::PointerType(\1) =>'

pattern6 = r'AnyTypeEnum::from\(BasicTypeEnum::StructType\(([^)]+)\) =>'
replacement6 = r'AnyTypeEnum::StructType(\1) =>'

pattern7 = r'AnyTypeEnum::from\(BasicTypeEnum::ArrayType\(([^)]+)\) =>'
replacement7 = r'AnyTypeEnum::ArrayType(\1) =>'

# 4. global.as_pointer_valueの修正
pattern8 = r'global\.as_pointer_value\(, "load"\)'
replacement8 = r'global.as_pointer_value(), "load"'

# 5. TypeKind::Primitive関連の修正
pattern9 = r'TypeKind::Primitive\(PrimitiveType::Int\)\(([^)]+)\)'
replacement9 = r'TypeKind::Primitive(PrimitiveType::Integer { bits: \1, signed: true })'

pattern10 = r'TypeKind::Primitive\(PrimitiveType::Float\)\(([^)]+)\)'
replacement10 = r'TypeKind::Primitive(PrimitiveType::Float { bits: \1 })'

# 6. 余分な括弧の修正
pattern11 = r'\)\)'
replacement11 = r')'

# すべてのパターンを適用
modified_content = content
modified_content = re.sub(pattern1, replacement1, modified_content)
modified_content = re.sub(pattern2, replacement2, modified_content)
modified_content = re.sub(pattern3, replacement3, modified_content)
modified_content = re.sub(pattern4, replacement4, modified_content)
modified_content = re.sub(pattern5, replacement5, modified_content)
modified_content = re.sub(pattern6, replacement6, modified_content)
modified_content = re.sub(pattern7, replacement7, modified_content)
modified_content = re.sub(pattern8, replacement8, modified_content)
modified_content = re.sub(pattern9, replacement9, modified_content)
modified_content = re.sub(pattern10, replacement10, modified_content)

# ファイルに修正を書き戻す
with open(file_path, "w", encoding="utf-8") as f:
    f.write(modified_content)

print(f"ファイル {file_path} の構文エラーを修正しました") 
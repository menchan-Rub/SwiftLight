#!/usr/bin/env python3
import os
import re

# 対象ファイル
file_path = "crates/swiftlight-compiler/src/middleend/ir/mod.rs"

# バックアップディレクトリ
backup_dir = "crates/backups/manual"
os.makedirs(backup_dir, exist_ok=True)

# バックアップ作成
backup_path = os.path.join(backup_dir, "mod.rs.syntax.bak")
with open(file_path, "r") as src:
    with open(backup_path, "w") as dst:
        dst.write(src.read())
print(f"バックアップを作成しました: {backup_path}")

# ファイル内容を読み込む
with open(file_path, "r", encoding="utf-8") as f:
    lines = f.readlines()

# 修正1: インデントの問題を修正
modified_lines = []
for line in lines:
    # インデントが不正の閉じ括弧を修正
    if re.match(r'^    \}', line.rstrip()):
        modified_lines.append('}\n')
    else:
        modified_lines.append(line)

# 修正2: 括弧のバランスをチェック
content = ''.join(modified_lines)
opening_brackets = content.count('{')
closing_brackets = content.count('}')
print(f"開き括弧: {opening_brackets}, 閉じ括弧: {closing_brackets}")

# 修正3: 特定の構文エラー箇所を修正
# 1. build_loadとbuild_struct_gepの修正
load_pattern = r'let value = self\.builder\.build_load\(([^,]+)(?:, "load")?\);'
modified_content = re.sub(load_pattern, r'let value = self.builder.build_load(\1, "load");', content)

gep_pattern = r'self\.builder\.build_struct_gep\(([^,]+), ([^,]+), ([^,]+), ([^\)]+)\)'
modified_content = re.sub(gep_pattern, r'self.builder.build_struct_gep(\2, \3, \4)', modified_content)

# 2. match式の修正（行1943付近）
match_expr_error = r'let expr_value = self\.builder\.build_load\(lit_value, "load"\) \{'
match_expr_fix = r'let expr_value = self.builder.build_load(lit_value, "load");\n                    let cond = match (expr_value, lit_value) {'
modified_content = modified_content.replace(match_expr_error, match_expr_fix)

# 3. 構造体フィールド検索関数の修正（行1300付近）
struct_field_error = r'let value = self\.builder\.build_load\(struct_name: &str, field_name: &str\) -> Option<usize> \{'
struct_field_fix = r"""    /// 構造体のフィールドインデックスを検索
    fn find_struct_field_index(&self, struct_name: &str, field_name: &str) -> Option<usize> {"""
modified_content = modified_content.replace(struct_field_error, struct_field_fix)

# 4. 配列要素のロード関数の修正（行1352付近）
array_elements_error = r'let value = self\.builder\.build_load\(elements: &\[Expression\], "load"\) -> Result<BasicValueEnum<\'ctx>> \{'
array_elements_fix = r"""    /// 配列要素リテラルの生成
    fn generate_array_elements(&self, elements: &[Expression]) -> Result<BasicValueEnum<'ctx>> {"""
modified_content = modified_content.replace(array_elements_error, array_elements_fix)

# 5. matchパターンの修正（行1834付近）
match_patterns = [
    (r'AnyTypeEnum::from\(BasicTypeEnum::IntType\(t\) => (.*?)\)', r'AnyTypeEnum::IntType(t) => \1'),
    (r'AnyTypeEnum::from\(BasicTypeEnum::FloatType\(t\) => (.*?)\)', r'AnyTypeEnum::FloatType(t) => \1'),
    (r'AnyTypeEnum::from\(BasicTypeEnum::PointerType\(t\) => (.*?)\)', r'AnyTypeEnum::PointerType(t) => \1'),
    (r'AnyTypeEnum::from\(BasicTypeEnum::StructType\(t\) => (.*?)\)', r'AnyTypeEnum::StructType(t) => \1'),
    (r'AnyTypeEnum::from\(BasicTypeEnum::ArrayType\(t\) => (.*?)\)', r'AnyTypeEnum::ArrayType(t) => \1')
]

for pattern, replacement in match_patterns:
    modified_content = re.sub(pattern, replacement, modified_content)

# 修正をファイルに書き戻す
with open(file_path, "w", encoding="utf-8") as f:
    f.write(modified_content)

print(f"ファイル {file_path} の構文エラーを修正しました") 
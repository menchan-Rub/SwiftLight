#!/usr/bin/env python3
import os
import re

def fix_file(file_path, backup_suffix, fixes):
    """指定されたファイルを修正します。
    
    Args:
        file_path: 修正対象のファイルパス
        backup_suffix: バックアップファイルの接尾辞
        fixes: 修正内容のリスト。(行番号, 修正前の内容, 修正後の内容)のタプルで指定
    """
    # バックアップディレクトリ
    backup_dir = "crates/backups/manual"
    os.makedirs(backup_dir, exist_ok=True)
    
    # バックアップ作成
    backup_path = os.path.join(backup_dir, os.path.basename(file_path) + backup_suffix)
    with open(file_path, "r") as src:
        with open(backup_path, "w") as dst:
            dst.write(src.read())
    print(f"バックアップを作成しました: {backup_path}")
    
    # ファイルを行ごとに読み込む
    with open(file_path, "r", encoding="utf-8") as f:
        lines = f.readlines()
    
    # 修正を適用
    for line_num, old_content, new_content in fixes:
        if 0 <= line_num < len(lines):
            # 文字列置換
            if old_content and new_content:
                lines[line_num] = lines[line_num].replace(old_content, new_content)
            # 行全体を置き換え
            elif not old_content:
                lines[line_num] = new_content + '\n'
    
    # ファイルに書き戻す
    with open(file_path, "w", encoding="utf-8") as f:
        f.writelines(lines)
    
    print(f"ファイル {file_path} を修正しました")

# 修正1: codegen.rsの修正
codegen_path = "crates/swiftlight-compiler/src/backend/llvm/codegen.rs"
codegen_fixes = [
    # 1. PrimitiveType::Boolean修正
    (125, "TypeKind::Primitive(PrimitiveType::Boolean", "TypeKind::Primitive(PrimitiveType::Boolean)"),
    
    # 2. PrimitiveType::Float { bits }修正
    (130, "TypeKind::Primitive(PrimitiveType::Float) { bits }", "TypeKind::Primitive(PrimitiveType::Float { bits })"),
    
    # 3. default_valueマッチ式の修正
    (974, None, "            let default_value = match global_var.as_pointer_value().get_type().get_element_type() {"),
    (975, None, "                AnyTypeEnum::IntType(int_ty) => int_ty.const_zero(),"),
    (976, None, "                AnyTypeEnum::FloatType(float_ty) => float_ty.const_zero(),"),
    (977, None, "                AnyTypeEnum::PointerType(ptr_ty) => ptr_ty.const_null(),"),
    (978, None, "                AnyTypeEnum::StructType(struct_ty) => struct_ty.const_zero(),"),
    (979, None, "                AnyTypeEnum::ArrayType(array_ty) => array_ty.const_zero(),"),
    
    # 4. global.as_pointer_value修正 (3530行目、0-indexedなので3529)
    (3530, "global.as_pointer_value(, \"load\")", "global.as_pointer_value(), \"load\""),
]

# 修正2: wasm/codegen.rsの修正
wasm_codegen_path = "crates/swiftlight-compiler/src/backend/wasm/codegen.rs"
wasm_codegen_fixes = [
    # 1. PrimitiveType::Boolean修正
    (175, "TypeKind::Primitive(PrimitiveType::Bool)ean", "TypeKind::Primitive(PrimitiveType::Boolean)"),
]

# 修正3: utils/logger.rsの修正
logger_path = "crates/swiftlight-compiler/src/utils/logger.rs"
logger_fixes = [
    # 余分な閉じ括弧を削除
    (199, "}", "}"),
    (200, "}", ""),
]

# 各ファイルを修正
fix_file(codegen_path, ".final1.bak", codegen_fixes)
fix_file(wasm_codegen_path, ".final.bak", wasm_codegen_fixes)
fix_file(logger_path, ".final.bak", logger_fixes)

print("すべてのファイルの修正が完了しました！") 
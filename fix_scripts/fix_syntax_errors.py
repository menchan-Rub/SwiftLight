#!/usr/bin/env python3
import re
import os
import glob

# バックアップディレクトリ
backup_dir = "crates/backups/syntax"
os.makedirs(backup_dir, exist_ok=True)

# 修正対象のファイル
files_to_fix = [
    "crates/swiftlight-compiler/src/frontend/semantic/name_resolver.rs",
    "crates/swiftlight-compiler/src/frontend/semantic/type_checker.rs",
    "crates/swiftlight-compiler/src/middleend/ir/mod.rs"
]

# 各ファイルを処理
for file_path in files_to_fix:
    if not os.path.exists(file_path):
        print(f"ファイル {file_path} が見つかりません")
        continue
    
    # バックアップ作成
    backup_path = os.path.join(backup_dir, os.path.basename(file_path) + ".bak")
    with open(file_path, "r") as src:
        with open(backup_path, "w") as dst:
            dst.write(src.read())
    print(f"バックアップを作成しました: {backup_path}")
    
    # ファイル内容を読み込む
    with open(file_path, "r", encoding="utf-8") as f:
        content = f.read()
    
    # 元のコンテンツを保存
    original_content = content
    
    # 1. "ersection" の修正 (Intersection の間違い)
    content = re.sub(
        r'TypeKind::Primitive\(PrimitiveType::Int\)ersection\(types\)',
        r'TypeKind::Intersection(types)',
        content
    )
    
    # 2. IR mod.rs の閉じ括弧問題を修正
    if "middleend/ir/mod.rs" in file_path:
        # 特定のエラー部分を検出して修正
        problematic_code = r"""        if let Some(var_ptr) = self.variables.get(var_name) {
            
            // 変数の値をロード
            let value = self.builder.build_load(var_name);
            Ok(value)
        } else if let Some(func) = self.functions.get(var_name) {
            // 関数参照の場合

            // グローバル変数や定数の場合
            if let Some(global_var) = self.llvm_module.get_global(var_name) {
                let var_type = global_var.get_type().get_element_type();
                let value = self.builder.build_load(var_name);
                Ok(value)
            } else {
                Err(CompilerError::new(ErrorKind::CodeGeneration, 
                    format!("未定義の識別子 '{}'", var_name),
                    ident.location.clone()
                ))
            }
        }"""
        
        fixed_code = r"""        if let Some(var_ptr) = self.variables.get(var_name) {
            // 変数の値をロード
            let value = self.builder.build_load(var_ptr, "load");
            Ok(value)
        } else if let Some(func) = self.functions.get(var_name) {
            // 関数参照の場合
            Ok(func.as_global_value().as_pointer_value().into())
        } else if let Some(global_var) = self.llvm_module.get_global(var_name) {
            // グローバル変数や定数の場合
            let value = self.builder.build_load(global_var.as_pointer_value(), "global_load");
            Ok(value)
        } else {
            Err(CompilerError::new(ErrorKind::CodeGeneration, 
                format!("未定義の識別子 '{}'", var_name),
                ident.location.clone()
            ))
        }"""
        
        content = content.replace(problematic_code, fixed_code)
    
    # 変更があれば更新
    if content != original_content:
        with open(file_path, "w", encoding="utf-8") as f:
            f.write(content)
        print(f"ファイル {file_path} を修正しました")
    else:
        print(f"ファイル {file_path} に必要な変更はありませんでした") 
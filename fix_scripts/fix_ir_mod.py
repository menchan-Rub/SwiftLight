#!/usr/bin/env python3
import os

# 対象ファイル
file_path = "crates/swiftlight-compiler/src/middleend/ir/mod.rs"

# バックアップディレクトリ
backup_dir = "crates/backups/manual"
os.makedirs(backup_dir, exist_ok=True)

# バックアップ作成
backup_path = os.path.join(backup_dir, "mod.rs.bak")
with open(file_path, "r") as src:
    with open(backup_path, "w") as dst:
        dst.write(src.read())
print(f"バックアップを作成しました: {backup_path}")

# ファイル内容を読み込む
with open(file_path, "r", encoding="utf-8") as f:
    lines = f.readlines()

# 行番号で該当部分を特定して置き換え
start_line = 840  # 問題の開始行
end_line = 858    # 問題の終了行
new_content = """    }
    
    /// 識別子の参照を生成
    fn generate_identifier(&self, ident: &Identifier) -> Result<BasicValueEnum<'ctx>> {
        // 変数参照を生成
        let var_name = &ident.name;
        
        if let Some(var_ptr) = self.variables.get(var_name) {
            // ローカル変数の場合
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
        }
    }
    
"""

# 新しい内容に置き換える
new_lines = lines[:start_line]
new_lines.append(new_content)
new_lines.extend(lines[end_line:])

# ファイルに書き戻す
with open(file_path, "w", encoding="utf-8") as f:
    f.writelines(new_lines)

print(f"ファイル {file_path} を修正しました") 
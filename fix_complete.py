import re
import os

target_file = "crates/swiftlight-compiler/src/middleend/ir/mod.rs"
backup_file = target_file + ".final-bak"

# バックアップ作成
if not os.path.exists(backup_file):
    with open(target_file, "r") as src:
        with open(backup_file, "w") as dst:
            dst.write(src.read())
    print(f"バックアップを作成しました: {backup_file}")

# ファイル内容を読み込み
with open(target_file, "r") as f:
    content = f.read()

# 関数の引数パターン修正
# 1. build_struct_gep関数の修正
pattern1 = r'build_struct_gep\(([^,]+),\s*([^,]+),\s*([^,]+),\s*([^,\)]+)\)'
replacement1 = r'build_struct_gep(\3, \4)'

# build_load関数の修正
pattern2 = r'build_load\(([^,]+),\s*([^,]+),\s*([^,\)]+)\)'
replacement2 = r'build_load(\2, \3)'

# 不正なパターンの修正
pattern3 = r'build_load\("([^"]+)"\)'
replacement3 = r'build_load(expr_ptr, "\1")'

# 修正を適用
modified_content = re.sub(pattern1, replacement1, content)
modified_content = re.sub(pattern2, replacement2, modified_content)
modified_content = re.sub(pattern3, replacement3, modified_content)

# ファイルの末尾が切れているかどうかをチェック
if not modified_content.strip().endswith("}"):
    # 適切な閉じ括弧を追加
    modified_content += """
                            }
                        }
                    }
                    
                    // パターンブロックの生成
                    self.builder.position_at_end(pattern_block);
                    
                    // アームの本体を評価
                    let arm_result = self.generate_expression(arm_body)?;
                    
                    // マージブロックに分岐
                    self.builder.build_unconditional_branch(merge_block);
                    
                    // PHI節点用に現在のブロックと値を記録
                    incoming_values.push(arm_result);
                    incoming_blocks.push(self.builder.get_insert_block().unwrap());
                    
                    // 次のブロックに位置を更新
                    current_block = next_pattern_block;
                }
                
                // デフォルトブロックの生成（マッチしなかった場合）
                self.builder.position_at_end(default_block);
                
                // デフォルトの処理（ここではエラーを返す）
                let error_msg = "どのパターンにもマッチしませんでした";
                let err_result = self.build_runtime_error(error_msg, None)?;
                self.builder.build_unreachable();
                
                // マージブロックの生成
                self.builder.position_at_end(merge_block);
                
                // PHI節点が必要な場合
                if incoming_values.len() > 0 {
                    let result_type = incoming_values[0].get_type();
                    let phi = self.builder.build_phi(result_type, "matchresult");
                    
                    for (i, value) in incoming_values.iter().enumerate() {
                        phi.add_incoming(&[(&value, incoming_blocks[i])]);
                    }
                    
                    Ok(phi.as_basic_value())
                } else {
                    // すべてエラーの場合は適当な値を返す（実行されることはない）
                    Ok(self.context.i32_type().const_zero().into())
                }
            }
        }
    }
}"""

# 変更内容を書き戻す
with open(target_file, "w") as f:
    f.write(modified_content)

print(f"ファイルの修正が完了しました: {target_file}") 
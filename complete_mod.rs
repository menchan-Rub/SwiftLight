// ファイルを修復するための閉じ括弧
                                    }
                                };
                                
                                // 条件分岐
                                self.builder.build_conditional_branch(cond, pattern_block, next_pattern_block);
                            }
                        }
                        _ => {
                            // その他のパターンは未サポート
                            self.builder.build_unconditional_branch(next_pattern_block);
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
} 
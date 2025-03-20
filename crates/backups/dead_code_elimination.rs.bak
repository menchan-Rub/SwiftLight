// SwiftLight 最適化パス - デッドコード除去
//
// このモジュールは、使用されない命令や到達不能なコードを除去する最適化を実装します。
// 主な役割：
// - 未使用の変数や命令の削除
// - 到達不能なコードブロックの削除
// - 未使用の関数の削除

use crate::frontend::error::Result;
use crate::middleend::ir::{Module, Function, Instruction, BasicBlock, Value, Type};
use std::collections::{HashSet, HashMap, VecDeque};

/// モジュールに対してデッドコード除去最適化を実行します
pub fn run(module: &mut Module) -> Result<()> {
    // 各関数に対してデッドコード除去を適用
    for function in &mut module.functions {
        eliminate_dead_code_in_function(function);
    }
    
    // 未使用の関数を特定
    let mut used_functions = HashSet::new();
    // mainやエクスポートされた関数はルート関数と見なす
    for function in &module.functions {
        if function.is_exported || function.name == "main" {
            used_functions.insert(function.id);
            mark_called_functions(function, &module.functions, &mut used_functions);
        }
    }
    
    // 未使用関数をフィルタリング
    module.functions.retain(|f| used_functions.contains(&f.id));
    
    Ok(())
}

/// 関数から呼び出される全ての関数を再帰的にマーク
fn mark_called_functions(function: &Function, all_functions: &[Function], used: &mut HashSet<u32>) {
    for block in &function.blocks {
        for instruction in &block.instructions {
            if let Instruction::Call { function_id, .. } = instruction {
                if !used.contains(function_id) {
                    used.insert(*function_id);
                    if let Some(called_function) = all_functions.iter().find(|f| f.id == *function_id) {
                        mark_called_functions(called_function, all_functions, used);
                    }
                }
            }
        }
    }
}

/// 関数内のデッドコードを除去します
fn eliminate_dead_code_in_function(function: &mut Function) {
    // 1. 使用されている変数を特定
    let mut used_values = HashSet::new();
    
    // 戻り値、条件分岐、ストア操作などの副作用を持つ命令は常に「使用されている」とマーク
    for block in &function.blocks {
        for instruction in &block.instructions {
            match instruction {
                Instruction::Return { value, .. } => {
                    if let Some(val) = value {
                        used_values.insert(*val);
                    }
                },
                Instruction::Branch { condition, .. } => {
                    used_values.insert(*condition);
                },
                Instruction::Store { value, .. } => {
                    used_values.insert(*value);
                },
                Instruction::Call { .. } => {
                    // 呼び出しは副作用を持つ可能性があるため保持
                    // 呼び出しの結果が使用されていなくても保持
                },
                _ => {}
            }
        }
    }
    
    // 2. 使用されている値に依存する全ての値を再帰的にマーク
    let mut work_list: VecDeque<u32> = used_values.iter().cloned().collect();
    
    while let Some(value_id) = work_list.pop_front() {
        // この値が依存する他の値を見つける
        for block in &function.blocks {
            for instruction in &block.instructions {
                if let Some((result_id, operands)) = get_instruction_result_and_operands(instruction) {
                    if result_id == value_id {
                        for &operand in operands {
                            if !used_values.contains(&operand) {
                                used_values.insert(operand);
                                work_list.push_back(operand);
                            }
                        }
                    }
                }
            }
        }
    }
    
    // 3. 使用されていない命令を削除
    for block in &mut function.blocks {
        block.instructions.retain(|inst| {
            if let Some((result_id, _)) = get_instruction_result_and_operands(inst) {
                // 結果が使用されているか、副作用を持つ命令は保持
                used_values.contains(&result_id) || has_side_effect(inst)
            } else {
                // 結果を生成しない命令は副作用があるものだけ保持
                has_side_effect(inst)
            }
        });
    }
    
    // 4. 空のブロックや到達不能なブロックを削除
    eliminate_unreachable_blocks(function);
}

/// 命令から結果IDとオペランドのリストを取得
fn get_instruction_result_and_operands(instruction: &Instruction) -> Option<(u32, Vec<u32>)> {
    match instruction {
        Instruction::BinaryOp { result, left, right, .. } => {
            Some((*result, vec![*left, *right]))
        },
        Instruction::UnaryOp { result, operand, .. } => {
            Some((*result, vec![*operand]))
        },
        Instruction::Load { result, address, .. } => {
            Some((*result, vec![*address]))
        },
        Instruction::GetElementPtr { result, base, indices, .. } => {
            let mut operands = vec![*base];
            operands.extend_from_slice(indices);
            Some((*result, operands))
        },
        Instruction::Call { result, arguments, .. } => {
            if let Some(res) = result {
                Some((*res, arguments.clone()))
            } else {
                None
            }
        },
        Instruction::Phi { result, incoming, .. } => {
            let values: Vec<u32> = incoming.iter().map(|(val, _)| *val).collect();
            Some((*result, values))
        },
        Instruction::Alloca { result, .. } => {
            Some((*result, vec![]))
        },
        Instruction::Cast { result, value, .. } => {
            Some((*result, vec![*value]))
        },
        // その他の命令は必要に応じて追加
        _ => None,
    }
}

/// 命令が副作用を持つかどうかを判定
fn has_side_effect(instruction: &Instruction) -> bool {
    match instruction {
        Instruction::Store { .. } => true,
        Instruction::Call { .. } => true,
        Instruction::Return { .. } => true,
        Instruction::Branch { .. } => true,
        Instruction::Jump { .. } => true,
        // その他の副作用を持つ命令があれば追加
        _ => false,
    }
}

/// 到達不能なブロックを除去
fn eliminate_unreachable_blocks(function: &mut Function) {
    // 到達可能なブロックを特定
    let mut reachable_blocks = HashSet::new();
    let mut work_list = VecDeque::new();
    
    // エントリーブロックは常に到達可能
    if !function.blocks.is_empty() {
        let entry_block_id = function.blocks[0].id;
        reachable_blocks.insert(entry_block_id);
        work_list.push_back(entry_block_id);
    }
    
    // 制御フローグラフをたどって到達可能なブロックをマーク
    while let Some(block_id) = work_list.pop_front() {
        let block_index = function.blocks.iter().position(|b| b.id == block_id).unwrap();
        let block = &function.blocks[block_index];
        
        // 最後の命令を確認
        if let Some(last_instruction) = block.instructions.last() {
            match last_instruction {
                Instruction::Branch { true_block, false_block, .. } => {
                    // 条件分岐がある場合、両方の分岐先を追加
                    if !reachable_blocks.contains(true_block) {
                        reachable_blocks.insert(*true_block);
                        work_list.push_back(*true_block);
                    }
                    if !reachable_blocks.contains(false_block) {
                        reachable_blocks.insert(*false_block);
                        work_list.push_back(*false_block);
                    }
                },
                Instruction::Jump { target, .. } => {
                    // 無条件ジャンプの場合、ジャンプ先を追加
                    if !reachable_blocks.contains(target) {
                        reachable_blocks.insert(*target);
                        work_list.push_back(*target);
                    }
                },
                Instruction::Return { .. } => {
                    // 戻り命令の場合、後続ブロックなし
                },
                _ => {
                    // 制御フロー命令がない場合、フォールスルーする可能性がある
                    // 次のブロックが存在すれば追加
                    if block_index + 1 < function.blocks.len() {
                        let next_block_id = function.blocks[block_index + 1].id;
                        if !reachable_blocks.contains(&next_block_id) {
                            reachable_blocks.insert(next_block_id);
                            work_list.push_back(next_block_id);
                        }
                    }
                }
            }
        }
    }
    
    // 到達不能なブロックを削除
    function.blocks.retain(|block| reachable_blocks.contains(&block.id));
}

// SwiftLight 最適化パス - 関数インライン化
//
// このモジュールは、関数呼び出しを呼び出し先の関数本体で置き換えるインライン展開を実装します。
// 主な役割：
// - 小さな関数の呼び出しオーバーヘッドを削減
// - 最適化の機会を増やす（インライン展開後の定数伝播など）
// - 再帰呼び出しの制限付きインライン展開

use crate::frontend::error::Result;
use crate::middleend::ir::{Module, Function, BasicBlock, Instruction, Value, Type};
use std::collections::{HashMap, HashSet};

// インライン化の閾値定数
// これらの値は調整可能
const MAX_FUNCTION_SIZE: usize = 50; // 命令数が50以下の関数をインライン化
const MAX_RECURSIVE_DEPTH: usize = 2; // 再帰呼び出しの最大インライン深さ
const MAX_INLINE_INSTANCES: usize = 10; // 一つの関数をインライン化する最大インスタンス数

/// モジュールに対して関数インライン化最適化を実行します
pub fn run(module: &mut Module) -> Result<()> {
    // 各関数の呼び出し回数を集計
    let mut call_counts: HashMap<u32, usize> = HashMap::new();
    count_function_calls(module, &mut call_counts);
    
    // インライン化対象の関数を特定
    let mut inline_candidates: HashMap<u32, bool> = HashMap::new();
    for function in &module.functions {
        // インライン化条件の評価:
        // 1. 関数が十分に小さい
        // 2. 再帰関数でない、または制限付き再帰
        let size = function_size(function);
        let is_recursive = is_recursive_function(function);
        let calls = call_counts.get(&function.id).copied().unwrap_or(0);
        
        // 小さな関数または中程度のサイズで頻繁に呼ばれる関数を選択
        let should_inline = size <= MAX_FUNCTION_SIZE || 
                            (size <= MAX_FUNCTION_SIZE * 2 && calls >= 3);
        
        inline_candidates.insert(function.id, should_inline && !is_recursive);
    }
    
    // インライン化の実行
    let mut changed = true;
    let mut iteration = 0;
    
    // 固定ポイントまで繰り返す（最大でも数回）
    while changed && iteration < 3 {
        changed = false;
        iteration += 1;
        
        // 各関数に対してインライン化を適用
        for func_idx in 0..module.functions.len() {
            let function_id = module.functions[func_idx].id;
            let mut inlined_instances = 0;
            
            // 各基本ブロックをクローンする必要があるため、別の変数で関数を参照
            let function = &mut module.functions[func_idx];
            
            // 各基本ブロックを処理
            for block_idx in 0..function.blocks.len() {
                let block = &mut function.blocks[block_idx];
                
                // インライン化対象の呼び出し命令を探す
                let mut i = 0;
                while i < block.instructions.len() {
                    if let Instruction::Call { function_id: callee_id, arguments, result, .. } = &block.instructions[i] {
                        // 呼び出し先関数がインライン化対象かどうか確認
                        if let Some(true) = inline_candidates.get(callee_id) {
                            // 呼び出し先関数を取得
                            if let Some(callee_idx) = module.functions.iter().position(|f| f.id == *callee_id) {
                                let callee = &module.functions[callee_idx];
                                
                                // 適切な条件下でインライン化を実行
                                if inlined_instances < MAX_INLINE_INSTANCES && 
                                   inline_function_call(function, block_idx, i, callee, arguments, *result) {
                                    changed = true;
                                    inlined_instances += 1;
                                    continue; // インライン化後は同じインデックスを再チェック
                                }
                            }
                        }
                    }
                    i += 1;
                }
            }
        }
    }
    
    Ok(())
}

/// 関数のサイズを命令数で計算
fn function_size(function: &Function) -> usize {
    function.blocks.iter().map(|block| block.instructions.len()).sum()
}

/// 関数が直接的または間接的に自身を呼び出すかどうかを判定
fn is_recursive_function(function: &Function) -> bool {
    // 単純な直接再帰チェック
    for block in &function.blocks {
        for instruction in &block.instructions {
            if let Instruction::Call { function_id, .. } = instruction {
                if *function_id == function.id {
                    return true;
                }
            }
        }
    }
    false // より複雑な間接再帰は呼び出しグラフ分析が必要
}

/// 呼び出し命令を呼び出し先関数の本体でインライン展開する
fn inline_function_call(
    caller: &mut Function,
    block_idx: usize,
    instruction_idx: usize,
    callee: &Function,
    arguments: &[u32],
    result: Option<u32>
) -> bool {
    // 呼び出し先が空の関数の場合は単にスキップ
    if callee.blocks.is_empty() {
        return false;
    }
    
    // 値のリマップテーブル（呼び出し先の値ID → インライン展開後の値ID）
    let mut value_map: HashMap<u32, u32> = HashMap::new();
    
    // 引数値をマッピング
    for (param_idx, &arg_value) in arguments.iter().enumerate() {
        if param_idx < callee.parameters.len() {
            let param_id = callee.parameters[param_idx];
            value_map.insert(param_id, arg_value);
        }
    }
    
    // 呼び出し先の基本ブロックをコピーしてIDをリマップ
    let mut block_map: HashMap<u32, u32> = HashMap::new();
    let mut new_blocks: Vec<BasicBlock> = Vec::new();
    
    // 新しいブロックIDの生成
    let mut next_block_id = caller.next_block_id();
    
    // 呼び出し先の各ブロックをクローン
    for callee_block in &callee.blocks {
        let new_block_id = next_block_id;
        next_block_id += 1;
        
        block_map.insert(callee_block.id, new_block_id);
        
        // 新しいブロックを作成（命令はまだコピーしない）
        let mut new_block = BasicBlock {
            id: new_block_id,
            name: format!("{}_inlined_{}", callee_block.name, caller.blocks[block_idx].id),
            instructions: Vec::new(),
        };
        
        new_blocks.push(new_block);
    }
    
    // 現在のブロックを分割
    // 呼び出し前の命令、インライン展開された関数本体、呼び出し後の命令
    
    // 呼び出し命令の後の命令を取得
    let current_block = &mut caller.blocks[block_idx];
    let mut instructions_after_call: Vec<Instruction> = current_block.instructions.drain(instruction_idx + 1..).collect();
    
    // 呼び出し命令自体を削除
    current_block.instructions.remove(instruction_idx);
    
    // インライン展開された関数からの戻り先ブロック
    let return_block_id = if !instructions_after_call.is_empty() {
        let return_block_id = next_block_id;
        next_block_id += 1;
        
        // 戻り先ブロックを作成
        let return_block = BasicBlock {
            id: return_block_id,
            name: format!("{}_after_inlined", current_block.name),
            instructions: instructions_after_call,
        };
        
        new_blocks.push(return_block);
        Some(return_block_id)
    } else {
        None
    };
    
    // 呼び出し先関数の命令をコピーしてリマップ
    for (i, callee_block) in callee.blocks.iter().enumerate() {
        let new_block_id = block_map[&callee_block.id];
        let new_block = &mut new_blocks[i];
        
        // 命令をコピーしてリマップ
        for instruction in &callee_block.instructions {
            let mut new_instruction = instruction.clone();
            
            // 命令を変換
            match &mut new_instruction {
                Instruction::BinaryOp { result, left, right, .. } => {
                    *result = get_or_create_mapped_value(result, &mut value_map, caller);
                    *left = get_mapped_value(left, &value_map);
                    *right = get_mapped_value(right, &value_map);
                },
                Instruction::UnaryOp { result, operand, .. } => {
                    *result = get_or_create_mapped_value(result, &mut value_map, caller);
                    *operand = get_mapped_value(operand, &value_map);
                },
                Instruction::Load { result, address, .. } => {
                    *result = get_or_create_mapped_value(result, &mut value_map, caller);
                    *address = get_mapped_value(address, &value_map);
                },
                Instruction::Store { address, value, .. } => {
                    *address = get_mapped_value(address, &value_map);
                    *value = get_mapped_value(value, &value_map);
                },
                Instruction::GetElementPtr { result, base, indices, .. } => {
                    *result = get_or_create_mapped_value(result, &mut value_map, caller);
                    *base = get_mapped_value(base, &value_map);
                    for index in indices.iter_mut() {
                        *index = get_mapped_value(index, &value_map);
                    }
                },
                Instruction::Call { result, arguments, .. } => {
                    if let Some(res) = result {
                        *res = get_or_create_mapped_value(res, &mut value_map, caller);
                    }
                    for arg in arguments.iter_mut() {
                        *arg = get_mapped_value(arg, &value_map);
                    }
                },
                Instruction::Alloca { result, .. } => {
                    *result = get_or_create_mapped_value(result, &mut value_map, caller);
                },
                Instruction::Branch { condition, true_block, false_block, .. } => {
                    *condition = get_mapped_value(condition, &value_map);
                    *true_block = block_map.get(true_block).copied().unwrap_or(*true_block);
                    *false_block = block_map.get(false_block).copied().unwrap_or(*false_block);
                },
                Instruction::Jump { target, .. } => {
                    *target = block_map.get(target).copied().unwrap_or(*target);
                },
                Instruction::Return { value, .. } => {
                    // 戻り値をリマップ
                    if let Some(val) = value {
                        *val = get_mapped_value(val, &value_map);
                    }
                    
                    // Returnを戻り先ブロックへのJumpに変換
                    if let Some(return_block_id) = return_block_id {
                        if let Some(val) = value {
                            if let Some(ret_val) = result {
                                // 戻り値の代入を追加
                                new_block.instructions.push(Instruction::Store {
                                    address: ret_val,
                                    value: *val,
                                    align: 8, // 適切なアラインメントを設定
                                });
                            }
                        }
                        
                        // Jumpに置き換え
                        new_instruction = Instruction::Jump {
                            target: return_block_id,
                        };
                    } else {
                        // 戻り先ブロックがない場合は、元のリターン命令をそのまま使用
                        if let Some(val) = value {
                            if let Some(ret_val) = result {
                                // 戻り値の代入を追加
                                new_block.instructions.push(Instruction::Store {
                                    address: ret_val,
                                    value: *val,
                                    align: 8,
                                });
                            }
                        }
                    }
                },
                Instruction::Phi { result, incoming, .. } => {
                    *result = get_or_create_mapped_value(result, &mut value_map, caller);
                    for (val, block) in incoming.iter_mut() {
                        *val = get_mapped_value(val, &value_map);
                        *block = block_map.get(block).copied().unwrap_or(*block);
                    }
                },
                Instruction::Cast { result, value, .. } => {
                    *result = get_or_create_mapped_value(result, &mut value_map, caller);
                    *value = get_mapped_value(value, &value_map);
                },
                // その他の命令は必要に応じて追加
                _ => {}
            }
            
            new_block.instructions.push(new_instruction);
        }
    }
    
    // 元のブロックから呼び出し先の最初のブロックへジャンプを追加
    let entry_block_id = block_map[&callee.blocks[0].id];
    caller.blocks[block_idx].instructions.push(Instruction::Jump {
        target: entry_block_id,
    });
    
    // 新しいブロックを呼び出し元関数に追加
    caller.blocks.extend(new_blocks);
    
    true
}

/// 呼び出し先関数の値IDを変換、マップにない場合は新しい値を作成
fn get_or_create_mapped_value(value_id: &u32, value_map: &mut HashMap<u32, u32>, function: &mut Function) -> u32 {
    if let Some(&mapped_id) = value_map.get(value_id) {
        mapped_id
    } else {
        let new_id = function.next_value_id();
        value_map.insert(*value_id, new_id);
        new_id
    }
}

/// 呼び出し先関数の値IDを変換、マップにある場合のみ
fn get_mapped_value(value_id: &u32, value_map: &HashMap<u32, u32>) -> u32 {
    *value_map.get(value_id).unwrap_or(value_id)
}

/// モジュール内の各関数の呼び出し回数を集計
fn count_function_calls(module: &Module, call_counts: &mut HashMap<u32, usize>) {
    for function in &module.functions {
        for block in &function.blocks {
            for instruction in &block.instructions {
                if let Instruction::Call { function_id, .. } = instruction {
                    *call_counts.entry(*function_id).or_insert(0) += 1;
                }
            }
        }
    }
}

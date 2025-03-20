// SwiftLight 最適化パス - ループ最適化
//
// このモジュールは、ループに関する様々な最適化を実装します。
// 主な役割：
// - ループ不変コード移動（Loop Invariant Code Motion, LICM）
// - ループ展開（Loop Unrolling）
// - ループ融合（Loop Fusion）
// - ループ分割（Loop Splitting）

use crate::frontend::error::Result;
use crate::middleend::ir::{Module, Function, BasicBlock, Instruction, Value, Type, BinaryOp};
use std::collections::{HashMap, HashSet, VecDeque};

/// モジュールに対してループ最適化を実行します
pub fn run(module: &mut Module) -> Result<()> {
    for function in &mut module.functions {
        // ループを検出
        let loops = detect_loops(function);
        
        // 各ループに対して最適化を適用
        for loop_info in &loops {
            // ループ不変コード移動
            perform_licm(function, loop_info);
            
            // ループ展開（小さなループや固定回数のループに対して）
            if should_unroll_loop(function, loop_info) {
                unroll_loop(function, loop_info);
            }
        }
        
        // 隣接するループの融合（ループ融合）
        fuse_adjacent_loops(function, &loops);
    }
    
    Ok(())
}

/// ループ情報を表す構造体
#[derive(Debug, Clone)]
struct LoopInfo {
    /// ヘッダブロックID（ループの入口）
    header: u32,
    
    /// ループに含まれる全ブロックID
    blocks: HashSet<u32>,
    
    /// ループの出口ブロックID（ループから抜ける分岐を持つブロック）
    exit_blocks: HashSet<u32>,
    
    /// ループの前のブロックID（プリヘッダ）
    preheader: Option<u32>,
    
    /// 帰納変数（イテレータ変数）とその増減パターン
    induction_vars: HashMap<u32, InductionPattern>,
}

/// 帰納変数の増減パターン
#[derive(Debug, Clone)]
enum InductionPattern {
    /// 線形増加/減少: 変数 += 定数
    Linear {
        /// 初期値（定数またはループ外で定義された値）
        init_value: u32,
        
        /// ステップ値（加算/減算される定数）
        step: i32,
        
        /// 更新命令ID
        update_instruction: u32,
    },
    
    /// その他/不明なパターン
    Unknown,
}

/// 関数内のループを検出
fn detect_loops(function: &Function) -> Vec<LoopInfo> {
    let mut loops = Vec::new();
    
    // 制御フローグラフ（CFG）の構築
    let cfg = build_cfg(function);
    
    // 深さ優先探索でバックエッジを検出（ループヘッダを特定）
    let mut visited = HashSet::new();
    let mut stack = HashSet::new();
    
    fn dfs(
        node: u32,
        cfg: &HashMap<u32, Vec<u32>>,
        visited: &mut HashSet<u32>,
        stack: &mut HashSet<u32>,
        back_edges: &mut Vec<(u32, u32)>,
    ) {
        visited.insert(node);
        stack.insert(node);
        
        if let Some(successors) = cfg.get(&node) {
            for &succ in successors {
                if !visited.contains(&succ) {
                    dfs(succ, cfg, visited, stack, back_edges);
                } else if stack.contains(&succ) {
                    // バックエッジを検出（from -> to）
                    back_edges.push((node, succ));
                }
            }
        }
        
        stack.remove(&node);
    }
    
    let mut back_edges = Vec::new();
    if let Some(entry) = function.blocks.first() {
        dfs(entry.id, &cfg, &mut visited, &mut stack, &mut back_edges);
    }
    
    // 各バックエッジに対応するループを特定
    for (from, to) in back_edges {
        // ループヘッダはバックエッジの宛先
        let header = to;
        
        // ループに含まれるブロックを特定（ヘッダからバックエッジの発生元までの全ブロック）
        let mut loop_blocks = HashSet::new();
        loop_blocks.insert(header);
        
        // 逆方向DFSでループブロックを特定
        let mut work_list = VecDeque::new();
        work_list.push_back(from);
        
        while let Some(block_id) = work_list.pop_front() {
            if loop_blocks.insert(block_id) {
                // ブロックの前任者をワークリストに追加
                if let Some(predecessors) = cfg.get(&block_id) {
                    for &pred in predecessors {
                        if pred != header && !loop_blocks.contains(&pred) {
                            work_list.push_back(pred);
                        }
                    }
                }
            }
        }
        
        // 出口ブロックを特定（ループ内から外へのエッジを持つブロック）
        let mut exit_blocks = HashSet::new();
        for &block_id in &loop_blocks {
            if let Some(successors) = cfg.get(&block_id) {
                for &succ in successors {
                    if !loop_blocks.contains(&succ) {
                        exit_blocks.insert(block_id);
                        break;
                    }
                }
            }
        }
        
        // プリヘッダブロックを特定（ループヘッダへ入るループ外のブロック）
        let mut preheader = None;
        if let Some(predecessors) = cfg.get(&header) {
            for &pred in predecessors {
                if !loop_blocks.contains(&pred) {
                    preheader = Some(pred);
                    break;
                }
            }
        }
        
        // 帰納変数の特定
        let induction_vars = identify_induction_variables(function, &loop_blocks, header);
        
        loops.push(LoopInfo {
            header,
            blocks: loop_blocks,
            exit_blocks,
            preheader,
            induction_vars,
        });
    }
    
    loops
}

/// 制御フローグラフ（CFG）の構築
fn build_cfg(function: &Function) -> HashMap<u32, Vec<u32>> {
    let mut cfg: HashMap<u32, Vec<u32>> = HashMap::new();
    
    for block in &function.blocks {
        let mut successors = Vec::new();
        
        // ブロックの最後の命令から後続ブロックを特定
        if let Some(last_instruction) = block.instructions.last() {
            match last_instruction {
                Instruction::Branch { true_block, false_block, .. } => {
                    successors.push(*true_block);
                    successors.push(*false_block);
                },
                Instruction::Jump { target, .. } => {
                    successors.push(*target);
                },
                Instruction::Return { .. } => {
                    // 戻り命令の場合は後続ブロックなし
                },
                _ => {
                    // 制御フロー命令がない場合、次のブロックにフォールスルー
                    if let Some(next_block) = function.blocks.iter().find(|b| 
                        function.blocks.iter().position(|bb| bb.id == block.id).unwrap() + 1 ==
                        function.blocks.iter().position(|bb| bb.id == b.id).unwrap()
                    ) {
                        successors.push(next_block.id);
                    }
                },
            }
        }
        
        cfg.insert(block.id, successors);
    }
    
    // 前任者リストも構築（逆CFG）
    let mut reverse_cfg: HashMap<u32, Vec<u32>> = HashMap::new();
    for (block_id, successors) in &cfg {
        for &succ in successors {
            reverse_cfg.entry(succ).or_insert_with(Vec::new).push(*block_id);
        }
    }
    
    cfg
}

/// ループ内の帰納変数を特定
fn identify_induction_variables(
    function: &Function,
    loop_blocks: &HashSet<u32>,
    header: u32
) -> HashMap<u32, InductionPattern> {
    let mut induction_vars = HashMap::new();
    
    // ヘッダブロックのPhi命令を探す
    let header_block = function.blocks.iter().find(|b| b.id == header).unwrap();
    
    for instruction in &header_block.instructions {
        if let Instruction::Phi { result, incoming, .. } = instruction {
            // Phi命令の入力を分析
            let mut init_value = None;
            let mut update_instruction = None;
            
            for &(value, block_id) in incoming {
                if !loop_blocks.contains(&block_id) {
                    // ループ外からの値は初期値
                    init_value = Some(value);
                } else {
                    // ループ内からの値は更新値
                    // この値がどのように生成されるかを特定
                    let block = function.blocks.iter().find(|b| b.id == block_id).unwrap();
                    for (i, inst) in block.instructions.iter().enumerate() {
                        if let Instruction::BinaryOp { result: r, op, left, right, .. } = inst {
                            if *r == value {
                                // 加算/減算パターンを確認
                                if (*op == BinaryOp::Add || *op == BinaryOp::Sub) && 
                                   (*left == *result || *right == *result) {
                                    let other = if *left == *result { *right } else { *left };
                                    // ステップ値が定数であるか確認
                                    if is_constant(function, other) {
                                        update_instruction = Some(i as u32);
                                        
                                        // ステップ値を抽出
                                        let step_value = get_constant_value(function, other).unwrap_or(1);
                                        let step = if *op == BinaryOp::Add { 
                                            step_value 
                                        } else { 
                                            -step_value 
                                        };
                                        
                                        if let Some(init) = init_value {
                                            induction_vars.insert(*result, InductionPattern::Linear {
                                                init_value: init,
                                                step,
                                                update_instruction: i as u32,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    induction_vars
}

/// 値が定数かどうかを判定
fn is_constant(function: &Function, value_id: u32) -> bool {
    for block in &function.blocks {
        for instruction in &block.instructions {
            if let Instruction::Constant { result, .. } = instruction {
                if *result == value_id {
                    return true;
                }
            }
        }
    }
    false
}

/// 定数値を取得
fn get_constant_value(function: &Function, value_id: u32) -> Option<i32> {
    for block in &function.blocks {
        for instruction in &block.instructions {
            if let Instruction::Constant { result, value, .. } = instruction {
                if *result == value_id {
                    // 値の型に応じて適切に変換
                    // 実際の実装では型に合わせて適切に処理する必要がある
                    return Some(*value as i32);
                }
            }
        }
    }
    None
}

/// ループ不変コード移動（LICM）の実行
fn perform_licm(function: &mut Function, loop_info: &LoopInfo) {
    // ループ内の命令を分析してループ不変コードを特定
    let mut invariant_instructions: Vec<(usize, usize, Instruction)> = Vec::new();
    
    // ループ内で定義される値のセット
    let mut loop_defined_values = HashSet::new();
    for &block_id in &loop_info.blocks {
        if let Some(block_idx) = function.blocks.iter().position(|b| b.id == block_id) {
            let block = &function.blocks[block_idx];
            for instruction in &block.instructions {
                if let Some(result_id) = get_instruction_result(instruction) {
                    loop_defined_values.insert(result_id);
                }
            }
        }
    }
    
    // ループ不変命令を特定
    for &block_id in &loop_info.blocks {
        if let Some(block_idx) = function.blocks.iter().position(|b| b.id == block_id) {
            let block = &function.blocks[block_idx];
            for (inst_idx, instruction) in block.instructions.iter().enumerate() {
                if is_loop_invariant(instruction, &loop_defined_values) && 
                   !has_side_effect(instruction) && 
                   instruction_can_be_moved(instruction) {
                    // ループ不変命令をマーク
                    invariant_instructions.push((block_idx, inst_idx, instruction.clone()));
                }
            }
        }
    }
    
    // プリヘッダブロックが存在しない場合は作成
    let preheader_id = if let Some(preheader) = loop_info.preheader {
        preheader
    } else {
        // プリヘッダブロックの作成（実際の実装では他の方法が必要かもしれません）
        // この例では単純化のために省略
        return;
    };
    
    // プリヘッダブロックを取得
    let preheader_idx = function.blocks.iter().position(|b| b.id == preheader_id).unwrap();
    
    // ループ不変命令をプリヘッダに移動
    // 注意: ここでは命令の依存関係を考慮せず単純に移動しています
    // 実際の実装では依存グラフに基づいて移動順序を決定する必要があります
    let mut instructions_to_remove: Vec<(usize, usize)> = Vec::new();
    
    for (block_idx, inst_idx, instruction) in invariant_instructions {
        // プリヘッダの最後（ジャンプ命令の前）に命令を挿入
        let preheader = &mut function.blocks[preheader_idx];
        let insert_pos = preheader.instructions.len() - 1;
        preheader.instructions.insert(insert_pos, instruction);
        
        // 移動した命令を削除リストに追加
        instructions_to_remove.push((block_idx, inst_idx));
    }
    
    // 命令を削除（後ろから削除して添字が変わらないようにする）
    instructions_to_remove.sort_by(|a, b| b.cmp(a));
    for (block_idx, inst_idx) in instructions_to_remove {
        function.blocks[block_idx].instructions.remove(inst_idx);
    }
}

/// 命令がループ不変かどうかを判定
fn is_loop_invariant(instruction: &Instruction, loop_defined_values: &HashSet<u32>) -> bool {
    // 命令のオペランドがループ内で定義されていなければループ不変
    match instruction {
        Instruction::BinaryOp { left, right, .. } => {
            !loop_defined_values.contains(left) && !loop_defined_values.contains(right)
        },
        Instruction::UnaryOp { operand, .. } => {
            !loop_defined_values.contains(operand)
        },
        Instruction::Load { address, .. } => {
            // ロード命令の場合、アドレスがループ不変であればよい
            // ただし、ループ内でストア命令が存在する場合は注意が必要
            !loop_defined_values.contains(address)
        },
        Instruction::GetElementPtr { base, indices, .. } => {
            !loop_defined_values.contains(base) && 
            indices.iter().all(|idx| !loop_defined_values.contains(idx))
        },
        Instruction::Cast { value, .. } => {
            !loop_defined_values.contains(value)
        },
        Instruction::Call { arguments, .. } => {
            // 呼び出しは副作用の可能性があるため移動は慎重に
            arguments.iter().all(|arg| !loop_defined_values.contains(arg))
        },
        Instruction::Constant { .. } => true,
        // その他の命令は適宜判定
        _ => false,
    }
}

/// 命令が副作用を持つかどうかを判定
fn has_side_effect(instruction: &Instruction) -> bool {
    match instruction {
        Instruction::Store { .. } => true,
        Instruction::Call { .. } => true, // 一般的には全ての呼び出しは副作用があると見なす
        Instruction::Return { .. } => true,
        Instruction::Branch { .. } => true,
        Instruction::Jump { .. } => true,
        _ => false,
    }
}

/// 命令が安全に移動可能かどうかを判定
fn instruction_can_be_moved(instruction: &Instruction) -> bool {
    match instruction {
        Instruction::Phi { .. } => false, // Phi命令は特定のブロックに結びついている
        Instruction::Branch { .. } => false,
        Instruction::Jump { .. } => false,
        Instruction::Return { .. } => false,
        Instruction::Store { .. } => false, // ストア命令は通常移動しない
        _ => true,
    }
}

/// 命令から結果IDを取得
fn get_instruction_result(instruction: &Instruction) -> Option<u32> {
    match instruction {
        Instruction::BinaryOp { result, .. } => Some(*result),
        Instruction::UnaryOp { result, .. } => Some(*result),
        Instruction::Load { result, .. } => Some(*result),
        Instruction::GetElementPtr { result, .. } => Some(*result),
        Instruction::Call { result, .. } => *result,
        Instruction::Phi { result, .. } => Some(*result),
        Instruction::Alloca { result, .. } => Some(*result),
        Instruction::Cast { result, .. } => Some(*result),
        Instruction::Constant { result, .. } => Some(*result),
        _ => None,
    }
}

/// ループを展開すべきかどうかを判定
fn should_unroll_loop(function: &Function, loop_info: &LoopInfo) -> bool {
    // ループサイズが小さい場合は展開を検討
    let loop_size = loop_info.blocks.iter()
        .map(|&id| function.blocks.iter().find(|b| b.id == id).unwrap().instructions.len())
        .sum::<usize>();
    
    if loop_size > 50 {
        return false; // 大きすぎるループは展開しない
    }
    
/// ループ展開の可否を高度な条件で判定
fn should_unroll_loop(function: &Function, loop_info: &LoopInfo) -> bool {
    // ループサイズと展開後の推定サイズを計算
    let loop_size = loop_info.blocks.iter()
        .map(|&id| function.blocks.iter().find(|b| b.id == id).unwrap().instructions.len())
        .sum::<usize>();
    let estimated_unrolled_size = loop_size * (loop_info.estimated_trip_count.unwrap_or(1) as usize);

    // 展開制限条件チェック
    if loop_size > 50 || estimated_unrolled_size > 200 {
        return false;
    }

    // 依存型解析による安全性保証
    let mut has_memory_dependency = false;
    let mut has_volatile_access = false;
    for &block_id in &loop_info.blocks {
        if let Some(block) = function.blocks.iter().find(|b| b.id == block_id) {
            for instr in &block.instructions {
                if let Instruction::Load { volatile, .. } | Instruction::Store { volatile, .. } = instr {
                    if *volatile {
                        has_volatile_access = true;
                    }
                }
                if has_side_effect(instr) {
                    has_memory_dependency = true;
                }
            }
        }
    }

    // イテレーション数がコンパイル時定数か解析
    let mut is_fix_iteration = false;
    let mut trip_count = None;
    for (var, pattern) in &loop_info.induction_vars {
        if let InductionPattern::Linear { start, step, end } = pattern {
            if let (Some(start_val), Some(step_val), Some(end_val)) = (
                start.as_constant(),
                step.as_constant(),
                end.as_constant(),
            ) {
                let count = ((end_val - start_val) / step_val).abs() as u32;
                if count <= 16 && count > 1 {
                    is_fix_iteration = true;
                    trip_count = Some(count);
                    break;
                }
            }
        }
    }

    // 展開の有効性条件
    is_fix_iteration &&
    !has_volatile_access &&
    !has_memory_dependency &&
    loop_info.control_flow.is_simple() &&
    function.verify().is_ok()
}

/// 完全なループ展開の実装
fn unroll_loop(function: &mut Function, loop_info: &mut LoopInfo) {
    let trip_count = loop_info.estimated_trip_count.unwrap_or(1);
    let header_id = loop_info.header_block;
    let latch_block = loop_info.latch_block.expect("Latch block must exist");
    
    // ヘッダーブロックのPHIノードを収集
    let mut phi_nodes = Vec::new();
    if let Some(header) = function.blocks.iter_mut().find(|b| b.id == header_id) {
        phi_nodes = header.instructions.iter()
            .filter_map(|instr| if let Instruction::Phi { result, ty, incoming } = instr {
                Some((*result, ty.clone(), incoming.clone()))
            } else {
                None
            })
            .collect();
    }

    // ループボディの複製と接続
    let mut cloned_blocks = Vec::new();
    for i in 0..trip_count {
        let mut new_blocks = function.clone_blocks(&loop_info.blocks);
        function.renumber_blocks(&mut new_blocks);
        function.renumber_instructions(&mut new_blocks);
        
        // PHIノードの更新
        for (result, ty, incoming) in &phi_nodes {
            let new_val = function.make_constant(Value::Int(i as i64));
            function.blocks.iter_mut().for_each(|block| {
                block.instructions.iter_mut().for_each(|instr| {
                    if let Instruction::Phi { incoming: ref mut inc, .. } = instr {
                        inc.iter_mut().for_each(|(val, pred)| {
                            if *val == *result {
                                *val = new_val;
                                *pred = latch_block;
                            }
                        });
                    }
                });
            });
        }
        
        cloned_blocks.extend(new_blocks);
    }

    // 制御フローの再接続
    function.cfg.redirect_loop_exits(&loop_info, &cloned_blocks);
    function.cfg.remove_block(latch_block);
    function.cfg.verify().expect("CFG integrity check failed");
    
    // 不要なPHIノードの削除
    function.remove_dead_phi_nodes();
    function.verify().expect("Function verification failed after unrolling");
}

/// 高度なループ融合の実装
fn fuse_adjacent_loops(function: &mut Function, loops: &[LoopInfo]) {
    let fusion_candidates = loops.windows(2)
        .filter(|pair| {
            let l1 = &pair[0];
            let l2 = &pair[1];
            
            // 融合条件チェック
            l1.is_adjacent_to(l2) &&
            l1.has_same_iteration_space(l2) &&
            !l1.has_dependency_with(l2) &&
            l1.control_flow.is_compatible_with(&l2.control_flow) &&
            function.dominance.is_safe_to_fuse(l1, l2)
        })
        .collect::<Vec<_>>();

    for candidates in fusion_candidates {
        let (l1, l2) = (&candidates[0], &candidates[1]);
        
        // ループヘッダの統合
        let new_header = function.fuse_headers(l1.header_block, l2.header_block);
        
        // ループボディのマージ
        let merged_body = function.merge_blocks(&[l1.body(), l2.body()]);
        
        // 制御フローの更新
        function.cfg.redirect_branches(l1.latch_block.unwrap(), new_header);
        function.cfg.redirect_branches(l2.latch_block.unwrap(), new_header);
        
        // データフロー解析の更新
        function.update_dataflow_analysis();
        
        // 不要なブロックの削除
        function.remove_blocks(&[l1.header_block, l2.header_block]);
    }
    
    // 最適化後の検証
    function.verify().expect("Function verification failed after loop fusion");
    function.cfg.verify().expect("CFG verification failed after loop fusion");
}
}

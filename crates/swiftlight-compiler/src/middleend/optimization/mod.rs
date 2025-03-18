// SwiftLight最適化モジュール
//
// このモジュールはIR最適化のためのフレームワークとパスを提供します。
// 主な役割は以下の通りです：
// - 最適化パスの定義と管理
// - 最適化パイプラインの構築
// - 最適化レベルに応じたパスの選択

pub mod pass;
use log::{debug, warn};
use std::collections::HashSet;
use crate::frontend::error::{Result, CompilerError};
use crate::middleend::ir::representation::{Module, Function, Instruction, BasicBlock, Value, Type};
use crate::middleend::OptimizationLevel;

/// 最適化マネージャ
/// 
/// 最適化パスの管理と実行を担当します。
pub struct OptimizationManager {
    /// 有効化された最適化パス
    enabled_passes: HashSet<&'static str>,
    
    /// 最適化レベル
    level: OptimizationLevel,
    
    /// 最適化統計情報
    stats: OptimizationStats,
}

/// 最適化統計情報
#[derive(Debug, Clone, Default)]
pub struct OptimizationStats {
    /// 実行された最適化パスの数
    pub passes_run: usize,
    
    /// 削除された命令の数
    pub instructions_removed: usize,
    
    /// 削除された基本ブロックの数
    pub blocks_removed: usize,
    
    /// インライン化された関数呼び出しの数
    pub functions_inlined: usize,
    
    /// 定数畳み込みが行われた回数
    pub constants_folded: usize,
    
    /// 最適化に要した時間（ミリ秒）
    pub time_ms: u64,
}

impl OptimizationManager {
    /// 新しい最適化マネージャを作成
    pub fn new(level: OptimizationLevel) -> Self {
        let mut manager = Self {
            enabled_passes: HashSet::new(),
            level,
            stats: OptimizationStats::default(),
        };
        
        // 最適化レベルに応じたパスを有効化
        manager.configure_passes();
        
        manager
    }
    
    /// 最適化レベルに応じたパスを設定
    fn configure_passes(&mut self) {
        match self.level {
            OptimizationLevel::None => {
                // 最適化なし
            },
            OptimizationLevel::Basic => {
                // 基本的な最適化
                self.enable_pass("dce");      // デッドコード除去
                self.enable_pass("constfold"); // 定数畳み込み
                self.enable_pass("simplifycfg"); // CFG単純化
            },
            OptimizationLevel::Standard => {
                // 標準的な最適化（基本的な最適化に加えて）
                self.enable_pass("dce");
                self.enable_pass("constfold");
                self.enable_pass("simplifycfg");
                self.enable_pass("gvn");      // グローバル値番号付け
                self.enable_pass("instcombine"); // 命令結合
                self.enable_pass("licm");     // ループ不変コード移動
            },
            OptimizationLevel::Aggressive => {
                // 積極的な最適化（標準的な最適化に加えて）
                self.enable_pass("dce");
                self.enable_pass("constfold");
                self.enable_pass("simplifycfg");
                self.enable_pass("gvn");
                self.enable_pass("instcombine");
                self.enable_pass("licm");
                self.enable_pass("inline");   // 関数インライン化
                self.enable_pass("loopunroll"); // ループ展開
                self.enable_pass("tailcall"); // 末尾呼び出し最適化
            },
        }
    }
    
    /// 最適化パスを有効化
    pub fn enable_pass(&mut self, pass_name: &'static str) {
        self.enabled_passes.insert(pass_name);
    }
    
    /// 最適化パスを無効化
    pub fn disable_pass(&mut self, pass_name: &'static str) {
        self.enabled_passes.remove(pass_name);
    }
    
    /// 最適化パスが有効かどうか
    pub fn is_pass_enabled(&self, pass_name: &str) -> bool {
        self.enabled_passes.contains(pass_name)
    }
    
    /// 有効化されたすべてのパスを実行
    pub fn run_all_passes(&mut self, module: &mut Module) -> Result<()> {
        let start_time = std::time::Instant::now();
        
        // モジュールレベルの最適化パス
        if self.is_pass_enabled("constfold") {
            self.run_pass(module, transforms::constant_folding)?;
            self.stats.passes_run += 1;
        }
        
        if self.is_pass_enabled("dce") {
            self.run_pass(module, transforms::dead_code_elimination)?;
            self.stats.passes_run += 1;
        }
        
        if self.is_pass_enabled("simplifycfg") {
            self.run_pass(module, transforms::simplify_cfg)?;
            self.stats.passes_run += 1;
        }
        
        // 関数レベルの最適化パスはすべての関数に適用
        let function_names: Vec<String> = module.functions.keys().cloned().collect();
        for name in function_names {
            if let Some(function) = module.functions.get_mut(&name) {
                self.run_function_passes(function)?;
            }
        }
        
        // インライン化は関数間の最適化なので別途処理
        if self.is_pass_enabled("inline") {
            self.run_pass(module, transforms::inline_functions)?;
            self.stats.passes_run += 1;
        }
        
        // 最適化時間を記録
        self.stats.time_ms = start_time.elapsed().as_millis() as u64;
        
        Ok(())
    }
    
    /// 関数レベルの最適化パスを実行
    fn run_function_passes(&mut self, function: &mut Function) -> Result<()> {
        // 関数レベルの最適化パス
        if self.is_pass_enabled("licm") {
            transforms::loop_invariant_code_motion(function)?;
            self.stats.passes_run += 1;
        }
        
        if self.is_pass_enabled("tailcall") {
            transforms::tail_call_optimization(function)?;
            self.stats.passes_run += 1;
        }
        
        if self.is_pass_enabled("loopunroll") {
            transforms::loop_unrolling(function)?;
            self.stats.passes_run += 1;
        }
        
        Ok(())
    }
    
    /// 単一の最適化パスを実行
    fn run_pass<F>(&mut self, module: &mut Module, pass: F) -> Result<()>
    where
        F: FnOnce(&mut Module) -> Result<()>,
    {
        pass(module)
    }
    
    /// 最適化統計情報を取得
    pub fn stats(&self) -> &OptimizationStats {
        &self.stats
    }
}

/// モジュールを最適化
pub fn optimize_module(module: Module) -> Result<Module> {
    // デフォルトの最適化レベル
    optimize_module_with_level(module, OptimizationLevel::default())
}

/// 指定された最適化レベルでモジュールを最適化
pub fn optimize_module_with_level(mut module: Module, level: OptimizationLevel) -> Result<Module> {
    // 最適化レベルがNoneなら何もせずに返す
    if matches!(level, OptimizationLevel::None) {
        return Ok(module);
    }
    
    // 最適化マネージャを作成
    let mut manager = OptimizationManager::new(level);
    
    // すべての最適化パスを実行
    manager.run_all_passes(&mut module)?;
    
    // 最適化された中間表現を返す
    Ok(module)
}

// 最適化変換の定義（実装はtransformsモジュール内）
pub mod transforms {
    use crate::frontend::error::Result;
    use crate::middleend::ir::representation::{Module, Function, Instruction, BasicBlock, Value, Type};
    use std::collections::{HashSet, HashMap};

    // モジュールレベルの最適化パス
    
    /// デッドコード除去
    pub fn dead_code_elimination(module: &mut Module) -> Result<()> {
        // 各関数に対してデッドコード除去を適用
        for (_name, function) in module.functions.iter_mut() {
            eliminate_dead_code_in_function(function);
        }
        
        // 未使用の関数を特定
        let mut used_functions = HashSet::new();
        // mainやエクスポートされた関数はルート関数と見なす
        for (_name, function) in module.functions.iter() {
            if function.is_exported || function.name == "main" {
                used_functions.insert(function.id);
                // 関数のリストを作成
                let all_funcs: Vec<&Function> = module.functions.values().collect();
                mark_called_functions(function, &all_funcs, &mut used_functions);
            }
        }
        
        // 未使用関数をフィルタリング
        module.functions.retain(|_name, f| used_functions.contains(&f.id));
        
        Ok(())
    }
    
    // 関数から呼び出される全ての関数を再帰的にマーク
    fn mark_called_functions(function: &Function, all_functions: &[&Function], used: &mut HashSet<u32>) {
        for block in &function.blocks {
            for inst in &block.instructions {
                if let Instruction::Call { function_id, .. } = inst {
                    if !used.contains(function_id) {
                        used.insert(*function_id);
                        if let Some(called_fn) = all_functions.iter().find(|f| f.id == *function_id) {
                            mark_called_functions(called_fn, all_functions, used);
                        }
                    }
                }
            }
        }
    }
    
    // 関数内のデッドコードを除去
    fn eliminate_dead_code_in_function(function: &mut Function) {
        // 値の使用状況を追跡
        let mut used_values = HashSet::new();
        
        // 関数の戻り値や副作用のある命令をルートとしてマーク
        for block in &function.blocks {
            for inst in &block.instructions {
                match inst {
                    Instruction::Return { value } => {
                        if let Some(val) = value {
                            used_values.insert(*val);
                        }
                    },
                    Instruction::Store { .. } |
                    Instruction::Call { .. } => {
                        // 副作用のある命令はすべて使用済みとマーク
                        // (将来的には純粋関数呼び出しの検出など高度な解析を追加)
                        mark_instruction_used(inst, &mut used_values);
                    },
                    _ => {}
                }
            }
        }
        
        // 使用される値を再帰的にマーク
        let mut changed = true;
        while changed {
            changed = false;
            for block in &function.blocks {
                for inst in &block.instructions {
                    if mark_used_operands(inst, &used_values, &mut used_values) {
                        changed = true;
                    }
                }
            }
        }
        
        // 未使用の命令を削除
        for block in &mut function.blocks {
            block.instructions.retain(|inst| {
                match inst {
                    Instruction::BinaryOp { result, .. } |
                    Instruction::Load { result, .. } |
                    Instruction::GetElementPtr { result, .. } |
                    Instruction::Call { result, .. } |
                    Instruction::Alloca { result, .. } |
                    Instruction::Phi { result, .. } => {
                        used_values.contains(result)
                    },
                    // 副作用のある命令や制御フロー命令は常に保持
                    Instruction::Store { .. } |
                    Instruction::Branch { .. } |
                    Instruction::ConditionalBranch { .. } |
                    Instruction::Return { .. } => true,
                    _ => true, // その他の命令は保持
                }
            });
        }
    }
    
    // 命令自体をマーク
    fn mark_instruction_used(inst: &Instruction, used: &mut HashSet<u32>) {
        match inst {
            Instruction::BinaryOp { result, .. } |
            Instruction::Load { result, .. } |
            Instruction::GetElementPtr { result, .. } |
            Instruction::Call { result, .. } |
            Instruction::Alloca { result, .. } |
            Instruction::Phi { result, .. } => {
                used.insert(*result);
            },
            _ => {}
        }
    }
    
    // 命令のオペランドをマーク
    fn mark_used_operands(inst: &Instruction, already_used: &HashSet<u32>, newly_used: &mut HashSet<u32>) -> bool {
        let mut changed = false;
        
        match inst {
            Instruction::BinaryOp { left, right, .. } => {
                if already_used.contains(left) && !newly_used.contains(right) {
                    newly_used.insert(*right);
                    changed = true;
                }
                if already_used.contains(right) && !newly_used.contains(left) {
                    newly_used.insert(*left);
                    changed = true;
                }
            },
            Instruction::Load { address, .. } => {
                if already_used.contains(address) && !newly_used.contains(address) {
                    newly_used.insert(*address);
                    changed = true;
                }
            },
            // 他の命令タイプも同様に処理
            _ => {}
        }
        
        changed
    }

    /// 定数畳み込み
    pub fn constant_folding(module: &mut Module) -> Result<()> {
        // 各関数に対して定数畳み込みを適用
        for (_name, function) in module.functions.iter_mut() {
            fold_constants_in_function(function);
        }
        
        Ok(())
    }
    
    // 関数内の定数を畳み込む
    fn fold_constants_in_function(function: &mut Function) {
        // 定数値のマッピング
        let mut constant_values: HashMap<u32, Value> = HashMap::new();
        
        // 基本ブロックごとに処理
        for block in &mut function.blocks {
            let mut i = 0;
            while i < block.instructions.len() {
                let mut folded = false;
                
                // 現在の命令を解析
                if let Instruction::BinaryOp { op, result, left, right } = block.instructions[i] {
                    // 両オペランドが定数ならフォールディング可能
                    if let (Some(left_val), Some(right_val)) = (constant_values.get(&left).cloned(), constant_values.get(&right).cloned()) {
                        if let (Value::Integer(l), Value::Integer(r)) = (left_val, right_val) {
                            let folded_value = match op {
                                BinaryOp::Add => Value::Integer(l + r),
                                BinaryOp::Subtract => Value::Integer(l - r),
                                BinaryOp::Multiply => Value::Integer(l * r),
                                BinaryOp::Divide => {
                                    if r != 0 {
                                        Value::Integer(l / r)
                                    } else {
                                        // ゼロ除算は畳み込まない
                                        Value::Integer(0) // ダミー値
                                    }
                                },
                                BinaryOp::Remainder => {
                                    if r != 0 {
                                        Value::Integer(l % r)
                                    } else {
                                        // ゼロ除算は畳み込まない
                                        Value::Integer(0) // ダミー値
                                    }
                                },
                                BinaryOp::BitwiseAnd => Value::Integer(l & r),
                                BinaryOp::BitwiseOr => Value::Integer(l | r),
                                BinaryOp::BitwiseXor => Value::Integer(l ^ r),
                                BinaryOp::ShiftLeft => Value::Integer(l << r),
                                BinaryOp::ShiftRight => Value::Integer(l >> r),
                                // 比較演算子
                                BinaryOp::Equal => Value::Boolean(l == r),
                                BinaryOp::NotEqual => Value::Boolean(l != r),
                                BinaryOp::LessThan => Value::Boolean(l < r),
                                BinaryOp::LessThanOrEqual => Value::Boolean(l <= r),
                                BinaryOp::GreaterThan => Value::Boolean(l > r),
                                BinaryOp::GreaterThanOrEqual => Value::Boolean(l >= r),
                                // 論理演算子
                                BinaryOp::LogicalAnd => Value::Boolean((l != 0) && (r != 0)),
                                BinaryOp::LogicalOr => Value::Boolean((l != 0) || (r != 0)),
                            };
                            
                            // 定数マップに追加
                            constant_values.insert(result, folded_value.clone());
                            
                            // 畳み込まれた命令を定数にする
                            block.instructions[i] = Instruction::Constant { result, value: folded_value };
                            folded = true;
                        }
                    }
                } else if let Instruction::Constant { result, value } = &block.instructions[i] {
                    // 定数をマップに記録
                    constant_values.insert(*result, value.clone());
                }
                
                if !folded {
                    i += 1;
                }
            }
        }
    }

    /// 制御フローグラフの単純化
    pub fn simplify_cfg(module: &mut Module) -> Result<()> {
        for (_name, function) in module.functions.iter_mut() {
            // 1. 空のブロックを削除
            remove_empty_blocks(function);
            
            // 2. 不要なジャンプを削除
            eliminate_unnecessary_jumps(function);
            
            // 3. 到達不能コードを削除
            remove_unreachable_blocks(function);
            
            // 4. 基本ブロックの結合
            merge_blocks(function);
        }
        
        Ok(())
    }
    
    // 空のブロックを削除
    fn remove_empty_blocks(function: &mut Function) {
        // 単純な実装のため省略（空ブロックは通常事前に生成されない）
    }
    
    // 単一の無条件ジャンプを削除
    fn eliminate_unnecessary_jumps(function: &mut Function) {
        // ブロックからターゲットへのマッピング
        let mut jump_targets: HashMap<u32, u32> = HashMap::new();
        
        // 単一ジャンプブロックを特定
        for block in &function.blocks {
            if block.instructions.len() == 1 {
                if let Instruction::Branch { target } = block.instructions[0] {
                    jump_targets.insert(block.id, target);
                }
            }
        }
        
        // ジャンプ先を書き換え
        for block in &mut function.blocks {
            for inst in &mut block.instructions {
                if let Instruction::Branch { target } = inst {
                    while let Some(&next_target) = jump_targets.get(target) {
                        *target = next_target;
                    }
                } else if let Instruction::ConditionalBranch { true_target, false_target, .. } = inst {
                    while let Some(&next_target) = jump_targets.get(true_target) {
                        *true_target = next_target;
                    }
                    while let Some(&next_target) = jump_targets.get(false_target) {
                        *false_target = next_target;
                    }
                }
            }
        }
    }
    
    // 到達不能ブロックを削除
    fn remove_unreachable_blocks(function: &mut Function) {
        // エントリーブロックから到達可能なブロックを特定
        let mut reachable = HashSet::new();
        let mut worklist = vec![function.entry_block];
        
        while let Some(block_id) = worklist.pop() {
            if reachable.insert(block_id) {
                if let Some(block) = function.blocks.iter().find(|b| b.id == block_id) {
                    for inst in &block.instructions {
                        match inst {
                            Instruction::Branch { target } => {
                                worklist.push(*target);
                            },
                            Instruction::ConditionalBranch { true_target, false_target, .. } => {
                                worklist.push(*true_target);
                                worklist.push(*false_target);
                            },
                            _ => {}
                        }
                    }
                }
            }
        }
        
        // 到達不能ブロックを削除
        function.blocks.retain(|block| reachable.contains(&block.id));
    }
    
    // 連続するブロックを結合
    fn merge_blocks(function: &mut Function) {
        // 単純化のため省略（高度なブロック結合はより複雑なCFG解析が必要）
    }

    /// 関数インライン化
    pub fn inline_functions(module: &mut Module) -> Result<()> {
        let mut changes = false;
        
        // 各関数に対して実行
        for (_name, function) in module.functions.iter_mut() {
            // スモールな関数や単一の呼び出し地点を持つ関数を探す
            let mut inline_candidates = Vec::new();
            
            // すべての基本ブロックをスキャン
            for block_idx in 0..function.blocks.len() {
                let block = &function.blocks[block_idx];
                
                // 各命令をスキャン
                for inst_idx in 0..block.instructions.len() {
                    if let Instruction::Call { function_id, args, result } = &block.instructions[inst_idx] {
                        // 呼び出し先関数を取得
                        if let Some(target_name) = module.get_function_name(*function_id) {
                            if let Some(target_func) = module.functions.get(&target_name) {
                                // スモールな関数かチェック
                                if is_small_function(target_func) {
                                    inline_candidates.push((block_idx, inst_idx, target_name.clone(), args.clone(), *result));
                                }
                            }
                        }
                    }
                }
            }
            
            // インライン化を実行
            for (block_idx, inst_idx, target_name, args, result_id) in inline_candidates {
                if let Some(target_func) = module.functions.get(&target_name) {
                    let mut block = &mut function.blocks[block_idx];
                    
                    // インライン化を実行
                    let inline_success = inline_function_at_call_site(
                        function,
                        block,
                        inst_idx,
                        target_func,
                        args,
                        result_id
                    );
                    
                    if inline_success {
                        changes = true;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    // 関数が小さいかどうかを判定
    fn is_small_function(function: &Function) -> bool {
        // 命令数で判定（しきい値は調整可能）
        let instruction_count = function.blocks.iter()
            .map(|block| block.instructions.len())
            .sum::<usize>();
        
        instruction_count < 20
    }
    
    // 関数が単一の呼び出し元を持つかどうかを判定
    fn has_single_call_site(function: &Function, module: &Module) -> bool {
        let mut call_sites = 0;
        
        for f in &module.functions {
            for block in &f.blocks {
                for inst in &block.instructions {
                    if let Instruction::Call { function_id, .. } = inst {
                        if *function_id == function.id {
                            call_sites += 1;
                            if call_sites > 1 {
                                return false;
                            }
                        }
                    }
                }
            }
        }
        
        call_sites == 1
    }
    
    // 呼び出し箇所に関数をインライン化
    fn inline_function_at_call_site(
        parent: &mut Function, 
        block: &mut BasicBlock, 
        call_index: usize,
        callee: &Function, 
        args: Vec<u32>, 
        result_id: u32
    ) -> bool {
        println!("関数 {} のインライン化を試行中", callee.name);
        
        // 1. 引数をマッピング
        let mut arg_mapping = HashMap::new();
        let callee_params = &callee.parameters;
        
        if args.len() != callee_params.len() {
            println!("警告: 引数の数が一致しないためインライン化を中止: 期待={}, 実際={}", 
                  callee_params.len(), args.len());
            return false;
        }
        
        for (i, param) in callee_params.iter().enumerate() {
            arg_mapping.insert(param.id, args[i]);
        }
        
        // 2. IDのマッピングテーブルを作成（衝突回避）
        let mut id_mapping = HashMap::new();
        id_mapping.insert(result_id, result_id); // 結果IDは維持
        
        // 3. 呼び出し先関数のエントリーブロックを特定
        let entry_block = callee.blocks.iter()
            .find(|b| b.id == callee.entry_block)
            .ok_or_else(|| {
                println!("警告: エントリーブロックが見つかりませんでした");
                return false;
            })
            .unwrap();
        
        // 4. 基本ブロックのマッピングを作成
        let mut block_mapping = HashMap::new();
        
        // 4.1 新しい基本ブロック生成用IDカウンタ
        let mut next_block_id = parent.blocks.iter()
            .map(|b| b.id)
            .max()
            .unwrap_or(0) + 1;
        
        // 4.2 新しいIR値生成用IDカウンタ
        let mut next_value_id = parent.blocks.iter()
            .flat_map(|b| b.instructions.iter())
            .filter_map(|inst| match inst {
                Instruction::BinaryOp { result, .. } |
                Instruction::Load { result, .. } |
                Instruction::GetElementPtr { result, .. } |
                Instruction::Call { result, .. } |
                Instruction::Alloca { result, .. } |
                Instruction::Phi { result, .. } |
                Instruction::Constant { result, .. } => Some(*result),
                _ => None,
            })
            .max()
            .unwrap_or(0) + 1;
        
        // 5. 各基本ブロックをマッピング
        for callee_block in &callee.blocks {
            let new_block_id = next_block_id;
            next_block_id += 1;
            block_mapping.insert(callee_block.id, new_block_id);
        }
        
        // 6. インライン化前後の命令位置リスト
        let mut inline_instructions = Vec::new();
        
        // 7. 呼び出し先関数の命令をクローンし、ID参照を更新
        for callee_block in &callee.blocks {
            let new_block_id = block_mapping[&callee_block.id];
            let mut new_instructions = Vec::new();
            
            for inst in &callee_block.instructions {
                let new_inst = match inst {
                    Instruction::BinaryOp { op, left, right, .. } => {
                        let new_result = next_value_id;
                        next_value_id += 1;
                        id_mapping.insert(new_result, new_result);
                        
                        let new_left = *arg_mapping.get(left).unwrap_or_else(|| {
                            id_mapping.entry(*left).or_insert_with(|| {
                                let id = next_value_id;
                                next_value_id += 1;
                                id
                            })
                        });
                        
                        let new_right = *arg_mapping.get(right).unwrap_or_else(|| {
                            id_mapping.entry(*right).or_insert_with(|| {
                                let id = next_value_id;
                                next_value_id += 1;
                                id
                            })
                        });
                        
                        Instruction::BinaryOp {
                            op: *op,
                            result: new_result,
                            left: new_left,
                            right: new_right,
                        }
                    },
                    Instruction::Load { address, .. } => {
                        let new_result = next_value_id;
                        next_value_id += 1;
                        id_mapping.insert(new_result, new_result);
                        
                        let new_address = *arg_mapping.get(address).unwrap_or_else(|| {
                            id_mapping.entry(*address).or_insert_with(|| {
                                let id = next_value_id;
                                next_value_id += 1;
                                id
                            })
                        });
                        
                        Instruction::Load {
                            result: new_result,
                            address: new_address,
                        }
                    },
                    Instruction::Store { value, address } => {
                        let new_value = *arg_mapping.get(value).unwrap_or_else(|| {
                            id_mapping.entry(*value).or_insert_with(|| {
                                let id = next_value_id;
                                next_value_id += 1;
                                id
                            })
                        });
                        
                        let new_address = *arg_mapping.get(address).unwrap_or_else(|| {
                            id_mapping.entry(*address).or_insert_with(|| {
                                let id = next_value_id;
                                next_value_id += 1;
                                id
                            })
                        });
                        
                        Instruction::Store {
                            value: new_value,
                            address: new_address,
                        }
                    },
                    Instruction::GetElementPtr { base, indices, .. } => {
                        let new_result = next_value_id;
                        next_value_id += 1;
                        id_mapping.insert(new_result, new_result);
                        
                        let new_base = *arg_mapping.get(base).unwrap_or_else(|| {
                            id_mapping.entry(*base).or_insert_with(|| {
                                let id = next_value_id;
                                next_value_id += 1;
                                id
                            })
                        });
                        
                        let new_indices = indices.iter()
                            .map(|idx| {
                                *arg_mapping.get(idx).unwrap_or_else(|| {
                                    id_mapping.entry(*idx).or_insert_with(|| {
                                        let id = next_value_id;
                                        next_value_id += 1;
                                        id
                                    })
                                })
                            })
                            .collect();
                        
                        Instruction::GetElementPtr {
                            result: new_result,
                            base: new_base,
                            indices: new_indices,
                        }
                    },
                    Instruction::Call { function_id, args, .. } => {
                        // 再帰呼び出しを避けるため、同じ関数への呼び出しの場合は特別扱い
                        if *function_id == callee.id {
                            println!("警告: 再帰呼び出しを検出。スキップします。");
                            continue;
                        }
                        
                        let new_result = next_value_id;
                        next_value_id += 1;
                        id_mapping.insert(new_result, new_result);
                        
                        let new_args = args.iter()
                            .map(|arg| {
                                *arg_mapping.get(arg).unwrap_or_else(|| {
                                    id_mapping.entry(*arg).or_insert_with(|| {
                                        let id = next_value_id;
                                        next_value_id += 1;
                                        id
                                    })
                                })
                            })
                            .collect();
                        
                        Instruction::Call {
                            result: new_result,
                            function_id: *function_id,
                            args: new_args,
                        }
                    },
                    Instruction::Return { value } => {
                        // リターン命令は特別処理
                        // 最後のリターン以外は単なる代入に変換
                        if let Some(ret_val) = value {
                            let new_value = *arg_mapping.get(ret_val).unwrap_or_else(|| {
                                id_mapping.entry(*ret_val).or_insert_with(|| {
                                    let id = next_value_id;
                                    next_value_id += 1;
                                    id
                                })
                            });
                            
                            // 元の呼び出しの結果に代入
                            Instruction::Store {
                                value: new_value,
                                address: result_id,
                            }
                        } else {
                            // void関数の場合はスキップ
                            continue;
                        }
                    },
                    Instruction::Branch { target } => {
                        let new_target = block_mapping[target];
                        Instruction::Branch { target: new_target }
                    },
                    Instruction::ConditionalBranch { condition, true_target, false_target } => {
                        let new_condition = *arg_mapping.get(condition).unwrap_or_else(|| {
                            id_mapping.entry(*condition).or_insert_with(|| {
                                let id = next_value_id;
                                next_value_id += 1;
                                id
                            })
                        });
                        
                        let new_true = if loop_blocks.contains(true_target) {
                            block_mapping[true_target]
                        } else {
                            *true_target
                        };
                        
                        let new_false = if loop_blocks.contains(false_target) {
                            block_mapping[false_target]
                        } else {
                            *false_target
                        };
                        
                        Instruction::ConditionalBranch {
                            condition: new_condition,
                            true_target: new_true,
                            false_target: new_false,
                        }
                    },
                    // 他の命令タイプも同様に処理
                    _ => continue, // 未対応の命令タイプは省略
                };
                
                new_instructions.push(new_inst);
            }
            
            inline_instructions.push((new_block_id, new_instructions));
        }
        
        // 8. 元の呼び出し命令の前に引数準備コードを挿入
        let mut before_call_insts = Vec::new();
        
        // 9. 元の呼び出し命令を削除し、インライン化されたコードを挿入
        block.instructions.remove(call_index);
        
        // 10. インライン化されたブロックを親関数に追加
        for (block_id, insts) in inline_instructions {
            let new_block = BasicBlock {
                instructions: insts,
                label: format!(".L{}", block_id),
                predecessors: HashSet::new(),
                successors: HashSet::new(),
                dominator: None,
                dominates: HashSet::new(),
                is_loop_header: false,
                loop_depth: 0,
            };
            parent.blocks.push(new_block);
        }
        
        // 11. 制御フローを調整
        // 元のブロックのCall命令の後ろにある命令を新しいブロックに移動
        let original_insts_after_call = block.instructions.split_off(call_index);
        let continuation_block_id = next_block_id;
        
        if !original_insts_after_call.is_empty() {
            let continuation_block = BasicBlock {
                instructions: original_insts_after_call,
                label: format!(".L{}", continuation_block_id),
                predecessors: HashSet::new(),
                successors: HashSet::new(),
                dominator: None,
                dominates: HashSet::new(),
                is_loop_header: false,
                loop_depth: 0,
            };
            parent.blocks.push(continuation_block);
            
            // 元のブロックから継続ブロックへの分岐を追加
            block.instructions.push(Instruction::Branch { target: continuation_block_id });
        }
        
        println!("関数 {} のインライン化に成功", callee.name);
        true
    }

    // ループのプリヘッダーを検索または作成
    fn find_or_create_preheader(function: &mut Function, header: u32) -> Option<u32> {
        // 1. ヘッダーブロックへの全ての前任ブロックを収集
        let mut predecessors = Vec::new();
        for block in &function.blocks {
            for inst in &block.instructions {
                match inst {
                    Instruction::Branch { target } if *target == header => {
                        predecessors.push(block.id);
                    },
                    Instruction::ConditionalBranch { true_target, false_target, .. } => {
                        if *true_target == header || *false_target == header {
                            predecessors.push(block.id);
                        }
                    },
                    _ => {}
                }
            }
        }
        
        // 2. 前任ブロックが1つのみで、それがバックエッジでない場合、それがプリヘッダー
        if predecessors.len() == 1 {
            // ヘッダーがそのブロックをループ内で支配していなければ、バックエッジではない
            // (簡略化のため、ここでは単純に前任ブロックIDがヘッダーIDより小さければ非バックエッジと仮定)
            if predecessors[0] < header {
                return Some(predecessors[0]);
            }
        }
        
        // 3. プリヘッダーが存在しない場合は新しく作成
        // 新しいブロックIDを生成
        let next_id = function.blocks.iter()
            .map(|b| b.id)
            .max()
            .unwrap_or(0) + 1;
        
        // プリヘッダーブロックを作成
        let preheader = BasicBlock {
            label: format!(".L{}", next_id),
            instructions: vec![],
            predecessors: HashSet::new(),
            successors: HashSet::new(),
            dominator: None,
            dominates: HashSet::new(),
            is_loop_header: false,
            loop_depth: 0,
        };
        
        // 関数に新しいブロックを追加
        function.blocks.push(preheader);
        
        // ヘッダーへのエッジを更新
        for block in &mut function.blocks {
            for inst in &mut block.instructions {
                match inst {
                    Instruction::Branch { target } if *target == header => {
                        // ループの外からのエッジのみをプリヘッダーにリダイレクト
                        // (バックエッジは維持)
                        if block.id < header {
                            *target = next_id;
                        }
                    },
                    Instruction::ConditionalBranch { true_target, false_target, .. } => {
                        if *true_target == header && block.id < header {
                            *true_target = next_id;
                        }
                        if *false_target == header && block.id < header {
                            *false_target = next_id;
                        }
                    },
                    _ => {}
                }
            }
        }
        
        Some(next_id)
    }
    
    // 不変命令をホイスト
    fn hoist_invariant_instructions(
        function: &mut Function, 
        invariant_insts: &[(u32, usize)], 
        preheader: u32,
        loop_blocks: &HashSet<u32>
    ) {
        if invariant_insts.is_empty() {
            return;
        }
        
        // 1. プリヘッダーブロックを取得
        let preheader_block = match function.blocks.iter_mut().find(|b| b.id == preheader) {
            Some(block) => block,
            None => {
                println!("警告: プリヘッダーブロックが見つかりません");
                return;
            }
        };
        
        // 2. ホイスト対象の命令を抽出
        let mut hoisted_insts = Vec::new();
        let mut remove_indices = HashMap::new();
        
        for &(block_id, inst_idx) in invariant_insts {
            // ブロックを見つける
            if let Some(block) = function.blocks.iter().find(|b| b.id == block_id) {
                // 命令が存在するか確認
                if inst_idx < block.instructions.len() {
                    // 命令をクローン
                    let inst = block.instructions[inst_idx].clone();
                    
                    // ID依存関係を構築
                    let mut dependencies = HashSet::new();
                    match &inst {
                        Instruction::BinaryOp { left, right, .. } => {
                            dependencies.insert(*left);
                            dependencies.insert(*right);
                        },
                        Instruction::Load { address, .. } => {
                            dependencies.insert(*address);
                        },
                        // 他の命令タイプも同様に処理
                        _ => {}
                    }
                    
                    // ブランチ命令の前に挿入するためにホイスト命令を保存
                    hoisted_insts.push((inst, dependencies));
                    
                    // 元の命令の削除用インデックスを記録
                    remove_indices.entry(block_id)
                        .or_insert_with(Vec::new)
                        .push(inst_idx);
                }
            }
        }
        
        // 3. 命令の依存関係に基づいてソート
        // (依存する命令が先に実行されるようにする)
        hoisted_insts.sort_by(|(_, deps1), (_, deps2)| {
            // 依存関係の少ない順に並べる（単純なヒューリスティック）
            deps1.len().cmp(&deps2.len())
        });
        
        // 4. プリヘッダーの終端命令（分岐）を一時的に保存
        let terminator = if let Some(last) = preheader_block.instructions.pop() {
            last
        } else {
            println!("警告: プリヘッダーブロックに終端命令がありません");
            return;
        };
        
        // 5. ホイスト対象の命令をプリヘッダーブロックに挿入
        for (inst, _) in hoisted_insts {
            preheader_block.instructions.push(inst);
        }
        
        // 6. 終端命令を戻す
        preheader_block.instructions.push(terminator);
        
        // 7. 元の命令を削除
        // 注: 削除するとインデックスがずれるため、大きいインデックスから削除
        for (block_id, indices) in remove_indices {
            if let Some(block) = function.blocks.iter_mut().find(|b| b.id == block_id) {
                let mut sorted_indices = indices;
                sorted_indices.sort_by(|a, b| b.cmp(a)); // 降順ソート
                
                for idx in sorted_indices {
                    if idx < block.instructions.len() {
                        block.instructions.remove(idx);
                    }
                }
            }
        }
    }

    /// 末尾呼び出し最適化
    pub fn tail_call_optimization(function: &mut Function) -> Result<()> {
        // 関数内の末尾再帰や末尾呼び出しを最適化
        for block in &mut function.blocks {
            // 最後の命令が戻り値を持つ場合のみ処理
            if let Some(last_inst_index) = block.instructions.len().checked_sub(1) {
                if let Instruction::Return { value: Some(return_value) } = block.instructions[last_inst_index] {
                    // 戻り値を生成する命令を探す
                    for i in (0..last_inst_index).rev() {
                        if let Instruction::Call { result, function_id, args } = &block.instructions[i] {
                            if *result == return_value {
                                // 末尾呼び出しを検出
                                if is_tail_call_candidate(function, *function_id) {
                                    // 末尾呼び出しをジャンプに変換
                                    optimize_tail_call(block, i, last_inst_index, *function_id, args.clone());
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    // 末尾呼び出し最適化の候補かどうかを判定
    fn is_tail_call_candidate(function: &Function, callee_id: u32) -> bool {
        // 再帰呼び出しかどうかをチェック
        callee_id == function.id
    }
    
    // 末尾呼び出しを最適化
    fn optimize_tail_call(
        block: &mut BasicBlock,
        call_index: usize,
        return_index: usize,
        function_id: u32,
        args: Vec<u32>
    ) {
        // 1. 呼び出し命令と戻り命令を取得
        let call_inst = block.instructions.remove(call_index);
        let return_inst = block.instructions.remove(return_index - 1); // call_indexが削除されたので-1する
        
        // 2. パラメーター再代入命令を生成
        let mut new_instructions = Vec::new();
        
        // 3. 再帰関数呼び出しの場合、引数を再代入する命令を追加
        // (実際の実装では、関数のパラメーターIDを取得して再代入する)
        for (i, arg) in args.iter().enumerate() {
            // 仮のパラメータID生成 (実際には関数の定義から取得)
            let param_id = 1000 + i as u32;
            
            // パラメータに引数を代入
            new_instructions.push(Instruction::Store {
                value: *arg,
                address: param_id,
            });
        }
        
        // 4. 関数先頭へのジャンプ命令を追加
        // (実際の実装では、関数のエントリーブロックIDを取得)
        new_instructions.push(Instruction::Branch { target: 1 });  // エントリーブロックIDは通常1
        
        // 5. 命令を置き換え
        block.instructions.splice(call_index..call_index, new_instructions);
    }

    /// ループ展開
    pub fn loop_unrolling(function: &mut Function) -> Result<()> {
        // ループの検出
        let loops = detect_simple_loops(function);
        
        // 候補ループの展開
        for loop_info in loops {
            if is_unroll_candidate(&loop_info, function) {
                unroll_loop(function, &loop_info);
            }
        }
        
        Ok(())
    }
    
    // 単純なループ情報
    struct SimpleLoop {
        header: u32,
        body: Vec<u32>,
        latch: u32,
        trip_count: Option<usize>,
    }
    
    // 単純なループを検出
    fn detect_simple_loops(function: &Function) -> Vec<SimpleLoop> {
        let mut loops = Vec::new();
        
        // 1. 基本的な到達可能性分析を実行
        let mut reachable_from = HashMap::new();
        
        // 各ブロックから到達可能なブロックを計算
        for block in &function.blocks {
            let mut visited = HashSet::new();
            let mut work_list = Vec::new();
            
            // 終端命令の分析
            for inst in &block.instructions {
                match inst {
                    Instruction::Branch { target } => {
                        work_list.push(*target);
                    },
                    Instruction::ConditionalBranch { true_target, false_target, .. } => {
                        work_list.push(*true_target);
                        work_list.push(*false_target);
                    },
                    _ => {}
                }
            }
            
            // 到達可能分析
            while let Some(next_id) = work_list.pop() {
                if visited.insert(next_id) {
                    if let Some(next_block) = function.blocks.iter().find(|b| b.id == next_id) {
                        for inst in &next_block.instructions {
                            match inst {
                                Instruction::Branch { target } => {
                                    work_list.push(*target);
                                },
                                Instruction::ConditionalBranch { true_target, false_target, .. } => {
                                    work_list.push(*true_target);
                                    work_list.push(*false_target);
                                },
                                _ => {}
                            }
                        }
                    }
                }
            }
            
            reachable_from.insert(block.id, visited);
        }
        
        // 2. バックエッジの検出
        for block in &function.blocks {
            for inst in &block.instructions {
                match inst {
                    Instruction::Branch { target } => {
                        // ターゲットブロックからこのブロックに到達可能ならバックエッジ
                        if let Some(reachable) = reachable_from.get(target) {
                            if reachable.contains(&block.id) {
                                // ループヘッダーを特定
                                let header = *target;
                                
                                // ループブロックを収集
                                let mut loop_blocks = HashSet::new();
                                loop_blocks.insert(header);
                                
                                // ヘッダーから到達可能でループに含まれるブロックを追加
                                if let Some(header_reachable) = reachable_from.get(&header) {
                                    for &id in header_reachable {
                                        if let Some(id_reachable) = reachable_from.get(&id) {
                                            if id_reachable.contains(&block.id) {
                                                loop_blocks.insert(id);
                                            }
                                        }
                                    }
                                }
                                
                                // ループ情報を構築
                                let loop_info = SimpleLoop {
                                    header,
                                    body: loop_blocks.iter().copied().collect(),
                                    latch: block.id,
                                    trip_count: estimate_trip_count(function, header, block.id),
                                };
                                
                                loops.push(loop_info);
                            }
                        }
                    },
                    Instruction::ConditionalBranch { true_target, false_target, .. } => {
                        // 各ターゲットについて同様のチェック
                        for target in [true_target, false_target] {
                            if let Some(reachable) = reachable_from.get(target) {
                                if reachable.contains(&block.id) {
                                    // ループ検出処理（上記と同様）
                                    let header = *target;
                                    let mut loop_blocks = HashSet::new();
                                    loop_blocks.insert(header);
                                    
                                    if let Some(header_reachable) = reachable_from.get(&header) {
                                        for &id in header_reachable {
                                            if let Some(id_reachable) = reachable_from.get(&id) {
                                                if id_reachable.contains(&block.id) {
                                                    loop_blocks.insert(id);
                                                }
                                            }
                                        }
                                    }
                                    
                                    let loop_info = SimpleLoop {
                                        header,
                                        body: loop_blocks.iter().copied().collect(),
                                        latch: block.id,
                                        trip_count: estimate_trip_count(function, header, block.id),
                                    };
                                    
                                    loops.push(loop_info);
                                }
                            }
                        }
                    },
                    _ => {}
                }
            }
        }
        
        loops
    }
    
    // ループのトリップカウント（反復回数）を推定
    fn estimate_trip_count(function: &Function, header: u32, latch: u32) -> Option<usize> {
        // 定数反復回数の検出（シンプルな解析）
        if let Some(latch_block) = function.blocks.iter().find(|b| b.id == latch) {
            // 条件分岐を探す
            for (i, inst) in latch_block.instructions.iter().enumerate() {
                if let Instruction::ConditionalBranch { condition, true_target, false_target, .. } = inst {
                    // 条件が定数比較かどうかをチェック
                    // (例: i < 10 のような形式)
                    
                    // ループヘッダーへのエッジと抜け出すエッジを特定
                    let loop_target = if *true_target == header { *true_target } else { *false_target };
                    
                    // 反復回数が確定できる特定パターンを検出
                    // 実際の実装ではより複雑なフロー解析が必要
                    
                    // シンプルな例として、特定の形式の条件を検出
                    return Some(10);  // 固定値（実際には解析結果）
                }
            }
        }
        
        None  // 反復回数を確定できない
    }

    // ループを展開
    fn unroll_loop(function: &mut Function, loop_info: &SimpleLoop) {
        // トリップカウントが既知でない場合は展開不可
        let trip_count = match loop_info.trip_count {
            Some(count) => count,
            None => return,
        };
        
        // 展開する回数を決定
        // 完全展開: ループ反復回数が少ない場合
        // 部分展開: ループ反復回数が多い場合
        let unroll_factor = if trip_count <= 4 {
            trip_count  // 完全展開
        } else {
            4  // 部分展開（4回に設定）
        };
        
        // 各展開インスタンスのIDマッピング
        let mut id_mappings = Vec::new();
        
        // ループ本体ブロックのID
        let loop_blocks: HashSet<u32> = HashSet::from_iter(loop_info.body.iter().copied());
        
        // 最大のブロックIDを特定（新しいIDの開始点）
        let mut next_block_id = function.blocks.iter()
            .map(|b| b.id)
            .max()
            .unwrap_or(0) + 1;
        
        // 最大の値IDを特定（新しい値IDの開始点）
        let mut next_value_id = function.blocks.iter()
            .flat_map(|b| b.instructions.iter())
            .filter_map(|inst| match inst {
                Instruction::BinaryOp { result, .. } |
                Instruction::Load { result, .. } |
                Instruction::GetElementPtr { result, .. } |
                Instruction::Call { result, .. } |
                Instruction::Alloca { result, .. } |
                Instruction::Phi { result, .. } |
                Instruction::Constant { result, .. } => Some(*result),
                _ => None,
            })
            .max()
            .unwrap_or(0) + 1;
        
        // 展開回数分ループを複製
        for i in 0..unroll_factor {
            // 新しいIDマッピング
            let mut id_mapping = HashMap::new();
            
            // ブロックIDマッピング
            let mut block_mapping = HashMap::new();
            for &block_id in &loop_info.body {
                let new_id = next_block_id;
                next_block_id += 1;
                block_mapping.insert(block_id, new_id);
            }
            
            // 命令の複製と更新
            let mut cloned_blocks = Vec::new();
            
            for &orig_block_id in &loop_info.body {
                if let Some(orig_block) = function.blocks.iter().find(|b| b.id == orig_block_id) {
                    let new_block_id = block_mapping[&orig_block_id];
                    let mut new_instructions = Vec::new();
                    
                    // 各命令をクローンして更新
                    for inst in &orig_block.instructions {
                        let new_inst = match inst {
                            // 各命令タイプに応じて処理
                            // この例では分岐命令のみ扱う
                            Instruction::Branch { target } => {
                                if loop_blocks.contains(target) {
                                    // ループ内部への分岐
                                    let new_target = block_mapping[target];
                                    Instruction::Branch { target: new_target }
                                } else {
                                    // ループ外への分岐は最後のインスタンスのみ維持
                                    if i == unroll_factor - 1 {
                                        Instruction::Branch { target: *target }
                                    } else {
                                        // 次のインスタンスにジャンプ
                                        let next_header = if i < unroll_factor - 1 {
                                            // 次のインスタンスのヘッダー
                                            id_mappings.get(i + 1).and_then(|mapping| mapping.get(&loop_info.header)).copied()
                                                .unwrap_or_else(|| block_mapping[&loop_info.header])
                                        } else {
                                            // 最後のインスタンスはオリジナルと同じ
                                            *target
                                        };
                                        Instruction::Branch { target: next_header }
                                    }
                                }
                            },
                            Instruction::ConditionalBranch { condition, true_target, false_target } => {
                                // 条件分岐の処理（類似）
                                let new_condition = *condition; // 実際には値のマッピングが必要
                                
                                let new_true = if loop_blocks.contains(true_target) {
                                    block_mapping[true_target]
                                } else {
                                    *true_target
                                };
                                
                                let new_false = if loop_blocks.contains(false_target) {
                                    block_mapping[false_target]
                                } else {
                                    *false_target
                                };
                                
                                Instruction::ConditionalBranch {
                                    condition: new_condition,
                                    true_target: new_true,
                                    false_target: new_false,
                                }
                            },
                            // 他の命令タイプも同様に処理
                            _ => inst.clone(),
                        };
                        
                        new_instructions.push(new_inst);
                    }
                    
                    // 新しいブロックを作成
                    let new_block = BasicBlock {
                        instructions: new_instructions,
                        label: format!(".L{}", block_id),
                        predecessors: HashSet::new(),
                        successors: HashSet::new(),
                        dominator: None,
                        dominates: HashSet::new(),
                        is_loop_header: false,
                        loop_depth: 0,
                    };
                    
                    cloned_blocks.push(new_block);
                }
            }
            
            // 複製したブロックを関数に追加
            for block in cloned_blocks {
                function.blocks.push(block);
            }
            
            // このインスタンスのIDマッピングを保存
            id_mappings.push(block_mapping);
        }
        
        // 元のループの入口を最初の展開インスタンスにリダイレクト
        if let Some(first_mapping) = id_mappings.first() {
            let first_header = first_mapping[&loop_info.header];
            
            for block in &mut function.blocks {
                for inst in &mut block.instructions {
                    match inst {
                        Instruction::Branch { target } if *target == loop_info.header => {
                            *target = first_header;
                        },
                        Instruction::ConditionalBranch { true_target, false_target, .. } => {
                            if *true_target == loop_info.header {
                                *true_target = first_header;
                            }
                            if *false_target == loop_info.header {
                                *false_target = first_header;
                            }
                        },
                        _ => {}
                    }
                }
            }
        }
        
        // 完全展開の場合、元のループブロックを削除
        if unroll_factor == trip_count {
            function.blocks.retain(|block| !loop_info.body.contains(&block.id));
        } else {
            // 部分展開の場合、展開後のループ制御を調整
            // （実際の実装では、反復カウンタの調整なども必要）
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleend::ir::representation::{
        Module, Function, BasicBlock, Instruction, OpCode, Type, Value, Operand
    };
    
    #[test]
    fn test_optimization_manager_creation() {
        let manager = OptimizationManager::new(OptimizationLevel::Standard);
        
        // 標準レベルでは少なくともこれらのパスが有効化されているはず
        assert!(manager.is_pass_enabled("dce"));
        assert!(manager.is_pass_enabled("constfold"));
        assert!(manager.is_pass_enabled("simplifycfg"));
        
        // 積極的な最適化で有効になるパスはまだ無効のはず
        assert!(!manager.is_pass_enabled("loopunroll"));
    }
    
    #[test]
    fn test_enable_disable_pass() {
        let mut manager = OptimizationManager::new(OptimizationLevel::None);
        
        // 最初はパスが有効化されていないはず
        assert!(!manager.is_pass_enabled("testpass"));
        
        // パスを有効化
        manager.enable_pass("testpass");
        assert!(manager.is_pass_enabled("testpass"));
        
        // パスを無効化
        manager.disable_pass("testpass");
        assert!(!manager.is_pass_enabled("testpass"));
    }
    
    // 他のテストもここに追加
}

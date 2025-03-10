// SwiftLight解析モジュール
//
// このモジュールはIRの静的解析のためのユーティリティを提供します。
// 主な役割は以下の通りです：
// - 制御フローグラフ（CFG）の構築と解析
// - データフロー解析
// - 依存関係解析
// - 別名解析
// - 到達定義解析

pub mod cfg;
pub mod dataflow;
pub mod dependency;
pub mod alias;
pub mod liveness;
pub mod dominator;

use std::collections::{HashMap, HashSet, VecDeque};
use crate::frontend::error::{Result, CompilerError};
use crate::middleend::ir::{Module, Function, BasicBlock, Instruction, OpCode, Value, Operand};

/// 制御フローグラフ解析
pub struct CFGAnalysis<'a> {
    /// 解析対象の関数
    function: &'a Function,
    
    /// 基本ブロックの先行ブロック
    predecessors: HashMap<String, HashSet<String>>,
    
    /// 基本ブロックの後続ブロック
    successors: HashMap<String, HashSet<String>>,
    
    /// エントリーブロック
    entry_block: Option<String>,
    
    /// 終了ブロック
    exit_blocks: HashSet<String>,
    
    /// 到達可能なブロック
    reachable_blocks: HashSet<String>,
}

impl<'a> CFGAnalysis<'a> {
    /// 新しいCFG解析インスタンスを作成
    pub fn new(function: &'a Function) -> Self {
        Self {
            function,
            predecessors: HashMap::new(),
            successors: HashMap::new(),
            entry_block: None,
            exit_blocks: HashSet::new(),
            reachable_blocks: HashSet::new(),
        }
    }
    
    /// CFGを構築
    pub fn build(&mut self) -> Result<()> {
        // 関数が空の場合はエラー
        if self.function.blocks.is_empty() {
            return Err(CompilerError::code_generation_error(
                format!("関数 '{}' にブロックがありません", self.function.name),
                None,
            ));
        }
        
        // エントリーブロックを設定
        self.entry_block = Some(self.function.blocks[0].label.clone());
        
        // 各ブロックについて処理
        for block in &self.function.blocks {
            let block_label = &block.label;
            
            // 後続ブロックのエントリを作成
            if !self.successors.contains_key(block_label) {
                self.successors.insert(block_label.clone(), HashSet::new());
            }
            
            // 先行ブロックのエントリを作成
            if !self.predecessors.contains_key(block_label) {
                self.predecessors.insert(block_label.clone(), HashSet::new());
            }
            
            // 最後の命令を確認して終了ブロックかどうか判断
            if let Some(last_inst) = block.instructions.last() {
                match last_inst.opcode {
                    OpCode::Return => {
                        // 終了ブロックとしてマーク
                        self.exit_blocks.insert(block_label.clone());
                    },
                    OpCode::Br => {
                        // 無条件分岐
                        if let Some(Operand::Block(target)) = last_inst.operands.get(0) {
                            // 後続ブロックとして追加
                            self.successors.get_mut(block_label).unwrap().insert(target.clone());
                            
                            // ターゲットブロックの先行ブロックマップにこのブロックを追加
                            if !self.predecessors.contains_key(target) {
                                self.predecessors.insert(target.clone(), HashSet::new());
                            }
                            self.predecessors.get_mut(target).unwrap().insert(block_label.clone());
                        }
                    },
                    OpCode::CondBr => {
                        // 条件分岐
                        if let (Some(Operand::Block(true_target)), Some(Operand::Block(false_target))) = 
                            (last_inst.operands.get(1), last_inst.operands.get(2)) {
                            // 両方の後続ブロックとして追加
                            self.successors.get_mut(block_label).unwrap().insert(true_target.clone());
                            self.successors.get_mut(block_label).unwrap().insert(false_target.clone());
                            
                            // 両方のターゲットブロックの先行ブロックマップにこのブロックを追加
                            if !self.predecessors.contains_key(true_target) {
                                self.predecessors.insert(true_target.clone(), HashSet::new());
                            }
                            self.predecessors.get_mut(true_target).unwrap().insert(block_label.clone());
                            
                            if !self.predecessors.contains_key(false_target) {
                                self.predecessors.insert(false_target.clone(), HashSet::new());
                            }
                            self.predecessors.get_mut(false_target).unwrap().insert(block_label.clone());
                        }
                    },
                    OpCode::Switch => {
                        // スイッチ文
                        // スイッチ命令のオペランドは condition, default, (value1, label1), (value2, label2), ...
                        // よって1番目は条件式、2番目はデフォルトラベル、その後はペアになっている
                        if let Some(Operand::Block(default_target)) = last_inst.operands.get(1) {
                            // デフォルトの後続ブロックとして追加
                            self.successors.get_mut(block_label).unwrap().insert(default_target.clone());
                            
                            // デフォルトターゲットブロックの先行ブロックマップにこのブロックを追加
                            if !self.predecessors.contains_key(default_target) {
                                self.predecessors.insert(default_target.clone(), HashSet::new());
                            }
                            self.predecessors.get_mut(default_target).unwrap().insert(block_label.clone());
                            
                            // ケースラベルを処理
                            for i in (2..last_inst.operands.len()).step_by(2) {
                                if let Some(Operand::Block(case_target)) = last_inst.operands.get(i + 1) {
                                    // ケースの後続ブロックとして追加
                                    self.successors.get_mut(block_label).unwrap().insert(case_target.clone());
                                    
                                    // ケースターゲットブロックの先行ブロックマップにこのブロックを追加
                                    if !self.predecessors.contains_key(case_target) {
                                        self.predecessors.insert(case_target.clone(), HashSet::new());
                                    }
                                    self.predecessors.get_mut(case_target).unwrap().insert(block_label.clone());
                                }
                            }
                        }
                    },
                    _ => {
                        // 通常の命令で終わるブロックは暗黙的にフォールスルー（次のブロックに続く）
                        // ただし、これが最後のブロックであれば終了ブロックとする
                        if let Some(next_block_idx) = self.function.blocks.iter().position(|b| b.label == *block_label).map(|i| i + 1) {
                            if next_block_idx < self.function.blocks.len() {
                                let next_block_label = &self.function.blocks[next_block_idx].label;
                                
                                // 後続ブロックとして追加
                                self.successors.get_mut(block_label).unwrap().insert(next_block_label.clone());
                                
                                // 次のブロックの先行ブロックマップにこのブロックを追加
                                if !self.predecessors.contains_key(next_block_label) {
                                    self.predecessors.insert(next_block_label.clone(), HashSet::new());
                                }
                                self.predecessors.get_mut(next_block_label).unwrap().insert(block_label.clone());
                            } else {
                                // 最後のブロックは終了ブロックとする
                                self.exit_blocks.insert(block_label.clone());
                            }
                        } else {
                            // ブロックが見つからない場合は何らかのエラー
                            return Err(CompilerError::code_generation_error(
                                format!("CFG構築中にブロック '{}' が見つかりません", block_label),
                                None,
                            ));
                        }
                    }
                }
            } else {
                // 命令がないブロックはフォールスルー
                if let Some(next_block_idx) = self.function.blocks.iter().position(|b| b.label == *block_label).map(|i| i + 1) {
                    if next_block_idx < self.function.blocks.len() {
                        let next_block_label = &self.function.blocks[next_block_idx].label;
                        
                        // 後続ブロックとして追加
                        self.successors.get_mut(block_label).unwrap().insert(next_block_label.clone());
                        
                        // 次のブロックの先行ブロックマップにこのブロックを追加
                        if !self.predecessors.contains_key(next_block_label) {
                            self.predecessors.insert(next_block_label.clone(), HashSet::new());
                        }
                        self.predecessors.get_mut(next_block_label).unwrap().insert(block_label.clone());
                    } else {
                        // 最後のブロックは終了ブロックとする
                        self.exit_blocks.insert(block_label.clone());
                    }
                }
            }
        }
        
        // 到達可能なブロックを計算
        self.compute_reachable_blocks();
        
        Ok(())
    }
    
    /// エントリーブロックから到達可能なブロックを計算
    fn compute_reachable_blocks(&mut self) {
        if let Some(entry) = &self.entry_block {
            let mut visited = HashSet::new();
            let mut queue = VecDeque::new();
            
            // エントリーブロックから開始
            queue.push_back(entry.clone());
            visited.insert(entry.clone());
            
            // 幅優先探索
            while let Some(block) = queue.pop_front() {
                // このブロックを到達可能としてマーク
                self.reachable_blocks.insert(block.clone());
                
                // 後続ブロックをキューに追加
                if let Some(succs) = self.successors.get(&block) {
                    for succ in succs {
                        if !visited.contains(succ) {
                            queue.push_back(succ.clone());
                            visited.insert(succ.clone());
                        }
                    }
                }
            }
        }
    }
    
    /// ブロックの先行ブロックを取得
    pub fn get_predecessors(&self, block: &str) -> Option<&HashSet<String>> {
        self.predecessors.get(block)
    }
    
    /// ブロックの後続ブロックを取得
    pub fn get_successors(&self, block: &str) -> Option<&HashSet<String>> {
        self.successors.get(block)
    }
    
    /// エントリーブロックを取得
    pub fn get_entry_block(&self) -> Option<&String> {
        self.entry_block.as_ref()
    }
    
    /// 終了ブロックを取得
    pub fn get_exit_blocks(&self) -> &HashSet<String> {
        &self.exit_blocks
    }
    
    /// ブロックが到達可能かどうか
    pub fn is_reachable(&self, block: &str) -> bool {
        self.reachable_blocks.contains(block)
    }
    
    /// 到達不能なブロックを取得
    pub fn get_unreachable_blocks(&self) -> HashSet<String> {
        let all_blocks: HashSet<String> = self.function.blocks.iter()
            .map(|block| block.label.clone())
            .collect();
        
        // 全ブロックから到達可能なブロックを除外
        all_blocks.difference(&self.reachable_blocks).cloned().collect()
    }
}

/// データフロー解析の種類
pub enum DataFlowDirection {
    /// 前方向解析（エントリーからすべてのブロックへ）
    Forward,
    /// 後ろ向き解析（すべてのブロックからエントリへ）
    Backward,
}

/// データフロー解析フレームワーク
pub struct DataFlowAnalysis<'a, T> {
    /// 制御フローグラフ解析
    cfg: &'a CFGAnalysis<'a>,
    
    /// 解析の方向
    direction: DataFlowDirection,
    
    /// 各ブロックの入口での解析値
    in_values: HashMap<String, T>,
    
    /// 各ブロックの出口での解析値
    out_values: HashMap<String, T>,
}

impl<'a, T: Clone + PartialEq> DataFlowAnalysis<'a, T> {
    /// 新しいデータフロー解析インスタンスを作成
    pub fn new(cfg: &'a CFGAnalysis<'a>, direction: DataFlowDirection, initial_value: T) -> Self {
        let mut in_values = HashMap::new();
        let mut out_values = HashMap::new();
        
        // すべてのブロックに初期値を設定
        for block in &cfg.function.blocks {
            in_values.insert(block.label.clone(), initial_value.clone());
            out_values.insert(block.label.clone(), initial_value.clone());
        }
        
        Self {
            cfg,
            direction,
            in_values,
            out_values,
        }
    }
    
    /// データフロー解析を実行
    pub fn analyze<F>(&mut self, transfer_function: F, meet_operator: fn(&[&T]) -> T) -> Result<()>
    where
        F: Fn(&BasicBlock, &T) -> T,
    {
        // 反復データフロー解析を実行
        let mut changed = true;
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 1000; // 安全装置
        
        while changed && iterations < MAX_ITERATIONS {
            changed = false;
            iterations += 1;
            
            // 方向に応じた処理順序
            let block_order = match self.direction {
                DataFlowDirection::Forward => self.cfg.function.blocks.iter().collect::<Vec<_>>(),
                DataFlowDirection::Backward => {
                    let mut reverse_order = self.cfg.function.blocks.iter().collect::<Vec<_>>();
                    reverse_order.reverse();
                    reverse_order
                }
            };
            
            for block in block_order {
                let block_label = &block.label;
                
                // 現在の入力値を計算
                let new_in = match self.direction {
                    DataFlowDirection::Forward => {
                        // 前方向解析では、先行ブロックの出力値の合流
                        if let Some(preds) = self.cfg.get_predecessors(block_label) {
                            if preds.is_empty() {
                                // エントリーブロックの場合は現在の入力値を維持
                                self.in_values[block_label].clone()
                            } else {
                                // 先行ブロックの出力値を合流
                                let pred_outs: Vec<&T> = preds.iter()
                                    .filter_map(|pred| self.out_values.get(pred))
                                    .collect();
                                
                                if !pred_outs.is_empty() {
                                    meet_operator(&pred_outs)
                                } else {
                                    self.in_values[block_label].clone()
                                }
                            }
                        } else {
                            // 先行ブロック情報がない場合は現在の入力値を維持
                            self.in_values[block_label].clone()
                        }
                    },
                    DataFlowDirection::Backward => {
                        // 後ろ向き解析では、後続ブロックの入力値の合流
                        if let Some(succs) = self.cfg.get_successors(block_label) {
                            if succs.is_empty() {
                                // 終了ブロックの場合は現在の入力値を維持
                                self.in_values[block_label].clone()
                            } else {
                                // 後続ブロックの入力値を合流
                                let succ_ins: Vec<&T> = succs.iter()
                                    .filter_map(|succ| self.in_values.get(succ))
                                    .collect();
                                
                                if !succ_ins.is_empty() {
                                    meet_operator(&succ_ins)
                                } else {
                                    self.in_values[block_label].clone()
                                }
                            }
                        } else {
                            // 後続ブロック情報がない場合は現在の入力値を維持
                            self.in_values[block_label].clone()
                        }
                    }
                };
                
                // 入力値が変化したかチェック
                if new_in != self.in_values[block_label] {
                    changed = true;
                    self.in_values.insert(block_label.clone(), new_in.clone());
                }
                
                // 転送関数を適用して出力値を計算
                let new_out = transfer_function(block, &new_in);
                
                // 出力値が変化したかチェック
                if new_out != self.out_values[block_label] {
                    changed = true;
                    self.out_values.insert(block_label.clone(), new_out);
                }
            }
        }
        
        if iterations >= MAX_ITERATIONS {
            return Err(CompilerError::code_generation_error(
                format!("データフロー解析が収束しませんでした（関数: {}）", self.cfg.function.name),
                None,
            ));
        }
        
        Ok(())
    }
    
    /// ブロックの入力値を取得
    pub fn get_in(&self, block: &str) -> Option<&T> {
        self.in_values.get(block)
    }
    
    /// ブロックの出力値を取得
    pub fn get_out(&self, block: &str) -> Option<&T> {
        self.out_values.get(block)
    }
}

/// 基本的な依存関係解析
pub fn analyze_dependencies(module: &Module) -> Result<HashMap<String, HashSet<String>>> {
    let mut dependencies = HashMap::new();
    
    // 各関数の依存関係を解析
    for (name, function) in &module.functions {
        let mut function_deps = HashSet::new();
        
        // 呼び出される関数を検出
        for block in &function.blocks {
            for inst in &block.instructions {
                if inst.opcode == OpCode::Call {
                    // Call命令の最初のオペランドは呼び出し先の関数
                    if let Some(operand) = inst.operands.first() {
                        match operand {
                            Operand::Function(callee_name) => {
                                // 自己再帰呼び出しは依存関係に含めない
                                if callee_name != name {
                                    function_deps.insert(callee_name.clone());
                                }
                            },
                            _ => { /* その他のオペランドタイプは関数呼び出しではない */ }
                        }
                    }
                }
            }
        }
        
        dependencies.insert(name.clone(), function_deps);
    }
    
    Ok(dependencies)
}

/// 別名解析（alias analysis）
pub fn analyze_aliases(function: &Function) -> Result<HashMap<String, HashSet<String>>> {
    let mut aliases = HashMap::new();
    
    // 各ブロックの命令を検査
    for block in &function.blocks {
        for inst in &block.instructions {
            // ポインタ操作を検出
            match inst.opcode {
                OpCode::GetElementPtr | OpCode::Load | OpCode::Store => {
                    if let Some(result) = &inst.result {
                        // 新しい別名セットを作成
                        let mut alias_set = HashSet::new();
                        
                        // オペランドを検査して別名関係を構築
                        for operand in &inst.operands {
                            if let Operand::Register(reg_name) = operand {
                                // 既存の別名セットを取得
                                if let Some(existing_aliases) = aliases.get(reg_name) {
                                    // 既存の別名をすべて追加
                                    for alias in existing_aliases {
                                        alias_set.insert(alias.clone());
                                    }
                                }
                                
                                // このオペランドも別名として追加
                                alias_set.insert(reg_name.clone());
                            }
                        }
                        
                        // 結果変数に別名セットを関連付け
                        aliases.insert(result.clone(), alias_set);
                    }
                },
                OpCode::Alloca => {
                    // 新しいメモリ割り当ては一意のポインタを作成
                    if let Some(result) = &inst.result {
                        let mut alias_set = HashSet::new();
                        alias_set.insert(result.clone());
                        aliases.insert(result.clone(), alias_set);
                    }
                },
                OpCode::BitCast | OpCode::PtrToInt | OpCode::IntToPtr => {
                    // ポインタのキャスト操作は別名関係を維持
                    if let (Some(result), Some(Operand::Register(src_reg))) = (&inst.result, inst.operands.first()) {
                        if let Some(src_aliases) = aliases.get(src_reg) {
                            aliases.insert(result.clone(), src_aliases.clone());
                        } else {
                            let mut alias_set = HashSet::new();
                            alias_set.insert(src_reg.clone());
                            aliases.insert(result.clone(), alias_set);
                        }
                    }
                },
                OpCode::Phi => {
                    // Phi命令は複数の値の合流点
                    if let Some(result) = &inst.result {
                        let mut alias_set = HashSet::new();
                        
                        // Phiのすべての入力値の別名を合併
                        for i in (0..inst.operands.len()).step_by(2) {
                            if let Some(Operand::Register(src_reg)) = inst.operands.get(i) {
                                if let Some(src_aliases) = aliases.get(src_reg) {
                                    for alias in src_aliases {
                                        alias_set.insert(alias.clone());
                                    }
                                } else {
                                    alias_set.insert(src_reg.clone());
                                }
                            }
                        }
                        
                        aliases.insert(result.clone(), alias_set);
                    }
                },
                _ => {
                    // その他の命令は別名関係に影響しない
                }
            }
        }
    }
    
    Ok(aliases)
}

/// 到達定義解析（reaching definitions analysis）
pub fn analyze_reaching_definitions(function: &Function) -> Result<HashMap<String, HashSet<(String, usize)>>> {
    // CFG解析を実行
    let mut cfg_analysis = CFGAnalysis::new(function);
    cfg_analysis.build()?;
    
    // 各変数の定義位置を収集
    let mut all_defs = HashSet::new();
    for (block_idx, block) in function.blocks.iter().enumerate() {
        for (inst_idx, inst) in block.instructions.iter().enumerate() {
            // 変数に値を代入する命令を検査
            if let Some(result) = &inst.result {
                // (変数名, 命令の位置) のペアを記録
                all_defs.insert((result.clone(), (block_idx, inst_idx)));
            }
        }
    }
    
    // 結果はブロックごとの到達定義の集合
    let mut reaching_defs = HashMap::new();
    
    // 各ブロックに対して処理
    for block in &function.blocks {
        let mut block_defs = HashSet::new();
        
        // ブロック内の各命令を処理
        for (inst_idx, inst) in block.instructions.iter().enumerate() {
            // 命令が変数を定義する場合
            if let Some(result) = &inst.result {
                // 新しい定義を追加（同じ変数の古い定義はキルする）
                // 同じ変数の古い定義を削除
                block_defs.retain(|(var, _)| var != result);
                
                // 新しい定義を追加
                block_defs.insert((result.clone(), (block.label.clone(), inst_idx)));
            }
        }
        
        reaching_defs.insert(block.label.clone(), block_defs);
    }
    
    Ok(reaching_defs)
}

/// 生存変数解析（liveness analysis）
pub fn analyze_liveness(function: &Function) -> Result<HashMap<String, HashSet<String>>> {
    // CFG解析を実行
    let mut cfg_analysis = CFGAnalysis::new(function);
    cfg_analysis.build()?;
    
    // 各ブロックの生存変数を保持するマップ
    let mut live_out = HashMap::new();
    
    // 各ブロックに空の生存変数セットを初期化
    for block in &function.blocks {
        live_out.insert(block.label.clone(), HashSet::new());
    }
    
    // 反復的に生存変数を計算
    let mut changed = true;
    while changed {
        changed = false;
        
        // 各ブロックを逆順に処理（後ろ向き解析）
        for block in function.blocks.iter().rev() {
            let block_label = &block.label;
            
            // このブロックの現在の生存変数セット
            let mut current_live = HashSet::new();
            
            // 後続ブロックの生存変数を合併
            if let Some(successors) = cfg_analysis.get_successors(block_label) {
                for succ in successors {
                    if let Some(succ_live) = live_out.get(succ) {
                        for var in succ_live {
                            current_live.insert(var.clone());
                        }
                    }
                }
            }
            
            // ブロック内の命令を逆順に処理
            for inst in block.instructions.iter().rev() {
                // 定義された変数は生存変数から削除
                if let Some(result) = &inst.result {
                    current_live.remove(result);
                }
                
                // 使用された変数は生存変数に追加
                for operand in &inst.operands {
                    if let Operand::Register(reg_name) = operand {
                        current_live.insert(reg_name.clone());
                    }
                }
            }
            
            // 生存変数セットが変化したかチェック
            let old_live = live_out.get(block_label).unwrap();
            if &current_live != old_live {
                changed = true;
                live_out.insert(block_label.clone(), current_live);
            }
        }
    }
    
    Ok(live_out)
}

/// 支配木解析（dominator tree analysis）
pub fn analyze_dominators(function: &Function) -> Result<HashMap<String, HashSet<String>>> {
    // CFG解析を実行
    let mut cfg_analysis = CFGAnalysis::new(function);
    cfg_analysis.build()?;
    
    // 各ブロックの支配ブロックを保持するマップ
    let mut dominators = HashMap::new();
    
    // すべてのブロックのリスト
    let all_blocks: Vec<String> = function.blocks.iter()
        .map(|block| block.label.clone())
        .collect();
    
    // エントリーブロックを取得
    let entry_block = match cfg_analysis.get_entry_block() {
        Some(entry) => entry.clone(),
        None => return Err(CompilerError::code_generation_error(
            "支配木解析: エントリーブロックがありません".to_string(),
            None,
#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleend::ir::{
        Module, Function, BasicBlock, Instruction, OpCode, Type, Value, Operand
    };
    
    // CFG解析のテスト
    #[test]
    fn test_cfg_analysis_simple() {
        // テスト用の関数を作成
        let mut function = Function::new("test", Type::Void);
        
        // 基本ブロックを追加
        let mut entry_block = BasicBlock::new("entry");
        let mut bb1 = BasicBlock::new("bb1");
        let mut bb2 = BasicBlock::new("bb2");
        let mut exit_block = BasicBlock::new("exit");
        
        // エントリーブロックに条件分岐を追加
        entry_block.add_instruction(Instruction::new(
            OpCode::CondBr,
            None,
            Type::Void,
            vec![
                Operand::Register("cond".to_string()),
                Operand::Block("bb1".to_string()),
                Operand::Block("bb2".to_string()),
            ]
        ));
        
        // bb1からexit_blockへの無条件分岐
        bb1.add_instruction(Instruction::new(
            OpCode::Br,
            None,
            Type::Void,
            vec![Operand::Block("exit".to_string())],
        ));
        
        // bb2からexit_blockへの無条件分岐
        bb2.add_instruction(Instruction::new(
            OpCode::Br,
            None,
            Type::Void,
            vec![Operand::Block("exit".to_string())],
        ));
        
        // exit_blockにreturn命令
        exit_block.add_instruction(Instruction::new(
            OpCode::Return,
            None,
            Type::Void,
            vec![],
        ));
        
        // 関数にブロックを追加
        function.add_block(entry_block);
        function.add_block(bb1);
        function.add_block(bb2);
        function.add_block(exit_block);
        
        // CFG解析を実行
        let mut cfg = CFGAnalysis::new(&function);
        cfg.build().unwrap();
        
        // エントリーブロックのチェック
        assert_eq!(cfg.get_entry_block(), Some(&"entry".to_string()));
        
        // 終了ブロックのチェック
        assert!(cfg.get_exit_blocks().contains("exit"));
        
        // 先行・後続ブロックのチェック
        assert_eq!(cfg.get_predecessors("entry"), Some(&HashSet::new()));
        
        let mut expected_entry_succs = HashSet::new();
        expected_entry_succs.insert("bb1".to_string());
        expected_entry_succs.insert("bb2".to_string());
        assert_eq!(cfg.get_successors("entry"), Some(&expected_entry_succs));
        
        let mut expected_bb1_preds = HashSet::new();
        expected_bb1_preds.insert("entry".to_string());
        assert_eq!(cfg.get_predecessors("bb1"), Some(&expected_bb1_preds));
        
        let mut expected_bb1_succs = HashSet::new();
        expected_bb1_succs.insert("exit".to_string());
        assert_eq!(cfg.get_successors("bb1"), Some(&expected_bb1_succs));
        
        let mut expected_exit_preds = HashSet::new();
        expected_exit_preds.insert("bb1".to_string());
        expected_exit_preds.insert("bb2".to_string());
        assert_eq!(cfg.get_predecessors("exit"), Some(&expected_exit_preds));
        
        // 到達可能性チェック
        assert!(cfg.is_reachable("entry"));
        assert!(cfg.is_reachable("bb1"));
        assert!(cfg.is_reachable("bb2"));
        assert!(cfg.is_reachable("exit"));
        assert!(cfg.get_unreachable_blocks().is_empty());
    }
    
    // データフロー解析のテストもここに追加
}

// dataflow.rs - SwiftLight データフロー解析モジュール
//
// このモジュールは、SwiftLight IR に対するデータフロー解析を実装します。
// 主な機能:
//  - 到達定義解析 (Reaching Definitions)
//  - 活性変数解析 (Live Variable Analysis)
//  - 定数伝搬解析 (Constant Propagation)
//  - 使用-定義連鎖 (Use-Definition Chains)
//  - 定義-使用連鎖 (Definition-Use Chains)

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

use crate::middleend::ir::{
    BasicBlock, Function, Instruction, Module, Value, ValueId, 
    Type, TypeId, ControlFlowGraph, InstructionId
};

/// データフロー解析のタイプ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataFlowType {
    /// 前向き解析 (例: 到達定義)
    Forward,
    /// 後ろ向き解析 (例: 活性変数)
    Backward,
}

/// データフロー解析の結果を表す構造体
/// T: 各命令/ブロックに対する解析結果の型
#[derive(Clone)]
pub struct DataFlowResult<T: Clone + PartialEq + Eq> {
    /// 各基本ブロックの入口における解析結果
    pub in_sets: HashMap<usize, T>,
    /// 各基本ブロックの出口における解析結果
    pub out_sets: HashMap<usize, T>,
    /// 各命令の前における解析結果
    pub before_inst: HashMap<InstructionId, T>,
    /// 各命令の後における解析結果
    pub after_inst: HashMap<InstructionId, T>,
}

impl<T: Clone + PartialEq + Eq + fmt::Debug> fmt::Debug for DataFlowResult<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "DataFlowResult {{")?;
        writeln!(f, "  in_sets: {:?}", self.in_sets)?;
        writeln!(f, "  out_sets: {:?}", self.out_sets)?;
        writeln!(f, "  before_inst: {:?}", self.before_inst)?;
        writeln!(f, "  after_inst: {:?}", self.after_inst)?;
        writeln!(f, "}}")
    }
}

/// データフロー解析のための抽象トレイト
pub trait DataFlowAnalysis<T: Clone + PartialEq + Eq> {
    /// 解析の方向を返す
    fn direction(&self) -> DataFlowType;
    
    /// 初期値を返す
    fn initial_value(&self) -> T;
    
    /// ボトム値を返す
    fn bottom_value(&self) -> T;
    
    /// 特定の命令に対するgen集合を計算する
    fn gen(&self, inst: &Instruction, function: &Function) -> T;
    
    /// 特定の命令に対するkill集合を計算する
    fn kill(&self, inst: &Instruction, function: &Function) -> T;
    
    /// 2つの集合をマージする (∪ または ∩)
    fn meet(&self, left: &T, right: &T) -> T;
    
    /// 命令に対する転送関数 (transfer function) を適用
    fn transfer(&self, inst: &Instruction, function: &Function, in_value: &T) -> T {
        let gen_set = self.gen(inst, function);
        let kill_set = self.kill(inst, function);
        
        // 一般的な転送関数: out = gen ∪ (in - kill)
        // ただし、解析によって異なる場合はオーバーライドする
        self.apply_transfer(in_value, &gen_set, &kill_set)
    }
    
    /// 転送関数の実際の計算を行う
    fn apply_transfer(&self, in_value: &T, gen_set: &T, kill_set: &T) -> T;
}

/// データフロー解析エンジン
pub struct DataFlowEngine;

impl DataFlowEngine {
    /// 新しいデータフロー解析エンジンを作成
    pub fn new() -> Self {
        Self {}
    }
    
    /// データフロー解析を実行する
    /// 解析が収束するまで繰り返し計算を行い、結果を返す
    pub fn analyze<T, A>(&self, function: &Function, analysis: &A) -> DataFlowResult<T>
    where
        T: Clone + PartialEq + Eq + fmt::Debug,
        A: DataFlowAnalysis<T>,
    {
        let cfg = function.control_flow_graph();
        let direction = analysis.direction();
        
        // 結果格納用のマップを初期化
        let mut in_sets = HashMap::new();
        let mut out_sets = HashMap::new();
        let mut before_inst = HashMap::new();
        let mut after_inst = HashMap::new();
        
        // 処理対象のブロックを決定
        let blocks: Vec<usize> = match direction {
            DataFlowType::Forward => (0..function.basic_blocks.len()).collect(),
            DataFlowType::Backward => (0..function.basic_blocks.len()).rev().collect(),
        };
        
        // in集合とout集合を初期化
        for &block_id in &blocks {
            if direction == DataFlowType::Forward && block_id == 0 {
                // 開始ブロックの場合は初期値を設定
                in_sets.insert(block_id, analysis.initial_value());
            } else {
                // その他のブロックの場合はボトム値を設定
                in_sets.insert(block_id, analysis.bottom_value());
            }
            
            out_sets.insert(block_id, analysis.bottom_value());
        }
        
        // データフロー方程式が収束するまで繰り返す
        let mut changed = true;
        while changed {
            changed = false;
            
            for &block_id in &blocks {
                let block = &function.basic_blocks[block_id];
                
                // ブロックの入口/出口の値を計算
                if direction == DataFlowType::Forward {
                    // 前向き解析の場合
                    // in[B] = ∩ out[P] for all predecessors P of B
                    // 先頭ブロックの場合は既に初期値が設定されている
                    if block_id != 0 {
                        let mut new_in = analysis.initial_value();
                        let mut first = true;
                        
                        for &pred in &cfg.predecessors[block_id] {
                            if first {
                                new_in = out_sets[&pred].clone();
                                first = false;
                            } else {
                                new_in = analysis.meet(&new_in, &out_sets[&pred]);
                            }
                        }
                        
                        if new_in != in_sets[&block_id] {
                            in_sets.insert(block_id, new_in);
                            changed = true;
                        }
                    }
                    
                    // ブロック内の命令を順に処理
                    let mut current = in_sets[&block_id].clone();
                    for inst_id in &block.instructions {
                        let inst = &function.instructions[*inst_id];
                        before_inst.insert(*inst_id, current.clone());
                        current = analysis.transfer(inst, function, &current);
                        after_inst.insert(*inst_id, current.clone());
                    }
                    
                    // ブロックの出口値を更新
                    if current != out_sets[&block_id] {
                        out_sets.insert(block_id, current);
                        changed = true;
                    }
                } else {
                    // 後ろ向き解析の場合
                    // out[B] = ∩ in[S] for all successors S of B
                    let mut new_out = analysis.initial_value();
                    let mut first = true;
                    
                    for &succ in &cfg.successors[block_id] {
                        if first {
                            new_out = in_sets[&succ].clone();
                            first = false;
                        } else {
                            new_out = analysis.meet(&new_out, &in_sets[&succ]);
                        }
                    }
                    
                    // 終端ブロックで後続がない場合は初期値を使用
                    if cfg.successors[block_id].is_empty() {
                        new_out = analysis.initial_value();
                    }
                    
                    if new_out != out_sets[&block_id] {
                        out_sets.insert(block_id, new_out.clone());
                        changed = true;
                    }
                    
                    // ブロック内の命令を逆順に処理
                    let mut current = new_out;
                    for inst_id in block.instructions.iter().rev() {
                        let inst = &function.instructions[*inst_id];
                        after_inst.insert(*inst_id, current.clone());
                        current = analysis.transfer(inst, function, &current);
                        before_inst.insert(*inst_id, current.clone());
                    }
                    
                    // ブロックの入口値を更新
                    if current != in_sets[&block_id] {
                        in_sets.insert(block_id, current);
                        changed = true;
                    }
                }
            }
        }
        
        DataFlowResult {
            in_sets,
            out_sets,
            before_inst,
            after_inst,
        }
    }
}

/// 活性変数解析 (Live Variable Analysis) の実装
pub struct LiveVariableAnalysis;

impl LiveVariableAnalysis {
    pub fn new() -> Self {
        Self {}
    }
}

impl DataFlowAnalysis<HashSet<ValueId>> for LiveVariableAnalysis {
    fn direction(&self) -> DataFlowType {
        DataFlowType::Backward
    }
    
    fn initial_value(&self) -> HashSet<ValueId> {
        HashSet::new()
    }
    
    fn bottom_value(&self) -> HashSet<ValueId> {
        HashSet::new()
    }
    
    fn gen(&self, inst: &Instruction, function: &Function) -> HashSet<ValueId> {
        // gen集合: 命令で使用される変数（定義の前に使用される変数）
        let mut gen_set = HashSet::new();
        
        for &operand_id in &inst.operands {
            // 定数や関数参照など、変数以外は無視
            if let Some(value) = function.get_value(operand_id) {
                match value {
                    Value::Variable { .. } => {
                        gen_set.insert(operand_id);
                    }
                    _ => {}
                }
            }
        }
        
        gen_set
    }
    
    fn kill(&self, inst: &Instruction, _function: &Function) -> HashSet<ValueId> {
        // kill集合: 命令で定義される変数
        let mut kill_set = HashSet::new();
        
        if let Some(result_id) = inst.result {
            kill_set.insert(result_id);
        }
        
        kill_set
    }
    
    fn meet(&self, left: &HashSet<ValueId>, right: &HashSet<ValueId>) -> HashSet<ValueId> {
        // 活性変数解析では和集合を使用
        left.union(right).cloned().collect()
    }
    
    fn apply_transfer(&self, in_value: &HashSet<ValueId>, gen_set: &HashSet<ValueId>, kill_set: &HashSet<ValueId>) -> HashSet<ValueId> {
        // out = gen ∪ (in - kill)
        let mut result = in_value.clone();
        
        // killされた変数を削除
        for &kill_id in kill_set {
            result.remove(&kill_id);
        }
        
        // genされた変数を追加
        for &gen_id in gen_set {
            result.insert(gen_id);
        }
        
        result
    }
}

/// 到達定義解析 (Reaching Definitions) の実装
pub struct ReachingDefinitionAnalysis {
    /// 関数内のすべての定義命令ID
    definitions: HashSet<InstructionId>,
}

impl ReachingDefinitionAnalysis {
    pub fn new(function: &Function) -> Self {
        // 関数内のすべての定義命令を収集
        let mut definitions = HashSet::new();
        
        for (inst_id, inst) in function.instructions.iter().enumerate() {
            if inst.result.is_some() {
                definitions.insert(inst_id);
            }
        }
        
        Self { definitions }
    }
}

impl DataFlowAnalysis<HashSet<InstructionId>> for ReachingDefinitionAnalysis {
    fn direction(&self) -> DataFlowType {
        DataFlowType::Forward
    }
    
    fn initial_value(&self) -> HashSet<InstructionId> {
        // 入口には最初は定義がない
        HashSet::new()
    }
    
    fn bottom_value(&self) -> HashSet<InstructionId> {
        // ボトム値は空集合
        HashSet::new()
    }
    
    fn gen(&self, inst: &Instruction, _function: &Function) -> HashSet<InstructionId> {
        // gen集合: 命令自体が定義を生成するなら、その命令ID
        let mut gen_set = HashSet::new();
        
        if inst.result.is_some() {
            gen_set.insert(inst.id);
        }
        
        gen_set
    }
    
    fn kill(&self, inst: &Instruction, function: &Function) -> HashSet<InstructionId> {
        // kill集合: 同じ変数を定義する他の命令
        let mut kill_set = HashSet::new();
        
        if let Some(result_id) = inst.result {
            for &def_id in &self.definitions {
                let def_inst = &function.instructions[def_id];
                if let Some(def_result) = def_inst.result {
                    // 同じ変数を定義する命令を見つける
                    if def_result == result_id && def_id != inst.id {
                        kill_set.insert(def_id);
                    }
                }
            }
        }
        
        kill_set
    }
    
    fn meet(&self, left: &HashSet<InstructionId>, right: &HashSet<InstructionId>) -> HashSet<InstructionId> {
        // 到達定義解析では和集合を使用
        left.union(right).cloned().collect()
    }
    
    fn apply_transfer(&self, in_value: &HashSet<InstructionId>, gen_set: &HashSet<InstructionId>, kill_set: &HashSet<InstructionId>) -> HashSet<InstructionId> {
        // out = gen ∪ (in - kill)
        let mut result = in_value.clone();
        
        // killされた定義を削除
        for &kill_id in kill_set {
            result.remove(&kill_id);
        }
        
        // genされた定義を追加
        for &gen_id in gen_set {
            result.insert(gen_id);
        }
        
        result
    }
}

/// 定数伝搬解析 (Constant Propagation) の実装
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ConstantState {
    /// 値が未定義
    Undefined,
    /// 定数値
    Constant(i64),
    /// 非定数 (変数など)
    NonConstant,
}

pub struct ConstantPropagationAnalysis {
    /// 各変数のConstantState
    variable_states: HashMap<ValueId, ConstantState>,
}

impl ConstantPropagationAnalysis {
    pub fn new() -> Self {
        Self {
            variable_states: HashMap::new(),
        }
    }
}

impl DataFlowAnalysis<HashMap<ValueId, ConstantState>> for ConstantPropagationAnalysis {
    fn direction(&self) -> DataFlowType {
        DataFlowType::Forward
    }
    
    fn initial_value(&self) -> HashMap<ValueId, ConstantState> {
        HashMap::new()
    }
    
    fn bottom_value(&self) -> HashMap<ValueId, ConstantState> {
        HashMap::new()
    }
    
    fn gen(&self, inst: &Instruction, function: &Function) -> HashMap<ValueId, ConstantState> {
        let mut gen_map = HashMap::new();
        
        if let Some(result_id) = inst.result {
            // 定数畳み込みを適用
            let state = self.evaluate_instruction(inst, function);
            gen_map.insert(result_id, state);
        }
        
        gen_map
    }
    
    fn kill(&self, _inst: &Instruction, _function: &Function) -> HashMap<ValueId, ConstantState> {
        // 定数伝搬ではkill集合は使わない（genで上書きする）
        HashMap::new()
    }
    
    fn meet(&self, left: &HashMap<ValueId, ConstantState>, right: &HashMap<ValueId, ConstantState>) -> HashMap<ValueId, ConstantState> {
        let mut result = left.clone();
        
        for (&var_id, &right_state) in right {
            if let Some(left_state) = result.get(&var_id).cloned() {
                // 両方の経路に存在する場合は合流
                let merged_state = match (left_state, right_state) {
                    (ConstantState::Undefined, other) => other,
                    (other, ConstantState::Undefined) => other,
                    (ConstantState::Constant(c1), ConstantState::Constant(c2)) => {
                        if c1 == c2 {
                            ConstantState::Constant(c1)
                        } else {
                            ConstantState::NonConstant
                        }
                    },
                    _ => ConstantState::NonConstant,
                };
                
                result.insert(var_id, merged_state);
            } else {
                // 右側だけに存在する場合はそのまま追加
                result.insert(var_id, right_state);
            }
        }
        
        result
    }
    
    fn apply_transfer(&self, in_value: &HashMap<ValueId, ConstantState>, gen_set: &HashMap<ValueId, ConstantState>, _kill_set: &HashMap<ValueId, ConstantState>) -> HashMap<ValueId, ConstantState> {
        let mut result = in_value.clone();
        
        // genされた定数状態で更新
        for (&var_id, &state) in gen_set {
            result.insert(var_id, state);
        }
        
        result
    }
}

impl ConstantPropagationAnalysis {
    /// 命令を評価し、結果の定数状態を返す
    fn evaluate_instruction(&self, inst: &Instruction, function: &Function) -> ConstantState {
        // 命令タイプに基づいて適切な評価を行う
        match &inst.op {
            // 定数値命令
            InstructionOp::ConstInt(value) => ConstantState::Constant(*value),
            InstructionOp::ConstFloat(_) => ConstantState::NonConstant, // 浮動小数点は非定数として扱う
            InstructionOp::ConstBool(value) => ConstantState::Constant(if *value { 1 } else { 0 }),
            InstructionOp::ConstNull => ConstantState::Constant(0),
            
            // 算術演算命令（オペランドが両方定数の場合は結果も定数）
            InstructionOp::Add | InstructionOp::Sub | InstructionOp::Mul | InstructionOp::Div |
            InstructionOp::Mod | InstructionOp::And | InstructionOp::Or | InstructionOp::Xor |
            InstructionOp::Shl | InstructionOp::Shr => {
                let operands = &inst.operands;
                if operands.len() == 2 {
                    let lhs_state = self.get_operand_state(&operands[0], function);
                    let rhs_state = self.get_operand_state(&operands[1], function);
                    
                    match (lhs_state, rhs_state) {
                        (ConstantState::Constant(lhs), ConstantState::Constant(rhs)) => {
                            // 両方の定数値に対して演算を適用
                            match &inst.op {
                                InstructionOp::Add => ConstantState::Constant(lhs + rhs),
                                InstructionOp::Sub => ConstantState::Constant(lhs - rhs),
                                InstructionOp::Mul => ConstantState::Constant(lhs * rhs),
                                InstructionOp::Div => {
                                    // ゼロ除算のチェック
                                    if rhs == 0 {
                                        ConstantState::NonConstant
                                    } else {
                                        ConstantState::Constant(lhs / rhs)
                                    }
                                },
                                InstructionOp::Mod => {
                                    // ゼロ除算のチェック
                                    if rhs == 0 {
                                        ConstantState::NonConstant
                                    } else {
                                        ConstantState::Constant(lhs % rhs)
                                    }
                                },
                                InstructionOp::And => ConstantState::Constant(lhs & rhs),
                                InstructionOp::Or => ConstantState::Constant(lhs | rhs),
                                InstructionOp::Xor => ConstantState::Constant(lhs ^ rhs),
                                InstructionOp::Shl => ConstantState::Constant(lhs << rhs),
                                InstructionOp::Shr => ConstantState::Constant(lhs >> rhs),
                                _ => ConstantState::NonConstant,
                            }
                        },
                        (ConstantState::Undefined, _) | (_, ConstantState::Undefined) => {
                            // どちらかが未定義の場合、結果も未定義
                            ConstantState::Undefined
                        },
                        _ => {
                            // どちらかが非定数の場合、結果も非定数
                            ConstantState::NonConstant
                        }
                    }
                } else {
                    // 無効なオペランド数
                    ConstantState::NonConstant
                }
            },
            
            // 単項演算命令
            InstructionOp::Neg | InstructionOp::Not => {
                let operands = &inst.operands;
                if operands.len() == 1 {
                    let operand_state = self.get_operand_state(&operands[0], function);
                    
                    match operand_state {
                        ConstantState::Constant(value) => {
                            match &inst.op {
                                InstructionOp::Neg => ConstantState::Constant(-value),
                                InstructionOp::Not => ConstantState::Constant(!value),
                                _ => ConstantState::NonConstant,
                            }
                        },
                        ConstantState::Undefined => ConstantState::Undefined,
                        _ => ConstantState::NonConstant,
                    }
                } else {
                    ConstantState::NonConstant
                }
            },
            
            // 比較演算命令
            InstructionOp::Eq | InstructionOp::Ne | InstructionOp::Lt |
            InstructionOp::Le | InstructionOp::Gt | InstructionOp::Ge => {
                let operands = &inst.operands;
                if operands.len() == 2 {
                    let lhs_state = self.get_operand_state(&operands[0], function);
                    let rhs_state = self.get_operand_state(&operands[1], function);
                    
                    match (lhs_state, rhs_state) {
                        (ConstantState::Constant(lhs), ConstantState::Constant(rhs)) => {
                            // 両方の定数値に対して比較を適用
                            let result = match &inst.op {
                                InstructionOp::Eq => lhs == rhs,
                                InstructionOp::Ne => lhs != rhs,
                                InstructionOp::Lt => lhs < rhs,
                                InstructionOp::Le => lhs <= rhs,
                                InstructionOp::Gt => lhs > rhs,
                                InstructionOp::Ge => lhs >= rhs,
                                _ => return ConstantState::NonConstant,
                            };
                            ConstantState::Constant(if result { 1 } else { 0 })
                        },
                        (ConstantState::Undefined, _) | (_, ConstantState::Undefined) => {
                            ConstantState::Undefined
                        },
                        _ => ConstantState::NonConstant,
                    }
                } else {
                    ConstantState::NonConstant
                }
            },
            
            // 変数参照
            InstructionOp::Load => {
                let operands = &inst.operands;
                if operands.len() == 1 {
                    let var_id = operands[0];
                    self.variable_states.get(&var_id).cloned().unwrap_or(ConstantState::Undefined)
                } else {
        ConstantState::NonConstant
                }
            },
            
            // その他の命令は全て非定数として扱う
            _ => ConstantState::NonConstant,
        }
    }
    
    /// オペランドの定数状態を取得
    fn get_operand_state(&self, operand_id: &ValueId, function: &Function) -> ConstantState {
        // オペランドが定数命令の場合はその値を返す
        if let Some(inst) = function.get_instruction(*operand_id) {
            match &inst.op {
                InstructionOp::ConstInt(value) => return ConstantState::Constant(*value),
                InstructionOp::ConstBool(value) => return ConstantState::Constant(if *value { 1 } else { 0 }),
                InstructionOp::ConstNull => return ConstantState::Constant(0),
                _ => {}
            }
        }
        
        // 変数の場合は現在の状態を返す
        self.variable_states.get(operand_id).cloned().unwrap_or(ConstantState::Undefined)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleend::ir::{BasicBlock, Module};
    
    // データフロー解析のテスト
    
    #[test]
    fn test_dataflow_analysis() {
        // データフロー解析のテスト実装
        
        // モジュールを作成
        let mut module = Module::new("test_module");
        
        // 関数を作成
        let mut function = Function::new("test_function".to_string());
        
        // 基本ブロックを作成
        let mut entry_block = BasicBlock::new(0);
        let mut loop_block = BasicBlock::new(1);
        let mut exit_block = BasicBlock::new(2);
        
        // エントリーブロックの命令
        let x_id = function.create_value("x".to_string());
        let const1 = function.add_instruction(Instruction::new(
            InstructionOp::ConstInt(1),
            vec![],
            "const1".to_string(),
        ));
        entry_block.add_instruction(function.add_instruction(Instruction::new(
            InstructionOp::Store,
            vec![x_id, const1],
            "store_x".to_string(),
        )));
        entry_block.add_instruction(function.add_instruction(Instruction::new(
            InstructionOp::Jump,
            vec![1], // loop_blockにジャンプ
            "jump".to_string(),
        )));
        
        // ループブロックの命令
        let load_x = function.add_instruction(Instruction::new(
            InstructionOp::Load,
            vec![x_id],
            "load_x".to_string(),
        ));
        loop_block.add_instruction(load_x);
        let const10 = function.add_instruction(Instruction::new(
            InstructionOp::ConstInt(10),
            vec![],
            "const10".to_string(),
        ));
        let cmp = function.add_instruction(Instruction::new(
            InstructionOp::Lt,
            vec![load_x, const10],
            "cmp".to_string(),
        ));
        loop_block.add_instruction(cmp);
        let br = function.add_instruction(Instruction::new(
            InstructionOp::Branch,
            vec![cmp, 1, 2], // cmpの値に応じてloop_blockまたはexit_blockにジャンプ
            "br".to_string(),
        ));
        loop_block.add_instruction(br);
        
        // exitブロックの命令
        let ret = function.add_instruction(Instruction::new(
            InstructionOp::Return,
            vec![const1], // 1を返す
            "ret".to_string(),
        ));
        exit_block.add_instruction(ret);
        
        // 基本ブロックを関数に追加
        function.add_block(entry_block);
        function.add_block(loop_block);
        function.add_block(exit_block);
        
        // モジュールに関数を追加
        module.add_function(function);
        
        // 関数を取得
        let function = module.get_function("test_function").unwrap();
        
        // 1. 到達定義解析のテスト
        {
            println!("テスト: 到達定義解析");
            let analysis = ReachingDefinitionAnalysis::new(function);
            let engine = DataFlowEngine::new();
            let result = engine.analyze(function, &analysis);
            
            // 各ブロックの到達定義を出力
            for (block_id, in_set) in &result.in_sets {
                println!("ブロック {} の入口での到達定義: {:?}", block_id, in_set);
            }
            
            // 検証: エントリーブロックの出口ではx_idの定義が到達している
            let entry_block_out = result.out_sets.get(&0).unwrap();
            let store_x_id = function.get_block(0).unwrap().instructions[0];
            assert!(entry_block_out.contains(&store_x_id));
            
            // 検証: ループブロックの入口でもx_idの定義が到達している
            let loop_block_in = result.in_sets.get(&1).unwrap();
            assert!(loop_block_in.contains(&store_x_id));
        }
        
        // 2. 活性変数解析のテスト
        {
            println!("テスト: 活性変数解析");
            let analysis = LiveVariableAnalysis::new();
            let engine = DataFlowEngine::new();
            let result = engine.analyze(function, &analysis);
            
            // 各ブロックの活性変数を出力
            for (block_id, out_set) in &result.out_sets {
                println!("ブロック {} の出口での活性変数: {:?}", block_id, out_set);
            }
            
            // 検証: ループブロックの入口ではx_idが活性
            let loop_block_in = result.in_sets.get(&1).unwrap();
            assert!(loop_block_in.contains(&x_id));
            
            // 検証: エントリーブロックの出口ではx_idが活性
            let entry_block_out = result.out_sets.get(&0).unwrap();
            assert!(entry_block_out.contains(&x_id));
        }
        
        // 3. 定数伝播解析のテスト
        {
            println!("テスト: 定数伝播解析");
            let analysis = ConstantPropagationAnalysis::new();
            let engine = DataFlowEngine::new();
            let result = engine.analyze(function, &analysis);
            
            // 各ブロックの定数状態を出力
            for (block_id, in_set) in &result.in_sets {
                println!("ブロック {} の入口での定数状態:", block_id);
                for (var_id, state) in in_set {
                    println!("  変数 {:?}: {:?}", var_id, state);
                }
            }
            
            // 検証: エントリーブロックの出口ではx_idが定数1
            let entry_block_out = result.out_sets.get(&0).unwrap();
            if let Some(state) = entry_block_out.get(&x_id) {
                match state {
                    ConstantState::Constant(value) => assert_eq!(*value, 1),
                    _ => panic!("x_idは定数1であるべき"),
                }
            } else {
                panic!("x_idの状態が見つからない");
            }
        }
    }
}

//! # ARM64 コード生成
//! 
//! ARM64アーキテクチャ向けのネイティブコードを生成するモジュールです。
//! 主にLLVMバックエンドが生成したオブジェクトコードに対して、さらなる最適化を行います。

use std::collections::HashMap;
use std::collections::HashSet;

use crate::frontend::ast::{BinaryOperator, UnaryOperator};
use crate::middleend::ir::representation::{Module, Instruction, Type, BasicBlock, Function};
use crate::frontend::error::{Result, CompilerError};

// 型エイリアスの定義
#[derive(Debug, Clone, PartialEq)]
enum ValueId {
    Variable(usize),
    Constant(i64),
}

#[derive(Debug, Clone, PartialEq)]
enum FunctionId {
    Function(usize),
    External(String),
}

// 命令の種類を表す列挙型
#[derive(Debug, Clone, PartialEq)]
enum InstructionKind {
    // バイナリ操作
    BinaryOp {
        op: BinaryOperator,
        lhs: ValueId,
        rhs: ValueId,
    },
    // 単項操作
    UnaryOp {
        op: UnaryOperator,
        operand: ValueId,
    },
    // メモリロード
    Load {
        address: ValueId,
        ty: Type,
    },
    // メモリストア
    Store {
        address: ValueId,
        value: ValueId,
        ty: Type,
    },
    // 関数呼び出し
    Call {
        func: FunctionId,
        args: Vec<ValueId>,
    },
    // 関数からのリターン
    Return {
        value: Option<ValueId>,
    },
    // 条件分岐
    Branch {
        condition: ValueId,
        then_block: usize,
        else_block: usize,
    },
    // 無条件ジャンプ
    Jump {
        target_block: usize,
    },
    // その他の命令
    Other,
}

/// ARM64向け最適化器
pub struct ARM64Optimizer {
    /// レジスタ割り当て
    register_allocation: HashMap<usize, String>,
    /// 命令選択情報
    instruction_selection: HashMap<usize, Vec<String>>,
    /// 浮動小数点変数のセット
    float_vars: HashSet<usize>,
    /// 関数名マッピング
    functions: HashMap<usize, String>,
}

impl ARM64Optimizer {
    /// 新しい最適化器を作成
    pub fn new() -> Self {
        Self {
            register_allocation: HashMap::new(),
            instruction_selection: HashMap::new(),
            float_vars: HashSet::new(),
            functions: HashMap::new(),
        }
    }
    
    /// オブジェクトコードを最適化
    pub fn optimize(&mut self, obj_code: &[u8]) -> Result<Vec<u8>> {
        // 現時点ではオブジェクトコードの最適化は行わずそのまま返す
        // 将来的には以下のような最適化を行う：
        // - ARM64固有の命令（Neon, SVE）を活用
        // - レジスタ割り当ての最適化
        // - 分岐予測に適した命令配置
        
        Ok(obj_code.to_vec())
    }
    
    /// 関数に対してARM64固有の最適化を適用
    pub fn optimize_function(&mut self, function: &Function) -> Result<()> {
        // レジスタ割り当て
        self.allocate_registers(function)?;
        
        // 命令選択
        self.select_instructions(function)?;
        
        // 命令スケジューリング
        self.schedule_instructions(function)?;
        
        Ok(())
    }
    
    /// レジスタ割り当て
    fn allocate_registers(&mut self, function: &Function) -> Result<()> {
        // グラフ彩色によるレジスタ割り当て実装
        
        // 利用可能なレジスタ一覧
        let general_registers = vec!["x0", "x1", "x2", "x3", "x4", "x5", "x6", "x7", "x8", "x9", "x10", "x11", "x12", "x13", "x14", "x15"];
        let float_registers = vec!["v0", "v1", "v2", "v3", "v4", "v5", "v6", "v7"];
        
        // 汎用レジスタの予約（呼び出し規約で予約されるレジスタ）
        let reserved_registers = HashSet::from(["x18", "x29", "x30", "sp"]);
        
        // 変数のライブ区間解析
        let liveness = self.analyze_liveness(function)?;
        
        // 干渉グラフの構築
        let interference_graph = self.build_interference_graph(function, &liveness)?;
        
        // グラフ彩色によるレジスタ割り当て
        let register_assignment = self.color_graph(
            &interference_graph, 
            &general_registers, 
            &float_registers, 
            &reserved_registers
        )?;
        
        // レジスタ割り当て結果を保存
        self.register_allocation = register_assignment;
        
        Ok(())
    }
    
    /// ライブ変数解析を実行
    fn analyze_liveness(&self, function: &Function) -> Result<HashMap<usize, HashSet<usize>>> {
        // 各ブロックの入口と出口におけるライブ変数集合
        let mut live_in = HashMap::new();
        let mut live_out = HashMap::new();
        
        // 各ブロックの生成(DEF)・使用(USE)変数集合
        let mut def_sets = HashMap::new();
        let mut use_sets = HashMap::new();
        
        // 1. 各ブロックのDEF集合とUSE集合を計算
        for (block_id, block) in &function.basic_blocks {
            let mut def_set = HashSet::new();
            let mut use_set = HashSet::new();
            
            for inst in &block.instructions {
                // 定義される変数（出力）
                if let Some(output) = inst.output {
                    def_set.insert(output);
                }
                
                // 使用される変数（入力）
                for input in self.get_instruction_inputs(inst) {
                    // まだ定義されていなければUSE集合に追加
                    if !def_set.contains(&input) {
                        use_set.insert(input);
                    }
                }
            }
            
            def_sets.insert(*block_id, def_set);
            use_sets.insert(*block_id, use_set);
            
            // 初期状態でのlive_in, live_outを空集合で初期化
            live_in.insert(*block_id, HashSet::new());
            live_out.insert(*block_id, HashSet::new());
        }
        
        // 2. 不動点に達するまでライブ変数解析を実行
        let mut changed = true;
        while changed {
            changed = false;
            
            // 各ブロックを逆順に処理
            for block_id in function.basic_blocks.keys().rev() {
                let block = &function.basic_blocks[block_id];
                let successors = self.get_successors(block);
                
                // OUT[B] = ∪ IN[S] for all successors S of B
                let mut new_live_out = HashSet::new();
                for succ in &successors {
                    if let Some(succ_live_in) = live_in.get(succ) {
                        new_live_out.extend(succ_live_in);
                    }
                }
                
                // OUT[B]に変化があるか確認
                let old_live_out = live_out.get(block_id).unwrap();
                if &new_live_out != old_live_out {
                    live_out.insert(*block_id, new_live_out.clone());
                    changed = true;
                }
                
                // IN[B] = USE[B] ∪ (OUT[B] - DEF[B])
                let mut new_live_in = use_sets.get(block_id).unwrap().clone();
                let def_set = def_sets.get(block_id).unwrap();
                
                // OUT[B] - DEF[B]の計算
                for var in &new_live_out {
                    if !def_set.contains(var) {
                        new_live_in.insert(*var);
                    }
                }
                
                // IN[B]に変化があるか確認
                let old_live_in = live_in.get(block_id).unwrap();
                if &new_live_in != old_live_in {
                    live_in.insert(*block_id, new_live_in);
                    changed = true;
                }
            }
        }
        
        Ok(live_out)
    }
    
    /// 干渉グラフの構築
    fn build_interference_graph(&self, function: &Function, liveness: &HashMap<usize, HashSet<usize>>) -> Result<HashMap<usize, HashSet<usize>>> {
        let mut interference_graph = HashMap::new();
        
        // すべての変数ノードを作成
        for (_, block) in &function.basic_blocks {
            for inst in &block.instructions {
                // 出力変数のノードを追加
                if let Some(output) = inst.output {
                    interference_graph.entry(output).or_insert_with(HashSet::new);
                }
                
                // 入力変数のノードを追加
                for input in self.get_instruction_inputs(inst) {
                    interference_graph.entry(input).or_insert_with(HashSet::new);
                }
            }
        }
        
        // 各ブロックに対して干渉関係を構築
        for (block_id, block) in &function.basic_blocks {
            // ブロック終了時点でのライブ変数集合
            if let Some(live_vars) = liveness.get(block_id) {
                let mut current_live = live_vars.clone();
                
                // ブロック内の命令を逆順に走査
                for inst in block.instructions.iter().rev() {
                    // 出力変数と現在のライブ変数の間に干渉関係を追加
                    if let Some(output) = inst.output {
                        for live_var in &current_live {
                            if *live_var != output {
                                // output <-> live_var の干渉関係を追加
                                interference_graph.entry(output).and_modify(|edges| {
                                    edges.insert(*live_var);
                                });
                                interference_graph.entry(*live_var).and_modify(|edges| {
                                    edges.insert(output);
                                });
                            }
                        }
                        
                        // 出力変数はこの命令の前ではライブでない
                        current_live.remove(&output);
                    }
                    
                    // 入力変数をライブ集合に追加
                    for input in self.get_instruction_inputs(inst) {
                        current_live.insert(input);
                    }
                }
            }
        }
        
        Ok(interference_graph)
    }
    
    /// グラフ彩色アルゴリズムによるレジスタ割り当て
    fn color_graph(
        &self, 
        interference_graph: &HashMap<usize, HashSet<usize>>,
        general_registers: &[&str],
        float_registers: &[&str],
        reserved_registers: &HashSet<&str>
    ) -> Result<HashMap<usize, String>> {
        let mut result = HashMap::new();
        
        // 1. 変数の次数（干渉数）に基づいてソート
        let mut nodes: Vec<usize> = interference_graph.keys().cloned().collect();
        nodes.sort_by_key(|node| interference_graph.get(node).map_or(0, |edges| edges.len()));
        nodes.reverse(); // 次数が高い順
        
        // 2. スタックに変数をプッシュ
        let mut stack = Vec::new();
        let mut removed_nodes = HashSet::new();
        
        while !nodes.is_empty() {
            // 次数が最も低いノードを探す（現在の有効なグラフ中で）
            let mut min_degree_idx = 0;
            let mut min_degree = usize::MAX;
            
            for (i, node) in nodes.iter().enumerate() {
                if !removed_nodes.contains(node) {
                    let degree = interference_graph.get(node)
                        .map_or(0, |edges| edges.iter().filter(|e| !removed_nodes.contains(e)).count());
                    
                    if degree < min_degree {
                        min_degree = degree;
                        min_degree_idx = i;
                    }
                }
            }
            
            // 次数が最も低いノードをスタックにプッシュして「削除」
            let node = nodes.remove(min_degree_idx);
            stack.push(node);
            removed_nodes.insert(node);
            
            // すべてのノードが削除されたら終了
            if removed_nodes.len() == interference_graph.len() {
                break;
            }
        }
        
        // 3. スタックから変数をポップして彩色
        while let Some(node) = stack.pop() {
            // 隣接ノードが使用中の色を確認
            let mut used_registers = HashSet::new();
            let mut neighbor_weights = HashMap::new();
            
            if let Some(neighbors) = interference_graph.get(&node) {
                for neighbor in neighbors {
                    if let Some(reg) = result.get(neighbor) {
                        used_registers.insert(reg.clone());
                        
                        // 隣接ノードの重みを計算（レジスタ割り当て優先度の計算に使用）
                        let weight = self.calculate_node_weight(*neighbor);
                        neighbor_weights.insert(reg.clone(), weight);
                    }
                }
            }
            
            // 変数の特性を分析
            let var_properties = self.analyze_variable_properties(node);
            let is_float = self.is_float_var(node);
            let is_vector = self.is_vector_var(node);
            let is_hot_var = self.is_hot_variable(node);
            let var_lifetime = self.calculate_variable_lifetime(node);
            let var_access_pattern = self.analyze_access_pattern(node);
            
            // レジスタプールの選択とカスタマイズ
            let mut register_pool = if is_vector {
                // ベクトル変数にはSIMDレジスタを割り当て
                self.get_vector_registers()
            } else if is_float {
                float_registers.to_vec()
            } else {
                general_registers.to_vec()
            };
            
            // 変数の使用パターンに基づいてレジスタの優先順位を調整
            self.optimize_register_order(&mut register_pool, &var_properties, is_hot_var, var_access_pattern);
            
            // 呼び出し規約に基づく制約を適用
            let calling_convention_constraints = self.apply_calling_convention_constraints(node);
            
            // ハードウェア特性に基づく最適化
            let hw_constraints = self.apply_hardware_specific_constraints(node, is_float, is_vector);
            
            // 使用されていないレジスタを探す（最適化された優先順位に基づく）
            let mut best_register = None;
            let mut best_score = f64::NEG_INFINITY;
            
            for reg in register_pool.iter() {
                if !used_registers.contains(*reg) && !reserved_registers.contains(*reg) &&
                   calling_convention_constraints.allows_register(*reg) &&
                   hw_constraints.allows_register(*reg) {
                    
                    // レジスタ割り当てスコアを計算
                    let score = self.calculate_register_assignment_score(
                        *reg, 
                        node, 
                        &var_properties, 
                        var_lifetime,
                        is_hot_var,
                        &neighbor_weights
                    );
                    
                    if score > best_score {
                        best_score = score;
                        best_register = Some(*reg);
                    }
                }
            }
            
            // コンテキスト認識型スピル決定
            if best_register.is_none() {
                // スピル候補を評価
                let spill_decision = self.make_intelligent_spill_decision(
                    node, 
                    &result, 
                    interference_graph,
                    is_hot_var,
                    var_lifetime,
                    var_access_pattern
                );
                
                match spill_decision {
                    SpillDecision::SpillCurrent => {
                        // 現在の変数をスピル
                        let spill_location = self.allocate_optimal_spill_location(node, is_float, is_vector);
                        result.insert(node, format!("spill_{}", spill_location));
                        self.register_spill_code(node, spill_location);
                    },
                    SpillDecision::SpillOther(other_node) => {
                        // 他の変数をスピルして、そのレジスタを再利用
                        if let Some(other_reg) = result.get(&other_node).cloned() {
                            if !other_reg.starts_with("spill_") {
                                let spill_location = self.allocate_optimal_spill_location(other_node, 
                                                                                         self.is_float_var(other_node), 
                                                                                         self.is_vector_var(other_node));
                                result.insert(other_node, format!("spill_{}", spill_location));
                                self.register_spill_code(other_node, spill_location);
                                result.insert(node, other_reg);
                            } else {
                                // 既にスピルされている場合は現在の変数もスピル
                                let spill_location = self.allocate_optimal_spill_location(node, is_float, is_vector);
                                result.insert(node, format!("spill_{}", spill_location));
                                self.register_spill_code(node, spill_location);
                            }
                        }
                    },
                    SpillDecision::Rematerialize(remat_node) => {
                        // 再具体化が可能な変数を処理
                        if let Some(remat_reg) = result.get(&remat_node).cloned() {
                            if !remat_reg.starts_with("spill_") {
                                self.register_rematerialization_code(remat_node);
                                result.insert(node, remat_reg);
                                result.insert(remat_node, format!("remat_{}", remat_node));
                            } else {
                                // 既にスピルされている場合は現在の変数もスピル
                                let spill_location = self.allocate_optimal_spill_location(node, is_float, is_vector);
                                result.insert(node, format!("spill_{}", spill_location));
                                self.register_spill_code(node, spill_location);
                            }
                        }
                    },
                    SpillDecision::SplitLiveRange(split_node, split_points) => {
                        // ライブ範囲分割を適用
                        self.register_live_range_split(split_node, split_points);
                        // 分割後に現在のノードにレジスタを割り当て
                        if let Some(freed_reg) = self.get_register_after_split(split_node) {
                            result.insert(node, freed_reg);
                        } else {
                            // 分割が成功しなかった場合はスピル
                            let spill_location = self.allocate_optimal_spill_location(node, is_float, is_vector);
                            result.insert(node, format!("spill_{}", spill_location));
                            self.register_spill_code(node, spill_location);
                        }
                    }
                }
            } else {
                // 最適なレジスタを割り当て
                let reg = best_register.unwrap();
                result.insert(node, reg.to_string());
                
                // レジスタ割り当て後の最適化機会を記録
                self.register_post_allocation_optimization_opportunities(node, reg);
            }
            
            // レジスタ割り当て決定を記録（後の分析用）
            self.record_allocation_decision(node, result.get(&node).unwrap().clone(), is_hot_var, var_lifetime);
        }
        
        // 最終的な割り当て結果に対して後処理最適化を適用
        self.apply_post_allocation_optimizations(&mut result, interference_graph);
        
        Ok(result)
    }
    
    /// 命令から入力変数のリストを取得
    fn get_instruction_inputs(&self, inst: &Instruction) -> Vec<usize> {
        let mut inputs = Vec::new();
        
        match &inst.kind {
            InstructionKind::BinaryOp { lhs, rhs, .. } => {
                if let ValueId::Variable(id) = lhs {
                    inputs.push(*id);
                }
                if let ValueId::Variable(id) = rhs {
                    inputs.push(*id);
                }
            },
            InstructionKind::UnaryOp { operand, .. } => {
                if let ValueId::Variable(id) = operand {
                    inputs.push(*id);
                }
            },
            InstructionKind::Load { address, .. } => {
                if let ValueId::Variable(id) = address {
                    inputs.push(*id);
                }
            },
            InstructionKind::Store { address, value, .. } => {
                if let ValueId::Variable(id) = address {
                    inputs.push(*id);
                }
                if let ValueId::Variable(id) = value {
                    inputs.push(*id);
                }
            },
            InstructionKind::Call { args, .. } => {
                for arg in args {
                    if let ValueId::Variable(id) = arg {
                        inputs.push(*id);
                    }
                }
            },
            InstructionKind::Return { value, .. } => {
                if let Some(val) = value {
                    if let ValueId::Variable(id) = val {
                        inputs.push(*id);
                    }
                }
            },
            InstructionKind::Branch { condition, .. } => {
                if let ValueId::Variable(id) = condition {
                    inputs.push(*id);
                }
            },
            // 他の命令種別も必要に応じて追加
            _ => {}
        }
        
        inputs
    }
    
    /// ブロックの後続ブロックを取得
    fn get_successors(&self, block: &BasicBlock) -> Vec<usize> {
        let mut successors = Vec::new();
        
        // 最後の命令を確認
        if let Some(last_inst) = block.instructions.last() {
            match &last_inst.kind {
                InstructionKind::Branch { then_block, else_block, .. } => {
                    successors.push(*then_block);
                    successors.push(*else_block);
                },
                InstructionKind::Jump { target_block } => {
                    successors.push(*target_block);
                },
                // 関数からのreturnの場合は後続ブロックなし
                InstructionKind::Return { .. } => {},
                // 他の終了命令も必要に応じて追加
                _ => {
                    // フォールスルーがあれば後続ブロック
                    // 注: BasicBlockにfallthroughフィールドがない場合は、
                    // 別の方法でフォールスルーを判断する必要があります
                    // この実装では簡易化のためフォールスルーは考慮しません
                }
            }
        }
        
        successors
    }
    
    /// 変数が浮動小数点型かどうかを判定
    fn is_float_var(&self, var_id: usize) -> bool {
        // 変数の型情報に基づいて判定
        // （実装例：変数IDから型を取得して判定）
        self.float_vars.contains(&var_id)
    }
    
    /// 命令の変数使用を解析
    fn analyze_instruction_variables(&self, inst: &Instruction, var_usage: &mut HashMap<usize, usize>) {
        // 命令の入力変数と出力変数を解析して使用頻度をカウント
        
        // 入力変数の解析
        for input in self.get_instruction_inputs(inst) {
            *var_usage.entry(input).or_insert(0) += 1;
        }
        
        // 出力変数の解析
        if let Some(output) = inst.output {
            *var_usage.entry(output).or_insert(0) += 1;
        }
    }
    
    /// ARM64命令の選択
    fn select_instructions(&mut self, function: &Function) -> Result<()> {
        // IR命令からARM64命令への変換
        let mut arm64_instructions = HashMap::new();
        
        for (block_id, block) in &function.basic_blocks {
            let mut block_instructions = Vec::new();
            
            for inst in &block.instructions {
                let arm64_insts = self.translate_instruction(inst)?;
                block_instructions.extend(arm64_insts);
            }
            
            arm64_instructions.insert(*block_id, block_instructions);
        }
        
        self.instruction_selection = arm64_instructions;
        Ok(())
    }
    
    /// 単一のIR命令をARM64命令に変換
    fn translate_instruction(&self, inst: &Instruction) -> Result<Vec<String>> {
        let mut result = Vec::new();
        
        match &inst.kind {
            InstructionKind::BinaryOp { op, lhs, rhs, .. } => {
                let lhs_reg = self.get_operand_register(lhs);
                let rhs_reg = self.get_operand_register(rhs);
                let dst_reg = if let Some(op_val) = &inst.output { self.get_operand_register(op_val) } else { self.get_register_for_var(0) };
                
                match op {
                    BinaryOperator::Add => {
                        result.push(format!("add {}, {}, {}", dst_reg, lhs_reg, rhs_reg));
                    },
                    BinaryOperator::Sub => {
                        result.push(format!("sub {}, {}, {}", dst_reg, lhs_reg, rhs_reg));
                    },
                    BinaryOperator::Mul => {
                        result.push(format!("mul {}, {}, {}", dst_reg, lhs_reg, rhs_reg));
                    },
                    BinaryOperator::Div => {
                        result.push(format!("sdiv {}, {}, {}", dst_reg, lhs_reg, rhs_reg));
                    },
                    // 他の二項演算子も同様に実装
                    _ => {
                        // より複雑な演算子は複数命令に分解
                        result.push(format!("// Complex operation: {:?}", op));
                        result.push(format!("// Simplified implementation"));
                    }
                }
            },
            InstructionKind::UnaryOp { op, operand, .. } => {
                let op_reg = self.get_operand_register(operand);
                let dst_reg = if let Some(op_val) = &inst.output { self.get_operand_register(op_val) } else { self.get_register_for_var(0) };
                
                match op {
                    UnaryOperator::Neg => {
                        result.push(format!("neg {}, {}", dst_reg, op_reg));
                    },
                    UnaryOperator::Not => {
                        result.push(format!("mvn {}, {}", dst_reg, op_reg));
                    },
                    // 他の単項演算子も同様に実装
                    _ => {
                        result.push(format!("// Complex unary operation: {:?}", op));
                    }
                }
            },
            InstructionKind::Load { address, ty, .. } => {
                let addr_reg = self.get_operand_register(address);
                let dst_reg = if let Some(op_val) = &inst.output { self.get_operand_register(op_val) } else { self.get_register_for_var(0) };
                
                // 型サイズに応じたロード命令を選択
                let size = self.get_type_size(ty);
                match size {
                    1 => result.push(format!("ldrb {}, [{}]", dst_reg, addr_reg)),
                    2 => result.push(format!("ldrh {}, [{}]", dst_reg, addr_reg)),
                    4 => result.push(format!("ldr {}, [{}]", dst_reg, addr_reg)),
                    8 => result.push(format!("ldr {}, [{}]", dst_reg, addr_reg)),
                    _ => {
                        // 複合型はメモリコピー
                        result.push(format!("// Complex load for size: {}", size));
                    }
                }
            },
            InstructionKind::Store { address, value, ty, .. } => {
                let addr_reg = self.get_operand_register(address);
                let val_reg = self.get_operand_register(value);
                
                // 型サイズに応じたストア命令を選択
                let size = self.get_type_size(ty);
                match size {
                    1 => result.push(format!("strb {}, [{}]", val_reg, addr_reg)),
                    2 => result.push(format!("strh {}, [{}]", val_reg, addr_reg)),
                    4 => result.push(format!("str {}, [{}]", val_reg, addr_reg)),
                    8 => result.push(format!("str {}, [{}]", val_reg, addr_reg)),
                    _ => {
                        // 複合型はメモリコピー
                        result.push(format!("// Complex store for size: {}", size));
                    }
                }
            },
            InstructionKind::Call { func, args, .. } => {
                // 引数をレジスタとスタックに配置
                for (i, arg) in args.iter().enumerate() {
                    let arg_reg = self.get_operand_register(arg);
                    if i < 8 {
                        // 最初の8つの引数はx0-x7レジスタ
                        result.push(format!("mov x{}, {}", i, arg_reg));
                    } else {
                        // それ以降はスタックに配置
                        result.push(format!("str {}, [sp, #{}]", arg_reg, (i - 8) * 8));
                    }
                }
                
                // 関数呼び出し
                let func_name = self.get_function_name(func);
                result.push(format!("bl {}", func_name));
                
                // 戻り値がある場合、x0から結果レジスタにコピー
                if let Some(output) = &inst.output {
                    let dst_reg = self.get_operand_register(output);
                    if dst_reg != "x0" {
                        result.push(format!("mov {}, x0", dst_reg));
                    }
                }
            },
            InstructionKind::Return { value, .. } => {
                if let Some(val) = value {
                    let val_reg = self.get_operand_register(val);
                    if val_reg != "x0" {
                        result.push(format!("mov x0, {}", val_reg));
                    }
                }
                
                // 関数エピローグと戻り
                result.push("ldp x29, x30, [sp], #16".to_string());
                result.push("ret".to_string());
            },
            InstructionKind::Branch { condition, then_block, else_block, .. } => {
                let cond_reg = self.get_operand_register(condition);
                let then_label = self.get_block_label(*then_block);
                let else_label = self.get_block_label(*else_block);
                
                // 条件レジスタが0でないかをテスト
                result.push(format!("cmp {}, #0", cond_reg));
                result.push(format!("b.ne {}", then_label));
                result.push(format!("b {}", else_label));
            },
            InstructionKind::Jump { target_block } => {
                let target_label = self.get_block_label(*target_block);
                result.push(format!("b {}", target_label));
            },
            // 他の命令も必要に応じて追加
            _ => {
                result.push(format!("// Unsupported instruction: {:?}", inst.kind));
            }
        }
        
        Ok(result)
    }
    
    /// オペランドのレジスタ名を取得
    fn get_operand_register(&self, operand: &ValueId) -> String {
        match operand {
            ValueId::Variable(id) => self.get_register_for_var(*id),
            ValueId::Constant(val) => {
                // 定数はimm形式で返す（ARM64では一部の命令のみ対応）
                format!("#{}", val)
            },
            _ => "#0".to_string() // デフォルト値
        }
    }
    
    /// 変数IDに割り当てられたレジスタを取得
    fn get_register_for_var(&self, var_id: usize) -> String {
        self.register_allocation.get(&var_id)
            .cloned()
            .unwrap_or_else(|| {
                if self.is_float_var(var_id) {
                    "v0".to_string() // デフォルトの浮動小数点レジスタ
                } else {
                    "x0".to_string() // デフォルトの汎用レジスタ
                }
            })
    }
    
    /// 関数名を取得
    fn get_function_name(&self, func: &FunctionId) -> String {
        match func {
            FunctionId::Function(id) => {
                self.functions.get(id)
                    .cloned()
                    .unwrap_or_else(|| format!("func_{}", id))
            },
            FunctionId::External(name) => name.clone(),
        }
    }
    
    /// ブロックのラベル名を取得
    fn get_block_label(&self, block_id: usize) -> String {
        format!(".L{}", block_id)
    }
    
    /// 型のサイズを取得（バイト単位）
    fn get_type_size(&self, ty: &Type) -> usize {
        match ty {
            Type::Int8 | Type::UInt8 => 1,
            Type::Int16 | Type::UInt16 => 2,
            Type::Int32 | Type::UInt32 | Type::Float32 => 4,
            Type::Int64 | Type::UInt64 | Type::Float64 => 8,
            Type::Bool => 1,
            Type::Pointer(_) => 8,
            Type::Array(elem_ty, size) => self.get_type_size(elem_ty) * size,
            Type::Struct(name, fields) => {
                let mut total = 0;
                for field in fields {
                    total += self.get_type_size(&field.ty);
                    // ARM64のアライメント要件に合わせる
                    let align = self.get_type_alignment(&field.ty);
                    total = (total + align - 1) & !(align - 1);
                }
                total
            },
            // 他の型も必要に応じて追加
            _ => 8 // デフォルトサイズ
        }
    }
    
    /// 型のアライメント要件を取得（バイト単位）
    fn get_type_alignment(&self, ty: &Type) -> usize {
        match ty {
            Type::Int8 | Type::UInt8 | Type::Bool => 1,
            Type::Int16 | Type::UInt16 => 2,
            Type::Int32 | Type::UInt32 | Type::Float32 => 4,
            Type::Int64 | Type::UInt64 | Type::Float64 | Type::Pointer(_) => 8,
            Type::Array(elem_ty, _) => self.get_type_alignment(elem_ty),
            Type::Struct(name, fields) => {
                // 構造体のアライメントは最大のフィールドアライメント
                let mut max_align = 1;
                for field in fields {
                    max_align = max_align.max(self.get_type_alignment(&field.ty));
                }
                max_align
            },
            // 他の型も必要に応じて追加
            _ => 8 // デフォルトアライメント
        }
    }
    
    /// 命令スケジューリング
    fn schedule_instructions(&mut self, function: &Function) -> Result<()> {
        // パイプライン最適化のための命令スケジューリング
        for (block_id, instructions) in &mut self.instruction_selection {
            // 依存関係の分析
            let dependencies = self.analyze_dependencies(instructions);
            
            // 命令遅延の分析
            let latencies = self.analyze_instruction_latencies(instructions);
            
            // ARM64パイプラインに基づく命令のスケジューリング
            let scheduled = self.list_scheduling(instructions, &dependencies, &latencies);
            
            // スケジュールされた命令で更新
            *instructions = scheduled;
        }
        
        Ok(())
    }
    
    /// 命令間の依存関係を分析
    fn analyze_dependencies(&self, instructions: &[String]) -> Vec<Vec<usize>> {
        let mut result = vec![Vec::new(); instructions.len()];
        
        // レジスタの定義と使用を追跡
        let mut reg_last_def = HashMap::new();
        
        for (i, inst) in instructions.iter().enumerate() {
            // 命令から定義レジスタと使用レジスタを抽出
            let (def_regs, use_regs) = self.extract_registers(inst);
            
            // 使用レジスタの依存関係を追加
            for reg in &use_regs {
                if let Some(&def_idx) = reg_last_def.get(reg) {
                    result[i].push(def_idx);
                }
            }
            
            // 定義レジスタを更新
            for reg in def_regs {
                reg_last_def.insert(reg, i);
            }
        }
        
        result
    }
    
    /// 命令からレジスタを抽出（定義される/使用されるレジスタのペア）
    fn extract_registers(&self, instruction: &str) -> (Vec<String>, Vec<String>) {
        let mut def_regs = Vec::new();
        let mut use_regs = Vec::new();
        
        // 命令をスペースで分割
        let parts: Vec<&str> = instruction.split_whitespace().collect();
        if parts.is_empty() {
            return (def_regs, use_regs);
        }
        
        match parts[0] {
            "add" | "sub" | "mul" | "div" | "and" | "orr" | "eor" | "lsl" | "lsr" | "asr" | "ror" => {
                if parts.len() >= 4 {
                    // 例: add x0, x1, x2
                    def_regs.push(parts[1].trim_end_matches(',').to_string());
                    use_regs.push(parts[2].trim_end_matches(',').to_string());
                    use_regs.push(parts[3].to_string());
                }
            },
            "neg" | "mvn" | "mov" => {
                if parts.len() >= 3 {
                    // 例: neg x0, x1
                    def_regs.push(parts[1].trim_end_matches(',').to_string());
                    use_regs.push(parts[2].to_string());
                }
            },
            "ldr" | "ldrb" | "ldrh" => {
                if parts.len() >= 3 {
                    // 例: ldr x0, [x1]
                    def_regs.push(parts[1].trim_end_matches(',').to_string());
                    // アドレスレジスタを抽出（[x1] から x1 を取得）
                    let addr_part = parts[2].trim_start_matches('[').trim_end_matches(']');
                    use_regs.push(addr_part.to_string());
                }
            },
            "str" | "strb" | "strh" => {
                if parts.len() >= 3 {
                    // 例: str x0, [x1]
                    use_regs.push(parts[1].trim_end_matches(',').to_string());
                    let addr_part = parts[2].trim_start_matches('[').trim_end_matches(']');
                    use_regs.push(addr_part.to_string());
                }
            },
            "cmp" => {
                if parts.len() >= 3 {
                    // 例: cmp x0, #0
                    use_regs.push(parts[1].trim_end_matches(',').to_string());
                    // parts[2]は即値の可能性があるため、レジスタの場合のみ追加
                    if !parts[2].starts_with('#') {
                        use_regs.push(parts[2].to_string());
                    }
                }
            },
            "tst" => {
                if parts.len() >= 3 {
                    // 例: tst x0, x1
                    use_regs.push(parts[1].trim_end_matches(',').to_string());
                    if !parts[2].starts_with('#') {
                        use_regs.push(parts[2].to_string());
                    }
                }
            },
            "b" | "beq" | "bne" | "blt" | "ble" | "bgt" | "bge" => {
                // 分岐命令はレジスタを定義/使用しない（フラグレジスタは暗黙的に使用）
                // ただし、条件付き分岐はCPSRフラグを使用するため、将来的にはこれを追加する可能性あり
            },
            "bl" | "blr" => {
                // 関数呼び出しは特別扱い（引数レジスタと戻り値レジスタに依存）
                // x0-x7は引数レジスタとして使用
                for i in 0..8 {
                    use_regs.push(format!("x{}", i));
                }
                // x0-x1は戻り値レジスタとして定義
                def_regs.push("x0".to_string());
                def_regs.push("x1".to_string());
                // リンクレジスタx30(lr)も定義される
                def_regs.push("x30".to_string());
                // 呼び出し規約に従い、揮発性レジスタも定義される
                for i in 9..16 {
                    def_regs.push(format!("x{}", i));
                }
            },
            // ... 他の命令パターンは省略 ...
            _ => {
                // 未知の命令、何もしない
            }
        }
        
        (def_regs, use_regs)
    }
    
    /// 命令の実行遅延を分析
    fn analyze_instruction_latencies(&self, instructions: &[String]) -> Vec<usize> {
        let mut latencies = Vec::with_capacity(instructions.len());
        
        for inst in instructions {
            let parts: Vec<&str> = inst.split_whitespace().collect();
            if parts.is_empty() {
                latencies.push(1);
                continue;
            }
            
            // 命令オペコードに基づく遅延の設定
            match parts[0] {
                "add" | "sub" | "and" | "orr" | "eor" | "mov" => latencies.push(1), // 論理・算術演算は通常1サイクル
                "mul" => latencies.push(3), // 乗算は3-4サイクル
                "sdiv" => latencies.push(12), // 除算は多くのサイクルを要する
                "ldr" | "ldrb" | "ldrh" => latencies.push(4), // メモリロードは通常4サイクル程度
                "str" | "strb" | "strh" => latencies.push(1), // メモリストアは通常1サイクル
                "bl" => latencies.push(4), // 関数呼び出しは最低4サイクル
                _ => latencies.push(1) // デフォルトは1サイクル
            }
        }
        
        latencies
    }
    
    /// リストスケジューリングアルゴリズムによる命令のスケジューリング
    fn list_scheduling(
        &self,
        instructions: &[String],
        dependencies: &[Vec<usize>],
        latencies: &[usize]
    ) -> Vec<String> {
        if instructions.is_empty() {
            return Vec::new();
        }
        
        let mut result = Vec::with_capacity(instructions.len());
        
        // 命令の依存カウント
        let mut dep_counts = vec![0; instructions.len()];
        for deps in dependencies {
            for &dep in deps {
                dep_counts[dep] += 1;
            }
        }
        
        // 準備完了キュー（依存関係がない命令）
        let mut ready_queue = Vec::new();
        for (i, &count) in dep_counts.iter().enumerate() {
            if count == 0 {
                ready_queue.push(i);
            }
        }
        
        // 完了した命令を追跡
        let mut completed = vec![false; instructions.len()];
        // 命令ごとの完了時間
        let mut finish_times = vec![0; instructions.len()];
        
        // 現在の時間
        let mut current_time = 0;
        
        // すべての命令がスケジュールされるまで
        while result.len() < instructions.len() {
            // 依存関係のない命令があれば選択
            if !ready_queue.is_empty() {
                // 最良の命令を選択（ここでは単純に先頭を選択）
                let next_idx = ready_queue.remove(0);
                
                // 命令を追加
                result.push(instructions[next_idx].clone());
                
                // 完了時間を設定
                let finish_time = current_time + latencies[next_idx];
                finish_times[next_idx] = finish_time;
                completed[next_idx] = true;
                
                // 依存する命令の依存カウントを更新
                for (i, deps) in dependencies.iter().enumerate() {
                    if deps.contains(&next_idx) && !completed[i] {
                        dep_counts[i] -= 1;
                        if dep_counts[i] == 0 {
                            ready_queue.push(i);
                        }
                    }
                }
                
                current_time += 1;
            } else {
                // 準備完了の命令がない場合は時間を進める
                current_time += 1;
            }
        }
        
        result
    }
    
    /// SIMD命令の活用
    fn utilize_simd(&mut self, function: &Function) -> Result<()> {
        // ベクトル化可能な処理を特定し、Neon/SVE命令に変換
        for (block_id, block) in &function.basic_blocks {
            // ループ内の並列化可能なパターンを検出
            let loop_patterns = self.detect_loop_patterns(block);
            
            for pattern in loop_patterns {
                // ベクトル化可能な操作を検出（例：配列要素に対する同一操作）
                if self.can_vectorize(&pattern) {
                    // 該当するコードブロックをSIMD命令に変換
                    self.apply_simd_optimization(*block_id, pattern)?;
                }
            }
        }
        
        Ok(())
    }
    
    /// ループパターンを検出
    fn detect_loop_patterns(&self, block: &BasicBlock) -> Vec<LoopPattern> {
        let mut patterns = Vec::new();
        
        // 高度なループ検出アルゴリズムの実装
        // 制御フロー解析とデータフロー解析を組み合わせて使用
        
        // ループ構造の識別
        let loop_info = self.analyze_loop_structure(block);
        
        // 各ループに対して詳細な分析を実行
        for loop_data in &loop_info {
            // ループ内のメモリアクセスパターンを分析
            let memory_patterns = self.analyze_memory_access_patterns(block, loop_data);
            
            // ループ内の計算パターンを分析
            let computation_patterns = self.analyze_computation_patterns(block, loop_data);
            
            // 依存関係分析
            let dependency_info = self.analyze_dependencies(block, loop_data);
            
            // ベクトル化可能性の詳細評価
            if self.is_vectorizable_loop(loop_data, &memory_patterns, &dependency_info) {
                // ストライド計算
                let stride = self.calculate_precise_stride(&memory_patterns);
                
                // 要素サイズとアライメント分析
                let element_info = self.analyze_element_properties(&memory_patterns);
                
                // 最適なSIMD命令セットの選択（Neon vs SVE）
                let simd_target = self.determine_optimal_simd_target(
                    &memory_patterns, 
                    &computation_patterns,
                    element_info.size
                );
                
                // ループの反復回数分析
                let iteration_info = self.analyze_iteration_count(loop_data);
                
                // 残余ループ処理の必要性を評価
                let needs_remainder = iteration_info.count % simd_target.vector_width != 0;
                
                // ベクトル化戦略の決定
                let strategy = self.determine_vectorization_strategy(
                    &memory_patterns,
                    &computation_patterns,
                    &dependency_info,
                    simd_target,
                    needs_remainder
                );
                
                // ループパターン情報の構築
                let pattern = LoopPattern {
                    start_idx: loop_data.header_idx,
                    length: loop_data.body_length,
                    stride,
                    element_size: element_info.size,
                    operation_type: self.determine_dominant_operation(&computation_patterns),
                    memory_access_pattern: memory_patterns,
                    computation_pattern: computation_patterns,
                    dependencies: dependency_info,
                    vectorization_strategy: strategy,
                    simd_target,
                    iteration_info,
                    alignment_info: element_info.alignment,
                    data_layout: self.analyze_data_layout(&memory_patterns),
                    reduction_operations: self.detect_reduction_operations(block, loop_data),
                    conditional_execution: self.analyze_conditional_execution(block, loop_data),
                    loop_carried_dependencies: self.analyze_loop_carried_dependencies(loop_data),
                    prefetch_distance: self.calculate_optimal_prefetch_distance(&memory_patterns),
                    unroll_factor: self.determine_optimal_unroll_factor(loop_data, &memory_patterns),
                };
                
                patterns.push(pattern);
            }
        }
        
        // 非ループコンテキストでのベクトル化可能なパターンも検出
        let sequential_patterns = self.detect_sequential_simd_patterns(block);
        patterns.extend(sequential_patterns);
        
        // 検出されたパターンの最適化機会を評価
        self.rank_vectorization_opportunities(&mut patterns);
        
        // 競合するパターンの解決（重複や入れ子になったパターンの処理）
        self.resolve_pattern_conflicts(&mut patterns);
        
        // 最終的なベクトル化パターンのリストを返す
        patterns
    }
        // 最後のパターンを追加
        if in_pattern {
            patterns.push(current_pattern);
        }
        
        patterns
    }
    
    /// アドレス計算が配列アクセスパターンかチェック
    fn is_array_access(&self, address: &ValueId) -> bool {
        // 命令列を解析して配列アクセスパターンを検出
        if let Some(def_inst) = self.get_defining_instruction(address) {
            match &def_inst.kind {
                // ベースアドレス + インデックス×要素サイズ の形式を検出
                InstructionKind::BinaryOp { op: BinaryOperator::Add, lhs, rhs } => {
                    // 左辺がベースアドレス、右辺がインデックス計算の場合
                    if self.is_base_address(lhs) && self.is_index_calculation(rhs) {
                        return true;
                    }
                    // または逆の場合
                    if self.is_base_address(rhs) && self.is_index_calculation(lhs) {
                        return true;
                    }
                },
                // ポインタ演算の場合（ptr + offset）
                InstructionKind::GetElementPtr { base, indices, .. } => {
                    // インデックスが変数または定数の場合
                    return !indices.is_empty() && indices.iter().any(|idx| self.is_loop_variant(idx));
                },
                // 配列添字アクセス
                InstructionKind::ArrayAccess { array, index } => {
                    // インデックスが変数または定数の場合
                    return self.is_loop_variant(index);
                },
                _ => {}
            }
        }
        false
    }
    
    /// メモリアクセスの間隔（ストライド）を計算
    fn calculate_stride(&self, address: &ValueId) -> usize {
        if let Some(def_inst) = self.get_defining_instruction(address) {
            match &def_inst.kind {
                // ベースアドレス + インデックス×要素サイズ
                InstructionKind::BinaryOp { op: BinaryOperator::Add, lhs, rhs } => {
                    // 右辺がインデックス計算の場合
                    if self.is_index_calculation(rhs) {
                        return self.extract_stride_from_index_calculation(rhs);
                    }
                    // 左辺がインデックス計算の場合
                    if self.is_index_calculation(lhs) {
                        return self.extract_stride_from_index_calculation(lhs);
                    }
                },
                // ポインタ演算の場合
                InstructionKind::GetElementPtr { base, indices, element_type } => {
                    // 要素サイズ × インデックスの増分
                    let element_size = self.get_type_size(element_type);
                    if let Some(idx) = indices.last() {
                        if let Some(step) = self.get_induction_variable_step(idx) {
                            return element_size * step;
                        }
                    }
                    return element_size; // デフォルトは要素サイズ
                },
                // 配列添字アクセス
                InstructionKind::ArrayAccess { array, index } => {
                    if let Some(array_type) = self.get_value_type(array) {
                        if let Type::Array(element_type, _) = array_type {
                            let element_size = self.get_type_size(&element_type);
                            if let Some(step) = self.get_induction_variable_step(index) {
                                return element_size * step;
                            }
                            return element_size;
                        }
                    }
                },
                _ => {}
            }
        }
        
        // データフロー解析で特定できない場合は、
        // 命令パターンから推測するヒューリスティックを適用
        self.estimate_stride_from_context(address)
    }
    
    /// 要素サイズを取得
    fn get_element_size(&self, inst: &Instruction) -> usize {
        match &inst.kind {
            InstructionKind::Load { ty, .. } | InstructionKind::Store { ty, .. } => {
                self.get_type_size(ty)
            },
            InstructionKind::ArrayAccess { array, .. } => {
                if let Some(array_type) = self.get_value_type(array) {
                    if let Type::Array(element_type, _) = array_type {
                        return self.get_type_size(&element_type);
                    }
                }
                self.default_element_size()
            },
            InstructionKind::GetElementPtr { element_type, .. } => {
                self.get_type_size(element_type)
            },
            InstructionKind::BinaryOp { lhs, rhs, .. } => {
                // 両オペランドの型サイズが一致すると仮定
                if let Some(ty) = self.get_value_type(lhs) {
                    return self.get_type_size(&ty);
                }
                if let Some(ty) = self.get_value_type(rhs) {
                    return self.get_type_size(&ty);
                }
                self.default_element_size()
            },
            InstructionKind::UnaryOp { operand, .. } => {
                if let Some(ty) = self.get_value_type(operand) {
                    return self.get_type_size(&ty);
                }
                self.default_element_size()
            },
            _ => self.default_element_size(),
        }
    }
    
    /// 型のサイズを取得（バイト単位）
    fn get_type_size(&self, ty: &Type) -> usize {
        match ty {
            Type::Int(width) => (width + 7) / 8, // ビット幅からバイト数へ（切り上げ）
            Type::Float32 => 4,
            Type::Float64 => 8,
            Type::Bool => 1,
            Type::Char => 1,
            Type::Pointer(_) => 8, // ARM64では64ビット（8バイト）
            Type::Array(element_type, size) => {
                if let Some(s) = size {
                    self.get_type_size(element_type) * (*s as usize)
                } else {
                    self.get_type_size(element_type) // サイズ不明の場合は要素サイズのみ
                }
            },
            Type::Struct(fields) => {
                // 構造体フィールドのアラインメントを考慮したサイズ計算
                let mut total_size = 0;
                let mut max_align = 1;
                
                for field_type in fields {
                    let field_size = self.get_type_size(field_type);
                    let field_align = self.get_type_alignment(field_type);
                    
                    // アラインメント調整
                    total_size = (total_size + field_align - 1) / field_align * field_align;
                    total_size += field_size;
                    max_align = max_align.max(field_align);
                }
                
                // 構造体全体のアラインメント調整
                (total_size + max_align - 1) / max_align * max_align
            },
            Type::Tuple(types) => {
                // タプルもアラインメントを考慮
                let mut total_size = 0;
                let mut max_align = 1;
                
                for element_type in types {
                    let element_size = self.get_type_size(element_type);
                    let element_align = self.get_type_alignment(element_type);
                    
                    // アラインメント調整
                    total_size = (total_size + element_align - 1) / element_align * element_align;
                    total_size += element_size;
                    max_align = max_align.max(element_align);
                }
                
                // タプル全体のアラインメント調整
                (total_size + max_align - 1) / max_align * max_align
            },
            Type::Union(types) => {
                // 共用体は最大のフィールドサイズ
                let mut max_size = 0;
                let mut max_align = 1;
                
                for ty in types {
                    let size = self.get_type_size(ty);
                    let align = self.get_type_alignment(ty);
                    max_size = max_size.max(size);
                    max_align = max_align.max(align);
                }
                
                // アラインメント調整
                (max_size + max_align - 1) / max_align * max_align
            },
            Type::Function(_, _) => 8, // 関数ポインタは64ビット
            Type::Void => 0,
            Type::Unknown => self.default_element_size(),
            // 依存型など高度な型システムのサポート
            Type::Dependent(_) => self.resolve_dependent_type_size(ty),
            Type::TypeVar(_) => self.default_element_size(), // 型変数はコンテキストから解決
            Type::Existential(_) => self.default_element_size(), // 存在型
        }
    }
    
    /// 型のアラインメントを取得
    fn get_type_alignment(&self, ty: &Type) -> usize {
        match ty {
            Type::Int(width) => {
                let size = (width + 7) / 8; // ビット幅からバイト数へ
                // 2のべき乗にアラインする
                if size <= 1 { 1 }
                else if size <= 2 { 2 }
                else if size <= 4 { 4 }
                else { 8 }
            },
            Type::Float32 => 4,
            Type::Float64 => 8,
            Type::Bool => 1,
            Type::Char => 1,
            Type::Pointer(_) => 8,
            Type::Array(element_type, _) => self.get_type_alignment(element_type),
            Type::Struct(fields) => {
                // 構造体のアラインメントは最大のフィールドアラインメント
                fields.iter()
                    .map(|field_type| self.get_type_alignment(field_type))
                    .max()
                    .unwrap_or(1)
            },
            Type::Tuple(types) => {
                // タプルのアラインメントも最大の要素アラインメント
                types.iter()
                    .map(|element_type| self.get_type_alignment(element_type))
                    .max()
                    .unwrap_or(1)
            },
            Type::Union(types) => {
                // 共用体のアラインメントも最大のフィールドアラインメント
                types.iter()
                    .map(|ty| self.get_type_alignment(ty))
                    .max()
                    .unwrap_or(1)
            },
            Type::Function(_, _) => 8,
            Type::Void => 1,
            Type::Unknown => 8, // 不明な場合は最大アラインメントを仮定
            Type::Dependent(_) => self.resolve_dependent_type_alignment(ty),
            Type::TypeVar(_) => 8, // 型変数は最大アラインメントを仮定
            Type::Existential(_) => 8, // 存在型も最大アラインメントを仮定
        }
    }
    
    /// 依存型のサイズを解決
    fn resolve_dependent_type_size(&self, ty: &Type) -> usize {
        if let Type::Dependent(expr) = ty {
            // 依存型式の評価を試みる
            match self.evaluate_dependent_type_expr(expr) {
                Some(evaluated_type) => {
                    // 評価された型のサイズを返す
                    return self.get_type_size(&evaluated_type);
                },
                None => {
                    // 式の部分評価を試みる
                    if let Some(partial_result) = self.partially_evaluate_dependent_expr(expr) {
                        // 部分評価の結果に基づいてサイズを計算
                        if let Some(size) = self.compute_size_from_partial_evaluation(&partial_result) {
                            return size;
                        }
                        
                        // 部分評価から型制約を抽出
                        let constraints = self.extract_type_constraints(&partial_result);
                        if let Some(min_size) = self.derive_minimum_size_from_constraints(&constraints) {
                            return min_size;
                        }
                    }
                    
                    // 型レベルの数値定数を抽出して計算
                    if let Some(constant_size) = self.extract_constant_size_from_expr(expr) {
                        return constant_size;
                    }
                    
                    // 依存型の構造解析
                    if let Some(structural_size) = self.analyze_dependent_type_structure(expr) {
                        return structural_size;
                    }
                    
                    // キャッシュされた評価結果を確認
                    if let Some(cached_size) = self.lookup_cached_dependent_type_size(expr) {
                        return cached_size;
                    }
                    
                    // 型推論システムに問い合わせ
                    if let Some(inferred_size) = self.query_type_inference_system(expr) {
                        // 結果をキャッシュして返す
                        self.cache_dependent_type_size(expr, inferred_size);
                        return inferred_size;
                    }
                }
            }
        }
        
        // 解決できない場合はコンテキストに基づいて最適なデフォルト値を選択
        let context_size = self.determine_context_appropriate_size(ty);
        if context_size > 0 {
            return context_size;
        }
        
        // 最終的なフォールバック
        self.default_element_size()
    }
    
    /// 依存型のアラインメントを解決
    fn resolve_dependent_type_alignment(&self, ty: &Type) -> usize {
        if let Type::Dependent(expr) = ty {
            if let Some(evaluated_type) = self.evaluate_dependent_type_expr(expr) {
                return self.get_type_alignment(&evaluated_type);
            }
        }
        8 // 解決できない場合は最大アラインメント
    }
    
    /// 依存型式を評価
    fn evaluate_dependent_type_expr(&self, expr: &DependentTypeExpr) -> Option<Type> {
        // 依存型式の評価ロジック
        // コンパイル時計算を実行
        match expr {
            // 型レベル関数適用
            DependentTypeExpr::Apply(func, args) => {
                // 関数と引数を評価
                self.evaluate_type_level_function(func, args)
            },
            // 型レベル条件分岐
            DependentTypeExpr::If(cond, then_type, else_type) => {
                if self.evaluate_type_level_condition(cond) {
                    Some(then_type.clone())
                } else {
                    Some(else_type.clone())
                }
            },
            // 他の依存型式...
            _ => None,
        }
    }
    
    /// デフォルトの要素サイズを返す
    fn default_element_size(&self) -> usize {
        4 // 32ビット（4バイト）をデフォルトとする
    }
    
    /// 値の定義命令を取得
    fn get_defining_instruction(&self, value: &ValueId) -> Option<&Instruction> {
        // データフローグラフを使用して値を定義している命令を探索
        // キャッシュを確認して高速アクセスを実現
        if let Some(cached_inst) = self.instruction_cache.get(value) {
            return Some(cached_inst);
        }
        
        // 関数のデータフローグラフから定義命令を取得
        let result = match value {
            ValueId::Instruction(id) => {
                // 命令IDから直接命令を取得
                self.function.instructions.get(id)
            },
            ValueId::Parameter(param_idx) => {
                // パラメータは定義命令を持たないのでNone
                None
            },
            ValueId::Constant(_) => {
                // 定数は定義命令を持たないのでNone
                None
            },
            ValueId::GlobalVariable(name) => {
                // グローバル変数の場合、モジュールレベルの定義を検索
                self.module.get_global_variable_definition(name)
            },
            ValueId::TemporarySSA(temp_id) => {
                // SSA一時変数の定義命令を探索
                self.function.get_ssa_definition(*temp_id)
            },
            ValueId::PhiNode(block_id, phi_idx) => {
                // Phi節点の場合、対応するブロックからPhi命令を取得
                self.function.get_phi_instruction(*block_id, *phi_idx)
            },
            ValueId::VirtualRegister(reg_id) => {
                // 仮想レジスタの最後の定義命令を取得
                self.function.get_virtual_register_definition(*reg_id)
            },
            ValueId::DependentValue(expr) => {
                // 依存値の場合、式を評価して対応する命令を取得
                if let Some(evaluated) = self.evaluate_dependent_value_expr(expr) {
                    self.get_defining_instruction(&evaluated)
                } else {
                    None
                }
            },
        };
        
        // 結果をキャッシュに格納して将来のアクセスを高速化
        if let Some(inst) = &result {
            self.instruction_cache.insert(value.clone(), inst.clone());
        }
        
        result
    }
    /// 値の型を取得
    fn get_value_type(&self, value: &ValueId) -> Option<Type> {
        // 値の型情報を取得
        self.function.get_value_type(value)
    }
    
    /// ベースアドレスかどうかを判定
    fn is_base_address(&self, value: &ValueId) -> bool {
        if let Some(def_inst) = self.get_defining_instruction(value) {
            match &def_inst.kind {
                // 配列や構造体のアドレス
                InstructionKind::GetElementPtr { .. } => true,
                // 変数のアドレス
                InstructionKind::AddressOf { .. } => true,
                // グローバル変数
                InstructionKind::GlobalVariable { .. } => true,
                // アロケーション
                InstructionKind::Alloca { .. } => true,
                // 他のベースアドレスパターン
                _ => false,
            }
        } else {
            // 関数パラメータなどの場合
            matches!(value, ValueId::Parameter(_))
        }
    }
    
    /// インデックス計算かどうかを判定
    fn is_index_calculation(&self, value: &ValueId) -> bool {
        if let Some(def_inst) = self.get_defining_instruction(value) {
            match &def_inst.kind {
                // インデックス × 要素サイズ
                InstructionKind::BinaryOp { op: BinaryOperator::Mul, lhs, rhs } => {
                    (self.is_loop_variant(lhs) && self.is_constant_or_size(rhs)) ||
                    (self.is_loop_variant(rhs) && self.is_constant_or_size(lhs))
                },
                // シフト演算（インデックス << 2 など）
                InstructionKind::BinaryOp { op: BinaryOperator::Shl, lhs, rhs } => {
                    self.is_loop_variant(lhs) && self.is_constant(rhs)
                },
                // 他のインデックス計算パターン
                _ => false,
            }
        } else {
            false
        }
    }
    
    /// ループ変数かどうかを判定
    fn is_loop_variant(&self, value: &ValueId) -> bool {
        // ループのインダクション変数かどうかを判定
        if let Some(def_inst) = self.get_defining_instruction(value) {
            match &def_inst.kind {
                // PHI命令はループ変数の可能性が高い
                InstructionKind::Phi { .. } => true,
                // インクリメント/デクリメント
                InstructionKind::BinaryOp { op, lhs, rhs } => {
                    if matches!(op, BinaryOperator::Add | BinaryOperator::Sub) {
                        // i = i + 1 または i = i - 1 のパターン
                        (self.is_self_reference(lhs, value) && self.is_constant(rhs)) ||
                        (self.is_self_reference(rhs, value) && self.is_constant(lhs))
                    } else {
                        false
                    }
                },
                // 他のループ変数パターン
                _ => false,
            }
        } else {
            // 関数パラメータなどの場合
            matches!(value, ValueId::Parameter(_))
        }
    }
    
    /// 定数または要素サイズを表す値かどうかを判定
    fn is_constant_or_size(&self, value: &ValueId) -> bool {
        self.is_constant(value) || self.is_element_size_value(value)
    }
    
    /// 定数かどうかを判定
    fn is_constant(&self, value: &ValueId) -> bool {
        matches!(value, ValueId::Constant(_)) ||
        if let Some(def_inst) = self.get_defining_instruction(value) {
            matches!(def_inst.kind, InstructionKind::Constant { .. })
        } else {
            false
        }
    }
    
    /// 要素サイズを表す値かどうかを判定
    fn is_element_size_value(&self, value: &ValueId) -> bool {
        // 要素サイズを表す定数かどうかを判定
        // 例: 4, 8, sizeof(T) など
        if let Some(constant_value) = self.get_constant_value(value) {
            // 2のべき乗チェック（要素サイズは通常2のべき乗）
            constant_value > 0 && (constant_value & (constant_value - 1)) == 0
        } else {
            false
        }
    }
    
    /// 定数値を取得
    fn get_constant_value(&self, value: &ValueId) -> Option<usize> {
        match value {
            ValueId::Constant(c) => Some(*c as usize),
            _ => {
                if let Some(def_inst) = self.get_defining_instruction(value) {
                    match &def_inst.kind {
                        InstructionKind::Constant { value: c, .. } => {
                            match c {
                                ConstantValue::Int(i) => Some(*i as usize),
                                _ => None,
                            }
                        },
                        _ => None,
                    }
                } else {
                    None
                }
            }
        }
    }
    
    /// 自己参照かどうかを判定
    fn is_self_reference(&self, value: &ValueId, reference: &ValueId) -> bool {
        value == reference
    }
    
    /// インデックス計算からストライドを抽出
    fn extract_stride_from_index_calculation(&self, value: &ValueId) -> usize {
        if let Some(def_inst) = self.get_defining_instruction(value) {
            match &def_inst.kind {
                // インデックス × 要素サイズ
                InstructionKind::BinaryOp { op: BinaryOperator::Mul, lhs, rhs } => {
                    if self.is_loop_variant(lhs) && self.is_constant_or_size(rhs) {
                        return self.get_constant_value(rhs).unwrap_or(1);
                    }
                    if self.is_loop_variant(rhs) && self.is_constant_or_size(lhs) {
                        return self.get_constant_value(lhs).unwrap_or(1);
                    }
                },
                // シフト演算（インデックス << 2 は × 4 と同等）
                InstructionKind::BinaryOp { op: BinaryOperator::Shl, lhs, rhs } => {
                    if self.is_loop_variant(lhs) && self.is_constant(rhs) {
                        if let Some(shift) = self.get_constant_value(rhs) {
                            return 1 << shift;
                        }
                    }
                },
                _ => {}
            }
        }
        1 // デフォルトストライド
    }
    
    /// インダクション変数のステップサイズを取得
    fn get_induction_variable_step(&self, value: &ValueId) -> Option<usize> {
        if let Some(def_inst) = self.get_defining_instruction(value) {
            match &def_inst.kind {
                InstructionKind::Phi { incoming } => {
                    // PHI命令の入力を解析してステップサイズを特定
                    for (val, _) in incoming {
                        if let Some(update_inst) = self.get_defining_instruction(val) {
                            match &update_inst.kind {
                                InstructionKind::BinaryOp { op: BinaryOperator::Add, lhs, rhs } => {
                                    // i = i + step パターン
                                    if self.is_self_reference(lhs, value) && self.is_constant(rhs) {
                                        return self.get_constant_value(rhs);
                                    }
                                    if self.is_self_reference(rhs, value) && self.is_constant(lhs) {
                                        return self.get_constant_value(lhs);
                                    }
                                },
                                _ => {}
                            }
                        }
                    }
                },
                _ => {}
            }
        }
        None
    }
    
    /// コンテキストからストライドを推測
    fn estimate_stride_from_context(&self, address: &ValueId) -> usize {
        // 周囲のコードパターンからストライドを推測
        // 例: 連続したメモリアクセスのパターンを検出
        
        // 同じブロック内の類似アドレス計算を探す
        let mut stride_candidates = Vec::new();
        
        // 現在の関数内の全命令を走査
        for block in &self.function.blocks {
            for inst in &block.instructions {
                match &inst.kind {
                    InstructionKind::Load { address: addr, .. } | 
                    InstructionKind::Store { address: addr, .. } => {
                        if addr != address && self.is_array_access(addr) {
                            // 類似のアドレス計算パターンを見つけた
                            if let Some(stride) = self.analyze_address_difference(address, addr) {
                                stride_candidates.push(stride);
                            }
                        }
                    },
                    _ => {}
                }
            }
        }
        
        // 最も頻度の高いストライド値を採用
        if !stride_candidates.is_empty() {
            let mut stride_counts = std::collections::HashMap::new();
            for stride in stride_candidates {
                *stride_counts.entry(stride).or_insert(0) += 1;
            }
            
            return stride_counts.into_iter()
                .max_by_key(|(_, count)| *count)
                .map(|(stride, _)| stride)
                .unwrap_or(4);
        }
        
        // デフォルトストライド
        4
    }
    
    /// 2つのアドレス計算の差分を解析
    fn analyze_address_difference(&self, addr1: &ValueId, addr2: &ValueId) -> Option<usize> {
        // アドレス計算の構造を取得
        let addr1_structure = self.extract_address_structure(addr1)?;
        let addr2_structure = self.extract_address_structure(addr2)?;
        
        // 同じベースアドレスを使用している場合のみ比較可能
        if addr1_structure.base != addr2_structure.base {
            return None;
        }
        
        // インデックス変数が関連している場合（同じ変数または連続した値）
        if self.are_indices_related(&addr1_structure.index, &addr2_structure.index) {
            // スケールが同じ場合、それがストライド
            if addr1_structure.scale == addr2_structure.scale {
                return Some(addr1_structure.scale);
            }
            
            // スケールが異なる場合、その差分を計算
            let scale_diff = if addr1_structure.scale > addr2_structure.scale {
                addr1_structure.scale - addr2_structure.scale
            } else {
                addr2_structure.scale - addr1_structure.scale
            };
            
            // 差分が一定のパターンに従っている場合
            if scale_diff > 0 && (scale_diff & (scale_diff - 1)) == 0 {  // 2のべき乗チェック
                return Some(scale_diff);
            }
        }
        
        // オフセットの差分を計算
        let offset_diff = if addr1_structure.offset > addr2_structure.offset {
            addr1_structure.offset - addr2_structure.offset
        } else {
            addr2_structure.offset - addr1_structure.offset
        };
        
        // オフセットの差分が有意義な値（2のべき乗など）であれば、それをストライドとして採用
        if offset_diff > 0 {
            // データ型サイズに基づく一般的なストライド値をチェック
            let common_strides = [1, 2, 4, 8, 16, 32, 64];
            if common_strides.contains(&offset_diff) {
                return Some(offset_diff);
            }
            
            // 2のべき乗チェック（SIMD操作に適したストライド）
            if (offset_diff & (offset_diff - 1)) == 0 {
                return Some(offset_diff);
            }
        }
        
        // 依存関係グラフを解析して、より複雑なパターンを検出
        if let Some(stride) = self.analyze_dependency_graph(addr1, addr2) {
            return Some(stride);
        }
        
        // 明確なパターンが見つからない場合
        None
    }
    /// 操作タイプを判定
    fn determine_operation_type(&self, inst: &Instruction) -> OperationType {
        match &inst.kind {
            InstructionKind::Load { .. } => OperationType::Load,
            InstructionKind::Store { .. } => OperationType::Store,
            InstructionKind::BinaryOp { op, .. } => {
                match op {
                    BinaryOperator::Add => OperationType::Add,
                    BinaryOperator::Sub => OperationType::Sub,
                    BinaryOperator::Mul => OperationType::Mul,
                    _ => OperationType::Other,
                }
            },
            _ => OperationType::Other,
        }
    }
    
    /// 命令がベクトル化可能かチェック
    fn is_vectorizable_operation(&self, inst: &Instruction) -> bool {
        match &inst.kind {
            InstructionKind::BinaryOp { op, .. } => {
                // 加算、減算、乗算などの基本演算はベクトル化可能
                matches!(op, BinaryOperator::Add | BinaryOperator::Sub | BinaryOperator::Mul)
            },
            InstructionKind::UnaryOp { op, .. } => {
                // 否定、ビット反転などもベクトル化可能
                matches!(op, UnaryOperator::Neg | UnaryOperator::Not)
            },
            _ => false,
        }
    }
    
    /// パターンがベクトル化可能かチェック
    fn can_vectorize(&self, pattern: &LoopPattern) -> bool {
        // ベクトル化の条件：
        // 1. パターンの長さが十分長い（SIMD命令の要素数以上）
        // 2. 連続したメモリアクセス（ストライドが一定）
        // 3. サポートされている操作タイプ
        
        // 最低4要素（128ビットNeon）以上であればベクトル化価値あり
        let min_length = 4;
        
        pattern.length >= min_length &&
        pattern.stride > 0 &&
        matches!(
            pattern.operation_type,
            OperationType::Add | OperationType::Sub | OperationType::Mul |
            OperationType::Load | OperationType::Store
        )
    }
    
    /// SIMD最適化を適用
    fn apply_simd_optimization(&mut self, block_id: usize, pattern: LoopPattern) -> Result<()> {
        // ブロックの命令リストを取得
        if let Some(instructions) = self.instruction_selection.get_mut(&block_id) {
            // 最適化対象の命令範囲
            let start_idx = pattern.start_idx;
            let end_idx = start_idx + pattern.length;
            
            // SIMD命令に置き換えるための新しい命令列
            let mut simd_instructions = Vec::new();
            
            match pattern.operation_type {
                OperationType::Load => {
                    // 連続したロード命令をSIMDロードに置き換え
                    simd_instructions.push(format!("// SIMD optimized load"));
                    simd_instructions.push(format!("ld1 {{v0.4s}}, [x0]"));
                },
                OperationType::Store => {
                    // 連続したストア命令をSIMDストアに置き換え
                    simd_instructions.push(format!("// SIMD optimized store"));
                    simd_instructions.push(format!("st1 {{v0.4s}}, [x0]"));
                },
                OperationType::Add => {
                    // 加算操作のベクトル化
                    simd_instructions.push(format!("// SIMD optimized add"));
                    simd_instructions.push(format!("ld1 {{v0.4s}}, [x0]"));
                    simd_instructions.push(format!("ld1 {{v1.4s}}, [x1]"));
                    simd_instructions.push(format!("fadd v0.4s, v0.4s, v1.4s"));
                    simd_instructions.push(format!("st1 {{v0.4s}}, [x0]"));
                },
                OperationType::Sub => {
                    // 減算操作のベクトル化
                    simd_instructions.push(format!("// SIMD optimized subtract"));
                    simd_instructions.push(format!("ld1 {{v0.4s}}, [x0]"));
                    simd_instructions.push(format!("ld1 {{v1.4s}}, [x1]"));
                    simd_instructions.push(format!("fsub v0.4s, v0.4s, v1.4s"));
                    simd_instructions.push(format!("st1 {{v0.4s}}, [x0]"));
                },
                OperationType::Mul => {
                    // 乗算操作のベクトル化
                    simd_instructions.push(format!("// SIMD optimized multiply"));
                    simd_instructions.push(format!("ld1 {{v0.4s}}, [x0]"));
                    simd_instructions.push(format!("ld1 {{v1.4s}}, [x1]"));
                    simd_instructions.push(format!("fmul v0.4s, v0.4s, v1.4s"));
                    simd_instructions.push(format!("st1 {{v0.4s}}, [x0]"));
                },
                _ => {
                    // サポートされていない操作
                    return Ok(());
                }
            }
            
            // 元の命令をSIMD命令に置き換え
            // 置き換え対象の命令を削除
            instructions.splice(start_idx..end_idx, simd_instructions);
        }
        
        Ok(())
    }



/// ループパターンの情報
#[derive(Clone, Default)]
struct LoopPattern {
    /// パターンの開始インデックス
    start_idx: usize,
    /// パターンの長さ（命令数）
    length: usize,
    /// メモリアクセスのストライド
    stride: usize,
    /// 要素サイズ
    element_size: usize,
    /// 操作タイプ
    operation_type: OperationType,
}

/// 操作タイプ
#[derive(Clone, Copy, PartialEq)]
enum OperationType {
    Load,
    Store,
    Add,
    Sub,
    Mul,
    Other,
}

impl Default for OperationType {
    fn default() -> Self {
        OperationType::Other
    }
}

pub struct ARM64Generator {
    // 生成したアセンブリコード
    asm_code: String,
    // 関数マップ
    functions: HashMap<usize, String>,
    // ラベルカウンタ
    label_counter: usize,
    // レジスタ割り当て結果
    register_map: HashMap<usize, String>,
    // 浮動小数点変数のセット
    float_vars: HashSet<usize>,
}

impl ARM64Generator {
    /// 新しいARM64生成器を作成
    pub fn new() -> Self {
        Self {
            asm_code: String::new(),
            functions: HashMap::new(),
            label_counter: 0,
            register_map: HashMap::new(),
            float_vars: HashSet::new(),
        }
    }
}

//! # ARM64 コード生成
//! 
//! ARM64アーキテクチャ向けのネイティブコードを生成するモジュールです。
//! 主にLLVMバックエンドが生成したオブジェクトコードに対して、さらなる最適化を行います。

use std::collections::HashMap;
use std::collections::HashSet;

use crate::frontend::error::{CompilerError, ErrorKind, Result};
use crate::middleend::ir::{Module, Function, Instruction};

/// ARM64向け最適化器
pub struct ARM64Optimizer {
    /// レジスタ割り当て
    register_allocation: HashMap<usize, String>,
    /// 命令選択情報
    instruction_selection: HashMap<usize, Vec<String>>,
}

impl ARM64Optimizer {
    /// 新しい最適化器を作成
    pub fn new() -> Self {
        Self {
            register_allocation: HashMap::new(),
            instruction_selection: HashMap::new(),
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
            
            if let Some(neighbors) = interference_graph.get(&node) {
                for neighbor in neighbors {
                    if let Some(reg) = result.get(neighbor) {
                        used_registers.insert(reg.clone());
                    }
                }
            }
            
            // ノードが整数型か浮動小数点型かに応じて適切なレジスタセットを選択
            let register_pool = if self.is_float_var(node) {
                float_registers
            } else {
                general_registers
            };
            
            // 使用されていないレジスタを探す
            let assigned_register = register_pool.iter()
                .find(|reg| !used_registers.contains(*reg) && !reserved_registers.contains(*reg))
                .cloned();
            
            if let Some(reg) = assigned_register {
                result.insert(node, reg.to_string());
            } else {
                // レジスタ割り当てに失敗した場合はスピル処理
                // （実際はスピル処理として変数をスタックにオフロードする）
                result.insert(node, format!("spill_{}", node));
            }
        }
        
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
                    // フォールスルーがあればそれも後続ブロック
                    if let Some(fallthrough) = block.fallthrough {
                        successors.push(fallthrough);
                    }
                }
            }
        } else if let Some(fallthrough) = block.fallthrough {
            // 命令がない場合でもフォールスルーがあれば後続ブロック
            successors.push(fallthrough);
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
                let dst_reg = self.get_register_for_var(inst.output.unwrap_or(0));
                
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
                let dst_reg = self.get_register_for_var(inst.output.unwrap_or(0));
                
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
                let dst_reg = self.get_register_for_var(inst.output.unwrap_or(0));
                
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
                if let Some(output) = inst.output {
                    let dst_reg = self.get_register_for_var(output);
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
            Type::Struct(fields) => {
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
            Type::Struct(fields) => {
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
    
    /// 命令の定義レジスタと使用レジスタを抽出
    fn extract_registers(&self, instruction: &str) -> (Vec<String>, Vec<String>) {
        let mut def_regs = Vec::new();
        let mut use_regs = Vec::new();
        
        // 命令文字列を解析して定義・使用レジスタを特定
        let parts: Vec<&str> = instruction.split_whitespace().collect();
        
        if parts.is_empty() {
            return (def_regs, use_regs);
        }
        
        // 命令オペコードに基づく解析
        match parts[0] {
            "add" | "sub" | "mul" | "sdiv" | "and" | "orr" | "eor" | "lsl" | "lsr" | "asr" | "ror" => {
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
            "bl" => {
                // 関数呼び出しは特別扱い（引数レジスタと戻り値レジスタに依存）
                // x0-x7は引数レジスタとして使用
                for i in 0..8 {
                    use_regs.push(format!("x{}", i));
                }
                // x0は戻り値レジスタとして定義
                def_regs.push("x0".to_string());
            },
            _ => {
                // その他の命令は簡略化して処理
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
        
        // ループの基本パターン検出のためのシンプルな解析
        // 実際の実装では制御フロー解析とループ検出アルゴリズムが必要
        
        // ここではシンプルな例として、連続したメモリアクセスパターンを持つ
        // 命令シーケンスを検出する。
        
        let mut current_pattern = LoopPattern::default();
        let mut in_pattern = false;
        
        for inst in &block.instructions {
            match &inst.kind {
                InstructionKind::Load { address, .. } | InstructionKind::Store { address, .. } => {
                    // メモリアクセスが配列インデックス計算のパターンかチェック
                    if self.is_array_access(address) {
                        if !in_pattern {
                            // 新しいパターンの開始
                            current_pattern = LoopPattern {
                                start_idx: 0, // 実際はブロック内の命令インデックス
                                length: 1,
                                stride: self.calculate_stride(address),
                                element_size: self.get_element_size(inst),
                                operation_type: self.determine_operation_type(inst),
                            };
                            in_pattern = true;
                        } else {
                            // 既存パターンの延長
                            current_pattern.length += 1;
                        }
                    } else if in_pattern {
                        // パターンの終了
                        patterns.push(current_pattern.clone());
                        in_pattern = false;
                    }
                },
                _ => {
                    // 他の命令はパターンを中断する可能性がある
                    if in_pattern {
                        // 特定の算術命令はパターンに含められるかチェック
                        if self.is_vectorizable_operation(inst) {
                            current_pattern.length += 1;
                        } else {
                            // パターンの終了
                            patterns.push(current_pattern.clone());
                            in_pattern = false;
                        }
                    }
                }
            }
        }
        
        // 最後のパターンを追加
        if in_pattern {
            patterns.push(current_pattern);
        }
        
        patterns
    }
    
    /// アドレス計算が配列アクセスパターンかチェック
    fn is_array_access(&self, address: &ValueId) -> bool {
        // 簡易的な実装：実際は命令列を調査し、ベースアドレス+インデックス×要素サイズの
        // パターンかどうかを判断する必要がある
        match address {
            ValueId::Variable(_) => true, // 変数アドレスを単純化のためtrueとする
            _ => false,
        }
    }
    
    /// メモリアクセスの間隔（ストライド）を計算
    fn calculate_stride(&self, address: &ValueId) -> usize {
        // 実際の実装では命令列から間隔を計算
        // 例えばアドレス計算が base + i*4 のような形式であれば、4がストライド
        4 // デフォルト値
    }
    
    /// 要素サイズを取得
    fn get_element_size(&self, inst: &Instruction) -> usize {
        match &inst.kind {
            InstructionKind::Load { ty, .. } | InstructionKind::Store { ty, .. } => {
                self.get_type_size(ty)
            },
            _ => 4, // デフォルト値
        }
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

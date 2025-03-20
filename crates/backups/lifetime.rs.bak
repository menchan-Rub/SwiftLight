// lifetime.rs - SwiftLight ライフタイム解析モジュール
//
// このモジュールは、中間表現に対してライフタイム解析を行い、
// 変数や参照のライフタイムを追跡するための機能を提供します。
// これにより、所有権と借用のルールを厳密に適用し、メモリ安全性を確保します。

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

use crate::middleend::ir::{
    BasicBlock, Function, Instruction, Module, Value, ValueId,
    Type, TypeId, ControlFlowGraph, InstructionId, BorrowKind
};
use crate::middleend::analysis::dataflow::{DataFlowEngine, DataFlowAnalysis, DataFlowType};

/// 変数の参照とそのライフタイム情報を表す構造体
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BorrowInfo {
    /// 借用元の変数ID
    pub source_id: ValueId,
    /// 借用先の変数ID（参照を保持する変数）
    pub borrower_id: ValueId,
    /// 借用の種類（共有または可変）
    pub kind: BorrowKind,
    /// 借用が開始する命令ID
    pub start_inst: InstructionId,
    /// 借用が終了する命令ID（存在する場合）
    pub end_inst: Option<InstructionId>,
    /// ライフタイム識別子（名前ベースでの追跡用）
    pub lifetime_id: String,
}

/// ライフタイム解析エンジン
pub struct LifetimeAnalyzer {
    /// 現在分析中の関数
    function: Option<Function>,
    /// 各変数の借用情報
    borrows: HashMap<ValueId, Vec<BorrowInfo>>,
    /// 各変数が現在借用されているか
    is_borrowed: HashMap<ValueId, bool>,
    /// 各命令で活性な借用のセット
    active_borrows: HashMap<InstructionId, HashSet<ValueId>>,
    /// データフロー解析エンジン
    dataflow_engine: DataFlowEngine,
    /// ライフタイム識別子のカウンタ
    lifetime_counter: usize,
}

impl LifetimeAnalyzer {
    /// 新しいライフタイム解析エンジンを作成
    pub fn new() -> Self {
        Self {
            function: None,
            borrows: HashMap::new(),
            is_borrowed: HashMap::new(),
            active_borrows: HashMap::new(),
            dataflow_engine: DataFlowEngine::new(),
            lifetime_counter: 0,
        }
    }
    
    /// 関数のライフタイム解析を実行
    pub fn analyze_function(&mut self, function: &Function) -> Result<LifetimeReport, String> {
        self.function = Some(function.clone());
        self.borrows.clear();
        self.is_borrowed.clear();
        self.active_borrows.clear();
        self.lifetime_counter = 0;
        
        // 借用を検出
        self.detect_borrows()?;
        
        // ライフタイムを計算
        self.compute_lifetimes()?;
        
        // 借用チェック
        self.validate_borrows()?;
        
        // 結果をレポートに集約
        let report = self.generate_report();
        
        Ok(report)
    }
    
    /// 借用操作を検出
    fn detect_borrows(&mut self) -> Result<(), String> {
        let function = self.function.as_ref()
            .ok_or_else(|| "関数が設定されていません".to_string())?;
        
        // すべての命令を走査
        for (inst_id, inst) in function.instructions.iter().enumerate() {
            match inst.opcode.as_str() {
                // 参照作成
                "ref" | "ref_mut" => {
                    if let Some(source_id) = inst.operands.get(0) {
                        if let Some(result_id) = inst.result {
                            let kind = if inst.opcode == "ref" {
                                BorrowKind::Shared
                            } else {
                                BorrowKind::Mutable
                            };
                            
                            let lifetime_id = self.generate_lifetime_id();
                            
                            let borrow_info = BorrowInfo {
                                source_id: *source_id,
                                borrower_id: result_id,
                                kind,
                                start_inst: inst_id,
                                end_inst: None, // まだ終了点は不明
                                lifetime_id,
                            };
                            
                            self.borrows.entry(*source_id)
                                .or_insert_with(Vec::new)
                                .push(borrow_info);
                            
                            self.is_borrowed.insert(*source_id, true);
                        }
                    }
                },
                
                // 変数への代入は、その変数への借用を無効化する可能性がある
                "store" => {
                    if let Some(target_id) = inst.operands.get(1) {
                        // 可変借用がある場合はエラー
                        if let Some(borrows) = self.borrows.get(target_id) {
                            for borrow in borrows {
                                if borrow.kind == BorrowKind::Mutable && borrow.end_inst.is_none() {
                                    return Err(format!(
                                        "可変借用中の変数 {} への代入です（命令ID: {}）",
                                        target_id, inst_id
                                    ));
                                }
                            }
                        }
                    }
                },
                
                // 他の命令は必要に応じて解析
                _ => {}
            }
        }
        
        Ok(())
    }
    
    /// ライフタイムを計算
    fn compute_lifetimes(&mut self) -> Result<(), String> {
        let function = self.function.as_ref()
            .ok_or_else(|| "関数が設定されていません".to_string())?;
        
        // 変数の活性解析を実行（借用の終了点を決定するため）
        let liveness_analyzer = self.compute_liveness(function)?;
        
        // 各借用の終了点を設定
        for (source_id, borrows) in &mut self.borrows {
            for borrow in borrows {
                // 借用先（参照を保持する変数）の最後の使用点を見つける
                let mut last_use = None;
                
                for (inst_id, live_vars) in &liveness_analyzer.after_inst {
                    if live_vars.contains(&borrow.borrower_id) {
                        if last_use.is_none() || *inst_id > last_use.unwrap() {
                            last_use = Some(*inst_id);
                        }
                    }
                }
                
                // 最後の使用点があれば、それを借用の終了点とする
                if let Some(last_inst) = last_use {
                    borrow.end_inst = Some(last_inst);
                }
            }
        }
        
        // 各命令で活性な借用を計算
        for (inst_id, _) in function.instructions.iter().enumerate() {
            let mut active = HashSet::new();
            
            for (source_id, borrows) in &self.borrows {
                for borrow in borrows {
                    // この命令が借用の開始と終了の間にあるか判定
                    if inst_id >= borrow.start_inst && 
                       (borrow.end_inst.is_none() || inst_id <= borrow.end_inst.unwrap()) {
                        active.insert(*source_id);
                    }
                }
            }
            
            self.active_borrows.insert(inst_id, active);
        }
        
        Ok(())
    }
    
    /// 借用の有効性を検証
    fn validate_borrows(&self) -> Result<(), String> {
        let function = self.function.as_ref()
            .ok_or_else(|| "関数が設定されていません".to_string())?;
        
        // 各命令で変数への可変アクセスをチェック
        for (inst_id, inst) in function.instructions.iter().enumerate() {
            if let Some(active) = self.active_borrows.get(&inst_id) {
                // 命令の操作対象変数を検査
                for &operand_id in &inst.operands {
                    // この変数に対する借用が活性か
                    if active.contains(&operand_id) {
                        // 可変操作をする命令か判定
                        let is_mutation = match inst.opcode.as_str() {
                            "store" | "modify" => true,
                            _ => false,
                        };
                        
                        if is_mutation {
                            // 可変操作時に共有借用があるとエラー
                            if let Some(borrows) = self.borrows.get(&operand_id) {
                                for borrow in borrows {
                                    if borrow.kind == BorrowKind::Shared && 
                                       (borrow.end_inst.is_none() || inst_id <= borrow.end_inst.unwrap()) {
                                        return Err(format!(
                                            "共有借用中の変数 {} への可変アクセスです（命令ID: {}）",
                                            operand_id, inst_id
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// 変数の活性解析を実行
    fn compute_liveness(&self, function: &Function) -> Result<super::dataflow::DataFlowResult<HashSet<ValueId>>, String> {
        use super::dataflow::LiveVariableAnalysis;
        
        let liveness_analysis = LiveVariableAnalysis::new();
        let result = self.dataflow_engine.analyze(function, &liveness_analysis);
        
        Ok(result)
    }
    
    /// ライフタイム解析結果をレポートに集約
    fn generate_report(&self) -> LifetimeReport {
        let mut report = LifetimeReport {
            variables: HashMap::new(),
            diagnostics: Vec::new(),
        };
        
        // 変数情報を集約
        if let Some(function) = &self.function {
            for (value_id, value) in &function.values {
                if let Value::Variable { name, ty, .. } = value {
                    let mut var_info = VariableInfo {
                        id: *value_id,
                        name: name.clone(),
                        borrows_in: Vec::new(),
                        borrows_out: Vec::new(),
                    };
                    
                    // この変数からの借用を追加
                    if let Some(borrows) = self.borrows.get(value_id) {
                        for borrow in borrows {
                            var_info.borrows_out.push(BorrowReference {
                                target_id: borrow.borrower_id,
                                kind: borrow.kind,
                                lifetime_id: borrow.lifetime_id.clone(),
                                start_inst: borrow.start_inst,
                                end_inst: borrow.end_inst,
                            });
                        }
                    }
                    
                    // この変数を借用している他の変数からの借用を追加
                    for (source_id, borrows) in &self.borrows {
                        for borrow in borrows {
                            if borrow.borrower_id == *value_id {
                                var_info.borrows_in.push(BorrowReference {
                                    target_id: *source_id,
                                    kind: borrow.kind,
                                    lifetime_id: borrow.lifetime_id.clone(),
                                    start_inst: borrow.start_inst,
                                    end_inst: borrow.end_inst,
                                });
                            }
                        }
                    }
                    
                    report.variables.insert(*value_id, var_info);
                }
            }
        }
        
        report
    }
    
    /// ライフタイム識別子を生成
    fn generate_lifetime_id(&mut self) -> String {
        let id = format!("'lt{}", self.lifetime_counter);
        self.lifetime_counter += 1;
        id
    }
}

/// 借用参照情報
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BorrowReference {
    /// 関連する変数ID
    pub target_id: ValueId,
    /// 借用の種類
    pub kind: BorrowKind,
    /// ライフタイム識別子
    pub lifetime_id: String,
    /// 開始命令
    pub start_inst: InstructionId,
    /// 終了命令（存在する場合）
    pub end_inst: Option<InstructionId>,
}

/// 変数情報
#[derive(Debug, Clone)]
pub struct VariableInfo {
    /// 変数ID
    pub id: ValueId,
    /// 変数名
    pub name: String,
    /// この変数への借用
    pub borrows_in: Vec<BorrowReference>,
    /// この変数からの借用
    pub borrows_out: Vec<BorrowReference>,
}

/// ライフタイム解析レポート
#[derive(Debug, Clone)]
pub struct LifetimeReport {
    /// 各変数の情報
    pub variables: HashMap<ValueId, VariableInfo>,
    /// 解析中に発生した診断情報
    pub diagnostics: Vec<String>,
}

/// ライフタイム付き型情報を生成するためのヘルパー
pub struct LifetimeInferenceHelper {
    /// 型とライフタイムのマッピング
    type_lifetimes: HashMap<TypeId, HashSet<String>>,
    /// 現在の関数で使用可能なライフタイム
    available_lifetimes: HashSet<String>,
}

impl LifetimeInferenceHelper {
    /// 新しいライフタイム推論ヘルパーを作成
    pub fn new() -> Self {
        Self {
            type_lifetimes: HashMap::new(),
            available_lifetimes: HashSet::new(),
        }
    }
    
    /// ライフタイム解析結果に基づいて型情報を更新
    pub fn enhance_types(&mut self, function: &mut Function, report: &LifetimeReport) {
        // 関数内で使用可能なライフタイムを収集
        self.available_lifetimes.clear();
        for var_info in report.variables.values() {
            for borrow in &var_info.borrows_out {
                self.available_lifetimes.insert(borrow.lifetime_id.clone());
            }
        }
        
        // 各変数の型に対してライフタイム情報を付与
        for (value_id, var_info) in &report.variables {
            if let Some(value) = function.values.get_mut(value_id) {
                if let Value::Variable { ref mut ty, .. } = value {
                    // 参照型ならライフタイムを更新
                    if let Some(type_info) = function.types.get_mut(ty) {
                        match type_info {
                            Type::Reference { ref mut lifetime, .. } => {
                                // この変数に関連するライフタイムを見つける
                                if let Some(borrow) = var_info.borrows_in.first() {
                                    *lifetime = Some(borrow.lifetime_id.clone());
                                }
                            },
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // TODO: ここにテストを追加
    
    #[test]
    fn test_lifetime_analysis() {
        // テスト用の関数と解析器を準備
        // TODO: 実装
        
        // テスト用の関数を構築
        let mut module = Module::new("test_module");
        
        // テスト用の型を定義
        let i32_type_id = module.add_type(Type::Integer { bits: 32, signed: true });
        let ref_i32_type_id = module.add_type(Type::Reference { 
            element_type: i32_type_id, 
            mutable: false,
            lifetime: None
        });
        let ref_mut_i32_type_id = module.add_type(Type::Reference { 
            element_type: i32_type_id, 
            mutable: true,
            lifetime: None
        });
        
        // テスト用の関数を構築
        let mut function = Function::new("test_function".to_string(), Vec::new(), None);
        
        // 基本ブロックを作成
        let entry_block_id = function.add_basic_block(BasicBlock::new("entry".to_string()));
        
        // 変数定義
        let var_a_id = module.add_value(Value::Variable { 
            name: "a".to_string(), 
            type_id: i32_type_id 
        });
        
        let var_b_id = module.add_value(Value::Variable { 
            name: "b".to_string(), 
            type_id: i32_type_id 
        });
        
        let ref_a_id = module.add_value(Value::Variable { 
            name: "ref_a".to_string(), 
            type_id: ref_i32_type_id 
        });
        
        let ref_mut_b_id = module.add_value(Value::Variable { 
            name: "ref_mut_b".to_string(), 
            type_id: ref_mut_i32_type_id 
        });
        
        // 命令を追加
        
        // a = 10
        let instr1_id = function.add_instruction(Instruction::new(
            InstructionKind::Store {
                address: ValueId::Variable(var_a_id),
                value: ValueId::Constant(module.add_value(Value::ConstInt(10))),
                type_id: i32_type_id
            },
            None
        ));
        
        // b = 20
        let instr2_id = function.add_instruction(Instruction::new(
            InstructionKind::Store {
                address: ValueId::Variable(var_b_id),
                value: ValueId::Constant(module.add_value(Value::ConstInt(20))),
                type_id: i32_type_id
            },
            None
        ));
        
        // ref_a = &a
        let instr3_id = function.add_instruction(Instruction::new(
            InstructionKind::Borrow {
                source: ValueId::Variable(var_a_id),
                kind: BorrowKind::Shared
            },
            Some(ref_a_id)
        ));
        
        // ref_mut_b = &mut b
        let instr4_id = function.add_instruction(Instruction::new(
            InstructionKind::Borrow {
                source: ValueId::Variable(var_b_id),
                kind: BorrowKind::Mutable
            },
            Some(ref_mut_b_id)
        ));
        
        // *ref_mut_b = *ref_a + 5
        let load_ref_a = module.add_value(Value::Temporary { type_id: i32_type_id });
        let instr5_id = function.add_instruction(Instruction::new(
            InstructionKind::Load {
                address: ValueId::Variable(ref_a_id),
                type_id: i32_type_id
            },
            Some(load_ref_a)
        ));
        
        let add_result = module.add_value(Value::Temporary { type_id: i32_type_id });
        let instr6_id = function.add_instruction(Instruction::new(
            InstructionKind::BinaryOp {
                op: BinaryOp::Add,
                lhs: ValueId::Variable(load_ref_a),
                rhs: ValueId::Constant(module.add_value(Value::ConstInt(5)))
            },
            Some(add_result)
        ));
        
        let instr7_id = function.add_instruction(Instruction::new(
            InstructionKind::Store {
                address: ValueId::Variable(ref_mut_b_id),
                value: ValueId::Variable(add_result),
                type_id: i32_type_id
            },
            None
        ));
        
        // return
        let instr8_id = function.add_instruction(Instruction::new(
            InstructionKind::Return { value: None },
            None
        ));
        
        // 基本ブロックに命令を追加
        function.basic_blocks.get_mut(&entry_block_id).unwrap().instructions.push(instr1_id);
        function.basic_blocks.get_mut(&entry_block_id).unwrap().instructions.push(instr2_id);
        function.basic_blocks.get_mut(&entry_block_id).unwrap().instructions.push(instr3_id);
        function.basic_blocks.get_mut(&entry_block_id).unwrap().instructions.push(instr4_id);
        function.basic_blocks.get_mut(&entry_block_id).unwrap().instructions.push(instr5_id);
        function.basic_blocks.get_mut(&entry_block_id).unwrap().instructions.push(instr6_id);
        function.basic_blocks.get_mut(&entry_block_id).unwrap().instructions.push(instr7_id);
        function.basic_blocks.get_mut(&entry_block_id).unwrap().instructions.push(instr8_id);
        
        // モジュールに関数を追加
        module.add_function(function);
        
        // ライフタイム解析を実行
        let mut lifetime_analyzer = LifetimeAnalyzer::new();
        lifetime_analyzer.analyze_module(&module).expect("ライフタイム解析に失敗");
        
        // 解析結果を取得
        let result = lifetime_analyzer.get_result();
        
        // ライフタイムの検証
        // ref_aのライフタイムがinstr3からinstr5まで続くことを確認
        let ref_a_info = result.get_borrow_info(ref_a_id).expect("ref_aの借用情報が見つかりません");
        assert_eq!(ref_a_info.source_id, var_a_id, "ref_aは変数aを参照しているはず");
        assert_eq!(ref_a_info.kind, BorrowKind::Shared, "ref_aは共有借用のはず");
        assert_eq!(ref_a_info.start_inst, instr3_id, "ref_aの借用開始はinstr3のはず");
        assert!(ref_a_info.end_inst.is_some(), "ref_aの借用終了が設定されているはず");
        
        // ref_mut_bのライフタイムがinstr4からinstr7まで続くことを確認
        let ref_mut_b_info = result.get_borrow_info(ref_mut_b_id).expect("ref_mut_bの借用情報が見つかりません");
        assert_eq!(ref_mut_b_info.source_id, var_b_id, "ref_mut_bは変数bを参照しているはず");
        assert_eq!(ref_mut_b_info.kind, BorrowKind::Mutable, "ref_mut_bは可変借用のはず");
        assert_eq!(ref_mut_b_info.start_inst, instr4_id, "ref_mut_bの借用開始はinstr4のはず");
        assert!(ref_mut_b_info.end_inst.is_some(), "ref_mut_bの借用終了が設定されているはず");
        
        // 変数aの借用情報を確認
        let var_a_borrows = result.get_variable_borrows(var_a_id).expect("変数aの借用情報が見つかりません");
        assert_eq!(var_a_borrows.borrows_out.len(), 1, "変数aは1つの借用を持つはず");
        assert_eq!(var_a_borrows.borrows_out[0].borrower_id, ref_a_id, "変数aはref_aに借用されているはず");
        
        // 変数bの借用情報を確認
        let var_b_borrows = result.get_variable_borrows(var_b_id).expect("変数bの借用情報が見つかりません");
        assert_eq!(var_b_borrows.borrows_out.len(), 1, "変数bは1つの借用を持つはず");
        assert_eq!(var_b_borrows.borrows_out[0].borrower_id, ref_mut_b_id, "変数bはref_mut_bに借用されているはず");
        
        // 借用の有効性チェック（エラーがないことを確認）
        assert!(result.validate_lifetimes().is_ok(), "ライフタイム検証はエラーを返さないはず");
    }
    
    #[test]
    fn test_lifetime_violation() {
        // 借用規則に違反するコードのテスト
        let mut module = Module::new("test_violation");
        
        // テスト用の型を定義
        let i32_type_id = module.add_type(Type::Integer { bits: 32, signed: true });
        let ref_i32_type_id = module.add_type(Type::Reference { 
            element_type: i32_type_id, 
            mutable: false,
            lifetime: None
        });
        let ref_mut_i32_type_id = module.add_type(Type::Reference { 
            element_type: i32_type_id, 
            mutable: true,
            lifetime: None
        });
        
        // テスト用の関数を構築
        let mut function = Function::new("test_violation".to_string(), Vec::new(), None);
        
        // 基本ブロックを作成
        let entry_block_id = function.add_basic_block(BasicBlock::new("entry".to_string()));
        
        // 変数定義
        let var_a_id = module.add_value(Value::Variable { 
            name: "a".to_string(), 
            type_id: i32_type_id 
        });
        
        let ref_a_id = module.add_value(Value::Variable { 
            name: "ref_a".to_string(), 
            type_id: ref_i32_type_id 
        });
        
        let ref_mut_a_id = module.add_value(Value::Variable { 
            name: "ref_mut_a".to_string(), 
            type_id: ref_mut_i32_type_id 
        });
        
        // 命令を追加
        
        // a = 10
        let instr1_id = function.add_instruction(Instruction::new(
            InstructionKind::Store {
                address: ValueId::Variable(var_a_id),
                value: ValueId::Constant(module.add_value(Value::ConstInt(10))),
                type_id: i32_type_id
            },
            None
        ));
        
        // ref_a = &a (共有借用)
        let instr2_id = function.add_instruction(Instruction::new(
            InstructionKind::Borrow {
                source: ValueId::Variable(var_a_id),
                kind: BorrowKind::Shared
            },
            Some(ref_a_id)
        ));
        
        // ref_mut_a = &mut a (可変借用) - 既に共有借用があるので違反
        let instr3_id = function.add_instruction(Instruction::new(
            InstructionKind::Borrow {
                source: ValueId::Variable(var_a_id),
                kind: BorrowKind::Mutable
            },
            Some(ref_mut_a_id)
        ));
        
        // return
        let instr4_id = function.add_instruction(Instruction::new(
            InstructionKind::Return { value: None },
            None
        ));
        
        // 基本ブロックに命令を追加
        function.basic_blocks.get_mut(&entry_block_id).unwrap().instructions.push(instr1_id);
        function.basic_blocks.get_mut(&entry_block_id).unwrap().instructions.push(instr2_id);
        function.basic_blocks.get_mut(&entry_block_id).unwrap().instructions.push(instr3_id);
        function.basic_blocks.get_mut(&entry_block_id).unwrap().instructions.push(instr4_id);
        
        // モジュールに関数を追加
        module.add_function(function);
        
        // ライフタイム解析を実行
        let mut lifetime_analyzer = LifetimeAnalyzer::new();
        lifetime_analyzer.analyze_module(&module).expect("ライフタイム解析そのものは成功するはず");
        
        // 解析結果を取得
        let result = lifetime_analyzer.get_result();
        
        // ライフタイムの検証 - 借用規則違反があるはず
        assert!(result.validate_lifetimes().is_err(), "借用規則違反があるためエラーを返すはず");
    }
}

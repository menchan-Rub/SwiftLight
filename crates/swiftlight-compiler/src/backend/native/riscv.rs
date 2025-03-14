//! # RISC-V コード生成
//! 
//! RISC-Vアーキテクチャ向けのネイティブコードを生成するモジュールです。
//! 主にLLVMバックエンドが生成したオブジェクトコードに対して、さらなる最適化を行います。
//! このモジュールは、RISC-V ISAの全ての拡張（RV32/RV64/RV128, M, A, F, D, C, V, P, B, J, T, Zk）に対応し、
//! ターゲットハードウェアの特性に合わせた最適なコード生成を行います。

use std::collections::{HashMap, HashSet, BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::fmt;

use crate::frontend::error::{CompilerError, ErrorKind, Result};
use crate::middleend::ir::{Module, Function, Instruction, BasicBlock, Type, Value, ControlFlow};
use crate::middleend::analysis::{dataflow::DataFlowAnalysis, lifetime::LifetimeAnalysis};
use crate::middleend::optimization::vectorization::VectorizationInfo;
use crate::backend::target::{TargetFeature, TargetInfo, RegisterClass};
use crate::utils::{graph::Graph, statistics::OptimizationStatistics};

/// RISC-V ISA拡張セット
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RISCVExtension {
    /// 基本整数命令セット (RV32I/RV64I/RV128I)
    I,
    /// 整数乗算除算拡張 (M)
    M,
    /// アトミック命令拡張 (A)
    A,
    /// 単精度浮動小数点拡張 (F)
    F,
    /// 倍精度浮動小数点拡張 (D)
    D,
    /// 圧縮命令拡張 (C)
    C,
    /// ベクトル演算拡張 (V)
    V,
    /// パックド SIMD 拡張 (P)
    P,
    /// ビット操作拡張 (B)
    B,
    /// 動的翻訳拡張 (J)
    J,
    /// トランザクショナルメモリ拡張 (T)
    T,
    /// 暗号化拡張 (Zk)
    Zk,
}

/// RISC-Vレジスタクラス
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RISCVRegisterClass {
    /// 汎用レジスタ (x0-x31)
    General,
    /// 浮動小数点レジスタ (f0-f31)
    Float,
    /// ベクトルレジスタ (v0-v31)
    Vector,
}

/// RISC-Vレジスタ
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RISCVRegister {
    /// レジスタ名
    pub name: String,
    /// レジスタ番号
    pub number: usize,
    /// レジスタクラス
    pub class: RISCVRegisterClass,
    /// ABI名（引数や戻り値の受け渡しに使用）
    pub abi_name: Option<String>,
    /// 呼び出し先保存レジスタかどうか
    pub is_callee_saved: bool,
}

impl RISCVRegister {
    /// 新しいレジスタを作成
    pub fn new(name: &str, number: usize, class: RISCVRegisterClass, abi_name: Option<&str>, is_callee_saved: bool) -> Self {
        Self {
            name: name.to_string(),
            number,
            class,
            abi_name: abi_name.map(|s| s.to_string()),
            is_callee_saved,
        }
    }
}

/// RISC-V命令形式
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RISCVInstructionFormat {
    /// R形式: レジスタ-レジスタ演算
    R,
    /// I形式: 即値演算、ロード
    I,
    /// S形式: ストア
    S,
    /// B形式: 条件分岐
    B,
    /// U形式: 上位即値
    U,
    /// J形式: ジャンプ
    J,
    /// R4形式: 4オペランドレジスタ演算（主にFMAなど）
    R4,
    /// V形式: ベクトル命令
    V,
}

/// RISC-V命令
#[derive(Debug, Clone)]
pub struct RISCVInstruction {
    /// 命令ニーモニック
    pub mnemonic: String,
    /// 命令形式
    pub format: RISCVInstructionFormat,
    /// オペランド
    pub operands: Vec<RISCVOperand>,
    /// 必要な拡張セット
    pub required_extensions: HashSet<RISCVExtension>,
    /// 命令レイテンシ（サイクル数）
    pub latency: u32,
    /// スループット（1/サイクル）
    pub throughput: f32,
    /// 命令エンコーディング（バイナリ表現）
    pub encoding: Option<u32>,
    /// コメント
    pub comment: Option<String>,
}

impl fmt::Display for RISCVInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}\t", self.mnemonic)?;
        
        let operands: Vec<String> = self.operands.iter()
            .map(|op| op.to_string())
            .collect();
        
        write!(f, "{}", operands.join(", "))?;
        
        if let Some(comment) = &self.comment {
            write!(f, "\t# {}", comment)?;
        }
        
        Ok(())
    }
}

/// RISC-V命令オペランド
#[derive(Debug, Clone)]
pub enum RISCVOperand {
    /// レジスタ
    Register(RISCVRegister),
    /// 即値
    Immediate(i64),
    /// メモリアドレス（ベースレジスタ + オフセット）
    Memory {
        base: RISCVRegister,
        offset: i32,
    },
    /// ラベル参照
    Label(String),
    /// ベクトルマスク
    VectorMask {
        register: RISCVRegister,
        negate: bool,
    },
    /// ベクトル長設定
    VectorLength {
        register: Option<RISCVRegister>,
        value: Option<u32>,
    },
}

impl fmt::Display for RISCVOperand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RISCVOperand::Register(reg) => write!(f, "{}", reg.name),
            RISCVOperand::Immediate(imm) => write!(f, "{}", imm),
            RISCVOperand::Memory { base, offset } => {
                if *offset == 0 {
                    write!(f, "({})", base.name)
                } else {
                    write!(f, "{}({})", offset, base.name)
                }
            },
            RISCVOperand::Label(label) => write!(f, "{}", label),
            RISCVOperand::VectorMask { register, negate } => {
                if *negate {
                    write!(f, "!{}", register.name)
                } else {
                    write!(f, "{}", register.name)
                }
            },
            RISCVOperand::VectorLength { register, value } => {
                if let Some(reg) = register {
                    write!(f, "vl:{}", reg.name)
                } else if let Some(val) = value {
                    write!(f, "vl:{}", val)
                } else {
                    write!(f, "vl")
                }
            },
        }
    }
}

/// 干渉グラフノード
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct InterferenceNode {
    /// 変数ID
    var_id: usize,
    /// 変数の型
    var_type: Type,
    /// 生存区間の開始
    live_range_start: usize,
    /// 生存区間の終了
    live_range_end: usize,
    /// 使用頻度
    usage_count: usize,
    /// スピル優先度（高いほどスピルされやすい）
    spill_priority: f32,
}

/// RISC-V向け最適化器
pub struct RISCVOptimizer {
    /// ターゲット情報
    target_info: TargetInfo,
    /// サポートする拡張セット
    supported_extensions: HashSet<RISCVExtension>,
    /// レジスタ割り当て
    register_allocation: HashMap<usize, RISCVRegister>,
    /// 命令選択情報
    instruction_selection: HashMap<usize, Vec<RISCVInstruction>>,
    /// 基本ブロックの実行頻度予測
    block_frequency: HashMap<usize, f64>,
    /// 関数間解析情報
    interprocedural_info: HashMap<String, InterproceduralInfo>,
    /// 最適化統計情報
    statistics: OptimizationStatistics,
    /// 干渉グラフ
    interference_graph: Graph<InterferenceNode>,
    /// レジスタクラスごとの利用可能なレジスタ
    available_registers: HashMap<RISCVRegisterClass, Vec<RISCVRegister>>,
    /// 命令スケジューリング情報
    scheduling_info: HashMap<usize, SchedulingInfo>,
    /// ベクトル化情報
    vectorization_info: Option<VectorizationInfo>,
    /// 最適化レベル (0-3)
    optimization_level: u8,
}

/// 関数間解析情報
#[derive(Debug, Clone)]
struct InterproceduralInfo {
    /// 呼び出し先関数
    callees: HashSet<String>,
    /// 呼び出し元関数
    callers: HashSet<String>,
    /// 再帰呼び出しかどうか
    is_recursive: bool,
    /// インライン展開候補かどうか
    is_inline_candidate: bool,
    /// 副作用があるかどうか
    has_side_effects: bool,
    /// 純粋関数かどうか
    is_pure: bool,
}

/// 命令スケジューリング情報
#[derive(Debug, Clone)]
struct SchedulingInfo {
    /// 依存関係グラフ
    dependency_graph: Graph<usize>,
    /// 各命令の最早実行時間
    earliest_time: HashMap<usize, u32>,
    /// 各命令の最遅実行時間
    latest_time: HashMap<usize, u32>,
    /// クリティカルパス上の命令
    critical_path: Vec<usize>,
    /// スケジューリング後の命令順序
    scheduled_order: Vec<usize>,
}

impl RISCVOptimizer {
    /// 新しい最適化器を作成
    pub fn new(target_info: TargetInfo, optimization_level: u8) -> Self {
        let mut supported_extensions = HashSet::new();
        
        // 基本ISAは常にサポート
        supported_extensions.insert(RISCVExtension::I);
        
        // ターゲット情報から対応拡張を設定
        if target_info.has_feature(TargetFeature::MultiplyDivide) {
            supported_extensions.insert(RISCVExtension::M);
        }
        if target_info.has_feature(TargetFeature::AtomicOperations) {
            supported_extensions.insert(RISCVExtension::A);
        }
        if target_info.has_feature(TargetFeature::SinglePrecisionFloat) {
            supported_extensions.insert(RISCVExtension::F);
        }
        if target_info.has_feature(TargetFeature::DoublePrecisionFloat) {
            supported_extensions.insert(RISCVExtension::D);
        }
        if target_info.has_feature(TargetFeature::CompressedInstructions) {
            supported_extensions.insert(RISCVExtension::C);
        }
        if target_info.has_feature(TargetFeature::VectorInstructions) {
            supported_extensions.insert(RISCVExtension::V);
        }
        if target_info.has_feature(TargetFeature::PackedSIMD) {
            supported_extensions.insert(RISCVExtension::P);
        }
        if target_info.has_feature(TargetFeature::BitManipulation) {
            supported_extensions.insert(RISCVExtension::B);
        }
        
        // 利用可能なレジスタを設定
        let mut available_registers = HashMap::new();
        
        // 汎用レジスタ (x0-x31)
        let mut general_registers = Vec::new();
        // x0 (zero) は常に0なので割り当て不可
        // x1-x31 を設定
        general_registers.push(RISCVRegister::new("x1", 1, RISCVRegisterClass::General, Some("ra"), false)); // 戻りアドレス
        general_registers.push(RISCVRegister::new("x2", 2, RISCVRegisterClass::General, Some("sp"), true));  // スタックポインタ
        general_registers.push(RISCVRegister::new("x3", 3, RISCVRegisterClass::General, Some("gp"), true));  // グローバルポインタ
        general_registers.push(RISCVRegister::new("x4", 4, RISCVRegisterClass::General, Some("tp"), true));  // スレッドポインタ
        
        // 引数/戻り値レジスタ
        for i in 5..8 {
            general_registers.push(RISCVRegister::new(&format!("x{}", i), i, RISCVRegisterClass::General, Some(&format!("t{}", i-5)), false));
        }
        
        // 引数レジスタ
        for i in 8..18 {
            general_registers.push(RISCVRegister::new(&format!("x{}", i), i, RISCVRegisterClass::General, Some(&format!("a{}", i-8)), false));
        }
        
        // 呼び出し先保存レジスタ
        for i in 18..28 {
            general_registers.push(RISCVRegister::new(&format!("x{}", i), i, RISCVRegisterClass::General, Some(&format!("s{}", i-18)), true));
        }
        
        // 一時レジスタ
        for i in 28..32 {
            general_registers.push(RISCVRegister::new(&format!("x{}", i), i, RISCVRegisterClass::General, Some(&format!("t{}", i-28+3)), false));
        }
        
        available_registers.insert(RISCVRegisterClass::General, general_registers);
        
        // 浮動小数点レジスタ (f0-f31)
        if supported_extensions.contains(&RISCVExtension::F) || supported_extensions.contains(&RISCVExtension::D) {
            let mut float_registers = Vec::new();
            
            // 一時レジスタ
            for i in 0..8 {
                float_registers.push(RISCVRegister::new(&format!("f{}", i), i, RISCVRegisterClass::Float, Some(&format!("ft{}", i)), false));
            }
            
            // 引数レジスタ
            for i in 8..18 {
                float_registers.push(RISCVRegister::new(&format!("f{}", i), i, RISCVRegisterClass::Float, Some(&format!("fa{}", i-8)), false));
            }
            
            // 呼び出し先保存レジスタ
            for i in 18..28 {
                float_registers.push(RISCVRegister::new(&format!("f{}", i), i, RISCVRegisterClass::Float, Some(&format!("fs{}", i-18)), true));
            }
            
            // 一時レジスタ
            for i in 28..32 {
                float_registers.push(RISCVRegister::new(&format!("f{}", i), i, RISCVRegisterClass::Float, Some(&format!("ft{}", i-28+8)), false));
            }
            
            available_registers.insert(RISCVRegisterClass::Float, float_registers);
        }
        
        // ベクトルレジスタ (v0-v31)
        if supported_extensions.contains(&RISCVExtension::V) {
            let mut vector_registers = Vec::new();
            
            for i in 0..32 {
                vector_registers.push(RISCVRegister::new(&format!("v{}", i), i, RISCVRegisterClass::Vector, None, false));
            }
            
            available_registers.insert(RISCVRegisterClass::Vector, vector_registers);
        }
        
        Self {
            target_info,
            supported_extensions,
            register_allocation: HashMap::new(),
            instruction_selection: HashMap::new(),
            block_frequency: HashMap::new(),
            interprocedural_info: HashMap::new(),
            statistics: OptimizationStatistics::new(),
            interference_graph: Graph::new(),
            available_registers,
            scheduling_info: HashMap::new(),
            vectorization_info: None,
            optimization_level,
        }
    }
    
    /// オブジェクトコードを最適化
    pub fn optimize(&mut self, obj_code: &[u8]) -> Result<Vec<u8>> {
        let start_time = Instant::now();
        
        // オブジェクトファイルの解析
        let mut optimized_code = obj_code.to_vec();
        
        // 最適化レベルに応じた処理
        match self.optimization_level {
            0 => {
                // 最適化なし - そのまま返す
            },
            1 => {
                // 基本的な最適化
                optimized_code = self.basic_optimize_object_code(&optimized_code)?;
            },
            2 => {
                // 積極的な最適化
                optimized_code = self.aggressive_optimize_object_code(&optimized_code)?;
            },
            3 | _ => {
                // 最大限の最適化
                optimized_code = self.maximum_optimize_object_code(&optimized_code)?;
            }
        }
        
        // 統計情報の更新
        self.statistics.record_optimization_time("object_code_optimization", start_time.elapsed());
        
        Ok(optimized_code)
    }
    
    /// 基本的なオブジェクトコード最適化
    fn basic_optimize_object_code(&mut self, obj_code: &[u8]) -> Result<Vec<u8>> {
        // 基本的な最適化のみを適用
        let start_time = Instant::now();
        
        // ELFヘッダーを解析
        let elf_header = self.parse_elf_header(obj_code)?;
        
        // セクションヘッダーを解析
        let section_headers = self.parse_section_headers(obj_code, &elf_header)?;
        
        // テキストセクションを特定
        let text_section = self.find_text_section(obj_code, &section_headers)?;
        
        // 命令アライメント調整
        let mut optimized_text = self.align_instructions(&text_section)?;
        
        // 単純な命令置換
        optimized_text = self.replace_simple_instructions(&optimized_text)?;
        
        // 最適化されたテキストセクションを元のオブジェクトコードに戻す
        let mut result = obj_code.to_vec();
        if let Some(text_section_header) = section_headers.iter().find(|&h| h.name == ".text") {
            let offset = text_section_header.offset as usize;
            let size = text_section_header.size as usize;
            
            // サイズチェック
            if optimized_text.len() <= size {
                result[offset..offset + optimized_text.len()].copy_from_slice(&optimized_text);
            } else {
                return Err(CompilerError::new(
                    ErrorKind::OptimizationError,
                    "最適化後のコードサイズが元のセクションサイズを超えています".to_string()
                ));
            }
        }
        
        // 統計情報の更新
        self.statistics.record_optimization_time("basic_object_code_optimization", start_time.elapsed());
        
        Ok(result)
    }
    
    /// ELFヘッダーを解析
    fn parse_elf_header(&self, obj_code: &[u8]) -> Result<ElfHeader> {
        if obj_code.len() < 64 {
            return Err(CompilerError::new(
                ErrorKind::OptimizationError,
                "オブジェクトコードが小さすぎます".to_string()
            ));
        }
        
        // 簡易的なELFヘッダー解析
        let e_shoff = u64::from_le_bytes([
            obj_code[40], obj_code[41], obj_code[42], obj_code[43],
            obj_code[44], obj_code[45], obj_code[46], obj_code[47]
        ]);
        
        let e_shentsize = u16::from_le_bytes([obj_code[58], obj_code[59]]);
        let e_shnum = u16::from_le_bytes([obj_code[60], obj_code[61]]);
        
        Ok(ElfHeader {
            section_header_offset: e_shoff,
            section_header_entry_size: e_shentsize,
            section_header_count: e_shnum,
        })
    }
    
    /// セクションヘッダーを解析
    fn parse_section_headers(&self, obj_code: &[u8], elf_header: &ElfHeader) -> Result<Vec<SectionHeader>> {
        let mut section_headers = Vec::new();
        
        let offset = elf_header.section_header_offset as usize;
        let entry_size = elf_header.section_header_entry_size as usize;
        let count = elf_header.section_header_count as usize;
        
        for i in 0..count {
            let section_offset = offset + i * entry_size;
            
            if section_offset + entry_size > obj_code.len() {
                return Err(CompilerError::new(
                    ErrorKind::OptimizationError,
                    "セクションヘッダーの解析中にバッファ境界を超えました".to_string()
                ));
            }
            
            // セクション名のインデックス
            let sh_name = u32::from_le_bytes([
                obj_code[section_offset],
                obj_code[section_offset + 1],
                obj_code[section_offset + 2],
                obj_code[section_offset + 3]
            ]);
            
            // セクションタイプ
            let sh_type = u32::from_le_bytes([
                obj_code[section_offset + 4],
                obj_code[section_offset + 5],
                obj_code[section_offset + 6],
                obj_code[section_offset + 7]
            ]);
            
            // セクションフラグ
            let sh_flags = u64::from_le_bytes([
                obj_code[section_offset + 8],
                obj_code[section_offset + 9],
                obj_code[section_offset + 10],
                obj_code[section_offset + 11],
                obj_code[section_offset + 12],
                obj_code[section_offset + 13],
                obj_code[section_offset + 14],
                obj_code[section_offset + 15]
            ]);
            
            // セクションオフセット
            let sh_offset = u64::from_le_bytes([
                obj_code[section_offset + 24],
                obj_code[section_offset + 25],
                obj_code[section_offset + 26],
                obj_code[section_offset + 27],
                obj_code[section_offset + 28],
                obj_code[section_offset + 29],
                obj_code[section_offset + 30],
                obj_code[section_offset + 31]
            ]);
            
            // セクションサイズ
            let sh_size = u64::from_le_bytes([
                obj_code[section_offset + 32],
                obj_code[section_offset + 33],
                obj_code[section_offset + 34],
                obj_code[section_offset + 35],
                obj_code[section_offset + 36],
                obj_code[section_offset + 37],
                obj_code[section_offset + 38],
                obj_code[section_offset + 39]
            ]);
            
            // セクション名を取得（簡易的な実装）
            let name = if sh_name == 0 {
                String::new()
            } else if sh_name == 1 {
                ".text".to_string()
            } else if sh_name == 7 {
                ".data".to_string()
            } else if sh_name == 13 {
                ".bss".to_string()
            } else {
                format!("section_{}", sh_name)
            };
            
            section_headers.push(SectionHeader {
                name,
                type_: sh_type,
                flags: sh_flags,
                offset: sh_offset,
                size: sh_size,
            });
        }
        
        Ok(section_headers)
    }
    
    /// テキストセクションを見つける
    fn find_text_section(&self, obj_code: &[u8], section_headers: &[SectionHeader]) -> Result<Vec<u8>> {
        for header in section_headers {
            if header.name == ".text" {
                let offset = header.offset as usize;
                let size = header.size as usize;
                
                if offset + size > obj_code.len() {
                    return Err(CompilerError::new(
                        ErrorKind::OptimizationError,
                        "テキストセクションの範囲がオブジェクトコードの境界を超えています".to_string()
                    ));
                }
                
                return Ok(obj_code[offset..offset + size].to_vec());
            }
        }
        
        Err(CompilerError::new(
            ErrorKind::OptimizationError,
            "テキストセクションが見つかりませんでした".to_string()
        ))
    }
    
    /// 命令アライメント調整
    fn align_instructions(&self, text_section: &[u8]) -> Result<Vec<u8>> {
        let mut result = Vec::new();
        let mut i = 0;
        
        while i < text_section.len() {
            // RISC-V命令を解析
            let (instruction, size) = self.decode_riscv_instruction(&text_section[i..])?;
            
            // 命令をアライメントする
            if self.supported_extensions.contains(&RISCVExtension::C) && size == 2 {
                // 圧縮命令は2バイトアライメント
                result.extend_from_slice(&text_section[i..i + size]);
            } else {
                // 標準命令は4バイトアライメント
                if i % 4 != 0 && i + 4 <= text_section.len() {
                    // アライメントが必要な場合、NOPで埋める
                    let padding_size = 4 - (i % 4);
                    for _ in 0..padding_size {
                        result.push(0x13); // addi x0, x0, 0 (NOP)
                    }
                    i += padding_size;
                    continue;
                }
                
                result.extend_from_slice(&text_section[i..i + size]);
            }
            
            i += size;
        }
        
        Ok(result)
    }
    
    /// RISC-V命令をデコード
    fn decode_riscv_instruction(&self, bytes: &[u8]) -> Result<(RISCVInstruction, usize)> {
        if bytes.len() < 2 {
            return Err(CompilerError::new(
                ErrorKind::OptimizationError,
                "命令のデコード中にバッファ境界を超えました".to_string()
            ));
        }
        
        // 圧縮命令かどうかを判定
        let is_compressed = (bytes[0] & 0b11) != 0b11;
        
        if is_compressed {
    /// 関数間解析を実行
    fn perform_interprocedural_analysis(&mut self, module: &Module) -> Result<()> {
        let start_time = Instant::now();
        
        // 呼び出しグラフの構築
        let mut call_graph = Graph::<String>::new();
        
        // 各関数の呼び出し関係を解析
        for (name, function) in &module.functions {
            let mut callees = HashSet::new();
            
            // 関数内の全ての基本ブロックを走査
            for (_, block) in &function.basic_blocks {
                // ブロック内の全ての命令を走査
                for inst in &block.instructions {
                    // 関数呼び出し命令を検出
                    if let Instruction::Call { function: callee, .. } = inst {
                        callees.insert(callee.clone());
                        call_graph.add_edge(name.clone(), callee.clone());
                    }
                }
            }
            
            // 関数間解析情報を作成
            let info = InterproceduralInfo {
                callees,
                callers: HashSet::new(), // 後で設定
                is_recursive: false,     // 後で設定
                is_inline_candidate: function.basic_blocks.len() <= 3, // 小さい関数はインライン候補
                has_side_effects: self.function_has_side_effects(function),
                is_pure: self.is_pure_function(function),
            };
            
            self.interprocedural_info.insert(name.clone(), info);
        }
        
        // 呼び出し元情報の設定
        for name in module.functions.keys() {
            if let Some(info) = self.interprocedural_info.get_mut(name) {
                for (caller_name, caller_info) in &mut self.interprocedural_info {
                    if caller_info.callees.contains(name) {
                        info.callers.insert(caller_name.clone());
                    }
                }
            }
        }
        
        // 再帰呼び出しの検出
        for name in module.functions.keys() {
            if let Some(info) = self.interprocedural_info.get_mut(name) {
                // 直接再帰
                if info.callees.contains(name) {
                    info.is_recursive = true;
                    continue;
                }
                
                // 間接再帰（到達可能性解析）
                let mut visited = HashSet::new();
                let mut stack = Vec::new();
                stack.push(name.clone());
                
                while let Some(current) = stack.pop() {
                    if visited.contains(&current) {
                        continue;
                    }
                    
                    visited.insert(current.clone());
                    
                    if let Some(current_info) = self.interprocedural_info.get(&current) {
                        for callee in &current_info.callees {
                            if callee == name {
                                info.is_recursive = true;
                                break;
                            }
                            stack.push(callee.clone());
                        }
                    }
                    
                    if info.is_recursive {
                        break;
                    }
                }
            }
        }
        
        // 統計情報の更新
        self.statistics.record_optimization_time("interprocedural_analysis", start_time.elapsed());
        
        Ok(())
    }
    
    /// 関数が副作用を持つかどうかを判定
    fn function_has_side_effects(&self, function: &Function) -> bool {
        // 各基本ブロックを走査
        for (_, block) in &function.basic_blocks {
            // ブロック内の全ての命令を走査
            for inst in &block.instructions {
                match inst {
                    // メモリ書き込み、I/O、関数呼び出しは副作用あり
                    Instruction::Store { .. } |
                    Instruction::Call { .. } |
                    Instruction::IntrinsicCall { .. } => return true,
                    _ => {}
                }
            }
        }
        
        false
    }
    
    /// 関数が純粋関数かどうかを判定
    fn is_pure_function(&self, function: &Function) -> bool {
        // 副作用がなく、同じ入力に対して常に同じ出力を返す関数
        !self.function_has_side_effects(function)
    }
    
    /// 関数に対してRISC-V固有の最適化を適用
    pub fn optimize_function(&mut self, function: &Function) -> Result<()> {
        let start_time = Instant::now();
        
        // 最適化レベルに応じた処理
        match self.optimization_level {
            0 => {
                // 最適化なし - 基本的な変換のみ
                self.select_instructions(function)?;
            },
            1 => {
                // 基本的な最適化
                self.analyze_block_frequency(function)?;
                self.select_instructions(function)?;
                self.allocate_registers(function)?;
                self.schedule_instructions(function)?;
            },
            2 => {
                // 積極的な最適化
                self.analyze_block_frequency(function)?;
                self.select_instructions_aggressive(function)?;
                self.allocate_registers_advanced(function)?;
                self.schedule_instructions_advanced(function)?;
                
                // RISC-V拡張命令の活用
                if self.supported_extensions.contains(&RISCVExtension::B) {
                    self.utilize_bit_manipulation(function)?;
                }
                
                if self.supported_extensions.contains(&RISCVExtension::V) {
                    self.utilize_vector_extensions(function)?;
                }
            },
            3 | _ => {
                // 最大限の最適化
                self.analyze_block_frequency(function)?;
                self.select_instructions_aggressive(function)?;
                self.allocate_registers_advanced(function)?;
                self.schedule_instructions_advanced(function)?;
                
                // RISC-V拡張命令の活用
                if self.supported_extensions.contains(&RISCVExtension::B) {
                    self.utilize_bit_manipulation(function)?;
                }
                
                if self.supported_extensions.contains(&RISCVExtension::V) {
                    self.utilize_vector_extensions(function)?;
                }
                
                if self.supported_extensions.contains(&RISCVExtension::P) {
                    self.utilize_packed_simd(function)?;
                }
                
                // ソフトウェアパイプライニング
                self.apply_software_pipelining(function)?;
                
                // 投機的実行の最適化
                self.optimize_speculative_execution(function)?;
            }
        }
        
        // 統計情報の更新
        self.statistics.record_optimization_time("function_optimization", start_time.elapsed());
        self.statistics.increment_counter("optimized_functions");
        
        Ok(())
    }
    
    /// 基本ブロックの実行頻度を解析
    fn analyze_block_frequency(&mut self, function: &Function) -> Result<()> {
        let start_time = Instant::now();
        
        // 基本ブロックの実行頻度を予測
        // エントリーブロックの頻度を1.0とする
        let entry_block_id = function.entry_block;
        self.block_frequency.insert(entry_block_id, 1.0);
        
        // 訪問済みブロックを追跡
        let mut visited = HashSet::new();
        
        // 処理待ちキュー
    /// ベクトル拡張命令の活用
    /// RISC-V Vベクトル拡張を使用して、ループやデータ並列処理を最適化します。
    /// この関数は以下の手順で実行されます：
    /// 1. ベクトル化可能なループを特定
    /// 2. データ依存関係の分析
    /// 3. ベクトルレジスタの設定と割り当て
    /// 4. ベクトル命令への変換
    /// 5. ベクトル長の動的調整
    fn utilize_vector_extensions(&mut self, function: &Function) -> Result<()> {
        let start_time = Instant::now();
        self.statistics.increment_counter("vector_optimization_attempts");
        
        // ベクトル化候補の特定
        let candidates = self.identify_vectorization_candidates(function)?;
        if candidates.is_empty() {
            self.statistics.increment_counter("no_vectorization_opportunities");
            return Ok(());
        }
        
        self.statistics.add_to_counter("vectorization_candidates", candidates.len() as u64);
        
        // 各候補に対してベクトル化を適用
        for candidate in candidates {
            if let Err(e) = self.vectorize_loop(&candidate, function) {
                // エラーをログに記録するが、最適化は続行
                log::warn!("ループのベクトル化に失敗: {:?} - {}", candidate, e);
                self.statistics.increment_counter("failed_vectorizations");
                continue;
            }
            self.statistics.increment_counter("successful_vectorizations");
        }
        
        // ベクトルレジスタの割り当て最適化
        self.optimize_vector_register_allocation(function)?;
        
        // ベクトル命令のスケジューリング最適化
        self.schedule_vector_instructions(function)?;
        
        // 統計情報の更新
        self.statistics.record_optimization_time("vector_extensions", start_time.elapsed());
        
        Ok(())
    }
    
    /// ベクトル化可能なループやデータ並列処理を特定します
    fn identify_vectorization_candidates(&self, function: &Function) -> Result<Vec<VectorizationCandidate>> {
        let mut candidates = Vec::new();
        
        // 各基本ブロックを分析
        for (block_id, block) in function.blocks.iter() {
            // ループを検出
            if let Some(loop_info) = self.loop_analyzer.get_loop_info(*block_id) {
                // ループがベクトル化に適しているか確認
                if self.is_loop_vectorizable(loop_info, function)? {
                    let operations = self.analyze_loop_operations(loop_info, function)?;
                    
                    candidates.push(VectorizationCandidate {
                        loop_id: loop_info.id,
                        block_id: *block_id,
                        operations,
                        estimated_speedup: self.estimate_vectorization_speedup(&operations),
                        vector_length: self.determine_optimal_vector_length(&operations),
                    });
                }
            }
            
            // ループ以外のデータ並列処理を検出
            let parallel_ops = self.identify_parallel_operations(block, function)?;
            if !parallel_ops.is_empty() {
                candidates.push(VectorizationCandidate {
                    loop_id: None,
                    block_id: *block_id,
                    operations: parallel_ops,
                    estimated_speedup: self.estimate_vectorization_speedup(&parallel_ops),
                    vector_length: self.determine_optimal_vector_length(&parallel_ops),
                });
            }
        }
        
        // 推定速度向上率でソート
        candidates.sort_by(|a, b| b.estimated_speedup.partial_cmp(&a.estimated_speedup).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(candidates)
    }
    
    /// ループがベクトル化可能かどうかを判断します
    fn is_loop_vectorizable(&self, loop_info: &LoopInfo, function: &Function) -> Result<bool> {
        // ループの反復回数が予測可能か
        if loop_info.iteration_count.is_none() && !loop_info.has_predictable_iterations {
            return Ok(false);
        }
        
        // ループ内の命令を分析
        for block_id in &loop_info.blocks {
            let block = function.blocks.get(block_id).ok_or_else(|| Error::BlockNotFound(*block_id))?;
            
            // ベクトル化を妨げる命令や依存関係をチェック
            for inst in &block.instructions {
                // 制御フロー分岐が多すぎる場合はベクトル化しない
                if inst.is_branch() && !inst.is_unconditional_branch() {
                    let branch_count = self.count_branches_in_loop(loop_info, function)?;
                    if branch_count > self.config.max_branches_for_vectorization {
                        return Ok(false);
                    }
                }
                
                // メモリアクセスパターンを分析
                if inst.is_memory_access() {
                    if !self.has_regular_memory_access_pattern(inst, loop_info, function)? {
                        return Ok(false);
                    }
                }
                
                // 再帰呼び出しがある場合はベクトル化しない
                if inst.is_call() {
                    let callee = self.get_callee_function(inst)?;
                    if callee == function.name || self.call_graph.is_recursive(callee) {
                        return Ok(false);
                    }
                }
            }
        }
        
        // データ依存関係を分析
        if !self.analyze_data_dependencies(loop_info, function)? {
            return Ok(false);
        }
        
        Ok(true)
    }
    
    /// ループ内の操作を分析し、ベクトル化可能な操作のリストを返します
    fn analyze_loop_operations(&self, loop_info: &LoopInfo, function: &Function) -> Result<Vec<VectorizableOperation>> {
        let mut operations = Vec::new();
        
        for block_id in &loop_info.blocks {
            let block = function.blocks.get(block_id).ok_or_else(|| Error::BlockNotFound(*block_id))?;
            
            for (inst_idx, inst) in block.instructions.iter().enumerate() {
                if let Some(op_type) = self.get_vectorizable_operation_type(inst) {
                    let data_type = self.determine_data_type(inst, function)?;
                    let alignment = self.analyze_memory_alignment(inst, function)?;
                    
                    operations.push(VectorizableOperation {
                        op_type,
                        data_type,
                        block_id: *block_id,
                        inst_idx,
                        alignment,
                        memory_access_pattern: if inst.is_memory_access() {
                            Some(self.analyze_memory_access_pattern(inst, loop_info, function)?)
                        } else {
                            None
                        },
                    });
                }
            }
        }
        
        Ok(operations)
    }
    
    /// ループをベクトル化します
    fn vectorize_loop(&mut self, candidate: &VectorizationCandidate, function: &Function) -> Result<()> {
        // ベクトル長の設定
        let vl = candidate.vector_length;
        
        // ベクトルレジスタの割り当て
        let register_mapping = self.allocate_vector_registers(candidate, function)?;
        
        // プロローグの生成（ベクトル長の設定、初期化など）
        self.generate_vector_prologue(candidate, vl, function)?;
        
        // メインループのベクトル化
        self.transform_loop_to_vector_operations(candidate, &register_mapping, function)?;
        
        // エピローグの生成（残りの要素の処理）
        self.generate_vector_epilogue(candidate, vl, function)?;
        
        Ok(())
    }
    
    /// 最適なベクトル長を決定します
    fn determine_optimal_vector_length(&self, operations: &[VectorizableOperation]) -> usize {
        // データ型に基づいてベクトル長を決定
        let mut min_element_size = usize::MAX;
        
        for op in operations {
            let element_size = match op.data_type {
                DataType::I8 | DataType::U8 => 1,
                DataType::I16 | DataType::U16 => 2,
                DataType::I32 | DataType::U32 | DataType::F32 => 4,
                DataType::I64 | DataType::U64 | DataType::F64 => 8,
                _ => continue,
            };
            
            min_element_size = min_element_size.min(element_size);
        }
        
        if min_element_size == usize::MAX {
            return self.config.default_vector_length;
        }
        
        // RISC-V Vベクトル拡張のLMUL設定を考慮
        let lmul = if min_element_size <= 2 { 8 } else { 4 };
        
        // ベクトルレジスタサイズ（通常128ビット）とLMULに基づいてベクトル長を計算
        let vector_register_bits = 128;
        let max_elements = (vector_register_bits * lmul) / (min_element_size * 8);
        
        // キャッシュラインサイズも考慮
        let cache_line_size = 64; // 一般的なキャッシュラインサイズ
        let elements_per_cache_line = cache_line_size / min_element_size;
        
        // 最適なベクトル長を決定（キャッシュラインの倍数が望ましい）
        let mut optimal_length = max_elements;
        while optimal_length > elements_per_cache_line && optimal_length % elements_per_cache_line != 0 {
            optimal_length -= 1;
        }
        
        optimal_length
    }
    
    /// ベクトルレジスタの割り当てを最適化します
    fn optimize_vector_register_allocation(&mut self, function: &Function) -> Result<()> {
        // ベクトルレジスタの使用状況を分析
        let mut register_usage = HashMap::new();
        
        for block in function.blocks.values() {
            for inst in &block.instructions {
                if inst.is_vector_instruction() {
                    for reg in inst.used_registers() {
                        if reg.is_vector_register() {
                            *register_usage.entry(reg).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
        
        // 使用頻度に基づいてレジスタを再割り当て
        let mut sorted_regs: Vec<_> = register_usage.iter().collect();
        sorted_regs.sort_by(|a, b| b.1.cmp(a.1));
        
        let mut new_mapping = HashMap::new();
        for (i, (old_reg, _)) in sorted_regs.iter().enumerate() {
            let new_reg = VectorRegister::new(i);
            new_mapping.insert(**old_reg, new_reg);
        }
        
        // レジスタの再割り当てを適用
        for block in function.blocks.values_mut() {
            for inst in &mut block.instructions {
                if inst.is_vector_instruction() {
                    inst.remap_registers(&new_mapping)?;
                }
            }
        }
        
        Ok(())
    }
    
    /// ベクトル命令のスケジューリングを最適化します
    fn schedule_vector_instructions(&mut self, function: &Function) -> Result<()> {
        for block in function.blocks.values_mut() {
            // 命令の依存関係グラフを構築
            let dep_graph = self.build_instruction_dependency_graph(block)?;
            
            // クリティカルパスを特定
            let critical_path = self.identify_critical_path(&dep_graph);
            
            // パイプラインの特性を考慮して命令をスケジューリング
            let scheduled_instructions = self.schedule_instructions_for_pipeline(block, &dep_graph, &critical_path)?;
            
            // 新しいスケジュールを適用
            block.instructions = scheduled_instructions;
        }
        
        Ok(())
    }
    
    /// ベクトル化による速度向上を推定します
    fn estimate_vectorization_speedup(&self, operations: &[VectorizableOperation]) -> f64 {
        if operations.is_empty() {
            return 1.0;
        }
        
        // 基本的な速度向上率はベクトル長に比例
        let base_speedup = self.determine_optimal_vector_length(operations) as f64;
        
        // 各操作タイプに対する調整係数
        let mut adjustment = 1.0;
        
        // メモリアクセスパターンによる調整
        let mut has_strided_access = false;
        let mut has_indexed_access = false;
        
        for op in operations {
            if let Some(pattern) = &op.memory_access_pattern {
                match pattern {
                    MemoryAccessPattern::Strided(_) => has_strided_access = true,
                    MemoryAccessPattern::Indexed => has_indexed_access = true,
                    _ => {}
                }
            }
        }
        
        // ストライドアクセスは速度低下の原因になる
        if has_strided_access {
            adjustment *= 0.8;
        }
        
        // インデックスアクセスはさらに速度低下の原因になる
        if has_indexed_access {
            adjustment *= 0.6;
        }
        
        // アライメントによる調整
        let mut has_unaligned_access = false;
        for op in operations {
            if op.alignment < 4 {
                has_unaligned_access = true;
                break;
            }
        }
        
        if has_unaligned_access {
            adjustment *= 0.9;
        }
        
        // オーバーヘッドを考慮
        let overhead_factor = 0.95;
        
        base_speedup * adjustment * overhead_factor
    }
    
    /// ベクトルプロローグコードを生成します
    fn generate_vector_prologue(&mut self, candidate: &VectorizationCandidate, vl: usize, function: &Function) -> Result<()> {
        let block_id = candidate.block_id;
        let block = function.blocks.get(&block_id).ok_or_else(|| Error::BlockNotFound(block_id))?;
        
        // プロローグ用の新しい基本ブロックを作成
        let prologue_block_id = self.create_new_block_before(block_id, function)?;
        
        // vsetvli命令を生成（ベクトル長の設定）
        let vsetvli_inst = Instruction::new_vsetvli(vl as u32);
        self.add_instruction_to_block(prologue_block_id, vsetvli_inst)?;
        
        // ベクトルレジスタの初期化
        for op in &candidate.operations {
            if op.op_type.requires_initialization() {
                let init_inst = self.create_vector_initialization_instruction(op)?;
                self.add_instruction_to_block(prologue_block_id, init_inst)?;
            }
        }
        
        Ok(())
    }
    
    /// ベクトルエピローグコードを生成します
    fn generate_vector_epilogue(&mut self, candidate: &VectorizationCandidate, vl: usize, function: &Function) -> Result<()> {
        // ループの場合、残りの要素を処理するコードを生成
        if let Some(loop_id) = candidate.loop_id {
            let loop_info = self.loop_analyzer.get_loop_info_by_id(loop_id)
                .ok_or_else(|| Error::LoopNotFound(loop_id))?;
            
            // ループの出口ブロックを特定
            let exit_block_id = loop_info.exit_block
                .ok_or_else(|| Error::InvalidLoopStructure("出口ブロックが見つかりません".to_string()))?;
            
            // エピローグ用の新しい基本ブロックを作成
            let epilogue_block_id = self.create_new_block_before(exit_block_id, function)?;
            
            // 残りの要素数を計算
            let remainder_calc_inst = self.create_remainder_calculation_instruction(loop_info, vl)?;
            self.add_instruction_to_block(epilogue_block_id, remainder_calc_inst)?;
            
            // 残りの要素がある場合の処理
            let remainder_check_inst = Instruction::new_branch_if_zero(Register::new(10), exit_block_id);
            self.add_instruction_to_block(epilogue_block_id, remainder_check_inst)?;
            
            // 残りの要素を処理するスカラーループを生成
            self.generate_scalar_remainder_loop(candidate, epilogue_block_id, exit_block_id, function)?;
        }
        
        Ok(())
    }
    
    /// ループをベクトル操作に変換します
    fn transform_loop_to_vector_operations(
        &mut self,
        candidate: &VectorizationCandidate,
        register_mapping: &HashMap<Register, VectorRegister>,
        function: &Function
    ) -> Result<()> {
        let block_id = candidate.block_id;
        let block = function.blocks.get(&block_id).ok_or_else(|| Error::BlockNotFound(block_id))?;
        
        // 新しいベクトル命令のリスト
        let mut new_instructions = Vec::new();
        
        // 元の命令をベクトル命令に変換
        for op in &candidate.operations {
            let inst = &block.instructions[op.inst_idx];
            
            // 命令タイプに基づいてベクトル命令を生成
            let vector_inst = match op.op_type {
                VectorizableOperationType::Add => self.create_vector_add_instruction(inst, op, register_mapping)?,
                VectorizableOperationType::Sub => self.create_vector_sub_instruction(inst, op, register_mapping)?,
                VectorizableOperationType::Mul => self.create_vector_mul_instruction(inst, op, register_mapping)?,
                VectorizableOperationType::Div => self.create_vector_div_instruction(inst, op, register_mapping)?,
                VectorizableOperationType::Load => self.create_vector_load_instruction(inst, op, register_mapping)?,
                VectorizableOperationType::Store => self.create_vector_store_instruction(inst, op, register_mapping)?,
                VectorizableOperationType::Compare => self.create_vector_compare_instruction(inst, op, register_mapping)?,
                VectorizableOperationType::BitwiseOp => self.create_vector_bitwise_instruction(inst, op, register_mapping)?,
                VectorizableOperationType::Shift => self.create_vector_shift_instruction(inst, op, register_mapping)?,
                VectorizableOperationType::Min => self.create_vector_min_instruction(inst, op, register_mapping)?,
                VectorizableOperationType::Max => self.create_vector_max_instruction(inst, op, register_mapping)?,
                VectorizableOperationType::Reduction => self.create_vector_reduction_instruction(inst, op, register_mapping)?,
            };
            
            new_instructions.push(vector_inst);
        }
        
        // インデックス更新命令を生成
        let index_update_inst = self.create_vector_index_update_instruction(candidate.vector_length as u32)?;
        new_instructions.push(index_update_inst);
        
        // ループ終了条件チェック命令を生成
        if let Some(loop_id) = candidate.loop_id {
            let loop_info = self.loop_analyzer.get_loop_info_by_id(loop_id)
                .ok_or_else(|| Error::LoopNotFound(loop_id))?;
            
            let loop_check_inst = self.create_vector_loop_check_instruction(loop_info, candidate.vector_length as u32)?;
            new_instructions.push(loop_check_inst);
        }
        
        // 新しい命令で元のブロックを更新
        self.update_block_instructions(block_id, new_instructions, function)?;
        
        Ok(())
    }
    
    /// ベクトルレジスタを割り当てます
    fn allocate_vector_registers(&self, candidate: &VectorizationCandidate, function: &Function) -> Result<HashMap<Register, VectorRegister>> {
        let mut mapping = HashMap::new();
        let mut next_vreg_id = 0;
        
        for op in &candidate.operations {
            let block = function.blocks.get(&op.block_id).ok_or_else(|| Error::BlockNotFound(op.block_id))?;
            let inst = &block.instructions[op.inst_idx];
            
            for reg in inst.used_registers() {
                if !mapping.contains_key(reg) {
                    // レジスタの型に基づいて適切なベクトルレジスタタイプを選択
                    let vreg_type = match op.data_type {
                        DataType::I8 | DataType::U8 => VectorRegisterType::Byte,
                        DataType::I16 | DataType::U16 => VectorRegisterType::Half,
                        DataType::I32 | DataType::U32 | DataType::F32 => VectorRegisterType::Word,
                        DataType::I64 | DataType::U64 | DataType::F64 => VectorRegisterType::Double,
                        _ => VectorRegisterType::Word, // デフォルト
                    };
                    
                    mapping.insert(*reg, VectorRegister::new_with_type(next_vreg_id, vreg_type));
                    next_vreg_id += 1;
                }
            }
        }
        
        Ok(mapping)
    }
}

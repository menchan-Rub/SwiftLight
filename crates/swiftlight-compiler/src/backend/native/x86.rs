// x86/x86_64アーキテクチャ向けのネイティブコード生成
// LLVM IRからx86_64アセンブリコードを生成する機能を提供します

use std::fmt;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::ir::{Value, Type};
use crate::optimization::BandwidthAwareOptimizer;
use crate::util::AlignedVec;
use crate::middleend::ir;
use crate::frontend::error::{CompilerError, ErrorKind, Result};
use crate::backend::native::{
    Register, Instruction, OperandType, AddressingMode,
    CallingConvention, InstructionEncoding
};
use crate::utils::{
    logger::Logger,
    error_handling::CompilerError as UtilsCompilerError
};

/// x86-64アーキテクチャ向けコード生成器
/// 帯域幅最適化、キャッシュライン配置、プリフェッチ制御を実装
pub struct X86CodeGenerator {
    code_buffer: AlignedVec<64>,
    current_label: String,
    bandwidth_optimizer: BandwidthAwareOptimizer,
    optimization_flags: X86OptimizationFlags,
}

/// x86-64の最適化フラグ（AVX-512、BMI2、ADX命令セットなど）
#[derive(Debug, Clone, Copy)]
struct X86OptimizationFlags {
    avx512: bool,
    bmi2: bool,
    adx: bool,
    sha: bool,
}

/// x86-64レジスタの完全なセット
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum X86Reg {
    RAX, RBX, RCX, RDX,
    RSI, RDI, RBP, RSP,
    R8,  R9,  R10, R11,
    R12, R13, R14, R15,
    XMM0, XMM1, XMM2, XMM3,
    XMM4, XMM5, XMM6, XMM7,
    YMM0, YMM1, YMM2, YMM3,
    YMM4, YMM5, YMM6, YMM7,
}

/// 命令エンコーディング用の補助構造体
struct InstructionEncoder {
    rex_prefix: u8,
    opcode: u8,
    modrm: Option<u8>,
    sib: Option<u8>,
    displacement: i32,
    immediate: i64,
}

/// キャッシュライン境界を考慮したコード配置
const CACHE_LINE_SIZE: usize = 64;

impl X86CodeGenerator {
    /// 新しいコード生成器を初期化（メモリ帯域幅検出付き）
    pub fn new() -> Self {
        Self {
            code_buffer: AlignedVec::new(),
            current_label: String::new(),
            bandwidth_optimizer: BandwidthAwareOptimizer::detect(),
            optimization_flags: detect_cpu_features(),
        }
    }

    /// 命令エンコーディングの主要メソッド
    fn encode_instruction(&mut self, inst: &InstructionEncoder) -> Result<(), CodegenError> {
        // 帯域幅最適化を適用
        self.bandwidth_optimizer.analyze(&inst);
        
        let mut bytes = Vec::with_capacity(15);
        // REXプレフィックスの処理
        if inst.rex_prefix != 0 {
            bytes.push(0x40 | inst.rex_prefix);
        }
        
        // オペコードの書き込み
        bytes.push(inst.opcode);
        
        // ModR/Mバイトの処理
        if let Some(modrm) = inst.modrm {
            bytes.push(modrm);
            
            // SIBバイトの処理
            if let Some(sib) = inst.sib {
                bytes.push(sib);
            }
        }
        
        // ディスプレースメントの処理
        if inst.displacement != 0 {
            let disp_bytes = inst.displacement.to_le_bytes();
            bytes.extend_from_slice(&disp_bytes);
        }
        
        // 即値の処理
        if inst.immediate != 0 {
            let imm_bytes = inst.immediate.to_le_bytes();
            bytes.extend_from_slice(&imm_bytes);
        }
        
        // キャッシュライン境界を考慮した配置
        self.align_code(CACHE_LINE_SIZE);
        self.code_buffer.extend(bytes);
        
        Ok(())
    }

    /// キャッシュライン境界に合わせたアライメント調整
    fn align_code(&mut self, alignment: usize) {
        let pos = self.code_buffer.len();
        let rem = pos % alignment;
        if rem != 0 {
            let pad = alignment - rem;
            self.code_buffer.resize(pos + pad, 0x90); // NOPでパディング
        }
    }

    /// メモリ帯域幅を考慮したデータ移動最適化
    fn optimize_data_transfer(&mut self, src: X86Reg, dst: X86Reg) {
        if self.bandwidth_optimizer.suggest_non_blocking() {
            self.prefetch_data(src);
        } else {
            self.mov(src, dst);
        }
    }

    /// プリフェッチ命令の生成（帯域幅に応じた距離調整）
    fn prefetch_data(&mut self, reg: X86Reg) {
        let distance = self.bandwidth_optimizer.prefetch_distance();
        self.encode_instruction(&InstructionEncoder {
            rex_prefix: 0,
            opcode: 0x0F,
            modrm: Some(0x18),
            sib: None,
            displacement: distance,
            immediate: 0,
        }).unwrap();
    }
}

/// システムV ABIに準拠した呼び出し規約
struct X86ABI {
    integer_args: [X86Reg; 6],
    float_args: [X86Reg; 8],
    return_reg: X86Reg,
}

impl X86ABI {
    /// System V AMD64 ABIの初期化
    fn system_v() -> Self {
        Self {
            integer_args: [RDI, RSI, RDX, RCX, R8, R9],
            float_args: [XMM0, XMM1, XMM2, XMM3, XMM4, XMM5, XMM6, XMM7],
            return_reg: RAX,
        }
    }
}

/// コード生成エラーの詳細な型
#[derive(Debug)]
pub enum CodegenError {
    InvalidRegisterCombination,
    UnsupportedInstruction(String),
    RegisterAllocationFailed,
    MemoryAlignmentError,
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidRegisterCombination => write!(f, "Invalid register combination"),
            Self::UnsupportedInstruction(s) => write!(f, "Unsupported instruction: {}", s),
            Self::RegisterAllocationFailed => write!(f, "Register allocation failed"),
            Self::MemoryAlignmentError => write!(f, "Memory alignment error"),
        }
    }
}

// 自動検出されたCPU機能の検出
fn detect_cpu_features() -> X86OptimizationFlags {
    let mut flags = X86OptimizationFlags {
        avx512: false,
        bmi2: false,
        adx: false,
        sha: false,
    };
    
    // CPUID命令を使用してCPU機能を検出
    unsafe {
        // 基本情報の取得（EAX=1）
        let mut eax: u32 = 1;
        let mut ebx: u32 = 0;
        let mut ecx: u32 = 0;
        let mut edx: u32 = 0;
        
        // CPUID命令を実行
        #[cfg(target_arch = "x86_64")]
        {
            use std::arch::x86_64::__cpuid;
            let cpuid_result = __cpuid(eax);
            ebx = cpuid_result.ebx;
            ecx = cpuid_result.ecx;
            edx = cpuid_result.edx;
        }
        
        #[cfg(target_arch = "x86")]
        {
            use std::arch::x86::__cpuid;
            let cpuid_result = __cpuid(eax);
            ebx = cpuid_result.ebx;
            ecx = cpuid_result.ecx;
            edx = cpuid_result.edx;
        }
        
        // 拡張機能の取得（EAX=7）
        eax = 7;
        ebx = 0;
        ecx = 0;
        edx = 0;
        
        #[cfg(target_arch = "x86_64")]
        {
            use std::arch::x86_64::__cpuid;
            let cpuid_result = __cpuid(eax);
            ebx = cpuid_result.ebx;
            ecx = cpuid_result.ecx;
            edx = cpuid_result.edx;
        }
        
        #[cfg(target_arch = "x86")]
        {
            use std::arch::x86::__cpuid;
            let cpuid_result = __cpuid(eax);
            ebx = cpuid_result.ebx;
            ecx = cpuid_result.ecx;
            edx = cpuid_result.edx;
        }
        
        // BMI2のチェック (EBX bit 8)
        flags.bmi2 = (ebx & (1 << 8)) != 0;
        
        // ADXのチェック (EBX bit 19)
        flags.adx = (ebx & (1 << 19)) != 0;
        
        // SHAのチェック (EBX bit 29)
        flags.sha = (ebx & (1 << 29)) != 0;
        
        // AVX-512のチェック (EBX bits 16, 17, 30, 31)
        let avx512f = (ebx & (1 << 16)) != 0;  // AVX-512 Foundation
        let avx512dq = (ebx & (1 << 17)) != 0; // AVX-512 Doubleword and Quadword
        let avx512bw = (ebx & (1 << 30)) != 0; // AVX-512 Byte and Word
        let avx512vl = (ebx & (1 << 31)) != 0; // AVX-512 Vector Length Extensions
        
        // すべての必要なAVX-512機能が利用可能な場合のみtrueに設定
        flags.avx512 = avx512f && avx512dq && avx512bw && avx512vl;
    }
    
    flags
}

// 命令セットのマクロ（AVX-512、BMI2などの条件付きコンパイル）
macro_rules! emit_instruction {
    ($self:ident, $opcode:expr, $modrm:expr) => {
        if $self.optimization_flags.avx512 {
            // AVX-512専用のエンコーディング
            $self.encode_avx512($opcode, $modrm);
        } else {
            // 標準エンコーディング
            $self.encode_standard($opcode, $modrm);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MOV命令のエンコーディングテスト
    #[test]
    fn test_mov_encoding() {
        let mut gen = X86CodeGenerator::new();
        gen.mov_rr(RAX, RBX).unwrap();
        assert_eq!(gen.code_buffer.as_slice(), &[0x48, 0x89, 0xD8]);
    }

    /// キャッシュラインアライメントのテスト
    #[test]
    fn test_cache_alignment() {
        let mut gen = X86CodeGenerator::new();
        gen.code_buffer.extend(vec![0x90; 60]);
        gen.align_code(CACHE_LINE_SIZE);
        assert_eq!(gen.code_buffer.len(), 64);
    }
}

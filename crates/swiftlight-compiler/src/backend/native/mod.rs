//! # ネイティブコードバックエンド
//! 
//! SwiftLight中間表現から各種アーキテクチャ向けのネイティブコードを生成するバックエンドです。
//! 主にLLVMバックエンドを利用して生成されたコードを最適化します。

use std::fs;
use std::path::Path;
use inkwell::OptimizationLevel;

use crate::frontend::error::{CompilerError, ErrorKind, Result};
use crate::middleend::ir as swift_ir;
use crate::backend::{Backend, Target as BackendTarget};
use crate::backend::llvm::LLVMBackend;

// アーキテクチャ別のモジュール
pub mod x86;
pub mod x86_64;
pub mod arm;
pub mod arm64;
pub mod riscv;

/// x86_64アーキテクチャ向けネイティブバックエンド
pub struct X86_64Backend {
    /// 内部的に使用するLLVMバックエンド
    llvm_backend: LLVMBackend,
}

impl X86_64Backend {
    /// 新しいx86_64バックエンドを作成
    pub fn new() -> Self {
        Self {
            llvm_backend: LLVMBackend::new()
                .with_optimization_level(OptimizationLevel::Aggressive)
                .with_target_triple("x86_64-unknown-linux-gnu".to_string()),
        }
    }
}

impl Backend for X86_64Backend {
    fn generate_code(&self, module: &swift_ir::Module) -> Result<Vec<u8>> {
        // LLVMバックエンドを使用してコード生成
        let obj_data = self.llvm_backend.generate_code(module)?;
        
        // x86_64固有の最適化を適用（必要に応じて）
        let optimized_data = self.optimize_x86_64(obj_data)?;
        
        Ok(optimized_data)
    }
    
    fn write_to_file(&self, code: &[u8], path: &Path) -> Result<()> {
        fs::write(path, code)
            .map_err(|e| CompilerError::new(
                ErrorKind::IO,
                format!("ファイル書き込みエラー: {}", e),
                None
            ))?;
        
        Ok(())
    }
    
    fn target(&self) -> BackendTarget {
        BackendTarget::X86_64
    }
}

impl X86_64Backend {
    /// x86_64固有の最適化を適用
    fn optimize_x86_64(&self, obj_data: Vec<u8>) -> Result<Vec<u8>> {
        // 現時点では追加の最適化は行わず、LLVMの最適化に任せる
        Ok(obj_data)
    }
}

/// ARM64アーキテクチャ向けネイティブバックエンド
pub struct ARM64Backend {
    /// 内部的に使用するLLVMバックエンド
    llvm_backend: LLVMBackend,
}

impl ARM64Backend {
    /// 新しいARM64バックエンドを作成
    pub fn new() -> Self {
        Self {
            llvm_backend: LLVMBackend::new()
                .with_optimization_level(OptimizationLevel::Aggressive)
                .with_target_triple("aarch64-unknown-linux-gnu".to_string()),
        }
    }
}

impl Backend for ARM64Backend {
    fn generate_code(&self, module: &swift_ir::Module) -> Result<Vec<u8>> {
        // LLVMバックエンドを使用してコード生成
        let obj_data = self.llvm_backend.generate_code(module)?;
        
        // ARM64固有の最適化を適用（必要に応じて）
        let optimized_data = self.optimize_arm64(obj_data)?;
        
        Ok(optimized_data)
    }
    
    fn write_to_file(&self, code: &[u8], path: &Path) -> Result<()> {
        fs::write(path, code)
            .map_err(|e| CompilerError::new(
                ErrorKind::IO,
                format!("ファイル書き込みエラー: {}", e),
                None
            ))?;
        
        Ok(())
    }
    
    fn target(&self) -> BackendTarget {
        BackendTarget::ARM64
    }
}

impl ARM64Backend {
    /// ARM64固有の最適化を適用
    fn optimize_arm64(&self, obj_data: Vec<u8>) -> Result<Vec<u8>> {
        // 現時点では追加の最適化は行わず、LLVMの最適化に任せる
        Ok(obj_data)
    }
}

/// RISC-Vアーキテクチャ向けネイティブバックエンド
pub struct RISCVBackend {
    /// 内部的に使用するLLVMバックエンド
    llvm_backend: LLVMBackend,
}

impl RISCVBackend {
    /// 新しいRISC-Vバックエンドを作成
    pub fn new() -> Self {
        Self {
            llvm_backend: LLVMBackend::new()
                .with_optimization_level(OptimizationLevel::Aggressive)
                .with_target_triple("riscv64-unknown-linux-gnu".to_string()),
        }
    }
}

impl Backend for RISCVBackend {
    fn generate_code(&self, module: &swift_ir::Module) -> Result<Vec<u8>> {
        // LLVMバックエンドを使用してコード生成
        let obj_data = self.llvm_backend.generate_code(module)?;
        
        // RISC-V固有の最適化を適用（必要に応じて）
        let optimized_data = self.optimize_riscv(obj_data)?;
        
        Ok(optimized_data)
    }
    
    fn write_to_file(&self, code: &[u8], path: &Path) -> Result<()> {
        fs::write(path, code)
            .map_err(|e| CompilerError::new(
                ErrorKind::IO,
                format!("ファイル書き込みエラー: {}", e),
                None
            ))?;
        
        Ok(())
    }
    
    fn target(&self) -> BackendTarget {
        BackendTarget::RISCV
    }
}

impl RISCVBackend {
    /// RISC-V固有の最適化を適用
    fn optimize_riscv(&self, obj_data: Vec<u8>) -> Result<Vec<u8>> {
        // 現時点では追加の最適化は行わず、LLVMの最適化に任せる
        Ok(obj_data)
    }
}

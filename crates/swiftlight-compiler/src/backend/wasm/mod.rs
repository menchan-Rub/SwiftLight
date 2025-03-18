//! # WebAssembly バックエンド
//! 
//! SwiftLight中間表現からWebAssemblyバイナリを生成するバックエンドです。

use std::fs;
use std::path::Path;

use crate::frontend::error::{CompilerError, ErrorKind, Result};
use crate::middleend::ir as swift_ir;
use crate::backend::{Backend, Target as BackendTarget, BackendConfig, OptimizationLevel};
use crate::backend::debug::DebugInfoLevel;

mod codegen;

pub use self::codegen::CodeGenerator;

/// WebAssembly バックエンド実装
pub struct WasmBackend {
    /// 最適化レベル
    optimization_level: u8,
    /// デバッグ情報の生成
    debug_info: bool,
    /// バックエンド設定
    config: BackendConfig,
}

impl WasmBackend {
    /// 新しいWebAssemblyバックエンドを作成
    pub fn new() -> Self {
        Self {
            optimization_level: 1, // デフォルトは最適化レベル1
            debug_info: false,
            config: BackendConfig::default(),
        }
    }
    
    /// 最適化レベルを設定
    pub fn with_optimization_level(mut self, level: u8) -> Self {
        self.optimization_level = level;
        self
    }
    
    /// デバッグ情報の生成を設定
    pub fn with_debug_info(mut self, debug_info: bool) -> Self {
        self.debug_info = debug_info;
        self
    }
}

impl Backend for WasmBackend {
    fn generate_code(&self, module: &swift_ir::Module) -> Result<Vec<u8>> {
        // Wasm コード生成の実行
        let mut codegen = CodeGenerator::new(self.optimization_level, self.debug_info);
        let wasm_binary = codegen.generate_module(module)?;
        
        Ok(wasm_binary)
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
        BackendTarget::Wasm
    }

    fn config(&self) -> &BackendConfig {
        &self.config
    }

    fn set_config(&mut self, config: BackendConfig) {
        self.config = config;
        // configから最適化レベルとデバッグ情報の設定も更新
        self.optimization_level = match config.optimization_level {
            OptimizationLevel::None => 0,
            OptimizationLevel::Less => 1,
            OptimizationLevel::Default => 2,
            OptimizationLevel::Aggressive => 3,
        };
        self.debug_info = config.debug_info_level != DebugInfoLevel::None;
    }

    fn optimize(&self, module: &mut swift_ir::Module) -> Result<()> {
        // 最適化レベルに応じた最適化を実行
        match self.config.optimization_level {
            OptimizationLevel::None => {
                // 最適化なし
            },
            OptimizationLevel::Less => {
                // 基本的な最適化のみ
                self.basic_optimizations(module)?;
            },
            OptimizationLevel::Default => {
                // 標準的な最適化
                self.basic_optimizations(module)?;
                self.standard_optimizations(module)?;
            },
            OptimizationLevel::Aggressive => {
                // 積極的な最適化
                self.basic_optimizations(module)?;
                self.standard_optimizations(module)?;
                self.aggressive_optimizations(module)?;
            },
        }
        
        Ok(())
    }

    fn clone_for_parallel_codegen(&self) -> Box<dyn Backend> {
        Box::new(Self {
            optimization_level: self.optimization_level,
            debug_info: self.debug_info,
            config: self.config.clone(),
        })
    }
}

// 最適化メソッドを実装
impl WasmBackend {
    /// 基本的な最適化を実行
    fn basic_optimizations(&self, module: &mut swift_ir::Module) -> Result<()> {
        // 定数畳み込みなどの基本的な最適化
        // 実際の実装では、各最適化パスを適用する
        
        Ok(())
    }
    
    /// 標準的な最適化を実行
    fn standard_optimizations(&self, module: &mut swift_ir::Module) -> Result<()> {
        // 基本ブロックの結合、冗長な計算の削除など
        
        Ok(())
    }
    
    /// 積極的な最適化を実行
    fn aggressive_optimizations(&self, module: &mut swift_ir::Module) -> Result<()> {
        // インライン展開、ループ最適化など
        
        Ok(())
    }
}

//! # WebAssembly バックエンド
//! 
//! SwiftLight中間表現からWebAssemblyバイナリを生成するバックエンドです。

use std::fs;
use std::path::Path;

use crate::frontend::error::{CompilerError, ErrorKind, Result};
use crate::middleend::ir as swift_ir;
use crate::backend::{Backend, Target as BackendTarget};

mod codegen;

pub use self::codegen::CodeGenerator;

/// WebAssembly バックエンド実装
pub struct WasmBackend {
    /// 最適化レベル
    optimization_level: u8,
    /// デバッグ情報の生成
    debug_info: bool,
}

impl WasmBackend {
    /// 新しいWebAssemblyバックエンドを作成
    pub fn new() -> Self {
        Self {
            optimization_level: 1, // デフォルトは最適化レベル1
            debug_info: false,
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
}

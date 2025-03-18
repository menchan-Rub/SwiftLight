//! # LLVM バックエンド
//! 
//! SwiftLight中間表現からLLVM IRを生成するバックエンドです。
//! inkwellクレートを使用してLLVMとの連携を実現します。

use std::fs;
use std::path::Path;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine, TargetTriple
};
use inkwell::OptimizationLevel;

use crate::frontend::error::{CompilerError, ErrorKind, Result};
use crate::middleend::ir as swift_ir;
use crate::backend::{Backend, Target as BackendTarget};

mod codegen;

pub use self::codegen::CodeGenerator;

/// LLVM バックエンド実装
pub struct LLVMBackend {
    /// 最適化レベル
    optimization_level: OptimizationLevel,
    /// ターゲットトリプル
    target_triple: Option<String>,
}

impl LLVMBackend {
    /// 新しいLLVMバックエンドを作成
    pub fn new() -> Self {
        // ターゲットの初期化
        Self::initialize_targets();
        
        Self {
            optimization_level: OptimizationLevel::Default,
            target_triple: None,
        }
    }
    
    /// 最適化レベルを設定
    pub fn with_optimization_level(mut self, level: OptimizationLevel) -> Self {
        self.optimization_level = level;
        self
    }
    
    /// ターゲットトリプルを設定
    pub fn with_target_triple(mut self, triple: String) -> Self {
        self.target_triple = Some(triple);
        self
    }
    
    /// LLVMターゲットを初期化
    fn initialize_targets() {
        // LLVMターゲットの初期化を一度だけ行う
        static INIT: std::sync::Once = std::sync::Once::new();
        
        INIT.call_once(|| {
            Target::initialize_all(&InitializationConfig::default());
        });
    }
    
    /// ターゲットマシンを取得
    fn create_target_machine(&self) -> Result<TargetMachine> {
        let triple = match &self.target_triple {
            Some(triple) => TargetTriple::create(triple),
            None => TargetTriple::create(&TargetMachine::get_default_triple().to_string()),
        };
        
        let target = Target::from_triple(&triple)
            .map_err(|e| CompilerError::new(
                ErrorKind::CodeGen,
                format!("ターゲットの取得に失敗: {}", e),
                None
            ))?;
        
        let target_machine = target
            .create_target_machine(
                &triple,
                &TargetMachine::get_host_cpu_name().to_string(),
                &TargetMachine::get_host_cpu_features().to_string(),
                self.optimization_level,
                RelocMode::Default,
                CodeModel::Default,
            )
            .ok_or_else(|| CompilerError::new(
                ErrorKind::CodeGen,
                "ターゲットマシンの作成に失敗".to_string(),
                None
            ))?;
        
        Ok(target_machine)
    }
}

impl Backend for LLVMBackend {
    fn generate_code(&self, module: &swift_ir::Module) -> Result<Vec<u8>> {
        // LLVMコンテキストの作成
        let context = Context::create();
        
        // コード生成の実行
        let mut codegen = CodeGenerator::new(&context);
        let llvm_module = codegen.generate_module(module)?;
        
        // オブジェクトファイルの生成
        let target_machine = self.create_target_machine()?;
        let obj_data = target_machine
            .write_to_memory_buffer(&llvm_module, FileType::Object)
            .map_err(|e| CompilerError::new(
                ErrorKind::CodeGen,
                format!("オブジェクトファイルの生成に失敗: {}", e),
                None
            ))?
            .as_slice()
            .to_vec();
        
        Ok(obj_data)
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
        BackendTarget::LLVMIR
    }
    
    fn config(&self) -> &super::BackendConfig {
        todo!()
    }
    
    fn set_config(&mut self, config: super::BackendConfig) {
        todo!()
    }
    
    fn optimize(&self, module: &mut crate::middleend::ir::Module) -> crate::frontend::error::Result<()> {
        todo!()
    }
    
    fn clone_for_parallel_codegen(&self) -> Box<dyn Backend> {
        todo!()
    }
}

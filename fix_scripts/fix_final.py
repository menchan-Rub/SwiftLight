#!/usr/bin/env python3
import os

# 対象ファイル
file_path = "crates/swiftlight-compiler/src/middleend/ir/mod.rs"

# バックアップディレクトリ
backup_dir = "crates/backups/manual"
os.makedirs(backup_dir, exist_ok=True)

# バックアップ作成
backup_path = os.path.join(backup_dir, "mod.rs.final.bak")
with open(file_path, "r") as src:
    with open(backup_path, "w") as dst:
        dst.write(src.read())
print(f"バックアップを作成しました: {backup_path}")

# 新しいファイル内容
new_content = """// ファイルの内容を置き換え
// IRジェネレーター実装

pub mod representation;

use std::collections::HashMap;
use std::fmt::{self, Display};

use inkwell::AddressSpace;
use inkwell::basic_block::BasicBlock as LLVMBasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module as LLVMModule;
use inkwell::types::{AnyTypeEnum, BasicTypeEnum, StructType};
use inkwell::values::{
    AnyValueEnum, BasicValueEnum, FunctionValue, PointerValue, IntValue, FloatValue
};
use inkwell::{FloatPredicate, IntPredicate};

use crate::common::errors::{CompilerError, ErrorKind, Result};
use crate::common::source_location::SourceLocation;
use crate::frontend::ast::*;
use crate::frontend::semantic::TypeCheckResult;
use crate::utils::logger::{Logger, LogLevel};

use self::representation::{
    BasicBlock, Function, Global, Module, StructField, Type, Value
};

/// IR ジェネレーター - LLVMコード生成
pub struct IRGenerator<'ctx> {
    /// LLVM コンテキスト
    context: &'ctx Context,
    
    /// LLVM モジュール
    llvm_module: LLVMModule<'ctx>,
    
    /// LLVM IR ビルダー
    builder: Builder<'ctx>,
    
    /// 型チェック結果（ノードの型情報）
    type_info: TypeCheckResult,
    
    /// 生成中のモジュール
    module: Module,
    
    /// 値のマッピング（AST ノードID -> IR 値）
    values: HashMap<NodeId, Value>,
    
    /// 基本ブロックのマッピング（SwiftLight関数ID -> 基本ブロックのリスト）
    blocks: HashMap<String, Vec<BasicBlock>>,
    
    /// 現在の関数の値
    current_function: Option<FunctionValue<'ctx>>,
    
    /// 現在の基本ブロック
    current_block: Option<LLVMBasicBlock<'ctx>>,
    
    /// ローカル変数のマッピング（変数名 -> LLVM ポインタ値）
    variables: HashMap<String, PointerValue<'ctx>>,
    
    /// 関数のマッピング（関数名 -> LLVM 関数値）
    functions: HashMap<String, FunctionValue<'ctx>>,
    
    /// 構造体のマッピング（構造体名 -> LLVM 構造体型）
    structs: HashMap<String, StructType<'ctx>>,
    
    /// 一時変数カウンタ（一意な名前生成用）
    temp_counter: usize,
    
    /// エラー情報
    errors: Vec<CompilerError>,
    
    /// デバッグ情報有効フラグ
    debug_info: bool,
    
    /// 現在のループの条件ブロック（continue先）
    current_loop_condition: Option<LLVMBasicBlock<'ctx>>,
    
    /// 現在のループの終了ブロック（break先）
    current_loop_exit: Option<LLVMBasicBlock<'ctx>>,
    
    /// 現在の例外ハンドラ情報
    current_exception_handler: Option<(PointerValue<'ctx>, PointerValue<'ctx>, LLVMBasicBlock<'ctx>)>,
    
    /// ジェネリック関数の型パラメータ
    current_type_parameters: HashMap<String, TypeAnnotation>,
}

/// IRGenerator の実装
impl<'ctx> IRGenerator<'ctx> {
    /// 新しいIRジェネレーターを作成
    pub fn new(type_info: &TypeCheckResult) -> Self {
        // 初期化を簡略化
        let context = Context::create();
        let module = LLVMModule::create("main_module");
        let builder = context.create_builder();
        
        Self {
            context: &context,
            llvm_module: module,
            builder,
            type_info: type_info.clone(),
            module: Module::new("main"),
            values: HashMap::new(),
            blocks: HashMap::new(),
            current_function: None,
            current_block: None,
            variables: HashMap::new(),
            functions: HashMap::new(),
            structs: HashMap::new(),
            temp_counter: 0,
            errors: Vec::new(),
            debug_info: false,
            current_loop_condition: None,
            current_loop_exit: None,
            current_exception_handler: None,
            current_type_parameters: HashMap::new(),
        }
    }
    
    /// エラーを追加
    fn add_error(&mut self, message: impl Into<String>, location: Option<SourceLocation>) {
        self.errors.push(CompilerError::new(ErrorKind::CodeGeneration, message, location));
    }
    
    /// 一時変数名を生成
    fn generate_temp_name(&mut self, prefix: &str) -> String {
        let name = format!("{}.{}", prefix, self.temp_counter);
        self.temp_counter += 1;
        name
    }
    
    /// モジュールの生成
    pub fn generate_module(&mut self, program: &Program) -> Result<Module> {
        // モジュールの初期化
        self.initialize_module(program)?;
        
        // AST変換
        for decl in &program.declarations {
            match &decl.kind {
                DeclarationKind::FunctionDecl(func) => {
                    self.generate_function(func)?;
                },
                DeclarationKind::StructDecl(struct_def) => {
                    // 事前宣言済みなので処理は不要
                },
                DeclarationKind::EnumDecl(enum_def) => {
                    // 列挙型の生成
                    // self.generate_enum(enum_def)?;
                },
                DeclarationKind::TraitDecl(trait_def) => {
                    // トレイトは型情報のみ利用
                },
                DeclarationKind::ImplementationDecl(impl_def) => {
                    // 実装ブロックのメソッドを生成
                    // self.generate_implementation(impl_def)?;
                },
                _ => {
                    // その他の宣言は現在サポート外
                }
            }
        }
        
        // エントリーポイント関数を生成
        let entry = self.generate_program_entry(program)?;
        
        // モジュールの最終処理
        self.finalize_module()?;
        
        Ok(self.module.clone())
    }
    
    // その他の必要なメソッド...
    
    /// モジュールの初期化処理
    fn initialize_module(&mut self, program: &Program) -> Result<()> {
        // モジュール名を設定
        self.module.name = program.source_path.clone();
        
        // ランタイム関数の宣言
        self.declare_runtime_functions()?;
        
        // 型の事前宣言
        self.predeclare_types(program)?;
        
        // 関数の事前宣言
        self.predeclare_functions(program)?;
        
        Ok(())
    }
    
    /// ランタイム関数の宣言
    fn declare_runtime_functions(&mut self) -> Result<()> {
        // 簡略化のためスケルトン実装のみ
        Ok(())
    }
    
    /// 型の事前宣言
    fn predeclare_types(&mut self, program: &Program) -> Result<()> {
        // 簡略化のためスケルトン実装のみ
        Ok(())
    }
    
    /// 関数の事前宣言
    fn predeclare_functions(&mut self, program: &Program) -> Result<()> {
        // 簡略化のためスケルトン実装のみ
        Ok(())
    }
    
    /// プログラムのエントリーポイント生成
    fn generate_program_entry(&mut self, program: &Program) -> Result<FunctionValue<'ctx>> {
        // 簡略化のためスケルトン実装のみ
        // 実際のプログラムではmain関数を生成
        Ok(self.current_function.unwrap_or_else(|| {
            let void_type = self.context.void_type();
            let fn_type = void_type.fn_type(&[], false);
            self.llvm_module.add_function("main", fn_type, None)
        }))
    }
    
    /// モジュールの最終処理
    fn finalize_module(&mut self) -> Result<()> {
        // 簡略化のためスケルトン実装のみ
        Ok(())
    }
}
"""

# ファイルに書き戻す
with open(file_path, "w", encoding="utf-8") as f:
    f.write(new_content)

print(f"ファイル {file_path} を修正しました") 
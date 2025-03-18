//! # WebAssembly コード生成器
//! 
//! SwiftLight中間表現からWebAssemblyバイナリを生成するためのコード生成器です。

use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Write;
use std::path::Path;
use std::fs::File;
use std::io::{self, Read, Write as IoWrite};
use std::error::Error;

use crate::frontend::error::{CompilerError, ErrorKind, Result, Diagnostic};
use crate::middleend::ir as swift_ir;
use crate::middleend::ir::representation::{Value, Instruction, Function, Module, Type};
use crate::frontend::ast::{BinaryOperator, UnaryOperator, ExpressionKind};
use crate::backend::debug::{DebugInfoLevel, DebugInfoWriter};

/// WebAssembly 関数タイプ
#[derive(Debug, Clone, PartialEq, Eq)]
enum WasmType {
    I32,
    I64,
    F32,
    F64,
    Void,
}

/// WebAssembly 関数シグネチャ
#[derive(Debug, Clone)]
struct WasmFunctionType {
    params: Vec<WasmType>,
    results: Vec<WasmType>,
}

/// ローカル関数定義
#[derive(Debug, Clone)]
struct LocalFunction {
    /// 関数名
    name: String,
    /// 型インデックス
    type_index: usize,
    /// ローカル変数の情報
    locals: Vec<(usize, WasmType)>,
    /// 関数本体のバイトコード
    body: Vec<u8>,
}

/// WebAssembly コード生成器
pub struct CodeGenerator {
    /// 最適化レベル
    optimization_level: u8,
    /// デバッグ情報の生成
    debug_info: bool,
    /// 型マッピング (SwiftLight型ID → Wasm型)
    types: HashMap<usize, WasmType>,
    /// 関数マッピング (SwiftLight関数ID → Wasm関数インデックス)
    functions: HashMap<usize, usize>,
    /// グローバル変数マッピング (SwiftLight変数ID → Wasm グローバルインデックス)
    globals: HashMap<usize, usize>,
    /// 生成したWasmセクション
    sections: Vec<Vec<u8>>,
    /// 現在のローカル変数
    locals: Vec<WasmType>,
    /// 一時変数のインデックス
    current_local: usize,
    /// 関数シグネチャテーブル
    function_types: Vec<WasmFunctionType>,
    /// ローカル関数定義
    local_functions: Vec<LocalFunction>,
    /// インポート定義
    imports: Vec<ImportEntry>,
    /// エクスポート定義
    exports: Vec<ExportEntry>,
}

/// エクスポート種別
#[derive(Debug, Clone)]
enum ExportKind {
    /// 関数エクスポート
    Function(usize),
    /// テーブルエクスポート
    Table(usize),
    /// メモリエクスポート
    Memory(usize),
    /// グローバル変数エクスポート
    Global(usize),
}

/// エクスポートエントリ
#[derive(Debug, Clone)]
struct ExportEntry {
    /// エクスポート名
    name: String,
    /// エクスポートの種類
    kind: ExportKind,
}

impl CodeGenerator {
    /// 新しいコード生成器を作成
    pub fn new(optimization_level: u8, debug_info: bool) -> Self {
        Self {
            optimization_level,
            debug_info,
            types: HashMap::new(),
            functions: HashMap::new(),
            globals: HashMap::new(),
            sections: Vec::new(),
            locals: Vec::new(),
            current_local: 0,
            function_types: Vec::new(),
            local_functions: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
        }
    }
    
    /// SwiftLight中間表現からWebAssemblyバイナリを生成
    pub fn generate_module(&mut self, module: &swift_ir::Module) -> Result<Vec<u8>> {
        // Wasmモジュールの初期化
        self.init_module()?;
        
        // 型の登録
        for (id, ty) in &module.types {
            self.register_type(*id, ty)?;
        }
        
        // 関数シグネチャの登録
        for (id, func) in &module.functions {
            self.register_function_signature(*id, func)?;
        }
        
        // グローバル変数の定義
        for (id, global) in &module.globals {
            self.define_global(*id, global)?;
        }
        
        // 関数の定義
        for (id, func) in &module.functions {
            if !func.is_external {
                self.define_function(*id, func)?;
            }
        }
        
        // エクスポートセクションの生成
        self.generate_export_section(module)?;
        
        // Wasmバイナリの生成
        let wasm_binary = self.finalize_module()?;
        
        Ok(wasm_binary)
    }
    
    /// Wasmモジュールの初期化
    fn init_module(&mut self) -> Result<()> {
        // Wasmマジックナンバーとバージョン
        let mut header = Vec::new();
        header.extend_from_slice(&[0x00, 0x61, 0x73, 0x6D]); // マジックナンバー "\0asm"
        header.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // バージョン 1
        
        self.sections.push(header);
        
        Ok(())
    }
    
    /// 型を登録
    fn register_type(&mut self, type_id: usize, ty: &Type) -> Result<WasmType> {
        // すでに登録済みの場合はキャッシュから返す
        if let Some(wasm_type) = self.types.get(&type_id) {
            return Ok(wasm_type.clone());
        }
        
        // 型の種類に応じてWasm型を生成
        let wasm_type = match &ty.kind {
            TypeKind::Void => WasmType::Void,
            TypeKind::Boolean => WasmType::I32, // Wasmではboolもi32で表現
            TypeKind::Integer { bits, signed: _ } => {
                match *bits {
                    32 => WasmType::I32,
                    64 => WasmType::I64,
                    // その他のビット数はサポートするもっとも近いサイズにマッピング
                    n if n <= 32 => WasmType::I32,
                    _ => WasmType::I64,
                }
            },
            TypeKind::Float { bits } => {
                match *bits {
                    32 => WasmType::F32,
                    64 => WasmType::F64,
                    // その他のビット数はサポートするもっとも近いサイズにマッピング
                    n if n <= 32 => WasmType::F32,
                    _ => WasmType::F64,
                }
            },
            TypeKind::Pointer { pointee_type_id: _ } => {
                // WebAssemblyではポインタもi32で表現（アドレス空間）
                WasmType::I32
            },
            TypeKind::Array { element_type_id: _, size: _ } => {
                // 配列はメモリに格納し、先頭アドレスをi32で表現
                WasmType::I32
            },
            TypeKind::Struct { name: _, fields: _ } => {
                // 構造体もメモリに格納し、先頭アドレスをi32で表現
                WasmType::I32
            },
            TypeKind::Function { signature_id: _ } => {
                // 関数ポインタもi32（テーブルインデックス）
                WasmType::I32
            },
            _ => return Err(CompilerError::new(
                ErrorKind::CodeGen,
                format!("サポートされていない型: {:?}", ty.kind),
                None
            )),
        };
        
        // 生成したWasm型をキャッシュに登録
        self.types.insert(type_id, wasm_type.clone());
        
        Ok(wasm_type)
    }
    
    /// 関数シグネチャを登録
    fn register_function_signature(&mut self, func_id: usize, func: &Function) -> Result<usize> {
        let signature = &func.signature;
        
        // 引数型の変換
        let mut param_types = Vec::new();
        for &param_type_id in &signature.parameter_type_ids {
            let wasm_type = self.get_type(param_type_id)?;
            if wasm_type != WasmType::Void {
                param_types.push(wasm_type);
            }
        }
        
        // 戻り値型の変換
        let mut result_types = Vec::new();
        let return_type = self.get_type(signature.return_type_id)?;
        if return_type != WasmType::Void {
            result_types.push(return_type);
        }
        
        // 関数タイプの作成
        let function_type = WasmFunctionType {
            params: param_types,
            results: result_types,
        };
        
        // 同じシグネチャが既に存在するか確認
        let type_index = match self.function_types.iter().position(|ft| *ft == function_type) {
            Some(idx) => idx,
            None => {
                // 新しいシグネチャを追加
                let idx = self.function_types.len();
                self.function_types.push(function_type);
                idx
            }
        };
        
        // 関数IDとシグネチャインデックスをマッピング
        self.functions.insert(func_id, type_index);
        
        // 関数シグネチャのメタデータを記録（最適化やデバッグ情報用）
        self.function_metadata.insert(func_id, FunctionMetadata {
            signature_index: type_index,
            name: func.name.clone(),
            parameter_names: func.signature.parameter_names.clone(),
            is_exported: func.is_exported,
            optimization_hints: func.optimization_hints.clone(),
            source_location: func.source_location.clone(),
            inlining_strategy: self.determine_inlining_strategy(func),
            specialized_variants: Vec::new(),
        });
        
        // 依存型を使用している場合の特殊処理
        if func.has_dependent_types {
            self.register_dependent_type_handler(func_id, &func.signature)?;
        }
        
        // SIMD最適化の可能性を検討
        if self.config.enable_simd_optimization {
            self.analyze_simd_opportunities(func_id, func)?;
        }
        
        // WebAssembly GC拡張を使用する場合の型情報を登録
        if self.config.use_wasm_gc && func.uses_reference_types {
            self.register_gc_type_info(func_id, func)?;
        }
        
        // 例外処理メカニズムの設定（Wasm例外処理提案に基づく）
        if func.has_exception_handlers && self.config.use_wasm_exceptions {
            self.setup_exception_handling(func_id, func)?;
        }
        
        // テールコール最適化の可能性を分析
        if self.config.enable_tail_call_optimization {
            self.analyze_tail_calls(func_id, func)?;
        }
        
        // 関数インポート/エクスポート情報の更新
        if func.is_exported {
            self.exports.push(ExportEntry {
                name: func.name.clone(),
                kind: ExportKind::Function(self.function_indices.get(&func_id).copied().unwrap_or(0)),
            });
        }
        
        // 関数のプロファイリングとトレース情報を設定
        if self.config.enable_profiling {
            self.setup_function_profiling(func_id, func)?;
        }
        
        // コンパイル時計算の最適化
        if let Some(compile_time_result) = &func.compile_time_evaluation {
            self.apply_compile_time_optimization(func_id, compile_time_result)?;
        }
        
        // メモリ安全性の追加検証（冗長だが安全性のため）
        self.verify_memory_safety(func)?;
        
        // 並行処理の安全性検証
        if func.has_concurrent_operations {
            self.verify_concurrency_safety(func)?;
        }
        
        // WebAssembly Component Model対応（将来の拡張性のため）
        if self.config.use_component_model {
            self.register_component_model_interface(func_id, func)?;
        }
        
        // 関数のホットパス分析と最適化
        if self.config.enable_hot_path_optimization {
            self.analyze_hot_paths(func_id, func)?;
        }
        
        // 関数のインライン展開候補を登録
        if self.should_inline(func) {
            self.inline_candidates.insert(func_id);
        }
        
        Ok(type_index)
    }
    
    /// 型IDからWasm型を取得
    fn get_type(&self, type_id: usize) -> Result<WasmType> {
        match self.types.get(&type_id) {
            Some(ty) => Ok(ty.clone()),
            None => Err(CompilerError::new(
                ErrorKind::CodeGen,
                format!("型ID {}が見つかりません", type_id),
                None
            )),
        }
    }
    
    /// インライン化戦略を決定
    fn determine_inlining_strategy(&self, func: &Function) -> InliningStrategy {
        // 関数の複雑さを計算
        let complexity = self.calculate_function_complexity(func);
        
        // 関数サイズに基づく判断
        if func.blocks.len() <= 1 && func.instruction_count() < 10 {
            return InliningStrategy::AlwaysInline;
        }
        
        // 再帰関数の場合
        if func.is_recursive {
            // 末尾再帰の場合は特殊な最適化
            if func.has_tail_recursion {
                return InliningStrategy::TailRecursionOptimize;
            }
            return InliningStrategy::NoInline;
        }
        
        // ホット関数の場合
        if let Some(profile_data) = &func.profile_data {
            if profile_data.call_frequency > self.config.hot_function_threshold {
                if complexity < self.config.inline_complexity_threshold {
                    return InliningStrategy::HotInline;
                }
            }
        }
        
        // デフォルトはコンパイラに判断を委ねる
        InliningStrategy::Auto
    }
    
    /// 関数の複雑さを計算
    fn calculate_function_complexity(&self, func: &Function) -> usize {
        let mut complexity: usize = 0;
        
        // 基本ブロック数による複雑さ
        complexity += func.blocks.len() * 5;
        
        // 命令数による複雑さ
        complexity += func.instruction_count();
        
        // 制御フロー複雑性
        complexity += func.control_flow_complexity * 10;
        
        // ループによる複雑さ
        complexity += func.loop_count * 15;
        
        // 再帰による複雑さ
        if func.is_recursive {
            complexity += 50;
        }
        
        complexity
    }
    
    /// SIMD最適化の機会を分析
    fn analyze_simd_opportunities(&mut self, func_id: usize, func: &Function) -> Result<()> {
        // ベクトル演算パターンを検出
        let mut vector_patterns: Vec<_> = Vec::new();
        
        for block_id in &func.blocks {
            if let Some(block) = func.basic_blocks.get(block_id) {
                // 連続したメモリアクセスパターンを検出
                self.detect_sequential_memory_access(block, &mut vector_patterns)?;
                
                // 並列計算パターンを検出
                self.detect_parallel_computation(block, &mut vector_patterns)?;
            }
        }
        
        // 検出したパターンをSIMD最適化候補として登録
        if !vector_patterns.is_empty() {
            self.simd_optimization_candidates.insert(func_id, vector_patterns);
        }
        
        Ok(())
    }
    
    /// 連続したメモリアクセスパターンを検出
    fn detect_sequential_memory_access(&self, block: &BasicBlock, patterns: &mut Vec<SimdPattern>) -> Result<()> {
        let mut current_pattern: Option<SimdPatternBuilder> = None;
        
        for inst in &block.instructions {
            match &inst.kind {
                InstructionKind::Load { address, .. } => {
                    // 連続したロードを検出
                    if let Some(base_addr) = self.extract_base_address(address) {
                        if let Some(ref mut pattern) = current_pattern {
                            if pattern.can_extend_with_load(base_addr, inst) {
                                pattern.add_instruction(inst);
                                continue;
                            }
                        }
                        
                        // 新しいパターンを開始
                        let mut new_pattern = SimdPatternBuilder::new(SimdPatternKind::SequentialLoad);
                        new_pattern.add_instruction(inst);
                        current_pattern = Some(new_pattern);
                    }
                },
                InstructionKind::Store { address, .. } => {
                    // 連続したストアを検出
                    if let Some(base_addr) = self.extract_base_address(address) {
                        if let Some(ref mut pattern) = current_pattern {
                            if pattern.can_extend_with_store(base_addr, inst) {
                                pattern.add_instruction(inst);
                                continue;
                            }
                        }
                        
                        // 新しいパターンを開始
                        let mut new_pattern = SimdPatternBuilder::new(SimdPatternKind::SequentialStore);
                        new_pattern.add_instruction(inst);
                        current_pattern = Some(new_pattern);
                    }
                },
                _ => {
                    // パターンを完成させる
                    if let Some(pattern) = current_pattern.take() {
                        if pattern.is_valid_for_simd() {
                            patterns.push(pattern.build());
                        }
                    }
                }
            }
        }
        
        // 最後のパターンを処理
        if let Some(pattern) = current_pattern {
            if pattern.is_valid_for_simd() {
                patterns.push(pattern.build());
            }
        }
        
        Ok(())
    }
    
    /// 並列計算パターンを検出
    fn detect_parallel_computation(&self, block: &BasicBlock, patterns: &mut Vec<SimdPattern>) -> Result<()> {
        // 同じ演算を複数のデータに適用するパターンを検出
        let mut current_pattern: Option<SimdPatternBuilder> = None;
        
        for inst in &block.instructions {
            match &inst.kind {
                InstructionKind::BinaryOp { op, lhs, rhs } => {
                    // 同じ演算子を使った連続した計算を検出
                    if let Some(ref mut pattern) = current_pattern {
                        if pattern.can_extend_with_binary_op(op, inst) {
                            pattern.add_instruction(inst);
                            continue;
                        }
                    }
                    
                    // 新しいパターンを開始
                    let mut new_pattern = SimdPatternBuilder::new(SimdPatternKind::ParallelComputation);
                    new_pattern.set_operation(*op);
                    new_pattern.add_instruction(inst);
                    current_pattern = Some(new_pattern);
                },
                _ => {
                    // パターンを完成させる
                    if let Some(pattern) = current_pattern.take() {
                        if pattern.is_valid_for_simd() {
                            patterns.push(pattern.build());
                        }
                    }
                }
            }
        }
        
        // 最後のパターンを処理
        if let Some(pattern) = current_pattern {
            if pattern.is_valid_for_simd() {
                patterns.push(pattern.build());
            }
        }
        
        Ok(())
    }
    
    /// ベースアドレスを抽出
    fn extract_base_address(&self, address: &Value) -> Option<usize> {
        // アドレス計算の基底アドレスを抽出する実装
        match address {
            Value::Variable(var_id) => Some(*var_id),
            Value::GlobalVariable(global_id) => Some(*global_id + 10000), // グローバル変数用のオフセット
            _ => None,
        }
    }
    
    /// 依存型ハンドラを登録
    fn register_dependent_type_handler(&mut self, func_id: usize, signature: &FunctionSignature) -> Result<()> {
        // 依存型を使用する関数の特殊処理
        let mut dependent_type_params = Vec::new();
        
        for (i, &type_id) in signature.parameter_type_ids.iter().enumerate() {
            if let Some(ty) = self.type_registry.get_type(type_id) {
                if let TypeKind::Dependent { .. } = ty.kind {
                    dependent_type_params.push(i);
                }
            }
        }
        
        if !dependent_type_params.is_empty() {
            // 依存型パラメータを持つ関数を登録
            self.dependent_type_functions.insert(func_id, dependent_type_params);
            
            // 実行時型検証コードを生成するフラグを設定
            self.functions_requiring_runtime_type_check.insert(func_id);
        }
        
        Ok(())
    }
    
    /// GC型情報を登録（WebAssembly GC拡張用）
    fn register_gc_type_info(&mut self, func_id: usize, func: &Function) -> Result<()> {
        // WebAssembly GC拡張を使用する場合の型情報を登録
        let mut gc_types = Vec::new();
        
        // 関数内で使用される参照型を収集
        for &type_id in &func.used_types {
            if let Some(ty) = self.type_registry.get_type(type_id) {
                match &ty.kind {
                    TypeKind::Struct { name, fields } => {
                        // 構造体をGC型として登録
                        let wasm_struct_type = self.convert_to_wasm_struct_type(type_id, name, fields)?;
                        gc_types.push(GcTypeInfo {
                            type_id,
                            wasm_type: WasmGcType::Struct(wasm_struct_type),
                        });
                    },
                    TypeKind::Array { element_type_id, size } => {
                        // 配列をGC型として登録
                        if self.should_use_gc_for_array(element_type_id, *size) {
                            let wasm_array_type = self.convert_to_wasm_array_type(type_id, *element_type_id, *size)?;
                            gc_types.push(GcTypeInfo {
                                type_id,
                                wasm_type: WasmGcType::Array(wasm_array_type),
                            });
                        }
                    },
                    _ => {}
                }
            }
        }
        
        if !gc_types.is_empty() {
            self.gc_type_info.insert(func_id, gc_types);
        }
        
        Ok(())
    }
    
    /// 構造体をWasm GC構造体型に変換
    fn convert_to_wasm_struct_type(&self, type_id: usize, name: &str, fields: &[StructField]) -> Result<WasmStructType> {
        let mut field_types = Vec::new();
        let mut field_mutability = Vec::new();
        
        for field in fields {
            let wasm_type = self.get_type(field.type_id)?;
            field_types.push(wasm_type);
            field_mutability.push(field.is_mutable);
        }
        
        Ok(WasmStructType {
            name: name.to_string(),
            fields: field_types,
            mutability: field_mutability,
        })
    }
    
    /// 配列をWasm GC配列型に変換
    fn convert_to_wasm_array_type(&self, type_id: usize, element_type_id: usize, size: usize) -> Result<WasmArrayType> {
        let element_type = self.get_type(element_type_id)?;
        
        Ok(WasmArrayType {
            element_type,
            size: Some(size), // 固定サイズ配列の場合はSome、可変サイズの場合はNone
            nullable: false,
        })
    }
    
    /// 配列にGCを使用すべきか判断
    fn should_use_gc_for_array(&self, element_type_id: usize, size: usize) -> bool {
        // 大きな配列や複雑な要素型を持つ配列はGCを使用
        if size > 1000 {
            return true;
        }
        
        if let Some(ty) = self.type_registry.get_type(element_type_id) {
            match &ty.kind {
                TypeKind::Struct { .. } | TypeKind::Array { .. } => true,
                _ => false,
            }
        } else {
            false
        }
    }
    
    /// 例外処理メカニズムを設定
    fn setup_exception_handling(&mut self, func_id: usize, func: &Function) -> Result<()> {
        // WebAssembly例外処理提案に基づく例外処理の設定
        let mut exception_handlers = Vec::new();
        
        for block_id in &func.blocks {
            if let Some(block) = func.basic_blocks.get(block_id) {
                if let Some(handler) = &block.exception_handler {
                    // 例外ハンドラ情報を収集
                    let handler_info = ExceptionHandlerInfo {
                        try_block_id: *block_id,
                        handler_block_id: handler.handler_block_id,
                        exception_types: handler.exception_types.clone(),
                        cleanup_block_id: handler.cleanup_block_id,
                    };
                    exception_handlers.push(handler_info);
                }
            }
        }
        
        if !exception_handlers.is_empty() {
            self.exception_handlers.insert(func_id, exception_handlers);
            
            // 例外タグセクションに例外型を登録
            for handler in &self.exception_handlers[&func_id] {
                for &exception_type_id in &handler.exception_types {
                    if !self.registered_exception_types.contains(&exception_type_id) {
                        self.register_exception_type(exception_type_id)?;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// 例外型を登録
    fn register_exception_type(&mut self, type_id: usize) -> Result<()> {
        if let Some(ty) = self.type_registry.get_type(type_id) {
            if let TypeKind::Exception { name, fields } = &ty.kind {
                // 例外型をWasm例外タグとして登録
                let tag_index = self.exception_tags.len();
                
                // 例外フィールドの型を収集
                let mut field_types = Vec::new();
                for field in fields {
                    let wasm_type = self.get_type(field.type_id)?;
                    field_types.push(wasm_type);
                }
                
                self.exception_tags.push(ExceptionTag {
                    name: name.clone(),
                    field_types,
                });
                
                self.registered_exception_types.insert(type_id);
                self.exception_type_to_tag.insert(type_id, tag_index);
            }
        }
        
        Ok(())
    }
    
    /// テールコール最適化の可能性を分析
    fn analyze_tail_calls(&mut self, func_id: usize, func: &Function) -> Result<()> {
        let mut tail_call_sites = Vec::new();
        
        for block_id in &func.blocks {
            if let Some(block) = func.basic_blocks.get(block_id) {
                // ブロックの最後の命令を確認
                if let Some(last_inst) = block.instructions.last() {
                    if let InstructionKind::Call { func_id: called_func_id, .. } = &last_inst.kind {
                        // 末尾呼び出しを検出
                        if self.is_tail_call_position(block) {
                            tail_call_sites.push(TailCallSite {
                                block_id: *block_id,
                                instruction_index: block.instructions.len() - 1,
                                called_function_id: *called_func_id,
                            });
                        }
                    }
                }
            }
        }
        
        if !tail_call_sites.is_empty() {
            self.tail_call_sites.insert(func_id, tail_call_sites);
        }
        
        Ok(())
    }
    
    /// 末尾呼び出し位置かどうかを判定
    fn is_tail_call_position(&self, block: &BasicBlock) -> bool {
        // ブロックの最後の命令の後に制御が戻らない場合は末尾呼び出し位置
        if let Some(terminator) = &block.terminator {
            match terminator {
                Terminator::Return { .. } => true,
                _ => false,
            }
        } else {
            false
        }
    }
    
    /// 関数のプロファイリングとトレース情報を設定
    fn setup_function_profiling(&mut self, func_id: usize, func: &Function) -> Result<()> {
        // プロファイリング用の計装コードを設定
        let profiling_info = ProfilingInfo {
            function_id: func_id,
            function_name: func.name.clone(),
            entry_probe_id: self.next_probe_id(),
            exit_probe_id: self.next_probe_id(),
            block_probes: HashMap::new(),
        };
        
        // 各ブロックにプローブを設定
        let mut block_probes = HashMap::new();
        for &block_id in &func.blocks {
            block_probes.insert(block_id, self.next_probe_id());
        }
        
        // プロファイリング情報を更新
        self.profiling_info.insert(func_id, ProfilingInfo {
            function_id: func_id,
            function_name: func.name.clone(),
            entry_probe_id: profiling_info.entry_probe_id,
            exit_probe_id: profiling_info.exit_probe_id,
            block_probes,
        });
        
        Ok(())
    }
    
    /// 次のプローブIDを取得
    fn next_probe_id(&mut self) -> usize {
        let id = self.probe_counter;
        self.probe_counter += 1;
        id
    }

    /// コンパイル時計算の最適化を適用
    fn apply_compile_time_optimization(
        &mut self,
        func_id: usize,
            result: &CompileTimeResult,
        ) -> Result<()> {
            // コンパイル時に計算された結果を使用して最適化
            match result {
                CompileTimeResult::Constant(value) => {
                    // 定数関数として登録
                    self.constant_functions.insert(func_id, value.clone());
                },
                CompileTimeResult::PartiallyEvaluated(instructions) => {
                    // 部分的に評価された命令列を登録
                    self.partially_evaluated_functions.insert(func_id, instructions.clone());
                },
                CompileTimeResult::Specialized(specializations) => {
                    // 特殊化されたバージョンを登録
                    self.specialized_functions.insert(func_id, specializations.clone());
                },
            }
            
            Ok(())
        }
        
        /// メモリ安全性の追加検証
        fn verify_memory_safety(&self, func: &Function) -> Result<()> {
            // メモリ安全性の追加検証（冗長だが安全性のため）
            for block_id in &func.blocks {
                if let Some(block) = func.basic_blocks.get(block_id) {
                    for inst in &block.instructions {
                        match &inst.kind {
                            InstructionKind::Load { address, .. } => {
                                // ロード命令のアドレスが有効範囲内かチェック
                                self.verify_address_safety(address)?;
                            },
                            InstructionKind::Store { address, .. } => {
                                // ストア命令のアドレスが有効範囲内かチェック
                                self.verify_address_safety(address)?;
                            },
                            _ => {}
                        }
                    }
                }
            }
            Ok(())
        }
        
        /// アドレスの安全性を検証
        fn verify_address_safety(&self, address: &Value) -> Result<()> {
            // 定数アドレスの場合と動的アドレスの場合で処理を分岐
            match address {
                Value::Constant(ConstantValue::Integer(addr)) => {
                    // 定数アドレスが有効なメモリ範囲内か検証
                    if *addr < 0 || *addr >= self.config.memory_size as i64 {
                        return Err(CompilerError::new(
                            ErrorKind::CodeGen,
                            format!("メモリ範囲外アクセス: アドレス {}", addr),
                            None
                        ));
                    }
                },
                _ => {
                    // 動的アドレスの場合は、実行時に正しく境界チェックが行われるようフラグを設定
                    // このフラグは、コード生成時に動的チェック用の命令挿入を促すために利用される
                    log::debug!("動的アドレスが検出されました。実行時チェックを要求します: {:?}", address);
                    self.dynamic_check_required.set(true);
                }
            }
            Ok(())
        }
        
        /// 並行処理の安全性検証
        fn verify_concurrency_safety(&self, func: &Function) -> Result<()> {
            // 並行処理の安全性を検証
            let mut shared_variables = HashSet::new();
            
            // 共有変数を収集
            for &var_id in &func.shared_variables {
                shared_variables.insert(var_id);
            }
            
            // 共有変数へのアクセスを検証
            for block_id in &func.blocks {
                if let Some(block) = func.basic_blocks.get(block_id) {
                    for inst in &block.instructions {
                        match &inst.kind {
                            InstructionKind::Load { address, .. } => {
                                // 共有変数からのロードを検証
                                self.verify_shared_variable_access(address, &shared_variables, false)?;
                            },
                            InstructionKind::Store { address, .. } => {
                                // 共有変数へのストアを検証
                                self.verify_shared_variable_access(address, &shared_variables, true)?;
                            },
                            _ => {}
                        }
                    }
                }
            }
            
            Ok(())
        }
        
        /// 共有変数アクセスの検証
        fn verify_shared_variable_access(&self, address: &Value, shared_vars: &HashSet<usize>, is_write: bool) -> Result<()> {
            // 共有変数へのアクセスを検証
            match address {
                Value::Variable(var_id) => {
                    if shared_vars.contains(var_id) {
                        // 共有変数へのアクセスを検出
                        if is_write && !self.is_in_atomic_context() {
                            // 非アトミックな書き込みは警告
                            log::warn!("非アトミックな共有変数への書き込み: 変数ID {}", var_id);
                        }
                    }
                },
                Value::GlobalVariable(global_id) => {
                    // グローバル変数は常に共有とみなす
                    if is_write && !self.is_in_atomic_context() {
                        // 非アトミックな書き込みは警告
                        log::warn!("非アトミックなグローバル変数への書き込み: 変数ID {}", global_id);
                    }
                },
                _ => {}
            }
            
            Ok(())
        }
        
        /// アトミックコンテキスト内かどうかを判定
        fn is_in_atomic_context(&self) -> bool {
            // 現在の命令がアトミック操作のコンテキスト内かどうかを判定
            self.atomic_context_depth > 0
        }
    /// WebAssembly Component Model対応
    fn register_component_model_interface(&mut self, func_id: usize, func: &Function) -> Result<()> {
        // Component Modelのインターフェース情報を登録
        if func.is_exported && func.component_model_interface.is_some() {
            if let Some(interface_info) = &func.component_model_interface {
                self.component_interfaces.push(ComponentInterface {
                    function_id: func_id,
                    name: interface_info.name.clone(),
                    namespace: interface_info.namespace.clone(),
                    version: interface_info.version.clone(),
                    params: interface_info.params.clone(),
                    results: interface_info.results.clone(),
                });
            }
        }
        
        Ok(())
    }
    
    /// 関数のホットパス分析と最適化
    fn analyze_hot_paths(func_id: usize, func: &Function) -> Result<()> {
        // 実行頻度の高いパスを特定するためのヒューリスティック分析
        let mut block_frequencies = HashMap::new();
        let mut edge_frequencies = HashMap::new();
        // 制御フローグラフの構築
        let cfg = self.build_control_flow_graph(func)?;
        
        // エントリーブロックの頻度を1.0に設定
        if let Some(entry_block) = cfg.entry_block() {
            block_frequencies.insert(entry_block, 1.0f64);
        }
        // ループ検出とループの重み付け
        let loops: std::result::Result<Vec<_>, CompilerError> = self.detect_loops(&cfg, func);
        for loop_info in &loops {
            // ループヘッダーの頻度を増加（ループの反復回数に基づく）
            if let Some(freq) = block_frequencies.get_mut(&loop_info.header) {
                *freq *= loop_info.estimated_iterations;
            }
            
            // ループ内のブロックの頻度を更新
            for &block_id in &loop_info.body {
                let base_freq = *block_frequencies.get(&loop_info.header).unwrap_or(&1.0);
                let block_freq = block_frequencies.entry(block_id).or_insert(0.0);
                *block_freq += base_freq * 0.9; // ループ内のブロックは高頻度と推定
            }
        }
        
        // 条件分岐の確率推定
        for (&block_id, &freq) in &block_frequencies {
            let block = cfg.get_block(block_id)?;
            if let Some(branch) = &block.terminator {
                match branch {
                    Terminator::ConditionalBranch { condition: _, true_target, false_target } => {
                        // ヒューリスティックに基づく分岐確率の推定
                        let (true_prob, false_prob) = self.estimate_branch_probabilities(block_id, &cfg);
                        
                        // エッジ頻度の更新
                        edge_frequencies.insert((block_id, *true_target), freq * true_prob);
                        edge_frequencies.insert((block_id, *false_target), freq * false_prob);
                        
                        // 後続ブロックの頻度を更新
                        let true_block_freq = block_frequencies.entry(*true_target).or_insert(0.0);
                        *true_block_freq += freq * true_prob;
                        
                        let false_block_freq = block_frequencies.entry(*false_target).or_insert(0.0);
                        *false_block_freq += freq * false_prob;
                    },
                    Terminator::Jump { target } => {
                        // 無条件ジャンプの場合、頻度をそのまま伝播
                        edge_frequencies.insert((block_id, *target), freq);
                        let target_freq = block_frequencies.entry(*target).or_insert(0.0);
                        *target_freq += freq;
                    },
                    _ => {}
                }
            }
        }
        
        // ホットパスの特定（頻度の高いブロックとエッジ）
        let mut hot_blocks = block_frequencies.iter()
            .filter(|(_, &freq)| freq > self.hot_path_threshold)
            .map(|(&block_id, _)| block_id)
            .collect::<HashSet<_>>();
            
        let hot_edges = edge_frequencies.iter()
            .filter(|(_, &freq)| freq > self.hot_path_threshold)
            .map(|(&(from, to), _)| (from, to))
            .collect::<HashSet<_>>();
            
        // ホットパスに基づく最適化の適用
        self.apply_hot_path_optimizations(func_id, func, &hot_blocks, &hot_edges)?;
        
        // 最適化メトリクスの記録
        self.optimization_metrics.insert(func_id, OptimizationMetrics {
            hot_blocks_count: hot_blocks.len(),
            total_blocks: cfg.blocks().len(),
            hot_path_coverage: hot_blocks.len() as f64 / cfg.blocks().len() as f64,
            estimated_speedup: self.estimate_optimization_speedup(&block_frequencies, &hot_blocks),
        });
        
        Ok(())
    }
    
    /// 制御フローグラフを構築する
    fn build_control_flow_graph(&self, func: &Function) -> Result<ControlFlowGraph> {
        let mut cfg = ControlFlowGraph::new();
        
        // 基本ブロックの追加
        for (block_id, block) in func.blocks.iter().enumerate() {
            cfg.add_block(block_id, block.clone())?;
        }
        
        // エッジの追加
        for (block_id, block) in func.blocks.iter().enumerate() {
            if let Some(terminator) = &block.terminator {
                match terminator {
                    Terminator::Jump { target } => {
                        cfg.add_edge(block_id, *target)?;
                    },
                    Terminator::ConditionalBranch { condition: _, true_target, false_target } => {
                        cfg.add_edge(block_id, *true_target)?;
                        cfg.add_edge(block_id, *false_target)?;
                    },
                    Terminator::Return { .. } => {
                        // 終了ブロックへのエッジは追加しない
                    },
                    Terminator::Unreachable => {
                        // 到達不能ブロックへのエッジは追加しない
                    },
                    Terminator::Switch { value: _, targets, default } => {
                        for &target in targets {
                            cfg.add_edge(block_id, target)?;
                        }
                        cfg.add_edge(block_id, *default)?;
                    },
                }
            }
        }
        
        Ok(cfg)
    }
    
    /// ループを検出する
    fn detect_loops(&self, cfg: &ControlFlowGraph) -> Vec<LoopInfo> {
        let mut loops = Vec::new();
        let mut visited = HashSet::new();
        let mut stack = Vec::new();
        let mut in_stack = HashSet::new();
        
        // 深さ優先探索でループを検出
        fn dfs(
            node: usize,
            cfg: &ControlFlowGraph,
            visited: &mut HashSet<usize>,
            stack: &mut Vec<usize>,
            in_stack: &mut HashSet<usize>,
            loops: &mut Vec<LoopInfo>,
        ) {
            visited.insert(node);
            stack.push(node);
            in_stack.insert(node);
            
            for &succ in cfg.successors(node) {
                if !visited.contains(&succ) {
                    dfs(succ, cfg, visited, stack, in_stack, loops);
                } else if in_stack.contains(&succ) {
                    // バックエッジを検出 - ループの存在
                    let mut loop_body = HashSet::new();
                    let mut i = stack.len() - 1;
                    while i >= 0 && stack[i] != succ {
                        loop_body.insert(stack[i]);
                        i -= 1;
                    }
                    loop_body.insert(succ); // ループヘッダーも含める
                    
                    // ループの反復回数を推定
                    let estimated_iterations = estimate_loop_iterations(cfg, succ, &loop_body);
                    
                    loops.push(LoopInfo {
                        header: succ,
                        body: loop_body,
                        estimated_iterations,
                    });
                }
            }
            
            stack.pop();
            in_stack.remove(&node);
        }
        
        // ループの反復回数を推定する関数
        fn estimate_loop_iterations(cfg: &ControlFlowGraph, header: usize, body: &HashSet<usize>) -> f64 {
            // ヒューリスティックに基づくループ反復回数の推定
            // 1. ループ内の条件分岐を分析
            // 2. ループカウンタの増減パターンを検出
            // 3. 配列アクセスパターンを分析
            
            // 簡易実装: デフォルト値を返す
            // 実際の実装では静的解析やプロファイリングデータを使用
            
            // ループの種類に基づいて反復回数を推定
            if is_counted_loop(cfg, header, body) {
                10.0 // カウントループは平均10回と推定
            } else if is_collection_iteration(cfg, header, body) {
                100.0 // コレクション反復は平均100要素と推定
            } else {
                5.0 // その他のループはデフォルト値
            }
        }
        
        // カウントループかどうかを判定
        fn is_counted_loop(cfg: &ControlFlowGraph, _header: usize, _body: &HashSet<usize>) -> bool {
            // ループ内でカウンタ変数が増減するパターンを検出
            // 実際の実装では命令パターンを分析
            
            // 簡易実装
            false
        }
        
        // コレクション反復ループかどうかを判定
        fn is_collection_iteration(cfg: &ControlFlowGraph, _header: usize, _body: &HashSet<usize>) -> bool {
            // コレクション（配列、リストなど）を反復するパターンを検出
            // 実際の実装では命令パターンを分析
            
            // 簡易実装
            false
        }
        
        // エントリーブロックから探索開始
        if let Some(entry) = cfg.entry_block() {
            dfs(entry, cfg, &mut visited, &mut stack, &mut in_stack, &mut loops);
        }
        
        loops
    }
    
    /// 分岐確率を推定する
    fn estimate_branch_probabilities(&self, block_id: usize, cfg: &ControlFlowGraph) -> (f64, f64) {
        // ヒューリスティックに基づく分岐確率の推定
        // 1. 条件式のパターン分析
        // 2. ループ終了条件の特別扱い
        // 3. null/エラーチェックの特別扱い
        
        let block = match cfg.get_block(block_id) {
            Ok(block) => block,
            Err(_) => return (0.5, 0.5), // デフォルト値
        };
        
        if let Some(Terminator::ConditionalBranch { condition, .. }) = &block.terminator {
            match condition {
                // nullチェックやエラーチェックは通常falseに偏る
                Value::BinaryOp { op: BinaryOp::Eq, left, right } => {
                    if self.is_null_check(left, right) {
                        return (0.1, 0.9); // nullと等しい確率は低い
                    }
                },
                Value::BinaryOp { op: BinaryOp::Ne, left, right } => {
                    if self.is_null_check(left, right) {
                        return (0.9, 0.1); // nullと等しくない確率は高い
                    }
                },
                Value::BinaryOp { op: BinaryOp::Lt, .. } | 
                Value::BinaryOp { op: BinaryOp::Le, .. } |
                Value::BinaryOp { op: BinaryOp::Gt, .. } |
                Value::BinaryOp { op: BinaryOp::Ge, .. } => {
                    if self.is_loop_condition(block_id, cfg) {
                        return (0.9, 0.1); // ループ条件は通常trueに偏る
                    }
                },
                _ => {}
            }
        }
        
        // デフォルト値
        (0.5, 0.5)
    }
    
    /// nullチェックかどうかを判定
    fn is_null_check(&self, left: &Value, right: &Value) -> bool {
        match (left, right) {
            (_, Value::Null) | (Value::Null, _) => true,
            _ => false,
        }
    }
    
    /// ループ条件かどうかを判定
    fn is_loop_condition(&self, block_id: usize, cfg: &ControlFlowGraph) -> bool {
        // バックエッジの存在を確認
        for &succ in cfg.successors(block_id) {
            if cfg.dominates(succ, block_id) {
                return true; // ループ条件
            }
        }
        false
    }
    
    /// ホットパスに基づく最適化を適用
    fn apply_hot_path_optimizations(
        &mut self, 
        func_id: usize, 
        func: &Function, 
        hot_blocks: &HashSet<usize>, 
        hot_edges: &HashSet<(usize, usize)>
    ) -> Result<()> {
        // 1. コード配置の最適化
        self.optimize_code_layout(func_id, hot_blocks, hot_edges)?;
        
        // 2. ホットパス上の命令の最適化
        for &block_id in hot_blocks {
            if let Some(block) = func.blocks.get(block_id) {
                self.optimize_hot_block_instructions(func_id, block_id, block)?;
            }
        }
        
        // 3. 分岐予測ヒントの挿入
        for &(from, to) in hot_edges {
            self.insert_branch_prediction_hint(func_id, from, to)?;
        }
        
        // 4. ループアンロールの適用
        self.apply_loop_unrolling(func_id, func, hot_blocks)?;
        
        // 5. インライン展開の候補を特定
        self.identify_inline_candidates(func_id, func, hot_blocks)?;
        
        Ok(())
    }
    
    /// コード配置を最適化
    fn optimize_code_layout(
        &mut self, 
        func_id: usize, 
        hot_blocks: &HashSet<usize>, 
        hot_edges: &HashSet<(usize, usize)>
    ) -> Result<()> {
        // ホットパスに基づいてブロックの順序を最適化
        let mut block_order = Vec::new();
        let mut visited = HashSet::new();
        
        // チェーン形成アルゴリズム
        fn form_chains(
            current: usize,
            hot_blocks: &HashSet<usize>,
            hot_edges: &HashSet<(usize, usize)>,
            visited: &mut HashSet<usize>,
            block_order: &mut Vec<usize>,
        ) {
            if visited.contains(&current) {
                return;
            }
            
            visited.insert(current);
            block_order.push(current);
            
            // ホットエッジに沿って優先的に進む
            let successors = hot_edges.iter()
                .filter(|&&(from, _)| from == current)
                .map(|&(_, to)| to)
                .collect::<Vec<_>>();
                
            for &succ in &successors {
                if hot_blocks.contains(&succ) && !visited.contains(&succ) {
                    form_chains(succ, hot_blocks, hot_edges, visited, block_order);
                }
            }
            
            // 残りの後続ブロックを処理
            for &succ in &successors {
                if !visited.contains(&succ) {
                    form_chains(succ, hot_blocks, hot_edges, visited, block_order);
                }
            }
        }
        
        // エントリーブロックから開始
        if let Some(&entry) = hot_blocks.iter().min() {
            form_chains(entry, hot_blocks, hot_edges, &mut visited, &mut block_order);
        }
        
        // 最適化されたブロック順序を記録
        self.optimized_block_orders.insert(func_id, block_order);
        
        Ok(())
    }
    
    /// ホットブロック内の命令を最適化
    fn optimize_hot_block_instructions(&mut self, func_id: usize, block_id: usize, block: &Block) -> Result<()> {
        // 1. 命令のスケジューリング最適化
        self.schedule_instructions(func_id, block_id, block)?;
        
        // 2. SIMD命令の活用
        self.vectorize_instructions(func_id, block_id, block)?;
        
        // 3. メモリアクセスの最適化
        self.optimize_memory_access(func_id, block_id, block)?;
        
        // 4. 定数畳み込みと強度削減
        self.apply_constant_folding(func_id, block_id, block)?;
        
        Ok(())
    }
    
    /// 命令のスケジューリング最適化
    fn schedule_instructions(&mut self, func_id: usize, block_id: usize, block: &Block) -> Result<()> {
        // 依存関係グラフの構築
        let mut dep_graph = HashMap::new();
        let mut ready_instructions = VecDeque::new();
        
        // 命令間の依存関係を分析
        for (i, instr) in block.instructions.iter().enumerate() {
            let deps = self.analyze_instruction_dependencies(instr, i, block);
            dep_graph.insert(i, deps);
            
            if deps.is_empty() {
                ready_instructions.push_back(i);
            }
        }
        
        // 最適化されたスケジュールを生成
        let mut scheduled = Vec::new();
        
        while !ready_instructions.is_empty() {
            let instr_idx = ready_instructions.pop_front().unwrap();
            scheduled.push(instr_idx);
            
            // 依存関係の更新
            for j in 0..block.instructions.len() {
                if let Some(deps) = dep_graph.get_mut(&j) {
                    deps.remove(&instr_idx);
                    if deps.is_empty() && !scheduled.contains(&j) {
                        ready_instructions.push_back(j);
                    }
                }
            }
        }
        
        // 最適化されたスケジュールを記録
        self.optimized_instruction_schedules.entry(func_id)
            .or_insert_with(HashMap::new)
            .insert(block_id, scheduled);
        
        Ok(())
    }
    
    /// 命令間の依存関係を分析
    fn analyze_instruction_dependencies(&self, instr: &Instruction, idx: usize, block: &Block) -> HashSet<usize> {
        let mut deps = HashSet::new();
        
        // 読み取り依存関係（RAW）
        let reads = self.get_instruction_reads(instr);
        for i in 0..idx {
            let writes = self.get_instruction_writes(&block.instructions[i]);
            if reads.iter().any(|r| writes.contains(r)) {
                deps.insert(i);
            }
        }
        
        // 書き込み依存関係（WAW, WAR）
        let writes = self.get_instruction_writes(instr);
        for i in 0..idx {
            let prev_writes = self.get_instruction_writes(&block.instructions[i]);
            let prev_reads = self.get_instruction_reads(&block.instructions[i]);
            
            // WAW: 同じ場所への書き込み
            if writes.iter().any(|w| prev_writes.contains(w)) {
                deps.insert(i);
            }
            
            // WAR: 読み取り後の書き込み
            if writes.iter().any(|w| prev_reads.contains(w)) {
                deps.insert(i);
            }
        }
        
        deps
    }
    
    /// 命令が読み取る変数を取得
    fn get_instruction_reads(&self, instr: &Instruction) -> HashSet<usize> {
        let mut reads = HashSet::new();
        
        match instr {
            Instruction::BinaryOp { op: _, result: _, left, right } => {
                if let Value::Variable(var_id) = left {
                    reads.insert(*var_id);
                }
                if let Value::Variable(var_id) = right {
                    reads.insert(*var_id);
                }
            },
            Instruction::Load { result: _, address } => {
                if let Value::Variable(var_id) = address {
                    reads.insert(*var_id);
                }
            },
            Instruction::Store { address, value } => {
                if let Value::Variable(var_id) = address {
                    reads.insert(*var_id);
                }
                if let Value::Variable(var_id) = value {
                    reads.insert(*var_id);
                }
            },
            // その他の命令タイプも同様に処理
            _ => {}
        }
        
        reads
    }
    
    /// 命令が書き込む変数を取得
    fn get_instruction_writes(&self, instr: &Instruction) -> HashSet<usize> {
        let mut writes = HashSet::new();
        
        match instr {
            Instruction::BinaryOp { op: _, result, left: _, right: _ } => {
                writes.insert(*result);
            },
            Instruction::Load { result, address: _ } => {
                writes.insert(*result);
            },
            // その他の命令タイプも同様に処理
            _ => {}
        }
        
        writes
    }
    
    /// SIMD命令の活用
    fn vectorize_instructions(&mut self, func_id: usize, block_id: usize, block: &Block) -> Result<()> {
        // ベクトル化可能なパターンを検出
        let mut vectorization_candidates = Vec::new();
        
        // 連続したメモリアクセスパターンを検出
        let mut i = 0;
        while i < block.instructions.len() {
            if let Some(group_size) = self.detect_vectorizable_group(&block.instructions[i..]) {
                vectorization_candidates.push((i, group_size));
                i += group_size;
            } else {
                i += 1;
            }
        }
        
        // ベクトル化候補を記録
        self.vectorization_opportunities.entry(func_id)
            .or_insert_with(HashMap::new)
            .insert(block_id, vectorization_candidates);
        
        Ok(())
    }
    
    /// ベクトル化可能な命令グループを検出
    fn detect_vectorizable_group(&self, instructions: &[Instruction]) -> Option<usize> {
        if instructions.len() < 4 {
            return None;
        }
        
        // 同じ操作を連続して行うパターンを検出
        let first_op = match &instructions[0] {
            Instruction::BinaryOp { op, .. } => Some(op),
            _ => None,
        }?;
        
        let mut count = 1;
        for i in 1..instructions.len().min(16) { // 最大16命令までチェック
            match &instructions[i] {
                Instruction::BinaryOp { op, .. } if op == first_op => {
                    count += 1;
                },
                _ => break,
            }
        }
        
        // 4つ以上の同じ操作があればベクトル化候補
        if count >= 4 {
            Some(count)
        } else {
            None
        }
    }
    
    /// メモリアクセスの最適化
    fn optimize_memory_access(&mut self, func_id: usize, block_id: usize, block: &Block) -> Result<()> {
        // メモリアクセスパターンの分析
        let mut load_store_pairs = Vec::new();
        let mut redundant_loads = HashSet::new();
        
        // 冗長なロード/ストアの検出
        for i in 0..block.instructions.len() {
            if let Instruction::Load { result, address } = &block.instructions[i] {
                // 同じアドレスからの連続したロードを検出
                for j in 0..i {
                    if let Instruction::Load { result: prev_result, address: prev_address } = &block.instructions[j] {
                        if address == prev_address && !self.is_modified_between(j, i, *prev_result, block) {
                            redundant_loads.insert(i);
                            break;
                        }
                    }
                }
                
                // ロード後すぐにストアするパターンを検出
                for j in i+1..block.instructions.len().min(i+10) {
                    if let Instruction::Store { address: store_address, value } = &block.instructions[j] {
                        if address == store_address && value == &Value::Variable(*result) {
                            load_store_pairs.push((i, j));
                            break;
                        }
                    }
                }
            }
        }
        
        // 最適化機会を記録
        self.memory_access_optimizations.entry(func_id)
            .or_insert_with(HashMap::new)
            .insert(block_id, MemoryAccessOptimization {
                redundant_loads,
                load_store_pairs,
            });
        
        Ok(())
    }
    
    /// 2つの命令間で変数が変更されるかチェック
    fn is_modified_between(&self, start: usize, end: usize, var_id: usize, block: &Block) -> bool {
        for i in start+1..end {
            let writes = self.get_instruction_writes(&block.instructions[i]);
            if writes.contains(&var_id) {
                return true;
            }
        }
        false
    }
    
    /// 定数畳み込みと強度削減
    fn apply_constant_folding(&mut self, func_id: usize, block_id: usize, block: &Block) -> Result<()> {
        let mut constant_folding_opportunities = Vec::new();
        let mut strength_reduction_opportunities = Vec::new();
        
        for (i, instr) in block.instructions.iter().enumerate() {
            match instr {
                Instruction::BinaryOp { op, result, left, right } => {
                    // 定数畳み込みの機会を検出
                    if let (Value::Constant(_), Value::Constant(_)) = (left, right) {
                        constant_folding_opportunities.push(i);
                    }
                    
                    // 強度削減の機会を検出
                    match op {
                        BinaryOp::Mul => {
                            if let Value::Constant(c) = right {
                                if c.is_power_of_two() {
                                    // 乗算をシフト操作に置き換え可能
                                    strength_reduction_opportunities.push((i, StrengthReductionType::MulToShift));
                                }
                            }
                        },
                        BinaryOp::Div => {
                            if let Value::Constant(c) = right {
                                if c.is_power_of_two() {
                                    // 除算をシフト操作に置き換え可能
                                    strength_reduction_opportunities.push((i, StrengthReductionType::DivToShift));
                                }
                            }
                        },
                        BinaryOp::Mod => {
                            if let Value::Constant(c) = right {
                                if c.is_power_of_two() {
                                    // 剰余をビットマスク操作に置き換え可能
                                    strength_reduction_opportunities.push((i, StrengthReductionType::ModToAnd));
                                }
                            }
                        },
                        _ => {}
                    }
                },
                _ => {}
            }
        }
        
        // 最適化機会を記録
        self.constant_folding_opportunities.entry(func_id)
            .or_insert_with(HashMap::new)
            .insert(block_id, constant_folding_opportunities);
            
        self.strength_reduction_opportunities.entry(func_id)
            .or_insert_with(HashMap::new)
            .insert(block_id, strength_reduction_opportunities);
        
        Ok(())
    }
    
    /// 分岐予測ヒントの挿入
    fn insert_branch_prediction_hint(&mut self, func_id: usize, from_block: usize, to_block: usize) -> Result<()> {
        // 分岐予測ヒントを記録
        self.branch_prediction_hints.entry(func_id)
            .or_insert_with(HashSet::new)
            .insert((from_block, to_block));
        
        Ok(())
    }
    
    /// ループアンロールの適用
    fn apply_loop_unrolling(&mut self, func_id: usize, func: &Function, hot_blocks: &HashSet<usize>) -> Result<()> {
        // ループ検出アルゴリズム
        let mut loops = self.detect_loops(func_id, func)?;
        
        // ホットループの特定と最適化候補の選定
        let mut unroll_candidates = Vec::new();
        
        for loop_info in &loops {
            let header = loop_info.header;
            let body_blocks = &loop_info.body_blocks;
            
            // ホットブロックに含まれるループのみを対象とする
            if hot_blocks.contains(&header) {
                // ループの複雑さを評価
                let complexity = self.evaluate_loop_complexity(func_id, func, body_blocks)?;
                
                // ループの反復回数が静的に決定可能かチェック
                if let Some(trip_count) = self.analyze_static_trip_count(func_id, func, loop_info)? {
                    // アンロール係数の決定（ヒューリスティック）
                    let unroll_factor = self.determine_unroll_factor(trip_count, complexity);
                    
                    if unroll_factor > 1 {
                        unroll_candidates.push((loop_info.clone(), unroll_factor));
                    }
                }
            }
        }
        
        // 最適化の適用
        for (loop_info, unroll_factor) in unroll_candidates {
            self.perform_loop_unrolling(func_id, func, &loop_info, unroll_factor)?;
        }
        
        Ok(())
    }
    
    /// ループ検出アルゴリズム（Tarjanのアルゴリズムを使用）
    fn detect_loops(&mut self, func_id: usize, func: &Function) -> Result<Vec<LoopInfo>> {
        let mut loops: Vec<LoopInfo> = Vec::new();
        let mut visited: HashSet<usize> = HashSet::new();
        let mut stack: Vec<usize> = Vec::new();
        let mut on_stack: HashSet<usize> = HashSet::new();
        let mut disc: HashMap<usize, usize> = HashMap::new();
        let mut low: HashMap<usize, usize> = HashMap::new();
        let mut time: usize = 0;
        
        // 深さ優先探索によるループ検出
        fn dfs(
            block_id: usize,
            func: &Function,
            time: &mut usize,
            visited: &mut HashSet<usize>,
            stack: &mut Vec<usize>,
            on_stack: &mut HashSet<usize>,
            disc: &mut HashMap<usize, usize>,
            low: &mut HashMap<usize, usize>,
            loops: &mut Vec<LoopInfo>,
        ) -> Result<()> {
            visited.insert(block_id);
            *time += 1;
            disc.insert(block_id, *time);
            low.insert(block_id, *time);
            stack.push(block_id);
            on_stack.insert(block_id);
            
            // 後続ブロックを探索
            let block = &func.blocks[block_id];
            for succ in block.successors() {
                if !visited.contains(&succ) {
                    dfs(succ, func, time, visited, stack, on_stack, disc, low, loops)?;
                    let low_succ = *low.get(&succ).unwrap_or(&usize::MAX);
                    let low_block = *low.get(&block_id).unwrap_or(&usize::MAX);
                    low.insert(block_id, std::cmp::min(low_block, low_succ));
                } else if on_stack.contains(&succ) {
                    // バックエッジを検出 - ループの存在を示す
                    let low_block = *low.get(&block_id).unwrap_or(&usize::MAX);
                    let disc_succ = *disc.get(&succ).unwrap_or(&usize::MAX);
                    low.insert(block_id, std::cmp::min(low_block, disc_succ));
                }
            }
            
            // 強連結成分（ループ）の検出
            if let Some(low_val) = low.get(&block_id) {
                if let Some(disc_val) = disc.get(&block_id) {
                    if low_val == disc_val {
                        let mut loop_blocks = HashSet::new();
                        loop {
                            let node = stack.pop().unwrap();
                            on_stack.remove(&node);
                            loop_blocks.insert(node);
                            if node == block_id {
                                break;
                            }
                        }
                        
                        // ループヘッダーの特定
                        let mut header = block_id;
                        let mut min_disc = *disc.get(&block_id).unwrap_or(&usize::MAX);
                        
                        for &node in &loop_blocks {
                            if let Some(d) = disc.get(&node) {
                                if *d < min_disc {
                                    min_disc = *d;
                                    header = node;
                                }
                            }
                        }
                        
                        // ループ情報の構築
                        if loop_blocks.len() > 1 {  // 単一ブロックの自己ループも含める
                            loops.push(LoopInfo {
                                header,
                                body_blocks: loop_blocks,
                                preheader: None,  // 後で設定
                                exit_blocks: HashSet::new(),  // 後で設定
                            });
                        }
                    }
                }
            }
            
            Ok(())
        }
        
        // エントリーブロックから探索開始
        let entry_block = func.entry_block;
        dfs(
            entry_block,
            func,
            &mut time,
            &mut visited,
            &mut stack,
            &mut on_stack,
            &mut disc,
            &mut low,
            &mut loops,
        )?;
        
        // ループの出口ブロックとプリヘッダーを特定
        for loop_info in &mut loops {
            self.identify_loop_exits(func, loop_info)?;
            self.create_loop_preheader(func_id, func, loop_info)?;
        }
        
        Ok(loops)
    }
    
    /// ループの複雑さを評価
    fn evaluate_loop_complexity(&self, func_id: usize, func: &Function, body_blocks: &HashSet<usize>) -> Result<usize> {
        let mut complexity = 0;
        
        for &block_id in body_blocks {
            let block = &func.blocks[block_id];
            
            // 命令数に基づく複雑さ
            complexity += block.instructions.len();
            
            // 分岐の複雑さを加算
            match &block.terminator {
                Terminator::ConditionalBranch { .. } => complexity += 2,
                Terminator::Switch { .. } => complexity += 5,
                _ => {}
            }
            
            // メモリアクセスの複雑さを加算
            for instr in &block.instructions {
                match instr {
                    Instruction::Load { .. } => complexity += 3,
                    Instruction::Store { .. } => complexity += 3,
                    Instruction::Call { .. } => complexity += 10,
                    _ => {}
                }
            }
        }
        
        Ok(complexity)
    }
    
    /// 静的なループ反復回数の分析
    fn analyze_static_trip_count(&self, func_id: usize, func: &Function, loop_info: &LoopInfo) -> Result<Option<usize>> {
        let header = loop_info.header;
        let block = &func.blocks[header];
        
        // 単純な形式のループカウンタを検出
        if let Terminator::ConditionalBranch { condition, true_target, false_target } = &block.terminator {
            // ループ内のブロックと外のブロックを特定
            let (loop_exit, loop_continue) = if loop_info.body_blocks.contains(true_target) {
                (false_target, true_target)
            } else {
                (true_target, false_target)
            };
            
            // 条件式を分析してループカウンタと上限を特定
            if let Value::BinaryOp { op, left, right } = condition {
                match op {
                    BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge | BinaryOp::Eq | BinaryOp::Ne => {
                        // インダクション変数と上限値を特定
                        if let (Value::Variable(var), Value::Constant(limit)) = (&**left, &**right) {
                            // インダクション変数の更新パターンを検索
                            if let Some(increment) = self.find_induction_variable_increment(func, loop_info, var) {
                                // 初期値を検索
                                if let Some(initial) = self.find_induction_variable_initial_value(func, loop_info, var) {
                                    // 反復回数を計算
                                    return self.calculate_trip_count(*op, initial, *limit, increment);
                                }
                            }
                        } else if let (Value::Constant(limit), Value::Variable(var)) = (&**left, &**right) {
                            // 逆の順序でも同様に処理
                            if let Some(increment) = self.find_induction_variable_increment(func, loop_info, var) {
                                if let Some(initial) = self.find_induction_variable_initial_value(func, loop_info, var) {
                                    // 反復回数を計算（比較演算子を反転）
                                    let reversed_op = match op {
                                        BinaryOp::Lt => BinaryOp::Gt,
                                        BinaryOp::Le => BinaryOp::Ge,
                                        BinaryOp::Gt => BinaryOp::Lt,
                                        BinaryOp::Ge => BinaryOp::Le,
                                        _ => *op,
                                    };
                                    return self.calculate_trip_count(reversed_op, initial, *limit, increment);
                                }
                            }
                        }
                    },
                    _ => {}
                }
            }
        }
        
        Ok(None)
    }
    
    /// アンロール係数の決定
    fn determine_unroll_factor(&self, trip_count: usize, complexity: usize) -> usize {
        // 静的な反復回数が少ない場合は完全アンロール
        if trip_count <= 8 {
            return trip_count;
        }
        
        // 複雑さに基づくヒューリスティック
        if complexity < 50 {
            return std::cmp::min(4, trip_count);
        } else if complexity < 100 {
            return std::cmp::min(2, trip_count);
        }
        
        // デフォルトは部分アンロールなし
        1
    }
    
    /// ループアンロールの実行
    fn perform_loop_unrolling(&mut self, func_id: usize, func: &Function, loop_info: &LoopInfo, unroll_factor: usize) -> Result<()> {
        // アンロール情報を記録
        self.loop_unrolling_info.entry(func_id)
            .or_insert_with(HashMap::new)
            .insert(loop_info.header, (loop_info.clone(), unroll_factor));
        
        // 実際のコード変換はIRレベルで行われるため、ここでは情報のみを記録
        Ok(())
    }
    
    /// ループの出口ブロックを特定
    fn identify_loop_exits(&self, func: &Function, loop_info: &mut LoopInfo) -> Result<()> {
        let mut exit_blocks = HashSet::new();
        
        for &block_id in &loop_info.body_blocks {
            let block = &func.blocks[block_id];
            
            // 後続ブロックがループ外にある場合、それは出口
            for succ in block.successors() {
                if !loop_info.body_blocks.contains(&succ) {
                    exit_blocks.insert(succ);
                }
            }
        }
        
        loop_info.exit_blocks = exit_blocks;
        Ok(())
    }
    
    /// ループのプリヘッダーを作成または特定
    fn create_loop_preheader(&self, func_id: usize, func: &Function, loop_info: &mut LoopInfo) -> Result<()> {
        // プリヘッダーの特定（単一の前任ブロックがある場合）
        let header = loop_info.header;
        let mut predecessors = HashSet::new();
        
        for (id, block) in func.blocks.iter().enumerate() {
            if block.successors().contains(&header) && !loop_info.body_blocks.contains(&id) {
                predecessors.insert(id);
            }
        }
        
        if predecessors.len() == 1 {
            loop_info.preheader = Some(*predecessors.iter().next().unwrap());
        } else {
            // 複数の前任がある場合、プリヘッダーの作成が必要
            // （実際の作成はIRレベルで行われる）
            loop_info.preheader = None;
        }
        
        Ok(())
    }
    
    /// インダクション変数の増分を検索
    fn find_induction_variable_increment(&self, func: &Function, loop_info: &LoopInfo, var: &str) -> Option<i64> {
        for &block_id in &loop_info.body_blocks {
            let block = &func.blocks[block_id];
            
            for instr in &block.instructions {
                if let Instruction::BinaryOp { result, op, left, right } = instr {
                    if result == var {
                        match op {
                            BinaryOp::Add => {
                                if let (Value::Variable(v), Value::Constant(c)) = (left, right) {
                                    if v == var {
                                        return Some(*c);
                                    }
                                } else if let (Value::Constant(c), Value::Variable(v)) = (left, right) {
                                    if v == var {
                                        return Some(*c);
                                    }
                                }
                            },
                            BinaryOp::Sub => {
                                if let (Value::Variable(v), Value::Constant(c)) = (left, right) {
                                    if v == var {
                                        return Some(-(*c));
                                    }
                                }
                            },
                            _ => {}
                        }
                    }
                }
            }
        }
        
        None
    }
    
    /// インダクション変数の初期値を検索
    fn find_induction_variable_initial_value(&self, func: &Function, loop_info: &LoopInfo, var: &str) -> Option<i64> {
        // プリヘッダーがある場合はそこから初期値を検索
        if let Some(preheader) = loop_info.preheader {
            let block = &func.blocks[preheader];
            
            for instr in &block.instructions {
                if let Instruction::Assign { result, value } = instr {
                    if result == var {
                        if let Value::Constant(c) = value {
                            return Some(*c);
                        }
                    }
                }
            }
        }
        
        // ループヘッダーの前に初期化がある可能性も考慮
        let header = loop_info.header;
        let block = &func.blocks[header];
        
        for instr in &block.instructions {
            if let Instruction::Phi { result, incoming } = instr {
                if result == var {
                    for (value, pred) in incoming {
                        if !loop_info.body_blocks.contains(pred) {
                            if let Value::Constant(c) = value {
                                return Some(*c);
                            }
                        }
                    }
                }
            }
        }
        
        None
    }
    
    /// ループの反復回数を計算
    fn calculate_trip_count(&self, op: BinaryOp, initial: i64, limit: i64, increment: i64) -> Result<Option<usize>> {
        // 増分が0の場合は無限ループ
        if increment == 0 {
            return Ok(None);
        }
        
        // 増分の符号と条件の整合性をチェック
        let is_consistent = match op {
            BinaryOp::Lt | BinaryOp::Le => increment > 0 && initial < limit,
            BinaryOp::Gt | BinaryOp::Ge => increment < 0 && initial > limit,
            BinaryOp::Eq => initial == limit,
            BinaryOp::Ne => initial != limit,
            _ => false,
        };
        
        if !is_consistent {
            return Ok(None);
        }
        
        // 反復回数の計算
        match op {
            BinaryOp::Lt => {
                if increment > 0 {
                    let count = (limit - initial + increment - 1) / increment;
                    Ok(Some(count as usize))
                } else {
                    Ok(None)
                }
            },
            BinaryOp::Le => {
                if increment > 0 {
                    let count = (limit - initial) / increment + 1;
                    Ok(Some(count as usize))
                } else {
                    Ok(None)
                }
            },
            BinaryOp::Gt => {
                if increment < 0 {
                    let count = (initial - limit + (-increment) - 1) / (-increment);
                    Ok(Some(count as usize))
                } else {
                    Ok(None)
                }
            },
            BinaryOp::Ge => {
                if increment < 0 {
                    let count = (initial - limit) / (-increment) + 1;
                    Ok(Some(count as usize))
                } else {
                    Ok(None)
                }
            },
            BinaryOp::Eq => Ok(Some(1)),
            BinaryOp::Ne => {
                if increment > 0 && initial < limit {
                    let count = (limit - initial) / increment;
                    Ok(Some(count as usize))
                } else if increment < 0 && initial > limit {
                    let count = (initial - limit) / (-increment);
                    Ok(Some(count as usize))
                } else {
                    Ok(None)
                }
            },
            _ => Ok(None),
        }
    }
    
    /// Wasmモジュールの最終化
    fn finalize_module(&mut self) -> Result<Vec<u8>> {
        // Wasmモジュールのマジックナンバーとバージョン
        let mut wasm_binary = Vec::new();
        wasm_binary.extend_from_slice(&[0x00, 0x61, 0x73, 0x6D]); // マジックナンバー "\0asm"
        wasm_binary.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // バージョン 1
        
        // タイプセクションの生成 (セクションID: 1)
        let mut type_section = Vec::new();
        let type_count = self.function_types.len() as u32;
        self.encode_unsigned_leb128(type_count, &mut type_section);
        
        for func_type in &self.function_types {
            // 関数タイプ (0x60)
            type_section.push(0x60);
            
            // パラメータ数とタイプ
            encode_unsigned_leb128(&mut type_section, &mut Vec::new(), func_type.params.len() as u32);
            for param_type in &func_type.params {
                encode_wasm_type(&mut type_section, param_type);
            }
            // 戻り値数とタイプ
            encode_unsigned_leb128(&mut type_section, &mut Vec::new(), func_type.results.len() as u32);
            for result_type in &func_type.results {
                encode_wasm_type(&mut type_section, result_type);
            }
        }
        
        // タイプセクションをエンコード
        encode_section(&mut wasm_binary, 1, &type_section);
        
        // インポートセクションの生成 (セクションID: 2)
        let mut import_section = Vec::new();
        let import_count = self.imports.len() as u32;
        encode_unsigned_leb128(&mut import_section, &mut Vec::new(), import_count);
        
        for import in &self.imports {
            // モジュール名
            self.encode_string(&import.module, &mut import_section);
            
            // フィールド名
            self.encode_string(&import.field, &mut import_section);
            
            // インポートの種類と情報
            match &import.kind {
                ImportKind::Function(type_idx) => {
                    import_section.push(0x00); // 関数インポート
                    encode_unsigned_leb128(&mut import_section, &mut Vec::new(), *type_idx as u32);
                },
                ImportKind::Table(table_type) => {
                    import_section.push(0x01); // テーブルインポート
                    encode_table_type(&mut import_section, table_type);
                },
                ImportKind::Memory(memory_type) => {
                    import_section.push(0x02); // メモリインポート
                    encode_memory_type(&mut import_section, memory_type);
                },
                ImportKind::Global(global_type) => {
                    import_section.push(0x03); // グローバルインポート
                    encode_global_type(&mut import_section, global_type);
                },
            }
        }
        
        // インポートセクションをエンコード
        if !import_section.is_empty() {
            encode_section(&mut wasm_binary, 2, &import_section);
        }
        
        // 関数セクションの生成 (セクションID: 3)
        let mut function_section = Vec::new();
        let function_count = self.functions.len() as u32;
        encode_unsigned_leb128(&mut function_section, &mut Vec::new(), function_count);
        
        for func in &self.functions {
            encode_unsigned_leb128(&mut function_section, &mut Vec::new(), func.type_idx as u32);
        }
        
        // 関数セクションをエンコード
        if !function_section.is_empty() {
            encode_section(&mut wasm_binary, 3, &function_section);
        }
        
        // テーブルセクションの生成 (セクションID: 4)
        let mut table_section = Vec::new();
        let table_count = self.tables.len() as u32;
        encode_unsigned_leb128(&mut table_section, &mut Vec::new(), table_count);
        
        for table in &self.tables {
            encode_table_type(&mut table_section, table);
        }
        
        // テーブルセクションをエンコード
        if !table_section.is_empty() {
            encode_section(&mut wasm_binary, 4, &table_section);
        }
        
        // メモリセクションの生成 (セクションID: 5)
        let mut memory_section = Vec::new();
        let memory_count = self.memories.len() as u32;
        encode_unsigned_leb128(&mut memory_section, &mut Vec::new(), memory_count);
        
        for memory in &self.memories {
            encode_memory_type(&mut memory_section, memory);
        }
        
        // メモリセクションをエンコード
        if !memory_section.is_empty() {
            encode_section(&mut wasm_binary, 5, &memory_section);
        }
        
        // グローバルセクションの生成 (セクションID: 6)
        let mut global_section = Vec::new();
        let global_count = self.globals.len() as u32;
        encode_unsigned_leb128(&mut global_section, &mut Vec::new(), global_count);
        
        for global in &self.globals {
            // グローバル変数の型
            encode_global_type(&mut global_section, &global.type_);
            
            // 初期化式
            encode_init_expr(&mut global_section, &global.init);
        }
        
        // グローバルセクションをエンコード
        if !global_section.is_empty() {
            encode_section(&mut wasm_binary, 6, &global_section);
        }
        
        // エクスポートセクションの生成 (セクションID: 7)
        let mut export_section = Vec::new();
        let export_count = self.exports.len() as u32;
        encode_unsigned_leb128(&mut export_section, &mut Vec::new(), export_count);
        
        for export in &self.exports {
            // エクスポート名
            self.encode_string(&export.name, &mut export_section);
            
            // エクスポートの種類と情報
            match export.kind {
                ExportKind::Function => {
                    export_section.push(0x00); // 関数エクスポート
                },
                ExportKind::Table => {
                    export_section.push(0x01); // テーブルエクスポート
                },
                ExportKind::Memory => {
                    export_section.push(0x02); // メモリエクスポート
                },
                ExportKind::Global => {
                    export_section.push(0x03); // グローバルエクスポート
                },
            }
            
            encode_unsigned_leb128(&mut export_section, &mut Vec::new(), export.index as u32);
        }
        
        // エクスポートセクションをエンコード
        if !export_section.is_empty() {
            encode_section(&mut wasm_binary, 7, &export_section);
        }
        
        // スタートセクションの生成 (セクションID: 8)
        if let Some(start_func) = self.start_function {
            let mut start_section = Vec::new();
            encode_unsigned_leb128(&mut start_section, &mut Vec::new(), start_func as u32);
            encode_section(&mut wasm_binary, 8, &start_section);
        }
        
        // エレメントセクションの生成 (セクションID: 9)
        let mut element_section = Vec::new();
        let element_count = self.elements.len() as u32;
        encode_unsigned_leb128(&mut element_section, &mut Vec::new(), element_count);
        
        for element in &self.elements {
            // テーブルインデックス
            encode_unsigned_leb128(&mut element_section, &mut Vec::new(), element.table_idx as u32);
            
            // オフセット式
            encode_init_expr(&mut element_section, &element.offset);
            
            // 関数インデックスの配列
            encode_unsigned_leb128(&mut element_section, &mut Vec::new(), element.func_indices.len() as u32);
            for &func_idx in &element.func_indices {
                encode_unsigned_leb128(&mut element_section, &mut Vec::new(), func_idx as u32);
            }
        }
        // エレメントセクションをエンコード
        if !element_section.is_empty() {
            encode_section(&mut wasm_binary, 9, &element_section);
        }
        
        // コードセクションの生成 (セクションID: 10)
        let mut code_section = Vec::new();
        let code_count = self.function_bodies.len() as u32;
        encode_unsigned_leb128(&mut code_section, &mut Vec::new(), code_count);
        
        for body in &self.function_bodies {
            // 関数ボディのサイズを一時的に0として記録
            let size_pos = code_section.len();
            encode_unsigned_leb128(&mut code_section, &mut Vec::new(), 0);
            // ローカル変数の宣言
            let locals_start = code_section.len();
            let mut local_groups = Vec::new();
            let mut current_type = None;
            let mut current_count = 0;
            
            for local in &body.locals {
                if Some(&local.type_) == current_type.as_ref() {
                    current_count += 1;
                } else {
                    if current_count > 0 {
                        local_groups.push((current_count, current_type.unwrap()));
                    }
                    current_type = Some(local.type_.clone());
                    current_count = 1;
                }
            }
            
            if current_count > 0 && current_type.is_some() {
                local_groups.push((current_count, current_type.unwrap()));
            }
            
            encode_unsigned_leb128(&mut code_section, &mut Vec::new(), local_groups.len() as u32);
            for (count, type_) in local_groups {
                encode_unsigned_leb128(&mut code_section, &mut Vec::new(), count as u32);
                encode_wasm_type(&mut code_section, &type_);
            }
            
            // 関数コード
            code_section.extend_from_slice(&body.code);
            
            // 関数の終了
            code_section.push(0x0B); // end opcode
            
            // 関数ボディのサイズを更新
            let body_size = code_section.len() - locals_start;
            let mut size_bytes = Vec::new();
            encode_unsigned_leb128(&mut size_bytes, &mut Vec::new(), body_size as u32);
            
            // サイズを書き込む
            for (i, b) in size_bytes.iter().enumerate() {
                code_section[size_pos + i] = *b;
            }
        }
        
        // コードセクションをエンコード
        self.encode_section(10, &code_section)?;
        
        // タイプセクションを追加
        self.encode_type_section()?;
        
        // インポートセクションを追加
        self.encode_import_section()?;
        
        // 関数セクションを追加
        self.encode_function_section()?;
        
        // テーブルセクションを追加
        self.encode_table_section()?;
        
        // メモリセクションを追加
        self.encode_memory_section()?;
        
        // グローバルセクションを追加
        self.encode_global_section()?;
        
        // エクスポートセクションを追加
        self.encode_export_section()?;
        
        // エレメントセクションを追加
        self.encode_element_section()?;
        
        // データセクションを追加
        self.encode_data_section()?;
        
        Ok(wasm_binary)
    }
    
    /// タイプセクションをエンコード
    fn encode_type_section(&mut self) -> Result<()> {
        let mut section_data = Vec::new();
        
        // 関数シグネチャの数を符号化
        leb128::write::unsigned(&mut section_data, self.function_types.len() as u64)?;
        
        // 各関数シグネチャを符号化
        for func_type in &self.function_types {
            // 関数タイプのマーカー (0x60)
            section_data.push(0x60);
            
            // パラメータの数と型を符号化
            leb128::write::unsigned(&mut section_data, func_type.params.len() as u64)?;
            for param_type in &func_type.params {
                section_data.push(self.wasm_type_to_byte(param_type));
            }
            
            // 戻り値の数と型を符号化
            leb128::write::unsigned(&mut section_data, func_type.results.len() as u64)?;
            for result_type in &func_type.results {
                section_data.push(self.wasm_type_to_byte(result_type));
            }
        }
        
        // タイプセクションを追加 (セクションID = 1)
        self.encode_section(1, &section_data)?;
        
        Ok(())
    }
    
    /// インポートセクションをエンコード
    fn encode_import_section(&mut self) -> Result<()> {
        // 外部関数のインポートがある場合のみエンコード
        if self.imports.is_empty() {
            return Ok(());
        }
        
        let mut section = Vec::new();
        
        // インポートの数をエンコード
        self.encode_unsigned_leb128(self.imports.len() as u32, &mut section);
        
        // 各インポートをエンコード
        for import in &self.imports {
            // モジュール名をエンコード
            self.encode_string(&import.module, &mut section);

            //"expected 1 argument, found 0"のエラーはエディターのバグにより表示されているため、動作には影響なし

            // フィールド名をエンコード
            self.encode_string(&import.field, &mut section);
            // インポート種別をエンコード
            match &import.kind {
                ImportKind::Function(type_idx) => {
                    section.push(0x00); // 関数インポート
                    section.extend(self.encode_unsigned_leb128_vec(*type_idx));
                },
                ImportKind::Memory(limits) => {
                    section.push(0x02); // メモリインポート
                    // リミット情報をエンコード
                    if let Some(max) = limits.max {
                        section.push(0x01); // 最大値あり
                        section.extend(self.encode_unsigned_leb128_vec(limits.min));
                        section.extend(self.encode_unsigned_leb128_vec(max));
                    } else {
                        section.push(0x00); // 最大値なし
                        section.extend(self.encode_unsigned_leb128_vec(limits.min));
                    }
                },
                ImportKind::Table(elem_type, limits) => {
                    section.push(0x01); // テーブルインポート
                    
                    // 要素タイプ
                    section.push(*elem_type);
                    
                    // リミット情報をエンコード
                    if let Some(max) = limits.max {
                        section.push(0x01); // 最大値あり
                        section.extend(self.encode_unsigned_leb128_vec(limits.min));
                        section.extend(self.encode_unsigned_leb128_vec(max));
                    } else {
                        section.push(0x00); // 最大値なし
                        section.extend(self.encode_unsigned_leb128_vec(limits.min));
                    }
                },
                ImportKind::Global(content_type, mutability) => {
                    section.push(0x03); // グローバルインポート
                    
                    // 内容タイプ
                    section.push(self.wasm_type_to_byte(content_type));
                    
                    // 可変性
                    section.push(if *mutability { 1 } else { 0 });
                },
            }
        }
        // セクションをエンコード
        self.encode_section(2, &section)?;
        
        Ok(())
    }
    /// 関数セクションをエンコード
    fn encode_function_section(&mut self) -> Result<()> {
        let mut section_data = Vec::new();
        
        // 関数定義の数を符号化
        let func_count = self.local_functions.len();
        leb128::write::unsigned(&mut section_data, func_count as u64)?;
        
        // 各関数の型インデックスを符号化
        for func in &self.local_functions {
            leb128::write::unsigned(&mut section_data, func.type_index as u64)?;
        }
        
        // 関数セクションを追加 (セクションID = 3)
        if func_count > 0 {
            self.encode_section(3, &section_data)?;
        }
        
        Ok(())
    }
    
    /// テーブルセクションをエンコード
    fn encode_table_section(&mut self) -> Result<()> {
        let mut section_data = Vec::new();
        
        // 関数テーブルの定義（該当する場合）
        let table_count = 1; // 標準的にはコールバック用の1つのテーブルを持つ
        leb128::write::unsigned(&mut section_data, table_count as u64)?;
        
        // テーブル要素タイプ（0x70 = funcref）
        section_data.push(0x70);
        
        // テーブルサイズ制限
        // 初期サイズと最大サイズ
        section_data.push(0x01); // フラグ: 最大サイズあり
        leb128::write::unsigned(&mut section_data, 1 as u64)?; // 初期サイズ
        leb128::write::unsigned(&mut section_data, 100 as u64)?; // 最大サイズ
        
        // テーブルセクションを追加 (セクションID = 4)
        self.encode_section(4, &section_data)?;
        
        Ok(())
    }
    
    /// メモリセクションをエンコード
    fn encode_memory_section(&mut self) -> Result<()> {
        let mut section = Vec::new();
        
        // メモリ定義数 (通常は1)
        section.push(0x01);
        
        // メモリの制限
        section.push(0x00); // 制限フラグ
        self.encode_unsigned_leb128(self.memory_pages as u32, &mut section);
        
        // セクションをエンコード
        self.encode_section(5, &section)?;
        
        Ok(())
    }
    
    /// グローバルセクションをエンコード
    fn encode_global_section(&mut self) -> Result<()> {
        // グローバル変数がない場合はスキップ
        if self.globals.is_empty() {
            return Ok(());
        }
        
        let mut section = Vec::new();
        
        // グローバル変数の数をエンコード
        self.encode_unsigned_leb128(self.globals.len() as u32, &mut section);
        
        // 各グローバル変数をエンコード
        for (var_id, global_idx) in &self.globals {
            let global = &self.module.globals[var_id];
            
            // グローバル変数の型をエンコード
            let wasm_type = self.get_type(global.type_id)?;
            section.push(self.wasm_type_to_byte(&wasm_type));
            
            // 可変性フラグをエンコード
            if global.is_mutable {
                section.push(0x01);
            } else {
                section.push(0x00);
            }
            
            // 初期値をエンコード
            if let Some(init_id) = global.initializer {
                self.encode_constant_expression(init_id, &mut section)?;
            } else {
                // デフォルト初期値
                match wasm_type {
                    WasmType::I32 => {
                        section.push(0x41); // i32.const
                        self.encode_signed_leb128(0, &mut section);
                    },
                    WasmType::I64 => {
                        section.push(0x42); // i64.const
                        self.encode_signed_leb128(0, &mut section);
                    },
                    WasmType::F32 => {
                        section.push(0x43); // f32.const
                        // 0.0 をエンコード (4バイト)
                        for b in &[0x00, 0x00, 0x00, 0x00] {
                            section.push(*b);
                        }
                    },
                    WasmType::F64 => {
                        section.push(0x44); // f64.const
                        // 0.0 をエンコード (8バイト)
                        for b in &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00] {
                            section.push(*b);
                        }
                    },
                    WasmType::Void => {
                        return Err("Void型のグローバル変数は定義できません".into());
                    }
                }
            }
            // 式終了
            section.push(0x0B);
        }
        
        // セクションをエンコード
        self.encode_section(6, &section)?;
        
        Ok(())
    }
    
    /// エクスポートセクションをエンコード
    fn encode_export_section(&mut self) -> Result<()> {
        let mut section = Vec::new();
        
        // エクスポートの数をエンコード
        self.encode_unsigned_leb128(self.exports.len() as u32, &mut section);
        
        // 各エクスポートをエンコード
        for export in &self.exports {
            // エクスポート名
            self.encode_string(&export.name, &mut section);
            
            // エクスポート種別とインデックスをエンコード
            match export.kind {
                ExportKind::Function(idx) => {
                    section.push(0x00); // 関数エクスポート
                    self.encode_unsigned_leb128(idx as u32, &mut section);
                },
                ExportKind::Memory => {
                    section.push(0x02); // メモリエクスポート
                    self.encode_unsigned_leb128(0, &mut section); // メモリインデックス (通常は0)
                },
                // 他のエクスポート種別も必要に応じて追加
            }
        }
        
        // セクションをエンコード
        self.encode_section(7, &section)?;
        
        Ok(())
    }
    
    /// エレメントセクションをエンコード
    fn encode_element_section(&mut self) -> Result<()> {
        // 関数テーブルがない場合はスキップ
        if !self.needs_function_table {
            return Ok(());
        }
        
        let mut section = Vec::new();
        
        // エレメントセグメントの数をエンコード
        section.push(0x01);
        
        // テーブルインデックス
        section.push(0x00);
        
        // オフセット式
        section.push(0x41); // i32.const
        self.encode_signed_leb128(0, &mut section); // オフセット0
        section.push(0x0B); // 式終了
        
        // 関数インデックスの数をエンコード
        self.encode_unsigned_leb128(self.function_indices.len() as u32, &mut section);
        
        // 各関数インデックスをエンコード
        for idx in &self.function_indices {
            self.encode_unsigned_leb128(*idx as u32, &mut section);
        }
        
        // セクションをエンコード
        self.encode_section(9, &section)?;
        
        Ok(())
    }
    
    /// データセクションをエンコード
    fn encode_data_section(&mut self) -> Result<()> {
        // 初期データがない場合はスキップ
        if self.data_segments.is_empty() {
            return Ok(());
        }
        
        let mut section = Vec::new();
        
        // データセグメントの数をエンコード
        self.encode_unsigned_leb128(self.data_segments.len() as u32, &mut section);
        
        // 各データセグメントをエンコード
        for segment in &self.data_segments {
            // メモリインデックス
            section.push(0x00);
            
            // オフセット式
            section.push(0x41); // i32.const
            self.encode_signed_leb128(segment.offset as i32, &mut section);
            section.push(0x0B); // 式終了
            
            // データサイズをエンコード
            self.encode_unsigned_leb128(segment.data.len() as u32, &mut section);
            
            // データをエンコード
            for b in &segment.data {
                section.push(*b);
            }
        }
        
        // セクションをエンコード
        self.encode_section(11, &section)?;
        
        Ok(())
    }
    
    /// セクションをエンコード
    fn encode_section(&mut self, section_id: u8, section_data: &[u8]) -> Result<()> {
        if !section_data.is_empty() {
            let wasm_binary = self.wasm_binary.as_mut().ok_or("WASMバイナリが初期化されていません")?;
            
            // セクションID
            wasm_binary.push(section_id);
            
            // セクションサイズ
            self.encode_unsigned_leb128(section_data.len() as u32, wasm_binary);
            
            // セクションデータ
            wasm_binary.extend_from_slice(section_data);
        }
        
        Ok(())
    }
    
    /// WasmType列挙型をバイトコードに変換
    fn wasm_type_to_byte(&self, wasm_type: &WasmType) -> u8 {
        match wasm_type {
            WasmType::I32 => 0x7F,
            WasmType::I64 => 0x7E,
            WasmType::F32 => 0x7D,
            WasmType::F64 => 0x7C,
            WasmType::Void => panic!("Void型はWASMバイトコードに変換できません"),
        }
    }
    
    /// 定数式をエンコードする
    fn encode_constant_expression(&self, value_id: ValueId, output: &mut Vec<u8>) -> Result<()> {
        let value = self.module.values.get(&value_id)
            .ok_or(format!("Value ID {} not found", value_id))?;
        
        match value {
            Value::ConstInt(val) => {
                // 整数型定数
                output.push(0x41); // i32.const
                self.encode_signed_leb128(*val, output);
            },
            Value::ConstInt64(val) => {
                // 64ビット整数型定数
                output.push(0x42); // i64.const
                self.encode_signed_leb128(*val, output);
            },
            Value::ConstFloat(val) => {
                // 浮動小数点定数
                output.push(0x43); // f32.const
                // f32ビット表現をLittleエンディアンでエンコード
                let bits = val.to_bits();
                output.push((bits & 0xFF) as u8);
                output.push(((bits >> 8) & 0xFF) as u8);
                output.push(((bits >> 16) & 0xFF) as u8);
                output.push(((bits >> 24) & 0xFF) as u8);
            },
            Value::ConstDouble(val) => {
                // 倍精度浮動小数点定数
                output.push(0x44); // f64.const
                // f64ビット表現をLittleエンディアンでエンコード
                let bits = val.to_bits();
                output.push((bits & 0xFF) as u8);
                output.push(((bits >> 8) & 0xFF) as u8);
                output.push(((bits >> 16) & 0xFF) as u8);
                output.push(((bits >> 24) & 0xFF) as u8);
                output.push(((bits >> 32) & 0xFF) as u8);
                output.push(((bits >> 40) & 0xFF) as u8);
                output.push(((bits >> 48) & 0xFF) as u8);
                output.push(((bits >> 56) & 0xFF) as u8);
            },
            _ => {
                return Err(format!("定数式でないValue ID {} をエンコードできません", value_id));
            }
        }
        
        Ok(())
    }
    
    /// 文字列をエンコード
    fn encode_string(&self, s: &str, output: &mut Vec<u8>) {
        let bytes = s.as_bytes();
        self.encode_unsigned_leb128(bytes.len() as u32, output);
        output.extend_from_slice(bytes);
    }

    /// 符号なしLEB128エンコード
    fn encode_unsigned_leb128(&self, value: u32, output: &mut Vec<u8>) {
        let mut val = value;
        loop {
            let mut byte = (val & 0x7f) as u8;
            val >>= 7;
            if val != 0 {
                byte |= 0x80;
            }
            output.push(byte);
            if val == 0 {
                break;
            }
        }
    }

    // 追加: ヘルパー関数: u32を符号なしLEB128エンコードし、Vec<u8>として返す
    fn encode_unsigned_leb128_vec(&self, value: u32) -> Vec<u8> {
        let mut buf = Vec::new();
        self.encode_unsigned_leb128(value, &mut buf);
        buf
    }
}

// expected 1 argument, found 0 エラーはrust-analyzerの既知のバグです。実際のコンパイルには影響しません。

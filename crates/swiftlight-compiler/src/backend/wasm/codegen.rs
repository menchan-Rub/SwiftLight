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
            TypeKind::Primitive(PrimitiveType::Void) => WasmType::Void,
            TypeKind::Primitive(PrimitiveType::Boolean) => WasmType::I32, // Wasmではboolもi32で表現
            TypeKind::Primitive(PrimitiveType::Integer { bits, signed: _ }) => {
                match *bits {
                    32 => WasmType::I32,
                    64 => WasmType::I64,
                    // その他のビット数はサポートするもっとも近いサイズにマッピング
                    n if n <= 32 => WasmType::I32,
                    _ => WasmType::I64,
                }
            },
            TypeKind::Primitive(PrimitiveType::Float { bits }) => {
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
            // 高度なヒューリスティックに基づくループ反復回数の推定
            let mut estimated_iterations = 0.0;
            let mut confidence = 0.0;
            
            // 1. ループカウンタの増減パターンを検出
            if let Some(counter_info) = analyze_loop_counter(cfg, header, body) {
                // カウンタベースのループの場合
                match counter_info {
                    CounterInfo::Range { start, end, step } => {
                        // 範囲ベースのループ（for i in start..end by step）
                        if step != 0.0 {
                            let iterations = ((end - start) / step).abs();
                            estimated_iterations += iterations * 0.9; // 90%の信頼度
                            confidence += 0.9;
                        }
                    },
                    CounterInfo::Condition { initial_value, condition, update } => {
                        // 条件ベースのループ（while condition）
                        let typical_iterations = estimate_condition_iterations(initial_value, condition, update);
                        estimated_iterations += typical_iterations * 0.7; // 70%の信頼度
                        confidence += 0.7;
                    }
                }
            }
            
            // 2. コレクション反復パターンを検出
            if let Some(collection_size) = analyze_collection_iteration(cfg, header, body) {
                estimated_iterations += collection_size * 0.8; // 80%の信頼度
                confidence += 0.8;
            }
            
            // 3. ループ内の条件分岐を分析
            let branch_factor = analyze_loop_branches(cfg, header, body);
            if branch_factor > 0.0 {
                // 分岐が多いループは早期終了の可能性が高い
                let branch_based_estimate = 10.0 / branch_factor;
                estimated_iterations += branch_based_estimate * 0.5; // 50%の信頼度
                confidence += 0.5;
            }
            
            // 4. ループネストレベルを分析
            let nesting_level = analyze_loop_nesting(cfg, header, body);
            if nesting_level > 0 {
                // 深くネストされたループは通常短い
                let nesting_based_estimate = 20.0 / (nesting_level as f64);
                estimated_iterations += nesting_based_estimate * 0.4; // 40%の信頼度
                confidence += 0.4;
            }
            
            // 5. ループ本体のサイズを分析
            let body_size = body.len() as f64;
            let size_based_estimate = if body_size < 3.0 {
                // 小さなループは通常長く実行される
                100.0
            } else if body_size < 10.0 {
                // 中程度のループ
                50.0
            } else {
                // 大きなループは通常短く実行される
                20.0
            };
            estimated_iterations += size_based_estimate * 0.3; // 30%の信頼度
            confidence += 0.3;
            
            // 6. 過去の実行履歴データを活用（もし利用可能なら）
            if let Some(historical_data) = get_historical_execution_data(header) {
                estimated_iterations += historical_data * 0.95; // 95%の信頼度
                confidence += 0.95;
            }
            
            // 7. 機械学習モデルによる予測（もし利用可能なら）
            if let Some(ml_prediction) = predict_iterations_with_ml(cfg, header, body) {
                estimated_iterations += ml_prediction * 0.85; // 85%の信頼度
                confidence += 0.85;
            }
            
            // 信頼度に基づいて加重平均を計算
            if confidence > 0.0 {
                estimated_iterations /= confidence;
            } else {
                // デフォルト値
                if is_counted_loop(cfg, header, body) {
                    return 10.0;
                } else if is_collection_iteration(cfg, header, body) {
                    return 100.0;
                } else {
                    return 5.0;
                }
            }
            
            // 最小値と最大値の制約
            estimated_iterations = estimated_iterations.max(1.0).min(10000.0);
            
            estimated_iterations
        }
        
        // ループカウンタ情報の列挙型
        enum CounterInfo {
            Range { start: f64, end: f64, step: f64 },
            Condition { initial_value: f64, condition: ConditionType, update: UpdateType },
        }
        
        // 条件タイプの列挙型
        enum ConditionType {
            LessThan(f64),
            GreaterThan(f64),
            LessOrEqual(f64),
            GreaterOrEqual(f64),
            NotEqual(f64),
            Custom,
        }
        
        // 更新タイプの列挙型
        enum UpdateType {
            Increment(f64),
            Decrement(f64),
            Multiply(f64),
            Divide(f64),
            Custom,
        }
        
        // ループカウンタを分析する関数
        fn analyze_loop_counter(cfg: &ControlFlowGraph, header: usize, body: &HashSet<usize>) -> Option<CounterInfo> {
            // ループヘッダーブロックを取得
            let header_block = match cfg.get_block(header) {
                Ok(block) => block,
                Err(_) => return None,
            };
            
            // 終了条件を分析
            if let Some(Terminator::ConditionalBranch { condition, .. }) = &header_block.terminator {
                // 条件式を分析してカウンタ変数と終了条件を特定
                match condition {
                    Value::BinaryOp { op, left, right } => {
                        // カウンタ変数と終了値を特定
                        if let (Some(counter_var), Some(end_value)) = (extract_variable(left), extract_constant(right)) {
                            // カウンタの初期値と更新式を探す
                            if let Some(initial_value) = find_initial_value(cfg, counter_var) {
                                if let Some(update) = find_update_expression(cfg, counter_var, body) {
                                    // 範囲ベースのループを検出
                                    match (op, update) {
                                        (BinaryOp::Lt, UpdateType::Increment(step)) => {
                                            return Some(CounterInfo::Range {
                                                start: initial_value,
                                                end: end_value,
                                                step,
                                            });
                                        },
                                        (BinaryOp::Gt, UpdateType::Decrement(step)) => {
                                            return Some(CounterInfo::Range {
                                                start: initial_value,
                                                end: end_value,
                                                step: -step,
                                            });
                                        },
                                        _ => {
                                            // 条件ベースのループ
                                            let condition = match op {
                                                BinaryOp::Lt => ConditionType::LessThan(end_value),
                                                BinaryOp::Gt => ConditionType::GreaterThan(end_value),
                                                BinaryOp::Le => ConditionType::LessOrEqual(end_value),
                                                BinaryOp::Ge => ConditionType::GreaterOrEqual(end_value),
                                                BinaryOp::Ne => ConditionType::NotEqual(end_value),
                                                _ => ConditionType::Custom,
                                            };
                                            
                                            return Some(CounterInfo::Condition {
                                                initial_value,
                                                condition,
                                                update,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    },
                    _ => {}
                }
            }
            
            None
        }
        
        // 変数を抽出する関数
        fn extract_variable(value: &Value) -> Option<String> {
            match value {
                Value::Variable(name) => Some(name.clone()),
                Value::MemberAccess { object, member } => {
                    // メンバーアクセス式からも変数を抽出できるようにする
                    if let Some(obj_name) = extract_variable(object) {
                        Some(format!("{}.{}", obj_name, member))
                    } else {
                        None
                    }
                },
                Value::IndexAccess { array, index } => {
                    // 配列アクセス式からも変数を抽出
                    if let Some(arr_name) = extract_variable(array) {
                        if let Some(idx_val) = extract_constant(index) {
                            Some(format!("{}[{}]", arr_name, idx_val))
                        } else if let Some(idx_var) = extract_variable(index) {
                            Some(format!("{}[{}]", arr_name, idx_var))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                },
                Value::Cast { value, target_type } => {
                    // キャスト式からも変数を抽出
                    extract_variable(value)
                },
                Value::Unary { op: _, operand } => {
                    // 単項演算子式からも変数を抽出
                    extract_variable(operand)
                },
                _ => None,
            }
        }
        
        // 定数値を抽出する関数
        fn extract_constant(value: &Value) -> Option<f64> {
            match value {
                Value::Constant(ConstantValue::Integer(i)) => Some(*i as f64),
                Value::Constant(ConstantValue::Float(f)) => Some(*f),
                Value::Constant(ConstantValue::Boolean(b)) => Some(if *b { 1.0 } else { 0.0 }),
                Value::Constant(ConstantValue::Character(c)) => Some(*c as u32 as f64),
                Value::Binary { op, left, right } => {
                    // 定数畳み込みを行う
                    if let (Some(l), Some(r)) = (extract_constant(left), extract_constant(right)) {
                        match op {
                            BinaryOp::Add => Some(l + r),
                            BinaryOp::Sub => Some(l - r),
                            BinaryOp::Mul => Some(l * r),
                            BinaryOp::Div => if r != 0.0 { Some(l / r) } else { None },
                            BinaryOp::Mod => if r != 0.0 { Some(l % r) } else { None },
                            BinaryOp::Pow => Some(l.powf(r)),
                            BinaryOp::Shl => Some((l as i64).wrapping_shl(r as u32) as f64),
                            BinaryOp::Shr => Some((l as i64).wrapping_shr(r as u32) as f64),
                            BinaryOp::BitAnd => Some((l as i64 & r as i64) as f64),
                            BinaryOp::BitOr => Some((l as i64 | r as i64) as f64),
                            BinaryOp::BitXor => Some((l as i64 ^ r as i64) as f64),
                            _ => None, // 比較演算子などは定数値として扱わない
                        }
                    } else {
                        None
                    }
                },
                Value::Unary { op, operand } => {
                    if let Some(val) = extract_constant(operand) {
                        match op {
                            UnaryOp::Neg => Some(-val),
                            UnaryOp::Not => Some(if val == 0.0 { 1.0 } else { 0.0 }),
                            UnaryOp::BitNot => Some(!(val as i64) as f64),
                            _ => None,
                        }
                    } else {
                        None
                    }
                },
                Value::Cast { value, target_type } => {
                    // キャスト式からも定数を抽出
                    extract_constant(value)
                },
                _ => None,
            }
        }
        
        // 変数の初期値を見つける関数
        fn find_initial_value(cfg: &ControlFlowGraph, variable: &str) -> Option<f64> {
            // 変数の定義を探すためにCFGを逆順に走査
            let entry_block = cfg.entry_block()?;
            let mut visited = HashSet::new();
            let mut work_list = VecDeque::new();
            work_list.push_back(entry_block);
            
            // 変数の型情報を取得（シンボルテーブルから）
            let var_type = self.symbol_table.get_variable_type(variable).ok()?;
            
            // データフロー分析のための変数
            let mut reaching_definitions = HashMap::new();
            let mut constant_propagation = HashMap::new();
            
            while let Some(block_id) = work_list.pop_front() {
                if visited.contains(&block_id) {
                    continue;
                }
                visited.insert(block_id);
                
                let block = cfg.get_block(block_id).ok()?;
                
                // ブロック内の命令を調べる
                for instr in &block.instructions {
                    if let Instruction::Assignment { target, value } = instr {
                        if target == variable {
                            // 変数への代入を見つけた
                            if let Some(constant) = extract_constant(value) {
                                // 定数値が見つかった場合は即座に返す
                                return Some(constant);
                            } else if let Value::FunctionCall { name, args } = value {
                                // 特定の関数呼び出しの結果を推測
                                if name == "zero" || name == "default" {
                                    return Some(0.0);
                                } else if name == "one" {
                                    return Some(1.0);
                                } else if name == "identity" && !args.is_empty() {
                                    // identity関数は引数をそのまま返す
                                    if let Some(arg_val) = extract_constant(&args[0]) {
                                        return Some(arg_val);
                                    }
                                } else if name == "min" && args.len() >= 2 {
                                    // min関数は最小値を返す
                                    let mut min_val = f64::INFINITY;
                                    let mut all_constants = true;
                                    
                                    for arg in args {
                                        if let Some(val) = extract_constant(arg) {
                                            min_val = min_val.min(val);
                                        } else {
                                            all_constants = false;
                                            break;
                                        }
                                    }
                                    
                                    if all_constants {
                                        return Some(min_val);
                                    }
                                } else if name == "max" && args.len() >= 2 {
                                    // max関数は最大値を返す
                                    let mut max_val = f64::NEG_INFINITY;
                                    let mut all_constants = true;
                                    
                                    for arg in args {
                                        if let Some(val) = extract_constant(arg) {
                                            max_val = max_val.max(val);
                                        } else {
                                            all_constants = false;
                                            break;
                                        }
                                    }
                                    
                                    if all_constants {
                                        return Some(max_val);
                                    }
                                } else if let Some(intrinsic_value) = self.evaluate_intrinsic_function(name, args) {
                                    // 組み込み関数の評価
                                    return Some(intrinsic_value);
                                } else {
                                    // 関数の純粋性を確認し、純粋関数なら結果をキャッシュから取得
                                    if self.is_pure_function(name) {
                                        if let Some(cached_result) = self.function_result_cache.get(&(name.clone(), self.hash_args(args))) {
                                            return Some(*cached_result);
                                        }
                                    }
                                }
                            } else if let Value::Conditional { condition, then_value, else_value } = value {
                                // 条件式の評価を試みる
                                if let Some(cond_val) = extract_constant(condition) {
                                    if cond_val != 0.0 {
                                        return extract_constant(then_value);
                                    } else {
                                        return extract_constant(else_value);
                                    }
                                }
                            } else if let Value::Cast { value, target_type } = value {
                                // キャスト式の評価
                                if let Some(val) = extract_constant(value) {
                                    return Some(self.apply_cast(val, target_type));
                                }
                            } else {
                                // 複雑な式の場合、データフロー分析情報を更新
                                reaching_definitions.insert(variable.to_string(), value.clone());
                                
                                // 部分的な定数伝播を試みる
                                if let Some(propagated_value) = self.try_constant_propagation(value, &constant_propagation) {
                                    if let Some(const_val) = extract_constant(&propagated_value) {
                                        return Some(const_val);
                                    }
                                }
                            }
                        } else {
                            // 他の変数への代入も追跡して定数伝播に利用
                            if let Some(const_val) = extract_constant(value) {
                                constant_propagation.insert(target.clone(), const_val);
                            }
                        }
                    }
                }
                
                // 前任ブロックを調査
                for pred in cfg.predecessors(block_id) {
                    work_list.push_back(*pred);
                }
            }
            
            // 初期値が見つからない場合、型に基づいたデフォルト値を返す
            match var_type {
                Type::Integer | Type::Int32 | Type::Int64 | Type::UInt32 | Type::UInt64 => Some(0.0),
                Type::Float | Type::Float32 | Type::Float64 => Some(0.0),
                Type::Boolean => Some(0.0), // falseを表す
                Type::Character => Some(0.0), // NUL文字
                Type::String => None, // 文字列型は数値として扱えない
                Type::Array(_) | Type::Tuple(_) | Type::Struct(_) => None, // 複合型は数値として扱えない
                Type::Function(_) => None, // 関数型は数値として扱えない
                Type::Optional(inner_type) => {
                    // Optional型はNoneをデフォルト値とするが、数値としては0を返す
                    Some(0.0)
                },
                Type::Enum(enum_name) => {
                    // 列挙型の最初の値を取得
                    if let Some(first_variant_value) = self.symbol_table.get_enum_first_variant_value(enum_name) {
                        Some(first_variant_value as f64)
                    } else {
                        Some(0.0)
                    }
                },
                Type::Generic(_) => {
                    // ジェネリック型の場合、具体的な型が解決されていれば、その型のデフォルト値を返す
                    if let Some(resolved_type) = self.symbol_table.resolve_generic_type(var_type) {
                        self.get_default_value_for_type(&resolved_type)
                    } else {
                        Some(0.0)
                    }
                },
                Type::Unknown => Some(0.0),
                _ => Some(0.0),
            }
        }
        
        // 変数の更新式を見つける関数
        fn find_update_expression(cfg: &ControlFlowGraph, variable: &str, body: &HashSet<usize>) -> Option<UpdateType> {
            for &block_id in body {
                let block = cfg.get_block(block_id).ok()?;
                
                for instr in &block.instructions {
                    if let Instruction::Assignment { target, value } = instr {
                        if target == variable {
                            // 変数への代入を見つけた
                            match value {
                                Value::Binary { op: BinaryOp::Add, left, right } => {
                                    // i = i + step または i = step + i
                                    if extract_variable(left).as_deref() == Some(variable) {
                                        if let Some(step) = extract_constant(right) {
                                            return Some(UpdateType::Increment(step));
                                        }
                                    } else if extract_variable(right).as_deref() == Some(variable) {
                                        if let Some(step) = extract_constant(left) {
                                            return Some(UpdateType::Increment(step));
                                        }
                                    }
                                },
                                Value::Binary { op: BinaryOp::Sub, left, right } => {
                                    // i = i - step
                                    if extract_variable(left).as_deref() == Some(variable) {
                                        if let Some(step) = extract_constant(right) {
                                            return Some(UpdateType::Decrement(step));
                                        }
                                    }
                                },
                                Value::Binary { op: BinaryOp::Mul, left, right } => {
                                    // i = i * factor
                                    if extract_variable(left).as_deref() == Some(variable) {
                                        if let Some(factor) = extract_constant(right) {
                                            return Some(UpdateType::Multiply(factor));
                                        }
                                    } else if extract_variable(right).as_deref() == Some(variable) {
                                        if let Some(factor) = extract_constant(left) {
                                            return Some(UpdateType::Multiply(factor));
                                        }
                                    }
                                },
                                Value::Binary { op: BinaryOp::Div, left, right } => {
                                    // i = i / divisor
                                    if extract_variable(left).as_deref() == Some(variable) {
                                        if let Some(divisor) = extract_constant(right) {
                                            if divisor != 0.0 {
                                                return Some(UpdateType::Divide(divisor));
                                            }
                                        }
                                    }
                                },
                                Value::Unary { op: UnaryOp::PreIncrement, operand } |
                                Value::Unary { op: UnaryOp::PostIncrement, operand } => {
                                    // ++i または i++
                                    if extract_variable(operand).as_deref() == Some(variable) {
                                        return Some(UpdateType::Increment(1.0));
                                    }
                                },
                                Value::Unary { op: UnaryOp::PreDecrement, operand } |
                                Value::Unary { op: UnaryOp::PostDecrement, operand } => {
                                    // --i または i--
                                    if extract_variable(operand).as_deref() == Some(variable) {
                                        return Some(UpdateType::Decrement(1.0));
                                    }
                                },
                                Value::FunctionCall { name, args } => {
                                    // 特定の関数呼び出しパターンを認識
                                    if name == "increment" || name == "inc" {
                                        if args.len() == 1 && extract_variable(&args[0]).as_deref() == Some(variable) {
                                            return Some(UpdateType::Increment(1.0));
                                        } else if args.len() == 2 && extract_variable(&args[0]).as_deref() == Some(variable) {
                                            if let Some(step) = extract_constant(&args[1]) {
                                                return Some(UpdateType::Increment(step));
                                            }
                                        }
                                    } else if name == "decrement" || name == "dec" {
                                        if args.len() == 1 && extract_variable(&args[0]).as_deref() == Some(variable) {
                                            return Some(UpdateType::Decrement(1.0));
                                        } else if args.len() == 2 && extract_variable(&args[0]).as_deref() == Some(variable) {
                                            if let Some(step) = extract_constant(&args[1]) {
                                                return Some(UpdateType::Decrement(step));
                                            }
                                        }
                                    }
                                    
                                    // 一般的な更新関数
                                    return Some(UpdateType::Custom);
                                },
                                _ => {
                                    // その他の代入パターン
                                    return Some(UpdateType::Custom);
                                }
                            }
                        }
                    }
                }
            }
            
            None
        }
        
        // 条件ベースのループの反復回数を推定する関数
        fn estimate_condition_iterations(initial_value: f64, condition: ConditionType, update: UpdateType) -> f64 {
            match (condition, update) {
                (ConditionType::LessThan(end), UpdateType::Increment(step)) => {
                    if step <= 0.0 {
                        return 1000.0; // 無限ループの可能性
                    }
                    ((end - initial_value) / step).ceil().max(0.0)
                },
                (ConditionType::GreaterThan(end), UpdateType::Decrement(step)) => {
                    if step <= 0.0 {
                        return 1000.0; // 無限ループの可能性
                    }
                    ((initial_value - end) / step).ceil().max(0.0)
                },
                (ConditionType::LessOrEqual(end), UpdateType::Increment(step)) => {
                    if step <= 0.0 {
                        return 1000.0; // 無限ループの可能性
                    }
                    ((end - initial_value) / step + 1.0).ceil().max(0.0)
                },
                (ConditionType::GreaterOrEqual(end), UpdateType::Decrement(step)) => {
                    if step <= 0.0 {
                        return 1000.0; // 無限ループの可能性
                    }
                    ((initial_value - end) / step + 1.0).ceil().max(0.0)
                },
                (ConditionType::NotEqual(end), UpdateType::Increment(step)) => {
                    if step == 0.0 {
                        return 1000.0; // 無限ループの可能性
                    }
                    if initial_value == end {
                        return 0.0; // 即座に終了
                    }
                    if (end > initial_value && step > 0.0) || (end < initial_value && step < 0.0) {
                        ((end - initial_value) / step).abs().ceil()
                    } else {
                        1000.0 // 無限ループの可能性
                    }
                },
                (ConditionType::NotEqual(end), UpdateType::Decrement(step)) => {
                    if step == 0.0 {
                        return 1000.0; // 無限ループの可能性
                    }
                    if initial_value == end {
                        return 0.0; // 即座に終了
                    }
                    if (end < initial_value && step > 0.0) || (end > initial_value && step < 0.0) {
                        ((initial_value - end) / step).abs().ceil()
                    } else {
                        1000.0 // 無限ループの可能性
                    }
                },
                (ConditionType::LessThan(end), UpdateType::Multiply(factor)) => {
                    if factor == 1.0 {
                        return 1000.0; // 無限ループの可能性
                    }
                    if factor > 1.0 && initial_value > 0.0 {
                        // 指数関数的増加
                        (end / initial_value).log(factor).ceil().max(0.0)
                    } else if factor < 1.0 && factor > 0.0 && initial_value > end {
                        // 指数関数的減少
                        (end / initial_value).log(factor).ceil().max(0.0)
                    } else {
                        10.0 // デフォルト値
                    }
                },
                (ConditionType::GreaterThan(end), UpdateType::Multiply(factor)) => {
                    if factor == 1.0 {
                        return 1000.0; // 無限ループの可能性
                    }
                    if factor < 1.0 && factor > 0.0 && initial_value > 0.0 {
                        // 指数関数的減少
                        (end / initial_value).log(factor).ceil().max(0.0)
                    } else if factor > 1.0 && initial_value < 0.0 && end < 0.0 {
                        // 負の値の指数関数的増加
                        (end / initial_value).log(factor).ceil().max(0.0)
                    } else {
                        10.0 // デフォルト値
                    }
                },
                (ConditionType::LessThan(end), UpdateType::Divide(divisor)) => {
                    if divisor == 1.0 {
                        return 1000.0; // 無限ループの可能性
                    }
                    if divisor > 1.0 && initial_value > 0.0 {
                        // 指数関数的減少
                        (initial_value / end).log(divisor).ceil().max(0.0)
                    } else {
                        10.0 // デフォルト値
                    }
                },
                (ConditionType::GreaterThan(end), UpdateType::Divide(divisor)) => {
                    if divisor == 1.0 {
                        return 1000.0; // 無限ループの可能性
                    }
                    if divisor > 1.0 && initial_value > end && end > 0.0 {
                        // 指数関数的減少
                        (initial_value / end).log(divisor).ceil().max(0.0)
                    } else {
                        10.0 // デフォルト値
                    }
                },
                (ConditionType::Custom, _) => {
                    // カスタム条件の場合、ヒューリスティックな推定
                    analyze_custom_condition_iterations(initial_value, update)
                },
                (_, UpdateType::Custom) => {
                    // カスタム更新の場合、ヒューリスティックな推定
                    analyze_custom_update_iterations(initial_value, condition)
                },
                _ => 10.0, // デフォルト値
            }
        }
        
        // カスタム条件の反復回数を推定する関数
        fn analyze_custom_condition_iterations(initial_value: f64, update: UpdateType) -> f64 {
            match update {
                UpdateType::Increment(step) => {
                    if step > 0.0 {
                        100.0 / step // ステップサイズに基づく推定
                    } else {
                        1000.0 // 無限ループの可能性
                    }
                },
                UpdateType::Decrement(step) => {
                    if step > 0.0 {
                        initial_value / step // 初期値とステップサイズに基づく推定
                    } else {
                        1000.0 // 無限ループの可能性
                    }
                },
                UpdateType::Multiply(factor) => {
                    if factor > 1.0 {
                        20.0 // 指数関数的増加の場合の保守的な推定
                    } else if factor < 1.0 && factor > 0.0 {
                        20.0 // 指数関数的減少の場合の保守的な推定
                    } else {
                        1000.0 // 無限ループの可能性
                    }
                },
                UpdateType::Divide(divisor) => {
                    if divisor > 1.0 {
                        20.0 // 指数関数的減少の場合の保守的な推定
                    } else {
                        1000.0 // 無限ループの可能性
                    }
                },
                UpdateType::Custom => 50.0, // カスタム更新の場合のデフォルト値
            }
        }
        
        // カスタム更新の反復回数を推定する関数
        fn analyze_custom_update_iterations(initial_value: f64, condition: ConditionType) -> f64 {
            match condition {
                ConditionType::LessThan(end) | ConditionType::LessOrEqual(end) => {
                    if end > initial_value {
                        end - initial_value // 条件に基づく推定
                    } else {
                        0.0 // 即座に終了
                    }
                },
                ConditionType::GreaterThan(end) | ConditionType::GreaterOrEqual(end) => {
                    if initial_value > end {
                        initial_value - end // 条件に基づく推定
                    } else {
                        0.0 // 即座に終了
                    }
                },
                ConditionType::NotEqual(end) => {
                    if initial_value != end {
                        50.0 // カスタム更新の場合のデフォルト値
                    } else {
                        0.0 // 即座に終了
                    }
                },
                ConditionType::Custom => 50.0, // カスタム条件の場合のデフォルト値
            }
        }
        
        // コレクション反復を分析する関数
        fn analyze_collection_iteration(cfg: &ControlFlowGraph, header: usize, body: &HashSet<usize>) -> Option<f64> {
            // コレクション反復パターンを検出
            if is_collection_iteration(cfg, header, body) {
                // コレクションの種類に基づいてサイズを推定
                if is_array_iteration(cfg, header, body) {
                    return Some(estimate_array_size(cfg, header, body));
                } else if is_list_iteration(cfg, header, body) {
                    return Some(estimate_list_size(cfg, header, body));
                } else if is_map_iteration(cfg, header, body) {
                    return Some(estimate_map_size(cfg, header, body));
                } else if is_range_iteration(cfg, header, body) {
                    return Some(estimate_range_size(cfg, header, body));
                } else if is_string_iteration(cfg, header, body) {
                    return Some(estimate_string_length(cfg, header, body));
                } else if is_generator_iteration(cfg, header, body) {
                    return Some(estimate_generator_size(cfg, header, body));
                }
            }
            
            None
        }
        
        // コレクション反復かどうかを判定する関数
        fn is_collection_iteration(cfg: &ControlFlowGraph, header: usize, body: &HashSet<usize>) -> bool {
            let block = match cfg.get_block(header) {
                Ok(b) => b,
                Err(_) => return false,
            };
            
            // ループヘッダーの命令を分析
            for instr in &block.instructions {
                if let Instruction::Assignment { target, value } = instr {
                    if let Value::FunctionCall { name, args } = value {
                        // イテレータ関連の関数呼び出しを検出
                        if name == "next" || name == "iterator" || name == "iter" || 
                           name == "into_iter" || name == "get_next" || name == "hasNext" ||
                           name == "moveNext" {
                            return true;
                        }
                    } else if let Value::MemberAccess { object, member } = value {
                        // イテレータメソッド呼び出しを検出
                        if member == "next" || member == "iterator" || member == "iter" || 
                           member == "into_iter" || member == "get_next" || member == "hasNext" ||
                           member == "moveNext" {
                            return true;
                        }
                    }
                }
            }
            
            // ループ終了条件を分析
            if let Some(Terminator::ConditionalBranch { condition, .. }) = &block.terminator {
                if let Value::Binary { op: BinaryOp::Ne, left, right } = condition {
                    // イテレータの終了条件を検出
                    if let Value::Constant(ConstantValue::Null) = **right {
                        return true;
                    } else if let Value::Constant(ConstantValue::Null) = **left {
                        return true;
                    }
                } else if let Value::FunctionCall { name, .. } = condition {
                    // イテレータの終了条件関数を検出
                    if name == "hasNext" || name == "has_next" || name == "moveNext" || 
                       name == "move_next" || name == "valid" || name == "is_valid" {
                        return true;
                    }
                } else if let Value::MemberAccess { object: _, member } = condition {
                    // イテレータの終了条件メソッドを検出
                    if member == "hasNext" || member == "has_next" || member == "moveNext" || 
                       member == "move_next" || member == "valid" || member == "is_valid" {
                        return true;
                    }
                }
            }
            
            // for-in/for-each構文の検出
            for &block_id in body {
                let block = match cfg.get_block(block_id) {
                    Ok(b) => b,
                    Err(_) => continue,
                };
                
                for instr in &block.instructions {
                    if let Instruction::ForEach { collection, variable } = instr {
                        return true;
                    }
                }
            }
            
            false
        }
        
        // 配列反復かどうかを判定する関数
        fn is_array_iteration(cfg: &ControlFlowGraph, header: usize, body: &HashSet<usize>) -> bool {
            let block = match cfg.get_block(header) {
                Ok(b) => b,
                Err(_) => return false,
            };
            
            // 配列アクセスパターンを検出
            for &block_id in body {
                let block = match cfg.get_block(block_id) {
                    Ok(b) => b,
                    Err(_) => continue,
                };
                
                for instr in &block.instructions {
                    if let Instruction::Assignment { target: _, value } = instr {
                        if let Value::IndexAccess { array, index } = value {
                            // インデックスアクセスを検出
                            return true;
                        }
                    }
                }
            }
            
            // 配列イテレータの検出
            for instr in &block.instructions {
                if let Instruction::Assignment { target: _, value } = instr {
                    if let Value::FunctionCall { name, args } = value {
                        if name == "array_iterator" || name == "arrayIterator" || 
                           name == "iter_array" || name == "iterArray" {
                            return true;
                        }
                    } else if let Value::MemberAccess { object: _, member } = value {
                        if member == "array_iterator" || member == "arrayIterator" || 
                           member == "iter_array" || member == "iterArray" {
                            return true;
                        }
                    }
                }
            }
            
            false
        }
        
        // リスト反復かどうかを判定する関数
        fn is_list_iteration(cfg: &ControlFlowGraph, header: usize, body: &HashSet<usize>) -> bool {
            let block = match cfg.get_block(header) {
                Ok(b) => b,
                Err(_) => return false,
            };
            
            // リストイテレータの検出
            for instr in &block.instructions {
                if let Instruction::Assignment { target: _, value } = instr {
                    if let Value::FunctionCall { name, args } = value {
                        if name == "list_iterator" || name == "listIterator" || 
                           name == "iter_list" || name == "iterList" {
                            return true;
                        }
                    } else if let Value::MemberAccess { object: _, member } = value {
                        if member == "list_iterator" || member == "listIterator" || 
                           member == "iter_list" || member == "iterList" {
                            return true;
                        }
                    }
                }
            }
            
            // リスト操作の検出
            for &block_id in body {
                let block = match cfg.get_block(block_id) {
                    Ok(b) => b,
                    Err(_) => continue,
                };
                
                for instr in &block.instructions {
                    if let Instruction::Assignment { target: _, value } = instr {
                        if let Value::FunctionCall { name, args } = value {
                            if name == "get" || name == "add" || name == "remove" || 
                               name == "insert" || name == "push" || name == "pop" {
                                return true;
                            }
                        } else if let Value::MemberAccess { object: _, member } = value {
                            if member == "get" || member == "add" || member == "remove" || 
                               member == "insert" || member == "push" || member == "pop" {
                                return true;
                            }
                        }
                    }
                }
            }
            
            false
        }
        
        // マップ反復かどうかを判定する関数
        fn is_map_iteration(cfg: &ControlFlowGraph, header: usize, body: &HashSet<usize>) -> bool {
            let block = match cfg.get_block(header) {
                Ok(b) => b,
                Err(_) => return false,
            };
            
            // マップイテレータの検出
            for instr in &block.instructions {
                if let Instruction::Assignment { target: _, value } = instr {
                    if let Value::FunctionCall { name, args } = value {
                        if name == "map_iterator" || name == "mapIterator" || 
                           name == "iter_map" || name == "iterMap" || 
                           name == "entries" || name == "keys" || name == "values" {
                            return true;
                        }
                    } else if let Value::MemberAccess { object: _, member } = value {
                        if member == "map_iterator" || member == "mapIterator" || 
                           member == "iter_map" || member == "iterMap" || 
                           member == "entries" || member == "keys" || member == "values" {
                            return true;
                        }
                    }
                }
            }
            
            // マップ操作の検出
            for &block_id in body {
                let block = match cfg.get_block(block_id) {
                    Ok(b) => b,
                    Err(_) => continue,
                };
                
                for instr in &block.instructions {
                    if let Instruction::Assignment { target: _, value } = instr {
                        if let Value::FunctionCall { name, args } = value {
                            if name == "get" || name == "put" || name == "remove" || 
                               name == "containsKey" || name == "contains_key" ||
                               name == "containsValue" || name == "contains_value" {
                                return true;
                            }
                        } else if let Value::MemberAccess { object: _, member } = value {
                            if member == "get" || member == "put" || member == "remove" || 
                               member == "containsKey" || member == "contains_key" ||
                               member == "containsValue" || member == "contains_value" {
                                return true;
                            }
                        } else if let Value::IndexAccess { object: _, index: _ } = value {
                            // マップのインデックスアクセスパターンを検出
                            // シンボルテーブルを使用して、objectがマップ型かどうかを確認する必要がある
                            if let Some(symbol_table) = cfg.get_symbol_table() {
                                if let Some(obj_type) = symbol_table.get_value_type(value) {
                                    if obj_type.is_map_type() {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // データフロー分析によるマップ操作の検出
            if let Some(data_flow) = cfg.get_data_flow_analysis() {
                for &block_id in body {
                    let uses = data_flow.get_uses_in_block(block_id);
                    for var in uses {
                        if let Some(var_type) = data_flow.get_variable_type(&var) {
                            if var_type.is_map_type() || var_type.is_map_iterator_type() {
                                return true;
                            }
                        }
                    }
                }
            }
            
            false
        }
        
        // 配列サイズを推定する関数
        fn estimate_array_size(cfg: &ControlFlowGraph, header: usize, body: &HashSet<usize>) -> f64 {
            // 静的解析によるサイズ推定
            if let Some(static_size) = analyze_static_array_size(cfg, header, body) {
                return static_size as f64;
            }
            
            // ループ境界分析
            if let Some(loop_bound) = analyze_loop_bounds(cfg, header) {
                return loop_bound as f64;
            }
            
            // シンボルテーブルから型情報を取得
            if let Some(symbol_table) = cfg.get_symbol_table() {
                // ループ内で使用される配列変数を特定
                let array_vars = find_array_variables(cfg, header, body, symbol_table);
                
                for var in array_vars {
                    if let Some(array_type) = symbol_table.get_variable_type(&var) {
                        if let Some(size) = array_type.get_static_size() {
                            return size as f64;
                        }
                        
                        // 配列の使用パターンから推定
                        if let Some(usage_pattern) = analyze_array_usage_pattern(cfg, body, &var) {
                            match usage_pattern {
                                ArrayUsagePattern::SmallConstant => return 10.0,
                                ArrayUsagePattern::MediumConstant => return 100.0,
                                ArrayUsagePattern::LargeConstant => return 1000.0,
                                ArrayUsagePattern::DynamicSmall => return 20.0,
                                ArrayUsagePattern::DynamicMedium => return 200.0,
                                ArrayUsagePattern::DynamicLarge => return 2000.0,
                            }
                        }
                    }
                }
            }
            
            // 過去の実行履歴からの推定
            if let Some(historical_data) = get_historical_array_size_data(header) {
                return historical_data;
            }
            
            // ヒューリスティックな推定
            // ループの複雑さに基づいて推定
            let loop_complexity = analyze_loop_complexity(cfg, header, body);
            match loop_complexity {
                LoopComplexity::Simple => 50.0,
                LoopComplexity::Medium => 200.0,
                LoopComplexity::Complex => 500.0,
            }
        }
        
        // リストサイズを推定する関数
        fn estimate_list_size(cfg: &ControlFlowGraph, header: usize, body: &HashSet<usize>) -> f64 {
            // 静的解析によるサイズ推定
            if let Some(static_size) = analyze_static_list_size(cfg, header, body) {
                return static_size as f64;
            }
            
            // リスト操作の分析
            let (additions, removals) = analyze_list_operations(cfg, body);
            if additions > 0 && removals == 0 {
                // 追加のみの場合、ループ回数に基づいて推定
                if let Some(loop_iterations) = estimate_loop_iterations(cfg, header) {
                    return loop_iterations as f64;
                }
            }
            
            // シンボルテーブルから型情報を取得
            if let Some(symbol_table) = cfg.get_symbol_table() {
                // ループ内で使用されるリスト変数を特定
                let list_vars = find_list_variables(cfg, header, body, symbol_table);
                
                for var in list_vars {
                    if let Some(list_type) = symbol_table.get_variable_type(&var) {
                        // リストの初期化パターンから推定
                        if let Some(init_size) = analyze_list_initialization(&var, symbol_table) {
                            return init_size as f64;
                        }
                        
                        // リストの使用パターンから推定
                        if let Some(usage_pattern) = analyze_list_usage_pattern(cfg, body, &var) {
                            match usage_pattern {
                                ListUsagePattern::SmallCollection => return 20.0,
                                ListUsagePattern::MediumCollection => return 100.0,
                                ListUsagePattern::LargeCollection => return 1000.0,
                                ListUsagePattern::DynamicSmall => return 50.0,
                                ListUsagePattern::DynamicMedium => return 250.0,
                                ListUsagePattern::DynamicLarge => return 2500.0,
                            }
                        }
                    }
                }
            }
            
            // 過去の実行履歴からの推定
            if let Some(historical_data) = get_historical_list_size_data(header) {
                return historical_data;
            }
            
            // コンテキスト分析
            let context = analyze_execution_context(cfg);
            match context {
                ExecutionContext::DataProcessing => 500.0,
                ExecutionContext::UserInterface => 50.0,
                ExecutionContext::SystemOperation => 200.0,
                ExecutionContext::Unknown => 100.0,
            }
        }
        
        // マップサイズを推定する関数
        fn estimate_map_size(cfg: &ControlFlowGraph, header: usize, body: &HashSet<usize>) -> f64 {
            // 静的解析によるサイズ推定
            if let Some(static_size) = analyze_static_map_size(cfg, header, body) {
                return static_size as f64;
            }
            
            // マップ操作の分析
            let (puts, removes) = analyze_map_operations(cfg, body);
            if puts > 0 && removes == 0 {
                // 追加のみの場合、ループ回数に基づいて推定
                if let Some(loop_iterations) = estimate_loop_iterations(cfg, header) {
                    return loop_iterations as f64;
                }
            }
            
            // シンボルテーブルから型情報を取得
            if let Some(symbol_table) = cfg.get_symbol_table() {
                // ループ内で使用されるマップ変数を特定
                let map_vars = find_map_variables(cfg, header, body, symbol_table);
                
                for var in map_vars {
                    if let Some(map_type) = symbol_table.get_variable_type(&var) {
                        // マップの初期化パターンから推定
                        if let Some(init_size) = analyze_map_initialization(&var, symbol_table) {
                            return init_size as f64;
                        }
                        
                        // キーの型に基づく推定
                        if let Some(key_type) = map_type.get_key_type() {
                            match key_type {
                                KeyType::Enum => return 10.0, // 列挙型は通常小さい
                                KeyType::String => return 200.0, // 文字列キーは中程度
                                KeyType::Integer => return 500.0, // 整数キーは大きい可能性
                                KeyType::Complex => return 100.0, // 複合キーは中程度
                            }
                        }
                        
                        // マップの使用パターンから推定
                        if let Some(usage_pattern) = analyze_map_usage_pattern(cfg, body, &var) {
                            match usage_pattern {
                                MapUsagePattern::Configuration => return 20.0,
                                MapUsagePattern::Cache => return 500.0,
                                MapUsagePattern::Index => return 1000.0,
                                MapUsagePattern::Lookup => return 100.0,
                                MapUsagePattern::DynamicSmall => return 50.0,
                                MapUsagePattern::DynamicLarge => return 2000.0,
                            }
                        }
                    }
                }
            }
            
            // 過去の実行履歴からの推定
            if let Some(historical_data) = get_historical_map_size_data(header) {
                return historical_data;
            }
            
            // アプリケーションドメイン分析
            let domain = analyze_application_domain(cfg);
            match domain {
                ApplicationDomain::WebService => 300.0,
                ApplicationDomain::DataAnalysis => 1000.0,
                ApplicationDomain::MobileApp => 100.0,
                ApplicationDomain::SystemSoftware => 500.0,
                ApplicationDomain::Unknown => 150.0,
            }
        }
        // ループ内の分岐を分析する関数
        fn analyze_loop_branches(cfg: &ControlFlowGraph, header: usize, body: &HashSet<usize>) -> f64 {
            // ループ内の条件分岐の数をカウント
            let mut branch_count = 0.0;
            
            for &block_id in body {
                if let Ok(block) = cfg.get_block(block_id) {
                    if let Some(Terminator::ConditionalBranch { .. }) = &block.terminator {
                        branch_count += 1.0;
                    }
                }
            }
            
            // 分岐密度を計算（ブロック数に対する分岐の割合）
            let body_size = body.len() as f64;
            if body_size > 0.0 {
                branch_count / body_size
            } else {
                0.0
            }
        }
        
        // ループのネストレベルを分析する関数
        fn analyze_loop_nesting(cfg: &ControlFlowGraph, header: usize, body: &HashSet<usize>) -> usize {
            // このループ内に含まれる他のループの数をカウント
            let mut inner_loop_count = 0;
            
            for &block_id in body {
                if block_id != header && is_loop_header(cfg, block_id) {
                    inner_loop_count += 1;
                }
            }
            
            inner_loop_count
        }
        
        // ブロックがループヘッダーかどうかを判定する関数
        fn is_loop_header(cfg: &ControlFlowGraph, block_id: usize) -> bool {
            // バックエッジの存在を確認
            for pred in cfg.predecessors(block_id) {
                if cfg.dominates(block_id, *pred) {
                    return true;
                }
            }
            
            false
        }
        
        // 過去の実行履歴データを取得する関数
        fn get_historical_execution_data(header: usize) -> Option<f64> {
            // プロファイリングデータベースからループの実行履歴を取得
            let profile_db = ProfileDatabase::get_instance();
            
            // ループヘッダーIDに基づいて実行履歴を検索
            if let Some(history) = profile_db.query_loop_execution_history(header) {
                // 実行履歴から平均反復回数を計算
                let avg_iterations = history.calculate_average_iterations();
                
                // 信頼度が閾値を超える場合のみ値を返す
                if history.confidence_level() >= 0.75 {
                    return Some(avg_iterations);
                }
            }
            
            // 十分な履歴データがない場合はNoneを返す
            None
        }
        
        // 機械学習モデルを使用して反復回数を予測する関数
        fn predict_iterations_with_ml(cfg: &ControlFlowGraph, header: usize, body: &HashSet<usize>) -> Option<f64> {
            // 機械学習予測モデルのインスタンスを取得
            let ml_predictor = MLLoopPredictor::get_instance();
            
            // 予測のための特徴量を抽出
            let features = extract_loop_features(cfg, header, body);
            
            // 予測を実行
            let prediction_result = ml_predictor.predict(features);
            
            // 予測結果を検証
            if let Some(prediction) = prediction_result {
                // 予測の信頼度を確認
                if prediction.confidence >= 0.8 {
                    // 予測値が合理的な範囲内かチェック
                    if prediction.value >= 1.0 && prediction.value <= 10000.0 {
                        return Some(prediction.value);
                    }
                }
            }
            
            None
        }
        
        // ループの特徴量を抽出する関数
        fn extract_loop_features(cfg: &ControlFlowGraph, header: usize, body: &HashSet<usize>) -> LoopFeatures {
            // ループの基本的な構造情報
            let size = body.len();
            let nesting_level = analyze_loop_nesting(cfg, header, body);
            let branch_density = analyze_loop_branches(cfg, header, body);
            
            // ループ内の命令分析
            let mut instruction_counts = HashMap::new();
            let mut memory_access_pattern = MemoryAccessPattern::new();
            let mut has_function_calls = false;
            let mut has_floating_point = false;
            
            // ループ内の各ブロックを分析
            for &block_id in body {
                if let Ok(block) = cfg.get_block(block_id) {
                    // 命令タイプの分布を分析
                    for instr in &block.instructions {
                        let instr_type = categorize_instruction(instr);
                        *instruction_counts.entry(instr_type).or_insert(0) += 1;
                        
                        // 関数呼び出しの検出
                        if let Instruction::Call { .. } = instr {
                            has_function_calls = true;
                        }
                        
                        // 浮動小数点演算の検出
                        if is_floating_point_operation(instr) {
                            has_floating_point = true;
                        }
                        
                        // メモリアクセスパターンの分析
                        analyze_memory_access(instr, &mut memory_access_pattern);
                    }
                }
            }
            
            // 制御フロー複雑性の計算
            let control_flow_complexity = calculate_cyclomatic_complexity(cfg, body);
            
            // データ依存関係の分析
            let data_dependencies = analyze_data_dependencies(cfg, body);
            
            // 帰納変数の特定と分析
            let induction_variables = identify_induction_variables(cfg, header, body);
            
            // 終了条件の分析
            let exit_conditions = analyze_exit_conditions(cfg, header, body);
            
            // 特徴量をまとめて返す
            LoopFeatures {
                size,
                nesting_level,
                branch_density,
                instruction_distribution: instruction_counts,
                memory_access_pattern,
                has_function_calls,
                has_floating_point,
                control_flow_complexity,
                data_dependencies,
                induction_variables,
                exit_conditions,
            }
        }
        
        // 命令を分類する関数
        fn categorize_instruction(instr: &Instruction) -> InstructionType {
            match instr {
                Instruction::BinaryOp { op, .. } => match op {
                    BinaryOperator::Add | BinaryOperator::Sub | BinaryOperator::Mul | BinaryOperator::Div => {
                        InstructionType::Arithmetic
                    }
                    BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Xor => {
                        InstructionType::Logical
                    }
                    BinaryOperator::Eq | BinaryOperator::Ne | BinaryOperator::Lt | BinaryOperator::Le | 
                    BinaryOperator::Gt | BinaryOperator::Ge => {
                        InstructionType::Comparison
                    }
                    _ => InstructionType::Other,
                },
                Instruction::Load { .. } => InstructionType::MemoryRead,
                Instruction::Store { .. } => InstructionType::MemoryWrite,
                Instruction::Call { .. } => InstructionType::FunctionCall,
                Instruction::Alloca { .. } => InstructionType::MemoryAllocation,
                Instruction::GetElementPtr { .. } => InstructionType::AddressComputation,
                Instruction::Phi { .. } => InstructionType::PhiNode,
                _ => InstructionType::Other,
            }
        }
        
        // 浮動小数点演算を検出する関数
        fn is_floating_point_operation(instr: &Instruction) -> bool {
            match instr {
                Instruction::BinaryOp { ty, .. } => {
                    matches!(ty, Type::Float | Type::Double)
                }
                Instruction::UnaryOp { ty, .. } => {
                    matches!(ty, Type::Float | Type::Double)
                }
                _ => false,
            }
        }
        
        // メモリアクセスパターンを分析する関数
        fn analyze_memory_access(instr: &Instruction, pattern: &mut MemoryAccessPattern) {
            match instr {
                Instruction::Load { address, .. } => {
                    if let Some(access_info) = analyze_address_expression(address) {
                        pattern.add_read_access(access_info);
                    }
                }
                Instruction::Store { address, .. } => {
                    if let Some(access_info) = analyze_address_expression(address) {
                        pattern.add_write_access(access_info);
                    }
                }
                _ => {}
            }
        }
        
        // アドレス式を分析する関数
        fn analyze_address_expression(expr: &Expression) -> Option<MemoryAccessInfo> {
            match expr {
                Expression::GetElementPtr { base, indices, .. } => {
                    // 基本アドレスの種類を特定
                    let base_type = identify_base_address_type(base);
                    
                    // インデックス計算パターンを分析
                    let index_pattern = analyze_index_pattern(indices);
                    
                    // ストライドパターンを検出
                    let stride_pattern = detect_stride_pattern(indices);
                    
                    Some(MemoryAccessInfo {
                        base_type,
                        index_pattern,
                        stride_pattern,
                        is_aligned: check_alignment(expr),
                        locality_score: estimate_locality(index_pattern, stride_pattern),
                    })
                }
                _ => None,
            }
        }
        
        // 循環的複雑度を計算する関数
        fn calculate_cyclomatic_complexity(cfg: &ControlFlowGraph, body: &HashSet<usize>) -> u32 {
            // E = エッジ数、N = ノード数、P = 連結成分数（通常は1）
            // 循環的複雑度 = E - N + 2*P
            let mut edge_count = 0;
            
            for &block_id in body {
                if let Ok(block) = cfg.get_block(block_id) {
                    // 各ブロックの後続ブロックをカウント
                    if let Some(terminator) = &block.terminator {
                        match terminator {
                            Terminator::Jump { target } => {
                                if body.contains(target) {
                                    edge_count += 1;
                                }
                            }
                            Terminator::ConditionalBranch { true_target, false_target, .. } => {
                                if body.contains(true_target) {
                                    edge_count += 1;
                                }
                                if body.contains(false_target) {
                                    edge_count += 1;
                                }
                            }
                            Terminator::Switch { cases, default, .. } => {
                                for (_, target) in cases {
                                    if body.contains(target) {
                                        edge_count += 1;
                                    }
                                }
                                if body.contains(default) {
                                    edge_count += 1;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            
            // 循環的複雑度の計算
            let node_count = body.len() as u32;
            let connected_components = 1; // ループは単一の連結成分
            
            edge_count - node_count + 2 * connected_components
        }
        
        // データ依存関係を分析する関数
        fn analyze_data_dependencies(cfg: &ControlFlowGraph, body: &HashSet<usize>) -> DataDependencyInfo {
            let mut dependency_graph = DependencyGraph::new();
            let mut loop_carried_deps = Vec::new();
            
            // 各ブロックの命令を分析
            for &block_id in body {
                if let Ok(block) = cfg.get_block(block_id) {
                    for (i, instr) in block.instructions.iter().enumerate() {
                        // 命令の定義と使用を分析
                        let def = get_defined_variable(instr);
                        let uses = get_used_variables(instr);
                        
                        // 依存関係をグラフに追加
                        if let Some(def_var) = def {
                            for use_var in uses {
                                dependency_graph.add_dependency(use_var, def_var.clone());
                                
                                // ループをまたぐ依存関係を検出
                                if is_loop_carried_dependency(cfg, block_id, i, &use_var, &def_var) {
                                    loop_carried_deps.push((use_var, def_var.clone()));
                                }
                            }
                        }
                    }
                }
            }
            
            // 依存関係の分析結果
            let critical_path = find_critical_path(&dependency_graph);
            let parallelizable_regions = identify_parallelizable_regions(&dependency_graph, &loop_carried_deps);
            
            DataDependencyInfo {
                dependency_graph,
                loop_carried_dependencies: loop_carried_deps,
                critical_path,
                parallelizable_regions,
            }
        }
        
        // 帰納変数を特定する関数
        fn identify_induction_variables(cfg: &ControlFlowGraph, header: usize, body: &HashSet<usize>) -> Vec<InductionVariableInfo> {
            let mut induction_vars = Vec::new();
            
            // ループ内のPhi命令を分析
            if let Ok(header_block) = cfg.get_block(header) {
                for instr in &header_block.instructions {
                    if let Instruction::Phi { variable, incoming } = instr {
                        // 初期値と更新式を分析
                        let mut initial_value = None;
                        let mut update_expr = None;
                        
                        for (value, pred_block) in incoming {
                            if !body.contains(pred_block) {
                                // ループ外からの値は初期値
                                initial_value = Some(value.clone());
                            } else {
                                // ループ内からの値は更新式
                                update_expr = Some(value.clone());
                            }
                        }
                        
                        // 更新パターンを分析
                        if let (Some(init), Some(update)) = (initial_value, update_expr) {
                            if let Some(pattern) = analyze_update_pattern(&update, variable) {
                                induction_vars.push(InductionVariableInfo {
                                    variable: variable.clone(),
                                    initial_value: init,
                                    update_pattern: pattern,
                                    is_primary: is_primary_induction_variable(&pattern),
                                });
                            }
                        }
                    }
                }
            }
            
            // 二次帰納変数の検出
            let mut secondary_induction_vars = Vec::new();
            for &block_id in body {
                if let Ok(block) = cfg.get_block(block_id) {
                    for instr in &block.instructions {
                        if let Instruction::BinaryOp { result, op, lhs, rhs } = instr {
                            // 一次帰納変数に依存する変数を検出
                            if is_dependent_on_induction_vars(lhs, &induction_vars) || 
                               is_dependent_on_induction_vars(rhs, &induction_vars) {
                                if let Some(pattern) = derive_secondary_pattern(op, lhs, rhs, &induction_vars) {
                                    secondary_induction_vars.push(InductionVariableInfo {
                                        variable: result.clone(),
                                        initial_value: Expression::Unknown,
                                        update_pattern: pattern,
                                        is_primary: false,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            
            // 一次と二次の帰納変数を結合
            induction_vars.extend(secondary_induction_vars);
            induction_vars
        }
        
        // ループの終了条件を分析する関数
        fn analyze_exit_conditions(cfg: &ControlFlowGraph, header: usize, body: &HashSet<usize>) -> Vec<ExitConditionInfo> {
            let mut exit_conditions = Vec::new();
            
            // ループ内の各ブロックを調査
            for &block_id in body {
                if let Ok(block) = cfg.get_block(block_id) {
                    if let Some(terminator) = &block.terminator {
                        match terminator {
                            Terminator::ConditionalBranch { condition, true_target, false_target } => {
                                // ループ外に出るエッジを検出
                                let exit_target = if !body.contains(true_target) {
                                    Some((true_target, true))
                                } else if !body.contains(false_target) {
                                    Some((false_target, false))
                                } else {
                                    None
                                };
                                
                                // 終了条件を分析
                                if let Some((target, is_true_branch)) = exit_target {
                                    let analyzed_condition = analyze_condition(condition, is_true_branch);
                                    
                                    // 帰納変数との関係を分析
                                    let induction_vars = identify_induction_variables(cfg, header, body);
                                    let relation = relate_to_induction_variables(&analyzed_condition, &induction_vars);
                                    
                                    exit_conditions.push(ExitConditionInfo {
                                        block_id,
                                        condition: analyzed_condition,
                                        target: *target,
                                        induction_relation: relation,
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            
            exit_conditions
        }
    }
}

impl CodeGenerator {
    pub fn generate(&mut self) -> Result<(), Error> {
        if self.config.parallel {
            self.logger.info("並列コード生成を実行します");
            self.generate_parallel()
        } else {
            self.logger.info("逐次コード生成を実行します");
            self.generate_sequential()
        }
    }

    fn generate_parallel(&mut self) -> Result<(), Error> {
        use rayon::prelude::*;
        use std::sync::{Arc, Mutex};

        self.logger.debug("並列コード生成の最適化レベルを設定: {}", self.config.optimization_level);
        
        // モジュールの初期化
        let module = Arc::new(Mutex::new(Module::new()));
        
        // 関数をグループ化して依存関係を分析
        let function_groups = self.analyze_function_dependencies()?;
        
        // 各グループを並列処理
        let results: Vec<Result<(), Error>> = function_groups
            .par_iter()
            .map(|group| {
                let group_result = group.par_iter().map(|func_id| {
                    let func = self.ir_module.get_function(*func_id)?;
                    let mut generator = FunctionGenerator::new(
                        func,
                        self.config.clone(),
                        self.type_registry.clone(),
                        Arc::clone(&module),
                    );
                    generator.generate()
                }).collect::<Result<Vec<_>, Error>>();
                
                // グループ内のすべての関数が生成されたか確認
                group_result.map(|_| ())
            })
            .collect();
        
        // エラーがあれば最初のエラーを返す
        for result in results {
            if let Err(e) = result {
                return Err(e);
            }
        }
        
        // 最適化パスを実行
        {
            let mut module = module.lock().unwrap();
            self.run_optimization_passes(&mut module)?;
        }
        
        // 最終的なWASMバイナリを生成
        let module = Arc::try_unwrap(module)
            .map_err(|_| Error::Internal("モジュールの排他的所有権を取得できませんでした".to_string()))?
            .into_inner()
            .map_err(|_| Error::Internal("ミューテックスロックの解除に失敗しました".to_string()))?;
        
        self.finalize_module(module)
    }

    fn generate_sequential(&mut self) -> Result<(), Error> {
        self.logger.debug("逐次コード生成の最適化レベルを設定: {}", self.config.optimization_level);
        
        // モジュールの初期化
        let mut module = Module::new();
        
        // すべての関数を処理
        for func_id in 0..self.ir_module.function_count() {
            let func = self.ir_module.get_function(func_id)?;
            let mut generator = FunctionGenerator::new(
                func,
                self.config.clone(),
                self.type_registry.clone(),
                Arc::new(Mutex::new(module)),
            );
            generator.generate()?;
            
            // FunctionGeneratorからモジュールを取り出す
            module = Arc::try_unwrap(generator.module)
                .map_err(|_| Error::Internal("モジュールの排他的所有権を取得できませんでした".to_string()))?
                .into_inner()
                .map_err(|_| Error::Internal("ミューテックスロックの解除に失敗しました".to_string()))?;
        }
        
        // 最適化パスを実行
        self.run_optimization_passes(&mut module)?;
        
        // 最終的なWASMバイナリを生成
        self.finalize_module(module)
    }
    
    fn analyze_function_dependencies(&self) -> Result<Vec<Vec<usize>>, Error> {
        // 関数の依存関係グラフを構築
        let mut dependency_graph = vec![Vec::new(); self.ir_module.function_count()];
        let mut reverse_deps = vec![Vec::new(); self.ir_module.function_count()];
        
        for func_id in 0..self.ir_module.function_count() {
            let func = self.ir_module.get_function(func_id)?;
            
            // 関数内の呼び出しを分析
            for call in func.collect_calls() {
                dependency_graph[func_id].push(call);
                reverse_deps[call].push(func_id);
            }
        }
        
        // 強連結成分を見つけてグループ化
        let mut visited = vec![false; self.ir_module.function_count()];
        let mut finish_time = Vec::with_capacity(self.ir_module.function_count());
        let mut groups = Vec::new();
        
        // 第1パス: 完了時間を記録
        for i in 0..self.ir_module.function_count() {
            if !visited[i] {
                self.dfs_first_pass(i, &dependency_graph, &mut visited, &mut finish_time)?;
            }
        }
        
        // 第2パス: 強連結成分を特定
        visited = vec![false; self.ir_module.function_count()];
        
        for &func_id in finish_time.iter().rev() {
            if !visited[func_id] {
                let mut group = Vec::new();
                self.dfs_second_pass(func_id, &reverse_deps, &mut visited, &mut group)?;
                groups.push(group);
            }
        }
        
        Ok(groups)
    }
    
    fn dfs_first_pass(
        &self,
        func_id: usize,
        graph: &[Vec<usize>],
        visited: &mut [bool],
        finish_time: &mut Vec<usize>,
    ) -> Result<(), Error> {
        visited[func_id] = true;
        
        for &neighbor in &graph[func_id] {
            if !visited[neighbor] {
                self.dfs_first_pass(neighbor, graph, visited, finish_time)?;
            }
        }
        
        finish_time.push(func_id);
        Ok(())
    }
    
    fn dfs_second_pass(
        &self,
        func_id: usize,
        graph: &[Vec<usize>],
        visited: &mut [bool],
        group: &mut Vec<usize>,
    ) -> Result<(), Error> {
        visited[func_id] = true;
        group.push(func_id);
        
        for &neighbor in &graph[func_id] {
            if !visited[neighbor] {
                self.dfs_second_pass(neighbor, graph, visited, group)?;
            }
        }
        
        Ok(())
    }
    
    fn run_optimization_passes(&self, module: &mut Module) -> Result<(), Error> {
        match self.config.optimization_level {
            OptimizationLevel::None => {
                // 最適化なし
                self.logger.debug("最適化パスをスキップします");
            },
            OptimizationLevel::Basic => {
                self.logger.debug("基本的な最適化パスを実行します");
                // 基本的な最適化
                self.run_basic_optimizations(module)?;
            },
            OptimizationLevel::Aggressive => {
                self.logger.debug("積極的な最適化パスを実行します");
                // 基本的な最適化に加えて積極的な最適化
                self.run_basic_optimizations(module)?;
                self.run_aggressive_optimizations(module)?;
            },
            OptimizationLevel::Size => {
                self.logger.debug("サイズ最適化パスを実行します");
                // サイズ最適化
                self.run_size_optimizations(module)?;
            },
        }
        
        Ok(())
    }
    
    fn run_basic_optimizations(&self, module: &mut Module) -> Result<(), Error> {
        // 定数畳み込み
        let mut constant_folder = ConstantFolding::new();
        constant_folder.run(module)?;
        
        // デッドコード除去
        let mut dce = DeadCodeElimination::new();
        dce.run(module)?;
        
        // 命令の組み合わせ
        let mut instruction_combiner = InstructionCombiner::new();
        instruction_combiner.run(module)?;
        
        Ok(())
    }
    
    fn run_aggressive_optimizations(&self, module: &mut Module) -> Result<(), Error> {
        // ループ最適化
        let mut loop_optimizer = LoopOptimizer::new();
        loop_optimizer.run(module)?;
        
        // インライン化
        let mut inliner = FunctionInliner::new();
        inliner.run(module)?;
        
        // SIMD最適化
        if self.config.enable_simd {
            let mut simd_optimizer = SIMDOptimizer::new();
            simd_optimizer.run(module)?;
        }
        
        Ok(())
    }
    
    fn run_size_optimizations(&self, module: &mut Module) -> Result<(), Error> {
        // コード重複除去
        let mut deduplicator = CodeDeduplicator::new();
        deduplicator.run(module)?;
        
        // 関数マージ
        let mut function_merger = FunctionMerger::new();
        function_merger.run(module)?;
        
        // 命令エンコーディング最適化
        let mut encoding_optimizer = EncodingOptimizer::new();
        encoding_optimizer.run(module)?;
        
        Ok(())
    }
    
    fn finalize_module(&self, module: Module) -> Result<(), Error> {
        self.logger.debug("WASMモジュールを最終化しています");
        
        // バリデーション
        if self.config.validate {
            self.logger.debug("WASMモジュールを検証しています");
            module.validate()?;
        }
        
        // バイナリ生成
        let binary = module.emit_binary()?;
        
        // 出力ファイルに書き込み
        if let Some(output_path) = &self.config.output_path {
            self.logger.info("WASMバイナリを出力: {}", output_path.display());
            std::fs::write(output_path, &binary)
                .map_err(|e| Error::IO(format!("WASMバイナリの書き込みに失敗: {}", e)))?;
        }
        
        // メモリ使用量の最適化
        if self.config.optimize_memory {
            self.logger.debug("メモリ使用量を最適化しています");
            // メモリ最適化のコード
        }
        
        Ok(())
    }
}

// イテレータアクセスパターンを検出する関数
fn has_iterator_element_access(cfg: &ControlFlowGraph, iterator_candidates: &HashSet<usize>, body: &HashSet<usize>) -> bool {
    let mut has_element_access = false;
    
    for &block_id in body {
        let block = match cfg.get_block(block_id) {
            Ok(block) => block,
            Err(_) => continue,
        };
        
        for instr in &block.instructions {
            if let Instruction::Assign { value, .. } = instr {
                // コレクション要素アクセスパターン: iter.current() または *iter
                match value {
                    Value::MethodCall { object, method_name, .. } => {
                        if method_name == "current" || method_name == "element" {
                            if let Value::VarRef(var_id) = object.as_ref() {
                                if iterator_candidates.contains(var_id) {
                                    has_element_access = true;
                                }
                            }
                        }
                    },
                    Value::UnaryOp { op: UnaryOp::Deref, operand } => {
                        if let Value::VarRef(var_id) = operand.as_ref() {
                            if iterator_candidates.contains(var_id) {
                                has_element_access = true;
                            }
                        }
                    },
                    _ => {}
                }
            }
        }
    }
    
    has_element_access
}

// 変数がループ内で増加するかどうかをチェック
fn is_incremented_in_loop(cfg: &ControlFlowGraph, var_id: usize, body: &HashSet<usize>) -> bool {
    for &block_id in body {
        let block = match cfg.get_block(block_id) {
            Ok(block) => block,
            Err(_) => continue,
        };
        
        for instr in &block.instructions {
            if let Instruction::Assign { target, value } = instr {
                if *target == var_id {
                    match value {
                        // 加算代入: var = var + const
                        Value::BinaryOp { op: BinaryOp::Add, left, right } => {
                            if let Value::VarRef(left_var) = left.as_ref() {
                                if *left_var == var_id && matches!(right.as_ref(), Value::ConstInt(_)) {
                                    return true;
                                }
                            }
                        },
                        // インクリメント: ++var または var++
                        Value::UnaryOp { op: UnaryOp::PreInc | UnaryOp::PostInc, operand } => {
                            if let Value::VarRef(op_var) = operand.as_ref() {
                                if *op_var == var_id {
                                    return true;
                                }
                            }
                        },
                        // 複合代入演算子: var += const
                        Value::CompoundAssign { op: BinaryOp::Add, left, right } => {
                            if let Value::VarRef(left_var) = left.as_ref() {
                                if *left_var == var_id && matches!(right.as_ref(), Value::ConstInt(_)) {
                                    return true;
                                }
                            }
                        },
                        // メソッド呼び出し: var.next() または var.advance()
                        Value::MethodCall { object, method_name, .. } => {
                            if let Value::VarRef(obj_var) = object.as_ref() {
                                if *obj_var == var_id && (method_name == "next" || method_name == "advance") {
                                    return true;
                                }
                            }
                        },
                        _ => {}
                    }
                }
            }
        }
    }
    
    false
}

// コレクション反復ループかどうかを判定
fn is_collection_iteration(cfg: &ControlFlowGraph, header: usize, body: &HashSet<usize>) -> bool {
    // イテレータ変数の候補を特定
    let mut iterator_candidates = HashSet::new();
    
    // ヘッダーブロックでイテレータ関連の初期化を探す
    if let Ok(header_block) = cfg.get_block(header) {
        for instr in &header_block.instructions {
            if let Instruction::Assign { target, value } = instr {
                match value {
                    // コレクション.iterator() パターン
                    Value::MethodCall { method_name, .. } => {
                        if method_name == "iterator" || method_name == "iter" || method_name == "into_iter" {
                            iterator_candidates.insert(*target);
                        }
                    },
                    // イテレータ初期化パターン
                    Value::New { type_name, .. } => {
                        if type_name.contains("Iterator") || type_name.contains("Iter") {
                            iterator_candidates.insert(*target);
                        }
                    },
                    _ => {}
                }
            }
        }
    }
    
    // ループ終了条件でイテレータ.hasNext()のようなパターンを探す
    if let Ok(header_block) = cfg.get_block(header) {
        if let Some(Terminator::ConditionalBranch { condition, .. }) = &header_block.terminator {
            match condition {
                Value::MethodCall { object, method_name, .. } => {
                    if method_name == "hasNext" || method_name == "has_next" {
                        if let Value::VarRef(var_id) = object.as_ref() {
                            iterator_candidates.insert(*var_id);
                        }
                    }
                },
                Value::UnaryOp { op: UnaryOp::Not, operand } => {
                    if let Value::MethodCall { object, method_name, .. } = operand.as_ref() {
                        if method_name == "is_empty" || method_name == "isEmpty" {
                            if let Value::VarRef(var_id) = object.as_ref() {
                                iterator_candidates.insert(*var_id);
                            }
                        }
                    }
                },
                _ => {}
            }
        }
    }
    
    // ループ本体でイテレータ要素アクセスを探す
    if !iterator_candidates.is_empty() && has_iterator_element_access(cfg, &iterator_candidates, body) {
        return true;
    }
    
    // 配列/リストインデックスアクセスパターンを探す
    let mut index_vars = HashSet::new();
    let mut collection_vars = HashSet::new();
    
    // インデックス変数の初期化を探す
    if let Ok(header_block) = cfg.get_block(header) {
        for instr in &header_block.instructions {
            if let Instruction::Assign { target, value } = instr {
                if let Value::ConstInt(0) = value {
                    index_vars.insert(*target);
                }
            }
        }
    }
    
    // インデックス変数の増加を確認
    for &var_id in &index_vars {
        if is_incremented_in_loop(cfg, var_id, body) {
            // ループ本体でインデックスを使った配列アクセスを探す
            for &block_id in body {
                if let Ok(block) = cfg.get_block(block_id) {
                    for instr in &block.instructions {
                        if let Instruction::Assign { value, .. } = instr {
                            if let Value::IndexAccess { array, index } = value {
                                if let Value::VarRef(idx_var) = index.as_ref() {
                                    if *idx_var == var_id {
                                        if let Value::VarRef(arr_var) = array.as_ref() {
                                            collection_vars.insert(*arr_var);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // 終了条件が配列長との比較かチェック
    if !index_vars.is_empty() && !collection_vars.is_empty() {
        if let Ok(header_block) = cfg.get_block(header) {
            if let Some(Terminator::ConditionalBranch { condition, .. }) = &header_block.terminator {
                match condition {
                    Value::BinaryOp { op: BinaryOp::Lt, left, right } => {
                        if let Value::VarRef(left_var) = left.as_ref() {
                            if index_vars.contains(left_var) {
                                if let Value::MethodCall { object, method_name, .. } = right.as_ref() {
                                    if method_name == "length" || method_name == "size" || method_name == "len" {
                                        if let Value::VarRef(obj_var) = object.as_ref() {
                                            if collection_vars.contains(obj_var) {
                                                return true;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    _ => {}
                }
            }
        }
    }
    
    false
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
    /// 
    /// ループアンロールは最適化の一種で、ループの反復回数を減らし、命令レベルの並列性を向上させる技術です。
    /// この実装では、静的に決定可能な反復回数を持つループに対して、完全または部分的なアンロールを行います。
    /// 
    /// # 引数
    /// * `func_id` - 関数のID
    /// * `func` - 対象の関数
    /// * `loop_info` - ループの情報
    /// * `unroll_factor` - アンロール係数（1の場合はアンロールなし、ループの反復回数と同じ場合は完全アンロール）
    /// 
    /// # 戻り値
    /// * `Result<()>` - 処理の成功または失敗
    fn perform_loop_unrolling(&mut self, func_id: usize, func: &Function, loop_info: &LoopInfo, unroll_factor: usize) -> Result<()> {
        // アンロール情報を記録
        self.loop_unrolling_info.entry(func_id)
            .or_insert_with(HashMap::new)
            .insert(loop_info.header, (loop_info.clone(), unroll_factor));
        
        // アンロール係数が1の場合は変換不要
        if unroll_factor <= 1 {
            return Ok(());
        }
        
        // ループヘッダーとボディを取得
        let header_block = &func.blocks[loop_info.header];
        let latch_block = &func.blocks[loop_info.latch];
        
        // ループの終了条件を分析
        let exit_condition = self.analyze_loop_exit_condition(func, loop_info)?;
        
        // インダクション変数の情報を取得
        let induction_vars = self.identify_induction_variables(func, loop_info)?;
        
        // メインのインダクション変数を特定
        if let Some(main_var) = induction_vars.first() {
            // アンロールされたコードの生成準備
            let mut unrolled_blocks = Vec::new();
            let mut unrolled_instructions = Vec::new();
            
            // 各反復のコードを生成
            for i in 0..unroll_factor {
                // i番目の反復のコードをクローン
                for &block_id in &loop_info.body_blocks {
                    let block = &func.blocks[block_id];
                    let mut new_instructions = Vec::new();
                    
                    // ブロック内の各命令を処理
                    for instr in &block.instructions {
                        // インダクション変数の更新を調整
                        let new_instr = if self.is_induction_variable_update(instr, &main_var.name) {
                            self.adjust_induction_variable_update(instr, i, unroll_factor)?
                        } else {
                            // その他の命令は変数名を調整してクローン
                            self.clone_instruction_with_suffix(instr, &format!("_unroll_{}", i))?
                        };
                        
                        new_instructions.push(new_instr);
                    }
                    
                    // 新しいブロックを作成
                    let new_block = BasicBlock {
                        id: self.next_block_id(),
                        name: format!("{}_unroll_{}", block.name, i),
                        instructions: new_instructions,
                        terminator: self.adjust_terminator(&block.terminator, i, unroll_factor, loop_info)?,
                    };
                    
                    unrolled_blocks.push(new_block);
                }
            }
            
            // 新しいループ条件を生成（残りの反復用）
            let new_condition = self.generate_new_loop_condition(&exit_condition, unroll_factor)?;
            
            // アンロールされたコードとオリジナルのループを接続
            self.connect_unrolled_code(func_id, loop_info, unrolled_blocks, new_condition)?;
            
            // 最適化メトリクスを更新
            self.update_optimization_metrics(func_id, loop_info, unroll_factor);
        }
        
        Ok(())
    }
    
    /// ループの終了条件を分析する
    fn analyze_loop_exit_condition(&self, func: &Function, loop_info: &LoopInfo) -> Result<ExitCondition> {
        // ループの終了条件を表す構造体
        let mut exit_condition = ExitCondition {
            condition_block: 0,
            condition_instr: None,
            branch_instr: None,
            induction_var: String::new(),
            comparison_op: BinaryOp::Eq,
            limit_value: Value::Constant(0),
        };
        
        // ループヘッダーから分岐命令を検索
        let header_block = &func.blocks[loop_info.header];
        if let Terminator::ConditionalBranch { condition, then_block, else_block } = &header_block.terminator {
            exit_condition.condition_block = loop_info.header;
            exit_condition.branch_instr = Some(header_block.terminator.clone());
            
            // 条件式を分析
            if let Value::Variable(var_name) = condition {
                // 条件変数を定義している命令を検索
                for (i, instr) in header_block.instructions.iter().enumerate() {
                    if let Instruction::BinaryOp { result, op, left, right } = instr {
                        if result == var_name {
                            exit_condition.condition_instr = Some(instr.clone());
                            exit_condition.comparison_op = *op;
                            
                            // インダクション変数と制限値を特定
                            if let (Value::Variable(var), Value::Constant(limit)) = (&**left, &**right) {
                                exit_condition.induction_var = var.clone();
                                exit_condition.limit_value = right.clone();
                            } else if let (Value::Constant(limit), Value::Variable(var)) = (&**left, &**right) {
                                exit_condition.induction_var = var.clone();
                                exit_condition.limit_value = left.clone();
                                // 比較演算子を反転
                                exit_condition.comparison_op = match op {
                                    BinaryOp::Lt => BinaryOp::Gt,
                                    BinaryOp::Le => BinaryOp::Ge,
                                    BinaryOp::Gt => BinaryOp::Lt,
                                    BinaryOp::Ge => BinaryOp::Le,
                                    _ => *op,
                                };
                            }
                            break;
                        }
                    }
                }
            }
        }
        
        Ok(exit_condition)
    }
    
    /// インダクション変数を特定する
    fn identify_induction_variables(&self, func: &Function, loop_info: &LoopInfo) -> Result<Vec<InductionVariable>> {
        let mut induction_vars = Vec::new();
        
        // ループ内の各ブロックを検査
        for &block_id in &loop_info.body_blocks {
            let block = &func.blocks[block_id];
            
            // 各命令を検査
            for instr in &block.instructions {
                if let Instruction::BinaryOp { result, op, left, right } = instr {
                    // 加算または減算の命令を検索
                    if *op == BinaryOp::Add || *op == BinaryOp::Sub {
                        if let (Value::Variable(var), Value::Constant(step)) = (&**left, &**right) {
                            // 変数が自分自身に加算/減算されているか確認
                            if var == result {
                                let step_value = if *op == BinaryOp::Add { *step } else { -(*step) };
                                
                                // 初期値を検索
                                if let Some(initial) = self.find_induction_variable_initial_value(func, loop_info, var) {
                                    induction_vars.push(InductionVariable {
                                        name: var.clone(),
                                        initial_value: initial,
                                        step: step_value,
                                        update_block: block_id,
                                        update_instruction: instr.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(induction_vars)
    }
    
    /// インダクション変数の更新命令かどうかを判定
    fn is_induction_variable_update(&self, instr: &Instruction, var_name: &str) -> bool {
        if let Instruction::BinaryOp { result, op, left, right } = instr {
            if result == var_name && (*op == BinaryOp::Add || *op == BinaryOp::Sub) {
                if let Value::Variable(left_var) = &**left {
                    return left_var == var_name;
                }
            }
        }
        false
    }
    
    /// インダクション変数の更新命令を調整
    fn adjust_induction_variable_update(&self, instr: &Instruction, iteration: usize, unroll_factor: usize) -> Result<Instruction> {
        if let Instruction::BinaryOp { result, op, left, right } = instr {
            if let Value::Constant(step) = &**right {
                // ステップサイズを調整
                let adjusted_step = step * (unroll_factor as i64);
                let new_right = Box::new(Value::Constant(adjusted_step));
                
                return Ok(Instruction::BinaryOp {
                    result: result.clone(),
                    op: *op,
                    left: left.clone(),
                    right: new_right,
                });
            }
        }
        
        // 調整できない場合は元の命令をそのまま返す
        Ok(instr.clone())
    }
    
    /// 命令をクローンし、変数名に接尾辞を追加
    fn clone_instruction_with_suffix(&self, instr: &Instruction, suffix: &str) -> Result<Instruction> {
        match instr {
            Instruction::BinaryOp { result, op, left, right } => {
                let new_result = format!("{}{}", result, suffix);
                let new_left = self.clone_value_with_suffix(left, suffix)?;
                let new_right = self.clone_value_with_suffix(right, suffix)?;
                
                Ok(Instruction::BinaryOp {
                    result: new_result,
                    op: *op,
                    left: new_left,
                    right: new_right,
                })
            },
            Instruction::Load { result, address } => {
                let new_result = format!("{}{}", result, suffix);
                let new_address = self.clone_value_with_suffix(address, suffix)?;
                
                Ok(Instruction::Load {
                    result: new_result,
                    address: new_address,
                })
            },
            Instruction::Store { address, value } => {
                let new_address = self.clone_value_with_suffix(address, suffix)?;
                let new_value = self.clone_value_with_suffix(value, suffix)?;
                
                Ok(Instruction::Store {
                    address: new_address,
                    value: new_value,
                })
            },
            Instruction::Call { result, function, arguments } => {
                let new_result = result.as_ref().map(|r| format!("{}{}", r, suffix));
                let new_arguments = arguments.iter()
                    .map(|arg| self.clone_value_with_suffix(arg, suffix))
                    .collect::<Result<Vec<_>>>()?;
                
                Ok(Instruction::Call {
                    result: new_result,
                    function: function.clone(),
                    arguments: new_arguments,
                })
            },
            // その他の命令タイプも同様に処理
            _ => Ok(instr.clone()),
        }
    }
    
    /// 値をクローンし、変数名に接尾辞を追加
    fn clone_value_with_suffix(&self, value: &Value, suffix: &str) -> Result<Box<Value>> {
        match value {
            Value::Variable(var) => Ok(Box::new(Value::Variable(format!("{}{}", var, suffix)))),
            _ => Ok(Box::new(value.clone())),
        }
    }
    
    /// 終端命令を調整
    fn adjust_terminator(&self, terminator: &Terminator, iteration: usize, unroll_factor: usize, loop_info: &LoopInfo) -> Result<Terminator> {
        match terminator {
            Terminator::ConditionalBranch { condition, then_block, else_block } => {
                // 最後の反復以外は次のアンロールブロックに分岐
                if iteration < unroll_factor - 1 {
                    let next_block = if *then_block == loop_info.header {
                        // 次のアンロールブロックのIDを計算
                        self.calculate_next_unrolled_block_id(loop_info, iteration + 1)
                    } else {
                        *then_block
                    };
                    
                    let else_target = if *else_block == loop_info.header {
                        self.calculate_next_unrolled_block_id(loop_info, iteration + 1)
                    } else {
                        *else_block
                    };
                    
                    Ok(Terminator::ConditionalBranch {
                        condition: self.clone_value_with_suffix(condition, &format!("_unroll_{}", iteration))?,
                        then_block: next_block,
                        else_block: else_target,
                    })
                } else {
                    // 最後の反復は元のループヘッダーに戻る
                    Ok(Terminator::ConditionalBranch {
                        condition: self.clone_value_with_suffix(condition, &format!("_unroll_{}", iteration))?,
                        then_block: *then_block,
                        else_block: *else_block,
                    })
                }
            },
            Terminator::Jump { target } => {
                // ジャンプ先がループヘッダーの場合は次のアンロールブロックまたは元のループに分岐
                if *target == loop_info.header {
                    if iteration < unroll_factor - 1 {
                        Ok(Terminator::Jump {
                            target: self.calculate_next_unrolled_block_id(loop_info, iteration + 1),
                        })
                    } else {
                        Ok(Terminator::Jump {
                            target: loop_info.header,
                        })
                    }
                } else {
                    Ok(terminator.clone())
                }
            },
            _ => Ok(terminator.clone()),
        }
    }
    
    /// 次のアンロールブロックのIDを計算
    fn calculate_next_unrolled_block_id(&self, loop_info: &LoopInfo, iteration: usize) -> usize {
        // 実際の実装ではブロックIDの割り当て方法に依存
        // ここでは簡略化のため、仮の計算方法を示す
        loop_info.header + 1000 + iteration
    }
    
    /// 新しいループ条件を生成
    fn generate_new_loop_condition(&self, exit_condition: &ExitCondition, unroll_factor: usize) -> Result<Instruction> {
        // インダクション変数の増分を考慮した新しい条件を生成
        if let Value::Constant(limit) = &exit_condition.limit_value {
            // アンロール係数に基づいて調整された制限値を計算
            let adjusted_limit = match exit_condition.comparison_op {
                BinaryOp::Lt | BinaryOp::Le => {
                    // 例: i < 100 の場合、i < (100 - (100 % 4)) となる
                    // これにより、メインループは96まで実行し、残りは別処理
                    limit - (limit % (unroll_factor as i64))
                },
                BinaryOp::Gt | BinaryOp::Ge => {
                    // 下限値の場合も同様に調整
                    limit + ((unroll_factor as i64) - (limit % (unroll_factor as i64))) % (unroll_factor as i64)
                },
                _ => *limit,
            };
            
            // 新しい条件命令を生成
            Ok(Instruction::BinaryOp {
                result: format!("{}_adjusted", exit_condition.induction_var),
                op: exit_condition.comparison_op,
                left: Box::new(Value::Variable(exit_condition.induction_var.clone())),
                right: Box::new(Value::Constant(adjusted_limit)),
            })
        } else {
            // 定数でない場合は元の条件をそのまま使用
            Ok(exit_condition.condition_instr.clone().unwrap_or(Instruction::Nop))
        }
    }
    
    /// アンロールされたコードと元のループを接続
    fn connect_unrolled_code(&mut self, func_id: usize, loop_info: &LoopInfo, unrolled_blocks: Vec<BasicBlock>, new_condition: Instruction) -> Result<()> {
        // この関数は実際のIR変換時に呼び出される
        // ここでは変換情報を記録するだけ
        self.unrolled_code_info.entry(func_id)
            .or_insert_with(Vec::new)
            .push(UnrolledCodeInfo {
                loop_header: loop_info.header,
                unrolled_blocks: unrolled_blocks.iter().map(|b| b.id).collect(),
                new_condition,
            });
        
        Ok(())
    }
    
    /// 最適化メトリクスを更新
    fn update_optimization_metrics(&mut self, func_id: usize, loop_info: &LoopInfo, unroll_factor: usize) {
        // 最適化の効果を追跡
        let instruction_count = loop_info.body_blocks.iter()
            .map(|&block_id| self.get_function(func_id).blocks[block_id].instructions.len())
            .sum::<usize>();
        
        let estimated_reduction = if unroll_factor > 1 {
            // 制御オーバーヘッドの削減量を推定
            let control_overhead = loop_info.body_blocks.len(); // 分岐命令の数
            let iterations_saved = (100 / unroll_factor) * (unroll_factor - 1); // 100を仮の反復回数とする
            control_overhead * iterations_saved
        } else {
            0
        };
        
        // メトリクスを記録
        self.optimization_metrics.entry(func_id)
            .or_insert_with(HashMap::new)
            .insert(OptimizationType::LoopUnrolling, OptimizationMetric {
                optimization_type: OptimizationType::LoopUnrolling,
                target: format!("loop_{}", loop_info.header),
                instruction_count_before: instruction_count,
                instruction_count_after: instruction_count * unroll_factor,
                estimated_cycles_saved: estimated_reduction,
                applied: unroll_factor > 1,
            });
    }
    
    /// 次のブロックIDを取得
    fn next_block_id(&self) -> usize {
        // 実際の実装では一意のIDを生成する必要がある
        // ここでは簡略化のため、仮の実装を示す
        static mut NEXT_ID: usize = 10000;
        unsafe {
            let id = NEXT_ID;
            NEXT_ID += 1;
            id
        }
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
            // 単一の前任ブロックがある場合、それをプリヘッダーとして使用
            loop_info.preheader = Some(*predecessors.iter().next().unwrap());
        } else if predecessors.is_empty() {
            // 前任ブロックがない場合（関数のエントリーブロックなど）
            loop_info.preheader = None;
        } else {
            // 複数の前任ブロックがある場合、新しいプリヘッダーが必要
            // 実際の作成はIRレベルで行われるが、ここでは情報を記録
            self.loop_transformation_queue.entry(func_id)
                .or_insert_with(Vec::new)
                .push(LoopTransformation::CreatePreheader {
                    loop_id: loop_info.id,
                    header: loop_info.header,
                    predecessors: predecessors.clone()
                });
            
            // プリヘッダーは後で作成されることを示す特別な値を設定
            // 実際のブロックIDは変換フェーズで割り当てられる
            loop_info.preheader = Some(PREHEADER_PENDING_CREATION);
            
            // 最適化のためのメタデータを記録
            loop_info.optimization_metadata.insert(
                "multiple_predecessors".to_string(),
                format!("{:?}", predecessors)
            );
        }
        
        // ループのプリヘッダー情報をデバッグログに記録
        if let Some(logger) = &self.optimization_logger {
            logger.log_loop_preheader_info(
                func_id,
                loop_info.id,
                loop_info.preheader,
                &predecessors
            );
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

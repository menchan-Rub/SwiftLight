// SwiftLight IR生成モジュール
//
// このモジュールは、AST（抽象構文木）からLLVM IRを生成するための
// 中間表現とユーティリティを提供します。

pub mod representation;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use inkwell::{
    context::Context,
    module::Module as LLVMModule,
    builder::Builder,
    values::{FunctionValue, BasicValueEnum, PointerValue, AnyValueEnum, IntValue, FloatValue, BasicValue},
    types::{BasicTypeEnum, BasicType, AnyTypeEnum, StructType, FunctionType},
    basic_block::BasicBlock as LLVMBasicBlock,
    AddressSpace,
};
use inkwell::values::BasicMetadataValueEnum;
use crate::frontend::ast::{self, Program, Declaration, Statement, Expression, NodeId, TypeAnnotation, TypeKind};
use crate::frontend::ast::{ExpressionKind, StatementKind, DeclarationKind, Identifier, Function};
use crate::frontend::ast::{VariableDeclaration, ConstantDeclaration, Parameter, Struct, Enum, Trait};
use crate::frontend::ast::{TypeAlias, Implementation, Import, BinaryOperator, UnaryOperator, Literal, LiteralKind};
use crate::frontend::error::{Result, CompilerError, Diagnostic, SourceLocation};
use crate::frontend::semantic::type_checker::TypeCheckResult;

// 公開型の再エクスポート
pub use representation::{Module, Function, BasicBlock, Instruction, Value, Type, Operand, OpCode};

/// LLVM IR生成器
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

impl<'ctx> IRGenerator<'ctx> {
    /// 新しいIRジェネレーターを作成
    pub fn new(type_info: &TypeCheckResult) -> Self {
        let context = Context::create();
        let module_name = "swiftlight_module";
        let llvm_module = context.create_module(module_name);
        let builder = context.create_builder();
        
        Self {
            context: &context,
            llvm_module,
            builder,
            type_info: type_info.clone(),
            module: Module::new(module_name),
            values: HashMap::new(),
            blocks: HashMap::new(),
            current_function: None,
            current_block: None,
            variables: HashMap::new(),
            functions: HashMap::new(),
            structs: HashMap::new(),
            temp_counter: 0,
            errors: Vec::new(),
            debug_info: true,
            current_loop_condition: None,
            current_loop_exit: None,
            current_exception_handler: None,
            current_type_parameters: HashMap::new(),
        }
    }
    
    /// 生成中にエラーを追加
    fn add_error(&mut self, message: impl Into<String>, location: Option<SourceLocation>) {
        self.errors.push(CompilerError::code_generation_error(message, location));
    }
    
    /// 一時変数名を生成
    fn generate_temp_name(&mut self, prefix: &str) -> String {
        let name = format!("{}.{}", prefix, self.temp_counter);
        self.temp_counter += 1;
        name
    }
    
    /// プログラムからIRモジュールを生成
    pub fn generate_module(&mut self, program: &Program) -> Result<Module> {
        // 初期化
        self.initialize_module(program)?;
        
        // 型定義の事前宣言（相互参照を解決するため）
        self.predeclare_types(program)?;
        
        // 関数シグネチャの事前宣言（相互参照を解決するため）
        self.predeclare_functions(program)?;
        
        // 宣言処理
        for declaration in &program.declarations {
            if let Err(e) = self.generate_declaration(declaration) {
                self.errors.push(e);
            }
        }
        
        // トップレベルの文を処理
        let entry_fn = self.generate_program_entry(program)?;
        
        // モジュールのクリーンアップと検証
        self.finalize_module()?;
        
        // エラーがあれば報告
        if !self.errors.is_empty() {
            let err_msg = format!("{} errors occurred during IR generation", self.errors.len());
            return Err(CompilerError::code_generation_error(err_msg, None).with_cause(self.errors[0].clone()));
        }
        
        Ok(self.module.clone())
    }
    
    /// モジュール初期化
    fn initialize_module(&mut self, program: &Program) -> Result<()> {
        // モジュール名を設定（ファイル名ベース）
        let module_name = program.file_name.clone();
        self.module.name = module_name.clone();
        self.module.set_source_file(program.file_name.clone());
        
        // LLVM データレイアウトの設定
        self.llvm_module.set_data_layout("e-m:e-i64:64-f80:128-n8:16:32:64-S128");
        
        // ターゲットトリプルの設定
        self.llvm_module.set_triple("x86_64-unknown-linux-gnu");
        
        // 標準ライブラリ関数の宣言
        self.declare_runtime_functions()?;
        
        Ok(())
    }
    
    /// ランタイム関数の宣言
    fn declare_runtime_functions(&mut self) -> Result<()> {
        // println関数の宣言 (void println(char*))
        let void_type = self.context.void_type();
        let str_type = self.context.i8_type().ptr_type(AddressSpace::Generic);
        let println_type = void_type.fn_type(&[str_type.into()], false);
        let println_fn = self.llvm_module.add_function("println", println_type, None);
        self.functions.insert("println".to_string(), println_fn);
        
        // メモリ操作関数の宣言
        // malloc: void* malloc(size_t)
        let i64_type = self.context.i64_type();
        let ptr_type = self.context.i8_type().ptr_type(AddressSpace::Generic);
        let malloc_type = ptr_type.fn_type(&[i64_type.into()], false);
        let malloc_fn = self.llvm_module.add_function("malloc", malloc_type, None);
        self.functions.insert("malloc".to_string(), malloc_fn);
        
        // free: void free(void*)
        let free_type = void_type.fn_type(&[ptr_type.into()], false);
        let free_fn = self.llvm_module.add_function("free", free_type, None);
        self.functions.insert("free".to_string(), free_fn);
        
        // memcpy: void* memcpy(void* dest, void* src, size_t n)
        let memcpy_type = ptr_type.fn_type(&[ptr_type.into(), ptr_type.into(), i64_type.into()], false);
        let memcpy_fn = self.llvm_module.add_function("memcpy", memcpy_type, None);
        self.functions.insert("memcpy".to_string(), memcpy_fn);
        
        // 基本的な数学関数の宣言
        let f64_type = self.context.f64_type();
        let math_fn_type = f64_type.fn_type(&[f64_type.into()], false);
        
        // double sqrt(double)
        let sqrt_fn = self.llvm_module.add_function("sqrt", math_fn_type, None);
        self.functions.insert("sqrt".to_string(), sqrt_fn);
        
        // double sin(double)
        let sin_fn = self.llvm_module.add_function("sin", math_fn_type, None);
        self.functions.insert("sin".to_string(), sin_fn);
        
        // double cos(double)
        let cos_fn = self.llvm_module.add_function("cos", math_fn_type, None);
        self.functions.insert("cos".to_string(), cos_fn);
        
        // プロファイリングとインストルメンテーション関数を宣言
        self.declare_instrumentation_functions()?;
        
        Ok(())
    }
    
    /// 型定義の事前宣言
    fn predeclare_types(&mut self, program: &Program) -> Result<()> {
        // 構造体宣言を収集し、前方宣言
        for declaration in &program.declarations {
            if let DeclarationKind::Struct(struct_decl) = &declaration.kind {
                let struct_name = &struct_decl.name.name;
                
                // LLVM構造体型の作成（フィールドなしで前方宣言）
                let struct_type = self.context.opaque_struct_type(struct_name);
                self.structs.insert(struct_name.clone(), struct_type);
                
                // 中間表現のモジュールにも追加
                self.module.add_struct(struct_name.clone(), Vec::new());
            }
        }
        
        // 構造体フィールドの設定
        for declaration in &program.declarations {
            if let DeclarationKind::Struct(struct_decl) = &declaration.kind {
                let struct_name = &struct_decl.name.name;
                
                if let Some(&struct_type) = self.structs.get(struct_name) {
                    // フィールド型を解決
                    let mut field_types = Vec::new();
                    let mut ir_field_types = Vec::new();
                    
                    for field in &struct_decl.fields {
                        if let Some(field_type) = self.type_info.get_node_type(field.type_annotation.id) {
                            if let Ok(llvm_type) = self.convert_type_from_annotation(&field_type) {
                                field_types.push(llvm_type);
                                
                                // 中間表現の型も収集
                                let ir_type = self.convert_to_ir_type(&field_type)?;
                                ir_field_types.push(ir_type);
                            } else {
                                self.add_error(
                                    format!("構造体 '{}' のフィールド '{}' の型を解決できません", 
                                            struct_name, field.name.name),
                                    field.location.clone()
                                );
                            }
                        } else {
                            self.add_error(
                                format!("構造体 '{}' のフィールド '{}' の型情報がありません", 
                                        struct_name, field.name.name),
                                field.location.clone()
                            );
                        }
                    }
                    
                    // 構造体の本体を設定
                    struct_type.set_body(&field_types, false);
                    
                    // 中間表現の構造体フィールドも更新
                    self.module.structs.insert(struct_name.clone(), ir_field_types);
                }
            }
        }
        
        Ok(())
    }
    
    /// 関数シグネチャの事前宣言
    fn predeclare_functions(&mut self, program: &Program) -> Result<()> {
        for declaration in &program.declarations {
            if let DeclarationKind::Function(func_decl) = &declaration.kind {
                let func_name = &func_decl.name.name;
                
                // 関数の型を解決
                let mut param_types = Vec::new();
                for param in &func_decl.parameters {
                    if let Some(param_type) = self.type_info.get_node_type(param.type_annotation.as_ref().unwrap().id) {
                        if let Ok(llvm_param_type) = self.convert_type_from_annotation(&param_type) {
                            param_types.push(llvm_param_type);
                        } else {
                            self.add_error(
                                format!("関数 '{}' のパラメータ '{}' の型を解決できません", 
                                        func_name, param.name.name),
                                param.location.clone()
                            );
                        }
                    } else {
                        self.add_error(
                            format!("関数 '{}' のパラメータ '{}' の型情報がありません", 
                                    func_name, param.name.name),
                            param.location.clone()
                        );
                    }
                }
                
                // 戻り値型の解決
                let return_type = if let Some(ret_type) = &func_decl.return_type {
                    if let Some(type_info) = self.type_info.get_node_type(ret_type.id) {
                        match self.convert_type_from_annotation(&type_info) {
                            Ok(t) => t,
                            Err(_) => {
                                self.add_error(
                                    format!("関数 '{}' の戻り値型を解決できません", func_name),
                                    ret_type.location.clone()
                                );
                                self.context.void_type().into()
                            }
                        }
                    } else {
                        self.add_error(
                            format!("関数 '{}' の戻り値型情報がありません", func_name),
                            ret_type.location.clone()
                        );
                        self.context.void_type().into()
                    }
                } else {
                    // 戻り値型の指定がない場合はvoid
                    self.context.void_type().into()
                };
                
                // LLVM関数型の作成
                let fn_type = match return_type {
                    BasicTypeEnum::IntType(t) => t.fn_type(&param_types, false),
                    BasicTypeEnum::FloatType(t) => t.fn_type(&param_types, false),
                    BasicTypeEnum::PointerType(t) => t.fn_type(&param_types, false),
                    BasicTypeEnum::StructType(t) => t.fn_type(&param_types, false),
                    BasicTypeEnum::ArrayType(t) => t.fn_type(&param_types, false),
                    _ => self.context.void_type().fn_type(&param_types, false),
                };
                
                // LLVM関数を追加
                let function = self.llvm_module.add_function(func_name, fn_type, None);
                self.functions.insert(func_name.clone(), function);
                
                // 中間表現の関数も作成
                let mut ir_function = Function::new(func_name.clone(), self.convert_to_ir_type_from_basic_type(return_type)?);
                
                // パラメータも追加
                for (i, param) in func_decl.parameters.iter().enumerate() {
                    if let Some(param_type) = self.type_info.get_node_type(param.type_annotation.as_ref().unwrap().id) {
                        let ir_param_type = self.convert_to_ir_type(&param_type)?;
                        let ir_param = representation::Parameter::new(
                            param.name.name.clone(),
                            ir_param_type,
                            false // バイ・リファレンスかどうかはここでは判断できない
                        );
                        ir_function.add_parameter(ir_param);
                    }
                }
                
                // 中間表現のモジュールに関数を追加
                self.module.add_function(ir_function);
            }
        }
        
        Ok(())
    }
    
    /// プログラムのエントリーポイントを生成
    fn generate_program_entry(&mut self, program: &Program) -> Result<FunctionValue<'ctx>> {
        // main関数の生成
        let void_type = self.context.void_type();
        let main_type = self.context.i32_type().fn_type(&[], false);
        let main_fn = self.llvm_module.add_function("main", main_type, None);
        
        // エントリーブロックの作成
        let entry_block = self.context.append_basic_block(main_fn, "entry");
        self.builder.position_at_end(entry_block);
        
        // 現在の関数コンテキストを設定
        self.current_function = Some(main_fn);
        self.current_block = Some(entry_block);
        
        // トップレベルの文を処理
        for stmt in &program.statements {
            if let Err(e) = self.generate_statement(stmt) {
                self.errors.push(e);
            }
        }
        
        // main関数の戻り値（成功を示す0）
        let ret_val = self.context.i32_type().const_int(0, false);
        self.builder.build_return(Some(&ret_val));
        
        // 中間表現のmain関数も作成
        let mut ir_main = Function::new("main", Type::Integer(32));
        let mut ir_entry = BasicBlock::new("entry");
        
        // returnの命令を追加
        let ret_inst = Instruction::new(
            OpCode::Return,
            None,
            Type::Void,
            vec![Operand::Constant(Value::Integer(0))],
        );
        ir_entry.add_instruction(ret_inst);
        
        // 基本ブロックを追加
        ir_main.add_block(ir_entry);
        
        // 中間表現のモジュールに追加
        self.module.add_function(ir_main);
        
        Ok(main_fn)
    }
    
    /// モジュールの最終化
    fn finalize_module(&mut self) -> Result<()> {
        // モジュールの検証
        if let Err(err) = self.llvm_module.verify() {
            return Err(CompilerError::code_generation_error(
                format!("LLVM モジュール検証エラー: {}", err),
                None
            ));
        }
        
        Ok(())
    }
    
    /// TypeAnnotationからLLVM型に変換
    fn convert_type_from_annotation(&self, type_ann: &TypeAnnotation) -> Result<BasicTypeEnum<'ctx>> {
        match &type_ann.kind {
            TypeKind::Int => Ok(self.context.i64_type().into()),
            TypeKind::Float => Ok(self.context.f64_type().into()),
            TypeKind::Bool => Ok(self.context.bool_type().into()),
            TypeKind::String => {
                // 文字列はi8のポインタとして表現
                Ok(self.context.i8_type().ptr_type(AddressSpace::Generic).into())
            },
            TypeKind::Char => Ok(self.context.i8_type().into()),
            TypeKind::Void => {
                // voidはLLVMではBasicTypeEnumとして表現できないため、便宜上i8*として扱う
                Err(CompilerError::code_generation_error(
                    "Void型は値として扱えません",
                    type_ann.location.clone()
                ))
            },
            TypeKind::Array(elem_type) => {
                if let Some(elem_type_info) = self.type_info.get_node_type(elem_type.id) {
                    let llvm_elem_type = self.convert_type_from_annotation(&elem_type_info)?;
                    
                    // 配列はポインタとして表現
                    match llvm_elem_type {
                        BasicTypeEnum::IntType(t) => Ok(t.ptr_type(AddressSpace::Generic).into()),
                        BasicTypeEnum::FloatType(t) => Ok(t.ptr_type(AddressSpace::Generic).into()),
                        BasicTypeEnum::PointerType(t) => Ok(t.ptr_type(AddressSpace::Generic).into()),
                        BasicTypeEnum::StructType(t) => Ok(t.ptr_type(AddressSpace::Generic).into()),
                        BasicTypeEnum::ArrayType(t) => Ok(t.ptr_type(AddressSpace::Generic).into()),
                    }
                } else {
                    Err(CompilerError::code_generation_error(
                        "配列要素の型情報がありません",
                        type_ann.location.clone()
                    ))
                }
            },
            TypeKind::Named(ident) => {
                // 名前付き型の解決
                let type_name = &ident.name;
                
                if type_name == "Int" {
                    return Ok(self.context.i64_type().into());
                } else if type_name == "Float" {
                    return Ok(self.context.f64_type().into());
                } else if type_name == "Bool" {
                    return Ok(self.context.bool_type().into());
                } else if type_name == "String" {
                    return Ok(self.context.i8_type().ptr_type(AddressSpace::Generic).into());
                } else if type_name == "Char" {
                    return Ok(self.context.i8_type().into());
                }
                
                // 構造体型の解決
                if let Some(&struct_type) = self.structs.get(type_name) {
                    Ok(struct_type.into())
                } else {
                    Err(CompilerError::code_generation_error(
                        format!("未知の型名 '{}'", type_name),
                        type_ann.location.clone()
                    ))
                }
            },
            TypeKind::Function(param_types, ret_type) => {
                // 関数型はポインタとして表現
                let mut llvm_param_types = Vec::new();
                
                for param_type in param_types {
                    if let Some(param_type_info) = self.type_info.get_node_type(param_type.id) {
                        let llvm_param_type = self.convert_type_from_annotation(&param_type_info)?;
                        llvm_param_types.push(llvm_param_type);
                    } else {
                        return Err(CompilerError::code_generation_error(
                            "関数パラメータの型情報がありません",
                            type_ann.location.clone()
                        ));
                    }
                }
                
                // 戻り値型の解決
                let ret_type_info = self.type_info.get_node_type(ret_type.id).ok_or_else(|| {
                    CompilerError::code_generation_error(
                        "関数戻り値型の型情報がありません",
                        type_ann.location.clone()
                    )
                })?;
                
                let llvm_ret_type = match ret_type_info.kind {
                    TypeKind::Void => AnyTypeEnum::VoidType(self.context.void_type()),
                    _ => {
                        let basic_type = self.convert_type_from_annotation(&ret_type_info)?;
                        basic_type.into()
                    }
                };
                
                // 関数型の作成
                let fn_type = match llvm_ret_type {
                    AnyTypeEnum::VoidType(t) => t.fn_type(&llvm_param_types, false),
                    AnyTypeEnum::IntType(t) => t.fn_type(&llvm_param_types, false),
                    AnyTypeEnum::FloatType(t) => t.fn_type(&llvm_param_types, false),
                    AnyTypeEnum::PointerType(t) => t.fn_type(&llvm_param_types, false),
                    AnyTypeEnum::StructType(t) => t.fn_type(&llvm_param_types, false),
                    AnyTypeEnum::ArrayType(t) => t.fn_type(&llvm_param_types, false),
                    _ => {
                        return Err(CompilerError::code_generation_error(
                            "サポートされていない関数戻り値型です",
                            type_ann.location.clone()
                        ));
                    }
                };
                
                // 関数型はポインタとして表現
                Ok(fn_type.ptr_type(AddressSpace::Generic).into())
            },
            // その他の型も必要に応じて追加
            _ => Err(CompilerError::code_generation_error(
                format!("サポートされていない型です: {:?}", type_ann.kind),
                type_ann.location.clone()
            )),
        }
    }
    
    /// NodeIdに関連付けられた型からLLVM型に変換
    fn convert_type(&self, node_id: NodeId) -> Result<BasicTypeEnum<'ctx>> {
        if let Some(type_ann) = self.type_info.get_node_type(node_id) {
            self.convert_type_from_annotation(&type_ann)
        } else {
            Err(CompilerError::code_generation_error(
                format!("ノードID {}の型情報が見つかりません", node_id),
                None
            ))
        }
    }
    
    /// TypeAnnotationからIR型に変換
    fn convert_to_ir_type(&self, type_ann: &TypeAnnotation) -> Result<Type> {
        match &type_ann.kind {
            TypeKind::Int => Ok(Type::Integer(64)),
            TypeKind::Float => Ok(Type::Double),
            TypeKind::Bool => Ok(Type::Boolean),
            TypeKind::String => Ok(Type::String),
            TypeKind::Char => Ok(Type::Char),
            TypeKind::Void => Ok(Type::Void),
            TypeKind::Array(elem_type) => {
                if let Some(elem_type_info) = self.type_info.get_node_type(elem_type.id) {
                    let ir_elem_type = self.convert_to_ir_type(&elem_type_info)?;
                    Ok(Type::Array(Box::new(ir_elem_type), 0)) // 動的サイズ配列
                } else {
                    Err(CompilerError::code_generation_error(
                        "配列要素の型情報がありません",
                        type_ann.location.clone()
                    ))
                }
            },
            TypeKind::Named(ident) => {
                // 名前付き型の解決
                let type_name = &ident.name;
                
                if type_name == "Int" {
                    return Ok(Type::Integer(64));
                } else if type_name == "Float" {
                    return Ok(Type::Double);
                } else if type_name == "Bool" {
                    return Ok(Type::Boolean);
                } else if type_name == "String" {
                    return Ok(Type::String);
                } else if type_name == "Char" {
                    return Ok(Type::Char);
                }
                
                // 構造体型の解決
                if self.module.structs.contains_key(type_name) {
                    // 構造体のフィールドタイプはすでに登録済み
                    let field_types = self.module.structs.get(type_name).unwrap().clone();
                    Ok(Type::Struct(type_name.clone(), field_types))
                } else {
                    // 中間表現には存在しない構造体
                    Ok(Type::Struct(type_name.clone(), Vec::new()))
                }
            },
            TypeKind::Function(param_types, ret_type) => {
                // パラメータ型の変換
                let mut ir_param_types = Vec::new();
                
                for param_type in param_types {
                    if let Some(param_type_info) = self.type_info.get_node_type(param_type.id) {
                        let ir_param_type = self.convert_to_ir_type(&param_type_info)?;
                        ir_param_types.push(ir_param_type);
                    } else {
                        return Err(CompilerError::code_generation_error(
                            "関数パラメータの型情報がありません",
                            type_ann.location.clone()
                        ));
                    }
                }
                
                // 戻り値型の解決
                let ret_type_info = self.type_info.get_node_type(ret_type.id).ok_or_else(|| {
                    CompilerError::code_generation_error(
                        "関数戻り値型の型情報がありません",
                        type_ann.location.clone()
                    )
                })?;
                
                let ir_ret_type = self.convert_to_ir_type(&ret_type_info)?;
                
                Ok(Type::Function(ir_param_types, Box::new(ir_ret_type)))
            },
            TypeKind::Optional(inner_type) => {
                if let Some(inner_type_info) = self.type_info.get_node_type(inner_type.id) {
                    let ir_inner_type = self.convert_to_ir_type(&inner_type_info)?;
                    Ok(Type::Optional(Box::new(ir_inner_type)))
                } else {
                    Err(CompilerError::code_generation_error(
                        "オプショナル型の内部型情報がありません",
                        type_ann.location.clone()
                    ))
                },
            // その他の型も必要に応じて追加
            _ => Err(CompilerError::code_generation_error(
                format!("サポートされていない型です: {:?}", type_ann.kind),
                type_ann.location.clone()
            )),
        }
    }
    
    /// BasicTypeEnumからIR型に変換
    fn convert_to_ir_type_from_basic_type(&self, basic_type: BasicTypeEnum<'ctx>) -> Result<Type> {
        match basic_type {
            BasicTypeEnum::IntType(_) => Ok(Type::Integer(64)),
            BasicTypeEnum::FloatType(_) => Ok(Type::Double),
            BasicTypeEnum::PointerType(_) => Ok(Type::Pointer(Box::new(Type::Unknown))),
            BasicTypeEnum::StructType(s) => {
                let name = s.get_name()
                    .map(|name_str| name_str.to_string_lossy().to_string())
                    .unwrap_or_else(|| "anonymous_struct".to_string());
                
                Ok(Type::Struct(name, Vec::new()))
            },
            BasicTypeEnum::ArrayType(a) => {
                let elem_count = a.len();
                Ok(Type::Array(Box::new(Type::Unknown), elem_count as usize))
            },
        }
    }
    
    // LLVM値からIR値に変換
    fn convert_to_ir_value(&self, value: BasicValueEnum<'ctx>) -> Value {
        match value {
            BasicValueEnum::IntValue(i) => {
                if i.get_type().get_bit_width() == 1 {
                    // Booleanの場合
                    let bool_val = i.get_zero_extended_constant()
                        .map(|v| v != 0)
                        .unwrap_or(false);
                    
                    Value::Boolean(bool_val)
                } else {
                    // 整数の場合
                    let int_val = i.get_zero_extended_constant()
                        .unwrap_or(0) as i64;
                    
                    Value::Integer(int_val)
                }
            },
            BasicValueEnum::FloatValue(f) => {
                // 浮動小数点の場合
                let float_val = f.get_constant()
                    .map(|v| v as f64)
                    .unwrap_or(0.0);
                
                Value::Float(float_val)
            },
            BasicValueEnum::PointerValue(_) => {
                // ポインタは適切に扱うのが難しいため、ローカル参照として扱う
                Value::LocalRef("ptr".to_string())
            },
            BasicValueEnum::StructValue(_) => {
                // 構造体も簡単な表現を使用
                Value::Struct("struct".to_string(), Vec::new())
            },
            BasicValueEnum::ArrayValue(_) => {
                // 配列も簡単な表現を使用
                Value::Array(Vec::new())
            },
        }
    }
    
    /// 宣言の処理
    fn generate_declaration(&mut self, declaration: &Declaration) -> Result<()> {
        match &declaration.kind {
            DeclarationKind::Function(func) => self.generate_function_declaration(func, declaration)?,
            DeclarationKind::Variable(var) => self.generate_global_variable(var, declaration)?,
            DeclarationKind::Constant(constant) => self.generate_global_constant(constant, declaration)?,
            DeclarationKind::Struct(struct_decl) => self.generate_struct_declaration(struct_decl, declaration)?,
            DeclarationKind::Enum(enum_decl) => self.generate_enum_declaration(enum_decl, declaration)?,
            DeclarationKind::Trait(trait_decl) => self.generate_trait_declaration(trait_decl, declaration)?,
            DeclarationKind::Implementation(impl_decl) => self.generate_implementation(impl_decl, declaration)?,
            DeclarationKind::TypeAlias(alias) => self.generate_type_alias(alias, declaration)?,
            DeclarationKind::Import(import) => self.generate_import(import, declaration)?,
        }
        
        Ok(())
    }
    
    /// 文の処理
    fn generate_statement(&mut self, statement: &Statement) -> Result<()> {
        match &statement.kind {
            StatementKind::Expression(expr) => {
                // 式の評価（結果は破棄される）
                self.generate_expression(expr)?;
            },
            StatementKind::Block(statements) => {
                // ブロック内の各文を評価
                for stmt in statements {
                    self.generate_statement(stmt)?;
                }
            },
            StatementKind::Declaration(decl) => {
                // 宣言を処理
                self.generate_declaration(decl)?;
            },
            StatementKind::If(condition, then_branch, else_branch) => {
                // if文の処理
                self.generate_if_statement(condition, then_branch, else_branch.as_deref())?;
            },
            StatementKind::While(condition, body) => {
                // while文の処理
                self.generate_while_statement(condition, body)?;
            },
            StatementKind::For(variable, range, body) => {
                // for文の処理
                self.generate_for_statement(variable, range, body)?;
            },
            StatementKind::Return(expr) => {
                // return文の処理
                self.generate_return_statement(expr.as_ref())?;
            },
            StatementKind::Break => {
                // break文の処理
                self.generate_break_statement()?;
            },
            StatementKind::Continue => {
                // continue文の処理
                self.generate_continue_statement()?;
            },
            // 他のケースも必要に応じて追加
        }
        
        Ok(())
    }
    
    /// 式の処理
    fn generate_expression(&mut self, expression: &Expression) -> Result<BasicValueEnum<'ctx>> {
        match &expression.kind {
            ExpressionKind::Literal(lit) => self.generate_literal(lit),
            ExpressionKind::Identifier(ident) => self.generate_identifier(ident),
            ExpressionKind::BinaryOp(op, left, right) => self.generate_binary_op(*op, left, right),
            ExpressionKind::UnaryOp(op, operand) => self.generate_unary_op(*op, operand),
            ExpressionKind::Call(callee, args) => self.generate_call(callee, args),
            ExpressionKind::MemberAccess(object, member) => self.generate_member_access(object, member),
            ExpressionKind::IndexAccess(array, index) => self.generate_index_access(array, index),
            ExpressionKind::ArrayLiteral(elements) => self.generate_array_literal(elements),
            ExpressionKind::StructLiteral(name, fields) => self.generate_struct_literal(name, fields),
            ExpressionKind::TupleLiteral(elements) => self.generate_tuple_literal(elements),
            ExpressionKind::Cast(expr, target_type) => self.generate_cast(expr, target_type),
            ExpressionKind::Lambda(params, body) => self.generate_lambda(params, body),
            ExpressionKind::BlockExpr(statements, expr) => self.generate_block_expr(statements, expr.as_deref()),
            ExpressionKind::IfExpr(cond, then_branch, else_branch) => {
                self.generate_if_expr(cond, then_branch, else_branch.as_deref())
            },
            ExpressionKind::MatchExpr(expr, arms) => self.generate_match_expr(expr, arms),
            // その他の式も必要に応じて追加
        }
    }
    
    /// リテラルの生成
    fn generate_literal(&self, literal: &Literal) -> Result<BasicValueEnum<'ctx>> {
        match &literal.kind {
            LiteralKind::Integer(value) => {
                // 整数リテラル
                let int_type = self.context.i64_type();
                Ok(int_type.const_int(*value as u64, false).into())
            },
            LiteralKind::Float(value) => {
                // 浮動小数点リテラル
                let float_type = self.context.f64_type();
                Ok(float_type.const_float(*value).into())
            },
            LiteralKind::String(value) => {
                // 文字列リテラル
                let string_val = self.builder.build_global_string_ptr(value, "str");
                Ok(string_val.as_pointer_value().into())
            },
            LiteralKind::Char(value) => {
                // 文字リテラル
                let char_type = self.context.i8_type();
                Ok(char_type.const_int(*value as u64, false).into())
            },
            LiteralKind::Boolean(value) => {
                // 論理値リテラル
                let bool_type = self.context.bool_type();
                Ok(bool_type.const_int(*value as u64, false).into())
            },
            LiteralKind::Nil => {
                // nil値（nullポインタとして表現）
                let ptr_type = self.context.i8_type().ptr_type(AddressSpace::Generic);
                Ok(ptr_type.const_null().into())
            },
        }
    }
    
    /// 識別子の参照を生成
    fn generate_identifier(&self, ident: &Identifier) -> Result<BasicValueEnum<'ctx>> {
        // 変数参照を生成
        let var_name = &ident.name;
        
        if let Some(var_ptr) = self.variables.get(var_name) {
            // ローカル変数の場合
            let var_type = var_ptr.get_type().get_element_type();
            
            // 変数の値をロード
            let value = self.builder.build_load(var_type, *var_ptr, var_name);
            Ok(value)
        } else if let Some(func) = self.functions.get(var_name) {
            // 関数参照の場合
            Ok(func.as_global_value().as_pointer_value().into())
        } else {
            // グローバル変数や定数の場合
            if let Some(global_var) = self.llvm_module.get_global(var_name) {
                let var_type = global_var.get_type().get_element_type();
                let value = self.builder.build_load(var_type, global_var.as_pointer_value(), var_name);
                Ok(value)
            } else {
                Err(CompilerError::code_generation_error(
                    format!("未定義の識別子 '{}'", var_name),
                    ident.location.clone()
                ))
            }
        }
    }
    
    /// 二項演算の生成
    fn generate_binary_op(&self, op: BinaryOperator, left: &Expression, right: &Expression) 
    -> Result<BasicValueEnum<'ctx>> {
        let left_value = self.generate_expression(left)?;
        let right_value = self.generate_expression(right)?;
        
        // 左右のオペランドの型をチェック
        match (left_value, right_value) {
            // 整数演算
            (BasicValueEnum::IntValue(left_int), BasicValueEnum::IntValue(right_int)) => {
                let result = match op {
                    BinaryOperator::Add => self.builder.build_int_add(left_int, right_int, "addtmp"),
                    BinaryOperator::Subtract => self.builder.build_int_sub(left_int, right_int, "subtmp"),
                    BinaryOperator::Multiply => self.builder.build_int_mul(left_int, right_int, "multmp"),
                    BinaryOperator::Divide => self.builder.build_int_signed_div(left_int, right_int, "divtmp"),
                    BinaryOperator::Modulo => self.builder.build_int_signed_rem(left_int, right_int, "modtmp"),
                    
                    // 比較演算
                    BinaryOperator::Equal => {
                        let cmp = self.builder.build_int_compare(inkwell::IntPredicate::EQ, 
                                                                left_int, right_int, "eqtmp");
                        return Ok(cmp.into());
                    },
                    BinaryOperator::NotEqual => {
                        let cmp = self.builder.build_int_compare(inkwell::IntPredicate::NE, 
                                                                left_int, right_int, "netmp");
                        return Ok(cmp.into());
                    },
                    BinaryOperator::LessThan => {
                        let cmp = self.builder.build_int_compare(inkwell::IntPredicate::SLT, 
                                                                left_int, right_int, "slttmp");
                        return Ok(cmp.into());
                    },
                    BinaryOperator::GreaterThan => {
                        let cmp = self.builder.build_int_compare(inkwell::IntPredicate::SGT, 
                                                                left_int, right_int, "sgttmp");
                        return Ok(cmp.into());
                    },
                    BinaryOperator::LessThanEqual => {
                        let cmp = self.builder.build_int_compare(inkwell::IntPredicate::SLE, 
                                                                left_int, right_int, "sletmp");
                        return Ok(cmp.into());
                    },
                    BinaryOperator::GreaterThanEqual => {
                        let cmp = self.builder.build_int_compare(inkwell::IntPredicate::SGE, 
                                                                left_int, right_int, "sgetmp");
                        return Ok(cmp.into());
                    },
                    
                    // 論理演算
                    BinaryOperator::LogicalAnd => {
                        // 整数としての論理積（0でない値はtrue）
                        let left_bool = self.builder.build_int_compare(
                            inkwell::IntPredicate::NE,
                            left_int,
                            left_int.get_type().const_zero(),
                            "leftbool"
                        );
                        
                        let right_bool = self.builder.build_int_compare(
                            inkwell::IntPredicate::NE,
                            right_int,
                            right_int.get_type().const_zero(),
                            "rightbool"
                        );
                        
                        let result = self.builder.build_and(left_bool, right_bool, "andtmp");
                        return Ok(result.into());
                    },
                    BinaryOperator::LogicalOr => {
                        // 整数としての論理和（0でない値はtrue）
                        let left_bool = self.builder.build_int_compare(
                            inkwell::IntPredicate::NE,
                            left_int,
                            left_int.get_type().const_zero(),
                            "leftbool"
                        );
                        
                        let right_bool = self.builder.build_int_compare(
                            inkwell::IntPredicate::NE,
                            right_int,
                            right_int.get_type().const_zero(),
                            "rightbool"
                        );
                        
                        let result = self.builder.build_or(left_bool, right_bool, "ortmp");
                        return Ok(result.into());
                    },
                    
                    // ビット演算
                    BinaryOperator::BitwiseAnd => self.builder.build_and(left_int, right_int, "andtmp"),
                    BinaryOperator::BitwiseOr => self.builder.build_or(left_int, right_int, "ortmp"),
                    BinaryOperator::BitwiseXor => self.builder.build_xor(left_int, right_int, "xortmp"),
                    BinaryOperator::LeftShift => self.builder.build_left_shift(left_int, right_int, "lshifttmp"),
                    BinaryOperator::RightShift => self.builder.build_right_shift(left_int, right_int, true, "rshifttmp"),
                    
                    // 他の演算子はサポート外
                    _ => return Err(CompilerError::code_generation_error(
                        format!("整数型に対する演算子 '{:?}' はサポートされていません", op),
                        left.location.clone()
                    ))
                };
                
                Ok(result.into())
            },
            
            // 浮動小数点演算
            (BasicValueEnum::FloatValue(left_float), BasicValueEnum::FloatValue(right_float)) => {
                let result = match op {
                    BinaryOperator::Add => self.builder.build_float_add(left_float, right_float, "addtmp"),
                    BinaryOperator::Subtract => self.builder.build_float_sub(left_float, right_float, "subtmp"),
                    BinaryOperator::Multiply => self.builder.build_float_mul(left_float, right_float, "multmp"),
                    BinaryOperator::Divide => self.builder.build_float_div(left_float, right_float, "divtmp"),
                    
                    // 比較演算
                    BinaryOperator::Equal => {
                        let cmp = self.builder.build_float_compare(inkwell::FloatPredicate::OEQ, 
                                                                 left_float, right_float, "eqtmp");
                        return Ok(cmp.into());
                    },
                    BinaryOperator::NotEqual => {
                        let cmp = self.builder.build_float_compare(inkwell::FloatPredicate::ONE, 
                                                                 left_float, right_float, "netmp");
                        return Ok(cmp.into());
                    },
                    BinaryOperator::LessThan => {
                        let cmp = self.builder.build_float_compare(inkwell::FloatPredicate::OLT, 
                                                                 left_float, right_float, "lttmp");
                        return Ok(cmp.into());
                    },
                    BinaryOperator::GreaterThan => {
                        let cmp = self.builder.build_float_compare(inkwell::FloatPredicate::OGT, 
                                                                 left_float, right_float, "gttmp");
                        return Ok(cmp.into());
                    },
                    BinaryOperator::LessThanEqual => {
                        let cmp = self.builder.build_float_compare(inkwell::FloatPredicate::OLE, 
                                                                 left_float, right_float, "letmp");
                        return Ok(cmp.into());
                    },
                    BinaryOperator::GreaterThanEqual => {
                        let cmp = self.builder.build_float_compare(inkwell::FloatPredicate::OGE, 
                                                                 left_float, right_float, "getmp");
                        return Ok(cmp.into());
                    },
                    
                    // 他の演算子はサポート外
                    _ => return Err(CompilerError::code_generation_error(
                        format!("浮動小数点型に対する演算子 '{:?}' はサポートされていません", op),
                        left.location.clone()
                    )),
                };
                
                Ok(result.into())
            },
            
            // ポインタの比較演算（文字列など）
            (BasicValueEnum::PointerValue(left_ptr), BasicValueEnum::PointerValue(right_ptr)) => {
                match op {
                    BinaryOperator::Equal => {
                        // ポインタの等価比較
                        let cmp = self.builder.build_ptr_diff(left_ptr, right_ptr, "ptrdiff");
                        let zero = self.context.i64_type().const_zero();
                        let result = self.builder.build_int_compare(
                            inkwell::IntPredicate::EQ, 
                            cmp, 
                            zero, 
                            "ptreq"
                        );
                        Ok(result.into())
                    },
                    BinaryOperator::NotEqual => {
                        // ポインタの非等価比較
                        let cmp = self.builder.build_ptr_diff(left_ptr, right_ptr, "ptrdiff");
                        let zero = self.context.i64_type().const_zero();
                        let result = self.builder.build_int_compare(
                            inkwell::IntPredicate::NE, 
                            cmp, 
                            zero, 
                            "ptrne"
                        );
                        Ok(result.into())
                    },
                    _ => Err(CompilerError::code_generation_error(
                        format!("ポインタ型に対する演算子 '{:?}' はサポートされていません", op),
                        left.location.clone()
                    )),
                }
            },
            
            // 型が混在している場合（例：整数と浮動小数点）
            (BasicValueEnum::IntValue(left_int), BasicValueEnum::FloatValue(right_float)) => {
                // 整数を浮動小数点に変換して演算
                let left_float = self.builder.build_signed_int_to_float(
                    left_int, 
                    self.context.f64_type(), 
                    "inttofloat"
                );
                
                // 再帰的に浮動小数点演算を実行
                self.generate_binary_op(
                    op,
                    &Expression {
                        id: ast::generate_id(),
                        kind: ExpressionKind::Literal(Literal {
                            kind: LiteralKind::Float(left_float.get_constant().unwrap_or(0.0) as f64),
                            location: left.location.clone(),
                        }),
                        location: left.location.clone(),
                    },
                    &Expression {
                        id: ast::generate_id(),
                        kind: ExpressionKind::Literal(Literal {
                            kind: LiteralKind::Float(right_float.get_constant().unwrap_or(0.0) as f64),
                            location: right.location.clone(),
                        }),
                        location: right.location.clone(),
                    },
                )
            },
            (BasicValueEnum::FloatValue(left_float), BasicValueEnum::IntValue(right_int)) => {
                // 整数を浮動小数点に変換して演算
                let right_float = self.builder.build_signed_int_to_float(
                    right_int, 
                    self.context.f64_type(), 
                    "inttofloat"
                );
                
                // 再帰的に浮動小数点演算を実行
                self.generate_binary_op(
                    op,
                    &Expression {
                        id: ast::generate_id(),
                        kind: ExpressionKind::Literal(Literal {
                            kind: LiteralKind::Float(left_float.get_constant().unwrap_or(0.0) as f64),
                            location: left.location.clone(),
                        }),
                        location: left.location.clone(),
                    },
                    &Expression {
                        id: ast::generate_id(),
                        kind: ExpressionKind::Literal(Literal {
                            kind: LiteralKind::Float(right_float.get_constant().unwrap_or(0.0) as f64),
                            location: right.location.clone(),
                        }),
                        location: right.location.clone(),
                    },
                )
            },
            
            // その他の型の組み合わせ
            _ => Err(CompilerError::code_generation_error(
                format!("演算子 '{:?}' に対して互換性のない型です", op),
                left.location.clone()
            )),
        }
    }
    
    /// 単項演算の生成
    fn generate_unary_op(&self, op: UnaryOperator, operand: &Expression) -> Result<BasicValueEnum<'ctx>> {
        let operand_value = self.generate_expression(operand)?;
        
        match operand_value {
            // 整数演算
            BasicValueEnum::IntValue(int_val) => {
                match op {
                    UnaryOperator::Plus => {
                        // 単項プラスは値をそのまま返す
                        Ok(int_val.into())
                    },
                    UnaryOperator::Minus => {
                        // 単項マイナスは0から引く
                        let zero = int_val.get_type().const_zero();
                        let result = self.builder.build_int_sub(zero, int_val, "negtmp");
                        Ok(result.into())
                    },
                    UnaryOperator::Not => {
                        // 論理否定（0でない値はtrueと見なし、その否定を返す）
                        let zero = int_val.get_type().const_zero();
                        let is_nonzero = self.builder.build_int_compare(
                            inkwell::IntPredicate::NE, 
                            int_val, 
                            zero, 
                            "isnonzero"
                        );
                        let result = self.builder.build_not(is_nonzero, "nottmp");
                        Ok(result.into())
                    },
                    UnaryOperator::BitwiseNot => {
                        // ビット否定
                        let result = self.builder.build_not(int_val, "bnottmp");
                        Ok(result.into())
                    },
                }
            },
            // 浮動小数点演算
            BasicValueEnum::FloatValue(float_val) => {
                match op {
                    UnaryOperator::Plus => {
                        // 単項プラスは値をそのまま返す
                        Ok(float_val.into())
                    },
                    UnaryOperator::Minus => {
                        // 単項マイナスは符号を反転
                        let result = self.builder.build_float_neg(float_val, "fneg");
                        Ok(result.into())
                    },
                    _ => Err(CompilerError::code_generation_error(
                        format!("浮動小数点型に対する単項演算子 '{:?}' はサポートされていません", op),
                        operand.location.clone()
                    )),
                }
            },
            // その他の型
            _ => Err(CompilerError::code_generation_error(
                format!("単項演算子 '{:?}' にサポートされていない型です", op),
                operand.location.clone()
            )),
        }
    }
    
    /// 関数呼び出しの生成
    fn generate_call(&self, callee: &Expression, args: &[Expression]) -> Result<BasicValueEnum<'ctx>> {
        // 関数値を評価
        let callee_value = match self.generate_expression(callee)? {
            BasicValueEnum::PointerValue(ptr) => ptr,
            _ => return Err(CompilerError::code_generation_error(
                "呼び出し対象が関数ではありません",
                callee.location.clone()
            )),
        };
        
        // 関数の型情報を取得
        let fn_type = match callee_value.get_type().get_element_type() {
            AnyTypeEnum::FunctionType(ft) => ft,
            _ => return Err(CompilerError::code_generation_error(
                "呼び出し対象が関数型ではありません",
                callee.location.clone()
            )),
        };
        
        // 引数を評価
        let mut arg_values = Vec::with_capacity(args.len());
        for arg in args {
            let arg_value = self.generate_expression(arg)?;
            arg_values.push(arg_value.into());
        }
        
        // 関数呼び出し
        let result = self.builder.build_call(fn_type, callee_value, &arg_values, "calltmp");
        
        // 戻り値の型を確認
        if let Some(ret_value) = result.try_as_basic_value().left() {
            Ok(ret_value)
        } else {
            // void型の関数の場合はダミー値を返す
            Ok(self.context.i32_type().const_zero().into())
        }
    }
    
    /// メンバーアクセスの生成
    fn generate_member_access(&self, object: &Expression, member: &Identifier) -> Result<BasicValueEnum<'ctx>> {
        // オブジェクトを評価
        let object_value = self.generate_expression(object)?;
        
        // オブジェクトがポインタでない場合はエラー
        let object_ptr = match object_value {
            BasicValueEnum::PointerValue(ptr) => ptr,
            _ => return Err(CompilerError::code_generation_error(
                "メンバーアクセスはポインタ型のみサポートされています",
                object.location.clone()
            )),
        };
        
        // 構造体型を解決
        let struct_type = match object_ptr.get_type().get_element_type() {
            AnyTypeEnum::StructType(st) => st,
            _ => return Err(CompilerError::code_generation_error(
                "メンバーアクセスは構造体型のみサポートされています",
                object.location.clone()
            )),
        };
        
        // 構造体名を取得
        let struct_name = struct_type.get_name()
            .map(|name_str| name_str.to_string_lossy().to_string())
            .ok_or_else(|| CompilerError::code_generation_error(
                "名前のない構造体型にはアクセスできません",
                object.location.clone()
            ))?;
        
        // フィールドインデックスを探す
        let field_index = self.find_struct_field_index(&struct_name, &member.name)
            .ok_or_else(|| CompilerError::code_generation_error(
                format!("構造体 '{}' にフィールド '{}' が見つかりません", struct_name, member.name),
                member.location.clone()
            ))?;
        
        // フィールドへのアクセスを生成
        unsafe {
            let field_ptr = self.builder.build_struct_gep(
                struct_type,
                object_ptr, 
                field_index as u32, 
                &format!("{}.{}", struct_name, member.name)
            ).map_err(|_| CompilerError::code_generation_error(
                format!("フィールド '{}' へのアクセス中にエラーが発生しました", member.name),
                member.location.clone()
            ))?;
            
            // フィールドの値をロード
            let field_type = struct_type.get_field_type_at_index(field_index as u32)
                .ok_or_else(|| CompilerError::code_generation_error(
                    format!("フィールド '{}' の型情報が見つかりません", member.name),
                    member.location.clone()
                ))?;
            
            let value = self.builder.build_load(field_type, field_ptr, &member.name);
            Ok(value)
        }
    }
    
    /// 構造体のフィールドインデックスを探す
    fn find_struct_field_index(&self, struct_name: &str, field_name: &str) -> Option<usize> {
        if let Some(fields) = self.module.structs.get(struct_name) {
            // 中間表現の構造体フィールド情報を使用
            for (i, (name, _)) in fields.iter().enumerate() {
                if name == field_name {
                    return Some(i);
                }
            }
        }
        
        None
    }
    
    /// 配列インデックスアクセスの生成
    fn generate_index_access(&self, array: &Expression, index: &Expression) -> Result<BasicValueEnum<'ctx>> {
        // 配列を評価
        let array_value = self.generate_expression(array)?;
        
        // インデックスを評価
        let index_value = match self.generate_expression(index)? {
            BasicValueEnum::IntValue(i) => i,
            _ => return Err(CompilerError::code_generation_error(
                "配列インデックスは整数型でなければなりません",
                index.location.clone()
            )),
        };
        
        // 配列がポインタでない場合はエラー
        let array_ptr = match array_value {
            BasicValueEnum::PointerValue(ptr) => ptr,
            _ => return Err(CompilerError::code_generation_error(
                "インデックスアクセスはポインタ型のみサポートされています",
                array.location.clone()
            )),
        };
        
        // 要素へのアクセスを生成
        let element_ptr = unsafe {
            self.builder.build_gep(
                array_ptr.get_type().get_element_type(),
                array_ptr,
                &[index_value],
                "arrayelement"
            )
        };
        
        // 要素の値をロード
        let element_type = match array_ptr.get_type().get_element_type() {
            AnyTypeEnum::ArrayType(at) => at.get_element_type(),
            _ => return Err(CompilerError::code_generation_error(
                "インデックスアクセスは配列型のみサポートされています",
                array.location.clone()
            )),
        };
        
        let value = self.builder.build_load(element_type, element_ptr, "indexload");
        Ok(value)
    }

    /// 配列リテラルの生成
    fn generate_array_literal(&self, elements: &[Expression]) -> Result<BasicValueEnum<'ctx>> {
        if elements.is_empty() {
            return Err(CompilerError::code_generation_error(
                "空の配列リテラルはサポートされていません",
                None
            ));
        }
        
        // 最初の要素から配列の型を決定
        let first_element = self.generate_expression(&elements[0])?;
        let element_type = first_element.get_type();
        
        // 要素の値を評価
        let mut element_values = Vec::with_capacity(elements.len());
        for element in elements {
            let element_value = self.generate_expression(element)?;
            
            // 型が一致するか確認
            if element_value.get_type() != element_type {
                return Err(CompilerError::code_generation_error(
                    "配列リテラルの要素の型が一致しません",
                    element.location.clone()
                ));
            }
            
            element_values.push(element_value);
        }
        
        // 配列型の作成
        let array_type = element_type.array_type(elements.len() as u32);
        
        // スタック上に配列を確保
        let array_alloca = self.builder.build_alloca(array_type, "arrayliteral");
        
        // 各要素を配列に格納
        for (i, value) in element_values.iter().enumerate() {
            let i32_type = self.context.i32_type();
            let idx = i32_type.const_int(i as u64, false);
            
            // 要素のポインタを取得
            let element_ptr = unsafe {
                self.builder.build_gep(
                    array_type,
                    array_alloca,
                    &[i32_type.const_zero(), idx],
                    &format!("array.{}", i)
                )
            };
            
            // 要素を格納
            self.builder.build_store(element_ptr, *value);
        }
        
        Ok(array_alloca.into())
    }

    /// 構造体リテラルの生成
    fn generate_struct_literal(&self, name: &Identifier, fields: &[(Identifier, Expression)]) -> Result<BasicValueEnum<'ctx>> {
        // 構造体型を取得
        let struct_name = &name.name;
        let struct_type = match self.structs.get(struct_name) {
            Some(st) => *st,
            None => return Err(CompilerError::code_generation_error(
                format!("未定義の構造体型 '{}'", struct_name),
                name.location.clone()
            )),
        };
        
        // 構造体のインスタンスを確保
        let struct_alloca = self.builder.build_alloca(struct_type, &format!("{}_instance", struct_name));
        
        // モジュールの構造体フィールド情報を取得
        let field_infos = self.module.structs.get(struct_name).ok_or_else(|| 
            CompilerError::code_generation_error(
                format!("構造体 '{}' のフィールド情報が見つかりません", struct_name),
                name.location.clone()
            )
        )?;
        
        // フィールドの初期化
        for (field_name, field_value) in fields {
            // フィールドのインデックスを取得
            let field_index = self.find_struct_field_index(struct_name, &field_name.name)
                .ok_or_else(|| CompilerError::code_generation_error(
                    format!("構造体 '{}' にフィールド '{}' が見つかりません", struct_name, field_name.name),
                    field_name.location.clone()
                ))?;
            
            // フィールド値を評価
            let value = self.generate_expression(field_value)?;
            
            // フィールドに値を格納
            unsafe {
                let field_ptr = self.builder.build_struct_gep(
                    struct_type,
                    struct_alloca, 
                    field_index as u32, 
                    &format!("{}.{}", struct_name, field_name.name)
                ).map_err(|_| CompilerError::code_generation_error(
                    format!("フィールド '{}' へのアクセス中にエラーが発生しました", field_name.name),
                    field_name.location.clone()
                ))?;
                
                self.builder.build_store(field_ptr, value);
            }
        }
        
        Ok(struct_alloca.into())
    }

    /// タプルリテラルの生成
    fn generate_tuple_literal(&self, elements: &[Expression]) -> Result<BasicValueEnum<'ctx>> {
        // 要素の型と値を評価
        let mut element_values = Vec::with_capacity(elements.len());
        let mut element_types = Vec::with_capacity(elements.len());
        
        for element in elements {
            let value = self.generate_expression(element)?;
            element_types.push(value.get_type());
            element_values.push(value);
        }
        
        // タプル型を作成
        let tuple_type = self.context.struct_type(&element_types, false);
        
        // スタック上にタプルを確保
        let tuple_alloca = self.builder.build_alloca(tuple_type, "tupleliteral");
        
        // 各要素をタプルに格納
        for (i, value) in element_values.iter().enumerate() {
            // 要素のポインタを取得
            let element_ptr = unsafe {
                self.builder.build_struct_gep(
                    tuple_type,
                    tuple_alloca,
                    i as u32,
                    &format!("tuple.{}", i)
                ).map_err(|_| CompilerError::code_generation_error(
                    format!("タプル要素 {} へのアクセス中にエラーが発生しました", i),
                    elements[i].location.clone()
                ))?
            };
            
            // 要素を格納
            self.builder.build_store(element_ptr, *value);
        }
        
        Ok(tuple_alloca.into())
    }

    /// 型キャストの生成
    fn generate_cast(&self, expr: &Expression, target_type: &TypeAnnotation) -> Result<BasicValueEnum<'ctx>> {
        // 式を評価
        let expr_value = self.generate_expression(expr)?;
        
        // ターゲット型を取得
        let target_llvm_type = self.convert_type_from_annotation(target_type)?;
        
        // 型に応じたキャスト処理
        match (expr_value, target_llvm_type) {
            // 整数から整数へのキャスト
            (BasicValueEnum::IntValue(int_val), BasicTypeEnum::IntType(target_int_type)) => {
                let source_bit_width = int_val.get_type().get_bit_width();
                let target_bit_width = target_int_type.get_bit_width();
                
                let result = if target_bit_width > source_bit_width {
                    // 拡張
                    self.builder.build_int_s_extend_or_bit_cast(int_val, target_int_type, "intext")
                } else if target_bit_width < source_bit_width {
                    // 縮小
                    self.builder.build_int_truncate(int_val, target_int_type, "inttrunc")
                } else {
                    // 同じビット幅の場合はそのまま
                    int_val
                };
                
                Ok(result.into())
            },
            
            // 整数から浮動小数点へのキャスト
            (BasicValueEnum::IntValue(int_val), BasicTypeEnum::FloatType(float_type)) => {
                let result = self.builder.build_signed_int_to_float(int_val, float_type, "inttofloat");
                Ok(result.into())
            },
            
            // 浮動小数点から整数へのキャスト
            (BasicValueEnum::FloatValue(float_val), BasicTypeEnum::IntType(int_type)) => {
                let result = self.builder.build_float_to_signed_int(float_val, int_type, "floattoint");
                Ok(result.into())
            },
            
            // 浮動小数点から浮動小数点へのキャスト（精度変換）
            (BasicValueEnum::FloatValue(float_val), BasicTypeEnum::FloatType(target_float_type)) => {
                let source_width = float_val.get_type().get_bit_width();
                let target_width = target_float_type.get_bit_width();
                
                let result = if target_width > source_width {
                    // 拡張（例: f32 -> f64）
                    self.builder.build_float_ext(float_val, target_float_type, "floatext")
                } else if target_width < source_width {
                    // 縮小（例: f64 -> f32）
                    self.builder.build_float_trunc(float_val, target_float_type, "floattrunc")
                } else {
                    // 同じサイズの場合はそのまま
                    float_val
                };
                
                Ok(result.into())
            },
            
            // ポインタ間のキャスト
            (BasicValueEnum::PointerValue(ptr_val), BasicTypeEnum::PointerType(target_ptr_type)) => {
                let result = self.builder.build_pointer_cast(ptr_val, target_ptr_type, "ptrcast");
                Ok(result.into())
            },
            
            // ポインタと整数間のキャスト
            (BasicValueEnum::PointerValue(ptr_val), BasicTypeEnum::IntType(int_type)) => {
                let result = self.builder.build_ptr_to_int(ptr_val, int_type, "ptrtoint");
                Ok(result.into())
            },
            (BasicValueEnum::IntValue(int_val), BasicTypeEnum::PointerType(ptr_type)) => {
                let result = self.builder.build_int_to_ptr(int_val, ptr_type, "inttoptr");
                Ok(result.into())
            },
            
            // その他の型変換はサポート外
            _ => Err(CompilerError::code_generation_error(
                format!("サポートされていない型変換です: {:?} -> {:?}", expr_value.get_type(), target_llvm_type),
                expr.location.clone()
            )),
        }
    }

    /// ラムダ式の生成
    fn generate_lambda(&mut self, params: &[Parameter], body: &Expression) -> Result<BasicValueEnum<'ctx>> {
        // 一意なラムダ関数名を生成
        let lambda_name = self.generate_temp_name("lambda");
        
        // 現在の関数と基本ブロックを保存
        let old_function = self.current_function;
        let old_block = self.current_block;
        let old_variables = std::mem::take(&mut self.variables);
        
        // パラメータの型を解決
        let mut param_types = Vec::with_capacity(params.len());
        for param in params {
            let param_type = if let Some(type_ann) = &param.type_annotation {
                self.convert_type_from_annotation(type_ann)?
            } else {
                // 型注釈がない場合は推論（ここでは仮にi64を使用）
                self.context.i64_type().into()
            };
            param_types.push(param_type);
        }
        
        // ラムダの戻り値型を推論
        let return_type = match self.type_info.get_node_type(body.id) {
            Some(ty) => self.convert_type_from_annotation(ty)?,
            None => self.context.void_type().into(), // デフォルトはvoid
        };
        
        // 関数型の作成
        let fn_type = match return_type {
            BasicTypeEnum::IntType(t) => t.fn_type(&param_types, false),
            BasicTypeEnum::FloatType(t) => t.fn_type(&param_types, false),
            BasicTypeEnum::PointerType(t) => t.fn_type(&param_types, false),
            BasicTypeEnum::StructType(t) => t.fn_type(&param_types, false),
            BasicTypeEnum::ArrayType(t) => t.fn_type(&param_types, false),
            BasicTypeEnum::VectorType(t) => t.fn_type(&param_types, false),
        };
        
        // 関数を作成
        let function = self.llvm_module.add_function(&lambda_name, fn_type, None);
        
        // エントリーブロックを作成
        let entry_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry_block);
        
        // 新しい関数コンテキストを設定
        self.current_function = Some(function);
        self.current_block = Some(entry_block);
        
        // パラメータを登録
        for (i, param) in params.iter().enumerate() {
            let param_value = function.get_nth_param(i as u32)
                .ok_or_else(|| CompilerError::code_generation_error(
                    format!("パラメータ {} が見つかりません", i),
                    param.location.clone()
                ))?;
            
            // パラメータ用のスタック変数を作成してストア
            let param_ptr = self.builder.build_alloca(param_value.get_type(), &param.name.name);
            self.builder.build_store(param_ptr, param_value);
            self.variables.insert(param.name.name.clone(), param_ptr);
        }
        
        // ラムダ本体を生成
        let body_value = self.generate_expression(body)?;
        
        // 戻り値を設定
        if !return_type.is_void_type() {
            self.builder.build_return(Some(&body_value));
        } else {
            self.builder.build_return(None);
        }
        
        // 関数コンテキストを復元
        self.current_function = old_function;
        self.current_block = old_block;
        self.variables = old_variables;
        
        // 関数ポインタを返す
        Ok(function.as_global_value().as_pointer_value().into())
    }

    /// ブロック式の生成
    fn generate_block_expr(&mut self, statements: &[Statement], final_expr: Option<&Expression>) -> Result<BasicValueEnum<'ctx>> {
        // 現在の変数環境を保存
        let old_variables = self.variables.clone();
        
        // ブロック内の各文を生成
        for stmt in statements {
            self.generate_statement(stmt)?;
        }
        
        // 最終式があれば評価し、その値を返す
        let result = if let Some(expr) = final_expr {
            self.generate_expression(expr)?
        } else {
            // 最終式がない場合はvoid値（ダミーの整数値）を返す
            self.context.i32_type().const_zero().into()
        };
        
        // 元の変数環境を復元（ブロックスコープから外れた変数を削除）
        // ただし、外側のスコープで定義された変数は保持
        let mut new_variables = HashMap::new();
        for (name, ptr) in old_variables {
            if self.variables.contains_key(&name) {
                new_variables.insert(name, ptr);
            }
        }
        self.variables = new_variables;
        
        Ok(result)
    }
    
    /// if文の生成
    fn generate_if_statement(&mut self, condition: &Expression, then_branch: &Statement, else_branch: Option<&Statement>) -> Result<()> {
        // 条件を評価
        let cond_value = self.generate_expression(condition)?;
        
        // 条件をブール値に変換（必要に応じて）
        let cond_bool = match cond_value {
            BasicValueEnum::IntValue(int_val) => {
                // 整数値を比較（0でない値はtrue）
                let zero = int_val.get_type().const_zero();
                self.builder.build_int_compare(
                    inkwell::IntPredicate::NE,
                    int_val,
                    zero,
                    "ifcond"
                )
            },
            _ => {
                return Err(CompilerError::code_generation_error(
                    "if文の条件にはブール型または整数型が必要です",
                    condition.location.clone()
                ));
            }
        };
        
        // 現在の関数
        let current_fn = self.current_function.ok_or_else(|| {
            CompilerError::code_generation_error(
                "関数コンテキスト外でのif文は無効です",
                condition.location.clone()
            )
        })?;
        
        // 分岐先の基本ブロックを作成
        let then_block = self.context.append_basic_block(current_fn, "then");
        let else_block = self.context.append_basic_block(current_fn, "else");
        let merge_block = self.context.append_basic_block(current_fn, "ifcont");
        
        // 条件に基づいて分岐
        self.builder.build_conditional_branch(cond_bool, then_block, else_block);
        
        // then部分の生成
        self.builder.position_at_end(then_block);
        self.current_block = Some(then_block);
        self.generate_statement(then_branch)?;
        
        // then部分の最後に無条件分岐を追加（fallthroughを防ぐ）
        if !self.builder.get_insert_block().unwrap().get_terminator().is_some() {
            self.builder.build_unconditional_branch(merge_block);
        }
        
        // else部分の生成
        self.builder.position_at_end(else_block);
        self.current_block = Some(else_block);
        if let Some(else_stmt) = else_branch {
            self.generate_statement(else_stmt)?;
        }
        
        // else部分の最後に無条件分岐を追加
        if !self.builder.get_insert_block().unwrap().get_terminator().is_some() {
            self.builder.build_unconditional_branch(merge_block);
        }
        
        // マージブロックに移動
        self.builder.position_at_end(merge_block);
        self.current_block = Some(merge_block);
        
        // 条件評価後に分岐インストルメンテーション
        let branch_id = self.generate_temp_name("if_branch");
        self.generate_branch_instrumentation(cond_bool, &branch_id)?;
        
        Ok(())
    }
    
    /// if式の生成
    fn generate_if_expr(&mut self, condition: &Expression, then_branch: &Expression, else_branch: Option<&Expression>) -> Result<BasicValueEnum<'ctx>> {
        // 条件を評価
        let cond_value = self.generate_expression(condition)?;
        
        // 条件をブール値に変換（必要に応じて）
        let cond_bool = match cond_value {
            BasicValueEnum::IntValue(int_val) => {
                // 整数値を比較（0でない値はtrue）
                let zero = int_val.get_type().const_zero();
                self.builder.build_int_compare(
                    inkwell::IntPredicate::NE,
                    int_val,
                    zero,
                    "ifcond"
                )
            },
            _ => {
                return Err(CompilerError::code_generation_error(
                    "if式の条件にはブール型または整数型が必要です",
                    condition.location.clone()
                ));
            }
        };
        
        // 現在の関数
        let current_fn = self.current_function.ok_or_else(|| {
            CompilerError::code_generation_error(
                "関数コンテキスト外でのif式は無効です",
                condition.location.clone()
            )
        })?;
        
        // then部分の型を調べて戻り値の型を決定
        let then_result_type = self.type_info.get_node_type(then_branch.id)
            .ok_or_else(|| CompilerError::code_generation_error(
                "if式のthen部分の型情報が見つかりません",
                then_branch.location.clone()
            ))?;
        
        let result_type = self.convert_type_from_annotation(then_result_type)?;
        
        // 分岐先の基本ブロックを作成
        let then_block = self.context.append_basic_block(current_fn, "then");
        let else_block = self.context.append_basic_block(current_fn, "else");
        let merge_block = self.context.append_basic_block(current_fn, "ifcont");
        
        // 条件に基づいて分岐
        self.builder.build_conditional_branch(cond_bool, then_block, else_block);
        
        // phi節点用の値を格納するための変数
        let mut incoming_values = Vec::new();
        let mut incoming_blocks = Vec::new();
        
        // then部分の生成
        self.builder.position_at_end(then_block);
        self.current_block = Some(then_block);
        let then_value = self.generate_expression(then_branch)?;
        
        // then部分からの遷移元情報を記録
        incoming_values.push(then_value);
        incoming_blocks.push(self.builder.get_insert_block().unwrap());
        
        // then部分の最後に無条件分岐を追加
        self.builder.build_unconditional_branch(merge_block);
        
        // else部分の生成
        self.builder.position_at_end(else_block);
        self.current_block = Some(else_block);
        
        let else_value = if let Some(else_expr) = else_branch {
            self.generate_expression(else_expr)?
        } else {
            // else部分がない場合はデフォルト値を使用
            match result_type {
                BasicTypeEnum::IntType(t) => t.const_zero().into(),
                BasicTypeEnum::FloatType(t) => t.const_zero().into(),
                BasicTypeEnum::PointerType(t) => t.const_null().into(),
                BasicTypeEnum::StructType(_) => {
                    return Err(CompilerError::code_generation_error(
                        "if式のelse部分が省略されている場合、構造体型は返せません",
                        condition.location.clone()
                    ));
                },
                BasicTypeEnum::ArrayType(_) => {
                    return Err(CompilerError::code_generation_error(
                        "if式のelse部分が省略されている場合、配列型は返せません",
                        condition.location.clone()
                    ));
                },
                BasicTypeEnum::VectorType(_) => {
                    return Err(CompilerError::code_generation_error(
                        "if式のelse部分が省略されている場合、ベクトル型は返せません",
                        condition.location.clone()
                    ));
                },
            }
        };
        
        // else部分からの遷移元情報を記録
        incoming_values.push(else_value);
        incoming_blocks.push(self.builder.get_insert_block().unwrap());
        
        // else部分の最後に無条件分岐を追加
        self.builder.build_unconditional_branch(merge_block);
        
        // マージブロックに移動してPhi節点を作成
        self.builder.position_at_end(merge_block);
        self.current_block = Some(merge_block);
        
        // Phi節点を作成
        let phi = self.builder.build_phi(result_type, "ifresult");
        
        // Phi節点に値を追加
        for (value, block) in incoming_values.iter().zip(incoming_blocks.iter()) {
            phi.add_incoming(&[(&*value, *block)]);
        }
        
        Ok(phi.as_basic_value())
    }

    /// match式の生成
    fn generate_match_expr(&mut self, expr: &Expression, arms: &[(Expression, Expression)]) -> Result<BasicValueEnum<'ctx>> {
        // 対象式の評価
        let expr_value = self.generate_expression(expr)?;
        
        // 現在の関数
        let current_fn = self.current_function.ok_or_else(|| {
            CompilerError::code_generation_error(
                "関数コンテキスト外でのmatch式は無効です",
                expr.location.clone()
            )
        })?;
        
        // 最初のアームの結果の型を推論してmatch式の戻り値の型を決定
        if arms.is_empty() {
            return Err(CompilerError::code_generation_error(
                "match式には少なくとも1つのアームが必要です",
                expr.location.clone()
            ));
        }
        
        let first_arm_type = self.type_info.get_node_type(arms[0].1.id)
            .ok_or_else(|| CompilerError::code_generation_error(
                "match式のアームの型情報が見つかりません",
                arms[0].1.location.clone()
            ))?;
        
        let result_type = self.convert_type_from_annotation(first_arm_type)?;
        
        // 終了ブロック（全アームの合流先）
        let merge_block = self.context.append_basic_block(current_fn, "matchend");
        
        // phi節点用の値を格納するための変数
        let mut incoming_values = Vec::new();
        let mut incoming_blocks = Vec::new();
        
        // デフォルトのケース（すべてのパターンにマッチしなかった場合）
        let default_block = self.context.append_basic_block(current_fn, "match_default");
        
        // 前のアームの次のチェックブロック（最初はここから開始）
        let mut current_block = self.builder.get_insert_block().unwrap();
        self.builder.position_at_end(current_block);
        
        // マッチ対象の値を一時変数に保存（複数回評価を避けるため）
        let expr_type = expr_value.get_type();
        let expr_ptr = self.builder.build_alloca(expr_type, "match_expr_var");
        self.builder.build_store(expr_ptr, expr_value);
        
        // 各アームごとにブロックを生成
        for (i, (pattern, arm_body)) in arms.iter().enumerate() {
            // このパターンのブロック
            let pattern_block = self.context.append_basic_block(current_fn, &format!("match_arm{}", i));
            // 次のパターンチェックのブロック（最後のアームの場合はデフォルトブロック）
            let next_pattern_block = if i == arms.len() - 1 {
                default_block
            } else {
                self.context.append_basic_block(current_fn, &format!("match_check{}", i + 1))
            };
            
            // パターンマッチのチェックコードをこのブロックに配置
            self.builder.position_at_end(current_block);
            
            match &pattern.kind {
                // リテラルパターン
                ExpressionKind::Literal(lit) => {
                    // リテラル値を評価
                    let lit_value = self.generate_literal(lit)?;
                    
                    // 対象式の値をロード
                    let expr_value = self.builder.build_load(expr_type, expr_ptr, "match_expr_val");
                    
                    // 対象式の値とリテラルを比較
                    let cond = match (expr_value, lit_value) {
                        (BasicValueEnum::IntValue(expr_int), BasicValueEnum::IntValue(lit_int)) => {
                            self.builder.build_int_compare(
                                inkwell::IntPredicate::EQ,
                                expr_int,
                                lit_int,
                                "matchcmp"
                            )
                        },
                        (BasicValueEnum::FloatValue(expr_float), BasicValueEnum::FloatValue(lit_float)) => {
                            self.builder.build_float_compare(
                                inkwell::FloatPredicate::OEQ,
                                expr_float,
                                lit_float,
                                "matchcmp"
                            )
                        },
                        _ => {
                            return Err(CompilerError::code_generation_error(
                                "サポートされていないmatchパターンです",
                                pattern.location.clone()
                            ));
                        }
                    };
                    
                    // 条件によって分岐
                    self.builder.build_conditional_branch(cond, pattern_block, next_pattern_block);
                },
                
                // 識別子パターン（変数バインディング）
                ExpressionKind::Identifier(ident) => {
                    // 対象式の値をロード
                    let expr_value = self.builder.build_load(expr_type, expr_ptr, "match_expr_val");
                    
                    // 現在のスコープにパターン変数をバインド
                    let var_name = &ident.name;
                    let var_ptr = self.builder.build_alloca(expr_type, var_name);
                    self.builder.build_store(var_ptr, expr_value);
                    self.variables.insert(var_name.clone(), var_ptr);
                    
                    // 常にマッチする
                    self.builder.build_unconditional_branch(pattern_block);
                },
                
                // 構造体パターン（新規実装）
                ExpressionKind::StructLiteral(struct_name, fields) => {
                    // 対象式の値をロード
                    let expr_value = self.builder.build_load(expr_type, expr_ptr, "match_expr_val");
                    
                    // 構造体型かチェック
                    if let BasicValueEnum::StructValue(struct_val) = expr_value {
                        let struct_type = struct_val.get_type();
                        let struct_name_str = &struct_name.name;
                        
                        // 一時変数に保存
                        let struct_ptr = self.builder.build_alloca(struct_type, "struct_pattern");
                        self.builder.build_store(struct_ptr, struct_val);
                        
                        // マッチ条件ブロックの開始
                        let mut current_match_block = self.builder.get_insert_block().unwrap();
                        let mut all_fields_match = true;
                        
                        // 各フィールドのパターンマッチ
                        for (field_name, field_pattern) in fields {
                            // フィールドのインデックスを取得
                            let field_idx = self.find_struct_field_index(struct_name_str, &field_name.name);
                            
                            if let Some(idx) = field_idx {
                                // フィールド値へのアクセス
                                let field_ptr = unsafe {
                                    self.builder.build_struct_gep(
                                        struct_type,
                                        struct_ptr,
                                        idx as u32,
                                        &format!("{}_ptr", field_name.name)
                                    )?
                                };
                                
                                // フィールドが識別子パターンの場合（変数バインド）
                                if let ExpressionKind::Identifier(pattern_var) = &field_pattern.kind {
                                    // フィールド値をロード
                                    let field_value = self.builder.build_load(
                                        self.builder.get_insert_block().unwrap().get_context().i32_type().into(),  // 仮の型
                                        field_ptr, 
                                        &format!("{}_val", field_name.name)
                                    );
                                    
                                    // 変数にバインド
                                    let var_ptr = self.builder.build_alloca(
                                        field_value.get_type(), 
                                        &pattern_var.name
                                    );
                                    self.builder.build_store(var_ptr, field_value);
                                    self.variables.insert(pattern_var.name.clone(), var_ptr);
                                } else {
                                    // より複雑なネストしたパターンマッチは省略（実際の実装ではここに再帰的なマッチング処理が必要）
                                    all_fields_match = false;
                                    break;
                                }
                            } else {
                                // フィールドが見つからない
                                all_fields_match = false;
                                break;
                            }
                        }
                        
                        // すべてのフィールドがマッチしたかどうかでブランチ
                        if all_fields_match {
                            self.builder.build_unconditional_branch(pattern_block);
                        } else {
                            self.builder.build_unconditional_branch(next_pattern_block);
                        }
                    } else {
                        // 構造体でなければ次のパターンへ
                        self.builder.build_unconditional_branch(next_pattern_block);
                    }
                },
                
                // ガード付きパターン（新規実装）- パターンと条件式のタプル
                ExpressionKind::TupleLiteral(elements) if elements.len() == 2 => {
                    // パターン部分（最初の要素）
                    let sub_pattern = &elements[0];
                    // ガード条件（2番目の要素）
                    let guard_expr = &elements[1];
                    
                    // 対象式の値をロード
                    let expr_value = self.builder.build_load(expr_type, expr_ptr, "match_expr_val");
                    
                    // パターンマッチングの一時ブロック
                    let pattern_match_block = self.context.append_basic_block(current_fn, &format!("guard_pattern{}", i));
                    let guard_eval_block = self.context.append_basic_block(current_fn, &format!("guard_eval{}", i));
                    
                    // サブパターンの種類に応じたマッチング
                    match &sub_pattern.kind {
                        // 識別子パターン
                        ExpressionKind::Identifier(ident) => {
                            // 変数にバインド
                            let var_name = &ident.name;
                            let var_ptr = self.builder.build_alloca(expr_type, var_name);
                            self.builder.build_store(var_ptr, expr_value);
                            self.variables.insert(var_name.clone(), var_ptr);
                            
                            // パターンマッチブロックへ分岐
                            self.builder.build_unconditional_branch(pattern_match_block);
                        },
                        
                        // リテラルパターン
                        ExpressionKind::Literal(lit) => {
                            // リテラル値を評価
                            let lit_value = self.generate_literal(lit)?;
                            
                            // 対象式の値とリテラルを比較
                            let cond = match (expr_value, lit_value) {
                                (BasicValueEnum::IntValue(expr_int), BasicValueEnum::IntValue(lit_int)) => {
                                    self.builder.build_int_compare(
                                        inkwell::IntPredicate::EQ,
                                        expr_int,
                                        lit_int,
                                        "guardcmp"
                                    )
                                },
                                (BasicValueEnum::FloatValue(expr_float), BasicValueEnum::FloatValue(lit_float)) => {
                                    self.builder.build_float_compare(
                                        inkwell::FloatPredicate::OEQ,
                                        expr_float,
                                        lit_float,
                                        "guardcmp"
                                    )
                                },
                                _ => {
                                    return Err(CompilerError::code_generation_error(
                                        "サポートされていないmatchパターンです",
                                        sub_pattern.location.clone()
                                    ));
                                }
                            };
                            
                            // 条件によって分岐
                            self.builder.build_conditional_branch(cond, pattern_match_block, next_pattern_block);
                        },
                        
                        // その他のパターン（簡略化のため省略）
                        _ => {
                            self.builder.build_unconditional_branch(next_pattern_block);
                        }
                    }
                    
                    // パターンマッチングが成功した場合のガード条件評価
                    self.builder.position_at_end(pattern_match_block);
                    
                    // ガード条件を評価
                    let guard_value = self.generate_expression(guard_expr)?;
                    
                    // ガード条件をブール値に変換
                    let guard_bool = match guard_value {
                        BasicValueEnum::IntValue(int_val) => {
                            // 整数値を比較（0でない値はtrue）
                            let zero = int_val.get_type().const_zero();
                            self.builder.build_int_compare(
                                inkwell::IntPredicate::NE,
                                int_val,
                                zero,
                                "guardbool"
                            )
                        },
                        _ => {
                            return Err(CompilerError::code_generation_error(
                                "ガード条件にはブール型または整数型が必要です",
                                guard_expr.location.clone()
                            ));
                        }
                    };
                    
                    // ガード条件によって分岐
                    self.builder.build_conditional_branch(guard_bool, pattern_block, next_pattern_block);
                },
                
                // その他のパターンはサポート外
                _ => {
                    return Err(CompilerError::code_generation_error(
                        "サポートされていないmatchパターン形式です",
                        pattern.location.clone()
                    ));
                }
            }
            
            // パターンの本体を生成
            self.builder.position_at_end(pattern_block);
            self.current_block = Some(pattern_block);
            
            // アームの本体を評価
            let arm_value = self.generate_expression(arm_body)?;
            
            // アームからの遷移元情報を記録
            incoming_values.push(arm_value);
            incoming_blocks.push(self.builder.get_insert_block().unwrap());
            
            // アームの最後に終了ブロックへの無条件分岐を追加
            self.builder.build_unconditional_branch(merge_block);
            
            // 次のパターンチェックブロックを更新
            current_block = next_pattern_block;
        }
        
        // デフォルトケースはエラーとして扱う
        self.builder.position_at_end(default_block);
        
        // ランタイムエラー関数を呼び出す（存在する場合）
        if let Some(panic_fn) = self.llvm_module.get_function("swiftlight_panic") {
            // エラーメッセージを生成
            let error_msg = self.builder.build_global_string_ptr(
                "Match error: no pattern matched", 
                "match_error_msg"
            );
            
            // エラー関数を呼び出す
            self.builder.build_call(
                panic_fn, 
                &[error_msg.as_pointer_value().into()], 
                "panicall"
            );
            
            // 呼び出し後も制御フローが継続するようにする（実際には戻らない）
            self.builder.build_unreachable();
        } else {
            // エラー関数がなければデフォルト値を返す
            let default_value = match result_type {
                BasicTypeEnum::IntType(t) => t.const_zero().into(),
                BasicTypeEnum::FloatType(t) => t.const_zero().into(),
                BasicTypeEnum::PointerType(t) => t.const_null().into(),
                _ => {
                    return Err(CompilerError::code_generation_error(
                        "サポートされていない型のmatch式です",
                        expr.location.clone()
                    ));
                }
            };
            
            // デフォルト値を記録
            incoming_values.push(default_value);
            incoming_blocks.push(default_block);
            
            // 終了ブロックへ分岐
            self.builder.build_unconditional_branch(merge_block);
        }
        
        // 終了ブロックでPhi節点を作成
        self.builder.position_at_end(merge_block);
        self.current_block = Some(merge_block);
        
        // Phi節点に値を追加
        let phi = self.builder.build_phi(result_type, "match_result");
        for (value, block) in incoming_values.iter().zip(incoming_blocks.iter()) {
            phi.add_incoming(&[(&*value, *block)]);
        }
        
        Ok(phi.as_basic_value())
    }
    
    /// 関数宣言の生成
    fn generate_function_declaration(&mut self, func: &Function, decl: &Declaration) -> Result<()> {
        let function_name = &func.name.name;
        
        // 関数が既に宣言されているか確認
        let function = if let Some(f) = self.functions.get(function_name) {
            *f
        } else {
            return Err(CompilerError::code_generation_error(
                format!("関数 '{}' は事前宣言されていません", function_name),
                func.name.location.clone()
            ));
        };
        
        // 現在の関数と変数環境を保存
        let old_function = self.current_function;
        let old_block = self.current_block;
        let old_variables = std::mem::take(&mut self.variables);
        
        // 現在の関数を設定
        self.current_function = Some(function);
        
        // 関数本体の基本ブロックを作成
        let entry_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry_block);
        self.current_block = Some(entry_block);
        
        // プロファイリング用のエントリーイベント生成
        self.generate_function_entry_profiling(function_name)?;
        
        // パラメータを登録
        for (i, param) in func.parameters.iter().enumerate() {
            let param_value = function.get_nth_param(i as u32)
                .ok_or_else(|| CompilerError::code_generation_error(
                    format!("パラメータ {} が見つかりません", i),
                    param.location.clone()
                ))?;
            
            // パラメータの型を取得
            let param_type = param_value.get_type();
            
            // パラメータ用のローカル変数を作成
            let alloca = self.builder.build_alloca(param_type, &param.name.name);
            
            // パラメータ値をローカル変数に格納
            self.builder.build_store(alloca, param_value);
            
            // 変数マップに追加
            self.variables.insert(param.name.name.clone(), alloca);
        }
        
        // 関数本体を生成
        self.generate_statement(&func.body)?;
        
        // 関数の最後に戻り値がなければvoid型の関数として戻り値なしのreturnを追加
        if !self.builder.get_insert_block().unwrap().get_terminator().is_some() {
            self.builder.build_return(None);
        }
        
        // プロファイリング用の終了イベント生成
        self.generate_function_exit_profiling(function_name)?;
        
        // 関数環境を復元
        self.current_function = old_function;
        self.current_block = old_block;
        self.variables = old_variables;
        
        Ok(())
    }
    
    /// グローバル変数の生成
    fn generate_global_variable(&mut self, var: &VariableDeclaration, decl: &Declaration) -> Result<()> {
        let var_name = &var.name.name;
        
        // 変数の型を取得
        let var_type = if let Some(type_ann) = &var.type_annotation {
            if let Some(type_info) = self.type_info.get_node_type(type_ann.id) {
                self.convert_type_from_annotation(&type_info)?
            } else {
                return Err(CompilerError::code_generation_error(
                    format!("変数 '{}' の型情報がありません", var_name),
                    var.name.location.clone()
                ));
            }
        } else if let Some(init) = &var.initializer {
            // 初期化式から型を推論
            if let Some(type_info) = self.type_info.get_node_type(init.id) {
                self.convert_type_from_annotation(&type_info)?
            } else {
                return Err(CompilerError::code_generation_error(
                    format!("変数 '{}' の初期化式の型情報がありません", var_name),
                    init.location.clone()
                ));
            }
        } else {
            return Err(CompilerError::code_generation_error(
                format!("変数 '{}' の型を決定できません", var_name),
                var.name.location.clone()
            ));
        };
        
        // グローバル変数を作成
        let global_var = self.llvm_module.add_global(var_type, None, var_name);
        
        // 初期値を設定
        if let Some(init) = &var.initializer {
            // 初期化式の評価のために一時的な関数とブロックを作成
            let initializer_fn_type = self.context.void_type().fn_type(&[], false);
            let initializer_fn = self.llvm_module.add_function(
                &format!("{}.initializer", var_name),
                initializer_fn_type,
                None
            );
            
            // 現在の関数と基本ブロックを保存
            let old_function = self.current_function;
            let old_block = self.current_block;
            
            // 初期化関数のエントリーブロックを作成
            let entry_block = self.context.append_basic_block(initializer_fn, "entry");
            self.builder.position_at_end(entry_block);
            
            // 現在の関数コンテキストを設定
            self.current_function = Some(initializer_fn);
            self.current_block = Some(entry_block);
            
            // 初期化式を評価
            let init_value = self.generate_expression(init)?;
            
            // グローバル変数の初期値を設定
            global_var.set_initializer(&init_value);
            
            // 初期化関数を終了
            self.builder.build_return(None);
            
            // 関数コンテキストを復元
            self.current_function = old_function;
            self.current_block = old_block;
        } else {
            // 初期化式がない場合はデフォルト値を設定
            match var_type {
                BasicTypeEnum::IntType(_) => {
                    global_var.set_initializer(&self.context.i64_type().const_zero());
                },
                BasicTypeEnum::FloatType(_) => {
                    global_var.set_initializer(&self.context.f64_type().const_zero());
                },
                BasicTypeEnum::PointerType(_) => {
                    global_var.set_initializer(&var_type.into_pointer_type().const_null());
                },
                BasicTypeEnum::StructType(st) => {
                    global_var.set_initializer(&st.const_zero());
                },
                BasicTypeEnum::ArrayType(at) => {
                    global_var.set_initializer(&at.const_zero());
                },
            }
        }
        
        // 中間表現のモジュールにグローバル変数を追加
        let ir_type = if let Some(type_ann) = &var.type_annotation {
            if let Some(type_info) = self.type_info.get_node_type(type_ann.id) {
                self.convert_to_ir_type(&type_info)?
            } else {
                return Err(CompilerError::code_generation_error(
                    format!("変数 '{}' の型情報がありません", var_name),
                    var.name.location.clone()
                ));
            }
        } else if let Some(init) = &var.initializer {
            if let Some(type_info) = self.type_info.get_node_type(init.id) {
                self.convert_to_ir_type(&type_info)?
            } else {
                return Err(CompilerError::code_generation_error(
                    format!("変数 '{}' の初期化式の型情報がありません", var_name),
                    init.location.clone()
                ));
            }
        } else {
            return Err(CompilerError::code_generation_error(
                format!("変数 '{}' の型を決定できません", var_name),
                var.name.location.clone()
            ));
        };
        
        // 中間表現にグローバル変数を追加
        self.module.add_global_variable(var_name.clone(), ir_type, false);
        
        Ok(())
    }
    
    /// グローバル定数の生成
    fn generate_global_constant(&mut self, constant: &ConstantDeclaration, decl: &Declaration) -> Result<()> {
        let const_name = &constant.name.name;
        
        // 定数の型を取得
        let const_type = if let Some(type_ann) = &constant.type_annotation {
            if let Some(type_info) = self.type_info.get_node_type(type_ann.id) {
                self.convert_type_from_annotation(&type_info)?
            } else {
                return Err(CompilerError::code_generation_error(
                    format!("定数 '{}' の型情報がありません", const_name),
                    constant.name.location.clone()
                ));
            }
        } else {
            // 初期化式から型を推論
            if let Some(type_info) = self.type_info.get_node_type(constant.initializer.id) {
                self.convert_type_from_annotation(&type_info)?
            } else {
                return Err(CompilerError::code_generation_error(
                    format!("定数 '{}' の初期化式の型情報がありません", const_name),
                    constant.initializer.location.clone()
                ));
            }
        };
        
        // グローバル定数を作成
        let global_const = self.llvm_module.add_global(const_type, None, const_name);
        
        // 定数として設定
        global_const.set_constant(true);
        
        // 初期化式の評価のために一時的な関数とブロックを作成
        let initializer_fn_type = self.context.void_type().fn_type(&[], false);
        let initializer_fn = self.llvm_module.add_function(
            &format!("{}.initializer", const_name),
            initializer_fn_type,
            None
        );
        
        // 現在の関数と基本ブロックを保存
        let old_function = self.current_function;
        let old_block = self.current_block;
        
        // 初期化関数のエントリーブロックを作成
        let entry_block = self.context.append_basic_block(initializer_fn, "entry");
        self.builder.position_at_end(entry_block);
        
        // 現在の関数コンテキストを設定
        self.current_function = Some(initializer_fn);
        self.current_block = Some(entry_block);
        
        
        // 初期化式を評価
        let init_value = self.generate_expression(&constant.initializer)?;
        
        // グローバル定数の初期値を設定
        global_const.set_initializer(&init_value);
        
        // 初期化関数を終了
        self.builder.build_return(None);
        
        // 関数コンテキストを復元
        self.current_function = old_function;
        self.current_block = old_block;
        
        // 中間表現のモジュールにグローバル定数を追加
        let ir_type = if let Some(type_ann) = &constant.type_annotation {
            if let Some(type_info) = self.type_info.get_node_type(type_ann.id) {
                self.convert_to_ir_type(&type_info)?
            } else {
                return Err(CompilerError::code_generation_error(
                    format!("定数 '{}' の型情報がありません", const_name),
                    constant.name.location.clone()
                ));
            }
        } else {
            if let Some(type_info) = self.type_info.get_node_type(constant.initializer.id) {
                self.convert_to_ir_type(&type_info)?
            } else {
                return Err(CompilerError::code_generation_error(
                    format!("定数 '{}' の初期化式の型情報がありません", const_name),
                    constant.initializer.location.clone()
                ));
            }
        };
        
        // 中間表現にグローバル定数を追加
        self.module.add_global_variable(const_name.clone(), ir_type, true);
        
        Ok(())
    }
    
    /// 構造体宣言の生成
    fn generate_struct_declaration(&mut self, struct_decl: &Struct, decl: &Declaration) -> Result<()> {
        // 構造体の処理は主に predeclare_types で行われるため、
        // ここでは中間表現の詳細情報のみを追加/更新
        let struct_name = &struct_decl.name.name;
        
        // フィールド情報の収集
        let mut field_names = Vec::new();
        for field in &struct_decl.fields {
            field_names.push(field.name.name.clone());
        }
        
        // 中間表現の構造体情報を更新
        if let Some(fields) = self.module.structs.get_mut(struct_name) {
            for (i, field) in struct_decl.fields.iter().enumerate() {
                if i < fields.len() {
                    // フィールド名のタプルを更新
                    let field_type = fields[i].1.clone();
                    fields[i] = (field.name.name.clone(), field_type);
                }
            }
        }
        
        Ok(())
    }
    
    /// 列挙型宣言の生成
    fn generate_enum_declaration(&mut self, enum_decl: &Enum, decl: &Declaration) -> Result<()> {
        let enum_name = &enum_decl.name.name;
        
        // 列挙型の表現：タグ付き共用体として実装
        // まず、タグの型（識別子）を作成
        let tag_type = self.context.i32_type();
        
        // バリアントごとのデータ型を収集
        let mut variant_types = Vec::new();
        let mut variant_names = Vec::new();
        let mut max_size = 0;
        let mut max_align = 1;
        
        for (i, variant) in enum_decl.variants.iter().enumerate() {
            variant_names.push(variant.name.name.clone());
            
            // 関連値があるバリアント
            if let Some(associated_values) = &variant.associated_values {
                let mut field_types = Vec::new();
                
                for value_type in associated_values {
                    if let Some(type_info) = self.type_info.get_node_type(value_type.id) {
                        if let Ok(llvm_type) = self.convert_type_from_annotation(&type_info) {
                            field_types.push(llvm_type);
                            
                            // サイズと整列を追跡
                            let size = self.llvm_module.get_data_layout().get_type_alloc_size(&llvm_type);
                            let align = self.llvm_module.get_data_layout().get_preferred_alignment(&llvm_type);
                            max_size = max_size.max(size);
                            max_align = max_align.max(align);
                        }
                    }
                }
                
                // バリアントのデータ部分の構造体型を作成
                if !field_types.is_empty() {
                    let variant_struct_name = format!("{}.{}", enum_name, variant.name.name);
                    let variant_struct = self.context.opaque_struct_type(&variant_struct_name);
                    variant_struct.set_body(&field_types, false);
                    
                    variant_types.push(Some(variant_struct.into()));
                } else {
                    variant_types.push(None);
                }
            } else {
                // データのないバリアント
                variant_types.push(None);
            }
        }
        
        // 列挙型自体の構造体型を作成
        let enum_struct_type = self.context.struct_type(&[
            tag_type.into(),                          // タグ（バリアント識別子）
            self.context.i8_type().array_type(max_size as u32).into() // データ（最大サイズの配列）
        ], false);
        
        // 列挙型を登録
        self.structs.insert(enum_name.clone(), enum_struct_type);
        
        // 中間表現にも列挙型を登録
        let mut ir_variants = Vec::new();
        for (i, variant) in enum_decl.variants.iter().enumerate() {
            let variant_name = variant.name.name.clone();
            
            let variant_type = if let Some(associated_values) = &variant.associated_values {
                let mut field_types = Vec::new();
                
                for value_type in associated_values {
                    if let Some(type_info) = self.type_info.get_node_type(value_type.id) {
                        if let Ok(ir_type) = self.convert_to_ir_type(&type_info) {
                            field_types.push(ir_type);
                        }
                    }
                }
                
                if !field_types.is_empty() {
                    Some(Type::Tuple(field_types))
                } else {
                    None
                }
            } else {
                None
            };
            
            ir_variants.push((variant_name, variant_type));
        }
        
        self.module.add_enum(enum_name.clone(), ir_variants);
        
        Ok(())
    }
    
    /// トレイト宣言の生成
    fn generate_trait_declaration(&mut self, trait_decl: &Trait, decl: &Declaration) -> Result<()> {
        let trait_name = &trait_decl.name.name;
        
        // トレイトメソッドの宣言を処理
        for method in &trait_decl.methods {
            let method_name = format!("{}_{}", trait_name, method.name.name);
            
            // メソッドの宣言を生成（実装はなし）
            self.generate_function_declaration(method, decl)?;
        }
        
        // 関連型情報を保存（型チェック結果から取得）
        if let Some(trait_info) = self.type_info.get_trait_info(trait_name) {
            if let Some(associated_types) = &trait_info.associated_types {
                // 関連型情報をIRモジュールのメタデータとして保存
                for assoc_type in associated_types {
                    let md_name = format!("{}_assoc_type_{}", trait_name, assoc_type.name);
                    
                    // 関連型のメタデータノードを作成
                    let md_nodes = vec![
                        self.context.metadata_string(&assoc_type.name).into(),
                        self.context.metadata_string(&format!("{:?}", assoc_type.bounds)).into(),
                        match &assoc_type.default_type {
                            Some(default) => self.context.metadata_string(&format!("{:?}", default)).into(),
                            None => self.context.metadata_node(&[]).into(),
                        },
                    ];
                    
                    let md = self.context.metadata_node(&md_nodes);
                    self.llvm_module.add_named_metadata_operand(&md_name, md);
                }
            }
        }
        
        Ok(())
    }
    
    /// インプリメンテーション（実装）の生成
    fn generate_implementation(&mut self, impl_decl: &Implementation, decl: &Declaration) -> Result<()> {
        let type_name = match &impl_decl.target_type.kind {
            TypeKind::Named(name) => &name.name,
            _ => {
                return Err(CompilerError::code_generation_error(
                    "実装対象が名前付き型ではありません",
                    impl_decl.target_type.location.clone()
                ));
            }
        };
        
        let trait_name = match &impl_decl.trait_name {
            Some(trait_name) => &trait_name.name,
            None => {
                // 自己実装（トレイトなし）の場合
                for method in &impl_decl.methods {
                    let method_impl_name = format!("{}_{}", type_name, method.name.name);
                    self.generate_function_declaration(method, decl)?;
                }
                
                return Ok(());
            }
        };
        
        // トレイト実装からメソッドを処理
        for method in &impl_decl.methods {
            let method_impl_name = format!("{}_{}_{}", trait_name, type_name, method.name.name);
            self.generate_function_declaration(method, decl)?;
        }
        
        // 関連型の具体化情報を保存
        if let Some(assoc_types) = &impl_decl.associated_types {
            for (assoc_name, assoc_type) in assoc_types {
                let md_name = format!("{}_impl_{}_assoc_{}", trait_name, type_name, assoc_name.name);
                
                // 関連型の具体化をメタデータとして保存
                if let Some(type_info) = self.type_info.get_node_type(assoc_type.id) {
                    let md = self.context.metadata_string(&format!("{:?}", type_info));
                    self.llvm_module.add_named_metadata_operand(&md_name, md);
                }
            }
        }
        
        Ok(())
    }
    
    /// 型エイリアスの生成
    fn generate_type_alias(&mut self, alias: &TypeAlias, decl: &Declaration) -> Result<()> {
        // 型エイリアスは主に中間表現に情報を追加
        let alias_name = &alias.name.name;
        
        // エイリアス先の型を解決
        if let Some(type_info) = self.type_info.get_node_type(alias.target_type.id) {
            let ir_type = self.convert_to_ir_type(&type_info)?;
            
            // 中間表現に型エイリアスを追加
            self.module.add_type_alias(alias_name.clone(), ir_type);
        } else {
            return Err(CompilerError::code_generation_error(
                format!("型エイリアス '{}' のターゲット型情報がありません", alias_name),
                alias.target_type.location.clone()
            ));
        }
        
        Ok(())
    }
    
    /// インポート宣言の生成
    fn generate_import(&mut self, import: &Import, decl: &Declaration) -> Result<()> {
        // インポートはコンパイル前の段階で処理されるので、
        // ここでは中間表現に記録するのみ
        let module_path = &import.path;
        
        // 中間表現にインポート情報を追加
        self.module.add_import(module_path.clone());
        
        Ok(())
    }
    
    /// while文の生成
    fn generate_while_statement(&mut self, condition: &Expression, body: &Statement) -> Result<()> {
        // 現在の関数
        let current_fn = self.current_function.ok_or_else(|| {
            CompilerError::code_generation_error(
                "関数コンテキスト外でのwhile文は無効です",
                condition.location.clone()
            )
        })?;
        
        // ループ関連の基本ブロックを作成
        let cond_block = self.context.append_basic_block(current_fn, "while.cond");
        let body_block = self.context.append_basic_block(current_fn, "while.body");
        let end_block = self.context.append_basic_block(current_fn, "while.end");
        
        // 現在のブロックから条件ブロックへの分岐を追加
        self.builder.build_unconditional_branch(cond_block);
        
        // 条件ブロックで条件を評価
        self.builder.position_at_end(cond_block);
        self.current_block = Some(cond_block);
        
        // 条件式を評価
        let cond_value = self.generate_expression(condition)?;
        
        // 条件をブール値に変換
        let cond_bool = match cond_value {
                        BasicValueEnum::IntValue(int_val) => {
                            // 整数値を比較（0でない値はtrue）
                            let zero = int_val.get_type().const_zero();
                            self.builder.build_int_compare(
                                inkwell::IntPredicate::NE,
                                int_val,
                                zero,
                    "whilecond"
                            )
                        },
                        _ => {
                            return Err(CompilerError::code_generation_error(
                    "while文の条件にはブール型または整数型が必要です",
                    condition.location.clone()
                            ));
                        }
                    };
                    
        // 条件に基づいて分岐
        self.builder.build_conditional_branch(cond_bool, body_block, end_block);
        
        // ループ本体ブロックの生成
        self.builder.position_at_end(body_block);
        self.current_block = Some(body_block);
        
        // ループ情報を保存（break/continueのため）
        let old_loop_info = (self.current_loop_condition, self.current_loop_exit);
        self.current_loop_condition = Some(cond_block);
        self.current_loop_exit = Some(end_block);
        
        // ループ本体を生成
        self.generate_statement(body)?;
        
        // ループ情報を復元
        self.current_loop_condition = old_loop_info.0;
        self.current_loop_exit = old_loop_info.1;
        
        // ループ本体の最後に条件ブロックへの無条件分岐を追加
        if !self.builder.get_insert_block().unwrap().get_terminator().is_some() {
            self.builder.build_unconditional_branch(cond_block);
        }
        
        // ループ終了ブロックに移動
        self.builder.position_at_end(end_block);
        self.current_block = Some(end_block);
        
        // ループ本体ブロックの先頭でブロックカウンターを生成
        self.builder.position_at_end(body_block);
        let loop_id = self.generate_temp_name("while_loop");
        self.generate_block_counter(&loop_id)?;
        
        // 条件評価部分でホットスポット検出
        self.builder.position_at_end(cond_block);
        self.generate_hotspot_detection(&format!("while_cond_{}", loop_id))?;
        
        Ok(())
    }
    
    /// for文の生成
    fn generate_for_statement(&mut self, variable: &Identifier, range: &Expression, body: &Statement) -> Result<()> {
        // 現在の関数
        let current_fn = self.current_function.ok_or_else(|| {
            CompilerError::code_generation_error(
                "関数コンテキスト外でのfor文は無効です",
                variable.location.clone()
            )
        })?;
        
        // range式を評価（イテレータまたは範囲）
        // 簡単のため、ここではrange式は整数範囲と仮定
        // 実際の実装ではもっと複雑な処理が必要
        
        // 初期値、終了値、ステップ値を決定
        let (start_value, end_value, step_value) = match &range.kind {
            ExpressionKind::Call(callee, args) => {
                // range関数呼び出しの場合
                if let ExpressionKind::Identifier(ident) = &callee.kind {
                    if ident.name == "range" && args.len() >= 2 {
                        // 開始値
                        let start = self.generate_expression(&args[0])?;
                        // 終了値
                        let end = self.generate_expression(&args[1])?;
                        // ステップ値（省略時は1）
                        let step = if args.len() > 2 {
                            self.generate_expression(&args[2])?
                        } else {
                            self.context.i64_type().const_int(1, false).into()
                        };
                        
                        match (start, end, step) {
                            (BasicValueEnum::IntValue(s), BasicValueEnum::IntValue(e), BasicValueEnum::IntValue(st)) => {
                                (s, e, st)
                            },
                _ => {
                    return Err(CompilerError::code_generation_error(
                                    "rangeの引数は整数型である必要があります",
                                    range.location.clone()
                    ));
                }
            }
                    } else {
                        return Err(CompilerError::code_generation_error(
                            "サポートされていないrange式です",
                            range.location.clone()
                        ));
                    }
                } else {
                    return Err(CompilerError::code_generation_error(
                        "サポートされていないrange式です",
                        range.location.clone()
                    ));
                }
            },
            _ => {
                return Err(CompilerError::code_generation_error(
                    "サポートされていないrange式です",
                    range.location.clone()
                ));
            }
        };
        
        // forループの基本ブロックを作成
        let init_block = self.context.append_basic_block(current_fn, "for.init");
        let cond_block = self.context.append_basic_block(current_fn, "for.cond");
        let body_block = self.context.append_basic_block(current_fn, "for.body");
        let inc_block = self.context.append_basic_block(current_fn, "for.inc");
        let end_block = self.context.append_basic_block(current_fn, "for.end");
        
        // 初期化ブロックへ分岐
        self.builder.build_unconditional_branch(init_block);
        
        // 初期化ブロック: イテレータ変数を初期化
        self.builder.position_at_end(init_block);
        self.current_block = Some(init_block);
        
        // イテレータ変数の型
        let var_type = start_value.get_type();
        
        // イテレータ変数のためのローカル変数を作成
        let var_ptr = self.builder.build_alloca(var_type, &variable.name);
        
        // 初期値をストア
        self.builder.build_store(var_ptr, start_value);
        
        // 変数マップに追加
        self.variables.insert(variable.name.clone(), var_ptr);
        
        // 条件ブロックへ分岐
        self.builder.build_unconditional_branch(cond_block);
        
        // 条件ブロック: ループ継続条件をチェック
        self.builder.position_at_end(cond_block);
        self.current_block = Some(cond_block);
        
        // 現在のイテレータ値をロード
        let current_value = self.builder.build_load(var_type, var_ptr, &variable.name);
        
        // 終了条件をチェック（i < end）
        let cond = self.builder.build_int_compare(
            inkwell::IntPredicate::SLT,
            current_value.into_int_value(),
            end_value,
            "forcond"
        );
        
        // 条件に基づいて分岐
        self.builder.build_conditional_branch(cond, body_block, end_block);
        
        // ループ本体ブロック
        self.builder.position_at_end(body_block);
        self.current_block = Some(body_block);
        
        // ループ情報を保存（break/continueのため）
        let old_loop_info = (self.current_loop_condition, self.current_loop_exit);
        self.current_loop_condition = Some(inc_block);  // continue先
        self.current_loop_exit = Some(end_block);      // break先
        
        // ループ本体を生成
        self.generate_statement(body)?;
        
        // ループ情報を復元
        self.current_loop_condition = old_loop_info.0;
        self.current_loop_exit = old_loop_info.1;
        
        // インクリメントブロックへ分岐
        if !self.builder.get_insert_block().unwrap().get_terminator().is_some() {
            self.builder.build_unconditional_branch(inc_block);
        }
        
        // インクリメントブロック: イテレータ変数を更新
        self.builder.position_at_end(inc_block);
        self.current_block = Some(inc_block);
        
        // 現在の値をロード
        let current_value = self.builder.build_load(var_type, var_ptr, &variable.name);
        
        // 値を増加
        let next_value = self.builder.build_int_add(
            current_value.into_int_value(),
            step_value,
            "fornext"
        );
        
        // 新しい値をストア
        self.builder.build_store(var_ptr, next_value);
        
        // 条件ブロックへ戻る
        self.builder.build_unconditional_branch(cond_block);
        
        // ループ終了ブロックに移動
        self.builder.position_at_end(end_block);
        self.current_block = Some(end_block);
        
        // ループ本体ブロックの先頭でブロックカウンターを生成
        self.builder.position_at_end(body_block);
        let loop_id = self.generate_temp_name("for_loop");
        self.generate_block_counter(&loop_id)?;
        
        // ループ分析
        self.generate_loop_analysis(&loop_id, Some(end_val))?;
        
        Ok(())
    }
    
    /// return文の生成
    fn generate_return_statement(&mut self, expr: Option<&Expression>) -> Result<()> {
        // 現在の関数
        let current_fn = self.current_function.ok_or_else(|| {
            CompilerError::code_generation_error(
                "関数コンテキスト外でのreturn文は無効です",
                None
            )
        })?;
        
        // 戻り値がある場合
        if let Some(return_expr) = expr {
            // 戻り値式を評価
            let return_value = self.generate_expression(return_expr)?;
            
            // 戻り値を返す
            self.builder.build_return(Some(&return_value));
        } else {
            // 戻り値がない場合（void関数）
            self.builder.build_return(None);
        }
        
        Ok(())
    }
    
    /// break文の生成
    fn generate_break_statement(&mut self) -> Result<()> {
        // 現在のループのexit先ブロック
        let exit_block = self.current_loop_exit.ok_or_else(|| {
            CompilerError::code_generation_error(
                "ループの外側でのbreak文は無効です",
                None
            )
        })?;
        
        // ループの終了ブロックへ分岐
        self.builder.build_unconditional_branch(exit_block);
        
        Ok(())
    }
    
    /// continue文の生成
    fn generate_continue_statement(&mut self) -> Result<()> {
        // 現在のループのcondition/increment先ブロック
        let cond_block = self.current_loop_condition.ok_or_else(|| {
            CompilerError::code_generation_error(
                "ループの外側でのcontinue文は無効です",
                None
            )
        })?;
        
        // ループの条件/インクリメントブロックへ分岐
        self.builder.build_unconditional_branch(cond_block);
        
        Ok(())
    }
    
    /// try-catch式の生成
    fn generate_try_catch_expr(&mut self, try_expr: &Expression, catch_clauses: &[(TypeAnnotation, Identifier, Expression)]) -> Result<BasicValueEnum<'ctx>> {
        // 現在の関数
        let current_fn = self.current_function.ok_or_else(|| {
            CompilerError::code_generation_error(
                "関数コンテキスト外でのtry-catch式は無効です",
                try_expr.location.clone()
            )
        })?;
        
        // 結果型を推論
        let result_type = if let Some(type_info) = self.type_info.get_node_type(try_expr.id) {
            self.convert_type_from_annotation(&type_info)?
        } else {
            return Err(CompilerError::code_generation_error(
                "try式の型情報が見つかりません",
                try_expr.location.clone()
            ));
        };
        
        // 基本ブロックの作成
        let try_block = self.context.append_basic_block(current_fn, "try");
        let catch_dispatch_block = self.context.append_basic_block(current_fn, "catch_dispatch");
        let end_block = self.context.append_basic_block(current_fn, "try_end");
        
        // catchハンドラのブロックを作成
        let mut catch_blocks = Vec::with_capacity(catch_clauses.len());
        for (i, _) in catch_clauses.iter().enumerate() {
            catch_blocks.push(self.context.append_basic_block(current_fn, &format!("catch_{}", i)));
        }
        
        // 結果値を格納する変数
        let result_alloca = self.builder.build_alloca(result_type, "try_result");
        
        // エラー情報を格納する変数
        let error_type = self.context.i8_type().ptr_type(AddressSpace::Generic);
        let error_alloca = self.builder.build_alloca(error_type, "error_info");
        
        // ステータスフラグ（0=成功、1=エラー）を格納する変数
        let status_type = self.context.i8_type();
        let status_alloca = self.builder.build_alloca(status_type, "try_status");
        self.builder.build_store(status_alloca, status_type.const_zero());
        
        // try部分へ分岐
        self.builder.build_unconditional_branch(try_block);
        
        // try部分の生成
        self.builder.position_at_end(try_block);
        self.current_block = Some(try_block);
        
        // 例外ハンドリングコンテキストを設定
        let old_exception_handler = self.current_exception_handler;
        self.current_exception_handler = Some((status_alloca, error_alloca, catch_dispatch_block));
        
        // try式を評価
        let try_value = self.generate_expression(try_expr)?;
        
        // 結果を格納
        self.builder.build_store(result_alloca, try_value);
        
        // 例外ハンドリングコンテキストを復元
        self.current_exception_handler = old_exception_handler;
        
        // ステータスをチェック
        let status = self.builder.build_load(status_type, status_alloca, "status_check");
        let is_error = self.builder.build_int_compare(
            inkwell::IntPredicate::EQ,
            status.into_int_value(),
            status_type.const_int(1, false),
            "is_error"
        );
        
        // 条件に基づいて分岐
        self.builder.build_conditional_branch(is_error, catch_dispatch_block, end_block);
        
        // catch処理のディスパッチブロック
        self.builder.position_at_end(catch_dispatch_block);
        self.current_block = Some(catch_dispatch_block);
        
        // エラーコードをロード
        let error_ptr = self.builder.build_load(error_type, error_alloca, "error_ptr");
        
        // エラータイプ情報をロード（仮にエラーポインタの先頭i32をタイプ情報とする）
        let error_type_ptr = unsafe {
            self.builder.build_struct_gep(
                self.context.i8_type(),
                error_ptr.into_pointer_value(),
                0,
                "error_type_ptr"
            )?
        };
        let error_type_id = self.builder.build_load(
            self.context.i32_type(),
            error_type_ptr,
            "error_type_id"
        );
        
        // Phi節点用の値を格納するための変数
        let mut incoming_values = Vec::new();
        let mut incoming_blocks = Vec::new();
        
        // 各catchハンドラをチェック
        let mut current_dispatch_block = catch_dispatch_block;
        
        for (i, (exception_type, binding, handler_expr)) in catch_clauses.iter().enumerate() {
            let next_dispatch_block = if i < catch_clauses.len() - 1 {
                self.context.append_basic_block(current_fn, &format!("catch_dispatch_{}", i + 1))
            } else {
                // 最後のcatchブロックの次はエラー終了
                end_block
            };
            
            // エラータイプをチェック
            let exception_type_id = self.get_exception_type_id(exception_type)?;
            let type_match = self.builder.build_int_compare(
                inkwell::IntPredicate::EQ,
                error_type_id.into_int_value(),
                self.context.i32_type().const_int(exception_type_id as u64, false),
                "type_match"
            );
            
            // 型が一致したらハンドラにジャンプ、そうでなければ次のディスパッチへ
            self.builder.build_conditional_branch(type_match, catch_blocks[i], next_dispatch_block);
            
            // 次のディスパッチブロックへ移動
            if i < catch_clauses.len() - 1 {
                self.builder.position_at_end(next_dispatch_block);
                self.current_block = Some(next_dispatch_block);
                current_dispatch_block = next_dispatch_block;
            }
            
            // ハンドラブロックを生成
            self.builder.position_at_end(catch_blocks[i]);
            self.current_block = Some(catch_blocks[i]);
            
            // エラーオブジェクトをローカル変数にバインド
            let binding_ptr = self.builder.build_alloca(error_ptr.get_type(), &binding.name);
            self.builder.build_store(binding_ptr, error_ptr);
            self.variables.insert(binding.name.clone(), binding_ptr);
            
            // ハンドラ式を評価
            let handler_value = self.generate_expression(handler_expr)?;
            
            // ハンドラからの遷移元情報を記録
            incoming_values.push(handler_value);
            incoming_blocks.push(self.builder.get_insert_block().unwrap());
            
            // 終了ブロックへ分岐
            self.builder.build_unconditional_branch(end_block);
        }
        
        // 終了ブロックに移動
        self.builder.position_at_end(end_block);
        self.current_block = Some(end_block);
        
        // try部分からの正常値をPhi入力として追加
        let normal_value = self.builder.build_load(result_type, result_alloca, "normal_result");
        incoming_values.push(normal_value);
        
        // try部分の正常終了ブロックを特定
        let try_success_block = try_block;
        incoming_blocks.push(try_success_block);
        
        // Phi節点を作成
        let phi = self.builder.build_phi(result_type, "try_result_phi");
        
        // Phi節点に値を追加
        for (value, block) in incoming_values.iter().zip(incoming_blocks.iter()) {
            phi.add_incoming(&[(&*value, *block)]);
        }
        
        Ok(phi.as_basic_value())
    }
    
    /// エラーを投げる（throw式の生成）
    fn generate_throw_expr(&mut self, error_expr: &Expression) -> Result<BasicValueEnum<'ctx>> {
        // 例外ハンドラ情報を取得
        let (status_alloca, error_alloca, catch_block) = self.current_exception_handler.ok_or_else(|| {
            CompilerError::code_generation_error(
                "try-catch外でのthrow式は無効です",
                error_expr.location.clone()
            )
        })?;
        
        // エラーオブジェクトを生成
        let error_value = self.generate_expression(error_expr)?;
        
        // エラー情報を格納
        self.builder.build_store(error_alloca, error_value);
        
        // ステータスを「エラー」に設定
        self.builder.build_store(status_alloca, self.context.i8_type().const_int(1, false));
        
        // catch処理へジャンプ
        self.builder.build_unconditional_branch(catch_block);
        
        // ダミーのunreachableブロックを作成
        let current_fn = self.current_function.unwrap();
        let unreachable_block = self.context.append_basic_block(current_fn, "unreachable_after_throw");
        self.builder.position_at_end(unreachable_block);
        self.current_block = Some(unreachable_block);
        
        // throwの戻り値はvoid（実際には到達しない）
        Ok(self.context.i32_type().const_zero().into())
    }
    
    /// 例外型IDを取得
    fn get_exception_type_id(&self, exception_type: &TypeAnnotation) -> Result<u32> {
        // 実際の実装では型システムと連携してユニークなID取得
        // ここでは簡易的に型名のハッシュ値を使用
        match &exception_type.kind {
            TypeKind::Named(ident) => {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                std::hash::Hash::hash(&ident.name, &mut hasher);
                let hash = std::hash::Hasher::finish(&hasher);
                Ok((hash % 0xFFFFFFFF) as u32)
            },
            _ => Err(CompilerError::code_generation_error(
                "名前付き型のみが例外型として使用できます",
                exception_type.location.clone()
            )),
        }
    }
    
    /// エラーモナド（Result型）のbind操作生成
    fn generate_result_bind(&mut self, result_expr: &Expression, bind_func: &Expression) -> Result<BasicValueEnum<'ctx>> {
        // 現在の関数
        let current_fn = self.current_function.ok_or_else(|| {
            CompilerError::code_generation_error(
                "関数コンテキスト外でのResult bind操作は無効です",
                result_expr.location.clone()
            )
        })?;
        
        // Result型の評価
        let result_value = self.generate_expression(result_expr)?;
        
        // Result型は{success: bool, value: T | error: E}として表現
        let result_ptr = self.builder.build_alloca(result_value.get_type(), "result_value");
        self.builder.build_store(result_ptr, result_value);
        
        // success フラグをロード (struct の最初のフィールドと仮定)
        let success_ptr = unsafe {
            self.builder.build_struct_gep(
                result_value.get_type(),
                result_ptr,
                0,
                "success_ptr"
            )?
        };
        
        let is_success = self.builder.build_load(
            self.context.bool_type(),
            success_ptr,
            "is_success"
        );
        
        // 分岐用ブロックを作成
        let success_block = self.context.append_basic_block(current_fn, "result_success");
        let error_block = self.context.append_basic_block(current_fn, "result_error");
        let merge_block = self.context.append_basic_block(current_fn, "result_merge");
        
        // 成功/失敗に基づいて分岐
        self.builder.build_conditional_branch(
            is_success.into_int_value(),
            success_block,
            error_block
        );
        
        // 成功ブロック - 値を取り出してバインド関数に渡す
        self.builder.position_at_end(success_block);
        self.current_block = Some(success_block);
        
        // 値フィールドをロード (struct の2番目のフィールドと仮定)
        let value_ptr = unsafe {
            self.builder.build_struct_gep(
                result_value.get_type(),
                result_ptr,
                1,
                "value_ptr"
            )?
        };
        
        let value = self.builder.build_load(
            self.context.i32_type(), // 仮の型、実際には動的に決定
            value_ptr,
            "success_value"
        );
        
        // バインド関数を呼び出し
        let bind_func_value = self.generate_expression(bind_func)?;
        let args = [value.into()];
        
        let bind_result = match bind_func_value {
            BasicValueEnum::PointerValue(func_ptr) => {
                // 関数ポインタを介した呼び出し
                let func_type = func_ptr.get_type().get_element_type();
                if let AnyTypeEnum::FunctionType(ft) = func_type {
                    self.builder.build_call(ft, func_ptr, &args, "bind_call")
                } else {
                    return Err(CompilerError::code_generation_error(
                        "バインド関数が関数型ではありません",
                        bind_func.location.clone()
                    ));
                }
            },
            _ => {
                return Err(CompilerError::code_generation_error(
                    "バインド関数がポインタ型ではありません",
                    bind_func.location.clone()
                ));
            }
        };
        
        let success_result = bind_result.try_as_basic_value().left().ok_or_else(|| {
            CompilerError::code_generation_error(
                "バインド関数の戻り値が無効です",
                bind_func.location.clone()
            )
        })?;
        
        // 成功ブロックから合流ブロックへ
            self.builder.build_unconditional_branch(merge_block);
        let success_end_block = self.builder.get_insert_block().unwrap();
        
        // エラーブロック - 元のエラーをそのまま渡す
        self.builder.position_at_end(error_block);
        self.current_block = Some(error_block);
        
        // エラーフィールドをロード (struct の3番目のフィールドと仮定)
        let error_ptr = unsafe {
            self.builder.build_struct_gep(
                result_value.get_type(),
                result_ptr,
                2,
                "error_ptr"
            )?
        };
        
        let error_value = self.builder.build_load(
            result_value.get_type(), // 仮の型、実際には動的に決定
            error_ptr,
            "error_value"
        );
        
        // 新しいResultオブジェクトを作成（エラー状態）
        let result_struct_type = result_value.get_type();
        let new_result_ptr = self.builder.build_alloca(result_struct_type, "new_error_result");
        
        // success = false
        let success_field_ptr = unsafe {
            self.builder.build_struct_gep(
                result_struct_type,
                new_result_ptr,
                0,
                "new_success_ptr"
            )?
        };
        self.builder.build_store(success_field_ptr, self.context.bool_type().const_zero());
        
        // error = 元のエラー
        let new_error_ptr = unsafe {
            self.builder.build_struct_gep(
                result_struct_type,
                new_result_ptr,
                2,
                "new_error_ptr"
            )?
        };
        self.builder.build_store(new_error_ptr, error_value);
        
        // 新しいResult構造体をロード
        let error_result = self.builder.build_load(result_struct_type, new_result_ptr, "propagated_error");
        
        // エラーブロックから合流ブロックへ
        self.builder.build_unconditional_branch(merge_block);
        let error_end_block = self.builder.get_insert_block().unwrap();
        
        // 合流ブロックでphi節点を作成
        self.builder.position_at_end(merge_block);
        self.current_block = Some(merge_block);
        
        let phi = self.builder.build_phi(result_struct_type, "result_phi");
        phi.add_incoming(&[
            (&success_result, success_end_block),
            (&error_result, error_end_block)
        ]);
        
        Ok(phi.as_basic_value())
    }
    
    /// エラーモナド（Result型）のマッピング操作生成
    fn generate_result_map(&mut self, result_expr: &Expression, map_func: &Expression) -> Result<BasicValueEnum<'ctx>> {
        // 現在の関数
        let current_fn = self.current_function.ok_or_else(|| {
            CompilerError::code_generation_error(
                "関数コンテキスト外でのResult map操作は無効です",
                result_expr.location.clone()
            )
        })?;
        
        // Result型の評価
        let result_value = self.generate_expression(result_expr)?;
        
        // Result型は{success: bool, value: T | error: E}として表現
        let result_ptr = self.builder.build_alloca(result_value.get_type(), "result_value");
        self.builder.build_store(result_ptr, result_value);
        
        // success フラグをロード (struct の最初のフィールドと仮定)
        let success_ptr = unsafe {
            self.builder.build_struct_gep(
                result_value.get_type(),
                result_ptr,
                0,
                "success_ptr"
            )?
        };
        
        let is_success = self.builder.build_load(
            self.context.bool_type(),
            success_ptr,
            "is_success"
        );
        
        // 分岐用ブロックを作成
        let success_block = self.context.append_basic_block(current_fn, "result_success");
        let error_block = self.context.append_basic_block(current_fn, "result_error");
        let merge_block = self.context.append_basic_block(current_fn, "result_merge");
        
        // 成功/失敗に基づいて分岐
        self.builder.build_conditional_branch(
            is_success.into_int_value(),
            success_block,
            error_block
        );
        
        // 成功ブロック - 値を取り出してマップ関数に渡す
        self.builder.position_at_end(success_block);
        self.current_block = Some(success_block);
        
        // 値フィールドをロード (struct の2番目のフィールドと仮定)
        let value_ptr = unsafe {
            self.builder.build_struct_gep(
                result_value.get_type(),
                result_ptr,
                1,
                "value_ptr"
            )?
        };
        
        let value = self.builder.build_load(
            self.context.i32_type(), // 仮の型、実際には動的に決定
            value_ptr,
            "success_value"
        );
        
        // マップ関数を呼び出し
        let map_func_value = self.generate_expression(map_func)?;
        let args = [value.into()];
        
        let map_result = match map_func_value {
            BasicValueEnum::PointerValue(func_ptr) => {
                // 関数ポインタを介した呼び出し
                let func_type = func_ptr.get_type().get_element_type();
                if let AnyTypeEnum::FunctionType(ft) = func_type {
                    self.builder.build_call(ft, func_ptr, &args, "map_call")
                } else {
                    return Err(CompilerError::code_generation_error(
                        "マップ関数が関数型ではありません",
                        map_func.location.clone()
                    ));
                }
            },
            _ => {
                return Err(CompilerError::code_generation_error(
                    "マップ関数がポインタ型ではありません",
                    map_func.location.clone()
                ));
            }
        };
        
        let mapped_value = map_result.try_as_basic_value().left().ok_or_else(|| {
            CompilerError::code_generation_error(
                "マップ関数の戻り値が無効です",
                map_func.location.clone()
            )
        })?;
        
        // 新しいResultオブジェクトを作成（成功状態）
        let result_struct_type = result_value.get_type();
        let new_result_ptr = self.builder.build_alloca(result_struct_type, "new_success_result");
        
        // success = true
        let success_field_ptr = unsafe {
            self.builder.build_struct_gep(
                result_struct_type,
                new_result_ptr,
                0,
                "new_success_ptr"
            )?
        };
        self.builder.build_store(success_field_ptr, self.context.bool_type().const_int(1, false));
        
        // value = 変換後の値
        let new_value_ptr = unsafe {
            self.builder.build_struct_gep(
                result_struct_type,
                new_result_ptr,
                1,
                "new_value_ptr"
            )?
        };
        self.builder.build_store(new_value_ptr, mapped_value);
        
        // 新しいResult構造体をロード
        let success_result = self.builder.build_load(result_struct_type, new_result_ptr, "mapped_success");
        
        // 成功ブロックから合流ブロックへ
        self.builder.build_unconditional_branch(merge_block);
        let success_end_block = self.builder.get_insert_block().unwrap();
        
        // エラーブロック - 元のエラーをそのまま渡す
        self.builder.position_at_end(error_block);
        self.current_block = Some(error_block);
        
        // エラーフィールドをロード (struct の3番目のフィールドと仮定)
        let error_ptr = unsafe {
            self.builder.build_struct_gep(
                result_value.get_type(),
                result_ptr,
                2,
                "error_ptr"
            )?
        };
        
        let error_value = self.builder.build_load(
            result_value.get_type(), // 仮の型、実際には動的に決定
            error_ptr,
            "error_value"
        );
        
        // 新しいResultオブジェクトを作成（エラー状態）
        let new_error_result_ptr = self.builder.build_alloca(result_struct_type, "new_error_result");
        
        // success = false
        let new_error_success_ptr = unsafe {
            self.builder.build_struct_gep(
                result_struct_type,
                new_error_result_ptr,
                0,
                "new_error_success_ptr"
            )?
        };
        self.builder.build_store(new_error_success_ptr, self.context.bool_type().const_zero());
        
        // error = 元のエラー
        let new_error_error_ptr = unsafe {
            self.builder.build_struct_gep(
                result_struct_type,
                new_error_result_ptr,
                2,
                "new_error_error_ptr"
            )?
        };
        self.builder.build_store(new_error_error_ptr, error_value);
        
        // 新しいResult構造体をロード
        let error_result = self.builder.build_load(result_struct_type, new_error_result_ptr, "unchanged_error");
        
        // エラーブロックから合流ブロックへ
        self.builder.build_unconditional_branch(merge_block);
        let error_end_block = self.builder.get_insert_block().unwrap();
        
        // 合流ブロックでphi節点を作成
        self.builder.position_at_end(merge_block);
        self.current_block = Some(merge_block);
        
        let phi = self.builder.build_phi(result_struct_type, "result_phi");
        phi.add_incoming(&[
            (&success_result, success_end_block),
            (&error_result, error_end_block)
        ]);
        
        Ok(phi.as_basic_value())
    }
    
    /// ジェネリック関数のモノモーフィゼーション
    fn generate_generic_function_instantiation(&mut self, func_decl: &Function, type_args: &[TypeAnnotation], decl: &Declaration) -> Result<FunctionValue<'ctx>> {
        // 元の関数名
        let base_func_name = &func_decl.name.name;
        
        // 型引数を含む完全な関数名を構築
        let mut instantiated_name = format!("{}<", base_func_name);
        for (i, type_arg) in type_args.iter().enumerate() {
            if i > 0 {
                instantiated_name.push_str(", ");
            }
            
            match &type_arg.kind {
                TypeKind::Named(ident) => {
                    instantiated_name.push_str(&ident.name);
                },
                _ => {
                    // 他の型も文字列表現を追加
                    instantiated_name.push_str(&format!("{:?}", type_arg.kind));
                }
            }
        }
        instantiated_name.push('>');
        
        // 既にインスタンス化されている場合はそれを返す
        if let Some(&existing_func) = self.functions.get(&instantiated_name) {
            return Ok(existing_func);
        }
        
        // 型引数の評価とマッピング
        let mut type_param_map = HashMap::new();
        let type_params = &func_decl.type_parameters;
        
        if type_params.len() != type_args.len() {
            return Err(CompilerError::code_generation_error(
                format!("型引数の数が一致しません: 予期された数 {}, 提供された数 {}", 
                        type_params.len(), type_args.len()),
                decl.location.clone()
            ));
        }
        
        // 型パラメータと引数のマッピングを作成
        for (param, arg) in type_params.iter().zip(type_args.iter()) {
            type_param_map.insert(param.name.clone(), arg.clone());
        }
        
        // 関数のパラメータと戻り値の型を具体化
        let mut concrete_param_types = Vec::new();
        
        for param in &func_decl.parameters {
            if let Some(type_ann) = &param.type_annotation {
                let concrete_type = self.specialize_type(type_ann, &type_param_map)?;
                let llvm_type = self.convert_type_from_annotation(&concrete_type)?;
                concrete_param_types.push(llvm_type);
            } else {
                return Err(CompilerError::code_generation_error(
                    format!("ジェネリック関数のパラメータには型注釈が必要です: {}", param.name.name),
                    param.location.clone()
                ));
            }
        }
        
        // 戻り値型を具体化
        let return_type = if let Some(ret_type) = &func_decl.return_type {
            let concrete_ret_type = self.specialize_type(ret_type, &type_param_map)?;
            self.convert_type_from_annotation(&concrete_ret_type)?
        } else {
            // 戻り値型が指定されていない場合はvoid
            return Err(CompilerError::code_generation_error(
                "ジェネリック関数には戻り値型の注釈が必要です",
                func_decl.location.clone()
            ));
        };
        
        // 関数型の作成
        let fn_type = match return_type {
            BasicTypeEnum::IntType(t) => t.fn_type(&concrete_param_types, false),
            BasicTypeEnum::FloatType(t) => t.fn_type(&concrete_param_types, false),
            BasicTypeEnum::PointerType(t) => t.fn_type(&concrete_param_types, false),
            BasicTypeEnum::StructType(t) => t.fn_type(&concrete_param_types, false),
            BasicTypeEnum::ArrayType(t) => t.fn_type(&concrete_param_types, false),
        };
        
        // 具体化された関数をモジュールに追加
        let function = self.llvm_module.add_function(&instantiated_name, fn_type, None);
        
        // 関数マッピングに追加
        self.functions.insert(instantiated_name.clone(), function);
        
        // 現在の状態を保存
        let old_function = self.current_function;
        let old_block = self.current_block;
        let old_variables = std::mem::take(&mut self.variables);
        
        // 型パラメータの元の状態を保存
        let old_type_params = self.current_type_parameters.clone();
        
        // 現在の型パラメータにこの関数の具体化された型を設定
        self.current_type_parameters = type_param_map;
        
        // 関数本体のエントリーブロックを作成
        let entry_block = self.context.append_basic_block(function, "entry");
        
        // 関数コンテキストを設定
        self.current_function = Some(function);
        self.current_block = Some(entry_block);
        self.builder.position_at_end(entry_block);
        
        // パラメータを登録
        for (i, param) in func_decl.parameters.iter().enumerate() {
            let param_value = function.get_nth_param(i as u32)
                .ok_or_else(|| CompilerError::code_generation_error(
                    format!("パラメータ {} が見つかりません", i),
                    param.location.clone()
                ))?;
            
            // パラメータ名を設定
            param_value.set_name(&param.name.name);
            
            // パラメータ用のローカル変数を作成
            let param_ptr = self.builder.build_alloca(param_value.get_type(), &param.name.name);
            self.builder.build_store(param_ptr, param_value);
            
            // 変数マップに追加
            self.variables.insert(param.name.name.clone(), param_ptr);
        }
        
        // 関数本体を生成
        self.generate_statement(&func_decl.body)?;
        
        // 関数の最後に戻り値がなければvoid型の関数として戻り値なしのreturnを追加
        if !self.builder.get_insert_block().unwrap().get_terminator().is_some() {
            self.builder.build_return(None);
        }
        
        // 状態を復元
        self.current_function = old_function;
        self.current_block = old_block;
        self.variables = old_variables;
        self.current_type_parameters = old_type_params;
        
        // 中間表現にも特殊化された関数を追加
        let mut ir_function = representation::Function::new(
            instantiated_name.clone(),
            self.convert_to_ir_type_from_basic_type(return_type)?
        );
        
        // パラメータを追加
        for (i, param) in func_decl.parameters.iter().enumerate() {
            if let Some(type_ann) = &param.type_annotation {
                let concrete_type = self.specialize_type(type_ann, &type_param_map)?;
                let ir_type = self.convert_to_ir_type(&concrete_type)?;
                let ir_param = representation::Parameter::new(
                    param.name.name.clone(),
                    ir_type,
                    false
                );
                ir_function.add_parameter(ir_param);
            }
        }
        
        self.module.add_function(ir_function);
        
        Ok(function)
    }
    
    /// 型変数を具体的な型で置き換える
    fn specialize_type(&self, type_ann: &TypeAnnotation, type_map: &HashMap<String, TypeAnnotation>) -> Result<TypeAnnotation> {
        match &type_ann.kind {
            TypeKind::Named(ident) => {
                // 型パラメータとして登録されているか確認
                if let Some(concrete_type) = type_map.get(&ident.name) {
                    // 型パラメータを具体的な型で置き換え
                    Ok(concrete_type.clone())
                } else {
                    // 型パラメータではない場合はそのまま
                    Ok(type_ann.clone())
                }
            },
            TypeKind::Array(elem_type) => {
                // 配列要素の型を具体化
                let concrete_elem_type = self.specialize_type(elem_type, type_map)?;
                Ok(TypeAnnotation {
                    id: ast::generate_id(),
                    kind: TypeKind::Array(Box::new(concrete_elem_type)),
                    location: type_ann.location.clone(),
                })
            },
            TypeKind::Function(param_types, ret_type) => {
                // 関数パラメータと戻り値の型を具体化
                let mut concrete_param_types = Vec::new();
                for param_type in param_types {
                    concrete_param_types.push(self.specialize_type(param_type, type_map)?);
                }
                
                let concrete_ret_type = self.specialize_type(ret_type, type_map)?;
                
                Ok(TypeAnnotation {
                    id: ast::generate_id(),
                    kind: TypeKind::Function(concrete_param_types, Box::new(concrete_ret_type)),
                    location: type_ann.location.clone(),
                })
            },
            TypeKind::Optional(inner_type) => {
                // オプショナル型の内部型を具体化
                let concrete_inner_type = self.specialize_type(inner_type, type_map)?;
                Ok(TypeAnnotation {
                    id: ast::generate_id(),
                    kind: TypeKind::Optional(Box::new(concrete_inner_type)),
                    location: type_ann.location.clone(),
                })
            },
            // その他の型はそのまま返す
            _ => Ok(type_ann.clone()),
        }
    }
    
    /// ジェネリックな型の実体化（構造体など）
    fn instantiate_generic_type(&mut self, type_ann: &TypeAnnotation, type_args: &[TypeAnnotation]) -> Result<StructType<'ctx>> {
        if let TypeKind::Named(type_name) = &type_ann.kind {
            // 型引数を含む完全な型名を構築
            let mut instantiated_name = format!("{}<", type_name.name);
            for (i, type_arg) in type_args.iter().enumerate() {
                if i > 0 {
                    instantiated_name.push_str(", ");
                }
                
                match &type_arg.kind {
                    TypeKind::Named(ident) => {
                        instantiated_name.push_str(&ident.name);
                    },
                    _ => {
                        // 他の型も文字列表現を追加
                        instantiated_name.push_str(&format!("{:?}", type_arg.kind));
                    }
                }
            }
            instantiated_name.push('>');
            
            // 既にインスタンス化されている場合はそれを返す
            if let Some(&existing_type) = self.structs.get(&instantiated_name) {
                return Ok(existing_type);
            }
            
            // 元の構造体定義を取得
            let struct_definition = self.module.get_struct_definition(&type_name.name)
                .ok_or_else(|| CompilerError::code_generation_error(
                    format!("ジェネリック型 '{}' の定義が見つかりません", type_name.name),
                    type_ann.location.clone()
                ))?;
            
            // 型パラメータと引数のマッピングを作成
            let mut type_param_map = HashMap::new();
            let type_params = struct_definition.type_parameters();
            
            if type_params.len() != type_args.len() {
                return Err(CompilerError::code_generation_error(
                    format!("型引数の数が一致しません: 予期された数 {}, 提供された数 {}", 
                            type_params.len(), type_args.len()),
                    type_ann.location.clone()
                ));
            }
            
            for (param_name, arg) in type_params.iter().zip(type_args.iter()) {
                type_param_map.insert(param_name.clone(), arg.clone());
            }
            
            // フィールド型を具体化
            let mut concrete_field_types = Vec::new();
            
            for (field_name, field_type) in struct_definition.fields() {
                let concrete_type = self.specialize_type(field_type, &type_param_map)?;
                let llvm_type = self.convert_type_from_annotation(&concrete_type)?;
                concrete_field_types.push(llvm_type);
            }
            
            // 具体化された構造体型を作成
            let struct_type = self.context.opaque_struct_type(&instantiated_name);
            struct_type.set_body(&concrete_field_types, false);
            
            // 構造体マッピングに追加
            self.structs.insert(instantiated_name.clone(), struct_type);
            
            // 中間表現にも具体化された構造体を追加
            let mut ir_fields = Vec::new();
            
            for ((field_name, _), field_type) in struct_definition.fields().iter().zip(concrete_field_types.iter()) {
                let ir_type = self.convert_to_ir_type_from_basic_type(*field_type)?;
                ir_fields.push((field_name.clone(), ir_type));
            }
            
            self.module.add_struct(instantiated_name, ir_fields);
            
            Ok(struct_type)
        } else {
            Err(CompilerError::code_generation_error(
                "指定された型はジェネリック型名ではありません",
                type_ann.location.clone()
            ))
        }
    }
    
    /// 参照カウント増加関数の生成
    fn generate_increment_ref_count(&mut self, object_ptr: PointerValue<'ctx>) -> Result<()> {
        // 参照カウントフィールドのオフセット（最初のフィールドと仮定）
        let ref_count_offset = 0;
        
        // 参照カウントフィールドへのポインタを取得
        let ref_count_ptr = unsafe {
            self.builder.build_struct_gep(
                self.context.i8_type(),
                object_ptr,
                ref_count_offset,
                "ref_count_ptr"
            )?
        };
        
        // 現在の参照カウント値をロード
        let ref_count = self.builder.build_load(
            self.context.i32_type(),
            ref_count_ptr,
            "ref_count"
        );
        
        // 参照カウントをインクリメント
        let new_ref_count = self.builder.build_int_add(
            ref_count.into_int_value(),
            self.context.i32_type().const_int(1, false),
            "new_ref_count"
        );
        
        // 更新した参照カウントを格納
        self.builder.build_store(ref_count_ptr, new_ref_count);
        
        Ok(())
    }
    
    /// 参照カウント減少関数の生成
    fn generate_decrement_ref_count(&mut self, object_ptr: PointerValue<'ctx>) -> Result<()> {
        // 参照カウントフィールドのオフセット（最初のフィールドと仮定）
        let ref_count_offset = 0;
        
        // 参照カウントフィールドへのポインタを取得
        let ref_count_ptr = unsafe {
            self.builder.build_struct_gep(
                self.context.i8_type(),
                object_ptr,
                ref_count_offset,
                "ref_count_ptr"
            )?
        };
        
        // 現在の参照カウント値をロード
        let ref_count = self.builder.build_load(
            self.context.i32_type(),
            ref_count_ptr,
            "ref_count"
        );
        
        // 参照カウントをデクリメント
        let new_ref_count = self.builder.build_int_sub(
            ref_count.into_int_value(),
            self.context.i32_type().const_int(1, false),
            "new_ref_count"
        );
        
        // 更新した参照カウントを格納
        self.builder.build_store(ref_count_ptr, new_ref_count);
        
        // 参照カウントが0になった場合はオブジェクトを解放
        let is_zero = self.builder.build_int_compare(
            inkwell::IntPredicate::EQ,
            new_ref_count,
            self.context.i32_type().const_zero(),
            "is_zero"
        );
        
        // 条件分岐を作成
        let current_fn = self.current_function.unwrap();
        let free_block = self.context.append_basic_block(current_fn, "free_object");
        let continue_block = self.context.append_basic_block(current_fn, "continue");
        
        self.builder.build_conditional_branch(is_zero, free_block, continue_block);
        
        // オブジェクト解放処理
        self.builder.position_at_end(free_block);
        
        // デストラクタを呼び出す
        self.generate_object_destructor(object_ptr)?;
        
        // freeを呼び出す
        if let Some(&free_fn) = self.functions.get("free") {
            // オブジェクトポインタをvoid*にキャスト
            let void_ptr = self.builder.build_bitcast(
                object_ptr,
                self.context.i8_type().ptr_type(AddressSpace::Generic),
                "void_ptr"
            );
            
            // free関数を呼び出し
            self.builder.build_call(free_fn.get_type(), free_fn, &[void_ptr.into()], "free_call");
            
            // continueブロックへ分岐
            self.builder.build_unconditional_branch(continue_block);
        } else {
            return Err(CompilerError::code_generation_error(
                "free関数が見つかりません",
                None
            ));
        }
        
        // 続行ブロックに移動
        self.builder.position_at_end(continue_block);
        
        Ok(())
    }
    
    /// オブジェクトデストラクタの生成
    fn generate_object_destructor(&mut self, object_ptr: PointerValue<'ctx>) -> Result<()> {
        // TypeInfoフィールドからデストラクタ関数ポインタを取得（2番目のフィールドと仮定）
        let type_info_offset = 1;
        
        // TypeInfoフィールドへのポインタを取得
        let type_info_ptr = unsafe {
            self.builder.build_struct_gep(
                self.context.i8_type(),
                object_ptr,
                type_info_offset,
                "type_info_ptr"
            )?
        };
        
        // TypeInfo構造体をロード
        let type_info = self.builder.build_load(
            self.context.i8_type().ptr_type(AddressSpace::Generic),
            type_info_ptr,
            "type_info"
        );
        
        // デストラクタ関数ポインタを取得（TypeInfo構造体の最初のフィールドと仮定）
        let dtor_ptr_ptr = unsafe {
            self.builder.build_struct_gep(
                self.context.i8_type(),
                type_info.into_pointer_value(),
                0,
                "dtor_ptr_ptr"
            )?
        };
        
        // デストラクタ関数ポインタをロード
        let dtor_ptr = self.builder.build_load(
            self.context.i8_type().ptr_type(AddressSpace::Generic),
            dtor_ptr_ptr,
            "dtor_ptr"
        );
        
        // デストラクタ関数型: void (*)(void*)
        let void_type = self.context.void_type();
        let void_ptr_type = self.context.i8_type().ptr_type(AddressSpace::Generic);
        let dtor_type = void_type.fn_type(&[void_ptr_type.into()], false);
        
        // オブジェクトポインタをvoid*にキャスト
        let void_obj_ptr = self.builder.build_bitcast(
            object_ptr,
            void_ptr_type,
            "void_obj_ptr"
        );
        
        // デストラクタを呼び出し
        self.builder.build_call(
            dtor_type,
            dtor_ptr.into_pointer_value(),
            &[void_obj_ptr.into()],
            "dtor_call"
        );
        
        Ok(())
    }
    
    /// リージョンベースのメモリ割り当て関数の生成
    fn generate_region_allocate(&mut self, size: IntValue<'ctx>, region_id: u32) -> Result<PointerValue<'ctx>> {
        // リージョン割り当て関数がある場合はそれを使用
        if let Some(&region_alloc_fn) = self.functions.get("region_allocate") {
            // リージョンIDを定数として作成
            let region_id_const = self.context.i32_type().const_int(region_id as u64, false);
            
            // 関数を呼び出し
            let result = self.builder.build_call(
                region_alloc_fn.get_type(),
                region_alloc_fn,
                &[size.into(), region_id_const.into()],
                "region_alloc_call"
            );
            
            // 結果をポインタとして取得
            let allocated_ptr = result.try_as_basic_value().left().ok_or_else(|| {
                CompilerError::code_generation_error(
                    "リージョン割り当て関数の戻り値が無効です",
                    None
                )
            })?.into_pointer_value();
            
            Ok(allocated_ptr)
        } else {
            // リージョン割り当て関数がない場合は通常のmallocにフォールバック
            if let Some(&malloc_fn) = self.functions.get("malloc") {
                let result = self.builder.build_call(
                    malloc_fn.get_type(),
                    malloc_fn,
                    &[size.into()],
                    "malloc_call"
                );
                
                let allocated_ptr = result.try_as_basic_value().left().ok_or_else(|| {
                    CompilerError::code_generation_error(
                        "malloc関数の戻り値が無効です",
                        None
                    )
                })?.into_pointer_value();
                
                Ok(allocated_ptr)
            } else {
                Err(CompilerError::code_generation_error(
                    "メモリ割り当て関数が見つかりません",
                    None
                ))
            }
        }
    }
    
    /// リージョンの解放関数の生成
    fn generate_region_free(&mut self, region_id: u32) -> Result<()> {
        // リージョン解放関数がある場合はそれを使用
        if let Some(&region_free_fn) = self.functions.get("region_free") {
            // リージョンIDを定数として作成
            let region_id_const = self.context.i32_type().const_int(region_id as u64, false);
            
            // 関数を呼び出し
            self.builder.build_call(
                region_free_fn.get_type(),
                region_free_fn,
                &[region_id_const.into()],
                "region_free_call"
            );
            
            Ok(())
        } else {
            // このケースではエラーではなく警告として扱う
            self.add_error("リージョン解放関数が見つからないため、メモリリークが発生する可能性があります", None);
            Ok(())
        }
    }
    
    /// スマートポインタの作成（所有権ベースのメモリ管理）
    fn generate_unique_ptr(&mut self, value_ptr: PointerValue<'ctx>, value_type: BasicTypeEnum<'ctx>) -> Result<BasicValueEnum<'ctx>> {
        // UniquePtr構造体型を取得または作成
        let unique_ptr_type_name = format!("UniquePtr<{:?}>", value_type);
        
        let unique_ptr_struct_type = if let Some(&st) = self.structs.get(&unique_ptr_type_name) {
            st
        } else {
            // UniquePtr構造体型を作成
            let struct_type = self.context.opaque_struct_type(&unique_ptr_type_name);
            let field_types = &[value_type.ptr_type(AddressSpace::Generic).into()];
            struct_type.set_body(field_types, false);
            
            // 構造体マッピングに追加
            self.structs.insert(unique_ptr_type_name.clone(), struct_type);
            
            // 中間表現にも登録
            let ir_value_type = self.convert_to_ir_type_from_basic_type(value_type)?;
            let ir_ptr_type = Type::Pointer(Box::new(ir_value_type));
            self.module.add_struct(
                unique_ptr_type_name.clone(),
                vec![("ptr".to_string(), ir_ptr_type)]
            );
            
            struct_type
        };
        
        // UniquePtr構造体のインスタンスをスタックに確保
        let unique_ptr = self.builder.build_alloca(unique_ptr_struct_type, "unique_ptr");
        
        // ポインタフィールドへのアクセス
        let ptr_field_ptr = unsafe {
            self.builder.build_struct_gep(
                unique_ptr_struct_type,
                unique_ptr,
                0,
                "ptr_field"
            )?
        };
        
        // ポインタ値を格納
        self.builder.build_store(ptr_field_ptr, value_ptr);
        
        // UniquePtr構造体をロード
        let result = self.builder.build_load(unique_ptr_struct_type, unique_ptr, "unique_ptr_val");
        
        Ok(result)
    }
    
    /// インライン展開のヒント設定
    fn set_inline_hint(&mut self, function: FunctionValue<'ctx>, strategy: InlineStrategy) -> Result<()> {
        let metadata_kind_id = match strategy {
            InlineStrategy::Always => self.context.get_enum_attribute_kind_id("alwaysinline"),
            InlineStrategy::Never => self.context.get_enum_attribute_kind_id("noinline"),
            InlineStrategy::Hint => self.context.get_enum_attribute_kind_id("inlinehint"),
        };
        
        let attribute = self.context.create_enum_attribute(metadata_kind_id, 0);
        function.add_attribute(inkwell::attributes::AttributeLoc::Function, attribute);
        
        Ok(())
    }
    
    /// ループのベクトル化ヒント設定
    fn set_vectorize_hint(&mut self, loop_id: &str, enable: bool) -> Result<()> {
        // 現在の関数を取得
        let function = self.current_function.ok_or_else(|| {
            CompilerError::code_generation_error(
                "関数コンテキスト外でのベクトル化ヒント設定は無効です",
                None
            )
        })?;
        
        // ループのメタデータノードを作成
        let loop_metadata = self.context.metadata_node(&[]);
        
        // ベクトル化ヒントを設定
        let hint_name = if enable { "llvm.loop.vectorize.enable" } else { "llvm.loop.vectorize.disable" };
        let hint_value = self.context.i1_type().const_int(1, false);
        
        // メタデータを関数に付加
        function.set_metadata(loop_metadata, self.context.get_md_kind_id(hint_name).unwrap_or(0));
        
        Ok(())
    }
    
    /// コンパイル時境界チェック
    fn generate_bounds_check(&mut self, array_ptr: PointerValue<'ctx>, index: IntValue<'ctx>, array_len: Option<IntValue<'ctx>>) -> Result<()> {
        // 現在の関数を取得
        let current_fn = self.current_function.ok_or_else(|| {
            CompilerError::code_generation_error(
                "関数コンテキスト外での境界チェックは無効です",
                None
            )
        })?;
        
        // 配列の長さを取得または計算
        let len = if let Some(len_val) = array_len {
            len_val
        } else {
            // 配列の型情報から長さを取得（配列の直前に長さが格納されていると仮定）
            let len_ptr = unsafe {
                self.builder.build_in_bounds_gep(
                    self.context.i32_type(),
                    array_ptr,
                    &[self.context.i32_type().const_int(u64::MAX, true)],
                    "array_len_ptr"
                )
            };
            
            self.builder.build_load(self.context.i32_type(), len_ptr, "array_len").into_int_value()
        };
        
        // インデックスが負の場合のチェック
        let is_negative = self.builder.build_int_compare(
            inkwell::IntPredicate::SLT,
            index,
            self.context.i32_type().const_zero(),
            "is_negative"
        );
        
        // インデックスが長さ以上の場合のチェック
        let is_too_large = self.builder.build_int_compare(
            inkwell::IntPredicate::SGE,
            index,
            len,
            "is_too_large"
        );
        
        // いずれかの条件に該当する場合は境界エラー
        let is_out_of_bounds = self.builder.build_or(
            is_negative,
            is_too_large,
            "is_out_of_bounds"
        );
        
        // 境界チェックブロックを作成
        let check_failed_block = self.context.append_basic_block(current_fn, "bounds_check_failed");
        let check_passed_block = self.context.append_basic_block(current_fn, "bounds_check_passed");
        
        // 条件分岐
        self.builder.build_conditional_branch(is_out_of_bounds, check_failed_block, check_passed_block);
        
        // 境界チェック失敗時の処理
        self.builder.position_at_end(check_failed_block);
        
        // エラーメッセージを作成
            let error_msg = self.builder.build_global_string_ptr(
            "インデックスが配列の境界外です", 
            "bounds_error_msg"
            );
            
        // ランタイムエラー関数を呼び出す
        if let Some(&panic_fn) = self.functions.get("swiftlight_panic") {
            self.builder.build_call(
                panic_fn.get_type(),
                panic_fn, 
                &[error_msg.as_pointer_value().into()], 
                "panic_call"
            );
            
            // 到達しないコード
            self.builder.build_unreachable();
        } else {
            // 標準的なプログラム終了関数にフォールバック
            if let Some(&exit_fn) = self.functions.get("exit") {
                let exit_code = self.context.i32_type().const_int(1, false);
                self.builder.build_call(
                    exit_fn.get_type(),
                    exit_fn,
                    &[exit_code.into()],
                    "exit_call"
                );
                
                // 到達しないコード
                self.builder.build_unreachable();
            } else {
                // エラー関数が見つからない場合は単にプログラムを停止
                self.builder.build_unreachable();
            }
        }
        
        // 境界チェック通過後の処理（続行）
        self.builder.position_at_end(check_passed_block);
        
        Ok(())
    }
    
    /// ゼロ初期化メモリ割り当て
    fn generate_calloc_array(&mut self, elem_type: BasicTypeEnum<'ctx>, count: IntValue<'ctx>) -> Result<PointerValue<'ctx>> {
        // 要素サイズを計算
        let elem_size = self.context.i64_type().const_int(
            self.llvm_module.get_data_layout().get_store_size(&elem_type) as u64,
            false
        );
        
        // 総サイズを計算（要素サイズ × 要素数）
        let count_64 = self.builder.build_int_z_extend(count, self.context.i64_type(), "count_64");
        let total_size = self.builder.build_int_mul(elem_size, count_64, "total_size");
        
        // callocがある場合はそれを使用
        if let Some(&calloc_fn) = self.functions.get("calloc") {
            // calloc(count, elem_size)を呼び出し
            let result = self.builder.build_call(
                calloc_fn.get_type(),
                calloc_fn,
                &[count_64.into(), elem_size.into()],
                "calloc_call"
            );
            
            // ポインタを取得して適切な型にキャスト
            let void_ptr = result.try_as_basic_value().left().ok_or_else(|| {
                CompilerError::code_generation_error(
                    "calloc関数の戻り値が無効です",
                    None
                )
            })?.into_pointer_value();
            
            let typed_ptr = self.builder.build_bitcast(
                void_ptr,
                elem_type.ptr_type(AddressSpace::Generic),
                "typed_array"
            );
            
            Ok(typed_ptr)
        } else if let Some(&malloc_fn) = self.functions.get("malloc") {
            // mallocを使用し、その後memset(0)で初期化
            let result = self.builder.build_call(
                malloc_fn.get_type(),
                malloc_fn,
                &[total_size.into()],
                "malloc_call"
            );
            
            let void_ptr = result.try_as_basic_value().left().ok_or_else(|| {
                CompilerError::code_generation_error(
                    "malloc関数の戻り値が無効です",
                    None
                )
            })?.into_pointer_value();
            
            // memsetで0初期化
            if let Some(&memset_fn) = self.functions.get("memset") {
                self.builder.build_call(
                    memset_fn.get_type(),
                    memset_fn,
                    &[
                        void_ptr.into(),
                        self.context.i8_type().const_zero().into(),
                        total_size.into()
                    ],
                    "memset_call"
                );
            }
            
            // 適切な型にキャスト
            let typed_ptr = self.builder.build_bitcast(
                void_ptr,
                elem_type.ptr_type(AddressSpace::Generic),
                "typed_array"
            );
            
            Ok(typed_ptr)
        } else {
            Err(CompilerError::code_generation_error(
                "メモリ割り当て関数が見つかりません",
                None
            ))
        }
    }
    
    /// 並行実行の非同期関数呼び出し
    fn generate_async_call(&mut self, callee: &Expression, args: &[Expression]) -> Result<BasicValueEnum<'ctx>> {
        // 現在の関数を取得
        let current_fn = self.current_function.ok_or_else(|| {
            CompilerError::code_generation_error(
                "関数コンテキスト外での非同期呼び出しは無効です",
                callee.location.clone()
            )
        })?;
        
        // 非同期呼び出し関数がない場合、ランタイムサポートがないためエラー
        if !self.functions.contains_key("async_invoke") {
            return Err(CompilerError::code_generation_error(
                "非同期呼び出しをサポートする'async_invoke'関数が見つかりません",
                callee.location.clone()
            ));
        }
        
        // 呼び出す関数の評価
        let callee_value = self.generate_expression(callee)?;
        
        // 関数ポインタが必要
        let function_ptr = match callee_value {
            BasicValueEnum::PointerValue(ptr) => ptr,
            _ => {
                return Err(CompilerError::code_generation_error(
                    "非同期呼び出しには関数ポインタが必要です",
                    callee.location.clone()
                ));
            }
        };
        
        // 引数の評価
        let mut arg_values = Vec::with_capacity(args.len());
        for arg in args {
            let arg_value = self.generate_expression(arg)?;
            arg_values.push(arg_value);
        }
        
        // 引数配列を作成（void*の配列として扱う）
        let void_ptr_type = self.context.i8_type().ptr_type(AddressSpace::Generic);
        let args_array_type = void_ptr_type.array_type(args.len() as u32);
        let args_array = self.builder.build_alloca(args_array_type, "args_array");
        
        // 引数を配列に格納
        for (i, arg_value) in arg_values.iter().enumerate() {
            // 配列要素へのポインタを取得
            let idx = self.context.i32_type().const_int(i as u64, false);
            let arg_ptr = unsafe {
                self.builder.build_in_bounds_gep(
                    args_array_type,
                    args_array,
                    &[self.context.i32_type().const_zero(), idx],
                    &format!("arg_ptr_{}", i)
                )
            };
            
            // 型に応じたキャスト
            match arg_value {
                BasicValueEnum::IntValue(int_val) => {
                    // 整数値をvoid*にボックス化
                    let int_ptr = self.builder.build_alloca(int_val.get_type(), &format!("int_box_{}", i));
                    self.builder.build_store(int_ptr, int_val);
                    
                    let void_ptr = self.builder.build_bitcast(
                        int_ptr,
                        void_ptr_type,
                        &format!("void_ptr_{}", i)
                    );
                    
                    self.builder.build_store(arg_ptr, void_ptr);
                },
                BasicValueEnum::FloatValue(float_val) => {
                    // 浮動小数点値をvoid*にボックス化
                    let float_ptr = self.builder.build_alloca(float_val.get_type(), &format!("float_box_{}", i));
                    self.builder.build_store(float_ptr, float_val);
                    
                    let void_ptr = self.builder.build_bitcast(
                        float_ptr,
                        void_ptr_type,
                        &format!("void_ptr_{}", i)
                    );
                    
                    self.builder.build_store(arg_ptr, void_ptr);
                },
                BasicValueEnum::PointerValue(ptr_val) => {
                    // ポインタ値をvoid*にキャスト
                    let void_ptr = self.builder.build_bitcast(
                        ptr_val,
                        void_ptr_type,
                        &format!("void_ptr_{}", i)
                    );
                    
                    self.builder.build_store(arg_ptr, void_ptr);
                },
                _ => {
                    return Err(CompilerError::code_generation_error(
                        format!("非同期呼び出しでサポートされていない引数型: {:?}", arg_value),
                        args[i].location.clone()
                    ));
                }
            }
        }
        
        // 引数配列の先頭ポインタを取得
        let args_ptr = self.builder.build_bitcast(
            args_array,
            void_ptr_type,
            "args_ptr"
        );
        
        // 関数ポインタをvoid(*)()型にキャスト
        let void_fn_type = self.context.void_type().fn_type(&[], false);
        let void_fn_ptr = self.builder.build_bitcast(
            function_ptr,
            void_fn_type.ptr_type(AddressSpace::Generic),
            "void_fn_ptr"
        );
        
        // async_invoke関数を呼び出し
        let async_invoke_fn = self.functions.get("async_invoke").unwrap();
        let result = self.builder.build_call(
            async_invoke_fn.get_type(),
            *async_invoke_fn,
            &[
                void_fn_ptr.into(),
                args_ptr.into(),
                self.context.i32_type().const_int(args.len() as u64, false).into()
            ],
            "async_task"
        );
        
        // 非同期タスクハンドルを返す
        let task_handle = result.try_as_basic_value().left().ok_or_else(|| {
            CompilerError::code_generation_error(
                "async_invoke関数の戻り値が無効です",
                callee.location.clone()
            )
        })?;
        
        Ok(task_handle)
    }
    
    /// 非同期タスクの結果の待機
    fn generate_await_expr(&mut self, task_handle: &Expression) -> Result<BasicValueEnum<'ctx>> {
        // 現在の関数を取得
        let current_fn = self.current_function.ok_or_else(|| {
            CompilerError::code_generation_error(
                "関数コンテキスト外でのawait式は無効です",
                task_handle.location.clone()
            )
        })?;
        
        // await関数がない場合、ランタイムサポートがないためエラー
        if !self.functions.contains_key("async_await") {
            return Err(CompilerError::code_generation_error(
                "await式をサポートする'async_await'関数が見つかりません",
                task_handle.location.clone()
            ));
        }
        
        // タスクハンドルの評価
        let handle_value = self.generate_expression(task_handle)?;
        
        // async_await関数を呼び出し
        let async_await_fn = self.functions.get("async_await").unwrap();
        let result = self.builder.build_call(
            async_await_fn.get_type(),
            *async_await_fn,
            &[handle_value.into()],
            "await_result"
        );
        
        // 結果を返す
        let await_result = result.try_as_basic_value().left().ok_or_else(|| {
            CompilerError::code_generation_error(
                "async_await関数の戻り値が無効です",
                task_handle.location.clone()
            )
        })?;
        
        Ok(await_result)
    }
    
    /// 並列処理のためのタスク分割
    fn generate_parallel_for(&mut self, iter_var: &Identifier, start: &Expression, end: &Expression, 
                            step: Option<&Expression>, body: &Statement) -> Result<()> {
        // 現在の関数を取得
        let current_fn = self.current_function.ok_or_else(|| {
            CompilerError::code_generation_error(
                "関数コンテキスト外での並列forループは無効です",
                iter_var.location.clone()
            )
        })?;
        
        // 並列forループ関数がない場合、ランタイムサポートがないためエラー
        if !self.functions.contains_key("parallel_for") {
            return Err(CompilerError::code_generation_error(
                "並列forループをサポートする'parallel_for'関数が見つかりません",
                iter_var.location.clone()
            ));
        }
        
        // 開始値の評価
        let start_value = self.generate_expression(start)?;
        
        // 終了値の評価
        let end_value = self.generate_expression(end)?;
        
        // ステップ値の評価（デフォルトは1）
        let step_value = if let Some(step_expr) = step {
            self.generate_expression(step_expr)?
        } else {
            self.context.i32_type().const_int(1, false).into()
        };
        
        // ループ本体をクロージャとして実装
        // まず一時的な関数を作成
        let parallel_body_name = self.generate_temp_name("parallel_for_body");
        
        // 関数型：void (*)(int32_t)
        let parallel_body_type = self.context.void_type().fn_type(
            &[self.context.i32_type().into()], 
            false
        );
        
        // 関数を作成
        let parallel_body_fn = self.llvm_module.add_function(
            &parallel_body_name, 
            parallel_body_type, 
            None
        );
        
        // 現在の関数環境を保存
        let old_function = self.current_function;
        let old_block = self.current_block;
        let old_variables = std::mem::take(&mut self.variables);
        
        // 関数本体のエントリーブロックを作成
        let entry_block = self.context.append_basic_block(parallel_body_fn, "entry");
        
        // 関数コンテキストを設定
        self.current_function = Some(parallel_body_fn);
        self.current_block = Some(entry_block);
        self.builder.position_at_end(entry_block);
        
        // イテレータ変数をパラメータから設定
        let iter_param = parallel_body_fn.get_nth_param(0).unwrap();
        iter_param.set_name(&iter_var.name);
        
        let iter_ptr = self.builder.build_alloca(self.context.i32_type(), &iter_var.name);
        self.builder.build_store(iter_ptr, iter_param);
        self.variables.insert(iter_var.name.clone(), iter_ptr);
        
        // ループ本体を生成
        self.generate_statement(body)?;
        
        // 関数の最後にreturnを追加
        self.builder.build_return(None);
        
        // 元の関数コンテキストを復元
        self.current_function = old_function;
        self.current_block = old_block;
        self.variables = old_variables;
        
        // parallel_for関数を呼び出し
        let parallel_for_fn = self.functions.get("parallel_for").unwrap();
        
        // 関数ポインタを取得
        let function_ptr = parallel_body_fn.as_global_value().as_pointer_value();
        
        self.builder.build_call(
            parallel_for_fn.get_type(),
            *parallel_for_fn,
            &[
                start_value.into_int_value().into(),
                end_value.into_int_value().into(),
                step_value.into_int_value().into(),
                function_ptr.into()
            ],
            "parallel_for_call"
        );
        
        Ok(())
    }
    
    /// 原子操作の生成（アトミックな加算、減算、比較交換など）
    fn generate_atomic_operation(&mut self, op: AtomicOp, ptr: PointerValue<'ctx>, 
                                value: BasicValueEnum<'ctx>, ordering: AtomicOrdering) -> Result<BasicValueEnum<'ctx>> {
        match op {
            AtomicOp::Load => {
                let atomic_load = self.builder.build_atomic_load(
                    ptr,
                    &format!("atomic_load_{:?}", ordering),
                    convert_atomic_ordering(ordering)
                );
                Ok(atomic_load)
            },
            
            AtomicOp::Store => {
                self.builder.build_atomic_store(
                    ptr,
                    value,
                    convert_atomic_ordering(ordering)
                );
                // ストア操作は値を返さない
                Ok(self.context.i32_type().const_zero().into())
            },
            
            AtomicOp::Add => {
                let int_ptr = ptr.into_pointer_value();
                let int_val = value.into_int_value();
                
                let result = self.builder.build_atomicrmw(
                    inkwell::AtomicRMWBinOp::Add,
                    int_ptr,
                    int_val,
                    convert_atomic_ordering(ordering),
                    false
                );
                
                Ok(result.into())
            },
            
            AtomicOp::Sub => {
                let int_ptr = ptr.into_pointer_value();
                let int_val = value.into_int_value();
                
                let result = self.builder.build_atomicrmw(
                    inkwell::AtomicRMWBinOp::Sub,
                    int_ptr,
                    int_val,
                    convert_atomic_ordering(ordering),
                    false
                );
                
                Ok(result.into())
            },
            
            AtomicOp::And => {
                let int_ptr = ptr.into_pointer_value();
                let int_val = value.into_int_value();
                
                let result = self.builder.build_atomicrmw(
                    inkwell::AtomicRMWBinOp::And,
                    int_ptr,
                    int_val,
                    convert_atomic_ordering(ordering),
                    false
                );
                
                Ok(result.into())
            },
            
            AtomicOp::Or => {
                let int_ptr = ptr.into_pointer_value();
                let int_val = value.into_int_value();
                
                let result = self.builder.build_atomicrmw(
                    inkwell::AtomicRMWBinOp::Or,
                    int_ptr,
                    int_val,
                    convert_atomic_ordering(ordering),
                    false
                );
                
                Ok(result.into())
            },
            
            AtomicOp::Xor => {
                let int_ptr = ptr.into_pointer_value();
                let int_val = value.into_int_value();
                
                let result = self.builder.build_atomicrmw(
                    inkwell::AtomicRMWBinOp::Xor,
                    int_ptr,
                    int_val,
                    convert_atomic_ordering(ordering),
                    false
                );
                
                Ok(result.into())
            },
            
            AtomicOp::Exchange => {
                let int_ptr = ptr.into_pointer_value();
                let int_val = value.into_int_value();
                
                let result = self.builder.build_atomicrmw(
                    inkwell::AtomicRMWBinOp::Xchg,
                    int_ptr,
                    int_val,
                    convert_atomic_ordering(ordering),
                    false
                );
                
                Ok(result.into())
            },
        }
    }
    
    /// 比較交換操作（CAS）の生成
    fn generate_compare_exchange(&mut self, ptr: PointerValue<'ctx>, expected: BasicValueEnum<'ctx>, 
                                desired: BasicValueEnum<'ctx>, success_ordering: AtomicOrdering, 
                                failure_ordering: AtomicOrdering) -> Result<BasicValueEnum<'ctx>> {
        let result = self.builder.build_cmpxchg(
            ptr,
            expected.into_int_value(),
            desired.into_int_value(),
            convert_atomic_ordering(success_ordering),
            convert_atomic_ordering(failure_ordering),
            false
        );
        
        // cmpxchgは{i1, iN}型のペアを返す（成功フラグと交換前の値）
        // 成功フラグのみを返す
        let success = self.builder.build_extract_value(
            result.as_any_value_enum().into_struct_value(),
            0,
            "cas_success"
        ).unwrap();
        
        Ok(success.into())
    }
    
    /// スレッド同期用のフェンス命令の生成
    fn generate_fence(&mut self, ordering: AtomicOrdering) -> Result<()> {
        self.builder.build_fence(
            convert_atomic_ordering(ordering),
            None,
            &format!("fence_{:?}", ordering)
        );
        
        Ok(())
    }
    
    /// 同期用のミューテックスロック獲得
    fn generate_mutex_lock(&mut self, mutex_ptr: PointerValue<'ctx>) -> Result<()> {
        // ランタイムのロック関数を使用
        if let Some(&lock_fn) = self.functions.get("mutex_lock") {
            self.builder.build_call(
                lock_fn.get_type(),
                lock_fn,
                &[mutex_ptr.into()],
                "lock_call"
            );
            
            Ok(())
        } else {
            Err(CompilerError::code_generation_error(
                "mutex_lock関数が見つかりません",
                None
            ))
        }
    }
    
    /// 同期用のミューテックスロック解放
    fn generate_mutex_unlock(&mut self, mutex_ptr: PointerValue<'ctx>) -> Result<()> {
        // ランタイムのアンロック関数を使用
        if let Some(&unlock_fn) = self.functions.get("mutex_unlock") {
            self.builder.build_call(
                unlock_fn.get_type(),
                unlock_fn,
                &[mutex_ptr.into()],
                "unlock_call"
            );
            
            Ok(())
        } else {
            Err(CompilerError::code_generation_error(
                "mutex_unlock関数が見つかりません",
                None
            ))
        }
    }
    
    /// プロファイリング用の関数エントリーイベント生成
    fn generate_function_entry_profiling(&mut self, function_name: &str) -> Result<()> {
        // プロファイリングランタイム関数が存在するか確認
        if let Some(profile_entry_fn) = self.llvm_module.get_function("swiftlight_profile_function_entry") {
            // 関数名の文字列定数を作成
            let func_name_str = self.builder.build_global_string_ptr(function_name, "profile_func_name");
            
            // プロファイリング関数を呼び出し
            self.builder.build_call(
                profile_entry_fn,
                &[func_name_str.as_pointer_value().into()],
                "profile_entry_call"
            );
        }
        
        Ok(())
    }
    
    /// プロファイリング用の関数終了イベント生成
    fn generate_function_exit_profiling(&mut self, function_name: &str) -> Result<()> {
        // プロファイリングランタイム関数が存在するか確認
        if let Some(profile_exit_fn) = self.llvm_module.get_function("swiftlight_profile_function_exit") {
            // 関数名の文字列定数を作成
            let func_name_str = self.builder.build_global_string_ptr(function_name, "profile_func_name");
            
            // プロファイリング関数を呼び出し
            self.builder.build_call(
                profile_exit_fn,
                &[func_name_str.as_pointer_value().into()],
                "profile_exit_call"
            );
        }
        
        Ok(())
    }
    
    /// ブロックカウンター（実行回数計測）の生成
    fn generate_block_counter(&mut self, block_name: &str) -> Result<()> {
        // ブロックカウンターランタイム関数が存在するか確認
        if let Some(block_counter_fn) = self.llvm_module.get_function("swiftlight_count_block") {
            // ブロック名の文字列定数を作成
            let block_name_str = self.builder.build_global_string_ptr(block_name, "block_name");
            
            // カウンター関数を呼び出し
            self.builder.build_call(
                block_counter_fn,
                &[block_name_str.as_pointer_value().into()],
                "block_counter_call"
            );
        }
        
        Ok(())
    }
    
    /// メモリアクセスインストルメンテーションの生成
    fn generate_memory_instrumentation(&mut self, ptr: PointerValue<'ctx>, is_write: bool, size: Option<IntValue<'ctx>>) -> Result<()> {
        // メモリアクセスインストルメンテーション関数が存在するか確認
        let fn_name = if is_write {
            "swiftlight_instrument_memory_write"
        } else {
            "swiftlight_instrument_memory_read"
        };
        
        if let Some(mem_instr_fn) = self.llvm_module.get_function(fn_name) {
            // デフォルトサイズは1バイト
            let access_size = size.unwrap_or_else(|| self.context.i32_type().const_int(1, false));
            
            // メモリアクセスインストルメンテーション関数を呼び出し
            self.builder.build_call(
                mem_instr_fn,
                &[
                    ptr.into(),
                    access_size.into()
                ],
                "mem_instr_call"
            );
        }
        
        Ok(())
    }
    
    /// 条件分岐インストルメンテーションの生成
    fn generate_branch_instrumentation(&mut self, condition: IntValue<'ctx>, branch_id: &str) -> Result<()> {
        // 分岐インストルメンテーション関数が存在するか確認
        if let Some(branch_instr_fn) = self.llvm_module.get_function("swiftlight_instrument_branch") {
            // 分岐IDの文字列定数を作成
            let branch_id_str = self.builder.build_global_string_ptr(branch_id, "branch_id");
            
            // 条件値を拡張
            let cond_ext = self.builder.build_int_z_extend(
                condition, 
                self.context.i32_type(), 
                "branch_cond_ext"
            );
            
            // インストルメンテーション関数を呼び出し
            self.builder.build_call(
                branch_instr_fn,
                &[
                    cond_ext.into(),
                    branch_id_str.as_pointer_value().into()
                ],
                "branch_instr_call"
            );
        }
        
        Ok(())
    }
    
    /// インストルメンテーション用ランタイム関数の宣言
    fn declare_instrumentation_functions(&mut self) -> Result<()> {
        // void swiftlight_profile_function_entry(const char* function_name)
        let char_ptr_type = self.context.i8_type().ptr_type(inkwell::AddressSpace::Generic);
        let profile_entry_type = self.context.void_type().fn_type(&[char_ptr_type.into()], false);
        self.llvm_module.add_function("swiftlight_profile_function_entry", profile_entry_type, None);
        
        // void swiftlight_profile_function_exit(const char* function_name)
        let profile_exit_type = self.context.void_type().fn_type(&[char_ptr_type.into()], false);
        self.llvm_module.add_function("swiftlight_profile_function_exit", profile_exit_type, None);
        
        // void swiftlight_count_block(const char* block_name)
        let block_counter_type = self.context.void_type().fn_type(&[char_ptr_type.into()], false);
        self.llvm_module.add_function("swiftlight_count_block", block_counter_type, None);
        
        // void swiftlight_instrument_memory_read(void* ptr, uint32_t size)
        let void_ptr_type = self.context.i8_type().ptr_type(inkwell::AddressSpace::Generic);
        let uint32_type = self.context.i32_type();
        let mem_read_type = self.context.void_type().fn_type(
            &[void_ptr_type.into(), uint32_type.into()], 
            false
        );
        self.llvm_module.add_function("swiftlight_instrument_memory_read", mem_read_type, None);
        
        // void swiftlight_instrument_memory_write(void* ptr, uint32_t size)
        let mem_write_type = self.context.void_type().fn_type(
            &[void_ptr_type.into(), uint32_type.into()], 
            false
        );
        self.llvm_module.add_function("swiftlight_instrument_memory_write", mem_write_type, None);
        
        // void swiftlight_instrument_branch(uint32_t condition, const char* branch_id)
        let branch_type = self.context.void_type().fn_type(
            &[uint32_type.into(), char_ptr_type.into()],
            false
        );
        self.llvm_module.add_function("swiftlight_instrument_branch", branch_type, None);
        
        Ok(())
    }
    
    /// パフォーマンスカウンターの生成
    fn generate_performance_counter(&mut self, counter_name: &str, increment_value: Option<IntValue<'ctx>>) -> Result<()> {
        // パフォーマンスカウンターグローバル変数の取得または作成
        let counter_var_name = format!("__perf_counter_{}", counter_name);
        let counter_var = match self.llvm_module.get_global(&counter_var_name) {
            Some(var) => var,
            None => {
                // カウンター変数が存在しない場合は作成
                let int64_type = self.context.i64_type();
                let counter = self.llvm_module.add_global(int64_type, None, &counter_var_name);
                counter.set_initializer(&int64_type.const_zero());
                counter
            }
        };
        
        // カウンターの現在値をロード
        let counter_ptr = counter_var.as_pointer_value();
        let current_value = self.builder.build_load(
            self.context.i64_type(),
            counter_ptr,
            "counter_value"
        );
        
        // カウンターをインクリメント
        let increment = match increment_value {
            Some(value) => {
                // 32ビット→64ビットに拡張
                self.builder.build_int_z_extend(
                    value,
                    self.context.i64_type(),
                    "counter_incr_ext"
                )
            },
            None => {
                // デフォルトは1
                self.context.i64_type().const_int(1, false)
            }
        };
        
        // 加算
        let new_value = match current_value {
            BasicValueEnum::IntValue(int_val) => {
                self.builder.build_int_add(int_val, increment, "counter_new_value")
            },
            _ => {
                return Err(CompilerError::code_generation_error(
                    "カウンター変数が整数型ではありません",
                    None
                ));
            }
        };
        
        // 新しい値を保存
        self.builder.build_store(counter_ptr, new_value);
        
        Ok(())
    }
    
    /// ホットスポット検出用のコード生成
    fn generate_hotspot_detection(&mut self, hotspot_id: &str) -> Result<()> {
        // ホットスポット検出関数が存在するか確認
        if let Some(hotspot_fn) = self.llvm_module.get_function("swiftlight_detect_hotspot") {
            // ホットスポットIDの文字列定数を作成
            let hotspot_id_str = self.builder.build_global_string_ptr(hotspot_id, "hotspot_id");
            
            // ホットスポット検出関数を呼び出し
            self.builder.build_call(
                hotspot_fn,
                &[hotspot_id_str.as_pointer_value().into()],
                "hotspot_call"
            );
        }
        
        Ok(())
    }
    
    /// ループ分析のためのコード生成
    fn generate_loop_analysis(&mut self, loop_id: &str, iteration_count: Option<IntValue<'ctx>>) -> Result<()> {
        // ループ分析関数が存在するか確認
        if let Some(loop_fn) = self.llvm_module.get_function("swiftlight_analyze_loop") {
            // ループIDの文字列定数を作成
            let loop_id_str = self.builder.build_global_string_ptr(loop_id, "loop_id");
            
            // イテレーション数（指定がなければ0）
            let iter_count = match iteration_count {
                Some(count) => count,
                None => self.context.i32_type().const_zero()
            };
            
            // ループ分析関数を呼び出し
            self.builder.build_call(
                loop_fn,
                &[
                    loop_id_str.as_pointer_value().into(),
                    iter_count.into()
                ],
                "loop_analysis_call"
            );
        }
        
        Ok(())
    }
    
    /// プロファイル情報の出力生成
    fn generate_profile_dump(&mut self) -> Result<()> {
        // プロファイル情報出力関数が存在するか確認
        if let Some(dump_fn) = self.llvm_module.get_function("swiftlight_dump_profile") {
            // 引数なしで関数を呼び出し
            self.builder.build_call(
                dump_fn,
                &[],
                "profile_dump_call"
            );
        }
        
        Ok(())
    }
    
    /// プログラム終了時のプロファイル情報出力を設定
    fn setup_profile_dump_at_exit(&mut self) -> Result<()> {
        // atexit関数の宣言
        let atexit_fn_type = self.context.i32_type().fn_type(
            &[self.context.void_type().fn_type(&[], false).ptr_type(inkwell::AddressSpace::Generic).into()],
            false
        );
        let atexit_fn = self.llvm_module.add_function("atexit", atexit_fn_type, None);
        
        // プロファイル情報出力関数の取得
        if let Some(dump_fn) = self.llvm_module.get_function("swiftlight_dump_profile") {
            // プログラムの初期化関数（main前に実行）
            let profile_init_type = self.context.void_type().fn_type(&[], false);
            let profile_init_fn = self.llvm_module.add_function(
                "swiftlight_profile_init",
                profile_init_type,
                None
            );
            
            // 初期化関数の実装
            let entry_block = self.context.append_basic_block(profile_init_fn, "entry");
            
            // 現在の関数と基本ブロックを保存
            let old_function = self.current_function;
            let old_block = self.current_block;
            
            // 初期化関数に移動
            self.builder.position_at_end(entry_block);
            self.current_function = Some(profile_init_fn);
            self.current_block = Some(entry_block);
            
            // atexit関数を呼び出して、プログラム終了時にプロファイル情報を出力
            self.builder.build_call(
                atexit_fn,
                &[dump_fn.as_global_value().as_pointer_value().into()],
                "atexit_call"
            );
            
            // 初期化関数を終了
            self.builder.build_return(None);
            
            // コンストラクタ属性を追加して、main前に実行されるようにする
            profile_init_fn.add_attribute(
                inkwell::AttributeLoc::Function,
                self.context.create_string_attribute("constructor", "")
            );
            
            // 元の関数に戻る
            self.current_function = old_function;
            self.current_block = old_block;
            
            if let Some(block) = old_block {
                self.builder.position_at_end(block);
            }
        }
        
        Ok(())
    }
}

/// アトミック操作の種類
enum AtomicOp {
    /// アトミックロード
    Load,
    /// アトミックストア
    Store,
    /// アトミック加算
    Add,
    /// アトミック減算
    Sub,
    /// アトミック論理積
    And,
    /// アトミック論理和
    Or,
    /// アトミック排他的論理和
    Xor,
    /// アトミック交換
    Exchange,
}

/// アトミック操作のメモリオーダリング
enum AtomicOrdering {
    /// 順序なし（最も弱い）
    Unordered,
    /// モノトニック（Monotonic）
    Monotonic,
    /// アクワイア
    Acquire,
    /// リリース
    Release,
    /// アクワイア・リリース
    AcquireRelease,
    /// シーケンシャリー・コンシステント（最も強い）
    SequentiallyConsistent,
}

/// AtomicOrderingからinkwellのAtomicOrderingへの変換
fn convert_atomic_ordering(ordering: AtomicOrdering) -> inkwell::AtomicOrdering {
    match ordering {
        AtomicOrdering::Unordered => inkwell::AtomicOrdering::Unordered,
        AtomicOrdering::Monotonic => inkwell::AtomicOrdering::Monotonic,
        AtomicOrdering::Acquire => inkwell::AtomicOrdering::Acquire,
        AtomicOrdering::Release => inkwell::AtomicOrdering::Release,
        AtomicOrdering::AcquireRelease => inkwell::AtomicOrdering::AcquireRelease,
        AtomicOrdering::SequentiallyConsistent => inkwell::AtomicOrdering::SequentiallyConsistent,
    }
}

/// インライン展開戦略
enum InlineStrategy {
    /// 常にインライン展開する
    Always,
    /// インライン展開のヒントを提供する
    Hint,
    /// インライン展開しない
    Never,
}

/// コンパイル時定数値の評価
/// 定数式を評価して、その結果を定数値として返します
fn evaluate_constexpr(&mut self, expr: &Expression) -> Result<BasicValueEnum<'ctx>> {
    match &expr.kind {
        // リテラルは直接評価可能
        ExpressionKind::Literal(lit) => self.generate_literal(lit),
        
        // 二項演算
        ExpressionKind::BinaryOp(op, left, right) => {
            // 左右の式を定数評価
            let left_val = self.evaluate_constexpr(left)?;
            let right_val = self.evaluate_constexpr(right)?;
            
            match op {
                // 整数演算
                BinaryOperator::Add => match (left_val, right_val) {
                    (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                        Ok(self.builder.build_int_add(l, r, "const_add").into())
                    },
                    (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
                        Ok(self.builder.build_float_add(l, r, "const_add").into())
                    },
                    _ => Err(CompilerError::code_generation_error(
                        "不正な定数式: 加算は数値型でのみ有効です",
                        expr.location.clone()
                    ))
                },
                BinaryOperator::Subtract => match (left_val, right_val) {
                    (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                        Ok(self.builder.build_int_sub(l, r, "const_sub").into())
                    },
                    (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
                        Ok(self.builder.build_float_sub(l, r, "const_sub").into())
                    },
                    _ => Err(CompilerError::code_generation_error(
                        "不正な定数式: 減算は数値型でのみ有効です",
                        expr.location.clone()
                    ))
                },
                BinaryOperator::Multiply => match (left_val, right_val) {
                    (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                        Ok(self.builder.build_int_mul(l, r, "const_mul").into())
                    },
                    (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
                        Ok(self.builder.build_float_mul(l, r, "const_mul").into())
                    },
                    _ => Err(CompilerError::code_generation_error(
                        "不正な定数式: 乗算は数値型でのみ有効です",
                        expr.location.clone()
                    ))
                },
                BinaryOperator::Divide => match (left_val, right_val) {
                    (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                        // 0による除算チェック
                        if r.is_const_zero() {
                            return Err(CompilerError::code_generation_error(
                                "コンパイル時エラー: ゼロによる除算",
                                expr.location.clone()
                            ));
                        }
                        
                        // 符号付き除算
                        Ok(self.builder.build_int_signed_div(l, r, "const_div").into())
                    },
                    (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
                        Ok(self.builder.build_float_div(l, r, "const_div").into())
                    },
                    _ => Err(CompilerError::code_generation_error(
                        "不正な定数式: 除算は数値型でのみ有効です",
                        expr.location.clone()
                    ))
                },
                
                // 論理演算
                BinaryOperator::And => match (left_val, right_val) {
                    (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                        Ok(self.builder.build_and(l, r, "const_and").into())
                    },
                    _ => Err(CompilerError::code_generation_error(
                        "不正な定数式: 論理積は整数型でのみ有効です",
                        expr.location.clone()
                    ))
                },
                BinaryOperator::Or => match (left_val, right_val) {
                    (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                        Ok(self.builder.build_or(l, r, "const_or").into())
                    },
                    _ => Err(CompilerError::code_generation_error(
                        "不正な定数式: 論理和は整数型でのみ有効です",
                        expr.location.clone()
                    ))
                },
                
                // 比較演算
                BinaryOperator::Equal => match (left_val, right_val) {
                    (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                        let cmp = self.builder.build_int_compare(
                            inkwell::IntPredicate::EQ, l, r, "const_eq"
                        );
                        Ok(self.builder.build_int_z_extend(
                            cmp, 
                            self.context.i32_type(), 
                            "const_eq_ext"
                        ).into())
                    },
                    (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
                        let cmp = self.builder.build_float_compare(
                            inkwell::FloatPredicate::OEQ, l, r, "const_eq"
                        );
                        Ok(self.builder.build_int_z_extend(
                            cmp, 
                            self.context.i32_type(), 
                            "const_eq_ext"
                        ).into())
                    },
                    _ => Err(CompilerError::code_generation_error(
                        "不正な定数式: 等価比較は同じ型でのみ有効です",
                        expr.location.clone()
                    ))
                },
                BinaryOperator::NotEqual => match (left_val, right_val) {
                    (BasicValueEnum::IntValue(l), BasicValueEnum::IntValue(r)) => {
                        let cmp = self.builder.build_int_compare(
                            inkwell::IntPredicate::NE, l, r, "const_ne"
                        );
                        Ok(self.builder.build_int_z_extend(
                            cmp, 
                            self.context.i32_type(), 
                            "const_ne_ext"
                        ).into())
                    },
                    (BasicValueEnum::FloatValue(l), BasicValueEnum::FloatValue(r)) => {
                        let cmp = self.builder.build_float_compare(
                            inkwell::FloatPredicate::ONE, l, r, "const_ne"
                        );
                        Ok(self.builder.build_int_z_extend(
                            cmp, 
                            self.context.i32_type(), 
                            "const_ne_ext"
                        ).into())
                    },
                    _ => Err(CompilerError::code_generation_error(
                        "不正な定数式: 不等価比較は同じ型でのみ有効です",
                        expr.location.clone()
                    ))
                },
                
                // その他の演算子は省略（必要に応じて追加）
                _ => Err(CompilerError::code_generation_error(
                    format!("コンパイル時評価でサポートされていない演算子: {:?}", op),
                    expr.location.clone()
                ))
            }
        },
        
        // 単項演算
        ExpressionKind::UnaryOp(op, operand) => {
            // オペランドを定数評価
            let operand_val = self.evaluate_constexpr(operand)?;
            
            match op {
                UnaryOperator::Minus => match operand_val {
                    BasicValueEnum::IntValue(val) => {
                        Ok(self.builder.build_int_neg(val, "const_neg").into())
                    },
                    BasicValueEnum::FloatValue(val) => {
                        Ok(self.builder.build_float_neg(val, "const_neg").into())
                    },
                    _ => Err(CompilerError::code_generation_error(
                        "不正な定数式: 単項マイナスは数値型でのみ有効です",
                        expr.location.clone()
                    ))
                },
                UnaryOperator::Not => match operand_val {
                    BasicValueEnum::IntValue(val) => {
                        Ok(self.builder.build_not(val, "const_not").into())
                    },
                    _ => Err(CompilerError::code_generation_error(
                        "不正な定数式: 論理否定は整数型でのみ有効です",
                        expr.location.clone()
                    ))
                },
                
                // その他の単項演算子は省略
                _ => Err(CompilerError::code_generation_error(
                    format!("コンパイル時評価でサポートされていない単項演算子: {:?}", op),
                    expr.location.clone()
                ))
            }
        },
        
        // 定数参照
        ExpressionKind::Identifier(ident) => {
            // 定数テーブルから値を検索
            if let Some(const_value) = self.lookup_constant_value(&ident.name) {
                Ok(const_value)
            } else {
                Err(CompilerError::code_generation_error(
                    format!("コンパイル時評価: 定数 '{}' が見つかりません", ident.name),
                    expr.location.clone()
                ))
            }
        },
        
        // 条件式（三項演算子）
        ExpressionKind::IfExpr(cond, then_branch, else_branch) => {
            // 条件を評価
            let cond_val = self.evaluate_constexpr(cond)?;
            
            // 条件値をブール値として解釈
            if let BasicValueEnum::IntValue(int_val) = cond_val {
                let is_true = !int_val.is_const_zero();
                
                // 条件に応じてthenまたはelse部分を評価
                if is_true {
                    self.evaluate_constexpr(then_branch)
                } else if let Some(else_expr) = else_branch {
                    self.evaluate_constexpr(else_expr)
                } else {
                    // else部分がない場合はユニット値（void）を返す
                    Err(CompilerError::code_generation_error(
                        "コンパイル時評価: else部分のない条件式を評価できません",
                        expr.location.clone()
                    ))
                }
            } else {
                Err(CompilerError::code_generation_error(
                    "コンパイル時評価: 条件式の条件部は整数型である必要があります",
                    cond.location.clone()
                ))
            }
        },
        
        // キャスト式
        ExpressionKind::Cast(sub_expr, target_type) => {
            // サブ式を評価
            let val = self.evaluate_constexpr(sub_expr)?;
            
            // 型アノテーションを解決
            let target_ir_type = if let Some(type_info) = self.type_info.get_node_type(target_type.id) {
                self.convert_type_from_annotation(type_info)?
            } else {
                return Err(CompilerError::code_generation_error(
                    "コンパイル時評価: キャスト先の型情報がありません",
                    target_type.location.clone()
                ));
            };
            
            // キャスト演算を実行
            match (val, target_ir_type) {
                // 整数→整数キャスト
                (BasicValueEnum::IntValue(int_val), BasicTypeEnum::IntType(target_int_type)) => {
                    let source_bits = int_val.get_type().get_bit_width();
                    let target_bits = target_int_type.get_bit_width();
                    
                    if source_bits == target_bits {
                        Ok(int_val.into())
                    } else if source_bits < target_bits {
                        // 拡張
                        Ok(self.builder.build_int_z_extend(
                            int_val, target_int_type, "const_int_ext"
                        ).into())
                    } else {
                        // 縮小
                        Ok(self.builder.build_int_truncate(
                            int_val, target_int_type, "const_int_trunc"
                        ).into())
                    }
                },
                
                // 整数→浮動小数点キャスト
                (BasicValueEnum::IntValue(int_val), BasicTypeEnum::FloatType(target_float_type)) => {
                    Ok(self.builder.build_signed_int_to_float(
                        int_val, target_float_type, "const_int_to_float"
                    ).into())
                },
                
                // 浮動小数点→整数キャスト
                (BasicValueEnum::FloatValue(float_val), BasicTypeEnum::IntType(target_int_type)) => {
                    Ok(self.builder.build_float_to_signed_int(
                        float_val, target_int_type, "const_float_to_int"
                    ).into())
                },
                
                // 浮動小数点→浮動小数点キャスト
                (BasicValueEnum::FloatValue(float_val), BasicTypeEnum::FloatType(target_float_type)) => {
                    match float_val.get_type().get_bit_width().cmp(&target_float_type.get_bit_width()) {
                        std::cmp::Ordering::Less => {
                            // 拡張 (例: float → double)
                            Ok(self.builder.build_float_ext(
                                float_val, target_float_type, "const_float_ext"
                            ).into())
                        },
                        std::cmp::Ordering::Greater => {
                            // 縮小 (例: double → float)
                            Ok(self.builder.build_float_trunc(
                                float_val, target_float_type, "const_float_trunc"
                            ).into())
                        },
                        std::cmp::Ordering::Equal => {
                            // 同じサイズ
                            Ok(float_val.into())
                        }
                    }
                },
                
                // サポートされていないキャスト
                _ => Err(CompilerError::code_generation_error(
                    "コンパイル時評価: サポートされていないキャスト操作です",
                    expr.location.clone()
                ))
            }
        },
        
        // コンパイル時評価でサポートされていない表現
        _ => Err(CompilerError::code_generation_error(
            "このタイプの式はコンパイル時評価でサポートされていません",
            expr.location.clone()
        ))
    }
}

/// 定数テーブルから定数値を検索
fn lookup_constant_value(&self, name: &str) -> Option<BasicValueEnum<'ctx>> {
    // グローバル定数を検索
    if let Some(global) = self.llvm_module.get_global(name) {
        if global.is_constant() {
            if let Some(init) = global.get_initializer() {
                return Some(init);
            }
        }
    }
    None
}

/// グローバル定数の生成（コンパイル時評価を使用）
fn generate_global_constant(&mut self, constant: &ConstantDeclaration, decl: &Declaration) -> Result<()> {
    let const_name = &constant.name.name;
    
    // 定数の型を取得
    let const_type = if let Some(type_ann) = &constant.type_annotation {
        if let Some(type_info) = self.type_info.get_node_type(type_ann.id) {
            self.convert_type_from_annotation(&type_info)?
        } else {
            return Err(CompilerError::code_generation_error(
                format!("定数 '{}' の型情報がありません", const_name),
                constant.name.location.clone()
            ));
        }
    } else {
        // 初期化式から型を推論
        if let Some(type_info) = self.type_info.get_node_type(constant.initializer.id) {
            self.convert_type_from_annotation(&type_info)?
        } else {
            return Err(CompilerError::code_generation_error(
                format!("定数 '{}' の初期化式の型情報がありません", const_name),
                constant.initializer.location.clone()
            ));
        }
    };
    
    // グローバル定数を作成
    let global_const = self.llvm_module.add_global(const_type, None, const_name);
    
    // 定数として設定
    global_const.set_constant(true);
    
    // 初期化式をコンパイル時に評価（可能な場合）
    let init_value = match self.evaluate_constexpr(&constant.initializer) {
        Ok(val) => val,
        Err(_) => {
            // コンパイル時評価に失敗した場合は通常の実行時初期化に戻る
            
            // 初期化式の評価のために一時的な関数とブロックを作成
            let initializer_fn_type = self.context.void_type().fn_type(&[], false);
            let initializer_fn = self.llvm_module.add_function(
                &format!("{}.initializer", const_name),
                initializer_fn_type,
                None
            );
            
            // 現在の関数と基本ブロックを保存
            let old_function = self.current_function;
            let old_block = self.current_block;
            
            // 初期化関数のエントリーブロックを作成
            let entry_block = self.context.append_basic_block(initializer_fn, "entry");
            self.builder.position_at_end(entry_block);
            
            // 現在の関数コンテキストを設定
            self.current_function = Some(initializer_fn);
            self.current_block = Some(entry_block);
            
            // 初期化式を評価
            let runtime_init_value = self.generate_expression(&constant.initializer)?;
            
            // 関数コンテキストを復元
            self.current_function = old_function;
            self.current_block = old_block;
            
            runtime_init_value
        }
    };
    
    // グローバル定数の初期値を設定
    global_const.set_initializer(&init_value);
    
    // 中間表現のモジュールにグローバル定数を追加
    let ir_type = if let Some(type_ann) = &constant.type_annotation {
        if let Some(type_info) = self.type_info.get_node_type(type_ann.id) {
            self.convert_to_ir_type(&type_info)?
        } else {
            return Err(CompilerError::code_generation_error(
                format!("定数 '{}' の型情報がありません", const_name),
                constant.name.location.clone()
            ));
        }
    } else {
        if let Some(type_info) = self.type_info.get_node_type(constant.initializer.id) {
            self.convert_to_ir_type(&type_info)?
        } else {
            return Err(CompilerError::code_generation_error(
                format!("定数 '{}' の初期化式の型情報がありません", const_name),
                constant.initializer.location.clone()
            ));
        }
    };
    
    // 中間表現にグローバル定数を追加
    self.module.add_global_variable(const_name.clone(), ir_type, true);
    
    Ok(())
}

//! # LLVM コード生成器
//! 
//! SwiftLight中間表現からLLVM IRを生成するためのコード生成器です。
//! inkwellクレートを使用してLLVMとの連携を実現します。

use std::collections::HashMap;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::builder::Builder;
use inkwell::values::{
    BasicValueEnum, FunctionValue, PointerValue, BasicValue, 
    IntValue, FloatValue, AnyValue
};
use inkwell::types::{
    AnyTypeEnum, BasicTypeEnum, FunctionType, StructType, 
    IntType, FloatType, PointerType
};
use inkwell::basic_block::BasicBlock;
use inkwell::AddressSpace;

use crate::frontend::error::{CompilerError, ErrorKind, Result};
use crate::middleend::ir as swift_ir;
use crate::middleend::ir::{
    BinaryOp, UnaryOp, TypeKind, Value, ValueKind, Type,
    Instruction, InstructionKind, Function, FunctionSignature
};

/// LLVM コード生成器
pub struct CodeGenerator<'ctx> {
    /// LLVM コンテキスト
    context: &'ctx Context,
    /// LLVM モジュール
    module: Option<Module<'ctx>>,
    /// LLVM IRビルダー
    builder: Builder<'ctx>,
    /// 現在の関数
    current_function: Option<FunctionValue<'ctx>>,
    /// 変数マップ（SwiftLight変数ID → LLVM値）
    variables: HashMap<usize, PointerValue<'ctx>>,
    /// 型マップ（SwiftLight型ID → LLVM型）
    types: HashMap<usize, AnyTypeEnum<'ctx>>,
    /// 関数マップ（SwiftLight関数ID → LLVM関数）
    functions: HashMap<usize, FunctionValue<'ctx>>,
    /// ブレークブロックスタック（ループ制御用）
    break_stack: Vec<BasicBlock<'ctx>>,
    /// 継続ブロックスタック（ループ制御用）
    continue_stack: Vec<BasicBlock<'ctx>>,
    /// 型キャッシュ（複合型の構築を最適化）
    type_cache: HashMap<String, AnyTypeEnum<'ctx>>,
    /// メタプログラミングコンテキスト
    meta_context: Option<MetaProgrammingContext<'ctx>>,
    /// 値マップ（SwiftLight値ID → LLVM値）
    values: HashMap<usize, BasicValueEnum<'ctx>>,
}

/// メタプログラミングコンテキスト
struct MetaProgrammingContext<'ctx> {
    /// コンパイル時に評価される値
    compile_time_values: HashMap<usize, BasicValueEnum<'ctx>>,
    /// コンパイル時に構築される型
    compile_time_types: HashMap<usize, AnyTypeEnum<'ctx>>,
    /// AST情報へのアクセス（リフレクション用）
    reflection_data: HashMap<usize, PointerValue<'ctx>>,
}

impl<'ctx> CodeGenerator<'ctx> {
    /// 新しいコード生成器を作成
    pub fn new(context: &'ctx Context) -> Self {
        Self {
            context,
            module: None,
            builder: context.create_builder(),
            current_function: None,
            variables: HashMap::new(),
            types: HashMap::new(),
            functions: HashMap::new(),
            break_stack: Vec::new(),
            continue_stack: Vec::new(),
            type_cache: HashMap::new(),
            meta_context: Some(MetaProgrammingContext {
                compile_time_values: HashMap::new(),
                compile_time_types: HashMap::new(),
                reflection_data: HashMap::new(),
            }),
            values: HashMap::new(),
        }
    }
    
    /// LLVMモジュールを生成
    pub fn generate_module(&mut self, ir_module: &swift_ir::Module) -> Result<Module<'ctx>> {
        // モジュールの初期化
        self.module = Some(self.context.create_module(&ir_module.name));
        let module = self.module.as_ref().unwrap();
        
        // 型の事前登録
        for (id, ty) in &ir_module.types {
            self.register_type(*id, ty)?;
        }
        
        // 関数シグネチャの事前登録
        for (id, func) in &ir_module.functions {
            self.declare_function(*id, func)?;
        }
        
        // グローバル変数の定義
        for (id, global) in &ir_module.globals {
            self.define_global(*id, global)?;
        }
        
        // 関数本体の定義
        for (id, func) in &ir_module.functions {
            self.define_function(*id, func)?;
        }
        
        Ok(module.clone())
    }
    
    /// 型を登録
    fn register_type(&mut self, type_id: usize, ty: &Type) -> Result<AnyTypeEnum<'ctx>> {
        // すでに登録済みの場合はキャッシュから返す
        if let Some(llvm_type) = self.types.get(&type_id) {
            return Ok(llvm_type.clone());
        }
        
        // 型の種類に応じてLLVM型を生成
        let llvm_type = match &ty.kind {
            TypeKind::Void => self.context.void_type().into(),
            TypeKind::Boolean => self.context.bool_type().into(),
            TypeKind::Integer { bits, signed } => {
                let int_type = self.context.custom_width_int_type(*bits as u32);
                int_type.into()
            },
            TypeKind::Float { bits } => {
                match *bits {
                    32 => self.context.f32_type().into(),
                    64 => self.context.f64_type().into(),
                    _ => return Err(CompilerError::new(
                        ErrorKind::CodeGen,
                        format!("サポートされていない浮動小数点数ビット数: {}", bits),
                        None
                    )),
                }
            },
            TypeKind::Pointer { pointee_type_id } => {
                // ポインタの対象となる型を取得（再帰的）
                let pointee_type = match self.types.get(pointee_type_id) {
                    Some(ty) => ty.clone(),
                    None => {
                        let pointee_ty = ir_module.types.get(pointee_type_id)
                            .ok_or_else(|| CompilerError::new(
                                ErrorKind::CodeGen,
                                format!("型ID {}が見つかりません", pointee_type_id),
                                None
                            ))?;
                        self.register_type(*pointee_type_id, pointee_ty)?
                    }
                };
                
                match pointee_type {
                    AnyTypeEnum::ArrayType(arr_ty) => arr_ty.ptr_type(AddressSpace::Generic).into(),
                    AnyTypeEnum::FloatType(float_ty) => float_ty.ptr_type(AddressSpace::Generic).into(),
                    AnyTypeEnum::FunctionType(fn_ty) => fn_ty.ptr_type(AddressSpace::Generic).into(),
                    AnyTypeEnum::IntType(int_ty) => int_ty.ptr_type(AddressSpace::Generic).into(),
                    AnyTypeEnum::PointerType(ptr_ty) => ptr_ty.ptr_type(AddressSpace::Generic).into(),
                    AnyTypeEnum::StructType(struct_ty) => struct_ty.ptr_type(AddressSpace::Generic).into(),
                    AnyTypeEnum::VectorType(vec_ty) => vec_ty.ptr_type(AddressSpace::Generic).into(),
                    AnyTypeEnum::VoidType(_) => {
                        self.context.i8_type().ptr_type(AddressSpace::Generic).into()
                    }
                }
            },
            TypeKind::Array { element_type_id, size } => {
                // 配列要素の型を取得（再帰的）
                let element_type = match self.types.get(element_type_id) {
                    Some(ty) => ty.clone(),
                    None => {
                        let element_ty = ir_module.types.get(element_type_id)
                            .ok_or_else(|| CompilerError::new(
                                ErrorKind::CodeGen,
                                format!("型ID {}が見つかりません", element_type_id),
                                None
                            ))?;
                        self.register_type(*element_type_id, element_ty)?
                    }
                };
                
                match element_type {
                    AnyTypeEnum::ArrayType(arr_ty) => arr_ty.array_type(*size as u32).into(),
                    AnyTypeEnum::FloatType(float_ty) => float_ty.array_type(*size as u32).into(),
                    AnyTypeEnum::IntType(int_ty) => int_ty.array_type(*size as u32).into(),
                    AnyTypeEnum::PointerType(ptr_ty) => ptr_ty.array_type(*size as u32).into(),
                    AnyTypeEnum::StructType(struct_ty) => struct_ty.array_type(*size as u32).into(),
                    AnyTypeEnum::VectorType(vec_ty) => vec_ty.array_type(*size as u32).into(),
                    _ => return Err(CompilerError::new(
                        ErrorKind::CodeGen,
                        format!("サポートされていない配列要素型: {:?}", element_type),
                        None
                    )),
                }
            },
            TypeKind::Struct { name, fields } => {
                // 構造体の場合、まず型を宣言してからフィールドを設定
                let struct_type = self.context.opaque_struct_type(&name);
                
                // 構造体をキャッシュに登録（循環参照対策）
                self.types.insert(type_id, struct_type.into());
                
                // フィールド型を取得
                let field_types: Result<Vec<BasicTypeEnum<'ctx>>> = fields
                    .iter()
                    .map(|(_, field_type_id)| {
                        // フィールドの型を取得（再帰的）
                        let field_type = match self.types.get(field_type_id) {
                            Some(ty) => ty.clone(),
                            None => {
                                let field_ty = ir_module.types.get(field_type_id)
                                    .ok_or_else(|| CompilerError::new(
                                        ErrorKind::CodeGen,
                                        format!("型ID {}が見つかりません", field_type_id),
                                        None
                                    ))?;
                                self.register_type(*field_type_id, field_ty)?
                            }
                        };
                        
                        // 構造体フィールドに使用可能な型に変換
                        match field_type {
                            AnyTypeEnum::ArrayType(arr_ty) => Ok(arr_ty.into()),
                            AnyTypeEnum::FloatType(float_ty) => Ok(float_ty.into()),
                            AnyTypeEnum::IntType(int_ty) => Ok(int_ty.into()),
                            AnyTypeEnum::PointerType(ptr_ty) => Ok(ptr_ty.into()),
                            AnyTypeEnum::StructType(struct_ty) => Ok(struct_ty.into()),
                            AnyTypeEnum::VectorType(vec_ty) => Ok(vec_ty.into()),
                            _ => Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                format!("サポートされていない構造体フィールド型: {:?}", field_type),
                                None
                            )),
                        }
                    })
                    .collect();
                
                // フィールドを設定
                struct_type.set_body(&field_types?, false);
                
                struct_type.into()
            },
            TypeKind::Function { signature_id } => {
                // 関数型の場合、シグネチャから関数型を生成
                let signature = ir_module.signatures.get(signature_id)
                    .ok_or_else(|| CompilerError::new(
                        ErrorKind::CodeGen,
                        format!("関数シグネチャID {}が見つかりません", signature_id),
                        None
                    ))?;
                
                // 戻り値の型を取得
                let return_type = match self.types.get(&signature.return_type_id) {
                    Some(ty) => ty.clone(),
                    None => {
                        let return_ty = ir_module.types.get(&signature.return_type_id)
                            .ok_or_else(|| CompilerError::new(
                                ErrorKind::CodeGen,
                                format!("型ID {}が見つかりません", signature.return_type_id),
                                None
                            ))?;
                        self.register_type(signature.return_type_id, return_ty)?
                    }
                };
                
                // 引数の型を取得
                let param_types: Result<Vec<BasicTypeEnum<'ctx>>> = signature
                    .parameter_type_ids
                    .iter()
                    .map(|param_type_id| {
                        let param_type = match self.types.get(param_type_id) {
                            Some(ty) => ty.clone(),
                            None => {
                                let param_ty = ir_module.types.get(param_type_id)
                                    .ok_or_else(|| CompilerError::new(
                                        ErrorKind::CodeGen,
                                        format!("型ID {}が見つかりません", param_type_id),
                                        None
                                    ))?;
                                self.register_type(*param_type_id, param_ty)?
                            }
                        };
                        
                        // 関数引数に使用可能な型に変換
                        match param_type {
                            AnyTypeEnum::ArrayType(arr_ty) => Ok(arr_ty.into()),
                            AnyTypeEnum::FloatType(float_ty) => Ok(float_ty.into()),
                            AnyTypeEnum::IntType(int_ty) => Ok(int_ty.into()),
                            AnyTypeEnum::PointerType(ptr_ty) => Ok(ptr_ty.into()),
                            AnyTypeEnum::StructType(struct_ty) => Ok(struct_ty.into()),
                            AnyTypeEnum::VectorType(vec_ty) => Ok(vec_ty.into()),
                            _ => Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                format!("サポートされていない関数引数型: {:?}", param_type),
                                None
                            )),
                        }
                    })
                    .collect();
                
                // 戻り値の型を関数型に適した形式に変換
                let fn_return_type = match return_type {
                    AnyTypeEnum::VoidType(_) => self.context.void_type(),
                    AnyTypeEnum::ArrayType(arr_ty) => return Ok(arr_ty.ptr_type(AddressSpace::Generic).into()),
                    AnyTypeEnum::FloatType(float_ty) => return Ok(float_ty.fn_type(&[], false).into()),
                    AnyTypeEnum::IntType(int_ty) => return Ok(int_ty.fn_type(&[], false).into()),
                    AnyTypeEnum::PointerType(ptr_ty) => return Ok(ptr_ty.fn_type(&[], false).into()),
                    AnyTypeEnum::StructType(struct_ty) => return Ok(struct_ty.fn_type(&[], false).into()),
                    AnyTypeEnum::VectorType(vec_ty) => return Ok(vec_ty.fn_type(&[], false).into()),
                    _ => return Err(CompilerError::new(
                        ErrorKind::CodeGen,
                        format!("サポートされていない関数戻り値型: {:?}", return_type),
                        None
                    )),
                };
                
                // 関数型を生成
                fn_return_type.fn_type(&param_types?, signature.is_variadic).into()
            },
            _ => return Err(CompilerError::new(
                ErrorKind::CodeGen,
                format!("サポートされていない型: {:?}", ty.kind),
                None
            )),
        };
        
        // 生成したLLVM型をキャッシュに登録
        self.types.insert(type_id, llvm_type.clone());
        
        Ok(llvm_type)
    }
    
    /// 関数を宣言（シグネチャのみ）
    fn declare_function(&mut self, func_id: usize, func: &Function) -> Result<FunctionValue<'ctx>> {
        let module = self.module.as_ref().unwrap();
        
        // 関数シグネチャを取得
        let signature = &func.signature;
        
        // 戻り値の型を取得
        let return_type = self.get_type(signature.return_type_id)?;
        let return_type = match return_type {
            AnyTypeEnum::VoidType(_) => self.context.void_type().into(),
            AnyTypeEnum::IntType(int_ty) => int_ty.into(),
            AnyTypeEnum::FloatType(float_ty) => float_ty.into(),
            AnyTypeEnum::PointerType(ptr_ty) => ptr_ty.into(),
            AnyTypeEnum::StructType(struct_ty) => struct_ty.into(),
            _ => return Err(CompilerError::new(
                ErrorKind::CodeGen,
                format!("サポートされていない関数戻り値型: {:?}", return_type),
                None
            )),
        };
        
        // 引数の型を取得
        let param_types: Result<Vec<BasicTypeEnum<'ctx>>> = signature
            .parameter_type_ids
            .iter()
            .map(|&param_type_id| {
                let param_type = self.get_type(param_type_id)?;
                match param_type {
                    AnyTypeEnum::IntType(int_ty) => Ok(int_ty.into()),
                    AnyTypeEnum::FloatType(float_ty) => Ok(float_ty.into()),
                    AnyTypeEnum::PointerType(ptr_ty) => Ok(ptr_ty.into()),
                    AnyTypeEnum::StructType(struct_ty) => Ok(struct_ty.into()),
                    _ => Err(CompilerError::new(
                        ErrorKind::CodeGen,
                        format!("サポートされていない関数引数型: {:?}", param_type),
                        None
                    )),
                }
            })
            .collect();
        
        // 関数型を生成
        let fn_type = match return_type {
            BasicTypeEnum::IntType(int_ty) => int_ty.fn_type(&param_types?, signature.is_variadic),
            BasicTypeEnum::FloatType(float_ty) => float_ty.fn_type(&param_types?, signature.is_variadic),
            BasicTypeEnum::PointerType(ptr_ty) => ptr_ty.fn_type(&param_types?, signature.is_variadic),
            BasicTypeEnum::StructType(struct_ty) => struct_ty.fn_type(&param_types?, signature.is_variadic),
            BasicTypeEnum::ArrayType(array_ty) => array_ty.fn_type(&param_types?, signature.is_variadic),
            BasicTypeEnum::VectorType(vec_ty) => vec_ty.fn_type(&param_types?, signature.is_variadic),
        };
        
        // 関数を宣言
        let function = module.add_function(&func.name, fn_type, None);
        
        // 関数をキャッシュに登録
        self.functions.insert(func_id, function);
        
        Ok(function)
    }
    
    /// 型IDからLLVM型を取得
    fn get_type(&self, type_id: usize) -> Result<AnyTypeEnum<'ctx>> {
        match self.types.get(&type_id) {
            Some(ty) => Ok(ty.clone()),
            None => Err(CompilerError::new(
                ErrorKind::CodeGen,
                format!("型ID {}が見つかりません", type_id),
                None
            )),
        }
    }
    
    /// グローバル変数を定義
    fn define_global(&mut self, var_id: usize, global: &swift_ir::Global) -> Result<PointerValue<'ctx>> {
        let module = self.module.as_ref().unwrap();
        
        // 変数の型を取得
        let var_type = self.get_type(global.type_id)?;
        let var_type = match var_type {
            AnyTypeEnum::IntType(int_ty) => int_ty.into(),
            AnyTypeEnum::FloatType(float_ty) => float_ty.into(),
            AnyTypeEnum::PointerType(ptr_ty) => ptr_ty.into(),
            AnyTypeEnum::StructType(struct_ty) => struct_ty.into(),
            AnyTypeEnum::ArrayType(array_ty) => array_ty.into(),
            _ => return Err(CompilerError::new(
                ErrorKind::CodeGen,
                format!("サポートされていないグローバル変数型: {:?}", var_type),
                None
            )),
        };
        
        // グローバル変数を宣言
        let global_var = module.add_global(var_type, None, &global.name);
        
        // 初期値があれば設定
        if let Some(init_value_id) = global.initializer {
            // 初期値を取得
            let module = self.module.as_ref().unwrap();
            
            // 初期値のIDから値を取得
            match self.get_value(init_value_id) {
                Ok(init_value) => {
                    // 初期値の型とグローバル変数の型の互換性を確認
                    let init_type = init_value.get_type();
                    let global_type = global_var.as_pointer_value().get_type().get_element_type();
                    
                    // 型が一致しない場合は必要に応じてキャスト
                    let compatible_value = if init_type != global_type {
                        match (init_type, global_type) {
                            // 整数型間のキャスト
                            (BasicTypeEnum::IntType(from_int), BasicTypeEnum::IntType(to_int)) => {
                                let from_width = from_int.get_bit_width();
                                let to_width = to_int.get_bit_width();
                                
                                let init_int_value = init_value.into_int_value();
                                
                                let casted_value = if from_width < to_width {
                                    // 小さい整数から大きい整数へのキャスト（符号拡張）
                                    self.builder.build_int_s_extend(init_int_value, to_int, &format!("{}_cast", global.name))
                                } else if from_width > to_width {
                                    // 大きい整数から小さい整数へのキャスト（切り捨て）
                                    self.builder.build_int_truncate(init_int_value, to_int, &format!("{}_cast", global.name))
                                } else {
                                    // 同じビット幅の場合はそのまま
                                    init_int_value
                                };
                                
                                global_var.set_initializer(&casted_value);
                            },
                            // 浮動小数点型間のキャスト
                            (BasicTypeEnum::FloatType(from_float), BasicTypeEnum::FloatType(to_float)) => {
                                let init_float_value = init_value.into_float_value();
                                
                                let casted_value = if from_float.get_type_kind() < to_float.get_type_kind() {
                                    // 小さい浮動小数点から大きい浮動小数点へのキャスト
                                    self.builder.build_float_ext(init_float_value, to_float, &format!("{}_cast", global.name))
                                } else if from_float.get_type_kind() > to_float.get_type_kind() {
                                    // 大きい浮動小数点から小さい浮動小数点へのキャスト
                                    self.builder.build_float_trunc(init_float_value, to_float, &format!("{}_cast", global.name))
                                } else {
                                    // 同じ型の場合はそのまま
                                    init_float_value
                                };
                                
                                global_var.set_initializer(&casted_value);
                            },
                            // ポインタ型間のキャスト（ビットキャスト）
                            (BasicTypeEnum::PointerType(from_ptr), BasicTypeEnum::PointerType(to_ptr)) => {
                                let init_ptr_value = init_value.into_pointer_value();
                                
                                // ポインタ型のキャスト（ビットキャスト）
                                let casted_value = self.builder.build_bitcast(
                                    init_ptr_value,
                                    to_ptr,
                                    &format!("{}_cast", global.name)
                                );
                                
                                global_var.set_initializer(&casted_value);
                            },
                            // 構造体型のキャスト（フィールド単位の互換性確認）
                            (BasicTypeEnum::StructType(from_struct), BasicTypeEnum::StructType(to_struct)) => {
                                // 構造体の互換性確認
                                if from_struct.count_fields() == to_struct.count_fields() {
                                    // フィールド数が同じ場合、フィールド単位でキャスト
                                    let struct_value = init_value.into_struct_value();
                                    let mut elements = Vec::new();
                                    
                                    // 各フィールドをキャスト
                                    for i in 0..from_struct.count_fields() {
                                        if let Some(from_field_type) = from_struct.get_field_type_at_index(i) {
                                            if let Some(to_field_type) = to_struct.get_field_type_at_index(i) {
                                                // フィールド値を取得
                                                let field_value = unsafe {
                                                    self.builder.build_extract_value(
                                                        struct_value,
                                                        i as u32,
                                                        &format!("{}_field_{}", global.name, i)
                                                    ).unwrap()
                                                };
                                                
                                                // フィールド型に応じてキャスト
                                                if from_field_type != to_field_type {
                                                    // 型が異なる場合、適切なキャストを適用
                                                    // （ここでは簡略化のため一部のケースのみ扱う）
                                                    elements.push(field_value.as_basic_value_enum());
                                                } else {
                                                    // 型が同じ場合はそのまま
                                                    elements.push(field_value);
                                                }
                                            }
                                        }
                                    }
                                    
                                    // 新しい構造体を構築
                                    if let Some(const_struct) = to_struct.const_named_struct(&elements) {
                                        global_var.set_initializer(&const_struct);
                                    } else {
                                        // 互換性のない構造体の場合はデフォルト値を使用
                                        global_var.set_initializer(&to_struct.const_zero());
                                    }
                                } else {
                                    // フィールド数が異なる場合はデフォルト値を使用
                                    global_var.set_initializer(&to_struct.const_zero());
                                }
                            },
                            // 配列型のキャスト（要素単位の互換性確認）
                            (BasicTypeEnum::ArrayType(from_array), BasicTypeEnum::ArrayType(to_array)) => {
                                // 配列長の確認
                                if from_array.len() == to_array.len() {
                                    let array_value = init_value.into_array_value();
                                    let element_count = from_array.len();
                                    
                                    // 要素型の確認
                                    let from_element_type = from_array.get_element_type();
                                    let to_element_type = to_array.get_element_type();
                                    
                                    // 要素型が同じ場合
                                    if from_element_type == to_element_type {
                                        global_var.set_initializer(&array_value);
                                    } else {
                                        // 要素型が異なる場合は個別にキャスト（複雑なため現在サポート外）
                                        global_var.set_initializer(&to_array.const_zero());
                                    }
                                } else {
                                    // 長さが異なる場合はデフォルト値を使用
                                    global_var.set_initializer(&to_array.const_zero());
                                }
                            },
                            // その他の型の組み合わせ（非互換）
                            _ => {
                                // 互換性のない型の場合はデフォルト値を設定
                                match global_type {
                                    BasicTypeEnum::IntType(int_ty) => {
                                        global_var.set_initializer(&int_ty.const_zero());
                                    },
                                    BasicTypeEnum::FloatType(float_ty) => {
                                        global_var.set_initializer(&float_ty.const_zero());
                                    },
                                    BasicTypeEnum::PointerType(_) => {
                                        global_var.set_initializer(&global_type.into_pointer_type().const_null());
                                    },
                                    BasicTypeEnum::StructType(struct_ty) => {
                                        global_var.set_initializer(&struct_ty.const_zero());
                                    },
                                    BasicTypeEnum::ArrayType(array_ty) => {
                                        global_var.set_initializer(&array_ty.const_zero());
                                    },
                                    _ => {
                                        return Err(CompilerError::new(
                                            ErrorKind::CodeGen,
                                            format!("非互換の初期化型: {} -> {}", init_type, global_type),
                                            None
                                        ));
                                    }
                                }
                            }
                        }
                    } else {
                        // 型が一致する場合はそのまま使用
                        init_value
                    };
                    
                    // グローバル変数に初期値を設定
                    global_var.set_initializer(&compatible_value);
                },
                Err(e) => {
                    return Err(CompilerError::new(
                        ErrorKind::CodeGen,
                        format!("グローバル変数の初期化子の取得に失敗しました: {}", e),
                        None
                    ));
                }
            }
        } else {
            // 初期値がない場合はゼロで初期化
            let default_value = match global_var.as_pointer_value().get_type().get_element_type() {
                BasicTypeEnum::IntType(int_ty) => int_ty.const_zero().into(),
                BasicTypeEnum::FloatType(float_ty) => float_ty.const_zero().into(),
                BasicTypeEnum::PointerType(ptr_ty) => ptr_ty.const_null().into(),
                BasicTypeEnum::StructType(struct_ty) => struct_ty.const_zero().into(),
                BasicTypeEnum::ArrayType(array_ty) => array_ty.const_zero().into(),
                _ => {
                    return Err(CompilerError::new(
                        ErrorKind::CodeGen,
                        format!("サポートされていないグローバル変数型のデフォルト初期化: {:?}", 
                            global_var.as_pointer_value().get_type().get_element_type()),
                        None
                    ));
                }
            };
            global_var.set_initializer(&default_value);
        }
        
        // 変数をキャッシュに登録
        self.variables.insert(var_id, global_var.as_pointer_value());
        
        Ok(global_var.as_pointer_value())
    }
    
    /// 関数本体を定義
    fn define_function(&mut self, func_id: usize, func: &Function) -> Result<()> {
        // 関数が既に宣言されているか確認
        let function = match self.functions.get(&func_id) {
            Some(f) => *f,
            None => self.declare_function(func_id, func)?,
        };
        
        // 外部関数の場合は本体を生成しない
        if func.is_external {
            return Ok(());
        }
        
        // 関数の現在のコンテキストを保存
        let old_function = self.current_function;
        self.current_function = Some(function);
        
        // 関数の最初の基本ブロックを作成
        let entry_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry_block);
        
        // 引数のアロケーション
        for (i, param) in function.get_param_iter().enumerate() {
            let param_name = func.signature.parameter_names.get(i)
                .map(|s| s.as_str())
                .unwrap_or(&format!("param{}", i));
            
            param.set_name(param_name);
            
            // 引数用のアロケーションを作成
            let alloca = self.create_entry_block_alloca(
                function,
                param_name,
                param.get_type(),
            );
            
            // 引数の値をアロケーションに格納
            self.builder.build_store(alloca, param);
            
            // 引数をローカル変数に登録
            let param_id = func.signature.parameter_ids.get(i)
                .ok_or_else(|| CompilerError::new(
                    ErrorKind::CodeGen,
                    format!("引数ID {}が見つかりません", i),
                    None
                ))?;
            
            self.variables.insert(*param_id, alloca);
        }
        
        // 関数本体の命令を生成
        for block_id in &func.blocks {
            let block = func.basic_blocks.get(&block_id)
                .ok_or_else(|| CompilerError::new(
                    ErrorKind::CodeGen,
                    format!("基本ブロックID {}が見つかりません", block_id),
                    None
                ))?;
            
            self.generate_basic_block(*block_id, block)?;
        }
        
        // 関数の終了処理
        if !function.verify(true) {
            function.print_to_stderr();
            return Err(CompilerError::new(
                ErrorKind::CodeGen,
                format!("LLVM関数の検証に失敗しました: {}", func.name),
                None
            ));
        }
        
        // 関数のコンテキストを復元
        self.current_function = old_function;
        
        Ok(())
    }
    
    /// 関数の入口ブロックに変数のアロケーションを作成
    fn create_entry_block_alloca(
        &self,
        function: FunctionValue<'ctx>,
        name: &str,
        ty: BasicTypeEnum<'ctx>,
    ) -> PointerValue<'ctx> {
        let builder = self.context.create_builder();
        let entry = function.get_first_basic_block().unwrap();
        
        match entry.get_first_instruction() {
            Some(inst) => builder.position_before(&inst),
            None => builder.position_at_end(entry),
        }
        
        builder.build_alloca(ty, name)
    }
    
    /// 基本ブロックを生成
    fn generate_basic_block(&mut self, block_id: usize, block: &swift_ir::BasicBlock) -> Result<()> {
        let function = self.current_function.unwrap();
        
        // 基本ブロックを作成
        let llvm_block = self.context.append_basic_block(function, &format!("block{}", block_id));
        self.builder.position_at_end(llvm_block);
        
        // ブロック内の命令を生成
        for inst in &block.instructions {
            self.generate_instruction(inst)?;
        }
        
        Ok(())
    }
    
    /// 命令を生成
    fn generate_instruction(&mut self, inst: &Instruction) -> Result<()> {
        match &inst.kind {
            // 変数定義
            InstructionKind::Alloca { var_id, type_id, name } => {
                // 変数の型を取得
                let var_type = self.get_type(*type_id)?;
                
                let alloca_type = match var_type {
                    AnyTypeEnum::IntType(int_ty) => int_ty.into(),
                    AnyTypeEnum::FloatType(float_ty) => float_ty.into(),
                    AnyTypeEnum::PointerType(ptr_ty) => ptr_ty.into(),
                    AnyTypeEnum::StructType(struct_ty) => struct_ty.into(),
                    AnyTypeEnum::ArrayType(array_ty) => array_ty.into(),
                    AnyTypeEnum::VectorType(vec_ty) => vec_ty.into(),
                    _ => return Err(CompilerError::new(
                        ErrorKind::CodeGen,
                        format!("サポートされていない変数型: {:?}", var_type),
                        None
                    )),
                };
                
                // アロケーションを作成
                let alloca = self.builder.build_alloca(alloca_type, name);
                
                // 変数をキャッシュに登録
                self.variables.insert(*var_id, alloca);
            },
            
            // 変数への代入
            InstructionKind::Store { value_id, var_id } => {
                // 格納する値を取得
                let value = self.get_value(*value_id)?;
                
                // 変数のポインタを取得
                let var_ptr = self.variables.get(var_id)
                    .ok_or_else(|| CompilerError::new(
                        ErrorKind::CodeGen,
                        format!("変数ID {}が見つかりません", var_id),
                        None
                    ))?;
                
                // 値を変数に格納
                self.builder.build_store(*var_ptr, value);
            },
            
            // 変数からの読み込み
            InstructionKind::Load { result_id, var_id, name } => {
                // 変数のポインタを取得
                let var_ptr = self.variables.get(var_id)
                    .ok_or_else(|| CompilerError::new(
                        ErrorKind::CodeGen,
                        format!("変数ID {}が見つかりません", var_id),
                        None
                    ))?;
                
                // 変数から値を読み込む
                let load = self.builder.build_load(*var_ptr, name);
                
                // 結果を値マップに登録
                self.values.insert(*result_id, load);
            },
            
            // 二項演算
            InstructionKind::BinaryOp { result_id, op, lhs_id, rhs_id } => {
                // オペランドを取得
                let lhs = self.get_value(*lhs_id)?;
                let rhs = self.get_value(*rhs_id)?;
                
                // 演算結果を計算
                let result = match op {
                    // 整数演算
                    BinaryOp::Add => {
                        if lhs.is_int_value() {
                            self.builder.build_int_add(
                                lhs.into_int_value(),
                                rhs.into_int_value(),
                                "add"
                            ).into()
                        } else if lhs.is_float_value() {
                            self.builder.build_float_add(
                                lhs.into_float_value(),
                                rhs.into_float_value(),
                                "fadd"
                            ).into()
                        } else {
                            return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "サポートされていない加算演算子の型".to_string(),
                                None
                            ));
                        }
                    },
                    BinaryOp::Sub => {
                        if lhs.is_int_value() {
                            self.builder.build_int_sub(
                                lhs.into_int_value(),
                                rhs.into_int_value(),
                                "sub"
                            ).into()
                        } else if lhs.is_float_value() {
                            self.builder.build_float_sub(
                                lhs.into_float_value(),
                                rhs.into_float_value(),
                                "fsub"
                            ).into()
                        } else {
                            return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "サポートされていない減算演算子の型".to_string(),
                                None
                            ));
                        }
                    },
                    BinaryOp::Mul => {
                        if lhs.is_int_value() {
                            self.builder.build_int_mul(
                                lhs.into_int_value(),
                                rhs.into_int_value(),
                                "mul"
                            ).into()
                        } else if lhs.is_float_value() {
                            self.builder.build_float_mul(
                                lhs.into_float_value(),
                                rhs.into_float_value(),
                                "fmul"
                            ).into()
                        } else {
                            return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "サポートされていない乗算演算子の型".to_string(),
                                None
                            ));
                        }
                    },
                    BinaryOp::Div => {
                        if lhs.is_int_value() {
                            self.builder.build_int_signed_div(
                                lhs.into_int_value(),
                                rhs.into_int_value(),
                                "div"
                            ).into()
                        } else if lhs.is_float_value() {
                            self.builder.build_float_div(
                                lhs.into_float_value(),
                                rhs.into_float_value(),
                                "fdiv"
                            ).into()
                        } else {
                            return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "サポートされていない除算演算子の型".to_string(),
                                None
                            ));
                        }
                    },
                    BinaryOp::Rem => {
                        if lhs.is_int_value() {
                            self.builder.build_int_signed_rem(
                                lhs.into_int_value(),
                                rhs.into_int_value(),
                                "rem"
                            ).into()
                        } else if lhs.is_float_value() {
                            self.builder.build_float_rem(
                                lhs.into_float_value(),
                                rhs.into_float_value(),
                                "frem"
                            ).into()
                        } else {
                            return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "サポートされていない剰余演算子の型".to_string(),
                                None
                            ));
                        }
                    },
                    
                    // 比較演算
                    BinaryOp::Eq => {
                        if lhs.is_int_value() {
                            self.builder.build_int_compare(
                                inkwell::IntPredicate::EQ,
                                lhs.into_int_value(),
                                rhs.into_int_value(),
                                "eq"
                            ).into()
                        } else if lhs.is_float_value() {
                            self.builder.build_float_compare(
                                inkwell::FloatPredicate::OEQ,
                                lhs.into_float_value(),
                                rhs.into_float_value(),
                                "feq"
                            ).into()
                        } else {
                            return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "サポートされていない等価演算子の型".to_string(),
                                None
                            ));
                        }
                    },
                    BinaryOp::Ne => {
                        if lhs.is_int_value() {
                            self.builder.build_int_compare(
                                inkwell::IntPredicate::NE,
                                lhs.into_int_value(),
                                rhs.into_int_value(),
                                "ne"
                            ).into()
                        } else if lhs.is_float_value() {
                            self.builder.build_float_compare(
                                inkwell::FloatPredicate::ONE,
                                lhs.into_float_value(),
                                rhs.into_float_value(),
                                "fne"
                            ).into()
                        } else {
                            return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "サポートされていない非等価演算子の型".to_string(),
                                None
                            ));
                        }
                    },
                    BinaryOp::Lt => {
                        if lhs.is_int_value() {
                            self.builder.build_int_compare(
                                inkwell::IntPredicate::SLT,
                                lhs.into_int_value(),
                                rhs.into_int_value(),
                                "lt"
                            ).into()
                        } else if lhs.is_float_value() {
                            self.builder.build_float_compare(
                                inkwell::FloatPredicate::OLT,
                                lhs.into_float_value(),
                                rhs.into_float_value(),
                                "flt"
                            ).into()
                        } else {
                            return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "サポートされていない小なり演算子の型".to_string(),
                                None
                            ));
                        }
                    },
                    BinaryOp::Le => {
                        if lhs.is_int_value() {
                            self.builder.build_int_compare(
                                inkwell::IntPredicate::SLE,
                                lhs.into_int_value(),
                                rhs.into_int_value(),
                                "le"
                            ).into()
                        } else if lhs.is_float_value() {
                            self.builder.build_float_compare(
                                inkwell::FloatPredicate::OLE,
                                lhs.into_float_value(),
                                rhs.into_float_value(),
                                "fle"
                            ).into()
                        } else {
                            return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "サポートされていない以下演算子の型".to_string(),
                                None
                            ));
                        }
                    },
                    BinaryOp::Gt => {
                        if lhs.is_int_value() {
                            self.builder.build_int_compare(
                                inkwell::IntPredicate::SGT,
                                lhs.into_int_value(),
                                rhs.into_int_value(),
                                "gt"
                            ).into()
                        } else if lhs.is_float_value() {
                            self.builder.build_float_compare(
                                inkwell::FloatPredicate::OGT,
                                lhs.into_float_value(),
                                rhs.into_float_value(),
                                "fgt"
                            ).into()
                        } else {
                            return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "サポートされていない大なり演算子の型".to_string(),
                                None
                            ));
                        }
                    },
                    BinaryOp::Ge => {
                        if lhs.is_int_value() {
                            self.builder.build_int_compare(
                                inkwell::IntPredicate::SGE,
                                lhs.into_int_value(),
                                rhs.into_int_value(),
                                "ge"
                            ).into()
                        } else if lhs.is_float_value() {
                            self.builder.build_float_compare(
                                inkwell::FloatPredicate::OGE,
                                lhs.into_float_value(),
                                rhs.into_float_value(),
                                "fge"
                            ).into()
                        } else {
                            return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "サポートされていない以上演算子の型".to_string(),
                                None
                            ));
                        }
                    },
                    
                    // 論理演算
                    BinaryOp::And => {
                        if lhs.is_int_value() {
                            self.builder.build_and(
                                lhs.into_int_value(),
                                rhs.into_int_value(),
                                "and"
                            ).into()
                        } else {
                            return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "サポートされていない論理積演算子の型".to_string(),
                                None
                            ));
                        }
                    },
                    BinaryOp::Or => {
                        if lhs.is_int_value() {
                            self.builder.build_or(
                                lhs.into_int_value(),
                                rhs.into_int_value(),
                                "or"
                            ).into()
                        } else {
                            return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "サポートされていない論理和演算子の型".to_string(),
                                None
                            ));
                        }
                    },
                    
                    // ビット演算
                    BinaryOp::BitAnd => {
                        if lhs.is_int_value() {
                            self.builder.build_and(
                                lhs.into_int_value(),
                                rhs.into_int_value(),
                                "bitand"
                            ).into()
                        } else {
                            return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "サポートされていないビット積演算子の型".to_string(),
                                None
                            ));
                        }
                    },
                    BinaryOp::BitOr => {
                        if lhs.is_int_value() {
                            self.builder.build_or(
                                lhs.into_int_value(),
                                rhs.into_int_value(),
                                "bitor"
                            ).into()
                        } else {
                            return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "サポートされていないビット和演算子の型".to_string(),
                                None
                            ));
                        }
                    },
                    BinaryOp::BitXor => {
                        if lhs.is_int_value() {
                            self.builder.build_xor(
                                lhs.into_int_value(),
                                rhs.into_int_value(),
                                "bitxor"
                            ).into()
                        } else {
                            return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "サポートされていないビット排他的論理和演算子の型".to_string(),
                                None
                            ));
                        }
                    },
                    BinaryOp::Shl => {
                        if lhs.is_int_value() {
                            self.builder.build_left_shift(
                                lhs.into_int_value(),
                                rhs.into_int_value(),
                                "shl"
                            ).into()
                        } else {
                            return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "サポートされていない左シフト演算子の型".to_string(),
                                None
                            ));
                        }
                    },
                    BinaryOp::Shr => {
                        if lhs.is_int_value() {
                            self.builder.build_right_shift(
                                lhs.into_int_value(),
                                rhs.into_int_value(),
                                true,
                                "shr"
                            ).into()
                        } else {
                            return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "サポートされていない右シフト演算子の型".to_string(),
                                None
                            ));
                        }
                    },
                    
                    // その他の演算子
                    _ => return Err(CompilerError::new(
                        ErrorKind::CodeGen,
                        format!("サポートされていない二項演算子: {:?}", op),
                        None
                    )),
                };
                
                // 結果を値マップに登録
                self.values.insert(*result_id, result);
            },
            
            // 単項演算
            InstructionKind::UnaryOp { result_id, op, operand_id } => {
                // オペランドを取得
                let operand = self.get_value(*operand_id)?;
                
                // 演算結果を計算
                let result = match op {
                    // 数値演算
                    UnaryOp::Neg => {
                        if operand.is_int_value() {
                            let zero = self.context.i64_type().const_zero();
                            self.builder.build_int_sub(
                                zero,
                                operand.into_int_value(),
                                "neg"
                            ).into()
                        } else if operand.is_float_value() {
                            self.builder.build_float_neg(
                                operand.into_float_value(),
                                "fneg"
                            ).into()
                        } else {
                            return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "サポートされていない否定演算子の型".to_string(),
                                None
                            ));
                        }
                    },
                    
                    // 論理演算
                    UnaryOp::Not => {
                        if operand.is_int_value() {
                            self.builder.build_not(
                                operand.into_int_value(),
                                "not"
                            ).into()
                        } else {
                            return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "サポートされていない論理否定演算子の型".to_string(),
                                None
                            ));
                        }
                    },
                    
                    // その他の演算子
                    _ => return Err(CompilerError::new(
                        ErrorKind::CodeGen,
                        format!("サポートされていない単項演算子: {:?}", op),
                        None
                    )),
                };
                
                // 結果を値マップに登録
                self.values.insert(*result_id, result);
            },
            
            // 分岐命令
            InstructionKind::Branch { condition_id, then_block_id, else_block_id } => {
                let function = self.current_function.unwrap();
                
                // 条件を取得
                let condition = self.get_value(*condition_id)?;
                let condition = match condition {
                    BasicValueEnum::IntValue(val) => val,
                    _ => return Err(CompilerError::new(
                        ErrorKind::CodeGen,
                        "分岐条件はブール値または整数値である必要があります".to_string(),
                        None
                    )),
                };
                
                // thenブロックとelseブロックを作成
                let then_block = self.context.append_basic_block(function, "then");
                let else_block = self.context.append_basic_block(function, "else");
                let merge_block = self.context.append_basic_block(function, "merge");
                
                // 条件分岐を構築
                self.builder.build_conditional_branch(condition, then_block, else_block);
                
                // thenブロックに移動
                self.builder.position_at_end(then_block);
                
                // thenブロックの命令を生成
                if let Some(then_block_id) = then_block_id {
                    let then_ir_block = &function.basic_blocks[*then_block_id];
                    
                    // thenブロック内の各命令を生成
                    for inst_id in &then_ir_block.instructions {
                        let inst = &function.instructions[*inst_id];
                        self.gen_instruction(inst, function)?;
                    }
                }
                
                // 明示的なreturnがない場合はマージブロックにジャンプ
                if !self.block_has_terminator(then_block) {
                    self.builder.build_unconditional_branch(merge_block);
                }
                
                // elseブロックに移動
                self.builder.position_at_end(else_block);
                
                // elseブロックの命令を生成
                if let Some(else_block_id) = else_block_id {
                    let else_ir_block = &function.basic_blocks[*else_block_id];
                    
                    // elseブロック内の各命令を生成
                    for inst_id in &else_ir_block.instructions {
                        let inst = &function.instructions[*inst_id];
                        self.gen_instruction(inst, function)?;
                    }
                }
                
                // 明示的なreturnがない場合はマージブロックにジャンプ
                if !self.block_has_terminator(else_block) {
                    self.builder.build_unconditional_branch(merge_block);
                }
                
                // マージブロックに移動
                self.builder.position_at_end(merge_block);
            },
            
            // 無条件分岐命令
            InstructionKind::Jump { target_block_id } => {
                // 対象ブロックへの無条件分岐を生成
                let function = self.current_function.unwrap();
                let target_block = self.context.append_basic_block(function, &format!("block{}", target_block_id));
                self.builder.build_unconditional_branch(target_block);
            },
            
            // 関数呼び出し命令
            InstructionKind::Call { result_id, function_id, args } => {
                // 関数を取得
                let function = self.functions.get(function_id)
                    .ok_or_else(|| CompilerError::new(
                        ErrorKind::CodeGen,
                        format!("関数ID {}が見つかりません", function_id),
                        None
                    ))?;
                
                // 引数を取得
                let mut llvm_args = Vec::new();
                for arg_id in args {
                    let arg = self.get_value(*arg_id)?;
                    llvm_args.push(arg.into());
                }
                
                // 関数を呼び出し
                let call = self.builder.build_call(*function, &llvm_args, "call");
                
                // 戻り値がある場合は結果を値マップに登録
                if let Some(result) = call.try_as_basic_value().left() {
                    self.values.insert(*result_id, result);
                }
            },
            
            // 戻り値命令
            InstructionKind::Return { value_id } => {
                if let Some(value_id) = value_id {
                    // 戻り値を取得
                    let return_value = self.get_value(*value_id)?;
                    
                    // 戻り値を生成
                    self.builder.build_return(Some(&return_value));
                } else {
                    // void戻り値
                    self.builder.build_return(None);
                }
            },
            
            // ループ制御命令
            InstructionKind::LoopControl { kind } => {
                match kind {
                    swift_ir::LoopControlKind::Break => {
                        // 最も内側のループのブレークブロックに分岐
                        if let Some(break_block) = self.break_stack.last() {
                            self.builder.build_unconditional_branch(*break_block);
                        } else {
                            return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "ループ外でのbreak文".to_string(),
                                None
                            ));
                        }
                    },
                    swift_ir::LoopControlKind::Continue => {
                        // 最も内側のループの継続ブロックに分岐
                        if let Some(continue_block) = self.continue_stack.last() {
                            self.builder.build_unconditional_branch(*continue_block);
                        } else {
                            return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "ループ外でのcontinue文".to_string(),
                                None
                            ));
                        }
                    },
                }
            },
            
            // 定数値
            InstructionKind::Constant { result_id, value, type_id } => {
                let ty = self.get_type(*type_id)?;
                
                let constant = match value {
                    swift_ir::ConstantValue::Int(i) => {
                        match ty {
                            AnyTypeEnum::IntType(int_ty) => {
                                int_ty.const_int(*i as u64, true).into()
                            },
                            _ => return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "整数定数に非整数型が指定されました".to_string(),
                                None
                            )),
                        }
                    },
                    swift_ir::ConstantValue::Float(f) => {
                        match ty {
                            AnyTypeEnum::FloatType(float_ty) => {
                                float_ty.const_float(*f).into()
                            },
                            _ => return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "浮動小数点数定数に非浮動小数点型が指定されました".to_string(),
                                None
                            )),
                        }
                    },
                    swift_ir::ConstantValue::Bool(b) => {
                        self.context.bool_type().const_int(*b as u64, false).into()
                    },
                    swift_ir::ConstantValue::Null => {
                        match ty {
                            AnyTypeEnum::PointerType(ptr_ty) => {
                                ptr_ty.const_null().into()
                            },
                            _ => return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "null定数に非ポインタ型が指定されました".to_string(),
                                None
                            )),
                        }
                    },
                    // その他の定数型
                    _ => return Err(CompilerError::new(
                        ErrorKind::CodeGen,
                        format!("サポートされていない定数値: {:?}", value),
                        None
                    )),
                };
                
                // 定数を値マップに登録
                self.values.insert(*result_id, constant);
            },
            
            // 配列要素へのアクセス
            InstructionKind::GetElementPtr { result_id, ptr_id, indices } => {
                // ポインタを取得
                let ptr = self.get_value(*ptr_id)?;
                let ptr_value = match ptr {
                    BasicValueEnum::PointerValue(p) => p,
                    _ => return Err(CompilerError::new(
                        ErrorKind::CodeGen,
                        "GetElementPtrの対象はポインタである必要があります".to_string(),
                        None
                    )),
                };
                
                // インデックスを取得
                let mut llvm_indices = Vec::new();
                for index_id in indices {
                    let index = self.get_value(*index_id)?;
                    match index {
                        BasicValueEnum::IntValue(i) => llvm_indices.push(i),
                        _ => return Err(CompilerError::new(
                            ErrorKind::CodeGen,
                            "GetElementPtrのインデックスは整数である必要があります".to_string(),
                            None
                        )),
                    }
                }
                
                // 要素へのポインタを計算
                let element_ptr = unsafe {
                    self.builder.build_gep(ptr_value, &llvm_indices, "gep")
                };
                
                // ポインタを値マップに登録
                self.values.insert(*result_id, element_ptr.into());
            },
            
            // 構造体フィールドへのアクセス
            InstructionKind::ExtractValue { result_id, struct_id, indices } => {
                // 構造体値を取得
                let struct_val = self.get_value(*struct_id)?;
                
                // インデックスをu32に変換
                let indices: Vec<u32> = indices.iter().map(|&i| i as u32).collect();
                
                // フィールド値を抽出
                let field_value = self.builder.build_extract_value(
                    struct_val,
                    &indices,
                    "extractvalue"
                ).unwrap();
                
                // 結果を値マップに登録
                self.values.insert(*result_id, field_value);
            },
            
            // 構造体フィールドの挿入
            InstructionKind::InsertValue { result_id, struct_id, value_id, indices } => {
                // 構造体値とフィールド値を取得
                let struct_val = self.get_value(*struct_id)?;
                let field_val = self.get_value(*value_id)?;
                
                // インデックスをu32に変換
                let indices: Vec<u32> = indices.iter().map(|&i| i as u32).collect();
                
                // フィールド値を挿入
                let new_struct = self.builder.build_insert_value(
                    struct_val,
                    field_val,
                    &indices,
                    "insertvalue"
                ).unwrap();
                
                // 結果を値マップに登録
                self.values.insert(*result_id, new_struct);
            },
            
            // メモリアロケーション
            InstructionKind::Malloc { result_id, type_id, size_id } => {
                // サイズを取得
                let size = if let Some(size_id) = size_id {
                    self.get_value(*size_id)?
                } else {
                    // デフォルトサイズの場合は1要素とする
                    self.context.i64_type().const_int(1, false).into()
                };
                
                // 型を取得
                let ty = self.get_type(*type_id)?;
                let element_ty = match ty {
                    AnyTypeEnum::PointerType(ptr_ty) => ptr_ty.get_element_type(),
                    _ => return Err(CompilerError::new(
                        ErrorKind::CodeGen,
                        "Mallocの対象はポインタ型である必要があります".to_string(),
                        None
                    )),
                };
                
                // mallocの呼び出しを生成
                let size_in_bytes = match element_ty {
                        BasicTypeEnum::IntType(int_ty) => {
                        let element_size = self.context.i64_type().const_int(
                            (int_ty.get_bit_width() + 7) / 8, false
                        );
                        
                        match size {
                            BasicValueEnum::IntValue(s) => {
                                self.builder.build_int_mul(s, element_size, "malloc_size")
                            },
                            _ => return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "サイズは整数である必要があります".to_string(),
                                None
                            )),
                        }
                        },
                        BasicTypeEnum::FloatType(float_ty) => {
                        let element_size = match float_ty.get_name().to_str().unwrap() {
                            "float" => self.context.i64_type().const_int(4, false),
                            "double" => self.context.i64_type().const_int(8, false),
                            _ => return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "サポートされていない浮動小数点型".to_string(),
                                None
                            )),
                        };
                        
                        match size {
                            BasicValueEnum::IntValue(s) => {
                                self.builder.build_int_mul(s, element_size, "malloc_size")
                            },
                            _ => return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "サイズは整数である必要があります".to_string(),
                                None
                            )),
                        }
                        },
                        BasicTypeEnum::StructType(struct_ty) => {
                        // 構造体のサイズは計算が複雑なため、簡易的に実装
                        let element_size = self.context.i64_type().const_int(
                            8 * struct_ty.count_fields() as u64, false
                        );
                        
                        match size {
                            BasicValueEnum::IntValue(s) => {
                                self.builder.build_int_mul(s, element_size, "malloc_size")
                            },
                            _ => return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "サイズは整数である必要があります".to_string(),
                                None
                            )),
                        }
                        },
                        _ => {
                        // デフォルトのサイズとして8バイトを使用
                        let element_size = self.context.i64_type().const_int(8, false);
                        
                        match size {
                            BasicValueEnum::IntValue(s) => {
                                self.builder.build_int_mul(s, element_size, "malloc_size")
                            },
                            _ => return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "サイズは整数である必要があります".to_string(),
                                None
                            )),
                        }
                    },
                };
                
                // mallocを呼び出す
                let malloc_type = self.context.i8_type().ptr_type(AddressSpace::Generic);
                let malloc_fn_type = self.context.i64_type().fn_type(&[self.context.i64_type().into()], false);
                let module = self.module.as_ref().unwrap();
                
                let malloc_fn = match module.get_function("malloc") {
                    Some(f) => f,
                    None => module.add_function("malloc", malloc_fn_type, None),
                };
                
                let malloc_result = self.builder.build_call(
                    malloc_fn,
                    &[size_in_bytes.into()],
                    "malloc_call"
                );
                
                let raw_ptr = malloc_result.try_as_basic_value().left().unwrap();
                
                // ポインタを適切な型にキャスト
                let result_ptr = self.builder.build_pointer_cast(
                    raw_ptr.into_pointer_value(),
                    match ty {
                        AnyTypeEnum::PointerType(p) => p,
                        _ => unreachable!(),
                    },
                    "malloc_cast"
                );
                
                // 結果を値マップに登録
                self.values.insert(*result_id, result_ptr.into());
            },
            
            // メモリ解放
            InstructionKind::Free { ptr_id } => {
                // ポインタを取得
                let ptr = self.get_value(*ptr_id)?;
                let ptr_value = match ptr {
                    BasicValueEnum::PointerValue(p) => p,
                    _ => return Err(CompilerError::new(
                        ErrorKind::CodeGen,
                        "Freeの対象はポインタである必要があります".to_string(),
                        None
                    )),
                };
                
                // i8*にキャスト
                let i8_ptr = self.builder.build_pointer_cast(
                    ptr_value,
                    self.context.i8_type().ptr_type(AddressSpace::Generic),
                    "free_cast"
                );
                
                // freeを呼び出す
                let free_fn_type = self.context.void_type().fn_type(
                    &[self.context.i8_type().ptr_type(AddressSpace::Generic).into()],
                    false
                );
                let module = self.module.as_ref().unwrap();
                
                let free_fn = match module.get_function("free") {
                    Some(f) => f,
                    None => module.add_function("free", free_fn_type, None),
                };
                
                self.builder.build_call(
                    free_fn,
                    &[i8_ptr.into()],
                    "free_call"
                );
            },
            
            // 型キャスト
            InstructionKind::Cast { result_id, value_id, target_type_id } => {
                // 値と型を取得
                let value = self.get_value(*value_id)?;
                let source_type = value.get_type();
                let target_type = self.get_type(*target_type_id)?;
                
                // キャスト結果を生成
                let result = match (source_type, target_type) {
                    // 整数→整数キャスト
                    (BasicTypeEnum::IntType(src_int), AnyTypeEnum::IntType(tgt_int)) => {
                        let src_bits = src_int.get_bit_width();
                        let tgt_bits = tgt_int.get_bit_width();
                        
                        if src_bits < tgt_bits {
                            // 符号付き拡張
                            self.builder.build_int_s_extend(value.into_int_value(), tgt_int, "s_ext").into()
                        } else if src_bits > tgt_bits {
                            // 切り捨て
                            self.builder.build_int_truncate(value.into_int_value(), tgt_int, "trunc").into()
                        } else {
                            // 同じビット幅の場合はそのまま
                            value
                        }
                    },
                    
                    // 浮動小数点→浮動小数点キャスト
                    (BasicTypeEnum::FloatType(src_float), AnyTypeEnum::FloatType(tgt_float)) => {
                        if src_float.get_name().to_str().unwrap() == "float" && 
                           tgt_float.get_name().to_str().unwrap() == "double" {
                            // float→double拡張
                            self.builder.build_float_ext(value.into_float_value(), tgt_float, "f_ext").into()
                        } else if src_float.get_name().to_str().unwrap() == "double" && 
                                  tgt_float.get_name().to_str().unwrap() == "float" {
                            // double→float切り捨て
                            self.builder.build_float_trunc(value.into_float_value(), tgt_float, "f_trunc").into()
                        } else {
                            // 同じ型の場合はそのまま
                            value
                        }
                    },
                    
                    // 整数→浮動小数点キャスト
                    (BasicTypeEnum::IntType(_), AnyTypeEnum::FloatType(tgt_float)) => {
                        // 符号付き整数→浮動小数点変換
                        self.builder.build_signed_int_to_float(
                            value.into_int_value(), tgt_float, "int_to_float"
                        ).into()
                    },
                    
                    // 浮動小数点→整数キャスト
                    (BasicTypeEnum::FloatType(_), AnyTypeEnum::IntType(tgt_int)) => {
                        // 浮動小数点→符号付き整数変換
                        self.builder.build_float_to_signed_int(
                            value.into_float_value(), tgt_int, "float_to_int"
                        ).into()
                    },
                    
                    // ポインタ→ポインタキャスト
                    (BasicTypeEnum::PointerType(_), AnyTypeEnum::PointerType(tgt_ptr)) => {
                        self.builder.build_pointer_cast(
                            value.into_pointer_value(), tgt_ptr, "ptr_cast"
                        ).into()
                    },
                    
                    // 整数→ポインタキャスト
                    (BasicTypeEnum::IntType(src_int), AnyTypeEnum::PointerType(tgt_ptr)) => {
                        self.builder.build_int_to_ptr(
                            value.into_int_value(), tgt_ptr, "int_to_ptr"
                        ).into()
                    },
                    
                    // ポインタ→整数キャスト
                    (BasicTypeEnum::PointerType(_), AnyTypeEnum::IntType(tgt_int)) => {
                        self.builder.build_ptr_to_int(
                            value.into_pointer_value(), tgt_int, "ptr_to_int"
                        ).into()
                    },
                    
                    // サポートされていないキャスト
                    _ => return Err(CompilerError::new(
                        ErrorKind::CodeGen,
                        format!("サポートされていないキャスト: {:?} to {:?}", source_type, target_type),
                        None
                    )),
                };
                
                // 結果を値マップに登録
                self.values.insert(*result_id, result);
            },
            
            // スレッド生成
            InstructionKind::CreateThread { result_id, function_id, args } => {
                // 関数の取得
                let thread_func = self.functions.get(function_id)
                    .ok_or_else(|| CompilerError::new(
                        ErrorKind::CodeGen,
                        format!("関数ID {}が見つかりません", function_id),
                        None
                    ))?;
                
                // 引数パックの構築
                // NOTE: 実際のスレッド生成はランタイムライブラリに依存するため、簡略化して実装
                
                // pthread_createをインポート
                let module = self.module.as_ref().unwrap();
                
                let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::Generic);
                let thread_id_type = self.context.i64_type().ptr_type(AddressSpace::Generic);
                let thread_attr_type = i8_ptr_type;
                let thread_func_type = self.context.i8_type().fn_type(&[i8_ptr_type.into()], false);
                let pthread_create_type = self.context.i32_type().fn_type(
                    &[
                        thread_id_type.into(),
                        thread_attr_type.into(),
                        thread_func_type.ptr_type(AddressSpace::Generic).into(),
                        i8_ptr_type.into()
                    ],
                    false
                );
                
                let pthread_create = match module.get_function("pthread_create") {
                    Some(f) => f,
                    None => module.add_function("pthread_create", pthread_create_type, None),
                };
                
                // スレッドIDのアロケーション
                let thread_id = self.builder.build_alloca(thread_id_type.get_element_type(), "thread_id");
                
                // 引数をvoid*にパック（実際には引数の構造体を作成し、キャストする必要があるが簡略化）
                let null_attr = i8_ptr_type.const_null();
                let func_ptr = self.builder.build_pointer_cast(
                    thread_func.as_global_value().as_pointer_value(),
                    thread_func_type.ptr_type(AddressSpace::Generic),
                    "thread_func"
                );
                let arg_ptr = i8_ptr_type.const_null(); // 簡略化のためnullを使用
                
                // pthread_createの呼び出し
                let create_result = self.builder.build_call(
                    pthread_create,
                    &[
                        thread_id.into(),
                        null_attr.into(),
                        func_ptr.into(),
                        arg_ptr.into()
                    ],
                    "pthread_create_call"
                );
                
                // スレッドIDを結果として保存
                let thread_id_loaded = self.builder.build_load(thread_id, "thread_id_load");
                self.values.insert(*result_id, thread_id_loaded);
            },
            
            // スレッド待機
            InstructionKind::JoinThread { thread_id } => {
                // スレッドIDを取得
                let thread = self.get_value(*thread_id)?;
                let thread_ptr = match thread {
                    BasicValueEnum::PointerValue(p) => p,
                    _ => return Err(CompilerError::new(
                        ErrorKind::CodeGen,
                        "スレッドIDはポインタである必要があります".to_string(),
                        None
                    )),
                };
                
                // pthread_joinをインポート
                let module = self.module.as_ref().unwrap();
                
                let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::Generic);
                let thread_id_type = self.context.i64_type();
                let pthread_join_type = self.context.i32_type().fn_type(
                    &[
                        thread_id_type.into(),
                        i8_ptr_type.ptr_type(AddressSpace::Generic).into()
                    ],
                    false
                );
                
                let pthread_join = match module.get_function("pthread_join") {
                    Some(f) => f,
                    None => module.add_function("pthread_join", pthread_join_type, None),
                };
                
                // 結果用のポインタ（通常はvoid**だが簡略化）
                let null_result_ptr = i8_ptr_type.ptr_type(AddressSpace::Generic).const_null();
                
                // pthread_joinの呼び出し
                self.builder.build_call(
                    pthread_join,
                    &[
                        thread.into_pointer_value().into(),
                        null_result_ptr.into()
                    ],
                    "pthread_join_call"
                );
            },
            
            // メタプログラミング命令
            InstructionKind::CompileTimeEval { result_id, expr_id } => {
                // コンパイル時評価結果の取得
                if let Some(meta_context) = &self.meta_context {
                    if let Some(value) = meta_context.compile_time_values.get(expr_id) {
                        self.values.insert(*result_id, *value);
                    } else {
                            return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                            format!("式ID {}のコンパイル時評価結果が見つかりません", expr_id),
                                None
                            ));
                    }
                } else {
                    return Err(CompilerError::new(
                        ErrorKind::CodeGen,
                        "メタプログラミングコンテキストが初期化されていません".to_string(),
                        None
                    ));
                }
            },
            
            // リフレクション命令
            InstructionKind::Reflection { result_id, target_id, info_kind } => {
                // リフレクション情報の取得
                // NOTE: 実際の実装では型情報や関数情報にアクセスする複雑なコードが必要
                
                // 簡略的な実装として、ダミーのポインタを返す
                let i8_ptr = self.context.i8_type().ptr_type(AddressSpace::Generic);
                let null_ptr = i8_ptr.const_null();
                self.values.insert(*result_id, null_ptr.into());
            },
            
            // アトミック操作
            InstructionKind::AtomicOp { result_id, op, ptr_id, value_id } => {
                // ポインタと値を取得
                let ptr = self.get_value(*ptr_id)?;
                let ptr_value = match ptr {
                    BasicValueEnum::PointerValue(p) => p,
                    _ => return Err(CompilerError::new(
                        ErrorKind::CodeGen,
                        "アトミック操作の対象はポインタである必要があります".to_string(),
                        None
                    )),
                };
                
                let value = if let Some(value_id) = value_id {
                    Some(self.get_value(*value_id)?)
                } else {
                    None
                };
                
                // アトミック操作の実行
                match op {
                    swift_ir::AtomicOp::Load => {
                        let result = self.builder.build_atomic_load(
                            ptr_value,
                            "atomic_load",
                            inkwell::AtomicOrdering::SequentiallyConsistent,
                            inkwell::AtomicSynchronization::CrossThread
                        );
                        self.values.insert(*result_id, result);
                    },
                    swift_ir::AtomicOp::Store => {
                        if let Some(val) = value {
                            self.builder.build_atomic_store(
                                ptr_value,
                                val,
                                inkwell::AtomicOrdering::SequentiallyConsistent,
                                inkwell::AtomicSynchronization::CrossThread
                            );
                        } else {
                            return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "アトミックストア操作には値が必要です".to_string(),
                                None
                            ));
                        }
                    },
                    swift_ir::AtomicOp::Exchange => {
                        if let Some(val) = value {
                            let result = self.builder.build_atomic_xchg(
                                ptr_value,
                                val.into_int_value(),
                                inkwell::AtomicOrdering::SequentiallyConsistent,
                                inkwell::AtomicSynchronization::CrossThread
                            );
                            self.values.insert(*result_id, result.into());
                        } else {
                            return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "アトミック交換操作には値が必要です".to_string(),
                                None
                            ));
                        }
                    },
                    swift_ir::AtomicOp::CompareExchange { expected_id } => {
                        let expected = self.get_value(*expected_id)?;
                        if let Some(val) = value {
                            let result = self.builder.build_atomic_cmpxchg(
                                ptr_value,
                                expected.into_int_value(),
                                val.into_int_value(),
                                inkwell::AtomicOrdering::SequentiallyConsistent,
                                inkwell::AtomicOrdering::SequentiallyConsistent,
                                inkwell::AtomicSynchronization::CrossThread
                            );
                            // 結果は{元の値, 成功したか}のタプル
                            let old_value = self.builder.build_extract_value(result, &[0], "old_value").unwrap();
                            self.values.insert(*result_id, old_value);
                        } else {
                            return Err(CompilerError::new(
                                ErrorKind::CodeGen,
                                "アトミック比較交換操作には値が必要です".to_string(),
                                None
                            ));
                        }
                    },
                    // その他のアトミック操作
                    _ => return Err(CompilerError::new(
                        ErrorKind::CodeGen,
                        format!("サポートされていないアトミック操作: {:?}", op),
                        None
                    )),
                }
            },
            
            // その他の命令
            _ => return Err(CompilerError::new(
                ErrorKind::CodeGen,
                format!("サポートされていない命令: {:?}", inst.kind),
                None
            )),
        }
        
        Ok(())
    }
    
    /// 値IDから値を取得
    fn get_value(&self, value_id: usize) -> Result<BasicValueEnum<'ctx>> {
        match self.values.get(&value_id) {
            Some(value) => Ok(*value),
            None => Err(CompilerError::new(
                ErrorKind::CodeGen,
                format!("値ID {}が見つかりません", value_id),
                None
            )),
        }
    }

    /// シンボル参照を処理
    fn generate_symbol_reference(&mut self, symbol_id: usize, name: &str) -> Result<BasicValueEnum<'ctx>> {
        let module = self.module.as_ref().unwrap();
        
        // 変数の参照
        if let Some(var_ptr) = self.variables.get(&symbol_id) {
            return Ok(self.builder.build_load(*var_ptr, name).map_err(|e| {
                CompilerError::new(
                    ErrorKind::CodegenError,
                    format!("LLVM load error for symbol {}: {}", name, e),
                )
            })?);
        }
        
        // 関数の参照
        if let Some(func) = self.functions.get(&symbol_id) {
            let func_value: BasicValueEnum<'ctx> = func.as_global_value().as_pointer_value().into();
            return Ok(func_value);
        }
        
        // グローバル変数の参照
        if let Some(global) = module.get_global(name) {
            return Ok(self.builder.build_load(global.as_pointer_value(), name).map_err(|e| {
                CompilerError::new(
                    ErrorKind::CodegenError,
                    format!("LLVM load error for global {}: {}", name, e),
                )
            })?);
        }
        
        Err(CompilerError::new(
            ErrorKind::CodegenError,
            format!("Unknown symbol reference: {}", name),
        ))
    }
    
    /// 型キャストを生成
    fn generate_cast(&mut self, value: BasicValueEnum<'ctx>, from_type: &Type, to_type: &Type, name: &str) -> Result<BasicValueEnum<'ctx>> {
        match (from_type.kind, to_type.kind) {
            // 整数→整数キャスト
            (TypeKind::Int(from_bits), TypeKind::Int(to_bits)) => {
                let value = value.into_int_value();
                if from_bits < to_bits {
                    Ok(self.builder.build_int_s_extend(value, self.context.custom_width_int_type(to_bits as u32), name).unwrap().into())
                } else if from_bits > to_bits {
                    Ok(self.builder.build_int_truncate(value, self.context.custom_width_int_type(to_bits as u32), name).unwrap().into())
                } else {
                    Ok(value.into())
                }
            },
            
            // 浮動小数点→浮動小数点キャスト
            (TypeKind::Float(from_kind), TypeKind::Float(to_kind)) => {
                let value = value.into_float_value();
                match (from_kind, to_kind) {
                    (swift_ir::FloatKind::F32, swift_ir::FloatKind::F64) => {
                        Ok(self.builder.build_float_ext(value, self.context.f64_type(), name).unwrap().into())
                    },
                    (swift_ir::FloatKind::F64, swift_ir::FloatKind::F32) => {
                        Ok(self.builder.build_float_trunc(value, self.context.f32_type(), name).unwrap().into())
                    },
                    _ => Ok(value.into()),
                }
            },
            
            // 整数→浮動小数点キャスト
            (TypeKind::Int(_), TypeKind::Float(float_kind)) => {
                let int_value = value.into_int_value();
                let float_type = match float_kind {
                    swift_ir::FloatKind::F32 => self.context.f32_type(),
                    swift_ir::FloatKind::F64 => self.context.f64_type(),
                };
                
                Ok(self.builder.build_signed_int_to_float(int_value, float_type, name).unwrap().into())
            },
            
            // 浮動小数点→整数キャスト
            (TypeKind::Float(_), TypeKind::Int(int_bits)) => {
                let float_value = value.into_float_value();
                let int_type = self.context.custom_width_int_type(int_bits as u32);
                
                Ok(self.builder.build_float_to_signed_int(float_value, int_type, name).unwrap().into())
            },
            
            // ポインタキャスト
            (TypeKind::Pointer(from_pointee_id), TypeKind::Pointer(to_pointee_id)) => {
                let ptr_value = value.into_pointer_value();
                
                // ポインタ型の取得
                let from_pointee_type = self.get_type(from_pointee_id as usize)?;
                let to_pointee_type = self.get_type(to_pointee_id as usize)?;
                
                let to_ptr_type = match to_pointee_type {
                    AnyTypeEnum::ArrayType(t) => t.ptr_type(AddressSpace::default()),
                    AnyTypeEnum::FloatType(t) => t.ptr_type(AddressSpace::default()),
                    AnyTypeEnum::FunctionType(t) => t.ptr_type(AddressSpace::default()),
                    AnyTypeEnum::IntType(t) => t.ptr_type(AddressSpace::default()),
                    AnyTypeEnum::PointerType(_) => self.context.i8_type().ptr_type(AddressSpace::default()),
                    AnyTypeEnum::StructType(t) => t.ptr_type(AddressSpace::default()),
                    AnyTypeEnum::VectorType(t) => t.ptr_type(AddressSpace::default()),
                    AnyTypeEnum::VoidType(t) => t.ptr_type(AddressSpace::default()),
                };
                
                Ok(self.builder.build_pointer_cast(ptr_value, to_ptr_type, name).unwrap().into())
            },
            
            // その他の型変換はエラーとする
            _ => Err(CompilerError::new(
                ErrorKind::CodegenError,
                format!("Unsupported cast: {:?} to {:?}", from_type.kind, to_type.kind),
            )),
        }
    }
    
    /// メタプログラミング命令の処理
    fn generate_meta_instruction(&mut self, inst: &Instruction) -> Result<Option<BasicValueEnum<'ctx>>> {
        match &inst.kind {
            InstructionKind::CompileTimeEval { value_id, expr_id } => {
                // コンパイル時評価の結果を記録
                if let Some(meta_ctx) = &self.meta_context {
                    if let Some(value) = meta_ctx.compile_time_values.get(expr_id) {
                        if let Some(ctx) = &mut self.meta_context {
                            ctx.compile_time_values.insert(*value_id, *value);
                            return Ok(Some(*value));
                        }
                    }
                }
                
                Err(CompilerError::new(
                    ErrorKind::CodegenError,
                    "Failed to evaluate expression at compile time".to_string(),
                ))
            },
            
            InstructionKind::TypeGeneration { type_id, template_id, args } => {
                // 型テンプレートから新しい型を生成
                // 実装は複雑なため省略（実際の実装ではテンプレート引数に基づいて型を構築）
                Ok(None)
            },
            
            InstructionKind::Reflection { target_id, info_kind } => {
                // リフレクション情報へのアクセスを提供
                // SwiftLightの型情報や関数情報をランタイムに利用可能にする
                // 実装は複雑なため省略
                Ok(None)
            },
            
            // その他のメタプログラミング命令
            _ => Ok(None),
        }
    }
    
    /// ブロックが終端命令を持っているかどうかを確認
    fn block_has_terminator(&self, block: BasicBlock) -> bool {
        // ブロックの最後の命令を取得
        if let Some(last_instr) = block.get_last_instruction() {
            // 終端命令かどうかをチェック
            last_instr.is_terminator()
        } else {
            // 命令がない場合は終端命令もない
            false
        }
    }
}

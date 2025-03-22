pub mod representation;

use std::collections::{HashMap, HashSet};
use std::ffi::CString;
use std::fmt::{self, Display};
use std::ptr;

use inkwell::AddressSpace;
use inkwell::basic_block::BasicBlock as LLVMBasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module as LLVMModule;
use inkwell::types::{AnyTypeEnum, BasicTypeEnum, StructType};
use inkwell::values::{
    AnyValueEnum, BasicValueEnum, FunctionValue, PointerValue, IntValue, FloatValue, BasicValue
};
use inkwell::{FloatPredicate, IntPredicate, OptimizationLevel};
use rustc_hash::FxHashMap;

use crate::common::errors::{CompilerError, ErrorKind, Result};
use crate::common::source_location::SourceLocation;
use crate::frontend::ast::*;
use crate::frontend::semantic::{TypeCheckResult, TypeKind};
use crate::utils::logger::{Logger, LogLevel};

// 公開リエクスポート - 他のモジュールからアクセス可能
pub use self::representation::{
    BasicBlock, Function, Global, Module, StructField, Type, Value, FunctionAttribute,
    LinkageType, CallingConv, DebugInfo
};

/// IR ジェネレーター - LLVMコード生成
pub struct IRGenerator<'ctx> {
    context: &'ctx Context,
    llvm_module: LLVMModule<'ctx>,
    builder: Builder<'ctx>,
    type_info: TypeCheckResult,
    module: Module,
    values: FxHashMap<NodeId, Value>,
    blocks: FxHashMap<String, Vec<BasicBlock>>,
    current_function: Option<FunctionValue<'ctx>>,
    current_block: Option<LLVMBasicBlock<'ctx>>,
    variables: FxHashMap<String, PointerValue<'ctx>>,
    functions: FxHashMap<String, FunctionValue<'ctx>>,
    structs: FxHashMap<String, StructType<'ctx>>,
    enums: FxHashMap<String, (StructType<'ctx>, IntValue<'ctx>)>,
    temp_counter: usize,
    errors: Vec<CompilerError>,
    debug_info: bool,
    current_loop_condition: Option<LLVMBasicBlock<'ctx>>,
    current_loop_exit: Option<LLVMBasicBlock<'ctx>>,
    current_exception_handler: Option<(PointerValue<'ctx>, PointerValue<'ctx>, LLVMBasicBlock<'ctx>)>,
    current_type_parameters: FxHashMap<String, TypeAnnotation>,
    generic_instances: FxHashMap<String, FunctionValue<'ctx>>,
    debug_metadata: FxHashMap<String, DebugInfo>,
    gc_roots: Vec<PointerValue<'ctx>>,
    gc_safepoints: Vec<LLVMBasicBlock<'ctx>>,
    type_substitutions: FxHashMap<String, TypeAnnotation>,
    vtable_map: FxHashMap<String, PointerValue<'ctx>>,
    runtime_functions: FxHashMap<&'static str, FunctionValue<'ctx>>,
}

impl<'ctx> IRGenerator<'ctx> {
    pub fn new(type_info: &TypeCheckResult) -> Self {
        let context = Context::create();
        let module = LLVMModule::create("main_module");
        let builder = context.create_builder();
        
        module.set_triple(&CString::new(env!("TARGET")).unwrap());
        module.set_data_layout("e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128");
        
        Self {
            context: &context,
            llvm_module: module,
            builder,
            type_info: type_info.clone(),
            module: Module::new("main"),
            values: FxHashMap::default(),
            blocks: FxHashMap::default(),
            current_function: None,
            current_block: None,
            variables: FxHashMap::default(),
            functions: FxHashMap::default(),
            structs: FxHashMap::default(),
            enums: FxHashMap::default(),
            temp_counter: 0,
            errors: Vec::new(),
            debug_info: cfg!(debug_assertions),
            current_loop_condition: None,
            current_loop_exit: None,
            current_exception_handler: None,
            current_type_parameters: FxHashMap::default(),
            generic_instances: FxHashMap::default(),
            debug_metadata: FxHashMap::default(),
            gc_roots: Vec::new(),
            gc_safepoints: Vec::new(),
            type_substitutions: FxHashMap::default(),
            vtable_map: FxHashMap::default(),
            runtime_functions: FxHashMap::default(),
        }
    }

    fn add_error(&mut self, message: impl Into<String>, location: Option<SourceLocation>) {
        self.errors.push(CompilerError::new(
            ErrorKind::CodeGeneration,
            message,
            location,
        ));
    }

    fn generate_temp_name(&mut self, prefix: &str) -> String {
        let name = format!("{}.{}", prefix, self.temp_counter);
        self.temp_counter += 1;
        name
    }

    pub fn generate_module(&mut self, program: &Program) -> Result<Module> {
        self.initialize_module(program)?;
        
        for decl in &program.declarations {
            match &decl.kind {
                DeclarationKind::FunctionDecl(func) => {
                    self.generate_function(func)?;
                }
                DeclarationKind::StructDecl(struct_def) => {
                    self.generate_struct(struct_def)?;
                }
                DeclarationKind::EnumDecl(enum_def) => {
                    self.generate_enum(enum_def)?;
                }
                DeclarationKind::TraitDecl(trait_def) => {
                    self.generate_trait(trait_def)?;
                }
                DeclarationKind::ImplementationDecl(impl_def) => {
                    self.generate_implementation(impl_def)?;
                }
                DeclarationKind::GlobalVarDecl(var) => {
                    self.generate_global_variable(var)?;
                }
            }
        }
        
        self.generate_program_entry(program)?;
        self.finalize_module()?;
        
        Ok(self.module.clone())
    }

    fn initialize_module(&mut self, program: &Program) -> Result<()> {
        self.module.name = program.source_path.clone();
        self.declare_runtime_functions()?;
        self.predeclare_types(program)?;
        self.predeclare_functions(program)?;
        self.generate_metadata();
        Ok(())
    }

    fn declare_runtime_functions(&mut self) -> Result<()> {
        let void_type = self.context.void_type();
        let i8_type = self.context.i8_type();
        let i8_ptr_type = i8_type.ptr_type(AddressSpace::Generic);
        let i32_type = self.context.i32_type();
        let i64_type = self.context.i64_type();
        
        let runtime_funcs = [
            ("swift_alloc", i8_ptr_type.fn_type(&[i64_type.into()], false)),
            ("swift_dealloc", void_type.fn_type(&[i8_ptr_type.into()], false)),
            ("swift_exception_throw", void_type.fn_type(&[i8_ptr_type.into()], false)),
            ("swift_gc_safepoint", void_type.fn_type(&[], false)),
            ("swift_vtable_lookup", i8_ptr_type.fn_type(&[i8_ptr_type.into(), i32_type.into()], false)),
        ];
        
        for (name, ty) in runtime_funcs {
            let func = self.llvm_module.add_function(name, ty, None);
            func.set_linkage(inkwell::module::Linkage::External);
            self.runtime_functions.insert(name, func);
        }
        
        Ok(())
    }

    fn predeclare_types(&mut self, program: &Program) -> Result<()> {
        for decl in &program.declarations {
            match &decl.kind {
                DeclarationKind::StructDecl(struct_def) => {
                    let name = &struct_def.name;
                    if self.structs.contains_key(name) {
                        self.add_error(format!("Duplicate struct definition: {}", name), decl.location);
                        continue;
                    }
                    
                    let struct_ty = self.context.opaque_struct_type(name);
                    self.structs.insert(name.clone(), struct_ty);
                }
                DeclarationKind::EnumDecl(enum_def) => {
                    let name = &enum_def.name;
                    if self.enums.contains_key(name) {
                        self.add_error(format!("Duplicate enum definition: {}", name), decl.location);
                        continue;
                    }
                    
                    let tag_ty = self.context.i32_type();
                    let struct_ty = self.context.struct_type(&[
                        tag_ty.into(),
                        self.context.i8_type().array_type(0).into()
                    ], false);
                    self.enums.insert(name.clone(), (struct_ty, tag_ty));
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn predeclare_functions(&mut self, program: &Program) -> Result<()> {
        for decl in &program.declarations {
            if let DeclarationKind::FunctionDecl(func) = &decl.kind {
                let mut param_types = Vec::new();
                for param in &func.parameters {
                    let ty = self.convert_type(&param.type_annotation)?;
                    param_types.push(ty.into());
                }
                
                let return_type = self.convert_type(&func.return_type)?.into();
                let fn_type = match self.context.function_type(return_type, &param_types, func.is_variadic) {
                    Some(ty) => ty,
                    None => {
                        self.add_error(format!("Invalid function type for {}", func.name), decl.location);
                        continue;
                    }
                };
                
                let linkage = if func.is_public {
                    LinkageType::External
                } else {
                    LinkageType::Internal
                };
                
                let func_value = self.llvm_module.add_function(&func.name, fn_type, None);
                func_value.set_linkage(linkage.into());
                func_value.set_calling_convention(CallingConv::Swift.into());
                
                if func.is_generic {
                    self.current_type_parameters = func.type_parameters
                        .iter()
                        .map(|tp| (tp.name.clone(), tp.constraint.clone()))
                        .collect();
                }
                
                self.functions.insert(func.name.clone(), func_value);
            }
        }
        Ok(())
    }

    fn generate_program_entry(&mut self, program: &Program) -> Result<FunctionValue<'ctx>> {
        let i32_type = self.context.i32_type();
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::Generic);
        
        let main_type = i32_type.fn_type(
            &[i32_type.into(), i8_ptr_type.ptr_type(AddressSpace::Generic).into()], 
            false
        );
        
        let main_func = self.llvm_module.add_function("main", main_type, None);
        let entry_block = self.context.append_basic_block(main_func, "entry");
        self.builder.position_at_end(entry_block);
        
        // ランタイム初期化
        let init_func = self.runtime_functions["swift_gc_safepoint"];
        self.builder.build_call(init_func, &[], "init");
        
        // ユーザー定義main関数の呼び出し
        if let Some(user_main) = self.functions.get("main") {
            let ret_val = self.builder.build_call(*user_main, &[], "main_result");
            let ret_int = ret_val.try_as_basic_value().left().unwrap().into_int_value();
            self.builder.build_return(Some(&ret_int));
        } else {
            self.builder.build_return(Some(&i32_type.const_int(0, false)));
        }
        
        Ok(main_func)
    }

    fn finalize_module(&mut self) -> Result<()> {
        if let Err(e) = self.llvm_module.verify() {
            return Err(CompilerError::new(
                ErrorKind::CodeGeneration,
                format!("Module verification failed: {}", e),
                None,
            ));
        }
        
        let output_path = format!("{}.ll", self.module.name);
        self.llvm_module.print_to_file(output_path).map_err(|e| {
            CompilerError::new(
                ErrorKind::IOError,
                format!("Failed to write module: {}", e),
                None,
            )
        })?;
        
        Ok(())
    }

    // 型変換ヘルパー
    fn convert_type(&self, type_ann: &TypeAnnotation) -> Result<BasicTypeEnum<'ctx>> {
        match &type_ann.kind {
            TypeKind::Int => Ok(self.context.i64_type().as_basic_type_enum()),
            TypeKind::Float => Ok(self.context.f64_type().as_basic_type_enum()),
            TypeKind::Bool => Ok(self.context.bool_type().as_basic_type_enum()),
            TypeKind::String => Ok(self.context.i8_type().ptr_type(AddressSpace::Generic).as_basic_type_enum()),
            TypeKind::Struct(name) => {
                if let Some(ty) = self.structs.get(name) {
                    Ok(ty.as_basic_type_enum())
                } else {
                    Err(CompilerError::new(
                        ErrorKind::TypeError,
                        format!("Undefined struct type: {}", name),
                        None,
                    ))
                }
            }
            TypeKind::Generic(name) => {
                if let Some(concrete) = self.type_substitutions.get(name) {
                    self.convert_type(concrete)
                } else {
                    Err(CompilerError::new(
                        ErrorKind::TypeError,
                        format!("Unresolved generic type: {}", name),
                        None,
                    ))
                }
            }
            _ => Err(CompilerError::new(
                ErrorKind::Unimplemented,
                format!("Unsupported type: {:?}", type_ann.kind),
                None,
            )),
        }
    }

    // ジェネリック関数のインスタンス化
    fn instantiate_generic_function(
        &mut self,
        generic_func: &FunctionValue<'ctx>,
        type_args: &[TypeAnnotation],
    ) -> Result<FunctionValue<'ctx>> {
        let mut mangled_name = generic_func.get_name().to_str().unwrap().to_string();
        for arg in type_args {
            mangled_name.push_str(&format!("_{}", arg));
        }
        
        if let Some(instance) = self.generic_instances.get(&mangled_name) {
            return Ok(*instance);
        }
        
        let new_func = self.llvm_module.add_function(
            &mangled_name,
            generic_func.get_type(),
            None,
        );
        
        new_func.set_linkage(generic_func.get_linkage());
        new_func.set_calling_convention(generic_func.get_calling_convention());
        
        self.generic_instances.insert(mangled_name, new_func);
        Ok(new_func)
    }

    // メタデータ生成
    fn generate_metadata(&mut self) {
        let producer = CString::new("SwiftLight Compiler").unwrap();
        self.llvm_module.set_metadata("llvm.module.flags", &[
            self.context.i32_type().const_int(2, false).into(), // Major version
            self.context.i32_type().const_int(0, false).into(), // Minor version
            self.context.i32_type().const_int(1, false).into(), // Patch version
        ]);
        self.llvm_module.set_data_layout("e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128");
        self.llvm_module.set_triple(&CString::new(env!("TARGET")).unwrap());
    }
}

//! # 名前解決（Name Resolution）
//!
//! 識別子の参照先を解決するモジュールです。
//! このモジュールはASTの各識別子が参照するシンボルを特定し、
//! シンボルテーブルに記録します。

use std::collections::{HashMap, HashSet};
use crate::frontend::ast::{
    self, Program, Declaration, DeclarationKind, 
    Statement, StatementKind, Expression, ExpressionKind, 
    Identifier, TypeAnnotation, TypeKind, NodeId, Parameter,
    Function, Struct, Enum, EnumVariant, Trait, Implementation,
};
use crate::frontend::error::{Result, CompilerError, SourceLocation};
use super::scope::{ScopeManager, ScopeKind};
use super::symbol_table::{Symbol, SymbolKind, Visibility};

/// 名前解決の結果
#[derive(Debug, Default)]
pub struct NameResolutionResult {
    /// 解決されたノード -> シンボルのマッピング
    pub resolved_nodes: HashMap<NodeId, NodeId>,
    
    /// エラーのリスト
    pub errors: Vec<CompilerError>,
    
    /// 警告のリスト
    pub warnings: Vec<CompilerError>,
}

impl NameResolutionResult {
    /// 新しい名前解決結果を作成
    pub fn new() -> Self {
        Self {
            resolved_nodes: HashMap::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
    
    /// エラーを追加
    pub fn add_error(&mut self, error: CompilerError) {
        self.errors.push(error);
    }
    
    /// 警告を追加
    pub fn add_warning(&mut self, warning: CompilerError) {
        self.warnings.push(warning);
    }
    
    /// 解決された参照を追加
    pub fn add_resolution(&mut self, reference_id: NodeId, symbol_id: NodeId) {
        self.resolved_nodes.insert(reference_id, symbol_id);
    }
    
    /// エラーがあるかどうか
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
    
    /// 警告があるかどうか
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
    
    /// 結果を合併
    pub fn merge(&mut self, other: NameResolutionResult) {
        self.resolved_nodes.extend(other.resolved_nodes);
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }
}

/// 名前解決を行うリゾルバー
pub struct NameResolver {
    /// スコープマネージャー
    scope_manager: ScopeManager,
    
    /// 名前解決結果
    result: NameResolutionResult,
    
    /// 使用済みのシンボル
    used_symbols: HashSet<NodeId>,
    
    /// 現在処理中のプログラム
    program: Option<Program>,
    
    /// シンボルID生成用カウンタ
    next_id: usize,
    
    /// 解決中のシンボル（再帰的な参照チェック用）
    resolving_symbols: HashSet<String>,
    
    /// インポート履歴（循環インポート検出用）
    import_history: HashSet<String>,
    
    /// インポートスタック（循環インポート検出用）
    import_stack: Vec<String>,
    
    /// インポート済みモジュール（循環インポート検出用）
    imported_modules: Vec<String>,
}

impl NameResolver {
    /// 新しい名前リゾルバーを作成
    pub fn new() -> Self {
        Self {
            scope_manager: ScopeManager::new(),
            result: NameResolutionResult::new(),
            used_symbols: HashSet::new(),
            program: None,
            next_id: 1,
            resolving_symbols: HashSet::new(),
            import_history: HashSet::new(),
            import_stack: Vec::new(),
            imported_modules: Vec::new(),
        }
    }
    
    /// プログラムの名前解決を実行
    pub fn resolve_program(&mut self, program: &Program) -> Result<NameResolutionResult> {
        // プログラムを保存
        self.program = Some(program.clone());
        
        // グローバル宣言を登録
        self.register_declarations(&program.declarations)?;
        
        // グローバル宣言を解決
        self.resolve_declarations(&program.declarations)?;
        
        // 未使用シンボルの警告生成
        self.check_unused_symbols();
        
        // 結果を返す
        Ok(std::mem::take(&mut self.result))
    }
    
    /// 宣言を登録（シンボルテーブルに追加）
    fn register_declarations(&mut self, declarations: &[Declaration]) -> Result<()> {
        for declaration in declarations {
            match &declaration.kind {
                DeclarationKind::Variable(var) => {
                    // 変数宣言の場合
                    let name = &var.name.name;
                    
                    // 変数宣言が許可されているかチェック
                    self.scope_manager.check_variable_declaration(
                        name,
                        declaration.location.clone(),
                    )?;
                    
                    // シンボルを作成
                    let symbol = Symbol::variable(
                        name.clone(),
                        declaration.id,
                        var.type_annotation.clone(),
                        declaration.location.clone(),
                        var.is_mutable,
                        self.scope_manager.current_scope_id(),
                        Visibility::Private, // デフォルトでプライベート
                    );
                    
                    // シンボルを追加
                    self.scope_manager.add_symbol(symbol)?;
                },
                DeclarationKind::Constant(constant) => {
                    // 定数宣言の場合
                    let name = &constant.name.name;
                    
                    // 変数宣言が許可されているかチェック
                    self.scope_manager.check_variable_declaration(
                        name,
                        declaration.location.clone(),
                    )?;
                    
                    // シンボルを作成
                    let symbol = Symbol::constant(
                        name.clone(),
                        declaration.id,
                        constant.type_annotation.clone(),
                        declaration.location.clone(),
                        self.scope_manager.current_scope_id(),
                        Visibility::Private, // デフォルトでプライベート
                    );
                    
                    // シンボルを追加
                    self.scope_manager.add_symbol(symbol)?;
                },
                DeclarationKind::Function(function) => {
                    // 関数宣言の場合
                    let name = &function.name.name;
                    
                    // 関数宣言が許可されているかチェック
                    self.scope_manager.check_function_declaration(
                        name,
                        declaration.location.clone(),
                    )?;
                    
                    // 関数の型情報を構築
                    let type_info = self.create_function_type_annotation(function);
                    
                    // シンボルを作成
                    let symbol = Symbol::function(
                        name.clone(),
                        declaration.id,
                        Some(type_info),
                        declaration.location.clone(),
                        self.scope_manager.current_scope_id(),
                        function.visibility.clone().into(),
                    );
                    
                    // シンボルを追加
                    self.scope_manager.add_symbol(symbol)?;
                },
                DeclarationKind::Struct(struct_decl) => {
                    // 構造体宣言の場合
                    let name = &struct_decl.name.name;
                    
                    // 型宣言が許可されているかチェック
                    self.scope_manager.check_type_declaration(
                        name,
                        declaration.location.clone(),
                    )?;
                    
                    // シンボルを作成
                    let symbol = Symbol::type_symbol(
                        name.clone(),
                        SymbolKind::Struct,
                        declaration.id,
                        declaration.location.clone(),
                        self.scope_manager.current_scope_id(),
                        struct_decl.visibility.clone().into(),
                    );
                    
                    // シンボルを追加
                    self.scope_manager.add_symbol(symbol)?;
                },
                DeclarationKind::Enum(enum_decl) => {
                    // 列挙型宣言の場合
                    let name = &enum_decl.name.name;
                    
                    // 型宣言が許可されているかチェック
                    self.scope_manager.check_type_declaration(
                        name,
                        declaration.location.clone(),
                    )?;
                    
                    // シンボルを作成
                    let symbol = Symbol::type_symbol(
                        name.clone(),
                        SymbolKind::Enum,
                        declaration.id,
                        declaration.location.clone(),
                        self.scope_manager.current_scope_id(),
                        enum_decl.visibility.clone().into(),
                    );
                    
                    // シンボルを追加
                    self.scope_manager.add_symbol(symbol)?;
                },
                DeclarationKind::Trait(trait_decl) => {
                    // トレイト宣言の場合
                    let name = &trait_decl.name.name;
                    
                    // 型宣言が許可されているかチェック
                    self.scope_manager.check_type_declaration(
                        name,
                        declaration.location.clone(),
                    )?;
                    
                    // シンボルを作成
                    let symbol = Symbol::type_symbol(
                        name.clone(),
                        SymbolKind::Trait,
                        declaration.id,
                        declaration.location.clone(),
                        self.scope_manager.current_scope_id(),
                        trait_decl.visibility.clone().into(),
                    );
                    
                    // シンボルを追加
                    self.scope_manager.add_symbol(symbol)?;
                },
                DeclarationKind::TypeAlias(alias) => {
                    // 型エイリアス宣言の場合
                    let name = &alias.name.name;
                    
                    // 型宣言が許可されているかチェック
                    self.scope_manager.check_type_declaration(
                        name,
                        declaration.location.clone(),
                    )?;
                    
                    // シンボルを作成
                    let symbol = Symbol::type_symbol(
                        name.clone(),
                        SymbolKind::TypeAlias,
                        declaration.id,
                        declaration.location.clone(),
                        self.scope_manager.current_scope_id(),
                        alias.visibility.clone().into(),
                    );
                    
                    // シンボルを追加
                    self.scope_manager.add_symbol(symbol)?;
                },
                DeclarationKind::Import(import) => {
                    // インポート宣言の解決
                    self.resolve_import(import, declaration.location.clone())?;
                },
                DeclarationKind::Implementation(_) => {
                    // 実装宣言
                    // 実装自体はシンボルテーブルに登録しないが、中の関数は登録する
                    // この処理は別途行う
                },
            }
        }
        
        Ok(())
    }
    
    /// 関数の型情報（シグネチャ）を作成
    fn create_function_type_annotation(&self, function: &Function) -> TypeAnnotation {
        // 引数の型のリストを作成
        let mut parameter_types = Vec::new();
        
        for param in &function.parameters {
            if let Some(type_ann) = &param.type_annotation {
                // 引数に型注釈がある場合はそれを使用
                parameter_types.push(type_ann.clone());
            } else {
                // 型注釈がない場合は不明な型（型推論用）
                parameter_types.push(self.create_unknown_type());
            }
        }
        
        // 戻り値の型
        let return_type = if let Some(ret_type) = &function.return_type {
            // 戻り値の型注釈がある場合はそれを使用
            ret_type.clone()
        } else {
            // 戻り値の型注釈がない場合はunit型
            self.create_unit_type()
        };
        
        // ジェネリック型パラメータのリスト
        let type_parameters = function.type_parameters
            .iter()
            .map(|type_param| {
                // 型パラメータ名をIdentifierからTypeAnnotationに変換
                TypeAnnotation {
                    id: ast::generate_id(),
                    kind: TypeKind::TypeParameter(type_param.name.clone()),
                    location: type_param.location.clone(),
                }
            })
            .collect::<Vec<_>>();
            
        // 関数型を作成
        TypeAnnotation {
            id: ast::generate_id(),
            kind: TypeKind::Function {
                parameters: parameter_types,
                return_type: Box::new(return_type),
                type_parameters,
                is_async: function.is_async,
            },
            location: function.location.clone(),
        }
    }
    
    /// 不明な型（型推論用）を作成
    fn create_unknown_type(&self) -> TypeAnnotation {
        TypeAnnotation {
            id: ast::generate_id(),
            kind: TypeKind::Unknown,
            location: None,
        }
    }
    
    /// unit型を作成
    fn create_unit_type(&self) -> TypeAnnotation {
        TypeAnnotation {
            id: ast::generate_id(),
            kind: TypeKind::Unit,
            location: None,
        }
    }
    
    /// 宣言を解決（参照を解決）
    fn resolve_declarations(&mut self, declarations: &[Declaration]) -> Result<()> {
        for declaration in declarations {
            match &declaration.kind {
                DeclarationKind::Variable(var) => {
                    // 変数宣言の型情報を解決
                    if let Some(type_ann) = &var.type_annotation {
                        self.resolve_type_annotation(type_ann)?;
                    }
                    
                    // 初期化式を解決
                    if let Some(init) = &var.initializer {
                        self.resolve_expression(init)?;
                    }
                },
                DeclarationKind::Constant(constant) => {
                    // 定数宣言の型情報を解決
                    if let Some(type_ann) = &constant.type_annotation {
                        self.resolve_type_annotation(type_ann)?;
                    }
                    
                    // 初期化式を解決（定数は必ず初期化式がある）
                    self.resolve_expression(&constant.initializer)?;
                },
                DeclarationKind::Function(function) => {
                    // 関数のシグネチャを解決
                    self.resolve_function_signature(function)?;
                    
                    // 関数のボディを解決
                    self.enter_function_scope(function, |this| {
                        // パラメータを登録
                        this.register_parameters(&function.parameters)?;
                        
                        // 関数本体を解決
                        this.resolve_statement(&function.body)?;
                        
                        Ok(())
                    })?;
                },
                DeclarationKind::Struct(struct_decl) => {
                    // 構造体を解決
                    self.enter_type_scope(ScopeKind::Struct, |this| {
                        // フィールドを解決
                        for field in &struct_decl.fields {
                            this.resolve_type_annotation(&field.type_annotation)?;
                        }
                        
                        Ok(())
                    })?;
                },
                DeclarationKind::Enum(enum_decl) => {
                    // 列挙型を解決
                    self.enter_type_scope(ScopeKind::Enum, |this| {
                        // バリアントを解決
                        for variant in &enum_decl.variants {
                            // バリアントが関連値を持つ場合、それらの型を解決
                            for field in &variant.fields {
                                this.resolve_type_annotation(&field.type_annotation)?;
                            }
                        }
                        
                        Ok(())
                    })?;
                },
                DeclarationKind::Trait(trait_decl) => {
                    // トレイトを解決
                    self.enter_type_scope(ScopeKind::Trait, |this| {
                        // トレイトのメソッドシグネチャを解決
                        for method in &trait_decl.methods {
                            this.resolve_function_signature(method)?;
                        }
                        
                        Ok(())
                    })?;
                },
                DeclarationKind::TypeAlias(alias) => {
                    // 型エイリアスを解決
                    self.resolve_type_annotation(&alias.target_type)?;
                },
                DeclarationKind::Import(import) => {
                    // インポート宣言の解決
                    self.resolve_import(import, decl.location.clone())?;
                },
                DeclarationKind::Implementation(impl_decl) => {
                    // 実装宣言を解決
                    self.resolve_implementation(impl_decl)?;
                },
            }
        }
        
        Ok(())
    }
    
    /// 関数シグネチャを解決
    fn resolve_function_signature(&mut self, function: &Function) -> Result<()> {
        // 返り値の型を解決
        if let Some(return_type) = &function.return_type {
            self.resolve_type_annotation(return_type)?;
        }
        
        // パラメータの型を解決
        for param in &function.parameters {
            self.resolve_type_annotation(&param.type_annotation)?;
        }
        
        Ok(())
    }
    
    /// 関数パラメータを登録
    fn register_parameters(&mut self, parameters: &[Parameter]) -> Result<()> {
        for param in parameters {
            // パラメータシンボルを作成
            let symbol = Symbol::parameter(
                param.name.name.clone(),
                param.id,
                Some(param.type_annotation.clone()),
                param.location.clone(),
                param.is_mutable,
                self.scope_manager.current_scope_id(),
            );
            
            // シンボルを追加
            self.scope_manager.add_symbol(symbol)?;
        }
        
        Ok(())
    }
    
    /// 実装宣言を解決
    fn resolve_implementation(&mut self, impl_decl: &Implementation) -> Result<()> {
        // 実装対象の型を解決
        self.resolve_type_annotation(&impl_decl.target_type)?;
        
        // トレイト型があれば解決
        if let Some(trait_type) = &impl_decl.trait_type {
            self.resolve_type_annotation(trait_type)?;
        }
        
        // 実装スコープを作成
        self.enter_type_scope(ScopeKind::Impl, |this| {
            // メソッドを登録
            for method in &impl_decl.methods {
                let name = &method.name.name;
                
                // シンボルを作成
                let symbol = Symbol::function(
                    name.clone(),
                    method.id,
                    None, // 型情報は後で追加
                    method.location.clone(),
                    this.scope_manager.current_scope_id(),
                    method.visibility.clone().into(),
                );
                
                // シンボルを追加
                this.scope_manager.add_symbol(symbol)?;
            }
            
            // メソッドを解決
            for method in &impl_decl.methods {
                // 関数のシグネチャを解決
                this.resolve_function_signature(method)?;
                
                // 関数のボディを解決
                this.enter_function_scope(method, |this| {
                    // パラメータを登録
                    this.register_parameters(&method.parameters)?;
                    
                    // 関数本体を解決
                    this.resolve_statement(&method.body)?;
                    
                    Ok(())
                })?;
            }
            
            Ok(())
        })?;
        
        Ok(())
    }
    
    /// 型アノテーションを解決
    fn resolve_type_annotation(&mut self, type_ann: &TypeAnnotation) -> Result<()> {
        match &type_ann.kind {
            TypeKind::Named(identifier) => {
                // 名前付き型の解決
                self.resolve_type_identifier(identifier)?;
            },
            TypeKind::Function(params, return_type) => {
                // 関数型の解決
                // パラメータの型を解決
                for param in params {
                    self.resolve_type_annotation(param)?;
                }
                
                // 戻り値の型を解決
                self.resolve_type_annotation(return_type)?;
            },
            TypeKind::Array(element_type) => {
                // 配列型の解決
                self.resolve_type_annotation(element_type)?;
            },
            TypeKind::Optional(inner_type) => {
                // オプショナル型の解決
                self.resolve_type_annotation(inner_type)?;
            },
            TypeKind::Generic(base_type, type_args) => {
                // ジェネリック型の解決
                self.resolve_type_annotation(base_type)?;
                
                // 型引数を解決
                for arg in type_args {
                    self.resolve_type_annotation(arg)?;
                }
            },
            TypeKind::Tuple(element_types) => {
                // タプル型の解決
                for element in element_types {
                    self.resolve_type_annotation(element)?;
                }
            },
            TypeKind::Union(types) => {
                // 合併型の解決
                for t in types {
                    self.resolve_type_annotation(t)?;
                }
            },
            TypeKind::Intersection(types) => {
                // 交差型の解決
                for t in types {
                    self.resolve_type_annotation(t)?;
                }
            },
            // 基本型はチェック不要
            TypeKind::Int | TypeKind::Float | TypeKind::Bool |
            TypeKind::String | TypeKind::Char | TypeKind::Void |
            TypeKind::Never | TypeKind::Any => {},
        }
        
        Ok(())
    }
    
    /// 型識別子を解決
    fn resolve_type_identifier(&mut self, identifier: &Identifier) -> Result<()> {
        // 名前を検索
        let name = &identifier.name;
        if let Some(symbol) = self.scope_manager.lookup_symbol(name) {
            // シンボルがアクセス可能かチェック
            if !self.scope_manager.is_symbol_accessible(symbol) {
                // アクセス不可
                self.result.add_error(CompilerError::semantic_error(
                    format!("型 '{}' はこのスコープからアクセスできません", name),
                    identifier.location.clone(),
                ));
                return Ok(());
            }
            
            // シンボルが型かチェック
            match symbol.kind {
                SymbolKind::Struct | SymbolKind::Enum |
                SymbolKind::TypeAlias | SymbolKind::Trait => {
                    // 型シンボルなので問題なし
                    self.mark_symbol_used(symbol.node_id);
                    self.result.add_resolution(identifier.id, symbol.node_id);
                },
                _ => {
                    // 型でないシンボル
                    self.result.add_error(CompilerError::semantic_error(
                        format!("'{}' は型ではありません", name),
                        identifier.location.clone(),
                    ));
                }
            }
        } else {
            // シンボルが見つからない
            self.result.add_error(CompilerError::semantic_error(
                format!("型 '{}' が見つかりません", name),
                identifier.location.clone(),
            ));
        }
        
        Ok(())
    }
    
    /// 式を解決
    fn resolve_expression(&mut self, expr: &Expression) -> Result<()> {
        match &expr.kind {
            ExpressionKind::Identifier(identifier) => {
                // 識別子の解決
                self.resolve_identifier(identifier)?;
            },
            ExpressionKind::Literal(_) => {
                // リテラルは解決不要
            },
            ExpressionKind::BinaryOp(op, left, right) => {
                // 二項演算子の解決
                self.resolve_expression(left)?;
                self.resolve_expression(right)?;
            },
            ExpressionKind::UnaryOp(op, operand) => {
                // 単項演算子の解決
                self.resolve_expression(operand)?;
            },
            ExpressionKind::Call(callee, args) => {
                // 関数呼び出しの解決
                self.resolve_expression(callee)?;
                
                // 引数を解決
                for arg in args {
                    self.resolve_expression(arg)?;
                }
            },
            ExpressionKind::MemberAccess(object, member) => {
                // メンバーアクセスの解決
                self.resolve_expression(object)?;
                // メンバー名の解決はオブジェクトの型が必要なため、ここではまだできない
                // 型チェック時に行う
            },
            ExpressionKind::IndexAccess(array, index) => {
                // インデックスアクセスの解決
                self.resolve_expression(array)?;
                self.resolve_expression(index)?;
            },
            ExpressionKind::ArrayLiteral(elements) => {
                // 配列リテラルの解決
                for element in elements {
                    self.resolve_expression(element)?;
                }
            },
            ExpressionKind::StructLiteral(struct_name, fields) => {
                // 構造体リテラルの解決
                self.resolve_identifier(struct_name)?;
                
                // フィールドを解決
                for (name, value) in fields {
                    self.resolve_expression(value)?;
                }
            },
            ExpressionKind::TupleLiteral(elements) => {
                // タプルリテラルの解決
                for element in elements {
                    self.resolve_expression(element)?;
                }
            },
            ExpressionKind::Cast(expr, target_type) => {
                // キャスト式の解決
                self.resolve_expression(expr)?;
                self.resolve_type_annotation(target_type)?;
            },
            ExpressionKind::Lambda(params, body) => {
                // ラムダ式の解決
                self.enter_scope(ScopeKind::Function, |this| {
                    // パラメータを登録
                    this.register_parameters(params)?;
                    
                    // ラムダ本体を解決
                    this.resolve_expression(body)?;
                    
                    Ok(())
                })?;
            },
            ExpressionKind::Block(statements) => {
                // ブロック式の解決
                self.enter_scope(ScopeKind::Block, |this| {
                    for stmt in statements {
                        this.resolve_statement(stmt)?;
                    }
                    Ok(())
                })?;
            },
            ExpressionKind::If(condition, then_branch, else_branch) => {
                // if式の解決
                self.resolve_expression(condition)?;
                
                // then部分を解決
                self.enter_scope(ScopeKind::Block, |this| {
                    this.resolve_statement(then_branch)?;
                    Ok(())
                })?;
                
                // else部分があれば解決
                if let Some(else_stmt) = else_branch {
                    self.enter_scope(ScopeKind::Block, |this| {
                        this.resolve_statement(else_stmt)?;
                        Ok(())
                    })?;
                }
            },
            ExpressionKind::Match(scrutinee, arms) => {
                // match式の解決
                self.resolve_expression(scrutinee)?;
                
                // 各アームを解決
                for arm in arms {
                    // パターンの解決はあまり複雑ではないため、特別な処理は不要
                    self.enter_scope(ScopeKind::Block, |this| {
                        this.resolve_expression(&arm.expression)?;
                        Ok(())
                    })?;
                }
            },
        }
        
        Ok(())
    }
    
    /// 識別子を解決
    fn resolve_identifier(&mut self, identifier: &Identifier) -> Result<()> {
        // 名前を検索
        let name = &identifier.name;
        if let Some(symbol) = self.scope_manager.lookup_symbol(name) {
            // シンボルがアクセス可能かチェック
            if !self.scope_manager.is_symbol_accessible(symbol) {
                // アクセス不可
                self.result.add_error(CompilerError::semantic_error(
                    format!("シンボル '{}' はこのスコープからアクセスできません", name),
                    identifier.location.clone(),
                ));
                return Ok(());
            }
            
            // シンボルを使用済みとしてマーク
            self.mark_symbol_used(symbol.node_id);
            
            // 解決情報を記録
            self.result.add_resolution(identifier.id, symbol.node_id);
        } else {
            // シンボルが見つからない
            self.result.add_error(CompilerError::semantic_error(
                format!("シンボル '{}' が見つかりません", name),
                identifier.location.clone(),
            ));
        }
        
        Ok(())
    }
    
    /// 文を解決
    fn resolve_statement(&mut self, stmt: &Statement) -> Result<()> {
        match &stmt.kind {
            StatementKind::Expression(expr) => {
                // 式文の解決
                self.resolve_expression(expr)?;
            },
            StatementKind::Declaration(decl) => {
                // 宣言文の解決
                self.register_declarations(&[decl.clone()])?;
                self.resolve_declarations(&[decl.clone()])?;
            },
            StatementKind::Block(statements) => {
                // ブロック文の解決
                self.enter_scope(ScopeKind::Block, |this| {
                    for stmt in statements {
                        this.resolve_statement(stmt)?;
                    }
                    Ok(())
                })?;
            },
            StatementKind::If(condition, then_branch, else_branch) => {
                // if文の解決
                self.resolve_expression(condition)?;
                
                // then部分を解決
                self.enter_scope(ScopeKind::Block, |this| {
                    this.resolve_statement(then_branch)?;
                    Ok(())
                })?;
                
                // else部分があれば解決
                if let Some(else_stmt) = else_branch {
                    self.enter_scope(ScopeKind::Block, |this| {
                        this.resolve_statement(else_stmt)?;
                        Ok(())
                    })?;
                }
            },
            StatementKind::While(condition, body) => {
                // while文の解決
                self.resolve_expression(condition)?;
                
                // ループ本体を解決
                self.enter_scope(ScopeKind::Loop, |this| {
                    this.resolve_statement(body)?;
                    Ok(())
                })?;
            },
            StatementKind::For(initializer, condition, increment, body) => {
                // for文の解決
                self.enter_scope(ScopeKind::Loop, |this| {
                    // 初期化部分を解決
                    if let Some(init) = initializer {
                        this.resolve_statement(init)?;
                    }
                    
                    // 条件式を解決
                    if let Some(cond) = condition {
                        this.resolve_expression(cond)?;
                    }
                    
                    // インクリメント部分を解決
                    if let Some(inc) = increment {
                        this.resolve_expression(inc)?;
                    }
                    
                    // ループ本体を解決
                    this.resolve_statement(body)?;
                    
                    Ok(())
                })?;
            },
            StatementKind::ForEach(variable, iterable, body) => {
                // forEach文の解決
                self.enter_scope(ScopeKind::Loop, |this| {
                    // イテラブルを解決
                    this.resolve_expression(iterable)?;
                    
                    // 変数を登録
                    let symbol = Symbol::variable(
                        variable.name.clone(),
                        variable.id,
                        variable.type_annotation.clone(),
                        variable.location.clone(),
                        variable.is_mutable,
                        this.scope_manager.current_scope_id(),
                        Visibility::Private,
                    );
                    
                    this.scope_manager.add_symbol(symbol)?;
                    
                    // ループ本体を解決
                    this.resolve_statement(body)?;
                    
                    Ok(())
                })?;
            },
            StatementKind::Return(value) => {
                // return文の解決
                if let Some(expr) = value {
                    self.resolve_expression(expr)?;
                }
            },
            StatementKind::Break => {
                // break文は特に解決不要
            },
            StatementKind::Continue => {
                // continue文は特に解決不要
            },
        }
        
        Ok(())
    }
    
    /// シンボルを使用済みとしてマーク
    fn mark_symbol_used(&mut self, node_id: NodeId) {
        self.used_symbols.insert(node_id);
    }
    
    /// 未使用シンボルをチェックし、警告を生成
    fn check_unused_symbols(&mut self) {
        // すべてのシンボルについて、使用されているかチェック
        for (node_id, symbol) in &self.scope_manager.symbol_table.symbols {
            if !self.used_symbols.contains(node_id) {
                // 使用されていないシンボル
                // 警告を生成（関数や型は警告しない）
                match symbol.kind {
                    SymbolKind::Variable | SymbolKind::Constant | SymbolKind::Parameter => {
                        self.result.add_warning(CompilerError::warning(
                            format!("{} '{}' は一度も使用されていません", symbol.kind, symbol.name),
                            symbol.location.clone(),
                        ));
                    },
                    _ => {
                        // その他のシンボル（関数、型など）は警告しない
                    }
                }
            }
        }
    }
    
    /// 関数スコープを作成し、関数内処理を実行
    fn enter_function_scope<F>(&mut self, function: &Function, action: F) -> Result<()>
    where
        F: FnOnce(&mut Self) -> Result<()>,
    {
        // 関数スコープを作成
        self.scope_manager.enter_scope(ScopeKind::Function);
        
        // アクションを実行
        let result = action(self);
        
        // スコープを抜ける
        self.scope_manager.exit_scope()?;
        
        result
    }
    
    /// 型スコープを作成し、内部処理を実行
    fn enter_type_scope<F>(&mut self, kind: ScopeKind, action: F) -> Result<()>
    where
        F: FnOnce(&mut Self) -> Result<()>,
    {
        // スコープを作成
        self.scope_manager.enter_scope(kind);
        
        // アクションを実行
        let result = action(self);
        
        // スコープを抜ける
        self.scope_manager.exit_scope()?;
        
        result
    }
    
    /// スコープを作成し、内部処理を実行
    fn enter_scope<F>(&mut self, kind: ScopeKind, action: F) -> Result<()>
    where
        F: FnOnce(&mut Self) -> Result<()>,
    {
        // スコープを作成
        self.scope_manager.enter_scope(kind);
        
        // アクションを実行
        let result = action(self);
        
        // スコープを抜ける
        self.scope_manager.exit_scope()?;
        
        result
    }
    
    /// インポート宣言の解決
    fn resolve_import(&mut self, import: &ast::Import, location: Option<SourceLocation>) -> Result<()> {
        // インポートパスを解析
        let path_components = self.resolve_import_path(import)?;
        
        // 循環インポートの検出
        if self.detect_circular_import(&path_components) {
            return Err(CompilerError::semantic_error(
                format!("循環インポートが検出されました: {}", 
                    path_components.join(".")),
                location,
            ));
        }
        
        // インポートするシンボルを解決
        match &import.kind {
            ast::ImportKind::AllSymbols => {
                // モジュール内のすべてのシンボルをインポート
                self.import_all_symbols(import, &path_components)?;
            },
            ast::ImportKind::SelectedSymbols(symbols) => {
                // 指定されたシンボルのみをインポート
                for symbol_ref in symbols {
                    self.import_selected_symbol(import, &path_components, symbol_ref)?;
                }
            },
            ast::ImportKind::ModuleOnly => {
                // モジュール自体のみをインポート（シンボルはインポートしない）
                self.import_module_only(import, &path_components)?;
            },
        }
        
        // インポート履歴に記録
        self.record_import(&path_components);
        
        // インポート処理完了時にスタックから削除
        self.complete_import(&path_components);
        
        Ok(())
    }
    
    /// インポートパスを解決し、コンポーネントの配列を返す
    fn resolve_import_path(&mut self, import: &ast::Import) -> Result<Vec<String>> {
        let mut path_components = Vec::new();
        
        // パスの各コンポーネントを処理
        for path_component in &import.path {
            let name = path_component.name.clone();
            path_components.push(name);
        }
        
        if path_components.is_empty() {
            return Err(CompilerError::semantic_error(
                "インポートパスが空です".to_string(),
                import.location.clone(),
            ));
        }
        
        Ok(path_components)
    }
    
    /// 循環インポートを検出する
    fn detect_circular_import(&self, path_components: &[String]) -> bool {
        // 循環インポートを検出するための簡易的な実装
        // 現在のインポート履歴を確認し、既に同じパスをインポートしようとしていないか確認
        
        let current_import_path = path_components.join(".");
        self.import_stack.contains(&current_import_path)
    }
    
    /// インポート履歴を記録する
    fn record_import(&mut self, path_components: &[String]) {
        let import_path = path_components.join(".");
        
        // 履歴に追加
        if !self.imported_modules.contains(&import_path) {
            self.imported_modules.push(import_path.clone());
        }
        
        // スタックに追加（循環インポート検出用）
        if !self.import_stack.contains(&import_path) {
            self.import_stack.push(import_path);
        }
    }
    
    /// インポート処理完了時にスタックから削除
    fn complete_import(&mut self, path_components: &[String]) {
        let import_path = path_components.join(".");
        
        // スタックから削除
        if let Some(index) = self.import_stack.iter().position(|p| p == &import_path) {
            self.import_stack.remove(index);
        }
    }
    
    /// モジュール自体のみをインポート（シンボルはインポートしない）
    fn import_module_only(&mut self, import: &ast::Import, path_components: &[String]) -> Result<()> {
        // モジュールが存在するか確認
        if !self.check_module_exists(path_components) {
            return Err(CompilerError::semantic_error(
                format!("モジュール '{}'が見つかりません", path_components.join(".")),
                import.location.clone(),
            ));
        }
        
        // モジュール用のシンボルを作成
        let module_name = if let Some(alias) = &import.alias {
            alias.name.clone()
        } else {
            // エイリアスがない場合は、パスの最後の部分を使用
            path_components.last()
                .map(|s| s.clone())
                .unwrap_or_else(|| "unnamed".to_string())
        };
        
        // シンボル名の衝突をチェック
        if let Some(existing) = self.scope_manager.lookup_symbol_in_current_scope(&module_name) {
            if !import.allow_overrides {
                return Err(CompilerError::semantic_error(
                    format!("シンボル '{}' はすでに定義されています", module_name),
                    import.location.clone(),
                ));
            }
            // 上書き許可がある場合は既存のシンボルを削除
            self.scope_manager.remove_symbol(existing.id)?;
        }
        
        // モジュールシンボルを作成
        let module_symbol = Symbol {
            id: self.next_symbol_id(),
            name: module_name,
            kind: SymbolKind::Module(path_components.to_vec()),
            visibility: Visibility::Public, // インポートされたモジュールは公開として扱う
            scope_id: self.scope_manager.current_scope_id(),
            declared_at: import.location.clone(),
            documentation: None,
        };
        
        // シンボルを追加
        self.scope_manager.add_symbol(module_symbol)?;
        
        Ok(())
    }
    
    /// モジュールの存在を確認
    fn check_module_exists(&self, path_components: &[String]) -> bool {
        // ファイルシステム上でモジュールの存在を確認
        if path_components.is_empty() {
            return false;
        }
        
        // プログラムが設定されていない場合は存在しないとみなす
        let program = match &self.program {
            Some(p) => p,
            None => return false,
        };
        
        // 現在のソースファイルのディレクトリを取得
        let source_dir = match std::path::Path::new(&program.source_path).parent() {
            Some(dir) => dir,
            None => return false,
        };
        
        // モジュールパスからファイルパスを構築
        let module_path = path_components.join("/");
        
        // 可能性のあるファイルパターンをチェック
        let possible_paths = [
            // スクリプトファイル
            format!("{}/{}.sl", source_dir.display(), module_path),
            // モジュールディレクトリ + mod.sl
            format!("{}/{}/mod.sl", source_dir.display(), module_path),
            // インデックスファイル
            format!("{}/{}/index.sl", source_dir.display(), module_path),
        ];
        
        // いずれかのファイルが存在するかチェック
        for path in &possible_paths {
            if std::path::Path::new(path).exists() {
                return true;
            }
        }
        
        // 組み込みモジュールのチェック
        self.is_builtin_module(path_components)
    }
    
    /// 組み込みモジュールかどうかをチェック
    fn is_builtin_module(&self, path_components: &[String]) -> bool {
        if path_components.len() == 1 {
            match path_components[0].as_str() {
                "std" | "core" | "io" | "math" | "string" | "time" | "async" => return true,
                _ => {}
            }
        } else if path_components.len() > 1 && (path_components[0] == "std" || path_components[0] == "core") {
            // std.* または core.* の標準ライブラリモジュール
            return true;
        }
        
        false
    }
    
    /// モジュールのシンボルテーブルを取得
    fn lookup_module_symbols(&self, path_components: &[String]) -> Result<Vec<Symbol>> {
        // モジュールの存在確認
        if !self.check_module_exists(path_components) {
            return Err(CompilerError::semantic_error(
                format!("モジュール '{}'が見つかりません", path_components.join(".")),
                None,
            ));
        }
        
        // 組み込みモジュールの場合は事前定義されたシンボルを返す
        if self.is_builtin_module(path_components) {
            return Ok(self.get_builtin_module_symbols(path_components));
        }
        
        // プログラムが設定されていない場合は空のリストを返す
        let program = match &self.program {
            Some(p) => p,
            None => return Ok(Vec::new()),
        };
        
        // 現在のソースファイルのディレクトリを取得
        let source_dir = match std::path::Path::new(&program.source_path).parent() {
            Some(dir) => dir,
            None => return Ok(Vec::new()),
        };
        
        // モジュールパスからファイルパスを構築
        let module_path = path_components.join("/");
        
        // 可能性のあるファイルパターン
        let possible_paths = [
            // スクリプトファイル
            format!("{}/{}.sl", source_dir.display(), module_path),
            // モジュールディレクトリ + mod.sl
            format!("{}/{}/mod.sl", source_dir.display(), module_path),
            // インデックスファイル
            format!("{}/{}/index.sl", source_dir.display(), module_path),
        ];
        
        // 最初に見つかったファイルをロード
        for path in &possible_paths {
            let path_obj = std::path::Path::new(path);
            if path_obj.exists() {
                // モジュールファイルの内容を読み込む
                let content = match std::fs::read_to_string(path) {
                    Ok(content) => content,
                    Err(_) => continue,
                };
                
                // モジュールを解析してシンボルテーブルを構築
                return self.parse_module_file(path, &content, path_components);
            }
        }
        
        // ファイルが見つからなかった場合は空のリストを返す
        Ok(Vec::new())
    }
    
    /// モジュールファイルを解析してシンボルテーブルを構築
    fn parse_module_file(&self, file_path: &str, content: &str, module_path: &[String]) -> Result<Vec<Symbol>> {
        // 字句解析
        let lexer_result = crate::frontend::lexer::lex(content);
        if let Err(e) = lexer_result {
            return Err(CompilerError::semantic_error(
                format!("モジュール '{}'の字句解析に失敗しました: {}", module_path.join("."), e.message),
                None,
            ));
        }
        
        let tokens = lexer_result.unwrap();
        
        // 構文解析
        let mut parser = crate::frontend::parser::Parser::new(&tokens, file_path);
        let parse_result = parser.parse_program();
        if let Err(e) = parse_result {
            return Err(CompilerError::semantic_error(
                format!("モジュール '{}'の構文解析に失敗しました: {}", module_path.join("."), e.message),
                None,
            ));
        }
        
        let program = parse_result.unwrap();
        
        // モジュールの公開シンボルを収集
        let mut symbols = Vec::new();
        
        for decl in &program.declarations {
            match &decl.kind {
                ast::DeclarationKind::VariableDeclaration(var) => {
                    if self.is_public(&var.visibility) {
                        symbols.push(Symbol::variable(
                            var.name.name.clone(),
                            decl.id,
                            var.type_annotation.clone(),
                            var.location.clone(),
                            var.is_mutable,
                            0, // スコープIDは後で調整
                            self.convert_visibility(&var.visibility),
                        ));
                    }
                },
                ast::DeclarationKind::ConstantDeclaration(constant) => {
                    if self.is_public(&constant.visibility) {
                        symbols.push(Symbol::constant(
                            constant.name.name.clone(),
                            decl.id,
                            constant.type_annotation.clone(),
                            constant.location.clone(),
                            0, // スコープIDは後で調整
                            self.convert_visibility(&constant.visibility),
                        ));
                    }
                },
                ast::DeclarationKind::FunctionDeclaration(function) => {
                    if self.is_public(&function.visibility) {
                        // 関数の型情報を構築
                        let type_info = self.create_function_type_annotation(function);
                        
                        symbols.push(Symbol::function(
                            function.name.name.clone(),
                            decl.id,
                            Some(type_info),
                            function.location.clone(),
                            0, // スコープIDは後で調整
                            self.convert_visibility(&function.visibility),
                        ));
                    }
                },
                ast::DeclarationKind::StructDeclaration(struct_decl) => {
                    if self.is_public(&struct_decl.visibility) {
                        symbols.push(Symbol::type_symbol(
                            struct_decl.name.name.clone(),
                            SymbolKind::Struct,
                            decl.id,
                            struct_decl.location.clone(),
                            0, // スコープIDは後で調整
                            self.convert_visibility(&struct_decl.visibility),
                        ));
                    }
                },
                ast::DeclarationKind::EnumDeclaration(enum_decl) => {
                    if self.is_public(&enum_decl.visibility) {
                        symbols.push(Symbol::type_symbol(
                            enum_decl.name.name.clone(),
                            SymbolKind::Enum,
                            decl.id,
                            enum_decl.location.clone(),
                            0, // スコープIDは後で調整
                            self.convert_visibility(&enum_decl.visibility),
                        ));
                    }
                },
                ast::DeclarationKind::TraitDeclaration(trait_decl) => {
                    if self.is_public(&trait_decl.visibility) {
                        symbols.push(Symbol::type_symbol(
                            trait_decl.name.name.clone(),
                            SymbolKind::Trait,
                            decl.id,
                            trait_decl.location.clone(),
                            0, // スコープIDは後で調整
                            self.convert_visibility(&trait_decl.visibility),
                        ));
                    }
                },
                ast::DeclarationKind::TypeAliasDeclaration(alias) => {
                    if self.is_public(&alias.visibility) {
                        symbols.push(Symbol::type_symbol(
                            alias.name.name.clone(),
                            SymbolKind::TypeAlias,
                            decl.id,
                            alias.location.clone(),
                            0, // スコープIDは後で調整
                            self.convert_visibility(&alias.visibility),
                        ));
                    }
                },
                // その他の宣言は無視
                _ => {}
            }
        }
        
        Ok(symbols)
    }
    
    /// 可視性が公開かどうかをチェック
    fn is_public(&self, visibility: &ast::Visibility) -> bool {
        matches!(visibility, ast::Visibility::Public)
    }
    
    /// ASTの可視性をシンボルテーブルの可視性に変換
    fn convert_visibility(&self, visibility: &ast::Visibility) -> Visibility {
        match visibility {
            ast::Visibility::Public => Visibility::Public,
            ast::Visibility::Private => Visibility::Private,
            ast::Visibility::Protected => Visibility::Trait,   // Protectedはトレイト可視性に対応
            ast::Visibility::Internal => Visibility::Crate,    // Internalはクレート可視性に対応
            ast::Visibility::Package => Visibility::Crate,     // Packageもクレート可視性に対応
            ast::Visibility::Restricted(paths) => {
                // 制限付き可視性はサポートされていない
                // 現在のバージョンではPrivateとして扱う
                Visibility::Private
            }
        }
    }
    
    /// 組み込みモジュールのシンボルを取得
    fn get_builtin_module_symbols(&self, path_components: &[String]) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        
        if path_components.len() == 1 {
            match path_components[0].as_str() {
                "std" => {
                    // 標準ライブラリのルートモジュール
                    // サブモジュールをシンボルとして登録
                    let submodules = ["io", "math", "string", "time", "async", "collections"];
                    for &submodule in &submodules {
                        symbols.push(Symbol {
                            id: self.next_symbol_id(),
                            name: submodule.to_string(),
                            kind: SymbolKind::Module {
                                path: vec!["std".to_string(), submodule.to_string()],
                            },
                            node_id: 0, // ダミーID
                            type_info: None,
                            location: None,
                            is_mutable: false,
                            scope_id: 0,
                            visibility: Visibility::Public,
                        });
                    }
                },
                "io" => {
                    // io標準モジュール
                    symbols.push(Symbol::function(
                        "println".to_string(),
                        self.next_symbol_id(),
                        None,
                        None,
                        0,
                        Visibility::Public,
                    ));
                    symbols.push(Symbol::function(
                        "print".to_string(),
                        self.next_symbol_id(),
                        None,
                        None,
                        0,
                        Visibility::Public,
                    ));
                    symbols.push(Symbol::function(
                        "readLine".to_string(),
                        self.next_symbol_id(),
                        None,
                        None,
                        0,
                        Visibility::Public,
                    ));
                },
                "math" => {
                    // math標準モジュール
                    let functions = ["sin", "cos", "tan", "sqrt", "pow", "log", "exp"];
                    for &func in &functions {
                        symbols.push(Symbol::function(
                            func.to_string(),
                            self.next_symbol_id(),
                            None,
                            None,
                            0,
                            Visibility::Public,
                        ));
                    }
                    
                    symbols.push(Symbol::constant(
                        "PI".to_string(),
                        self.next_symbol_id(),
                        None,
                        None,
                        0,
                        Visibility::Public,
                    ));
                    symbols.push(Symbol::constant(
                        "E".to_string(),
                        self.next_symbol_id(),
                        None,
                        None,
                        0,
                        Visibility::Public,
                    ));
                },
                // 他の標準モジュールも同様に実装
                _ => {}
            }
        } else if path_components.len() > 1 {
            // サブモジュールの実装
            // 例: std.collections モジュール
            if path_components[0] == "std" && path_components[1] == "collections" {
                let types = ["Vector", "HashMap", "Set", "Queue", "Stack"];
                for &typ in &types {
                    symbols.push(Symbol::type_symbol(
                        typ.to_string(),
                        SymbolKind::Struct,
                        self.next_symbol_id(),
                        None,
                        0,
                        Visibility::Public,
                    ));
                }
            }
        }
        
        symbols
    }
    
    /// 次のシンボルIDを生成
    fn next_symbol_id(&self) -> NodeId {
        // 一意のIDを生成
        // 実際の実装では、一意性を保証する仕組みが必要
        self.next_id
    }
    
    /// 現在のモジュールパスを取得
    fn get_current_module_path(&self) -> Vec<String> {
        // プログラムが設定されていない場合は空のパスを返す
        let program = match &self.program {
            Some(p) => p,
            None => return Vec::new(),
        };
        
        // ソースファイルパスからモジュールパスを構築
        let source_path = std::path::Path::new(&program.source_path);
        let module_name = source_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
            
        if module_name == "mod" || module_name == "index" {
            // mod.slまたはindex.slの場合は親ディレクトリ名を使用
            if let Some(parent) = source_path.parent() {
                if let Some(dir_name) = parent.file_name() {
                    if let Some(dir_str) = dir_name.to_str() {
                        return vec![dir_str.to_string()];
                    }
                }
            }
        }
        
        // 通常のファイルの場合はファイル名をモジュール名として使用
        if !module_name.is_empty() {
            vec![module_name.to_string()]
        } else {
            Vec::new()
        }
    }
    
    /// モジュールから特定のシンボルを検索
    fn lookup_module_symbol(&self, path_components: &[String], symbol_name: &str) -> Result<Option<Symbol>> {
        // モジュールのシンボルテーブルを取得
        let module_symbols = self.lookup_module_symbols(path_components)?;
        
        // 指定された名前のシンボルを検索
        for symbol in module_symbols {
            if symbol.name == symbol_name {
                return Ok(Some(symbol));
            }
        }
        
        Ok(None)
    }
    
    /// モジュール内のすべてのシンボルをインポート
    fn import_all_symbols(&mut self, import: &ast::Import, path_components: &[String]) -> Result<()> {
        // モジュールが存在するか確認
        if !self.check_module_exists(path_components) {
            return Err(CompilerError::semantic_error(
                format!("モジュール '{}'が見つかりません", path_components.join(".")),
                import.location.clone(),
            ));
        }
        
        // モジュール内のシンボルを取得
        let symbols = self.lookup_module_symbols(path_components)?;
        
        // 各シンボルを現在のスコープにインポート
        for symbol in symbols {
            // 公開シンボルのみをインポート
            if symbol.visibility == Visibility::Public {
                // 名前衝突をチェック
                if let Some(existing) = self.scope_manager.lookup_symbol_in_current_scope(&symbol.name) {
                    if !import.allow_overrides {
                        // 上書き不可
                        self.result.add_error(CompilerError::semantic_error(
                            format!("シンボル '{}' はすでに定義されています", symbol.name),
                            import.location.clone(),
                        ));
                        continue;
                    }
                    // 上書き許可がある場合は既存のシンボルを削除
                    if let Err(e) = self.scope_manager.remove_symbol(existing.id) {
                        self.result.add_error(e);
                        continue;
                    }
                }
                
                // シンボルを現在のスコープに追加
                if let Err(e) = self.scope_manager.add_symbol(symbol) {
                    self.result.add_error(e);
                }
            }
        }
        
        Ok(())
    }
    
    /// 選択したシンボルのみをインポート
    fn import_selected_symbol(&mut self, import: &ast::Import, path_components: &[String], symbol_ref: &ast::Identifier) -> Result<()> {
        // モジュールが存在するか確認
        if !self.check_module_exists(path_components) {
            return Err(CompilerError::semantic_error(
                format!("モジュール '{}'が見つかりません", path_components.join(".")),
                import.location.clone(),
            ));
        }
        
        // 指定されたシンボルをモジュールから検索
        let symbol_name = &symbol_ref.name;
        let result = self.lookup_module_symbol(path_components, symbol_name)?;
        
        if let Some(symbol) = result {
            // シンボルが見つかった場合
            
            // エイリアスが指定されている場合は名前を変更
            let final_name = if let Some(alias) = &symbol_ref.alias {
                alias.name.clone()
            } else {
                symbol.name.clone()
            };
            
            // 名前衝突をチェック
            if let Some(existing) = self.scope_manager.lookup_symbol_in_current_scope(&final_name) {
                if !import.allow_overrides {
                    // 上書き不可
                    return Err(CompilerError::semantic_error(
                        format!("シンボル '{}' はすでに定義されています", final_name),
                        symbol_ref.alias.as_ref().map_or_else(|| symbol_ref.location.clone(), |a| a.location.clone()),
                    ));
                }
                // 上書き許可がある場合は既存のシンボルを削除
                self.scope_manager.remove_symbol(existing.id)?;
            }
            
            // エイリアスが指定されている場合は名前を変更したコピーを作成
            let imported_symbol = if final_name != symbol.name {
                Symbol {
                    id: self.next_symbol_id(),
                    name: final_name,
                    kind: symbol.kind.clone(),
                    visibility: symbol.visibility,
                    scope_id: self.scope_manager.current_scope_id(),
                    declared_at: symbol_ref.location.clone(),
                    documentation: symbol.documentation.clone(),
                }
            } else {
                // エイリアスがない場合は元のシンボルをそのまま使用
                symbol
            };
            
            // シンボルを現在のスコープに追加
            self.scope_manager.add_symbol(imported_symbol)?;
        } else {
            // シンボルが見つからない場合
            return Err(CompilerError::semantic_error(
                format!("モジュール '{}'にシンボル '{}'が見つかりません", 
                    path_components.join("."), symbol_name),
                symbol_ref.location.clone(),
            ));
        }
        
        Ok(())
    }
}

// Visibilityの変換
impl From<ast::Visibility> for Visibility {
    fn from(vis: ast::Visibility) -> Self {
        match vis {
            ast::Visibility::Public => Visibility::Public,
            ast::Visibility::Private => Visibility::Private,
            ast::Visibility::Protected => Visibility::Trait,   // Protectedはトレイト可視性に対応
            ast::Visibility::Internal => Visibility::Crate,    // Internalはクレート可視性に対応
            ast::Visibility::Package => Visibility::Crate,     // Packageもクレート可視性に対応
            ast::Visibility::Restricted(paths) => {
                // 制限付き可視性はサポートされていない
                // 現在のバージョンではPrivateとして扱う
                Visibility::Private
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::ast::{
        self, BinaryOperator, Expression, ExpressionKind, Identifier, Literal, LiteralKind, 
        Program, Statement, StatementKind, Declaration, DeclarationKind, VariableDeclaration, 
        ConstantDeclaration, Function, Struct, StructField, TypeAnnotation, TypeKind
    };
    use crate::frontend::error::SourceLocation;
    use std::path::PathBuf;
    
    // テスト用ヘルパー関数: 識別子を作成
    fn create_identifier(name: &str, id: NodeId) -> Identifier {
        Identifier {
            name: name.to_string(),
            id,
            location: None,
        }
    }
    
    // テスト用ヘルパー関数: ソースの位置情報を作成
    fn create_location(line: usize, column: usize) -> Option<SourceLocation> {
        Some(SourceLocation::new(
            "test.sl".to_string(),
            line,
            column,
            line,
            column + 1,
        ))
    }
    
    // テスト用ヘルパー関数: シンプルなプログラムを作成
    fn create_test_program() -> Program {
        Program {
            name: "test".to_string(),
            source_path: PathBuf::from("test.sl"),
            declarations: Vec::new(),
            id: 0,
            location: None,
        }
    }
    
    // テスト用ヘルパー関数: 変数宣言を作成
    fn create_variable_declaration(name: &str, id: NodeId, is_mutable: bool, initializer: Option<Expression>, visibility: ast::Visibility) -> Declaration {
        Declaration {
            id,
            kind: DeclarationKind::VariableDeclaration(
                VariableDeclaration {
                    name: create_identifier(name, id + 1),
                    type_annotation: None,
                    is_mutable,
                    initializer,
                    visibility,
                    id: id + 2,
                    location: None,
                }
            ),
            location: None,
        }
    }
    
    // テスト用ヘルパー関数: 式としての識別子参照を作成
    fn create_identifier_expression(name: &str, id: NodeId) -> Expression {
        Expression {
            id,
            kind: ExpressionKind::Identifier(create_identifier(name, id + 1)),
            location: None,
        }
    }
    
    // テスト用ヘルパー関数: 構造体宣言を作成
    fn create_struct_declaration(name: &str, id: NodeId, visibility: ast::Visibility) -> Declaration {
        Declaration {
            id,
            kind: DeclarationKind::StructDeclaration(
                Struct {
                    name: create_identifier(name, id + 1),
                    type_parameters: Vec::new(),
                    fields: vec![
                        StructField {
                            name: create_identifier("field1", id + 2),
                            type_annotation: TypeAnnotation {
                                id: id + 3,
                                kind: TypeKind::Named(create_identifier("int", id + 4)),
                                location: None,
                            },
                            visibility: ast::Visibility::Public,
                            id: id + 5,
                            location: None,
                        }
                    ],
                    visibility,
                    id: id + 6,
                    location: None,
                }
            ),
            location: None,
        }
    }
    
    #[test]
    fn test_basic_variable_resolution() {
        // 基本的な変数宣言と参照の名前解決をテスト
        let mut program = create_test_program();
        
        // 変数宣言: let x = 42
        let var_decl = create_variable_declaration(
            "x", 
            1, 
            false, 
            Some(Expression {
                id: 4,
                kind: ExpressionKind::Literal(
                    Literal {
                        kind: LiteralKind::Integer(42),
                        id: 5,
                        location: None,
                    }
                ),
                location: None,
            }),
            ast::Visibility::Private
        );
        
        // 参照式: x
        let ref_expr = create_identifier_expression("x", 6);
        
        // 式文: x;
        let expr_stmt = Statement {
            id: 8,
            kind: StatementKind::Expression(ref_expr),
            location: None,
        };
        
        // プログラムに宣言と文を追加
        program.declarations.push(var_decl);
        
        // シンプルな参照式として文を追加（本来は文のリストだが、テストでは宣言だけを使用）
        let expr_decl = Declaration {
            id: 9,
            kind: DeclarationKind::VariableDeclaration(
                VariableDeclaration {
                    name: create_identifier("_test", 10),
                    type_annotation: None,
                    is_mutable: false,
                    initializer: Some(create_identifier_expression("x", 11)),
                    visibility: ast::Visibility::Private,
                    id: 12,
                    location: None,
                }
            ),
            location: None,
        };
        program.declarations.push(expr_decl);
        
        // 名前解決を実行
        let mut resolver = NameResolver::new();
        let result = resolver.resolve_program(&program).unwrap();
        
        // 変数参照が変数宣言を指していることを確認
        assert!(result.resolved_nodes.contains_key(&7)); // 識別子のID (6+1)
        assert_eq!(result.resolved_nodes[&7], 1); // 変数宣言のID
        
        // もう一つの参照も確認
        assert!(result.resolved_nodes.contains_key(&12)); // 識別子のID (11+1)
        assert_eq!(result.resolved_nodes[&12], 1); // 変数宣言のID
    }
    
    #[test]
    fn test_struct_resolution() {
        // 構造体宣言と参照の名前解決をテスト
        let mut program = create_test_program();
        
        // 構造体宣言: struct Point { x: int, y: int }
        let struct_decl = create_struct_declaration("Point", 1, ast::Visibility::Public);
        
        // 構造体の参照: Point
        let struct_ref = create_identifier_expression("Point", 10);
        
        // 変数宣言: let p: Point = ...
        let var_decl = Declaration {
            id: 15,
            kind: DeclarationKind::VariableDeclaration(
                VariableDeclaration {
                    name: create_identifier("p", 16),
                    type_annotation: Some(TypeAnnotation {
                        id: 17,
                        kind: TypeKind::Named(create_identifier("Point", 18)),
                        location: None,
                    }),
                    is_mutable: false,
                    initializer: None,
                    visibility: ast::Visibility::Private,
                    id: 19,
                    location: None,
                }
            ),
            location: None,
        };
        
        // プログラムに宣言を追加
        program.declarations.push(struct_decl);
        program.declarations.push(var_decl);
        
        // 名前解決を実行
        let mut resolver = NameResolver::new();
        let result = resolver.resolve_program(&program).unwrap();
        
        // 型注釈での構造体参照が構造体宣言を指していることを確認
        assert!(result.resolved_nodes.contains_key(&18)); // 型注釈内の識別子のID
        assert_eq!(result.resolved_nodes[&18], 1); // 構造体宣言のID
    }
    
    #[test]
    fn test_undeclared_variable() {
        // 未宣言変数の参照で適切なエラーが発生することをテスト
        let mut program = create_test_program();
        
        // 未宣言変数の参照: z
        let undeclared_ref = create_identifier_expression("z", 1);
        
        // 式文: z;
        let expr_stmt = Statement {
            id: 3,
            kind: StatementKind::Expression(undeclared_ref),
            location: None,
        };
        
        // 変数宣言で未宣言変数を使用: let y = z
        let var_decl = Declaration {
            id: 4,
            kind: DeclarationKind::VariableDeclaration(
                VariableDeclaration {
                    name: create_identifier("y", 5),
                    type_annotation: None,
                    is_mutable: false,
                    initializer: Some(create_identifier_expression("z", 6)),
                    visibility: ast::Visibility::Private,
                    id: 7,
                    location: None,
                }
            ),
            location: None,
        };
        
        // プログラムに宣言を追加
        program.declarations.push(var_decl);
        
        // 名前解決を実行
        let mut resolver = NameResolver::new();
        let result = resolver.resolve_program(&program).unwrap();
        
        // エラーが発生していることを確認
        assert!(result.has_errors());
        
        // エラーメッセージが「シンボル 'z' が見つかりません」を含むこと
        let contains_error = result.errors.iter().any(|e| {
            e.message.contains("'z'") && e.message.contains("見つかりません")
        });
        
        assert!(contains_error);
    }
    
    #[test]
    fn test_variable_scope() {
        // スコープの境界をまたいだ変数参照をテスト
        let mut program = create_test_program();
        
        // グローバル変数宣言: let global = 42
        let global_decl = create_variable_declaration(
            "global", 
            1, 
            false, 
            Some(Expression {
                id: 4,
                kind: ExpressionKind::Literal(
                    Literal {
                        kind: LiteralKind::Integer(42),
                        id: 5,
                        location: None,
                    }
                ),
                location: None,
            }),
            ast::Visibility::Public
        );
        
        // 関数宣言: fn test() { ... }
        let function_decl = Declaration {
            id: 6,
            kind: DeclarationKind::FunctionDeclaration(
                Function {
                    name: create_identifier("test", 7),
                    type_parameters: Vec::new(),
                    parameters: Vec::new(),
                    return_type: None,
                    body: Statement {
                        id: 8,
                        kind: StatementKind::Block(vec![
                            // ローカル変数宣言: let local = global
                            Statement {
                                id: 9,
                                kind: StatementKind::Declaration(
                                    Declaration {
                                        id: 10,
                                        kind: DeclarationKind::VariableDeclaration(
                                            VariableDeclaration {
                                                name: create_identifier("local", 11),
                                                type_annotation: None,
                                                is_mutable: false,
                                                initializer: Some(create_identifier_expression("global", 12)),
                                                visibility: ast::Visibility::Private,
                                                id: 13,
                                                location: None,
                                            }
                                        ),
                                        location: None,
                                    }
                                ),
                                location: None,
                            }
                        ]),
                        location: None,
                    },
                    visibility: ast::Visibility::Public,
                    is_async: false,
                    is_extern: false,
                    id: 14,
                    location: None,
                }
            ),
            location: None,
        };
        
        // プログラムに宣言を追加
        program.declarations.push(global_decl);
        program.declarations.push(function_decl);
        
        // 名前解決を実行
        let mut resolver = NameResolver::new();
        let result = resolver.resolve_program(&program).unwrap();
        
        // グローバル変数への参照が正しく解決されていることを確認
        assert!(result.resolved_nodes.contains_key(&13)); // global参照の識別子ID
        assert_eq!(result.resolved_nodes[&13], 1); // グローバル変数宣言のID
        
        // エラーがないことを確認
        assert!(!result.has_errors());
    }
    
    #[test]
    fn test_visibility_levels() {
        // 可視性レベルが適切に処理されることをテスト
        let mut program = create_test_program();
        
        // 公開構造体: public struct PublicStruct { ... }
        let public_struct = create_struct_declaration("PublicStruct", 1, ast::Visibility::Public);
        
        // 非公開構造体: private struct PrivateStruct { ... }
        let private_struct = create_struct_declaration("PrivateStruct", 10, ast::Visibility::Private);
        
        // プログラムに宣言を追加
        program.declarations.push(public_struct);
        program.declarations.push(private_struct);
        
        // 名前解決を実行
        let mut resolver = NameResolver::new();
        let result = resolver.resolve_program(&program).unwrap();
        
        // シンボルテーブルから公開構造体を取得
        let public_symbol = resolver.scope_manager.lookup_symbol("PublicStruct");
        assert!(public_symbol.is_some());
        let public_symbol = public_symbol.unwrap();
        
        // シンボルテーブルから非公開構造体を取得
        let private_symbol = resolver.scope_manager.lookup_symbol("PrivateStruct");
        assert!(private_symbol.is_some());
        let private_symbol = private_symbol.unwrap();
        
        // 公開シンボルの可視性がPublicであることを確認
        assert!(matches!(public_symbol.visibility, Visibility::Public));
        
        // 非公開シンボルの可視性がPrivateであることを確認
        assert!(matches!(private_symbol.visibility, Visibility::Private));
    }
} 
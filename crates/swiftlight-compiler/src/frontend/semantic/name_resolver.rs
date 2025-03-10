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
use crate::frontend::error::{Result, CompilerError, Diagnostic, SourceLocation};
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
}

impl NameResolver {
    /// 新しい名前リゾルバーを作成
    pub fn new() -> Self {
        Self {
            scope_manager: ScopeManager::new(),
            result: NameResolutionResult::new(),
            used_symbols: HashSet::new(),
            program: None,
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
                    // インポート宣言
                    // TODO: インポートの処理
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
        // TODO: 関数シグネチャの型アノテーション作成
        // 仮実装：一旦空の型を返す
        TypeAnnotation {
            id: ast::generate_id(),
            kind: TypeKind::Named(Identifier {
                id: ast::generate_id(),
                name: "Function".to_string(),
            }),
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
                DeclarationKind::Import(_) => {
                    // インポート宣言の解決
                    // TODO: インポートの処理
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
}

// Visibilityの変換
impl From<ast::Visibility> for Visibility {
    fn from(vis: ast::Visibility) -> Self {
        match vis {
            ast::Visibility::Public => Visibility::Public,
            ast::Visibility::Private => Visibility::Private,
            // TODO: 他の可視性レベルのサポート
            _ => Visibility::Private,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // TODO: 名前解決のテスト追加
} 
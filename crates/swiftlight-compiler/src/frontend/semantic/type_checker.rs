//! SwiftLight言語の型チェックを担当するモジュールです。
//! 式や文の型を検証し、型エラーを検出します。

use std::collections::{HashMap, HashSet, VecDeque};
use crate::frontend::ast::{
    self, Program, Declaration, Statement, Expression, TypeAnnotation, TypeKind,
    NodeId, ExpressionKind, StatementKind, DeclarationKind, Identifier, Function,
    VariableDeclaration, ConstantDeclaration, Parameter, Struct, Enum, Trait, TypeAlias,
    Implementation, Import, BinaryOperator, UnaryOperator, Literal, LiteralKind, 
    // MatchArm, // 一時的にコメントアウト
};
use crate::frontend::parser::ast::{MatchArm, Pattern}; // 正しいパスからMatchArmとPatternをインポート
use crate::frontend::error::{Result, CompilerError, SourceLocation};
use crate::diagnostics::Diagnostic; // 正しいパスからDiagnosticをインポート
use super::name_resolver::NameResolutionResult;
use super::symbol_table::{SymbolTable, Symbol, SymbolKind, Visibility};
use super::scope::ScopeManager;
use crate::typesystem::TypeRegistry;
use crate::diagnostics::DiagnosticEmitter;

/// 型チェックの結果
#[derive(Debug, Clone, Default)]
pub struct TypeCheckResult {
    /// ノードの型情報
    pub node_types: HashMap<NodeId, TypeAnnotation>,
    
    /// 検出されたエラー
    pub errors: Vec<CompilerError>,
    
    /// 警告
    pub warnings: Vec<CompilerError>,
    
    /// 型推論の結果
    pub inferred_types: HashMap<NodeId, TypeAnnotation>,
    
    /// 型キャスト情報
    pub cast_operations: HashMap<NodeId, (TypeAnnotation, TypeAnnotation)>,
    
    /// ノードの統計情報（診断用）
    pub statistics: TypeCheckStatistics,
}

/// 型チェックの統計情報
#[derive(Debug, Clone, Default)]
pub struct TypeCheckStatistics {
    /// チェックされた式の数
    pub expressions_checked: usize,
    
    /// チェックされた文の数
    pub statements_checked: usize,
    
    /// チェックされた宣言の数
    pub declarations_checked: usize,
    
    /// 推論された型の数
    pub types_inferred: usize,
    
    /// キャスト操作の数
    pub casts_performed: usize,
    
    /// エラーの数
    pub errors_detected: usize,
    
    /// 警告の数
    pub warnings_generated: usize,
}

impl TypeCheckResult {
    /// 新しい型チェック結果を作成
    pub fn new() -> Self {
        Self {
            node_types: HashMap::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
            inferred_types: HashMap::new(),
            cast_operations: HashMap::new(),
            statistics: TypeCheckStatistics::default(),
        }
    }
    
    /// エラーを追加
    pub fn add_error(&mut self, error: CompilerError) {
        self.errors.push(error);
        self.statistics.errors_detected += 1;
    }
    
    /// 警告を追加
    pub fn add_warning(&mut self, warning: CompilerError) {
        self.warnings.push(warning);
        self.statistics.warnings_generated += 1;
    }
    
    /// ノードの型を記録
    pub fn set_node_type(&mut self, node_id: NodeId, type_ann: TypeAnnotation) {
        self.node_types.insert(node_id, type_ann);
    }
    
    /// ノードの型を取得
    pub fn get_node_type(&self, node_id: NodeId) -> Option<&TypeAnnotation> {
        self.node_types.get(&node_id)
    }
    
    /// エラーがあるか
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
    
    /// 警告があるか
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
    
    /// 推論された型を記録
    pub fn set_inferred_type(&mut self, node_id: NodeId, type_ann: TypeAnnotation) {
        self.inferred_types.insert(node_id, type_ann);
        self.statistics.types_inferred += 1;
    }
    
    /// キャスト操作を記録
    pub fn record_cast(&mut self, node_id: NodeId, from_type: TypeAnnotation, to_type: TypeAnnotation) {
        self.cast_operations.insert(node_id, (from_type, to_type));
        self.statistics.casts_performed += 1;
    }
    
    /// 型チェック結果を合併
    pub fn merge(&mut self, other: TypeCheckResult) {
        self.node_types.extend(other.node_types);
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
        self.inferred_types.extend(other.inferred_types);
        self.cast_operations.extend(other.cast_operations);
        
        // 統計情報の合併
        self.statistics.expressions_checked += other.statistics.expressions_checked;
        self.statistics.statements_checked += other.statistics.statements_checked;
        self.statistics.declarations_checked += other.statistics.declarations_checked;
        self.statistics.types_inferred += other.statistics.types_inferred;
        self.statistics.casts_performed += other.statistics.casts_performed;
        self.statistics.errors_detected += other.statistics.errors_detected;
        self.statistics.warnings_generated += other.statistics.warnings_generated;
    }
    
    /// 診断情報を文字列として取得
    pub fn get_diagnostics_summary(&self) -> String {
        format!(
            "型チェック結果: {} 宣言, {} 文, {} 式をチェック\n\
             {} 型を推論, {} キャスト操作を実行\n\
             {} エラー, {} 警告を検出",
            self.statistics.declarations_checked,
            self.statistics.statements_checked,
            self.statistics.expressions_checked,
            self.statistics.types_inferred,
            self.statistics.casts_performed,
            self.statistics.errors_detected,
            self.statistics.warnings_generated
        )
    }
}

/// 型チェックで使用するコンテキスト情報
#[derive(Debug, Clone)]
struct TypeCheckContext {
    /// 現在の関数の戻り値型
    return_type: Option<TypeAnnotation>,
    
    /// ループの深さ（break/continueの検証用）
    loop_depth: usize,
    
    /// 現在のスコープのID
    scope_id: usize,
    
    /// 型変数のマッピング（ジェネリック型のためのマッピング）
    type_variables: HashMap<String, TypeAnnotation>,
    
    /// 型制約リスト
    constraints: Vec<TypeConstraint>,
}

/// 型制約
#[derive(Debug, Clone, PartialEq)]
enum TypeConstraint {
    /// 部分型制約 (sub <: super)
    Subtype {
        sub: TypeAnnotation,
        super_type: TypeAnnotation,
        location: Option<SourceLocation>,
    },
    
    /// 等価制約 (t1 == t2)
    Equal {
        left: TypeAnnotation,
        right: TypeAnnotation,
        location: Option<SourceLocation>,
    },
    
    /// メンバー存在制約 (type has member)
    HasMember {
        type_ann: TypeAnnotation,
        member_name: String,
        member_type: Option<TypeAnnotation>,
        location: Option<SourceLocation>,
    },
}

/// 型変数のバインド状態
#[derive(Debug, Clone, PartialEq)]
enum TypeVariableBinding {
    /// バインドされていない
    Unbound,
    
    /// 他の型にバインドされている
    Bound(TypeAnnotation),
}

/// 型チェッカー
pub struct TypeChecker {
    /// 名前解決の結果
    name_resolution: NameResolutionResult,
    
    /// シンボルテーブル
    symbol_table: SymbolTable,
    
    /// 型チェック結果
    result: TypeCheckResult,
    
    /// 型レジストリ
    type_registry: TypeRegistry,
    
    /// 診断情報エミッタ
    diagnostic_emitter: DiagnosticEmitter,
    
    /// 型エイリアスマッピング（型エイリアスID → 解決された型）
    type_aliases: HashMap<NodeId, TypeAnnotation>,
    
    /// 型チェックコンテキスト
    context: TypeCheckContext,
    
    /// バインディング環境（型変数 → バインド状態）
    bindings: HashMap<String, TypeVariableBinding>,
    
    /// 構造体のフィールド型情報（構造体ID → (フィールド名 → 型)）
    struct_fields: HashMap<NodeId, HashMap<String, TypeAnnotation>>,
    
    /// トレイトのメソッド型情報（トレイトID → (メソッド名 → 型)）
    trait_methods: HashMap<NodeId, HashMap<String, TypeAnnotation>>,
    
    /// 列挙型のバリアント情報（列挙型ID → (バリアント名 → 型)）
    enum_variants: HashMap<NodeId, HashMap<String, TypeAnnotation>>,
    
    /// 型チェック設定
    options: TypeCheckOptions,
}

/// 型チェックのオプション設定
#[derive(Debug, Clone)]
pub struct TypeCheckOptions {
    /// 警告を有効にするか
    pub enable_warnings: bool,
    
    /// 厳格なモード（より厳しい型チェック）
    pub strict_mode: bool,
    
    /// 型推論を行うか
    pub enable_type_inference: bool,
    
    /// 型キャストの安全性制約レベル
    pub cast_safety_level: CastSafetyLevel,
    
    /// 未使用の型を警告するか
    pub warn_unused_types: bool,
    
    /// 曖昧な型推論を警告するか
    pub warn_type_ambiguities: bool,
    
    /// 冗長なキャストを警告するか
    pub warn_redundant_casts: bool,
}

/// キャストの安全性レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastSafetyLevel {
    /// 非常に厳格（ほとんどのキャストを禁止）
    VeryStrict,
    
    /// 標準（安全なキャストのみ許可）
    Standard,
    
    /// 緩和（潜在的に安全でないキャストも許可）
    Relaxed,
}

impl Default for TypeCheckOptions {
    fn default() -> Self {
        Self {
            enable_warnings: true,
            strict_mode: false,
            enable_type_inference: true,
            cast_safety_level: CastSafetyLevel::Standard,
            warn_unused_types: true,
            warn_type_ambiguities: true,
            warn_redundant_casts: true,
        }
    }
}

impl TypeChecker {
    /// 新しい型チェッカーを作成
    pub fn new(
        type_registry: TypeRegistry,
        diagnostic_emitter: DiagnosticEmitter
    ) -> Self {
        let name_resolution = NameResolutionResult::default();
        let symbol_table = SymbolTable::new();
        Self::with_options(
            name_resolution, 
            symbol_table, 
            type_registry,
            diagnostic_emitter,
            TypeCheckOptions::default()
        )
    }
    
    /// オプション付きで型チェッカーを作成
    pub fn with_options(
        name_resolution: NameResolutionResult,
        symbol_table: SymbolTable,
        type_registry: TypeRegistry,
        diagnostic_emitter: DiagnosticEmitter,
        options: TypeCheckOptions,
    ) -> Self {
        Self {
            name_resolution,
            symbol_table,
            result: TypeCheckResult::new(),
            type_registry,
            diagnostic_emitter,
            type_aliases: HashMap::new(),
            context: TypeCheckContext {
                return_type: None,
                loop_depth: 0,
                scope_id: 0, // グローバルスコープ
                type_variables: HashMap::new(),
                constraints: Vec::new(),
            },
            bindings: HashMap::new(),
            struct_fields: HashMap::new(),
            trait_methods: HashMap::new(),
            enum_variants: HashMap::new(),
            options,
        }
    }
    
    /// プログラムの型チェックを実行
    pub fn check_program(&mut self, program: &Program) -> Result<TypeCheckResult> {
        // 名前解決でエラーがある場合は型チェックをスキップ
        if self.name_resolution.has_errors() {
            // 名前解決のエラーを転送
            for error in &self.name_resolution.errors {
                self.result.add_error(error.clone());
            }
            return Ok(self.result.clone());
        }
        
        // 型チェックの準備：プリミティブ型や組み込み型を登録
        self.initialize_builtin_types();
        
        // プログラム内の型を事前収集（相互参照を解決するため）
        self.collect_type_declarations(program)?;
        
        // 型エイリアスの循環参照チェック
        self.check_circular_dependencies()?;
        
        // 構造体のフィールド情報を収集
        self.collect_struct_fields(program)?;
        
        // 列挙型のバリアント情報を収集
        self.collect_enum_variants(program)?;
        
        // トレイトのメソッド情報を収集
        self.collect_trait_methods(program)?;
        
        // オプショナル型や共用体型などの派生型情報を処理
        self.process_derived_types();
        
        // 宣言の型チェック（関数、構造体、列挙型等）
        let mut total_errors = 0;
        
        for decl in &program.declarations {
            // 統計情報を更新
            self.result.statistics.declarations_checked += 1;
            
            match self.check_declaration(decl) {
                Ok(_) => {},
                Err(e) => {
                    // エラーを記録するが、可能な限り続行する
                    self.result.add_error(e);
                    total_errors += 1;
                    
                    // エラーが多すぎる場合は中断
                    if total_errors > 100 {
                        self.result.add_error(CompilerError::internal_error(
                            "型チェックを中断: エラーが多すぎます".to_string(),
                            None,
                        ));
                        break;
                    }
                }
            }
        }
        
        // 文の型チェック（プログラム本体のステートメント）
        for stmt in &program.declarations {
            // 統計情報を更新
            self.result.statistics.statements_checked += 1;
            
            match self.check_statement(stmt) {
                Ok(_) => {},
                Err(e) => {
                    // エラーを記録するが、可能な限り続行する
                    self.result.add_error(e);
                    total_errors += 1;
                    
                    // エラーが多すぎる場合は中断
                    if total_errors > 100 {
                        self.result.add_error(CompilerError::internal_error(
                            "型チェックを中断: エラーが多すぎます".to_string(),
                            None,
                        ));
                        break;
                    }
                }
            }
        }
        
        // 型制約を解決
        if let Err(e) = self.solve_type_constraints() {
            self.result.add_error(e);
        }
        
        // 未使用の型や警告の生成（警告が有効な場合のみ）
        if self.options.enable_warnings {
            self.generate_warnings();
        }
        
        // 診断情報を出力（デバッグ用）
        // println!("{}", self.result.get_diagnostics_summary());
        
        // 型チェック結果を返す
        Ok(self.result.clone())
    }
    
    /// 宣言の型チェック
    fn check_declaration(&mut self, declaration: &Declaration) -> Result<()> {
        match &declaration.kind {
            DeclarationKind::VariableDecl(var_decl) => self.check_variable_declaration(var_decl, declaration)?,
            DeclarationKind::ConstantDecl(const_decl) => self.check_constant_declaration(const_decl, declaration)?,
            DeclarationKind::FunctionDecl(func_decl) => self.check_function_declaration(func_decl, declaration)?,
            DeclarationKind::StructDecl(struct_decl) => self.check_struct_declaration(struct_decl, declaration)?,
            DeclarationKind::EnumDecl(enum_decl) => self.check_enum_declaration(enum_decl, declaration)?,
            DeclarationKind::TraitDecl(trait_decl) => self.check_trait_declaration(trait_decl, declaration)?,
            DeclarationKind::TypeAliasDecl(alias) => self.check_type_alias_declaration(alias, declaration)?,
            DeclarationKind::ImplementationDecl(impl_decl) => self.check_implementation_declaration(impl_decl, declaration)?,
            DeclarationKind::ImportDecl(import) => self.check_import_declaration(import, declaration)?,
        }
        
        Ok(())
    }
    
    // 変数宣言のチェック
    fn check_variable_declaration(&mut self, var_decl: &VariableDeclaration, decl: &Declaration) -> Result<()> {
        // 初期化式があれば型チェック
        let init_type = if let Some(initializer) = &var_decl.initializer {
            self.check_expression(initializer)?
        } else if let Some(type_ann) = &var_decl.type_annotation {
            // 初期化式がなく、型注釈がある場合はその型
            self.resolve_type_annotation(type_ann)?
        } else {
            // 初期化式も型注釈もない場合はエラー
            return Err(CompilerError::type_error(
                "変数宣言には初期化式または型注釈が必要です",
                decl.location.clone(),
            ));
        };
        
        // 明示的な型注釈があれば、初期化式の型と一致するかチェック
        if let Some(type_ann) = &var_decl.type_annotation {
            let declared_type = self.resolve_type_annotation(type_ann)?;
            
            if !self.is_compatible(&declared_type, &init_type) {
                return Err(self.type_error(
                    "変数の型と初期化式の型が一致しません",
                    &declared_type,
                    &init_type,
                    decl.location.clone(),
                ));
            }
            
            // 変数の型を記録
            self.result.set_node_type(decl.id, declared_type.clone());
            
            // 冗長な型注釈の警告
            if self.options.warn_redundant_casts && self.is_redundant_annotation(&declared_type, &init_type) {
                self.result.add_warning(CompilerError::warning(
                    format!("冗長な型注釈 '{}' - 初期化式から型を推論できます", self.type_to_string(&declared_type)),
                    type_ann.location.clone(),
                ));
            }
        } else if var_decl.initializer.is_some() {
            // 型注釈がなければ初期化式から型を推論
            self.result.set_node_type(decl.id, init_type.clone());
            self.result.set_inferred_type(decl.id, init_type);
        }
        
        // 型変数や識別子の型も記録
        self.result.set_node_type(var_decl.name.id, self.result.node_types[&decl.id].clone());
        
        Ok(())
    }
    
    // 冗長な型注釈かどうかチェック
    fn is_redundant_annotation(&self, declared_type: &TypeAnnotation, inferred_type: &TypeAnnotation) -> bool {
        match (&declared_type.kind, &inferred_type.kind) {
            // 基本型同士で完全に一致する場合は冗長
            (TypeKind::Primitive(PrimitiveType::Int), TypeKind::Primitive(PrimitiveType::Int)) |
            (TypeKind::Primitive(PrimitiveType::Float), TypeKind::Primitive(PrimitiveType::Float)) |
            (TypeKind::Primitive(PrimitiveType::Bool), TypeKind::Primitive(PrimitiveType::Bool)) |
            (TypeKind::Primitive(PrimitiveType::String), TypeKind::Primitive(PrimitiveType::String)) |
            (TypeKind::Primitive(PrimitiveType::Char), TypeKind::Primitive(PrimitiveType::Char)) => true,
            
            // 名前付き型が完全に一致する場合は冗長
            (TypeKind::Named(name1), TypeKind::Named(name2)) if name1.name == name2.name => true,
            
            // それ以外の場合は冗長ではない
            _ => false,
        }
    }
    
    // 定数宣言のチェック
    fn check_constant_declaration(&mut self, const_decl: &ConstantDeclaration, decl: &Declaration) -> Result<()> {
        // 初期化式の型をチェック（定数には必ず初期化式がある）
        let init_type = self.check_expression(&const_decl.initializer)?;
        
        // 明示的な型注釈があれば、初期化式の型と一致するかチェック
        if let Some(type_ann) = &const_decl.type_annotation {
            let declared_type = self.resolve_type_annotation(type_ann)?;
            
            if !self.is_compatible(&declared_type, &init_type) {
                return Err(self.type_error(
                    "定数の型と初期化式の型が一致しません",
                    &declared_type,
                    &init_type,
                    decl.location.clone(),
                ));
            }
            
            // 定数の型を記録
            self.result.set_node_type(decl.id, declared_type.clone());
            
            // 冗長な型注釈の警告
            if self.options.warn_redundant_casts && self.is_redundant_annotation(&declared_type, &init_type) {
                self.result.add_warning(CompilerError::warning(
                    format!("冗長な型注釈 '{}' - 初期化式から型を推論できます", self.type_to_string(&declared_type)),
                    type_ann.location.clone(),
                ));
            }
        } else {
            // 型注釈がなければ初期化式から型を推論
            self.result.set_node_type(decl.id, init_type.clone());
            self.result.set_inferred_type(decl.id, init_type);
        }
        
        // 型変数や識別子の型も記録
        self.result.set_node_type(const_decl.name.id, self.result.node_types[&decl.id].clone());
        
        // 定数の初期化式が定数式かどうかチェック
        if !self.is_constant_expression(&const_decl.initializer) {
            self.result.add_warning(CompilerError::warning(
                "定数宣言の初期化式は定数式である必要があります",
                const_decl.initializer.location.clone(),
            ));
        }
        
        Ok(())
    }
    
    // 式が定数式かどうかチェック
    fn is_constant_expression(&self, expr: &Expression) -> bool {
        match &expr.kind {
            // リテラルは定数式
            ExpressionKind::Literal(_) => true,
            
            // 識別子は定数の参照なら定数式
            ExpressionKind::Identifier(ident) => {
                if let Some(symbol_id) = self.name_resolution.resolved_nodes.get(&ident.id) {
                    if let Some(symbol) = self.symbol_table.get_symbol(*symbol_id) {
                        return symbol.kind == SymbolKind::Constant;
                    }
                }
                false
            },
            
            // 二項演算は両辺が定数式なら定数式
            ExpressionKind::BinaryOp(_, left, right) => {
                self.is_constant_expression(left) && self.is_constant_expression(right)
            },
            
            // 単項演算はオペランドが定数式なら定数式
            ExpressionKind::UnaryOp(_, operand) => self.is_constant_expression(operand),
            
            // その他は定数式ではない
            _ => false,
        }
    }
    
    // 関数宣言のチェック
    fn check_function_declaration(&mut self, func_decl: &Function, decl: &Declaration) -> Result<()> {
        // パラメータの型をチェック
        let mut param_types = Vec::with_capacity(func_decl.parameters.len());
        
        for param in &func_decl.parameters {
            let param_type = self.resolve_type_annotation(&param.type_annotation)?;
            param_types.push(param_type);
            
            // パラメータIDに型情報を記録
            self.result.set_node_type(param.id, param_type);
        }
        
        // 戻り値の型をチェック
        let return_type = if let Some(type_ann) = &func_decl.return_type {
            self.resolve_type_annotation(type_ann)?
        } else {
            // 戻り値の型注釈がなければVoid型
            TypeAnnotation {
                id: ast::generate_id(),
                kind: TypeKind::Primitive(PrimitiveType::Void),
                location: None,
            }
        };
        
        // 関数の型を記録
        let function_type = TypeAnnotation {
            id: decl.id,
            kind: TypeKind::Function(param_types.clone(), Some(Box::new(return_type.clone()))),
            location: decl.location.clone(),
        };
        
        self.result.set_node_type(decl.id, function_type);
        
        // 型変数や識別子の型も記録
        self.result.set_node_type(func_decl.name.id, self.result.node_types[&decl.id].clone());
        
        // 関数コンテキストを設定
        let prev_context = self.context.clone();
        self.context.return_type = Some(return_type.clone());
        self.context.scope_id += 1; // 新しいスコープに入る
        
        // 関数本体をチェック
        self.check_statement(&func_decl.body)?;
        
        // 戻り値チェック
        self.verify_return_paths(&func_decl.body, &return_type, decl.location.clone())?;
        
        // コンテキストを復元
        self.context = prev_context;
        
        Ok(())
    }
    
    // 関数の全パスが適切な戻り値を持つことを検証
    fn verify_return_paths(&self, body: &Statement, expected_return_type: &TypeAnnotation, location: Option<SourceLocation>) -> Result<()> {
        // Void型の関数は戻り値チェック不要
        if matches!(expected_return_type.kind, TypeKind::Primitive(PrimitiveType::Void)) {
            return Ok(());
        }
        
        // 関数本体が必ず値を返すかチェック
        if !self.always_returns(body) {
            return Err(CompilerError::type_error(
                format!("関数は '{}' 型の値を返す必要がありますが、一部の実行パスで値が返されません", 
                        self.type_to_string(expected_return_type)),
                location,
            ));
        }
        
        Ok(())
    }
    
    // 文が必ず値を返すかチェック
    fn always_returns(&self, stmt: &Statement) -> bool {
        match &stmt.kind {
            // 式文は値を返さない
            StatementKind::ExpressionStmt(_) => false,
            
            // 宣言は値を返さない
            StatementKind::DeclarationStmt(_) => false,
            
            // ブロックは最後の文が値を返すなら値を返す
            StatementKind::Block(statements) => {
                statements.last().map_or(false, |last| self.always_returns(last))
            },
            
            // if文は両方の分岐が値を返すなら値を返す
            StatementKind::IfStmt(_, then_branch, else_branch) => {
                self.always_returns(then_branch) && 
                else_branch.as_ref().map_or(false, |e| self.always_returns(e))
            },
            
            // while文はループが必ず実行されるなら値を返す（保守的に評価してfalse）
            StatementKind::WhileStmt(_, _) => false,
            
            // for文はループが必ず実行されるなら値を返す（保守的に評価してfalse）
            StatementKind::ForStmt(_, _, _, _) => false,
            StatementKind::ForStmtEach(_, _, _) => false,
            
            // return文は値を返す
            StatementKind::ReturnStmt(_) => true,
            
            // break/continue文は値を返さない
            StatementKind::BreakStmt | StatementKind::ContinueStmt => false,
        }
    }
    
    // 組み込み型の初期化
    fn initialize_builtin_types(&mut self) {
        // 基本型（Int, Float, Bool, String, Char, Void）を登録
        let primitive_types = [
            ("Int", TypeKind::Primitive(PrimitiveType::Int)),
            ("Float", TypeKind::Primitive(PrimitiveType::Float)),
            ("Bool", TypeKind::Primitive(PrimitiveType::Bool)),
            ("String", TypeKind::Primitive(PrimitiveType::String)),
            ("Char", TypeKind::Primitive(PrimitiveType::Char)),
            ("Void", TypeKind::Primitive(PrimitiveType::Void)),
        ];
        
        for (name, kind) in primitive_types {
            let type_id = ast::generate_id();
            let type_ann = TypeAnnotation {
                id: type_id,
                kind,
                location: None,
            };
            
            // 組み込み型をノードIDに関連付けて登録
            self.result.node_types.insert(type_id, type_ann);
        }
    }
    
    // 型宣言の事前収集
    fn collect_type_declarations(&mut self, program: &Program) -> Result<()> {
        // 構造体、列挙型、トレイト、型エイリアスなどの型宣言を収集
        for decl in &program.declarations {
            match &decl.kind {
                DeclarationKind::StructDecl(struct_decl) => {
                    // 構造体の型情報を仮登録（フィールド解決は後で行う）
                    let type_id = decl.id;
                    let struct_name = &struct_decl.name.name;
                    
                    let type_ann = TypeAnnotation {
                        id: type_id,
                        kind: TypeKind::Named(struct_decl.name.clone()),
                        location: decl.location.clone(),
                    };
                    
                    self.result.node_types.insert(type_id, type_ann);
                },
                DeclarationKind::EnumDecl(enum_decl) => {
                    // 列挙型の型情報を仮登録
                    let type_id = decl.id;
                    let enum_name = &enum_decl.name.name;
                    
                    let type_ann = TypeAnnotation {
                        id: type_id,
                        kind: TypeKind::Named(enum_decl.name.clone()),
                        location: decl.location.clone(),
                    };
                    
                    self.result.node_types.insert(type_id, type_ann);
                },
                DeclarationKind::TraitDecl(trait_decl) => {
                    // トレイトの型情報を仮登録
                    let type_id = decl.id;
                    let trait_name = &trait_decl.name.name;
                    
                    let type_ann = TypeAnnotation {
                        id: type_id,
                        kind: TypeKind::Named(trait_decl.name.clone()),
                        location: decl.location.clone(),
                    };
                    
                    self.result.node_types.insert(type_id, type_ann);
                },
                DeclarationKind::TypeAliasDecl(alias) => {
                    // 型エイリアスの型情報を仮登録
                    // エイリアス先の型はまだ解決しない（循環参照の可能性があるため）
                    let type_id = decl.id;
                    let alias_name = &alias.name.name;
                    
                    let type_ann = TypeAnnotation {
                        id: type_id,
                        kind: TypeKind::Named(alias.name.clone()),
                        location: decl.location.clone(),
                    };
                    
                    self.result.node_types.insert(type_id, type_ann);
                },
                _ => {
                    // その他の宣言は型宣言ではないのでスキップ
                }
            }
        }
        
        Ok(())
    }
    
    // 型エイリアスの循環参照をチェック
    fn check_circular_dependencies(&mut self) -> Result<()> {
        // 依存関係グラフを構築
        let mut dependencies: HashMap<NodeId, HashSet<NodeId>> = HashMap::new();
        
        // 各型エイリアスについて依存関係を収集
        for (alias_id, alias_type) in &self.type_aliases {
            let mut deps = HashSet::new();
            self.collect_type_dependencies(*alias_id, alias_type, &mut deps);
            dependencies.insert(*alias_id, deps);
        }
        
        // サイクルチェックを実行
        let mut visited = HashSet::new();
        let mut path = HashSet::new();
        
        for alias_id in self.type_aliases.keys() {
            if !visited.contains(alias_id) {
                self.check_cycle(*alias_id, &dependencies, &mut visited, &mut path)?;
            }
        }
        
        Ok(())
    }
    
    // 型の依存関係を収集
    fn collect_type_dependencies(&self, alias_id: NodeId, typ: &TypeAnnotation, deps: &mut HashSet<NodeId>) {
        match &typ.kind {
            TypeKind::Named(ident) => {
                // 名前付き型の場合、その型が型エイリアスかどうかチェック
                if let Some(symbol_id) = self.name_resolution.resolved_nodes.get(&ident.id) {
                    if let Some(symbol) = self.symbol_table.get_symbol(*symbol_id) {
                        if symbol.kind == SymbolKind::TypeAlias {
                            deps.insert(*symbol_id);
                            
                            // さらに、その型エイリアスの依存先も調べる
                            if let Some(target_type) = self.type_aliases.get(symbol_id) {
                                self.collect_type_dependencies(*symbol_id, target_type, deps);
                            }
                        }
                    }
                }
            },
            TypeKind::Array(elem_type, _) => {  // 配列長の情報は依存関係に不要なため無視
                // 配列の要素型の依存関係を調べる
                self.collect_type_dependencies(alias_id, elem_type, deps);
            },
            TypeKind::Optional(inner_type) => {
                // オプショナル型の内部型の依存関係を調べる
                self.collect_type_dependencies(alias_id, inner_type, deps);
            },
            TypeKind::Union(types) => {
                // ユニオン型の各要素の依存関係を調べる
                for typ in types {
                    self.collect_type_dependencies(alias_id, typ, deps);
                }
            },
            TypeKind::Intersection(types) => {
                // インターセクション型の各要素の依存関係を調べる
                for typ in types {
                    self.collect_type_dependencies(alias_id, typ, deps);
                }
            },
            TypeKind::Tuple(types) => {
                // タプル型の各要素の依存関係を調べる
                for typ in types {
                    self.collect_type_dependencies(alias_id, typ, deps);
                }
            },
            TypeKind::Function(param_types, return_type) => {
                // 関数型のパラメータ型と戻り値型の依存関係を調べる
                for param_type in param_types {
                    self.collect_type_dependencies(alias_id, param_type, deps);
                }
                self.collect_type_dependencies(alias_id, return_type, deps);
            },
            TypeKind::Generic(base, args) => {
                // ジェネリック型の基本型と型引数の依存関係を調べる
                self.collect_type_dependencies(alias_id, base, deps);
                for arg in args {
                    self.collect_type_dependencies(alias_id, arg, deps);
                }
            },
            // 基本型は依存関係なし
            _ => {}
        }
    }
    
    // サイクル検出アルゴリズム（深さ優先探索）
    fn check_cycle(
        &self,
        current: NodeId,
        dependencies: &HashMap<NodeId, HashSet<NodeId>>,
        visited: &mut HashSet<NodeId>,
        path: &mut HashSet<NodeId>,
    ) -> Result<()> {
        // すでに訪問したノードはスキップ
        if visited.contains(&current) {
            return Ok(());
        }
        
        // 現在のパスにあるなら循環参照
        if path.contains(&current) {
            // 循環参照のエラーメッセージを作成
            let cycle_path: Vec<String> = path.iter()
                .filter_map(|id| self.symbol_table.get_symbol(*id))
                .map(|symbol| symbol.name.clone())
                .collect();
            
            let cycle_str = format!("{} -> {}", cycle_path.join(" -> "), 
                                  self.symbol_table.get_symbol(current)
                                      .map(|s| s.name.clone())
                                      .unwrap_or_else(|| "?".to_string()));
            
            return Err(CompilerError::type_error(
                format!("型エイリアスの循環参照を検出しました: {}", cycle_str),
                self.symbol_table.get_symbol(current).and_then(|s| s.location.clone()),
            ));
        }
        
        // 現在のノードをパスに追加
        path.insert(current);
        
        // 依存先のノードを再帰的にチェック
        if let Some(deps) = dependencies.get(&current) {
            for dep in deps {
                self.check_cycle(*dep, dependencies, visited, path)?;
            }
        }
        
        // 現在のノードをパスから削除
        path.remove(&current);
        
        // 現在のノードを訪問済みとしてマーク
        visited.insert(current);
        
        Ok(())
    }
    
    // 関数コンテキストに入る
    fn enter_function_context(&mut self, return_type: TypeAnnotation) {
        self.context.return_type = Some(return_type);
    }
    
    // 関数コンテキストから出る
    fn exit_function_context(&mut self) {
        self.context.return_type = None;
    }
    
    // ループコンテキストに入る
    fn enter_loop_context(&mut self) {
        self.context.loop_depth += 1;
    }
    
    // ループコンテキストから出る
    fn exit_loop_context(&mut self) {
        if self.context.loop_depth > 0 {
            self.context.loop_depth -= 1;
        }
    }
    
    // 現在ループ内にいるかどうか
    fn is_in_loop(&self) -> bool {
        self.context.loop_depth > 0
    }
    
    // 戻り値の型チェック
    fn check_return_expression(&mut self, expr: Option<&Expression>, location: Option<SourceLocation>) -> Result<()> {
        if let Some(expected_return_type) = &self.context.return_type {
            match expr {
                Some(e) => {
                    // 戻り値がある場合、型が一致するかチェック
                    let actual_type = self.check_expression(e)?;
                    
                    if !self.is_compatible(expected_return_type, &actual_type) {
                        return Err(self.type_error(
                            "関数の戻り値の型が宣言と一致しません",
                            expected_return_type,
                            &actual_type,
                            location,
                        ));
                    }
                },
                None => {
                    // 戻り値がない場合、Void型と一致するかチェック
                    let void_type = TypeAnnotation {
                        id: ast::generate_id(),
                        kind: TypeKind::Primitive(PrimitiveType::Void),
                        location: None,
                    };
                    
                    if !self.is_compatible(expected_return_type, &void_type) {
                        return Err(self.type_error(
                            "戻り値が必要な関数に空のreturn文が使われています",
                            expected_return_type,
                            &void_type,
                            location,
                        ));
                    }
                }
            }
        } else {
            // 関数コンテキスト外でのreturn文はエラー
            return Err(CompilerError::type_error(
                "関数外でreturn文は使用できません".to_string(),
                location,
            ));
        }
        
        Ok(())
    }
    
    // 型が互換性を持つかどうかを確認
    fn is_compatible(&self, expected: &TypeAnnotation, actual: &TypeAnnotation) -> bool {
        match (&expected.kind, &actual.kind) {
            // 同じ型は互換性あり
            (a, b) if a == b => true,
            
            // Any型は任意の型と互換性あり
            (TypeKind::Any, _) | (_, TypeKind::Any) => true,
            
            // 整数型と浮動小数点型の互換性
            (TypeKind::Primitive(PrimitiveType::Int), TypeKind::Primitive(PrimitiveType::Float)) => false, // 暗黙的な型変換は許可しない
            (TypeKind::Primitive(PrimitiveType::Float), TypeKind::Primitive(PrimitiveType::Int)) => false,
            
            // Optional型の互換性
            (TypeKind::Optional(expected_inner), inner_type) => {
                self.is_compatible(expected_inner, actual)
            },
            (_, TypeKind::Optional(actual_inner)) => false, // 非Optional→Optionalは暗黙変換しない
            // 配列型の互換性（要素型と配列長の両方をチェック）
            (TypeKind::Array(expected_element, expected_len), TypeKind::Array(actual_element, actual_len)) => {
                // 要素型の互換性と配列長の一致を確認
                self.is_compatible(expected_element, actual_element) && expected_len == actual_len
            },
            // 関数型の互換性
            (TypeKind::Function(expected_params, expected_return), 
             TypeKind::Function(actual_params, actual_return)) => {
                // パラメータの数が同じ
                if expected_params.len() != actual_params.len() {
                    return false;
                }
                
                // 各パラメータの型が互換性を持つ（反変）
                for (exp_param, act_param) in expected_params.iter().zip(actual_params.iter()) {
                    if !self.is_compatible(act_param, exp_param) {
                        return false;
                    }
                }
                
                // 戻り値の型が互換性を持つ（共変）
                self.is_compatible(expected_return, actual_return)
            },
            
            // ユニオン型の互換性
            (TypeKind::Union(expected_types), _) => {
                // 実際の型がユニオン型のいずれかの型と互換性があるかチェック
                expected_types.iter().any(|expected_type| self.is_compatible(expected_type, actual))
            },
            (_, TypeKind::Union(actual_types)) => {
                // ユニオン型の各型が期待される型と互換性があるかチェック
                actual_types.iter().all(|actual_type| self.is_compatible(expected, actual_type))
            },
            
            // 名前付き型の互換性
            (TypeKind::Named(expected_name), TypeKind::Named(actual_name)) => {
                // 名前が同じ場合は互換性あり
                // 実際の実装ではシンボルの解決が必要
                expected_name.name == actual_name.name
            },
            
            // デフォルトは互換性なし
            _ => false,
        }
    }
    
    // 2つの型の最小共通型（union）を計算
    fn common_type(&self, type1: &TypeAnnotation, type2: &TypeAnnotation) -> Result<TypeAnnotation> {
        if self.is_compatible(type1, type2) {
            Ok(type1.clone())
        } else if self.is_compatible(type2, type1) {
            Ok(type2.clone())
        } else {
            // 互換性がない場合はユニオン型を作成
            let union_id = ast::generate_id();
            Ok(TypeAnnotation {
                id: union_id,
                kind: TypeKind::Union(vec![type1.clone(), type2.clone()]),
                location: None,
            })
        }
    }
    
    // 型エラーを生成
    fn type_error(&mut self, message: impl Into<String>, expected: &TypeAnnotation, 
                 actual: &TypeAnnotation, location: Option<SourceLocation>) -> CompilerError {
        let msg = format!("{}: 期待される型: {}, 実際の型: {}", 
                          message.into(), 
                          self.type_to_string(expected), 
                          self.type_to_string(actual));
        CompilerError::type_error(msg, location)
    }
    
    // 型を文字列に変換
    fn type_to_string(&self, typ: &TypeAnnotation) -> String {
        match &typ.kind {
            TypeKind::Primitive(PrimitiveType::Int) => "Int".to_string(),
            TypeKind::Primitive(PrimitiveType::Float) => "Float".to_string(),
            TypeKind::Primitive(PrimitiveType::Bool) => "Bool".to_string(),
            TypeKind::Primitive(PrimitiveType::String) => "String".to_string(),
            TypeKind::Primitive(PrimitiveType::Char) => "Char".to_string(),
            TypeKind::Primitive(PrimitiveType::Void) => "Void".to_string(),
            TypeKind::Never => "Never".to_string(),
            TypeKind::Any => "Any".to_string(),
            
            TypeKind::Named(ident) => ident.name.clone(),
            
            TypeKind::Array(element_type, _) => 
                format!("[{}]", self.type_to_string(element_type)),
            TypeKind::Optional(inner_type) => 
                format!("{}?", self.type_to_string(inner_type)),
                
            TypeKind::Function(params, return_type) => {
                let params_str = params.iter()
                    .map(|param| self.type_to_string(param))
                    .collect::<Vec<_>>()
                    .join(", ");
                    
                format!("({}) -> {}", params_str, self.type_to_string(return_type))
            },
            
            TypeKind::Tuple(element_types) => {
                let elements_str = element_types.iter()
                    .map(|typ| self.type_to_string(typ))
                    .collect::<Vec<_>>()
                    .join(", ");
                    
                format!("({})", elements_str)
            },
            
            TypeKind::Union(types) => {
                let types_str = types.iter()
                    .map(|typ| self.type_to_string(typ))
                    .collect::<Vec<_>>()
                    .join(" | ");
                    
                format!("({})", types_str)
            },
            
            TypeKind::Intersection(types) => {
                let types_str = types.iter()
                    .map(|typ| self.type_to_string(typ))
                    .collect::<Vec<_>>()
                    .join(" & ");
                    
                format!("({})", types_str)
            },
            
            TypeKind::Generic(base_type, type_args) => {
                let args_str = type_args.iter()
                    .map(|arg| self.type_to_string(arg))
                    .collect::<Vec<_>>()
                    .join(", ");
                    
                format!("{}<{}>", self.type_to_string(base_type), args_str)
            },
        }
    }
    
    // 警告生成
    fn generate_warnings(&mut self) {
        // 未使用の型や変数の警告
        self.check_unused_types();
        
        // 冗長なキャストの警告
        self.check_redundant_casts();
        
        // 型の曖昧さに関する警告
        self.check_type_ambiguities();
    }
    
    // 未使用の型を検出して警告
    fn check_unused_types(&mut self) {
        let mut used_types = HashSet::new();
        
        // 使用されている型を収集
        for type_ann in self.result.node_types.values() {
            self.collect_used_types(type_ann, &mut used_types);
        }
        
        // 未使用の型を検出
        for (node_id, symbol) in self.symbol_table.all_symbols() {
            match symbol.kind {
                SymbolKind::Struct | SymbolKind::Enum | SymbolKind::Trait | SymbolKind::TypeAlias => {
                    if !used_types.contains(node_id) {
                        self.result.add_warning(CompilerError::warning(
                            format!("未使用の型 '{}'", symbol.name),
                            symbol.location.clone(),
                        ));
                    }
                },
                _ => {}
            }
        }
    }
    
    // 型に含まれる型参照を収集
    fn collect_used_types(&self, typ: &TypeAnnotation, used_types: &mut HashSet<NodeId>) {
        match &typ.kind {
            TypeKind::Named(name) => {
                if let Some(symbol_id) = self.name_resolution.resolved_nodes.get(&name.id) {
                    used_types.insert(*symbol_id);
                }
            },
            TypeKind::Array(element_type, _) => {
                self.collect_used_types(element_type, used_types);
            },
            TypeKind::Optional(inner_type) => {
                self.collect_used_types(inner_type, used_types);
            },
            TypeKind::Function(params, return_type) => {
                for param in params {
                    self.collect_used_types(param, used_types);
                }
                self.collect_used_types(return_type, used_types);
            },
            TypeKind::Tuple(elements) => {
                for element in elements {
                    self.collect_used_types(element, used_types);
                }
            },
            TypeKind::Union(types) | TypeKind::Intersection(types) => {
                for t in types {
                    self.collect_used_types(t, used_types);
                }
            },
            TypeKind::Generic(base, args) => {
                self.collect_used_types(base, used_types);
                for arg in args {
                    self.collect_used_types(arg, used_types);
                }
            },
            _ => {},
        }
    }
    
    // 冗長なキャストを検出して警告
    fn check_redundant_casts(&mut self) {
        // ASTで記録されたキャスト操作をチェックし、キャスト前後の型が同一なら警告を出します。
        for (node_id, (from_type, to_type)) in self.result.cast_operations.iter() {
            if from_type == to_type {
                self.result.add_warning(CompilerError::new_warning(
                    format!("冗長なキャスト: ノード {} で型 '{}' から同じ型へのキャストは不要です.", node_id, self.type_to_string(from_type))
                ));
            }
        }
    }
    
    // 型の曖昧さを検出して警告
    fn check_type_ambiguities(&mut self) {
        // 推論された各ノードの型情報をチェックし、Ambiguous型の場合、候補が2つ以上あれば警告を出します。
        for (node_id, type_ann) in self.result.inferred_types.iter() {
            if let TypeAnnotation::Ambiguous(variants) = type_ann {
                if variants.len() > 1 {
                    self.result.add_warning(CompilerError::new_warning(
                        format!("あいまいな型が検出されました: ノード {} に複数の型候補が存在します: {:?}", node_id, variants)
                    ));
                }
            }
        }
    }
    
    // トレイトのメソッド情報を収集
    fn collect_trait_methods(&mut self, program: &Program) -> Result<()> {
        for decl in &program.declarations {
            if let DeclarationKind::TraitDecl(trait_decl) = &decl.kind {
                let trait_id = decl.id;
                let mut methods = HashMap::new();
                
                // メソッドの型情報を収集
                for method in &trait_decl.methods {
                    // メソッドのパラメータと戻り値型を解決
                    let mut param_types = Vec::new();
                    for param in &method.parameters {
                        let param_type = self.resolve_type_annotation(&param.type_annotation)?;
                        param_types.push(param_type);
                    }
                    
                    // 戻り値型（省略時はVoid）
                    let return_type = if let Some(ret_type) = &method.return_type {
                        self.resolve_type_annotation(ret_type)?
                    } else {
                        TypeAnnotation {
                            id: ast::generate_id(),
                            kind: TypeKind::Primitive(PrimitiveType::Void),
                            location: None,
                        }
                    };
                    
                    // メソッドの型を関数型として登録
                    let method_type = TypeAnnotation {
                        id: method.id,
                        kind: TypeKind::Function(param_types, Some(Box::new(return_type))),
                        location: method.location.clone(),
                    };
                    
                    methods.insert(method.name.name.clone(), method_type);
                }
                
                self.trait_methods.insert(trait_id, methods);
            }
        }
        
        Ok(())
    }
    
    // 構造体宣言のチェック
    fn check_struct_declaration(&mut self, struct_decl: &Struct, decl: &Declaration) -> Result<()> {
        // フィールド名の重複チェック
        let mut field_names = HashSet::new();
        for field in &struct_decl.fields {
            let field_name = &field.name.name;
            if !field_names.insert(field_name.clone()) {
                return Err(CompilerError::type_error(
                    format!("構造体 '{}' に重複したフィールド名 '{}' があります", struct_decl.name.name, field_name),
                    field.location.clone(),
                ));
            }
            
            // フィールドの型をチェック
            let field_type = self.resolve_type_annotation(&field.type_annotation)?;
            
            // デフォルト値がある場合、その型がフィールドの型と互換性があるかチェック
            if let Some(default_value) = &field.default_value {
                let value_type = self.check_expression(default_value)?;
                if !self.is_compatible(&field_type, &value_type) {
                    return Err(self.type_error(
                        format!("フィールド '{}' のデフォルト値の型がフィールドの型と一致しません", field_name),
                        &field_type,
                        &value_type,
                        field.location.clone(),
                    ));
                }
            }
        }
        
        // 構造体の型を記録
        // すでに型宣言収集フェーズで登録済みなので割愛
        
        Ok(())
    }
    
    // 列挙型宣言のチェック
    fn check_enum_declaration(&mut self, enum_decl: &Enum, decl: &Declaration) -> Result<()> {
        // バリアント名の重複チェック
        let mut variant_names = HashSet::new();
        for variant in &enum_decl.variants {
            let variant_name = &variant.name.name;
            if !variant_names.insert(variant_name.clone()) {
                return Err(CompilerError::type_error(
                    format!("列挙型 '{}' に重複したバリアント名 '{}' があります", enum_decl.name.name, variant_name),
                    variant.location.clone(),
                ));
            }
            
            // 関連値がある場合、その型をチェック
            if let Some(associated_values) = &variant.associated_types {
                for (i, type_ann) in associated_values.iter().enumerate() {
                    self.resolve_type_annotation(type_ann)?;
                }
            }
        }
        
        // 列挙型の型を記録
        // すでに型宣言収集フェーズで登録済みなので割愛
        
        Ok(())
    }
    
    // トレイト宣言のチェック
    fn check_trait_declaration(&mut self, trait_decl: &Trait, decl: &Declaration) -> Result<()> {
        // メソッド名の重複チェック
        let mut method_names = HashSet::new();
        for method in &trait_decl.methods {
            let method_name = &method.name.name;
            if !method_names.insert(method_name.clone()) {
                return Err(CompilerError::type_error(
                    format!("トレイト '{}' に重複したメソッド名 '{}' があります", trait_decl.name.name, method_name),
                    method.location.clone(),
                ));
            }
            
            // パラメータの型をチェック
            for param in &method.parameters {
                self.resolve_type_annotation(&param.type_annotation)?;
            }
            
            // 戻り値の型をチェック
            if let Some(return_type) = &method.return_type {
                self.resolve_type_annotation(return_type)?;
            }
            
            // デフォルト実装がある場合、その本体をチェック
            if let Some(body) = &method.default_implementation {
                // 関数コンテキストを設定
                let return_type = if let Some(rt) = &method.return_type {
                    self.resolve_type_annotation(rt)?
                } else {
                    TypeAnnotation {
                        id: ast::generate_id(),
                        kind: TypeKind::Primitive(PrimitiveType::Void),
                        location: None,
                    }
                };
                
                let prev_context = self.context.clone();
                self.context.return_type = Some(return_type.clone());
                self.context.scope_id += 1; // 新しいスコープに入る
                
                // メソッド本体をチェック
                self.check_statement(body)?;
                
                // コンテキストを復元
                self.context = prev_context;
            }
        }
        
        // トレイトの型を記録
        // すでに型宣言収集フェーズで登録済みなので割愛
        
        Ok(())
    }
    
    // 型エイリアス宣言のチェック
    fn check_type_alias_declaration(&mut self, alias: &TypeAlias, decl: &Declaration) -> Result<()> {
        // エイリアス先の型をチェック
        self.resolve_type_annotation(&alias.target_type)?;
        
        // 型エイリアスの型を記録
        // すでに型宣言収集フェーズで登録済みなので割愛
        
        Ok(())
    }
    
    // トレイト実装宣言のチェック
    fn check_implementation_declaration(&mut self, impl_decl: &Implementation, decl: &Declaration) -> Result<()> {
        // 実装対象の型をチェック
        let target_type = self.resolve_type_annotation(&impl_decl.target_type)?;
        
        // トレイトがある場合、そのトレイトをチェック
        let trait_type = if let Some(trait_name) = &impl_decl.trait_name {
            // トレイト名を解決
            if let Some(symbol_id) = self.name_resolution.resolved_nodes.get(&trait_name.id) {
                if let Some(symbol) = self.symbol_table.get_symbol(*symbol_id) {
                    if symbol.kind != SymbolKind::Trait {
                        return Err(CompilerError::type_error(
                            format!("'{}' はトレイトではありません", trait_name.name),
                            trait_name.location.clone(),
                        ));
                    }
                    
                    // トレイトのメソッド情報を取得
                    if let Some(methods) = self.trait_methods.get(symbol_id) {
                        Some((symbol, methods.clone()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                return Err(CompilerError::type_error(
                    format!("トレイト '{}' が見つかりません", trait_name.name),
                    trait_name.location.clone(),
                ));
            }
        } else {
            None
        };
        
        // トレイトメソッドの実装を検証
        if let Some((trait_symbol, trait_methods)) = trait_type {
            // 必須メソッドが実装されているか確認
            let mut implemented_methods = HashSet::new();
            for method in &impl_decl.methods {
                if let StatementKind::FunctionDeclaration(func) = &method.kind {
                    implemented_methods.insert(func.name.name.clone());
                }
            }
            
            // トレイトの各メソッドについてチェック
            for (method_name, method_type) in &trait_methods {
                if !implemented_methods.contains(method_name) {
                    // トレイトのメソッドがデフォルト実装を持つ場合はOK
                    let has_default = self.trait_has_default_implementation(trait_symbol.node_id, method_name);
                    if !has_default {
                        return Err(CompilerError::type_error(
                            format!("トレイト '{}' のメソッド '{}' が実装されていません", 
                                    trait_symbol.name, method_name),
                            decl.location.clone(),
                        ));
                    }
                }
            }
        }
        
        // メソッドの実装をチェック
        for method in &impl_decl.methods {
            if let StatementKind::FunctionDeclaration(func) = &method.kind {
                // 関数コンテキストを設定
                let return_type = if let Some(rt) = &func.return_type {
                    self.resolve_type_annotation(rt)?
                } else {
                    TypeAnnotation {
                        id: ast::generate_id(),
                        kind: TypeKind::Primitive(PrimitiveType::Void),
                        location: None,
                    }
                };
                
                let prev_context = self.context.clone();
                self.context.return_type = Some(return_type.clone());
                self.context.scope_id += 1; // 新しいスコープに入る
                
                // メソッド本体をチェック
                self.check_statement(&func.body)?;
                
                // コンテキストを復元
                self.context = prev_context;
                
                // トレイトメソッドとの整合性チェック
                if let Some((_, trait_methods)) = &trait_type {
                    if let Some(trait_method_type) = trait_methods.get(&func.name.name) {
                        self.check_method_signature_compatibility(trait_method_type, func, method.location.clone())?;
                    }
                }
            } else {
                return Err(CompilerError::type_error(
                    "実装ブロック内には関数宣言のみ許可されています",
                    method.location.clone(),
                ));
            }
        }
        
        Ok(())
    }
    
    // トレイトメソッドがデフォルト実装を持つかチェック
    fn trait_has_default_implementation(&self, trait_id: NodeId, method_name: &str) -> bool {
        // プログラムフィールドへの参照を修正する必要があるかもしれません
        for decl in &self.name_resolution.program.declarations {
            if let DeclarationKind::TraitDecl(trait_decl) = &decl.kind {
                if decl.id == trait_id {
                    for method in &trait_decl.methods {
                        if method.name.name == method_name {
                            return method.default_implementation.is_some();
                        }
                    }
                }
            }
        }
        false
    }
    
    // メソッドのシグネチャが互換性を持つかチェック
    fn check_method_signature_compatibility(
        &self,
        trait_method_type: &TypeAnnotation,
        impl_method: &Function,
        location: Option<SourceLocation>,
    ) -> Result<()> {
        if let TypeKind::Function(trait_params, trait_return) = &trait_method_type.kind {
            // パラメータ数のチェック
            if trait_params.len() != impl_method.parameters.len() {
                return Err(CompilerError::type_error(
                    format!("メソッド実装のパラメータ数がトレイト定義と一致しません: {} != {}", 
                            impl_method.parameters.len(), trait_params.len()),
                    location,
                ));
            }
            
            // パラメータ型のチェック
            for (i, (trait_param, impl_param)) in trait_params.iter().zip(impl_method.parameters.iter()).enumerate() {
                let impl_param_type = self.resolve_type_annotation(&impl_param.type_annotation)?;
                if !self.is_compatible(trait_param, &impl_param_type) {
                    return Err(self.type_error(
                        format!("メソッド実装の第{}パラメータの型がトレイト定義と一致しません", i+1),
                        trait_param,
                        &impl_param_type,
                        impl_param.location.clone(),
                    ));
                }
            }
            
            // 戻り値型のチェック
            let impl_return_type = if let Some(rt) = &impl_method.return_type {
                self.resolve_type_annotation(rt)?
            } else {
                TypeAnnotation {
                    id: ast::generate_id(),
                    kind: TypeKind::Primitive(PrimitiveType::Void),
                    location: None,
                }
            };
            
            if !self.is_compatible(trait_return, &impl_return_type) {
                return Err(self.type_error(
                    "メソッド実装の戻り値型がトレイト定義と一致しません",
                    trait_return,
                    &impl_return_type,
                    impl_method.location.clone(),
                ));
            }
        }
        
        Ok(())
    }
    
    // 構造体型のメンバーアクセス
    fn check_member_access(&self, object: &Expression, member: &Expression) -> Result<TypeAnnotation> {
        let object_type = self.check_expression(object)?;
        
        match &object_type.kind {
            // 構造体型のメンバーアクセス
            TypeKind::Named(struct_name) => {
                // 構造体名から構造体IDを解決
                if let Some(struct_id) = self.name_resolution.resolved_nodes.get(&struct_name.id) {
                    // 構造体のフィールド情報を取得
                    if let Some(fields) = self.struct_fields.get(struct_id) {
                        // メンバー名に対応するフィールドを検索
                        if let Some(field_type) = fields.get(&member.name) {
                            return Ok(field_type.clone());
                        } else {
                            return Err(CompilerError::type_error(
                                format!("構造体 '{}' にフィールド '{}' は存在しません", 
                                        struct_name.name, member.name),
                                member.location.clone(),
                            ));
                        }
                    }
                }
                
                // シンボルが見つからない場合
                Err(CompilerError::type_error(
                    format!("型 '{}' のメンバー '{}' にアクセスできません", 
                            self.type_to_string(&object_type), member.name),
                    member.location.clone(),
                ))
            },
            
            // タプル型のメンバーアクセス（添字による）
            TypeKind::Tuple(element_types) => {
                // 数値インデックスの場合
                if let Ok(index) = member.name.parse::<usize>() {
                    if index < element_types.len() {
                        Ok(element_types[index].clone())
                    } else {
                        Err(CompilerError::type_error(
                            format!("タプルのインデックス {} は範囲外です（サイズ: {}）", 
                                    index, element_types.len()),
                            member.location.clone(),
                        ))
                    }
                } else {
                    Err(CompilerError::type_error(
                        format!("タプルのアクセスには数値インデックスが必要です: '{}'", member.name),
                        member.location.clone(),
                    ))
                }
            },
            
            // それ以外の型
            _ => {
                Err(CompilerError::type_error(
                    format!("型 '{}' はメンバーアクセスをサポートしていません", 
                            self.type_to_string(&object_type)),
                    member.location.clone(),
                ))
            }
        }
    }
    
    // インデックスアクセスの型チェック
    fn check_index_access(&self, array: &Expression, index: &Expression) -> Result<TypeAnnotation> {
        let array_type = self.check_expression(array)?;
        let index_type = self.check_expression(index)?;
        
        // インデックスが整数型であることを確認
        if !matches!(index_type.kind, TypeKind::Primitive(PrimitiveType::Int)) {
            return Err(self.type_error(
                "配列のインデックスは整数型である必要があります",
                &TypeAnnotation {
                    id: ast::generate_id(),
                    kind: TypeKind::Primitive(PrimitiveType::Int),
                    location: None,
                },
                &index_type,
                index.location.clone(),
            ));
        }
        
        // 配列型か文字列型かを確認
        match &array_type.kind {
            TypeKind::Array(element_type, _) => {
                // 配列の要素型を返す
                Ok(element_type.as_ref().clone())
            },
            TypeKind::Primitive(PrimitiveType::String) => {
                // 文字列のインデックスアクセスは文字を返す
                Ok(TypeAnnotation {
                    id: ast::generate_id(),
                    kind: TypeKind::Primitive(PrimitiveType::Char),
                    location: None,
                })
            },
            TypeKind::Tuple(element_types) => {
                // タプルのインデックスが定数の場合
                if let ExpressionKind::Literal(lit) = &index.kind {
                    if let LiteralKind::Integer(i) = &lit.kind {
                        let idx = i.parse::<usize>().unwrap_or(usize::MAX);
                        if idx < element_types.len() {
                            return Ok(element_types[idx].clone());
                        } else {
                            return Err(CompilerError::type_error(
                                format!("タプルのインデックス {} は範囲外です（サイズ: {}）", 
                                        idx, element_types.len()),
                                index.location.clone(),
                            ));
                        }
                    }
                }
                
                // 定数でない場合はコンパイル時に型を決定できない
                Err(CompilerError::type_error(
                    "タプルのインデックスは定数である必要があります",
                    index.location.clone(),
                ))
            },
            _ => {
                Err(CompilerError::type_error(
                    format!("型 '{}' はインデックスアクセスをサポートしていません", 
                            self.type_to_string(&array_type)),
                    index.location.clone(),
                ))
            }
        }
    }
    
    // 配列リテラルの型チェック
    fn check_array_literal(&self, elements: &[Expression]) -> Result<TypeAnnotation> {
        if elements.is_empty() {
            // 空の配列の場合、Any型の配列として扱う
            return Ok(TypeAnnotation {
                id: ast::generate_id(),
                kind: TypeKind::Array(Box::new(TypeAnnotation {
                    id: ast::generate_id(),
                    kind: TypeKind::Any,
                    location: None,
                }), Box::new(TypeAnnotation {
                    id: ast::generate_id(),
                    kind: TypeKind::Primitive(PrimitiveType::Int),
                    location: None,
                })),
                location: None,
            });
        }
        // 最初の要素の型をチェック
        let first_type = self.check_expression(&elements[0])?;
        
        // 残りの要素が同じ型かチェック
        for (i, elem) in elements.iter().skip(1).enumerate() {
            let elem_type = self.check_expression(elem)?;
            if !self.is_compatible(&first_type, &elem_type) {
                return Err(self.type_error(
                    format!("配列の要素 {} の型が一致しません", i+1),
                    &first_type,
                    &elem_type,
                    elem.location.clone(),
                ));
            }
        }
        
        // 配列型を返す
        Ok(TypeAnnotation {
            id: ast::generate_id(),
            kind: TypeKind::Array(Box::new(first_type), Box::new(TypeAnnotation {
                id: ast::generate_id(),
                kind: TypeKind::Primitive(PrimitiveType::Int),
                location: None,
            })),
            location: None,
        })
    }
    // 構造体リテラルの型チェック
    fn check_struct_literal(&self, name: &Identifier, fields: &[(Identifier, Expression)]) -> Result<TypeAnnotation> {
        // 構造体名を解決
        if let Some(struct_id) = self.name_resolution.resolved_nodes.get(&name.id) {
            // 構造体のフィールド情報を取得
            if let Some(struct_fields) = self.struct_fields.get(struct_id) {
                // 与えられたフィールドが構造体の定義と一致するかチェック
                let mut provided_fields = HashSet::new();
                
                for (field_name, field_value) in fields {
                    let field_name_str = &field_name.name;
                    
                    // フィールド名が構造体に存在するかチェック
                    if let Some(expected_type) = struct_fields.get(field_name_str) {
                        // フィールド値の型をチェック
                        let actual_type = self.check_expression(field_value)?;
                        if !self.is_compatible(expected_type, &actual_type) {
                            return Err(self.type_error(
                                format!("フィールド '{}' の型が一致しません", field_name_str),
                                expected_type,
                                &actual_type,
                                field_value.location.clone(),
                            ));
                        }
                    } else {
                        return Err(CompilerError::type_error(
                            format!("構造体 '{}' にフィールド '{}' は存在しません", 
                                    name.name, field_name_str),
                            field_name.location.clone(),
                        ));
                    }
                    
                    // 重複フィールドのチェック
                    if !provided_fields.insert(field_name_str.clone()) {
                        return Err(CompilerError::type_error(
                            format!("フィールド '{}' が重複して指定されています", field_name_str),
                            field_name.location.clone(),
                        ));
                    }
                }
                
                // 必須フィールドがすべて提供されているかチェック
                for (field_name, field_type) in struct_fields {
                    if !provided_fields.contains(field_name) {
                        // 構造体定義からフィールドのデフォルト値情報を取得
                        let struct_def = self.get_struct_definition(*struct_id)?;
                        let has_default = struct_def.fields.iter()
                            .find(|f| f.name.name == *field_name)
                            .map(|f| f.default_value.is_some())
                            .unwrap_or(false);
                        
                        // オプショナル型かどうかをチェック
                        let is_optional = match &field_type.kind {
                            TypeKind::Optional(_) => true,
                            // 依存型の場合、条件によってはオプショナルになる可能性がある
                            TypeKind::Dependent(base_type, condition) => {
                                self.evaluate_dependent_type_optionality(base_type, condition)?
                            },
                            _ => false
                        };
                        
                        // コンパイル時定数式の評価によるデフォルト値の存在チェック
                        let has_compile_time_default = if !has_default {
                            self.check_compile_time_default_for_field(*struct_id, field_name)?
                        } else {
                            false
                        };
                        
                        // デフォルト値がなく、オプショナル型でもない場合はエラー
                        if !has_default && !is_optional && !has_compile_time_default {
                            // フィールドの重要度を取得（ドキュメンテーションコメントから解析）
                            let field_importance = self.get_field_importance(*struct_id, field_name);
                            
                            // 重要なフィールドの場合はエラーレベルを上げる
                            let error_message = match field_importance {
                                FieldImportance::Critical => 
                                    format!("重大: 構造体 '{}' の必須フィールド '{}' が指定されていません。このフィールドは正常な動作に不可欠です。", 
                                            name.name, field_name),
                                FieldImportance::High => 
                                    format!("構造体 '{}' の重要フィールド '{}' が指定されていません。このフィールドは推奨されています。", 
                                            name.name, field_name),
                                _ => 
                                    format!("構造体 '{}' のフィールド '{}' が指定されていません", 
                                            name.name, field_name)
                            };
                            
                            // 型情報を含めたエラーメッセージを生成
                            let detailed_error = format!("{}。期待される型: {}", 
                                                        error_message, 
                                                        self.type_to_string(field_type));
                            
                            // 可能な修正候補を提案
                            let suggestions = self.generate_field_suggestions(*struct_id, field_name);
                            let error_with_suggestions = if !suggestions.is_empty() {
                                format!("{}。推奨される値: {}", detailed_error, suggestions.join(", "))
                            } else {
                                detailed_error
                            };
                            
                            return Err(CompilerError::type_error_with_help(
                                error_with_suggestions,
                                name.location.clone(),
                                format!("フィールド '{}' を追加するか、構造体定義でデフォルト値を設定してください", field_name)
                            ));
                        }
                    }
                }
                
                // 構造体の不変条件（invariant）をチェック
                if let Some(invariants) = self.get_struct_invariants(*struct_id) {
                    for invariant in invariants {
                        if !self.check_invariant(invariant, fields, struct_fields)? {
                            return Err(CompilerError::type_error(
                                format!("構造体 '{}' の不変条件に違反しています: {}", 
                                        name.name, invariant.description),
                                name.location.clone(),
                            ));
                        }
                    }
                }
                
                // フィールド間の依存関係をチェック
                self.check_field_dependencies(*struct_id, fields)?;
                
                // 型レベルの計算を実行（依存型の場合）
                if self.has_dependent_types(*struct_id) {
                    self.evaluate_dependent_types(*struct_id, fields)?;
                }
                // 構造体型を返す
                return Ok(TypeAnnotation {
                    id: ast::generate_id(),
                    kind: TypeKind::Named(name.clone()),
                    location: None,
                });
            }
        }
        
        // 構造体が見つからない場合
        Err(CompilerError::type_error(
            format!("構造体 '{}' が見つかりません", name.name),
            name.location.clone(),
        ))
    }
    
    // タプルリテラルの型チェック
    fn check_tuple_literal(&self, elements: &[Expression]) -> Result<TypeAnnotation> {
        let mut element_types = Vec::with_capacity(elements.len());
        
        // 各要素の型をチェック
        for elem in elements {
            let elem_type = self.check_expression(elem)?;
            element_types.push(elem_type);
        }
        
        // タプル型を返す
        Ok(TypeAnnotation {
            id: ast::generate_id(),
            kind: TypeKind::Tuple(element_types),
            location: None,
        })
    }
    
    // 型キャストの型チェック
    fn check_cast(&mut self, expr: &Expression, target_type: &TypeAnnotation) -> Result<TypeAnnotation> {
        let expr_type = self.check_expression(expr)?;
        let resolved_target = self.resolve_type_annotation(target_type)?;
        
        // キャストの安全性をチェック
        let is_safe = match (self.options.cast_safety_level, &expr_type.kind, &resolved_target.kind) {
            // 同じ型へのキャスト（冗長）
            (_, _, _) if expr_type.kind == resolved_target.kind => true,
            
            // 数値型間のキャスト
            (_, TypeKind::Primitive(PrimitiveType::Int), TypeKind::Primitive(PrimitiveType::Float)) => true,
            (_, TypeKind::Primitive(PrimitiveType::Float), TypeKind::Primitive(PrimitiveType::Int)) => true,
            
            // 厳格モードでない場合に許可されるキャスト
            (CastSafetyLevel::Standard | CastSafetyLevel::Relaxed, TypeKind::Primitive(PrimitiveType::Int), TypeKind::Primitive(PrimitiveType::Char)) => true,
            (CastSafetyLevel::Standard | CastSafetyLevel::Relaxed, TypeKind::Primitive(PrimitiveType::Char), TypeKind::Primitive(PrimitiveType::Int)) => true,
            
            // 緩和モードでのみ許可されるキャスト
            (CastSafetyLevel::Relaxed, TypeKind::Any, _) => true,
            (CastSafetyLevel::Relaxed, _, TypeKind::Any) => true,
            (CastSafetyLevel::Relaxed, TypeKind::Nil, TypeKind::Optional(_)) => true,
            
            // その他のキャスト（許可されない）
            _ => false,
        };
        if !is_safe {
            return Err(CompilerError::type_error(
                format!("不安全なキャストです: {} から {} へ", 
                        self.type_to_string(&expr_type), 
                        self.type_to_string(&resolved_target)),
                expr.location.clone(),
            ));
        }
        
        // 冗長なキャストを警告
        if expr_type.kind == resolved_target.kind && self.options.warn_redundant_casts {
            self.result.add_warning(CompilerError::warning(
                format!("冗長なキャストです: {} から {} へ", 
                        self.type_to_string(&expr_type), 
                        self.type_to_string(&resolved_target)),
                expr.location.clone(),
            ));
        }
        
        // キャスト操作を記録
        self.result.record_cast(expr.id, expr_type, resolved_target.clone());
        
        // キャスト後の型を返す
        Ok(resolved_target)
    }
    
    // ラムダ式の型チェック
    fn check_lambda(&mut self, params: &[Parameter], body: &Statement) -> Result<TypeAnnotation> {
        // 現在の関数コンテキストを退避
        let prev_context = self.context.clone();
        self.context.scope_id += 1; // 新しいスコープに入る
        
        // パラメータの型をチェック
        let mut param_types = Vec::with_capacity(params.len());
        for param in params {
            let param_type = self.resolve_type_annotation(&param.type_annotation)?;
            param_types.push(param_type);
        }
        
        // ラムダ本体をチェック
        let body_type = if let StatementKind::Block(stmts) = &body.kind {
            let mut last_expr_type = TypeAnnotation {
                id: ast::generate_id(),
                kind: TypeKind::Primitive(PrimitiveType::Void),
                location: None,
            };
            
            for stmt in stmts {
                // 式文の場合は型を記録
                if let StatementKind::ExpressionStmt(expr) = &stmt.kind {
                    last_expr_type = self.check_expression(expr)?;
                } else {
                    self.check_statement(stmt)?;
                }
            }
            
            last_expr_type
        } else {
            self.check_statement(body)?;
            
            // 単一文の場合はVoid型を返す
            TypeAnnotation {
                id: ast::generate_id(),
                kind: TypeKind::Primitive(PrimitiveType::Void),
                location: None,
            }
        };
        
        // コンテキストを復元
        self.context = prev_context;
        
        // 関数型を返す
        Ok(TypeAnnotation {
            id: ast::generate_id(),
            kind: TypeKind::Function(param_types, Some(Box::new(body_type))),
            location: None,
        })
    }
    
    // ブロック式の型チェック
    fn check_block_expr(&mut self, statements: &[Statement], final_expr: Option<&Expression>) -> Result<TypeAnnotation> {
        // 現在のスコープを退避
        let prev_scope_id = self.context.scope_id;
        self.context.scope_id += 1; // 新しいスコープに入る
        
        // 各文をチェック
        for stmt in statements {
            self.check_statement(stmt)?;
        }
        
        // 最終式があれば、その型を返す
        let result_type = if let Some(expr) = final_expr {
            self.check_expression(expr)?
        } else {
            // 最終式がなければVoid型
            TypeAnnotation {
                id: ast::generate_id(),
                kind: TypeKind::Primitive(PrimitiveType::Void),
                location: None,
            }
        };
        
        // スコープを復元
        self.context.scope_id = prev_scope_id;
        
        Ok(result_type)
    }
    
    // if式の型チェック
    fn check_if_expr(&mut self, cond: &Expression, then_branch: &Statement, else_branch: Option<&Statement>) -> Result<TypeAnnotation> {
        // 条件式の型をチェック
        let cond_type = self.check_expression(cond)?;
        
        // 条件式はブール型であるべき
        if !matches!(cond_type.kind, TypeKind::Primitive(PrimitiveType::Bool)) {
            return Err(self.type_error(
                "if式の条件部はブール型である必要があります",
                &TypeAnnotation {
                    id: ast::generate_id(),
                    kind: TypeKind::Primitive(PrimitiveType::Bool),
                    location: None,
                },
                &cond_type,
                cond.location.clone(),
            ));
        }
        
        // then部の型をチェック
        let then_type = self.check_block_expr(&[then_branch.clone()], None)?;
        
        // else部がある場合はその型をチェック
        let else_type = if let Some(else_stmt) = else_branch {
            self.check_block_expr(&[else_stmt.clone()], None)?
        } else {
            // else部がない場合はVoid型
            TypeAnnotation {
                id: ast::generate_id(),
                kind: TypeKind::Primitive(PrimitiveType::Void),
                location: None,
            }
        };
        
        // 共通の型を求める
        self.common_type(&then_type, &else_type)
    }
    
    // match式の型チェック
    fn check_match_expr(&mut self, expr: &Expression, arms: &[MatchArm]) -> Result<TypeAnnotation> {
        // マッチ対象の式の型をチェック
        let expr_type = self.check_expression(expr)?;
        
        if arms.is_empty() {
            return Err(CompilerError::type_error(
                "match式には少なくとも1つの分岐が必要です",
                expr.location.clone(),
            ));
        }
        
        // 各分岐の型をチェック
        let mut arm_types = Vec::with_capacity(arms.len());
        
        for arm in arms {
            // パターンと対象式の型の互換性をチェック
            // frontend/parser/ast.rsのMatchArmは.patternフィールドを持ちますが、Spanを持つため
            // location()メソッドの代わりにspanフィールドを使用
            let pattern_location = expr.location.clone(); // patternの位置情報がないため代替として式の位置情報を使用
            self.check_pattern_compatibility(&arm.pattern, &expr_type, pattern_location)?;
            
            // 分岐の本体の型をチェック
            // frontend/parser/ast.rsのMatchArmは.bodyフィールドの代わりに.expressionフィールドを持つ
            let arm_type = self.check_expression(&arm.expression)?;
            arm_types.push(arm_type);
        }
        
        // すべての分岐の型が互換性を持つ共通の型を求める
        self.common_type(&arm_types[0], &arm_types[1])
    }
    
    // 数値型かどうかを判定
    fn is_numeric_type(&self, typ: &TypeAnnotation) -> bool {
        matches!(typ.kind, TypeKind::Primitive(PrimitiveType::Int) | TypeKind::Primitive(PrimitiveType::Float))
    }
    
    // 二つの型が比較可能かどうかを判定
    fn is_comparable(&self, type1: &TypeAnnotation, type2: &TypeAnnotation) -> bool {
        // 同じ型なら比較可能
        if self.is_compatible(type1, type2) || self.is_compatible(type2, type1) {
            return true;
        }
        
        // 数値型同士は比較可能
        if self.is_numeric_type(type1) && self.is_numeric_type(type2) {
            return true;
        }
        
        // その他の特殊ケース
        match (&type1.kind, &type2.kind) {
            (TypeKind::Optional(inner1), TypeKind::Nil) |
            (TypeKind::Nil, TypeKind::Optional(inner1)) => true,
            _ => false,
        }
    }
    
    // 数値型の共通型を求める（Int と Float の場合は Float を優先）
    fn common_numeric_type(&self, type1: &TypeAnnotation, type2: &TypeAnnotation) -> Result<TypeAnnotation> {
        if matches!(type1.kind, TypeKind::Primitive(PrimitiveType::Float)) || matches!(type2.kind, TypeKind::Primitive(PrimitiveType::Float)) {
            Ok(TypeAnnotation {
                id: ast::generate_id(),
                kind: TypeKind::Primitive(PrimitiveType::Float),
                location: None,
            })
        } else {
            Ok(TypeAnnotation {
                id: ast::generate_id(),
                kind: TypeKind::Primitive(PrimitiveType::Int),
                location: None,
            })
        }
    }
    
    // パターンと式の型の互換性をチェック
    fn check_pattern_compatibility(&self, pattern: &Expression, expr_type: &TypeAnnotation, location: Option<SourceLocation>) -> Result<()> {
        match &pattern.kind {
            // リテラルパターン
            ExpressionKind::Literal(lit) => {
                let lit_type = self.check_literal(lit)?;
                Ok(if !self.is_compatible(expr_type, &lit_type) {
                    return Err(self.type_error(
                        "パターンの型がマッチ対象の式の型と互換性がありません",
                        expr_type,
                        &lit_type,
                        location,
                    ));
                })
            },
            
            // 識別子パターン（変数バインディング）
            ExpressionKind::Identifier(ident) => {
                // 変数バインディングは基本的に任意の型とマッチするが、
                // 型アノテーションがある場合は互換性をチェックする
                if let Some(symbol) = self.symbol_table.lookup(&ident.name) {
                    if let Some(symbol_type) = &symbol.type_annotation {
                        if !self.is_compatible(expr_type, symbol_type) && !self.is_compatible(symbol_type, expr_type) {
                            return Err(self.type_error(
                                "変数の型アノテーションがマッチ対象の式の型と互換性がありません",
                                expr_type,
                                symbol_type,
                                location,
                            ));
                        }
                    }
                }
                
                // 変数の型情報を環境に追加または更新
                self.update_variable_type(&ident.name, expr_type.clone())?;
                return Ok(());
            },
            
            // タプルパターン
            ExpressionKind::TupleLiteral(elements) => {
                if let TypeKind::Tuple(type_elements) = &expr_type.kind {
                    // 要素数が一致するか確認
                    if elements.len() != type_elements.len() {
                        return Err(CompilerError::type_error(
                            format!("タプルパターンの要素数 {} がマッチ対象のタプル型の要素数 {} と一致しません",
                                    elements.len(), type_elements.len()),
                            location,
                        ));
                    }
                    
                    // 各要素のパターンマッチングを再帰的にチェック
                    for (i, (pattern_elem, type_elem)) in elements.iter().zip(type_elements.iter()).enumerate() {
                        self.check_pattern_compatibility(pattern_elem, type_elem, pattern_elem.location.clone())?;
                    }
                    
                    return Ok(());
                } else {
                    return Err(CompilerError::type_error(
                        format!("タプルパターンに対して型 '{}' とマッチできません",
                                self.type_to_string(expr_type)),
                        location,
                    ));
                }
            },
            
            // 配列パターン
            ExpressionKind::ArrayLiteral(elements) => {
                if let TypeKind::Array(elem_type, _) = &expr_type.kind {
                    // 各要素のパターンマッチングを再帰的にチェック
                    for (i, pattern_elem) in elements.iter().enumerate() {
                        self.check_pattern_compatibility(pattern_elem, elem_type, pattern_elem.location.clone())?;
                    }
                    
                    return Ok(());
                } else {
                    return Err(CompilerError::type_error(
                        format!("配列パターンに対して型 '{}' とマッチできません",
                                self.type_to_string(expr_type)),
                        location,
                    ));
                }
            },
            
            // レンジパターン
            ExpressionKind::Range(start, end) => {
                // レンジパターンは整数型または文字型とのみマッチ可能
                if !matches!(expr_type.kind, TypeKind::Primitive(PrimitiveType::Int) | TypeKind::Primitive(PrimitiveType::Char)) {
                    return Err(CompilerError::type_error(
                        format!("レンジパターンは整数型または文字型とのみマッチ可能ですが、型 '{}' が指定されました",
                                self.type_to_string(expr_type)),
                        location,
                    ));
                }
                
                // 開始値と終了値の型チェック
                if let Some(start_expr) = start {
                    let start_type = self.check_expression(start_expr)?;
                    if !self.is_compatible(expr_type, &start_type) {
                        return Err(self.type_error(
                            "レンジの開始値の型がマッチ対象の型と互換性がありません",
                            expr_type,
                            &start_type,
                            start_expr.location.clone(),
                        ));
                    }
                }
                
                if let Some(end_expr) = end {
                    let end_type = self.check_expression(end_expr)?;
                    if !self.is_compatible(expr_type, &end_type) {
                        return Err(self.type_error(
                            "レンジの終了値の型がマッチ対象の型と互換性がありません",
                            expr_type,
                            &end_type,
                            end_expr.location.clone(),
                        ));
                    }
                }
                
                return Ok(());
            },
            
            // ワイルドカードパターン
            ExpressionKind::Wildcard => {
                // ワイルドカードは任意の型とマッチする
                return Ok(());
            },
            
            // 構造体パターン
            ExpressionKind::StructLiteral(name, fields) => {
                // 対象が構造体型であることを確認
                if let TypeKind::Named(struct_name) = &expr_type.kind {
                    // 構造体名が一致するか確認
                    if name.name != struct_name.name {
                        return Err(CompilerError::type_error(
                            format!("パターンの構造体名 '{}' がマッチ対象の型 '{}' と一致しません",
                                    name.name, struct_name.name),
                            location,
                        ));
                    }
                    
                    // 構造体の定義を取得
                    let struct_def = match self.get_struct_definition(&name.name) {
                        Some(def) => def,
                        None => return Err(CompilerError::type_error(
                            format!("構造体 '{}' の定義が見つかりません", name.name),
                            location,
                        )),
                    };
                    
                    // フィールドの存在確認とパターンマッチング
                    let mut matched_fields = std::collections::HashSet::new();
                    
                    for (field_name, pattern) in fields {
                        // フィールドが構造体に存在するか確認
                        let field_type = match struct_def.fields.iter().find(|f| f.name == field_name.name) {
                            Some(field) => &field.type_annotation,
                            None => return Err(CompilerError::type_error(
                                format!("フィールド '{}' は構造体 '{}' に存在しません", 
                                        field_name.name, name.name),
                                field_name.location.clone(),
                            )),
                        };
                        
                        // フィールドのパターンマッチングを再帰的にチェック
                        self.check_pattern_compatibility(pattern, field_type, pattern.location.clone())?;
                        
                        // 同じフィールドが複数回指定されていないか確認
                        if !matched_fields.insert(field_name.name.clone()) {
                            return Err(CompilerError::type_error(
                                format!("フィールド '{}' が複数回指定されています", field_name.name),
                                field_name.location.clone(),
                            ));
                        }
                    }
                    
                    // 必須フィールドがすべて指定されているか確認（非オプショナルフィールド）
                    for field in &struct_def.fields {
                        if !matched_fields.contains(&field.name) && 
                           !matches!(field.type_annotation.kind, TypeKind::Optional(_)) {
                            // デフォルト値があるフィールドはスキップ可能
                            if field.default_value.is_none() {
                                return Err(CompilerError::type_error(
                                    format!("必須フィールド '{}' がパターンで指定されていません", field.name),
                                    location,
                                ));
                            }
                        }
                    }
                    
                    return Ok(());
                } else {
                    return Err(CompilerError::type_error(
                        format!("構造体パターンに対して型 '{}' とマッチできません",
                                self.type_to_string(expr_type)),
                        location,
                    ));
                }
            },
            
            // それ以外のパターン
            _ => {
                Err(CompilerError::type_error(
                    "サポートされていないパターン形式です",
                    location,
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::ast::{self, Statement, StatementKind};
    use crate::frontend::parser::Parser;
    use crate::frontend::lexer::Lexer;
    
    // テスト用ヘルパー関数：ソースからASTを構築
    fn parse_source(source: &str) -> Result<Program> {
        let tokens = Lexer::new(source, "test.sl").collect::<Result<Vec<_>>>()?;
        let mut parser = Parser::new(&tokens, "test.sl");
        parser.parse_program()
    }
    
    // 型チェッカーを準備
    fn setup_type_checker(program: &Program) -> TypeChecker {
        // 名前解決結果とシンボルテーブルは実際のテストでは適切に構築する必要がある
        let name_resolver = NameResolver::new();
        let name_resolution = name_resolver.resolve_program(program).unwrap();
        
        TypeChecker::new(name_resolution, name_resolver.scope_manager.symbol_table)
    }
    
    #[test]
    fn test_simple_variable_declaration() {
        let source = "let x: Int = 42;";
        let program = parse_source(source).unwrap();
        let mut type_checker = setup_type_checker(&program);
        
        let result = type_checker.check_program(&program).unwrap();
        assert!(!result.has_errors());
    }
    
    #[test]
    fn test_incompatible_type_error() {
        let source = "let x: String = 42;";
        let program = parse_source(source).unwrap();
        let mut type_checker = setup_type_checker(&program);
        
        let result = type_checker.check_program(&program).unwrap();
        assert!(result.has_errors());
    }
    
    #[test]
    fn test_function_declaration() {
        let source = "func add(a: Int, b: Int) -> Int { return a + b; }";
        let program = parse_source(source).unwrap();
        let mut type_checker = setup_type_checker(&program);
        
        let result = type_checker.check_program(&program).unwrap();
        assert!(!result.has_errors());
    }
    
    #[test]
    fn test_function_return_type_mismatch() {
        let source = "func add(a: Int, b: Int) -> String { return a + b; }";
        let program = parse_source(source).unwrap();
        let mut type_checker = setup_type_checker(&program);
        
        let result = type_checker.check_program(&program).unwrap();
        assert!(result.has_errors());
    }
    
    #[test]
    fn test_struct_declaration() {
        let source = "struct Point { x: Int, y: Int }";
        let program = parse_source(source).unwrap();
        let mut type_checker = setup_type_checker(&program);
        
        let result = type_checker.check_program(&program).unwrap();
        assert!(!result.has_errors());
    }
    
    #[test]
    fn test_duplicate_field_error() {
        let source = "struct Point { x: Int, x: Int }";
        let program = parse_source(source).unwrap();
        let mut type_checker = setup_type_checker(&program);
        
        let result = type_checker.check_program(&program).unwrap();
        assert!(result.has_errors());
    }
}

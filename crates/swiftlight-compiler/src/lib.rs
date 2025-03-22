// SwiftLight Compiler Library
// 言語コンパイラのメインライブラリ

#![allow(unused_imports, unused_variables, dead_code, unused_macros)]
#![allow(unexpected_cfgs, hidden_glob_reexports, ambiguous_glob_reexports)]

//! # SwiftLight Compiler
//! 
//! SwiftLight言語のコンパイラライブラリです。
//! このライブラリは、SwiftLight言語のソースコードを解析し、
//! 中間表現(IR)を経由してLLVMバックエンドを通じて
//! ネイティブコードまたはその他のターゲットにコンパイルします。
//! 
//! SwiftLightは安全性、効率性、表現力、開発体験の全てにおいて最高水準を目指す
//! 次世代プログラミング言語です。メモリ安全性、並行処理、メタプログラミング、
//! コンパイル時計算などの高度な機能を極限まで追求します。

// 標準ライブラリのインポート
use std::error::Error;
use std::path::{Path, PathBuf};
use std::fs;
use std::sync::Arc;
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};
use std::io::{self, Read, Write};
use std::thread;
use std::fmt;

// 外部クレートのインポート
use log::{debug, error, info, trace, warn};
// use rayon::prelude::*;
// use serde::{Serialize, Deserialize};
use thiserror::Error;
// use parking_lot::{RwLock, Mutex as PLMutex};
// use dashmap::DashMap;
// use once_cell::sync::Lazy;
// use smallvec::{smallvec, SmallVec};
// use indexmap::{IndexMap, IndexSet};

// 内部モジュールの宣言
pub mod frontend;
pub mod middleend;
pub mod backend;
pub mod driver;
pub mod utils;
pub mod diagnostics;
pub mod config;
pub mod optimization;
pub mod typesystem;

// 再エクスポート
pub use self::driver::Driver;
pub use self::frontend::ast;
pub use self::frontend::parser;
pub use self::frontend::lexer;
pub use self::frontend::error::{CompilerError, ErrorKind, Result};
pub use self::driver::diagnostics::{Diagnostic, DiagnosticLevel};
pub use self::config::CompilerConfig;
pub use self::typesystem::{Type, TypeId, TypeRegistry};
pub use self::diagnostics::DiagnosticEmitter;

// utils モジュールからの再エクスポート
pub use self::utils::error_handling::{BasicErrorHandler, ErrorHandler, CompilerError as UtilsError};
pub use self::utils::logger::{Logger, LogLevel, LogEntry, ConsoleLogger, FileLogger, CompositeLogger};
pub use self::utils::hash::{ContentHasher, HashAlgorithm};
pub use self::utils::memory_tracker::{MemoryTracker, MemoryUsageSnapshot};
pub use self::utils::parallel::{ThreadPool, Task, TaskPriority, WorkQueue};
pub use self::utils::file_system::{FileSystem, VirtualFileSystem, FileWatcher};
pub use self::utils::config_parser::ConfigParser;

// コンパイラの構成要素の型エイリアス
pub type SymbolTable = frontend::semantic::symbol_table::SymbolTable;
pub type TypeRegistry = frontend::semantic::type_registry::TypeRegistry;
pub type DiagnosticEmitter = diagnostics::DiagnosticEmitter;
pub type Module = frontend::module::Module;

/// コンパイラのバージョン
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// コンパイラの構築日時
// pub const BUILD_DATE: &str = env!("BUILD_DATE", "unknown");
pub const BUILD_DATE: &str = "2023-03-17"; // 一時的に固定値を使用

/// コンパイラのGitコミットハッシュ
// pub const GIT_COMMIT_HASH: &str = env!("GIT_COMMIT_HASH", "unknown");
pub const GIT_COMMIT_HASH: &str = "abcdef123456789"; // 一時的に固定値を使用

/// コンパイラのデフォルト最適化レベル
pub const DEFAULT_OPTIMIZATION_LEVEL: optimization::OptimizationLevel = optimization::OptimizationLevel::O2;

/// グローバルコンパイラインスタンス
// static GLOBAL_COMPILER: Lazy<Arc<RwLock<Compiler>>> = Lazy::new(|| {
//     Arc::new(RwLock::new(Compiler::new(CompilerConfig::default())))
// });

/// コンパイラのメインクラス
#[derive(Debug)]
pub struct Compiler {
    /// コンパイラの設定
    config: CompilerConfig,
    
    /// 型レジストリ
    // type_registry: Arc<RwLock<TypeRegistry>>,
    type_registry: Arc<std::sync::RwLock<TypeRegistry>>,
    
    /// 診断情報エミッタ
    // diagnostic_emitter: Arc<RwLock<DiagnosticEmitter>>,
    diagnostic_emitter: Arc<std::sync::RwLock<DiagnosticEmitter>>,
    
    /// モジュールレジストリ
    // modules: Arc<DashMap<String, Arc<RwLock<frontend::module::Module>>>>,
    modules: Arc<std::collections::HashMap<String, Arc<std::sync::RwLock<frontend::module::Module>>>>,
}

/// コンパイル統計情報
#[derive(Debug, Default, Clone)]
pub struct CompilationStats {
    /// コンパイル開始時間
    pub start_time: Option<std::time::SystemTime>,
    
    /// コンパイル終了時間
    pub end_time: Option<std::time::SystemTime>,
    
    /// 処理されたファイル数
    pub files_processed: usize,
    
    /// 処理された行数
    pub lines_processed: usize,
    
    /// 検出されたエラー数
    pub errors_count: usize,
    
    /// 検出された警告数
    pub warnings_count: usize,
    
    /// 各フェーズの実行時間
    pub phase_timings: HashMap<String, Duration>,
    
    /// メモリ使用量（ピーク）
    pub peak_memory_usage: usize,
    
    /// スレッド使用数
    pub threads_used: usize,
    
    /// キャッシュヒット率
    pub cache_hit_ratio: f64,
}

impl Compiler {
    /// 新しいコンパイラインスタンスを作成
    pub fn new(config: CompilerConfig) -> Self {
        Self {
            config,
            type_registry: Arc::new(std::sync::RwLock::new(TypeRegistry::new())),
            diagnostic_emitter: Arc::new(std::sync::RwLock::new(DiagnosticEmitter::new())),
            modules: Arc::new(std::collections::HashMap::new()),
        }
    }
    
    /// グローバルコンパイラインスタンスを取得
    pub fn global() -> Arc<std::sync::RwLock<Self>> {
        // GLOBAL_COMPILER.clone()
        Arc::new(std::sync::RwLock::new(Self::new(CompilerConfig::default())))
    }
    
    /// コンパイラの設定を取得
    pub fn config(&self) -> &CompilerConfig {
        &self.config
    }
    
    /// 型レジストリを取得
    pub fn type_registry(&self) -> Arc<std::sync::RwLock<TypeRegistry>> {
        self.type_registry.clone()
    }
    
    /// 診断情報エミッタを取得
    pub fn diagnostic_emitter(&self) -> Arc<std::sync::RwLock<DiagnosticEmitter>> {
        self.diagnostic_emitter.clone()
    }
    
    /// モジュールレジストリを取得
    pub fn modules(&self) -> Arc<std::collections::HashMap<String, Arc<std::sync::RwLock<frontend::module::Module>>>> {
        self.modules.clone()
    }
}

/// SwiftLightのコンパイル処理を実行する主要関数
/// 
/// # 引数
/// 
/// * `input_path` - ソースファイルまたはディレクトリのパス
/// * `output_path` - 出力先のパス
/// * `options` - コンパイルオプション
/// 
/// # 戻り値
/// 
/// * `Result<(), Box<dyn Error>>` - 成功時は`()`、失敗時はエラー
pub fn compile<P: AsRef<Path>>(
    input_path: P,
    output_path: P,
    options: driver::CompileOptions,
) -> std::result::Result<(), Box<dyn Error>> {
    // コンパイラインスタンスの取得
    let compiler = Compiler::global();
    let compiler = compiler.write();
    
    // ドライバーの作成と実行
    let driver = Driver::new(options);
    let result = driver.compile(input_path, output_path);
    
    // 結果を返す
    result
}

/// ソースコードから抽象構文木(AST)を生成する
/// 
/// # 引数
/// 
/// * `source` - SwiftLightのソースコード
/// * `file_name` - ソースファイル名（エラーメッセージに使用）
/// * `config` - パース設定（オプション）
/// 
/// # 戻り値
/// 
/// * `Result<ast::Program>` - 成功時はAST、失敗時はエラー
pub fn parse_source(
    source: &str, 
    file_name: &str,
    config: Option<frontend::parser::ParserConfig>,
) -> frontend::error::Result<frontend::ast::Program> {
    // レキサーの作成
    let lexer = frontend::lexer::Lexer::new(source, file_name);
    
    // パーサーの作成と実行
    let parser_config = config.unwrap_or_default();
    let mut parser = frontend::parser::Parser::new_with_config(lexer, parser_config);
    
    // プログラムのパース
    parser.parse_program()
}

/// ソースファイルから抽象構文木(AST)を生成する
/// 
/// # 引数
/// 
/// * `file_path` - ソースファイルのパス
/// * `config` - パース設定（オプション）
/// 
/// # 戻り値
/// 
/// * `Result<ast::Program>` - 成功時はAST、失敗時はエラー
pub fn parse_file<P: AsRef<Path>>(
    file_path: P,
    config: Option<frontend::parser::ParserConfig>,
) -> frontend::error::Result<frontend::ast::Program> {
    let path = file_path.as_ref();
    
    // ファイルの存在確認
    if !path.exists() {
        return Err(frontend::error::CompilerError::new(
            frontend::error::ErrorKind::IO,
            format!("ファイルが存在しません: {}", path.display()),
            None
        ));
    }
    
    // ファイルの読み込み
    let source = fs::read_to_string(path)
        .map_err(|e| frontend::error::CompilerError::new(
            frontend::error::ErrorKind::IO,
            format!("ファイル読み込みエラー: {}", e),
            None
        ))?;
    
    // ソースからASTを生成
    parse_source(&source, path.to_string_lossy().as_ref(), config)
}

/// 複数のソースファイルを並列処理でパースする
/// 
/// # 引数
/// 
/// * `file_paths` - ソースファイルのパスのリスト
/// * `config` - パース設定（オプション）
/// 
/// # 戻り値
/// 
/// * `Result<Vec<(PathBuf, ast::Program)>>` - 成功時はパスとASTのペアのベクター、失敗時はエラー
pub fn parse_files<P: AsRef<Path> + Send + Sync>(
    file_paths: &[P],
    config: Option<frontend::parser::ParserConfig>,
) -> frontend::error::Result<Vec<(PathBuf, frontend::ast::Program)>> {
    // 並列処理でファイルをパース（rayonがないため、順次処理に変更）
    let results: Vec<Result<(PathBuf, frontend::ast::Program), frontend::error::CompilerError>> = 
        file_paths.iter() // par_iterからiterに変更
            .map(|path| {
                let path_buf = path.as_ref().to_path_buf();
                parse_file(path, config.clone())
                    .map(|program| (path_buf, program))
            })
            .collect();
    
    // エラーがあれば最初のエラーを返す
    let mut errors = Vec::new();
    let mut successes = Vec::new();
    
    for result in results {
        match result {
            Ok(success) => successes.push(success),
            Err(error) => errors.push(error),
        }
    }
    
    if !errors.is_empty() {
        return Err(errors.remove(0));
    }
    
    Ok(successes)
}

/// ソースコードをコンパイルして実行可能ファイルを生成する
/// 
/// # 引数
/// 
/// * `source` - SwiftLightのソースコード
/// * `file_name` - ソースファイル名（エラーメッセージに使用）
/// * `output_path` - 出力先のパス
/// * `options` - コンパイルオプション
/// 
/// # 戻り値
/// 
/// * `Result<(), Box<dyn Error>>` - 成功時は`()`、失敗時はエラー
pub fn compile_source<P: AsRef<Path>>(
    source: &str,
    file_name: &str,
    output_path: P,
    options: driver::CompileOptions,
) -> std::result::Result<(), Box<dyn Error>> {
    // 一時ファイルの作成
    let temp_dir = tempfile::tempdir()?;
    let temp_file_path = temp_dir.path().join(file_name);
    
    // ソースコードを一時ファイルに書き込み
    fs::write(&temp_file_path, source)?;
    
    // コンパイル実行
    let result = compile(temp_file_path, output_path, options);
    
    // 一時ディレクトリは自動的に削除される
    result
}

/// コンパイラのテスト用ヘルパー関数
#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::frontend::semantic::{
        name_resolver::NameResolver,
        type_checker::TypeChecker,
    };
    use crate::frontend::error::CompilerError;
    
    /// テスト用にソースコードからASTを生成し、検証する
    pub fn parse_and_validate(source: &str) -> frontend::error::Result<frontend::ast::Program> {
        // パーサーを作成して実行
        let mut parser = frontend::parser::Parser::new(source);
        let ast = parser.parse()?;
        
        // 名前解決
        let mut name_resolver = NameResolver::new();
        name_resolver.resolve(&ast)?;
        
        // 型チェックの実行
        let type_registry = TypeRegistry::new();
        let diagnostic_emitter = DiagnosticEmitter::new();
        let mut type_checker = TypeChecker::new(type_registry, diagnostic_emitter);
        type_checker.check(&ast)
            .map_err(|e| CompilerError::semantic_error(
                format!("型チェックエラー: {}", e),
                None,
                None,
            ))?;
        
        Ok(ast)
    }
    
    /// ASTの循環依存関係をチェック
    fn check_circular_dependencies(ast: &frontend::ast::Program) -> std::result::Result<(), String> {
        // モジュール依存関係グラフの構築
        let mut dependencies = std::collections::HashMap::new();
        
        // モジュール依存関係の収集
        for decl in &ast.declarations {
            if let frontend::ast::DeclarationKind::ImportDecl(import) = &decl.kind {
                let importing_module = decl.module_id.unwrap_or(0);
                let imported_module = import.module_id;
                
                dependencies
                    .entry(importing_module)
                    .or_insert_with(std::collections::HashSet::new)
                    .insert(imported_module);
            }
        }
        
        // 循環検出
        let mut visited = std::collections::HashSet::new();
        let mut path = std::collections::HashSet::new();
        
        for module_id in dependencies.keys() {
            if detect_cycle(*module_id, &dependencies, &mut visited, &mut path)? {
                return Err(format!("モジュールID {}を含む循環依存関係が検出されました", module_id));
            }
        }
        
        Ok(())
    }
    
    /// グラフの循環検出（DFSベース）
    fn detect_cycle(
        current: u32,
        dependencies: &std::collections::HashMap<u32, std::collections::HashSet<u32>>,
        visited: &mut std::collections::HashSet<u32>,
        path: &mut std::collections::HashSet<u32>,
    ) -> std::result::Result<bool, String> {
        if path.contains(&current) {
            return Ok(true); // 循環検出
        }
        
        if visited.contains(&current) {
            return Ok(false); // 既に訪問済み、循環なし
        }
        
        visited.insert(current);
        path.insert(current);
        
        if let Some(deps) = dependencies.get(&current) {
            for &next in deps {
                if detect_cycle(next, dependencies, visited, path)? {
                    return Ok(true);
                }
            }
        }
        
        path.remove(&current);
        Ok(false)
    }
    
    /// 未使用の識別子をチェックして警告
    fn check_unused_identifiers(ast: &frontend::ast::Program) {
        // 識別子の使用状況の追跡
        let mut declared = std::collections::HashMap::new();
        let mut used = std::collections::HashSet::new();
        
        // 宣言された識別子の収集
        for decl in &ast.declarations {
            match &decl.kind {
                frontend::ast::DeclarationKind::VariableDecl(var) => {
                    declared.insert(var.name.clone(), (decl.id, "変数".to_string(), decl.span.clone()));
                },
                frontend::ast::DeclarationKind::ConstantDecl(constant) => {
                    declared.insert(constant.name.clone(), (decl.id, "定数".to_string(), decl.span.clone()));
                },
                frontend::ast::DeclarationKind::FunctionDecl(func) => {
                    declared.insert(func.name.clone(), (decl.id, "関数".to_string(), decl.span.clone()));
                },
                frontend::ast::DeclarationKind::StructDecl(struct_decl) => {
                    declared.insert(struct_decl.name.clone(), (decl.id, "構造体".to_string(), decl.span.clone()));
                },
                frontend::ast::DeclarationKind::EnumDecl(enum_decl) => {
                    declared.insert(enum_decl.name.clone(), (decl.id, "列挙型".to_string(), decl.span.clone()));
                },
                frontend::ast::DeclarationKind::Interface(interface) => {
                    declared.insert(interface.name.clone(), (decl.id, "インターフェース".to_string(), decl.span.clone()));
                },
                frontend::ast::DeclarationKind::TypeAliasDecl(alias) => {
                    declared.insert(alias.name.clone(), (decl.id, "型エイリアス".to_string(), decl.span.clone()));
                },
                _ => {}
            }
        }
        
        // 使用された識別子の収集
        collect_used_identifiers(ast, &mut used);
        
        // 未使用の識別子を警告
        for (name, (id, kind, span)) in declared {
            if !used.contains(&name) && !name.starts_with('_') {
                // パブリックシンボルは警告しない
                if is_public_symbol(&name, ast) {
                    continue;
                }
                
                warn!("未使用の{}「{}」（ID: {}、位置: {:?}）", kind, name, id, span);
                
                // 診断情報の発行
                let compiler = Compiler::global();
                // let compiler = compiler.read(); // parking_lotのRwLockを使用していたためコメントアウト
                let compiler = compiler.read().expect("コンパイラのロックに失敗しました"); // std::sync::RwLockを使用
                compiler.diagnostic_emitter.emit(Diagnostic::new(DiagnosticLevel::Warning, format!("未使用の{}「{}」", kind, name))
                    .with_code("W0001".to_string()));
            }
        }
    }
    
    /// シンボルがパブリックかどうかを判定
    fn is_public_symbol(name: &str, ast: &frontend::ast::Program) -> bool {
        for decl in &ast.declarations {
            match &decl.kind {
                frontend::ast::DeclarationKind::FunctionDecl(func) => {
                    if func.name == name && func.is_public {
                        return true;
                    }
                },
                frontend::ast::DeclarationKind::StructDecl(struct_decl) => {
                    if struct_decl.name == name && struct_decl.is_public {
                        return true;
                    }
                },
                frontend::ast::DeclarationKind::EnumDecl(enum_decl) => {
                    if enum_decl.name == name && enum_decl.is_public {
                        return true;
                    }
                },
                frontend::ast::DeclarationKind::Interface(interface) => {
                    if interface.name == name && interface.is_public {
                        return true;
                    }
                },
                frontend::ast::DeclarationKind::ConstantDecl(constant) => {
                    if constant.name == name && constant.is_public {
                        return true;
                    }
                },
                frontend::ast::DeclarationKind::VariableDecl(var) => {
                    if var.name == name && var.is_public {
                        return true;
                    }
                },
                _ => {}
            }
        }
        false
    }
    
    /// 使用された識別子を再帰的に収集
    /// 
    /// ASTを走査して、プログラム内で使用されているすべての識別子を収集します。
    /// これは未使用変数の検出や最適化のために使用されます。
    fn collect_used_identifiers(ast: &frontend::ast::Program, used: &mut std::collections::HashSet<String>) {
        // 宣言部分の処理
        for decl in &ast.declarations {
            match &decl.kind {
                frontend::ast::DeclarationKind::FunctionDecl(func) => {
                    // 関数本体内の識別子を収集
                    if let Some(body) = &func.body {
                        for stmt in body {
                            collect_identifiers_from_stmt(stmt, used);
                        }
                    }
                    // 関数パラメータの型や戻り値の型に含まれる識別子を収集
                    for param in &func.parameters {
                        collect_identifiers_from_type(&param.type_annotation, used);
                    }
                    if let Some(return_type) = &func.return_type {
                        collect_identifiers_from_type(return_type, used);
                    }
                },
                frontend::ast::DeclarationKind::StructDecl(struct_decl) => {
                    // 構造体フィールドの型に含まれる識別子を収集
                    for field in &struct_decl.fields {
                        collect_identifiers_from_type(&field.type_annotation, used);
                    }
                    // ジェネリックパラメータに含まれる識別子を収集
                    if let Some(generic_params) = &struct_decl.generic_params {
                        for param in generic_params {
                            if let Some(constraint) = &param.constraint {
                                collect_identifiers_from_type(constraint, used);
                            }
                        }
                    }
                },
                frontend::ast::DeclarationKind::EnumDecl(enum_decl) => {
                    // 列挙型のバリアントに含まれる識別子を収集
                    for variant in &enum_decl.variants {
                        if let Some(fields) = &variant.fields {
                            for field in fields {
                                collect_identifiers_from_type(&field.type_annotation, used);
                            }
                        }
                    }
                    // ジェネリックパラメータに含まれる識別子を収集
                    if let Some(generic_params) = &enum_decl.generic_params {
                        for param in generic_params {
                            if let Some(constraint) = &param.constraint {
                                collect_identifiers_from_type(constraint, used);
                            }
                        }
                    }
                },
                frontend::ast::DeclarationKind::Interface(interface) => {
                    // インターフェースのメソッド宣言に含まれる識別子を収集
                    for method in &interface.methods {
                        for param in &method.parameters {
                            collect_identifiers_from_type(&param.type_annotation, used);
                        }
                        if let Some(return_type) = &method.return_type {
                            collect_identifiers_from_type(return_type, used);
                        }
                    }
                    // ジェネリックパラメータに含まれる識別子を収集
                    if let Some(generic_params) = &interface.generic_params {
                        for param in generic_params {
                            if let Some(constraint) = &param.constraint {
                                collect_identifiers_from_type(constraint, used);
                            }
                        }
                    }
                },
                frontend::ast::DeclarationKind::ConstantDecl(constant) => {
                    // 定数の型と初期化式に含まれる識別子を収集
                    if let Some(type_annotation) = &constant.type_annotation {
                        collect_identifiers_from_type(type_annotation, used);
                    }
                    if let Some(initializer) = &constant.initializer {
                        collect_identifiers_from_expr(initializer, used);
                    }
                },
                frontend::ast::DeclarationKind::VariableDecl(var) => {
                    // 変数の型と初期化式に含まれる識別子を収集
                    if let Some(type_annotation) = &var.type_annotation {
                        collect_identifiers_from_type(type_annotation, used);
                    }
                    if let Some(initializer) = &var.initializer {
                        collect_identifiers_from_expr(initializer, used);
                    }
                },
                frontend::ast::DeclarationKind::ImportDecl(import) => {
                    // インポート宣言に含まれる識別子を収集
                    used.insert(import.module_path.clone());
                    for item in &import.imported_items {
                        used.insert(item.clone());
                    }
                },
                frontend::ast::DeclarationKind::TypeAlias(type_alias) => {
                    // 型エイリアスの元の型に含まれる識別子を収集
                    collect_identifiers_from_type(&type_alias.target_type, used);
                    // ジェネリックパラメータに含まれる識別子を収集
                    if let Some(generic_params) = &type_alias.generic_params {
                        for param in generic_params {
                            if let Some(constraint) = &param.constraint {
                                collect_identifiers_from_type(constraint, used);
                            }
                        }
                    }
                },
                frontend::ast::DeclarationKind::Extension(extension) => {
                    // 拡張対象の型に含まれる識別子を収集
                    collect_identifiers_from_type(&extension.target_type, used);
                    // 拡張内のメソッドに含まれる識別子を収集
                    for method in &extension.methods {
                        if let Some(body) = &method.body {
                            for stmt in body {
                                collect_identifiers_from_stmt(stmt, used);
                            }
                        }
                        for param in &method.parameters {
                            collect_identifiers_from_type(&param.type_annotation, used);
                        }
                        if let Some(return_type) = &method.return_type {
                            collect_identifiers_from_type(return_type, used);
                        }
                    }
                },
                frontend::ast::DeclarationKind::Trait(trait_decl) => {
                    // トレイトのメソッド宣言に含まれる識別子を収集
                    for method in &trait_decl.methods {
                        for param in &method.parameters {
                            collect_identifiers_from_type(&param.type_annotation, used);
                        }
                        if let Some(return_type) = &method.return_type {
                            collect_identifiers_from_type(return_type, used);
                        }
                        // デフォルト実装がある場合は本体も処理
                        if let Some(body) = &method.default_impl {
                            for stmt in body {
                                collect_identifiers_from_stmt(stmt, used);
                            }
                        }
                    }
                    // ジェネリックパラメータに含まれる識別子を収集
                    if let Some(generic_params) = &trait_decl.generic_params {
                        for param in generic_params {
                            if let Some(constraint) = &param.constraint {
                                collect_identifiers_from_type(constraint, used);
                            }
                        }
                    }
                },
                frontend::ast::DeclarationKind::Implementation(impl_decl) => {
                    // 実装対象の型に含まれる識別子を収集
                    collect_identifiers_from_type(&impl_decl.target_type, used);
                    // 実装するトレイトに含まれる識別子を収集
                    if let Some(trait_name) = &impl_decl.trait_name {
                        used.insert(trait_name.clone());
                    }
                    // メソッド実装に含まれる識別子を収集
                    for method in &impl_decl.methods {
                        if let Some(body) = &method.body {
                            for stmt in body {
                                collect_identifiers_from_stmt(stmt, used);
                            }
                        }
                    }
                },
            }
        }

        // 文の処理
        for stmt in &ast.statements {
            collect_identifiers_from_stmt(stmt, used);
        }

        // モジュールの再帰的処理
        for module in &ast.modules {
            collect_used_identifiers(&module.program, used);
        }
    }
    
    /// 文から識別子を収集する補助関数
    fn collect_identifiers_from_stmt(stmt: &frontend::ast::Statement, used: &mut std::collections::HashSet<String>) {
        match &stmt.kind {
            frontend::ast::StatementKind::ExpressionStmt(expr) => {
                collect_identifiers_from_expr(expr, used);
            },
            frontend::ast::StatementKind::VariableDecl(var_decl) => {
                if let Some(type_annotation) = &var_decl.type_annotation {
                    collect_identifiers_from_type(type_annotation, used);
                }
                if let Some(initializer) = &var_decl.initializer {
                    collect_identifiers_from_expr(initializer, used);
                }
            },
            frontend::ast::StatementKind::ConstantDecl(const_decl) => {
                if let Some(type_annotation) = &const_decl.type_annotation {
                    collect_identifiers_from_type(type_annotation, used);
                }
                if let Some(initializer) = &const_decl.initializer {
                    collect_identifiers_from_expr(initializer, used);
                }
            },
            frontend::ast::StatementKind::ReturnStmt(expr_opt) => {
                if let Some(expr) = expr_opt {
                    collect_identifiers_from_expr(expr, used);
                }
            },
            frontend::ast::StatementKind::IfStmt(if_stmt) => {
                collect_identifiers_from_expr(&if_stmt.condition, used);
                for stmt in &if_stmt.then_branch {
                    collect_identifiers_from_stmt(stmt, used);
                }
                if let Some(else_branch) = &if_stmt.else_branch {
                    for stmt in else_branch {
                        collect_identifiers_from_stmt(stmt, used);
                    }
                }
            },
            frontend::ast::StatementKind::WhileStmt(while_stmt) => {
                collect_identifiers_from_expr(&while_stmt.condition, used);
                for stmt in &while_stmt.body {
                    collect_identifiers_from_stmt(stmt, used);
                }
            },
            frontend::ast::StatementKind::ForStmt(for_stmt) => {
                // イテレータ式から識別子を収集
                collect_identifiers_from_expr(&for_stmt.iterator, used);
                // ループ変数は新しい変数なので収集しない
                // ループ本体から識別子を収集
                for stmt in &for_stmt.body {
                    collect_identifiers_from_stmt(stmt, used);
                }
            },
            frontend::ast::StatementKind::BlockStmt(block) => {
                for stmt in block {
                    collect_identifiers_from_stmt(stmt, used);
                }
            },
            frontend::ast::StatementKind::MatchStmt(match_stmt) => {
                collect_identifiers_from_expr(&match_stmt.expression, used);
                for arm in &match_stmt.arms {
                    // パターンから識別子を収集
                    collect_identifiers_from_pattern(&arm.pattern, used);
                    // ガード式がある場合は処理
                    if let Some(guard) = &arm.guard {
                        collect_identifiers_from_expr(guard, used);
                    }
                    // 本体から識別子を収集
                    for stmt in &arm.body {
                        collect_identifiers_from_stmt(stmt, used);
                    }
                }
            },
            frontend::ast::StatementKind::TryStmt(try_stmt) => {
                // try本体から識別子を収集
                for stmt in &try_stmt.try_block {
                    collect_identifiers_from_stmt(stmt, used);
                }
                // catchブロックから識別子を収集
                for catch_clause in &try_stmt.catch_clauses {
                    // エラー型から識別子を収集
                    if let Some(error_type) = &catch_clause.error_type {
                        collect_identifiers_from_type(error_type, used);
                    }
                    // catch本体から識別子を収集
                    for stmt in &catch_clause.body {
                        collect_identifiers_from_stmt(stmt, used);
                    }
                }
                // finallyブロックがある場合は処理
                if let Some(finally_block) = &try_stmt.finally_block {
                    for stmt in finally_block {
                        collect_identifiers_from_stmt(stmt, used);
                    }
                }
            },
            frontend::ast::StatementKind::DeferStmt(deferred_stmt) => {
                collect_identifiers_from_stmt(deferred_stmt, used);
            },
            frontend::ast::StatementKind::BreakStmt | frontend::ast::StatementKind::ContinueStmt => {
                // 識別子を含まない
            },
            frontend::ast::StatementKind::AssertStmt(assert_stmt) => {
                collect_identifiers_from_expr(&assert_stmt.condition, used);
                if let Some(message) = &assert_stmt.message {
                    collect_identifiers_from_expr(message, used);
                }
            },
            frontend::ast::StatementKind::CompTimeStmt(comp_time_stmt) => {
                // コンパイル時実行文の中の識別子を収集
                for stmt in comp_time_stmt {
                    collect_identifiers_from_stmt(stmt, used);
                }
            },
        }
    }

    /// 型から識別子を収集する補助関数
    fn collect_identifiers_from_type(type_annotation: &frontend::ast::TypeAnnotation, used: &mut std::collections::HashSet<String>) {
        match &type_annotation.kind {
            frontend::ast::TypeAnnotationKind::Named(name) => {
                used.insert(name.clone());
            },
            frontend::ast::TypeAnnotationKind::Array(element_type) => {
                collect_identifiers_from_type(element_type, used);
            },
            frontend::ast::TypeAnnotationKind::Optional(inner_type) => {
                collect_identifiers_from_type(inner_type, used);
            },
            frontend::ast::TypeAnnotationKind::Function(params, return_type) => {
                for param in params {
                    collect_identifiers_from_type(param, used);
                }
                collect_identifiers_from_type(return_type, used);
            },
            frontend::ast::TypeAnnotationKind::Generic(base_type, type_args) => {
                used.insert(base_type.clone());
                for arg in type_args {
                    collect_identifiers_from_type(arg, used);
                }
            },
            frontend::ast::TypeAnnotationKind::Tuple(element_types) => {
                for element_type in element_types {
                    collect_identifiers_from_type(element_type, used);
                }
            },
            frontend::ast::TypeAnnotationKind::Union(types) => {
                for type_annotation in types {
                    collect_identifiers_from_type(type_annotation, used);
                }
            },
            frontend::ast::TypeAnnotationKind::Intersection(types) => {
                for type_annotation in types {
                    collect_identifiers_from_type(type_annotation, used);
                }
            },
            frontend::ast::TypeAnnotationKind::Dependent(param_name, dependent_type) => {
                used.insert(param_name.clone());
                collect_identifiers_from_type(dependent_type, used);
            },
            frontend::ast::TypeAnnotationKind::Primitive(_) => {
                // プリミティブ型は識別子を含まない
            },
            frontend::ast::TypeAnnotationKind::SelfType => {
                used.insert("Self".to_string());
            },
            frontend::ast::TypeAnnotationKind::Never => {
                // Never型は識別子を含まない
            },
        }
    }

    /// パターンから識別子を収集する補助関数
    fn collect_identifiers_from_pattern(pattern: &frontend::ast::Pattern, used: &mut std::collections::HashSet<String>) {
        match &pattern.kind {
            frontend::ast::PatternKind::Identifier(_) => {
                // 束縛変数は新しい変数なので収集しない
            },
            frontend::ast::PatternKind::Literal(_) => {
                // リテラルパターンは識別子を含まない
            },
            frontend::ast::PatternKind::Wildcard => {
                // ワイルドカードパターンは識別子を含まない
            },
            frontend::ast::PatternKind::Tuple(patterns) => {
                for pat in patterns {
                    collect_identifiers_from_pattern(pat, used);
                }
            },
            frontend::ast::PatternKind::Struct(struct_name, field_patterns) => {
                used.insert(struct_name.clone());
                for (_, pattern) in field_patterns {
                    collect_identifiers_from_pattern(pattern, used);
                }
            },
            frontend::ast::PatternKind::Enum(enum_name, variant_name, payload) => {
                used.insert(enum_name.clone());
                used.insert(variant_name.clone());
                if let Some(patterns) = payload {
                    for pattern in patterns {
                        collect_identifiers_from_pattern(pattern, used);
                    }
                }
            },
            frontend::ast::PatternKind::Or(patterns) => {
                for pattern in patterns {
                    collect_identifiers_from_pattern(pattern, used);
                }
            },
            frontend::ast::PatternKind::Range(start, end) => {
                if let Some(start_expr) = start {
                    collect_identifiers_from_expr(start_expr, used);
                }
                if let Some(end_expr) = end {
                    collect_identifiers_from_expr(end_expr, used);
                }
            },
            frontend::ast::PatternKind::Reference(inner_pattern) => {
                collect_identifiers_from_pattern(inner_pattern, used);
            },
            frontend::ast::PatternKind::Rest => {
                // 残余パターンは識別子を含まない
            },
        }
    }
    /// 式から識別子を収集
    fn collect_identifiers_from_expr(expr: &frontend::ast::Expression, used: &mut std::collections::HashSet<String>) {
        match &expr.kind {
            frontend::ast::ExpressionKind::Identifier(ident) => {
                used.insert(ident.name.clone());
            },
            // Callは(Box<Expression>, Vec<Expression>)の形式
            frontend::ast::ExpressionKind::Call(callee, arguments) => {
                collect_identifiers_from_expr(callee, used);
                for arg in arguments {
                    collect_identifiers_from_expr(arg, used);
                }
            },
            // 他の式タイプの処理
            _ => {}
        }
    }
}

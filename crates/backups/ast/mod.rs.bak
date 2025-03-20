//! # SwiftLight コンパイラドライバー
//! 
//! コンパイルプロセス全体を管理するドライバーモジュールです。
//! コマンドライン引数の解析、コンパイルパイプラインの実行、エラー処理などを担当します。
//! このモジュールはコンパイラのエントリーポイントとして機能し、フロントエンド、ミドルエンド、
//! バックエンドの各段階を調整します。

pub mod compiler;
pub mod config;
pub mod options;
pub mod diagnostics;
pub mod pipeline;
pub mod cache;
pub mod dependency;
pub mod incremental;
pub mod module_manager;
pub mod plugin_manager;
pub mod build_plan;

// 再エクスポート
pub use self::compiler::Driver;
pub use self::config::CompilerConfig;
pub use self::options::CompileOptions;
// 一時的にコメントアウト - 必要に応じて診断モジュールを実装後に復活させる
// pub use self::diagnostics::{DiagnosticEmitter, Severity};
// pub use self::pipeline::{CompilationStage, Pipeline};
pub use self::cache::CompilationCache;
pub use self::dependency::{DependencyGraph, DependencyNode, DependencyType};
pub use self::incremental::{IncrementalCompilationManager, ChangeDetector, ChangeImpactAnalyzer};
pub use self::module_manager::ModuleManager;
pub use self::plugin_manager::PluginManager;
pub use self::build_plan::BuildPlan;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use rayon::prelude::*;
use crate::frontend::error::{CompilerError, ErrorKind, Result};
// 一時的にコメントアウト - frontend/source モジュールを作成後に復活させる
// use crate::frontend::source::SourceFile;
use crate::middleend::ir::Module;
// 一時的にコメントアウト - backend/target モジュールを作成後に復活させる
// use crate::backend::target::TargetMachine;
use crate::utils::{
    logger::Logger,
    error_handling::BasicErrorHandler,
    parallel::ThreadPool,
};

/// コンパイル処理のエントリーポイント
/// 
/// 指定されたソースファイルをコンパイルし、出力ファイルを生成します。
/// 最適化レベルやターゲットアーキテクチャなどのオプションに基づいて
/// コンパイルパイプラインを構成します。
/// 
/// # 引数
/// 
/// * `source_path` - ソースファイルのパス
/// * `output_path` - 出力ファイルのパス
/// * `options` - コンパイルオプション
/// 
/// # 戻り値
/// 
/// * `Result<()>` - 成功時は`()`、失敗時はエラー
/// 
/// # 例
/// 
/// ```no_run
/// use swiftlight_compiler::driver::{compile, CompileOptions};
/// 
/// let options = CompileOptions::default()
///     .with_optimization_level(2)
///     .with_target("x86_64-unknown-linux-gnu");
///     
/// let result = compile("src/main.sl", "output/main", options);
/// ```
pub fn compile<P: AsRef<Path>>(source_path: P, output_path: P, options: CompileOptions) -> Result<()> {
    let mut driver = compiler::Driver::new(options);
    let result = driver.compile(source_path, output_path)?;
    
    // エラーがあれば失敗を返す
    if result.has_errors() {
        return Err(CompilerError::new(
            ErrorKind::CompilationFailed,
            format!("コンパイルに失敗しました: {} エラー, {} 警告", 
                    result.stats.error_count, result.stats.warning_count),
            None
        ));
    }
    
    Ok(())
}

/// 複数のソースファイルをコンパイルして一つの出力ファイルを生成
/// 
/// 複数のソースファイルを並列処理し、最終的に一つのモジュールに統合します。
/// インクリメンタルコンパイルをサポートし、変更されたファイルのみを再コンパイルします。
/// 
/// # 引数
/// 
/// * `source_paths` - ソースファイルのパスのリスト
/// * `output_path` - 出力ファイルのパス
/// * `options` - コンパイルオプション
/// 
/// # 戻り値
/// 
/// * `Result<()>` - 成功時は`()`、失敗時はエラー
/// 
/// # 例
/// 
/// ```no_run
/// use swiftlight_compiler::driver::{compile_multiple, CompileOptions};
/// 
/// let source_files = vec!["src/main.sl", "src/utils.sl", "src/math.sl"];
/// let options = CompileOptions::default().with_incremental(true);
///     
/// let result = compile_multiple(&source_files, "output/program", options);
/// ```
pub fn compile_multiple<P: AsRef<Path>>(source_paths: &[P], output_path: P, options: CompileOptions) -> Result<()> {
    let mut driver = compiler::Driver::new(options.clone());
    
    // 並列コンパイルを使用する場合
    if options.thread_count.unwrap_or(1) > 1 {
        return compile_multiple_parallel(source_paths, output_path, &mut driver);
    }
    
    let result = driver.compile_multiple(source_paths, output_path)?;
    
    // エラーがあれば失敗を返す
    if result.has_errors() {
        return Err(CompilerError::new(
            ErrorKind::CompilationFailed,
            format!("コンパイルに失敗しました: {} エラー, {} 警告", 
                    result.stats.error_count, result.stats.warning_count),
            None
        ));
    }
    
    Ok(())
}

/// 複数ファイルの並列コンパイル
fn compile_multiple_parallel<P: AsRef<Path>>(
    source_paths: &[P], 
    output_path: P, 
    driver: &mut compiler::Driver
) -> Result<()> {
    // ここで並列コンパイルの実装
    // 依存関係グラフの構築
    driver.build_dependency_graph(source_paths)?;
    
    // コンパイル順序の取得
    let compilation_order = driver.get_compilation_order()?;
    
    // 並列処理用のスレッドプールを初期化
    let thread_count = driver.options().thread_count.unwrap_or_else(num_cpus::get);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .build()
        .map_err(|e| CompilerError::new(
            ErrorKind::Internal,
            format!("スレッドプールの作成に失敗しました: {}", e),
            None
        ))?;
    
    // 各ファイルを並列コンパイル
    let results = Arc::new(Mutex::new(Vec::new()));
    
    pool.install(|| {
        compilation_order.par_iter().try_for_each(|module_name| -> Result<()> {
            let source_path = driver.get_module_source_path(module_name)?;
            let module_result = driver.compile_module(&source_path, module_name)?;
            
            let mut results_guard = results.lock().unwrap();
            results_guard.push(module_result);
            
            Ok(())
        })
    })?;
    
    // 最終リンク
    let modules = results.lock().unwrap();
    driver.link_modules(&modules, &output_path)?;
    
    // コンパイル統計を記録
    let elapsed = driver.total_time();
    record_compilation_stats(driver, elapsed);
    
    Ok(())
}

/// プロジェクト全体をビルド（複数のソースファイルとライブラリ）
/// 
/// プロジェクト設定ファイルに基づいて、プロジェクト全体をビルドします。
/// 依存関係の解決、インクリメンタルビルド、並列コンパイルをサポートします。
/// 
/// # 引数
/// 
/// * `project_path` - プロジェクトのルートディレクトリ
/// * `options` - コンパイルオプション
/// 
/// # 戻り値
/// 
/// * `Result<()>` - 成功時は`()`、失敗時はエラー
pub fn build_project<P: AsRef<Path>>(project_path: P, options: CompileOptions) -> Result<()> {
    // プロジェクト設定の読み込み
    let config_path = project_path.as_ref().join("swiftlight.toml");
    let config = if config_path.exists() {
        config::load_project_config(&config_path)?
    } else {
        config::default_project_config()
    };
    
    // ソースファイルの検索
    let source_dir = project_path.as_ref().join("src");
    let source_files = find_source_files(&source_dir)?;
    
    // 出力ディレクトリの作成
    let output_dir = project_path.as_ref().join("target");
    std::fs::create_dir_all(&output_dir)?;
    
    // 出力ファイルパスの決定
    let output_file = output_dir.join(config.output_name);
    
    // コンパイル実行
    compile_multiple(&source_files, output_file, options)
}

/// コンパイル統計情報を記録
fn record_compilation_stats(driver: &compiler::Driver, elapsed: Duration) {
    let stats = driver.get_stats();
    
    println!("===== コンパイル統計 =====");
    println!("合計時間: {:?}", elapsed);
    println!("ファイル数: {}", stats.file_count);
    println!("合計行数: {}", stats.total_lines);
    println!("字句解析時間: {:?}", stats.lexing_time);
    println!("構文解析時間: {:?}", stats.parsing_time);
    println!("意味解析時間: {:?}", stats.semantic_time);
    println!("コード生成時間: {:?}", stats.codegen_time);
    println!("キャッシュヒット: {}", stats.cache_hits);
    println!("キャッシュミス: {}", stats.cache_misses);
    println!("ピークメモリ使用量: {} MB", stats.peak_memory_usage / (1024 * 1024));
}

/// 指定されたソースコードの静的解析を実行
/// 
/// コンパイルせずに静的解析のみを実行し、警告やエラーを報告します。
/// 
/// # 引数
/// 
/// * `source_path` - ソースファイルのパス
/// * `options` - 解析オプション
/// 
/// # 戻り値
/// 
/// * `Result<Vec<diagnostics::Diagnostic>>` - 診断結果のリスト
pub fn analyze<P: AsRef<Path>>(source_path: P, options: CompileOptions) -> Result<Vec<diagnostics::Diagnostic>> {
    let mut driver = compiler::Driver::new(options);
    driver.analyze(source_path)
}

/// 指定されたソースコードのドキュメント生成
/// 
/// ソースコードからAPIドキュメントを生成します。
/// 
/// # 引数
/// 
/// * `source_paths` - ソースファイルのパスのリスト
/// * `output_dir` - ドキュメント出力ディレクトリ
/// * `options` - ドキュメント生成オプション
/// 
/// # 戻り値
/// 
/// * `Result<()>` - 成功時は`()`、失敗時はエラー
pub fn generate_docs<P: AsRef<Path>>(source_paths: &[P], output_dir: P, options: CompileOptions) -> Result<()> {
    let mut driver = compiler::Driver::new(options);
    driver.generate_docs(source_paths, output_dir)
}

// ユーティリティ関数

/// ディレクトリ内のSwiftLightソースファイルを再帰的に検索
fn find_source_files<P: AsRef<Path>>(dir: P) -> Result<Vec<PathBuf>> {
    let dir = dir.as_ref();
    let mut result = Vec::new();
    
    if !dir.exists() || !dir.is_dir() {
        return Err(CompilerError::new(
            ErrorKind::IO,
            format!("ディレクトリが存在しません: {}", dir.display()),
            None
        ));
    }
    
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            // 再帰的に検索
            let mut sub_files = find_source_files(path)?;
            result.append(&mut sub_files);
        } else if let Some(ext) = path.extension() {
            // SwiftLightソースファイル (.sl) のみを追加
            if ext == "sl" {
                result.push(path);
            }
        }
    }
    
    Ok(result)
}

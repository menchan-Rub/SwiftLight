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

// 再エクスポート
pub use self::compiler::Driver;
pub use self::config::CompilerConfig;
pub use self::options::CompileOptions;
pub use self::diagnostics::{DiagnosticEmitter, Severity};
pub use self::pipeline::{CompilationStage, Pipeline};
pub use self::cache::CompilationCache;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use rayon::prelude::*;
use crate::frontend::error::{CompilerError, ErrorKind, Result};
use crate::frontend::source::SourceFile;
use crate::middleend::ir::Module;
use crate::backend::target::TargetMachine;

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
    let start_time = Instant::now();
    
    // ドライバーを初期化
    let mut driver = Driver::new(options);
    
    // コンパイルを実行
    let result = driver.compile(source_path, output_path);
    
    // コンパイル時間の計測と記録
    let elapsed = start_time.elapsed();
    if driver.config().verbose {
        log::info!("コンパイル完了: {:?}", elapsed);
    }
    
    // コンパイル統計情報の収集
    if driver.config().collect_stats {
        record_compilation_stats(&driver, elapsed);
    }
    
    result
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
    let start_time = Instant::now();
    
    // ドライバーを初期化
    let mut driver = Driver::new(options);
    
    // 並列コンパイルが有効な場合
    if driver.config().parallel_compilation {
        return compile_multiple_parallel(source_paths, output_path, &mut driver);
    }
    
    // 複数ファイルのコンパイルを実行（シーケンシャル）
    let result = driver.compile_multiple(source_paths, output_path);
    
    // コンパイル時間の計測と記録
    let elapsed = start_time.elapsed();
    if driver.config().verbose {
        log::info!("複数ファイルのコンパイル完了: {:?}", elapsed);
    }
    
    // コンパイル統計情報の収集
    if driver.config().collect_stats {
        record_compilation_stats(&driver, elapsed);
    }
    
    result
}

/// 複数ソースファイルの並列コンパイル実装
fn compile_multiple_parallel<P: AsRef<Path>>(
    source_paths: &[P], 
    output_path: P, 
    driver: &mut Driver
) -> Result<()> {
    let start_time = Instant::now();
    
    // ソースファイルの依存関係を解析
    let dependency_graph = driver.analyze_dependencies(source_paths)?;
    
    // 依存関係に基づいてコンパイル順序を決定
    let compilation_order = dependency_graph.topological_sort()?;
    
    // 並列処理可能なファイルグループを特定
    let compilation_groups = dependency_graph.parallel_groups();
    
    // コンパイル結果を保持する共有コンテナ
    let compiled_modules = Arc::new(Mutex::new(Vec::with_capacity(source_paths.len())));
    
    // 各グループを順番に処理し、グループ内は並列処理
    for group in compilation_groups {
        group.par_iter().try_for_each(|file_index| {
            let source_path = &source_paths[*file_index];
            let module_result = driver.compile_to_ir(source_path)?;
            
            // 成功したモジュールを共有コンテナに追加
            if let Ok(module) = module_result {
                let mut modules = compiled_modules.lock().unwrap();
                modules.push(module);
            }
            
            Ok::<_, CompilerError>(())
        })?;
    }
    
    // すべてのモジュールをリンク
    let modules = compiled_modules.lock().unwrap();
    let linked_module = driver.link_modules(&modules)?;
    
    // 最終的なコード生成
    driver.generate_code(&linked_module, output_path)?;
    
    // コンパイル時間の計測と記録
    let elapsed = start_time.elapsed();
    if driver.config().verbose {
        log::info!("並列コンパイル完了: {:?}", elapsed);
    }
    
    // コンパイル統計情報の収集
    if driver.config().collect_stats {
        record_compilation_stats(driver, elapsed);
    }
    
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
    let start_time = Instant::now();
    
    // プロジェクト設定を読み込み
    let project_config = config::ProjectConfig::load(project_path.as_ref())?;
    
    // ドライバーを初期化（プロジェクト設定を反映）
    let mut driver = Driver::new(options.merge_with_project(&project_config));
    
    // プロジェクトのビルドを実行
    let result = driver.build_project(&project_config);
    
    // ビルド時間の計測と記録
    let elapsed = start_time.elapsed();
    if driver.config().verbose {
        log::info!("プロジェクトビルド完了: {:?}", elapsed);
    }
    
    result
}

/// コンパイル統計情報を記録
fn record_compilation_stats(driver: &Driver, elapsed: Duration) {
    let stats = driver.get_compilation_stats();
    
    log::info!("コンパイル統計:");
    log::info!("  総時間: {:?}", elapsed);
    log::info!("  パース時間: {:?}", stats.parse_time);
    log::info!("  型チェック時間: {:?}", stats.type_check_time);
    log::info!("  最適化時間: {:?}", stats.optimization_time);
    log::info!("  コード生成時間: {:?}", stats.codegen_time);
    log::info!("  メモリ使用量: {} MB", stats.memory_usage_mb);
    log::info!("  生成コードサイズ: {} KB", stats.output_size_kb);
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
    let mut driver = Driver::new(options);
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
    let mut driver = Driver::new(options);
    driver.generate_docs(source_paths, output_dir)
}

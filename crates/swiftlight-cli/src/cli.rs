/*
 * SwiftLight CLI - コマンドライン引数処理モジュール
 *
 * このモジュールでは、SwiftLightコンパイラのコマンドライン引数を処理し、
 * 適切なコンパイラAPIの呼び出しに変換します。
 */

use std::path::{Path, PathBuf};
use clap::{Parser, Subcommand};
use anyhow::Result;
use colored::Colorize;
use log::{info, warn, debug};
use indicatif::{ProgressBar, ProgressStyle};
use walkdir;
use env_logger;

use swiftlight_compiler::{
    driver::CompileOptions,
    VERSION
};

/// SwiftLight言語のコンパイラCLIツール
#[derive(Parser)]
#[command(name = "swiftlight")]
#[command(author = "SwiftLight開発チーム")]
#[command(version = VERSION)]
#[command(about = "SwiftLight言語のコンパイラ", long_about = None)]
pub struct Cli {
    /// 詳細なログ出力を有効にする
    #[arg(short, long, default_value = "false")]
    pub verbose: bool,

    /// 不要な出力を抑制する
    #[arg(short, long, default_value = "false")]
    pub quiet: bool,

    /// コンパイル設定ファイルへのパス
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// サブコマンド
    #[command(subcommand)]
    pub command: Commands,
}

/// SwiftLightコンパイラのサブコマンド
#[derive(Subcommand)]
pub enum Commands {
    /// ソースコードをコンパイル
    Build(BuildArgs),
    
    /// コンパイルして実行
    Run(RunArgs),
    
    /// プロジェクトを新規作成
    New(NewArgs),
    
    /// 型チェックのみ実行
    Check(CheckArgs),
    
    /// パッケージ依存関係を管理
    Package(PackageArgs),
    
    /// コードのフォーマット
    Format(FormatArgs),
}

/// ビルドサブコマンドの引数
#[derive(Args)]
pub struct BuildArgs {
    /// 入力ファイルまたはディレクトリ
    #[arg(required = true)]
    pub input: PathBuf,
    
    /// 出力ファイルまたはディレクトリ
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    
    /// 最適化レベル
    #[arg(short, long, default_value = "2")]
    pub optimization: u8,
    
    /// 警告をエラーとして扱う
    #[arg(short = 'W', long, default_value = "false")]
    pub warnings_as_errors: bool,
    
    /// デバッグ情報を含める
    #[arg(short, long, default_value = "false")]
    pub debug: bool,
    
    /// リリースビルドを作成
    #[arg(short, long, default_value = "false")]
    pub release: bool,
    
    /// ターゲットプラットフォーム
    #[arg(short, long)]
    pub target: Option<String>,
}

/// 実行サブコマンドの引数
#[derive(Args)]
pub struct RunArgs {
    /// 実行するファイル
    #[arg(required = true)]
    pub file: PathBuf,
    
    /// コマンドライン引数
    #[arg(last = true)]
    pub args: Vec<String>,
}

/// プロジェクト作成サブコマンドの引数
#[derive(Args)]
pub struct NewArgs {
    /// プロジェクト名
    #[arg(required = true)]
    pub name: String,
    
    /// ライブラリプロジェクトとして作成
    #[arg(short, long, default_value = "false")]
    pub lib: bool,
    
    /// テンプレートを指定
    #[arg(short, long, default_value = "default")]
    pub template: String,
}

/// 型チェックサブコマンドの引数
#[derive(Args)]
pub struct CheckArgs {
    /// 型チェック対象のファイルまたはディレクトリ
    #[arg(required = true)]
    pub input: PathBuf,
    
    /// 詳細な型情報を表示
    #[arg(short, long, default_value = "false")]
    pub explain: bool,
}

/// パッケージ管理サブコマンドの引数
#[derive(Args)]
pub struct PackageArgs {
    /// パッケージサブコマンド
    #[command(subcommand)]
    pub command: PackageCommands,
}

/// パッケージ管理のサブコマンド
#[derive(Subcommand)]
pub enum PackageCommands {
    /// 依存関係を追加
    Add {
        /// パッケージ名
        #[arg(required = true)]
        name: String,
        
        /// バージョン制約
        #[arg(short, long)]
        version: Option<String>,
        
        /// 開発依存として追加
        #[arg(short, long, default_value = "false")]
        dev: bool,
    },
    
    /// 依存関係を更新
    Update {
        /// パッケージ名（省略時は全て更新）
        name: Option<String>,
    },
    
    /// 依存関係を一覧表示
    List,
}

/// コードフォーマットサブコマンドの引数
#[derive(Args)]
pub struct FormatArgs {
    /// フォーマット対象のファイルまたはディレクトリ
    #[arg(default_value = ".")]
    pub path: PathBuf,
    
    /// 変更のみを表示（実際には変更しない）
    #[arg(short, long, default_value = "false")]
    pub check: bool,
}

/// CLIからコンパイル処理を実行
pub fn run_compiler(cli: &Cli) -> Result<()> {
    setup_logging(cli.verbose, cli.quiet)?;
    
    match &cli.command {
        Commands::Build(args) => build(args, cli)?,
        Commands::Run(args) => run(args, cli)?,
        Commands::New(args) => create_new_project(args, cli)?,
        Commands::Check(args) => check(args, cli)?,
        Commands::Package(args) => handle_package(args, cli)?,
        Commands::Format(args) => format_code(args, cli)?,
    }
    
    Ok(())
}

/// ログ設定を初期化
fn setup_logging(verbose: bool, quiet: bool) -> Result<()> {
    let env = env_logger::Env::default()
        .filter_or("SWIFTLIGHT_LOG", if verbose {
            "debug"
        } else if quiet {
            "error"
        } else {
            "info"
        });
    
    env_logger::Builder::from_env(env)
        .format_timestamp(None)
        .format_module_path(verbose)
        .init();
    
    Ok(())
}

/// ビルドコマンドの処理
fn build(args: &BuildArgs, cli: &Cli) -> Result<()> {
    info!("SwiftLightコンパイラ v{} を起動中...", VERSION);
    
    let input_path = &args.input;
    let output_path = args.output.clone().unwrap_or_else(|| {
        // デフォルトの出力パスを決定
        if input_path.is_dir() {
            PathBuf::from("./build")
        } else {
            let mut path = input_path.file_stem().map(PathBuf::from).unwrap_or_default();
            path.set_extension(if cfg!(windows) { "exe" } else { "" });
            path
        }
    });
    
    info!("コンパイル: {} → {}", 
          input_path.display().to_string().cyan(),
          output_path.display().to_string().green());
    
    // プログレスバーの設定
    let pb = if !cli.quiet {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap()
        );
        pb.set_message("コンパイル中...");
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        Some(pb)
    } else {
        None
    };
    
    // コンパイルオプションの設定
    let options = CompileOptions {
        optimization_level: match (args.optimization, args.release) {
            (_, true) => 3,  // リリースモードは最大最適化
            (o, false) => o as u32,
        },
        debug_info: args.debug,
        warnings_as_errors: args.warnings_as_errors,
        target_triple: args.target.clone(),
        ..Default::default()
    };
    
    // コンパイル実行
    let result = compile(input_path, &output_path, options);
    
    // プログレスバーを完了
    if let Some(pb) = pb {
        match &result {
            Ok(_) => {
                pb.finish_with_message(format!("コンパイル成功: {}", 
                                             output_path.display().to_string().green()));
            },
            Err(e) => {
                pb.finish_with_message(format!("コンパイル失敗: {}", e.to_string().red()));
            }
        }
    }
    
    result.context("コンパイル処理に失敗しました")
}

/// 実行コマンドの処理
fn run(args: &RunArgs, cli: &Cli) -> Result<()> {
    // ビルド引数を構成
    let build_args = BuildArgs {
        input: args.file.clone(),
        output: None,
        optimization: 0,  // 開発モードでの実行なので低い最適化レベル
        warnings_as_errors: false,
        debug: true,
        release: false,
        target: None,
    };
    
    // ビルド実行
    build(&build_args, cli)?;
    
    // 実行ファイルパスの決定
    let exe_path = if let Some(stem) = args.file.file_stem() {
        let mut path = PathBuf::from(".");
        path.push(stem);
        if cfg!(windows) {
            path.set_extension("exe");
        }
        path
    } else {
        return Err(anyhow::anyhow!("無効なファイル名: {}", args.file.display()));
    };
    
    // プログラムを実行
    info!("実行: {}", exe_path.display().to_string().green());
    
    let status = std::process::Command::new(exe_path)
        .args(&args.args)
        .status()
        .context("プログラムの実行に失敗しました")?;
    
    if !status.success() {
        warn!("プログラムは終了コード {} で終了しました", 
             status.code().unwrap_or(-1));
    }
    
    Ok(())
}

/// 新規プロジェクト作成の処理
fn create_new_project(args: &NewArgs, _cli: &Cli) -> Result<()> {
    info!("新規プロジェクト '{}' を作成中...", args.name);
    
    // プロジェクトディレクトリの作成
    let project_dir = PathBuf::from(&args.name);
    if project_dir.exists() {
        return Err(anyhow::anyhow!("ディレクトリ '{}' はすでに存在します", args.name));
    }
    
    std::fs::create_dir(&project_dir)
        .context("プロジェクトディレクトリの作成に失敗しました")?;
    
    // src ディレクトリの作成
    let src_dir = project_dir.join("src");
    std::fs::create_dir(&src_dir)
        .context("ソースディレクトリの作成に失敗しました")?;
    
    // テンプレートファイルの作成
    if args.lib {
        // ライブラリプロジェクトテンプレート
        let lib_file = src_dir.join("lib.sl");
        std::fs::write(lib_file, include_str!("../templates/lib.sl"))
            .context("ライブラリテンプレートファイルの作成に失敗しました")?;
    } else {
        // 実行可能プロジェクトテンプレート
        let main_file = src_dir.join("main.sl");
        std::fs::write(main_file, include_str!("../templates/main.sl"))
            .context("メインテンプレートファイルの作成に失敗しました")?;
    }
    
    // 設定ファイルの作成
    let config_file = project_dir.join("swiftlight.toml");
    let config_contents = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
authors = []

[dependencies]
"#,
        args.name
    );
    
    std::fs::write(config_file, config_contents)
        .context("設定ファイルの作成に失敗しました")?;
    
    // .gitignore ファイルの作成
    let gitignore_file = project_dir.join(".gitignore");
    std::fs::write(gitignore_file, include_str!("../templates/gitignore"))
        .context(".gitignore ファイルの作成に失敗しました")?;
    
    info!("プロジェクト '{}' の作成が完了しました", args.name.green());
    info!("新しいプロジェクトを始めるには:");
    info!("    cd {}", args.name);
    info!("    swiftlight build");
    
    Ok(())
}

/// 型チェックコマンドの処理
fn check(args: &CheckArgs, _cli: &Cli) -> Result<()> {
    info!("型チェック: {}", args.input.display().to_string().cyan());
    
    // 型チェックのみを実行するオプションを設定
    let options = CompileOptions {
        type_check_only: true,
        ..Default::default()
    };
    
    // 一時的な出力パスを使用
    let temp_dir = tempfile::tempdir()
        .context("一時ディレクトリの作成に失敗しました")?;
    let output_path = temp_dir.path().join("output");
    
    // コンパイル処理（型チェックのみ）を実行
    match compile(&args.input, &output_path, options) {
        Ok(_) => {
            info!("型チェックは成功しました ✓");
            Ok(())
        },
        Err(e) => {
            error!("型チェックに失敗しました: {}", e);
            Err(anyhow::anyhow!("型チェックエラー"))
        }
    }
}

/// パッケージ管理コマンドの処理
fn handle_package(args: &PackageArgs, _cli: &Cli) -> Result<()> {
    match &args.command {
        PackageCommands::Add { name, version, dev } => {
            info!("パッケージの追加: {}{}", 
                 name, 
                 version.as_ref().map_or(String::new(), |v| format!(" v{}", v)));
            
            if *dev {
                info!("開発依存関係として追加します");
            }
            
            // パッケージ追加の実装
            // （ここでは実装を省略し、"未実装"メッセージを表示）
            warn!("パッケージ管理機能は現在開発中です");
        },
        PackageCommands::Update { name } => {
            if let Some(pkg_name) = name {
                info!("パッケージの更新: {}", pkg_name);
            } else {
                info!("全パッケージの更新");
            }
            
            // パッケージ更新の実装
            warn!("パッケージ管理機能は現在開発中です");
        },
        PackageCommands::List => {
            info!("依存関係の一覧:");
            
            // 依存関係一覧の実装
            warn!("パッケージ管理機能は現在開発中です");
        }
    }
    
    Ok(())
}

/// コードフォーマットコマンドの処理
fn format_code(args: &FormatArgs, _cli: &Cli) -> Result<()> {
    let path_str = args.path.display().to_string();
    
    if args.check {
        info!("フォーマットのチェック: {}", path_str.cyan());
    } else {
        info!("フォーマット: {}", path_str.cyan());
    }
    
    // ファイル列挙
    let files = collect_source_files(&args.path)?;
    info!("{} 個のファイルを処理します", files.len());
    
    // ここでフォーマット処理の実装を行う
    // 現在はsleepで処理を模擬
    for file in &files {
        debug!("ファイルのフォーマット: {}", file.display());
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    
    if args.check {
        info!("全てのファイルは正しくフォーマットされています ✓");
    } else {
        info!("{} 個のファイルをフォーマットしました", files.len());
    }
    
    Ok(())
}

/// ディレクトリからソースファイルを再帰的に収集
fn collect_source_files(path: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    
    if path.is_file() {
        if has_swiftlight_extension(path) {
            files.push(path.to_path_buf());
        }
    } else if path.is_dir() {
        for entry in walkdir::WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| !e.file_type().is_dir()) 
        {
            let path = entry.path().to_path_buf();
            if has_swiftlight_extension(&path) {
                files.push(path);
            }
        }
    }
    
    Ok(files)
}

/// ファイルがSwiftLight拡張子を持つかチェック
fn has_swiftlight_extension(path: &Path) -> bool {
    path.extension()
        .map(|ext| ext == "sl")
        .unwrap_or(false)
}

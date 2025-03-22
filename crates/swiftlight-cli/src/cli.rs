/*
 * SwiftLight CLI - コマンドライン引数処理モジュール
 *
 * このモジュールでは、SwiftLightコンパイラのコマンドライン引数を処理し、
 * 適切なコンパイラAPIの呼び出しに変換します。
 */

use std::path::{Path, PathBuf};
use clap::{Parser, Subcommand};
use anyhow::{Context, Result};
use colored::Colorize;
use log::{info, warn, debug, error};
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use walkdir;
use env_logger;
use semver::VersionReq;
use tempfile::tempdir;
use swiftlight_compiler::{
    driver::{CompileOptions, compile},
    formatter::format_code as format_swiftlight_code,
    package::{PackageManager, DependencyType},
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
    
    /// 最適化レベル (0-3)
    #[arg(short, long, default_value = "2", value_parser = clap::value_parser!(u8).range(0..=3))]
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
    /// プロジェクト名 (小文字、数字、ハイフンのみ許可)
    #[arg(required = true, value_parser = validate_project_name)]
    pub name: String,
    
    /// ライブラリプロジェクトとして作成
    #[arg(short, long, default_value = "false")]
    pub lib: bool,
    
    /// 使用するテンプレート
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
        /// パッケージ名 (形式: name@version)
        #[arg(required = true, value_parser = parse_package_spec)]
        spec: (String, Option<VersionReq>),
        
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
    
    /// 依存関係を削除
    Remove {
        /// パッケージ名
        #[arg(required = true)]
        name: String,
    },
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
    
    /// 再帰的に処理
    #[arg(short, long, default_value = "false")]
    pub recursive: bool,
    
    /// 特定のコード構造のみをフォーマット（例: imports,functions,all）
    #[arg(short, long, value_enum, default_value = "all")]
    pub scope: FormatScope,
    
    /// フォーマットルールの設定ファイル
    #[arg(short, long)]
    pub config: Option<PathBuf>,
    
    /// 並列処理の最大スレッド数（0は自動的にCPUコア数を使用）
    #[arg(short, long, default_value = "0")]
    pub jobs: usize,
    
    /// フォーマット前後の差分を表示
    #[arg(short, long, default_value = "false")]
    pub diff: bool,
    
    /// 特定のパターンに一致するファイルを除外
    #[arg(short, long, value_delimiter = ',')]
    pub exclude: Option<Vec<String>>,
    
    /// 特定のパターンに一致するファイルのみを処理
    #[arg(short = 'i', long, value_delimiter = ',')]
    pub include: Option<Vec<String>>,
    
    /// フォーマットの詳細ログを出力
    #[arg(short = 'v', long, default_value = "false")]
    pub verbose_format: bool,
    
    /// 自動修正の提案を表示
    #[arg(long, default_value = "false")]
    pub suggest_fixes: bool,
    
    /// コードスタイルプリセット
    #[arg(long, value_enum, default_value = "standard")]
    pub style: FormatStyle,
    
    /// フォーマット後に静的解析を実行
    #[arg(long, default_value = "false")]
    pub analyze: bool,
    
    /// フォーマット時にコメントの文法チェックを行う
    #[arg(long, default_value = "false")]
    pub check_comments: bool,
    
    /// フォーマット時にドキュメンテーションコメントの完全性を検証
    #[arg(long, default_value = "false")]
    pub verify_docs: bool,
    
    /// フォーマット時にコードの複雑さメトリクスを計算して表示
    #[arg(long, default_value = "false")]
    pub metrics: bool,
    
    /// フォーマット時に未使用のインポートや変数を削除
    #[arg(long, default_value = "false")]
    pub fix_unused: bool,
    
    /// フォーマット時にコードの最適化提案を表示
    #[arg(long, default_value = "false")]
    pub suggest_optimizations: bool,
    
    /// フォーマット結果をキャッシュして次回の実行を高速化
    #[arg(long, default_value = "true")]
    pub cache: bool,
    
    /// フォーマット前にバックアップを作成
    #[arg(long, default_value = "false")]
    pub backup: bool,
    
    /// フォーマット時にコードの一貫性を検証
    #[arg(long, default_value = "false")]
    pub verify_consistency: bool,
    
    /// フォーマット時に型の整合性を検証
    #[arg(long, default_value = "false")]
    pub verify_types: bool,
    
    /// フォーマット時にセキュリティの問題を検出
    #[arg(long, default_value = "false")]
    pub security_check: bool,
    
    /// フォーマット時にパフォーマンスの問題を検出
    #[arg(long, default_value = "false")]
    pub performance_check: bool,
    
    /// フォーマット時にメモリ安全性の問題を検出
    #[arg(long, default_value = "false")]
    pub memory_safety_check: bool,
    
    /// フォーマット時に並行処理の問題を検出
    #[arg(long, default_value = "false")]
    pub concurrency_check: bool,
    
    /// フォーマット時にコードの可読性スコアを計算して表示
    #[arg(long, default_value = "false")]
    pub readability_score: bool,
    
    /// フォーマット時にコードの保守性スコアを計算して表示
    #[arg(long, default_value = "false")]
    pub maintainability_score: bool,
    
    /// フォーマット時にコードの再利用性スコアを計算して表示
    #[arg(long, default_value = "false")]
    pub reusability_score: bool,
    
    /// フォーマット時にコードの品質レポートを生成
    #[arg(long)]
    pub quality_report: Option<PathBuf>,
    
    /// フォーマット時にコードの変更履歴を考慮
    #[arg(long, default_value = "false")]
    pub consider_history: bool,
    
    /// フォーマット時にチーム固有のコーディング規約を適用
    #[arg(long)]
    pub team_rules: Option<PathBuf>,
    
    /// フォーマット時にAIによるコード改善提案を表示
    #[arg(long, default_value = "false")]
    pub ai_suggestions: bool,
}

/// フォーマット対象のスコープ
#[derive(Clone, Debug, ValueEnum)]
pub enum FormatScope {
    /// すべての要素をフォーマット
    All,
    /// インポート文のみをフォーマット
    Imports,
    /// 関数定義のみをフォーマット
    Functions,
    /// 型定義のみをフォーマット
    Types,
    /// コメントのみをフォーマット
    Comments,
    /// 空白行と字下げのみをフォーマット
    Whitespace,
    /// 変数宣言のみをフォーマット
    Variables,
    /// 制御構造のみをフォーマット
    ControlFlow,
    /// エラー処理のみをフォーマット
    ErrorHandling,
    /// 並行処理コードのみをフォーマット
    Concurrency,
    /// メタプログラミングコードのみをフォーマット
    Metaprogramming,
    /// 依存型関連コードのみをフォーマット
    DependentTypes,
    /// マクロのみをフォーマット
    Macros,
    /// ドキュメンテーションコメントのみをフォーマット
    Documentation,
    /// テストコードのみをフォーマット
    Tests,
    /// 特定のモジュールのみをフォーマット（パスと組み合わせて使用）
    Module,
    /// 特定のトレイト実装のみをフォーマット
    Traits,
    /// 特定のパターンマッチングのみをフォーマット
    PatternMatching,
    /// 特定のメモリ管理コードのみをフォーマット
    MemoryManagement,
    /// 特定のunsafeブロックのみをフォーマット
    UnsafeBlocks,
    /// 特定の最適化アノテーションのみをフォーマット
    OptimizationHints,
}

/// フォーマットスタイルのプリセット
#[derive(Clone, Debug, ValueEnum)]
pub enum FormatStyle {
    /// 最小限の変更のみ適用
    Minimal,
    /// 標準的なコードスタイルを適用
    Standard,
    /// 包括的なコードスタイルを適用
    Comprehensive,
    /// プロジェクト固有の設定を使用
    Custom,
    /// 高度な読みやすさを重視したスタイル
    Readable,
    /// コードの簡潔さを重視したスタイル
    Compact,
    /// 学術的なコードスタイル（数学的表記に近い）
    Academic,
    /// 企業向けの厳格なコードスタイル
    Enterprise,
    /// オープンソースプロジェクト向けのコードスタイル
    OpenSource,
    /// 教育目的のコードスタイル（詳細なコメント付き）
    Educational,
    /// パフォーマンス重視のコードスタイル
    Performance,
    /// メモリ効率重視のコードスタイル
    MemoryEfficient,
    /// 並行処理に最適化されたコードスタイル
    ConcurrencyOptimized,
    /// セキュリティを重視したコードスタイル
    SecurityFocused,
    /// 保守性を重視したコードスタイル
    Maintainable,
    /// 特定のドメイン向けにカスタマイズされたスタイル
    DomainSpecific,
    /// 最新の言語機能を活用したモダンなスタイル
    Modern,
    /// 従来の言語との互換性を重視したスタイル
    Compatible,
    /// AIによる自動最適化スタイル
    AIOptimized,
    /// チーム固有のスタイル（team_rulesと組み合わせて使用）
    Team,
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
        .format_timestamp(Some(env_logger::TimestampPrecision::Millis))
        .format_module_path(verbose)
        .init();
    
    Ok(())
}

/// ビルドコマンドの処理
fn build(args: &BuildArgs, cli: &Cli) -> Result<()> {
    info!("SwiftLightコンパイラ v{} を起動中...", VERSION);
    
    let input_path = &args.input;
    let output_path = args.output.clone().unwrap_or_else(|| {
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
    
    let mp = MultiProgress::new();
    let pb = if !cli.quiet {
        let pb = mp.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} [{elapsed_precise}] {msg}")
                .unwrap()
        );
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        Some(pb)
    } else {
        None
    };
    
    let options = CompileOptions {
        optimization_level: match (args.optimization, args.release) {
            (_, true) => 3,
            (o, false) => o as u32,
        },
        debug_info: args.debug,
        warnings_as_errors: args.warnings_as_errors,
        target_triple: args.target.clone(),
        ..Default::default()
    };
    
    let result = compile(input_path, &output_path, options)
        .with_context(|| format!("{} のコンパイルに失敗しました", input_path.display()));
    
    if let Some(pb) = pb {
        match &result {
            Ok(_) => pb.finish_with_message(format!("✅ コンパイル成功: {}", output_path.display())),
            Err(e) => pb.finish_with_message(format!("❌ コンパイル失敗: {}", e.to_string().red())),
        }
    }
    
    result
}

/// 実行コマンドの処理
fn run(args: &RunArgs, cli: &Cli) -> Result<()> {
    let build_args = BuildArgs {
        input: args.file.clone(),
        output: None,
        optimization: 0,
        warnings_as_errors: false,
        debug: true,
        release: false,
        target: None,
    };
    
    build(&build_args, cli)?;
    
    let exe_path = args.file.with_extension(if cfg!(windows) { "exe" } else { "" });
    if !exe_path.exists() {
        return Err(anyhow::anyhow!("実行ファイル {} が見つかりません", exe_path.display()));
    }
    
    info!("🚀 実行開始: {}", exe_path.display().green());
    let output = std::process::Command::new(exe_path)
        .args(&args.args)
        .output()
        .context("プログラムの実行に失敗しました")?;
    
    print!("{}", String::from_utf8_lossy(&output.stdout));
    eprint!("{}", String::from_utf8_lossy(&output.stderr));
    
    if !output.status.success() {
        Err(anyhow::anyhow!("プログラムが終了コード {} で異常終了しました", 
            output.status.code().unwrap_or(-1)))
    } else {
        Ok(())
    }
}

/// 新規プロジェクト作成の処理
fn create_new_project(args: &NewArgs, _cli: &Cli) -> Result<()> {
    let project_dir = PathBuf::from(&args.name);
    if project_dir.exists() {
        return Err(anyhow::anyhow!("ディレクトリ '{}' は既に存在します", args.name));
    }
    
    fs::create_dir_all(project_dir.join("src"))?;
    fs::create_dir_all(project_dir.join("tests"))?;
    
    let template_path = PathBuf::from("templates").join(&args.template);
    if template_path.exists() {
        copy_dir_all(template_path, &project_dir)?;
    } else {
        let main_file = project_dir.join("src/main.sl");
        fs::write(main_file, "func main() {\n    println(\"Hello, SwiftLight!\");\n}\n")?;
    }
    
    let config_content = format!(
        "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nauthors = []\n\n[dependencies]\n",
        args.name
    );
    fs::write(project_dir.join("swiftlight.toml"), config_content)?;
    
    info!("🎉 プロジェクト '{}' が正常に作成されました", args.name.green());
    info!("次のコマンドでビルドできます:\n    cd {}\n    swiftlight build", args.name);
    
    Ok(())
}

/// 型チェックコマンドの処理
fn check(args: &CheckArgs, _cli: &Cli) -> Result<()> {
    let options = CompileOptions {
        type_check_only: true,
        explain_types: args.explain,
        ..Default::default()
    };
    
    let temp_dir = tempdir()?;
    compile(&args.input, &temp_dir.path().join("output"), options)
        .map(|_| info!("✅ 型チェックが正常に完了しました"))
        .map_err(|e| {
            error!("❌ 型チェックエラー: {}", e);
            anyhow::anyhow!("型チェックに失敗しました")
        })
}

/// パッケージ管理コマンドの処理
fn handle_package(args: &PackageArgs, _cli: &Cli) -> Result<()> {
    let mut pm = PackageManager::new()?;
    
    match &args.command {
        PackageCommands::Add { spec: (name, version), dev } => {
            pm.add_dependency(
                name,
                version.clone(),
                if *dev { DependencyType::Dev } else { DependencyType::Normal }
            )?;
            info!("📦 パッケージ '{}' を追加しました", name);
        },
        PackageCommands::Update { name } => {
            if let Some(name) = name {
                pm.update_dependency(name)?;
                info!("🔄 パッケージ '{}' を更新しました", name);
            } else {
                pm.update_all()?;
                info!("🔄 全てのパッケージを更新しました");
            }
        },
        PackageCommands::List => {
            let deps = pm.list_dependencies()?;
            if deps.is_empty() {
                info!("📭 依存関係はありません");
            } else {
                info!("📜 依存関係一覧:");
                for (name, version) in deps {
                    info!("  - {} {}", name, version.map_or("".into(), |v| v.to_string()));
                }
            }
        },
        PackageCommands::Remove { name } => {
            pm.remove_dependency(name)?;
            info!("🗑️ パッケージ '{}' を削除しました", name);
        }
    }
    
    Ok(())
}

/// コードフォーマットコマンドの処理
fn format_code(args: &FormatArgs, _cli: &Cli) -> Result<()> {
    let files = collect_source_files(&args.path, args.recursive)?;
    let mut changed = 0;
    
    for file in files {
        let original = fs::read_to_string(&file)?;
        let formatted = format_swiftlight_code(&original)?;
        
        if original != formatted {
            if args.check {
                warn!("⚠️ フォーマットが必要: {}", file.display());
                changed += 1;
            } else {
                fs::write(&file, formatted)?;
                info!("✨ フォーマット完了: {}", file.display());
            }
        }
    }
    
    if args.check {
        if changed > 0 {
            Err(anyhow::anyhow!("{} 個のファイルにフォーマットが必要です", changed))
        } else {
            info!("✅ 全てのファイルが正しくフォーマットされています");
            Ok(())
        }
    } else {
        info!("🎉 {} 個のファイルをフォーマットしました", changed);
        Ok(())
    }
}

/// ディレクトリからソースファイルを収集
fn collect_source_files(path: &Path, recursive: bool) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    
    if path.is_file() {
        if has_swiftlight_extension(path) {
            files.push(path.to_path_buf());
        }
    } else if path.is_dir() {
        let walker = walkdir::WalkDir::new(path)
            .follow_links(true)
            .max_depth(if recursive { 100 } else { 1 });
        
        for entry in walker.into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file() && has_swiftlight_extension(path) {
                files.push(path.to_path_buf());
            }
        }
    }
    
    Ok(files)
}

/// 拡張子チェック
fn has_swiftlight_extension(path: &Path) -> bool {
    path.extension()
        .map(|ext| ext == "sl" || ext == "swiftlight")
        .unwrap_or(false)
}

/// プロジェクト名のバリデーション
fn validate_project_name(name: &str) -> Result<String> {
    let valid = name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !name.starts_with('-')
        && !name.ends_with('-')
        && name.len() >= 3;
    
    if valid {
        Ok(name.to_string())
    } else {
        Err(anyhow::anyhow!("プロジェクト名は小文字、数字、ハイフンのみ使用可能で、3文字以上必要です"))
    }
}

/// パッケージ仕様のパース
fn parse_package_spec(spec: &str) -> Result<(String, Option<VersionReq>)> {
    let parts: Vec<_> = spec.splitn(2, '@').collect();
    let name = parts[0].to_string();
    let version = parts.get(1).map(|s| VersionReq::parse(s)).transpose()?;
    Ok((name, version))
}

/// ディレクトリコピーユーティリティ
fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

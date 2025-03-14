/*
 * SwiftLight パッケージマネージャ - メインエントリーポイント
 *
 * SwiftLight言語のパッケージ管理ツールのエントリーポイントです。
 * 単独のコマンドとしても、CLIツールからサブコマンドとしても使用できます。
 */

use clap::{Parser, Subcommand};
use anyhow::{Result, Context};
use log::{info, warn, debug};
use env_logger::{Builder, Env};
use log::LevelFilter;

mod registry;
mod dependency;

/// SwiftLight パッケージマネージャのコマンドラインインターフェース
#[derive(Parser)]
#[command(name = "swiftlight-package")]
#[command(author = "Shard")]
#[command(version = "0.1.0")]
#[command(about = "SwiftLight言語のパッケージマネージャ", long_about = None)]
struct Cli {
    /// 詳細なログ出力を有効にする
    #[arg(short, long, default_value = "false")]
    verbose: bool,

    /// 不要な出力を抑制する
    #[arg(short, long, default_value = "false")]
    quiet: bool,

    /// パッケージマネージャのサブコマンド
    #[command(subcommand)]
    command: Commands,
}

/// パッケージマネージャのサブコマンド
#[derive(Subcommand)]
enum Commands {
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
    
    /// 依存関係を検索
    Search {
        /// 検索キーワード
        #[arg(required = true)]
        query: String,
    },
    
    /// パッケージの情報を表示
    Info {
        /// パッケージ名
        #[arg(required = true)]
        name: String,
    },
    
    /// パッケージの公開
    Publish,
    
    /// レジストリの管理
    Registry {
        /// レジストリのURL
        #[arg(required = true)]
        url: String,
        
        /// 認証トークン
        #[arg(short, long)]
        token: Option<String>,
    },
}

/// CLIエントリーポイント
fn main() -> Result<()> {
    // コマンドライン引数の解析
    let cli = Cli::parse();
    
    // ログレベルの初期化
    setup_logging(cli.verbose, cli.quiet);
    
    // コマンドの実行
    match &cli.command {
        Commands::Add { name, version, dev } => {
            let version_str = version.as_ref().map_or("最新", |v| v.as_str());
            info!("パッケージの追加: {} ({})", name, version_str);
            
            let dep_type = if *dev { "開発依存関係" } else { "依存関係" };
            info!("パッケージタイプ: {}", dep_type);
            
            let result = dependency::add_dependency(name, version.as_deref(), *dev)?;
            info!("パッケージの追加が完了しました: {}", result);
        },
        Commands::Update { name } => {
            if let Some(pkg_name) = name {
                info!("パッケージの更新: {}", pkg_name);
                dependency::update_dependency(Some(pkg_name))?;
            } else {
                info!("全パッケージの更新");
                dependency::update_dependency(None)?;
            }
            info!("更新が完了しました");
        },
        Commands::List => {
            info!("インストール済みパッケージ一覧:");
            let dependencies = dependency::list_dependencies()?;
            
            if dependencies.is_empty() {
                info!("  パッケージはインストールされていません");
            } else {
                for (name, version) in dependencies {
                    info!("  {} ({})", name, version);
                }
            }
        },
        Commands::Search { query } => {
            info!("パッケージの検索: {}", query);
            let results = registry::search_packages(query)?;
            
            info!("検索結果 ({} 件):", results.len());
            for (name, desc) in results {
                info!("  {} - {}", name, desc);
            }
        },
        Commands::Info { name } => {
            info!("パッケージ情報の取得: {}", name);
            let pkg_info = registry::get_package_info(name)?;
            
            info!("パッケージ: {}", pkg_info.name);
            info!("バージョン: {}", pkg_info.version);
            info!("説明: {}", pkg_info.description);
            info!("作者: {}", pkg_info.author);
            info!("ライセンス: {}", pkg_info.license);
            info!("ダウンロード数: {}", pkg_info.downloads);
            
            info!("依存関係:");
            for dep in pkg_info.dependencies {
                info!("  {}", dep);
            }
        },
        Commands::Publish => {
            info!("パッケージの公開を開始します...");
            registry::publish_package()?;
            info!("パッケージの公開が完了しました");
        },
        Commands::Registry { url, token } => {
            info!("レジストリの設定: {}", url);
            if token.is_some() {
                info!("認証情報が提供されました");
            }
            registry::configure_registry(url, token.as_deref())?;
            info!("レジストリの設定が完了しました");
        },
    }
    
    Ok(())
}

/// ログ設定を初期化
fn setup_logging(verbose: bool, quiet: bool) {
    // env_loggerを使用してログレベルを設定
    let mut builder = Builder::from_env(Env::default());
    
    // ログレベルを設定
    if verbose {
        builder.filter_level(LevelFilter::Debug);
        builder.init();
        info!("詳細ログモードが有効になりました");
    } else if quiet {
        builder.filter_level(LevelFilter::Error);
        builder.init();
    } else {
        builder.filter_level(LevelFilter::Info);
        builder.init();
    }
}

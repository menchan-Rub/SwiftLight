/*
 * SwiftLight パッケージマネージャ - メインエントリーポイント
 *
 * SwiftLight言語のパッケージ管理ツールのエントリーポイントです。
 * 単独のコマンドとしても、CLIツールからサブコマンドとしても使用できます。
 * 
 * 特徴:
 * - 高速な依存関係解決アルゴリズム
 * - インクリメンタルダウンロードとキャッシング
 * - セキュリティ検証と脆弱性スキャン
 * - 分散型レジストリサポート
 * - オフラインモード対応
 * - プラグイン拡張システム
 */

use clap::{Parser, Subcommand};
use anyhow::{Result, Context, bail, anyhow};
use log::{info, warn, debug, error, trace};
use env_logger::{Builder, Env};
use log::LevelFilter;
use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::collections::{HashMap, HashSet, BTreeMap};
use std::time::{Duration, Instant, SystemTime};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use semver::{Version, VersionReq};
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use chrono::{DateTime, Utc};
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use console::style;
use reqwest::Client;
use rayon::prelude::*;
use tempfile::TempDir;
use toml::{self, Value};
use walkdir::WalkDir;
use async_trait::async_trait;
use futures::{stream, StreamExt};
use crossbeam_channel::{bounded, Sender, Receiver};
use parking_lot::RwLock;

mod registry;
mod dependency;
mod cache;
mod config;
mod lockfile;
mod resolver;
mod security;
mod build;
mod plugin;
mod workspace;
mod manifest;
mod network;
mod storage;
mod metrics;
mod validation;
mod hooks;
mod telemetry;
mod offline;
mod mirror;
mod compression;
mod signature;
mod progress;
mod error;
mod utils;

use crate::config::Config;
use crate::manifest::Manifest;
use crate::lockfile::Lockfile;
use crate::resolver::DependencyResolver;
use crate::security::SecurityScanner;
use crate::cache::PackageCache;
use crate::plugin::PluginManager;
use crate::workspace::Workspace;
use crate::network::NetworkManager;
use crate::storage::StorageManager;
use crate::metrics::MetricsCollector;
use crate::validation::PackageValidator;
use crate::hooks::HookManager;
use crate::telemetry::TelemetryManager;
use crate::offline::OfflineMode;
use crate::mirror::MirrorManager;
use crate::compression::CompressionManager;
use crate::signature::SignatureVerifier;
use crate::progress::ProgressManager;
use crate::error::PackageError;

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

    /// デバッグレベルのログ出力を有効にする
    #[arg(short = 'd', long, default_value = "false")]
    debug: bool,

    /// トレースレベルのログ出力を有効にする
    #[arg(long, default_value = "false")]
    trace: bool,

    /// 不要な出力を抑制する
    #[arg(short, long, default_value = "false")]
    quiet: bool,

    /// 設定ファイルのパス
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// キャッシュディレクトリのパス
    #[arg(long)]
    cache_dir: Option<PathBuf>,

    /// オフラインモードで実行
    #[arg(long, default_value = "false")]
    offline: bool,

    /// 進捗表示を無効化
    #[arg(long, default_value = "false")]
    no_progress: bool,

    /// テレメトリを無効化
    #[arg(long, default_value = "false")]
    no_telemetry: bool,

    /// 実行時間の計測と表示
    #[arg(long, default_value = "false")]
    timing: bool,

    /// 色付き出力を強制
    #[arg(long)]
    color: Option<String>,

    /// ワークスペースのルートディレクトリ
    #[arg(long)]
    workspace: Option<PathBuf>,

    /// パッケージマネージャのサブコマンド
    #[command(subcommand)]
    command: Commands,
}

/// パッケージマネージャのサブコマンド
#[derive(Subcommand)]
enum Commands {
    /// 新しいパッケージを作成
    New {
        /// パッケージ名
        #[arg(required = true)]
        name: String,
        
        /// テンプレート名
        #[arg(short, long)]
        template: Option<String>,
        
        /// ライブラリパッケージとして作成
        #[arg(short, long, default_value = "false")]
        lib: bool,
        
        /// バイナリパッケージとして作成
        #[arg(short, long, default_value = "false")]
        bin: bool,
        
        /// 作成先ディレクトリ
        #[arg(short, long)]
        directory: Option<PathBuf>,
        
        /// VCSの初期化をスキップ
        #[arg(long, default_value = "false")]
        no_vcs: bool,
        
        /// 依存関係の追加
        #[arg(short, long)]
        deps: Vec<String>,
    },

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
        
        /// ビルド依存として追加
        #[arg(short = 'b', long, default_value = "false")]
        build: bool,
        
        /// オプショナル依存として追加
        #[arg(short, long, default_value = "false")]
        optional: bool,
        
        /// 特定の機能フラグを有効化
        #[arg(short, long)]
        features: Vec<String>,
        
        /// すべての機能フラグを有効化
        #[arg(long, default_value = "false")]
        all_features: bool,
        
        /// デフォルト機能を無効化
        #[arg(long, default_value = "false")]
        no_default_features: bool,
        
        /// 特定のレジストリから追加
        #[arg(long)]
        registry: Option<String>,
        
        /// Gitリポジトリから追加
        #[arg(long)]
        git: Option<String>,
        
        /// ブランチ指定
        #[arg(long)]
        branch: Option<String>,
        
        /// タグ指定
        #[arg(long)]
        tag: Option<String>,
        
        /// コミットハッシュ指定
        #[arg(long)]
        rev: Option<String>,
        
        /// ローカルパスから追加
        #[arg(long)]
        path: Option<PathBuf>,
        
        /// 依存関係の更新をスキップ
        #[arg(long, default_value = "false")]
        no_update: bool,
    },
    
    /// 依存関係を削除
    Remove {
        /// パッケージ名
        #[arg(required = true)]
        name: String,
        
        /// 開発依存から削除
        #[arg(short, long, default_value = "false")]
        dev: bool,
        
        /// ビルド依存から削除
        #[arg(short = 'b', long, default_value = "false")]
        build: bool,
        
        /// 依存関係の更新をスキップ
        #[arg(long, default_value = "false")]
        no_update: bool,
    },
    
    /// 依存関係を更新
    Update {
        /// パッケージ名（省略時は全て更新）
        name: Option<String>,
        
        /// 更新するパッケージ（複数指定可）
        #[arg(short, long)]
        packages: Vec<String>,
        
        /// 特定の機能フラグを有効化
        #[arg(short, long)]
        features: Vec<String>,
        
        /// すべての機能フラグを有効化
        #[arg(long, default_value = "false")]
        all_features: bool,
        
        /// デフォルト機能を無効化
        #[arg(long, default_value = "false")]
        no_default_features: bool,
        
        /// ドライラン（実際には更新しない）
        #[arg(long, default_value = "false")]
        dry_run: bool,
        
        /// ロックファイルを無視して更新
        #[arg(long, default_value = "false")]
        force: bool,
        
        /// 互換性のある最新バージョンに更新
        #[arg(long, default_value = "false")]
        compatible: bool,
        
        /// メジャーバージョンも含めて最新に更新
        #[arg(long, default_value = "false")]
        latest: bool,
        
        /// 特定のワークスペースメンバーのみ更新
        #[arg(long)]
        workspace: Option<String>,
    },
    
    /// 依存関係を一覧表示
    List {
        /// 詳細表示
        #[arg(short, long, default_value = "false")]
        verbose: bool,
        
        /// 開発依存のみ表示
        #[arg(short, long, default_value = "false")]
        dev: bool,
        
        /// 直接依存のみ表示
        #[arg(short, long, default_value = "false")]
        direct: bool,
        
        /// 依存関係をツリー形式で表示
        #[arg(short, long, default_value = "false")]
        tree: bool,
        
        /// 特定のパッケージの依存関係のみ表示
        #[arg(short, long)]
        package: Option<String>,
        
        /// 出力形式（text, json, yaml）
        #[arg(short, long, default_value = "text")]
        format: String,
        
        /// 逆依存関係を表示（どのパッケージがこのパッケージに依存しているか）
        #[arg(short, long, default_value = "false")]
        reverse: bool,
        
        /// 特定の機能フラグを持つ依存のみ表示
        #[arg(short, long)]
        feature: Option<String>,
        
        /// 重複する依存関係を表示
        #[arg(long, default_value = "false")]
        duplicates: bool,
    },
    
    /// 依存関係を検索
    Search {
        /// 検索キーワード
        #[arg(required = true)]
        query: String,
        
        /// 検索結果の最大数
        #[arg(short, long, default_value = "10")]
        limit: usize,
        
        /// 検索結果のソート基準（downloads, recent-downloads, recent-updates, relevance）
        #[arg(short, long, default_value = "relevance")]
        sort: String,
        
        /// 特定のカテゴリで絞り込み
        #[arg(short, long)]
        category: Option<String>,
        
        /// 特定のキーワードで絞り込み
        #[arg(short, long)]
        keyword: Option<String>,
        
        /// 出力形式（text, json, yaml）
        #[arg(short, long, default_value = "text")]
        format: String,
        
        /// 詳細表示
        #[arg(short, long, default_value = "false")]
        verbose: bool,
        
        /// 特定のレジストリで検索
        #[arg(long)]
        registry: Option<String>,
    },
    
    /// パッケージの情報を表示
    Info {
        /// パッケージ名
        #[arg(required = true)]
        name: String,
        
        /// 特定のバージョン
        #[arg(short, long)]
        version: Option<String>,
        
        /// 出力形式（text, json, yaml）
        #[arg(short, long, default_value = "text")]
        format: String,
        
        /// 詳細表示
        #[arg(short, long, default_value = "false")]
        verbose: bool,
        
        /// 特定のレジストリから情報取得
        #[arg(long)]
        registry: Option<String>,
        
        /// 依存関係も表示
        #[arg(short, long, default_value = "false")]
        dependencies: bool,
        
        /// 逆依存関係も表示
        #[arg(short, long, default_value = "false")]
        reverse_dependencies: bool,
        
        /// ダウンロード統計を表示
        #[arg(long, default_value = "false")]
        downloads: bool,
        
        /// 脆弱性情報を表示
        #[arg(long, default_value = "false")]
        vulnerabilities: bool,
    },
    
    /// パッケージの公開
    Publish {
        /// ドライラン（実際には公開しない）
        #[arg(long, default_value = "false")]
        dry_run: bool,
        
        /// 公開前の検証をスキップ
        #[arg(long, default_value = "false")]
        no_verify: bool,
        
        /// 特定のレジストリに公開
        #[arg(long)]
        registry: Option<String>,
        
        /// トークンを指定
        #[arg(long)]
        token: Option<String>,
        
        /// 公開前に確認を求めない
        #[arg(long, default_value = "false")]
        no_confirm: bool,
        
        /// パッケージのパス（デフォルトはカレントディレクトリ）
        #[arg(short, long)]
        path: Option<PathBuf>,
        
        /// 既存バージョンの上書きを許可（管理者のみ）
        #[arg(long, default_value = "false")]
        allow_overwrite: bool,
    },
    
    /// パッケージのインストール
    Install {
        /// パッケージ名（複数指定可）
        #[arg(required = true)]
        packages: Vec<String>,
        
        /// グローバルにインストール
        #[arg(short, long, default_value = "false")]
        global: bool,
        
        /// 特定のバージョン
        #[arg(short, long)]
        version: Option<String>,
        
        /// 特定の機能フラグを有効化
        #[arg(short, long)]
        features: Vec<String>,
        
        /// すべての機能フラグを有効化
        #[arg(long, default_value = "false")]
        all_features: bool,
        
        /// デフォルト機能を無効化
        #[arg(long, default_value = "false")]
        no_default_features: bool,
        
        /// 特定のレジストリからインストール
        #[arg(long)]
        registry: Option<String>,
        
        /// Gitリポジトリからインストール
        #[arg(long)]
        git: Option<String>,
        
        /// ブランチ指定
        #[arg(long)]
        branch: Option<String>,
        
        /// タグ指定
        #[arg(long)]
        tag: Option<String>,
        
        /// コミットハッシュ指定
        #[arg(long)]
        rev: Option<String>,
        
        /// ローカルパスからインストール
        #[arg(long)]
        path: Option<PathBuf>,
        
        /// インストール先ディレクトリ
        #[arg(long)]
        target_dir: Option<PathBuf>,
        
        /// 既存のインストールを上書き
        #[arg(long, default_value = "false")]
        force: bool,
    },
    
    /// パッケージのアンインストール
    Uninstall {
        /// パッケージ名（複数指定可）
        #[arg(required = true)]
        packages: Vec<String>,
        
        /// グローバルからアンインストール
        #[arg(short, long, default_value = "false")]
        global: bool,
    },
    
    /// レジストリの管理
    Registry {
        /// レジストリのサブコマンド
        #[command(subcommand)]
        command: RegistryCommands,
    },
    
    /// キャッシュの管理
    Cache {
        /// キャッシュのサブコマンド
        #[command(subcommand)]
        command: CacheCommands,
    },
    
    /// 依存関係グラフの分析
    Graph {
        /// 出力形式（dot, json, svg, png）
        #[arg(short, long, default_value = "dot")]
        format: String,
        
        /// 出力ファイル（省略時は標準出力）
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// 開発依存を含める
        #[arg(short, long, default_value = "false")]
        dev: bool,
        
        /// ビルド依存を含める
        #[arg(short = 'b', long, default_value = "false")]
        build: bool,
        
        /// 特定のパッケージの依存関係のみ表示
        #[arg(short, long)]
        package: Option<String>,
        
        /// 依存関係の深さ制限
        #[arg(short, long)]
        depth: Option<usize>,
        
        /// 特定の機能フラグを持つ依存のみ表示
        #[arg(short, long)]
        feature: Option<String>,
    },
    
    /// パッケージの検証
    Verify {
        /// パッケージ名（省略時は現在のプロジェクト）
        name: Option<String>,
        
        /// 特定のバージョン
        #[arg(short, long)]
        version: Option<String>,
        
        /// セキュリティ脆弱性をチェック
        #[arg(long, default_value = "true")]
        security: bool,
        
        /// ライセンスをチェック
        #[arg(long, default_value = "true")]
        license: bool,
        
        /// 依存関係をチェック
        #[arg(long, default_value = "true")]
        dependencies: bool,
        
        /// パッケージの整合性をチェック
        #[arg(long, default_value = "true")]
        integrity: bool,
        
        /// 詳細な検証レポートを表示
        #[arg(short, long, default_value = "false")]
        verbose: bool,
        
        /// 出力形式（text, json, yaml）
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    
    /// ワークスペースの管理
    Workspace {
        /// ワークスペースのサブコマンド
        #[command(subcommand)]
        command: WorkspaceCommands,
    },
    
    /// プラグインの管理
    Plugin {
        /// プラグインのサブコマンド
        #[command(subcommand)]
        command: PluginCommands,
    },
    
    /// 設定の管理
    Config {
        /// 設定のサブコマンド
        #[command(subcommand)]
        command: ConfigCommands,
    },
    
    /// パッケージのビルド
    Build {
        /// ビルドターゲット（省略時は全て）
        target: Option<String>,
        
        /// リリースビルド
        #[arg(short, long, default_value = "false")]
        release: bool,
        
        /// 特定の機能フラグを有効化
        #[arg(short, long)]
        features: Vec<String>,
        
        /// すべての機能フラグを有効化
        #[arg(long, default_value = "false")]
        all_features: bool,
        
        /// デフォルト機能を無効化
        #[arg(long, default_value = "false")]
        no_default_features: bool,
        
        /// ビルド出力先ディレクトリ
        #[arg(long)]
        target_dir: Option<PathBuf>,
        
        /// 並列ジョブ数
        #[arg(short = 'j', long)]
        jobs: Option<usize>,
        
        /// 警告を表示しない
        #[arg(long, default_value = "false")]
        no_warnings: bool,
    },
    
    /// パッケージのクリーンアップ
    Clean {
        /// ターゲットディレクトリのみクリーン
        #[arg(long, default_value = "false")]
        target: bool,
        
        /// キャッシュディレクトリのみクリーン
        #[arg(long, default_value = "false")]
        cache: bool,
        
        /// 全てクリーン
        #[arg(long, default_value = "false")]
        all: bool,
    },
    
    /// パッケージのテスト
    Test {
        /// テスト名パターン
        #[arg(short, long)]
        test: Option<String>,
        
        /// リリースモードでテスト
        #[arg(short, long, default_value = "false")]
        release: bool,
        
        /// 特定の機能フラグを有効化
        #[arg(short, long)]
        features: Vec<String>,
        
        /// すべての機能フラグを有効化
        #[arg(long, default_value = "false")]
        all_features: bool,
        
        /// デフォルト機能を無効化
        #[arg(long, default_value = "false")]
        no_default_features: bool,
        
        /// 並列ジョブ数
        #[arg(short = 'j', long)]
        jobs: Option<usize>,
        
        /// テスト出力を詳細表示
        #[arg(short, long, default_value = "false")]
        verbose: bool,
    },
    
    /// パッケージのベンチマーク
    Bench {
        /// ベンチマーク名パターン
        #[arg(short, long)]
        bench: Option<String>,
        
        /// 特定の機能フラグを有効化
        #[arg(short, long)]
        features: Vec<String>,
        
        /// すべての機能フラグを有効化
        #[arg(long, default_value = "false")]
        all_features: bool,
        
        /// デフォルト機能を無効化
        #[arg(long, default_value = "false")]
        no_default_features: bool,
        
        /// 並列ジョブ数
        #[arg(short = 'j', long)]
        jobs: Option<usize>,
    },
    
    /// パッケージのドキュメント生成
    Doc {
        /// プライベート項目もドキュメント化
        #[arg(long, default_value = "false")]
        private: bool,
        
        /// ドキュメントをブラウザで開く
        #[arg(long, default_value = "false")]
        open: bool,
        
        /// 特定の機能フラグを有効化
        #[arg(short, long)]
        features: Vec<String>,
        
        /// すべての機能フラグを有効化
        #[arg(long, default_value = "false")]
        all_features: bool,
        
        /// デフォルト機能を無効化
        #[arg(long, default_value = "false")]
        no_default_features: bool,
    },
    
    /// パッケージの実行
    Run {
        /// 実行するバイナリ名
        #[arg(required = true)]
        name: String,
        
        /// 引数（-- の後に指定）
        #[arg(last = true)]
        args: Vec<String>,
        
        /// リリースモードで実行
        #[arg(short, long, default_value = "false")]
        release: bool,
        
        /// 特定の機能フラグを有効化
        #[arg(short, long)]
        features: Vec<String>,
        
        /// すべての機能フラグを有効化
        #[arg(long, default_value = "false")]
        all_features: bool,
        
        /// デフォルト機能を無効化
        #[arg(long, default_value = "false")]
        no_default_features: bool,
    },
    
    /// パッケージのフォーマット
    Fmt {
        /// チェックのみ（変更しない）
        #[arg(long, default_value = "false")]
        check: bool,
        
        /// 特定のファイルのみフォーマット
        #[arg(short, long)]
        files: Vec<PathBuf>,
        
        /// 再帰的にディレクトリ内をフォーマット
        #[arg(short, long, default_value = "true")]
        recursive: bool,
    },
    
    /// パッケージの静的解析
    Lint {
        /// 特定のファイルのみ解析
        #[arg(short, long)]
        files: Vec<PathBuf>,
        
        /// 特定のリントルールを無効化
        #[arg(long)]
        disable: Vec<String>,
        
        /// 警告を表示しない
        #[arg(long, default_value = "false")]
        no_warnings: bool,
        
        /// 修正可能な問題を自動修正
        #[arg(short, long, default_value = "false")]
        fix: bool,
    },
    
    /// パッケージのバージョン管理
    Version {
        /// 新しいバージョン（major, minor, patch, または具体的なバージョン）
        #[arg(required = true)]
        version: String,
        
        /// 変更をコミットしない
        #[arg(long, default_value = "false")]
        no_commit: bool,
        
        /// タグを作成しない
        #[arg(long, default_value = "false")]
        no_tag: bool,
        
        /// 変更履歴を更新
        #[arg(long, default_value = "true")]
        changelog: bool,
    },
    
    /// パッケージのエクスポート
    Export {
        /// エクスポート形式（zip, tar, dir）
        #[arg(short, long, default_value = "zip")]
        format: String,
        
        /// 出力先ファイルまたはディレクトリ
        #[arg(short, long, required = true)]
        output: PathBuf,
        
        /// 開発依存を含める
        #[arg(short, long, default_value = "false")]
        dev: bool,
        
        /// ソースコードを含める
        #[arg(long, default_value = "true")]
        source: bool,
    },
    
    /// パッケージのインポート
    Import {
        /// インポート元ファイルまたはディレクトリ
        #[arg(required = true)]
        source: PathBuf,
        
        /// インポート先ディレクトリ
        #[arg(short, long)]
        target: Option<PathBuf>,
        
        /// 既存のファイルを上書き
        #[arg(long, default_value = "false")]
        force: bool,
    },
    
    /// パッケージの依存関係監査
    Audit {
        /// セキュリティ脆弱性のみチェック
        #[arg(long, default_value = "false")]
        security: bool,
        
        /// ライセンスのみチェック
        #[arg(long, default_value = "false")]
        license: bool,
        
        /// 依存関係のみチェック
        #[arg(long, default_value = "false")]
        dependencies: bool,
        
        /// 詳細な監査レポートを表示
        #[arg(long, default_value = "false")]
        verbose: bool,
    },
    
    /// パッケージのセキュリティ監査
    SecurityAudit {
        /// セキュリティ脆弱性のみチェック
        #[arg(long, default_value = "false")]
        security: bool,
        
        /// 詳細な監査レポートを表示
        #[arg(long, default_value = "false")]
        verbose: bool,
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
        Commands::Add { name, version, dev, build, optional, features, all_features, no_default_features, registry, git, branch, tag, rev, path, no_update } => {
            let version_str = version.as_ref().map_or("最新", |v| v.as_str());
            info!("パッケージの追加: {} ({})", name, version_str);
            
            let dep_type = if *dev { 
                "開発依存関係" 
            } else if *build { 
                "ビルド依存関係" 
            } else if *optional { 
                "オプション依存関係" 
            } else { 
                "通常依存関係" 
            };
            info!("パッケージタイプ: {}", dep_type);
            
            // 依存関係ソースの決定
            let source = if let Some(git_url) = git {
                info!("Gitリポジトリから依存関係を追加: {}", git_url);
                let git_ref = if let Some(branch_name) = branch {
                    info!("ブランチ: {}", branch_name);
                    Some(format!("branch={}", branch_name))
                } else if let Some(tag_name) = tag {
                    info!("タグ: {}", tag_name);
                    Some(format!("tag={}", tag_name))
                } else if let Some(rev_hash) = rev {
                    info!("リビジョン: {}", rev_hash);
                    Some(format!("rev={}", rev_hash))
                } else {
                    None
                };
                DependencySource::Git(git_url.clone(), git_ref)
            } else if let Some(path_str) = path {
                info!("ローカルパスから依存関係を追加: {}", path_str.display());
                DependencySource::Path(path_str.clone())
            } else if let Some(registry_url) = registry {
                info!("カスタムレジストリから依存関係を追加: {}", registry_url);
                DependencySource::Registry(registry_url.clone())
            } else {
                info!("デフォルトレジストリから依存関係を追加");
                DependencySource::DefaultRegistry
            };
            
            // 機能フラグの処理
            let feature_config = FeatureConfig {
                specific_features: features.clone(),
                all_features: *all_features,
                no_default_features: *no_default_features,
            };
            
            if !feature_config.specific_features.is_empty() {
                info!("指定された機能: {}", feature_config.specific_features.join(", "));
            }
            
            if feature_config.all_features {
                info!("すべての機能を有効化");
            }
            
            if feature_config.no_default_features {
                info!("デフォルト機能を無効化");
            }
            
            // 依存関係の追加
            let dependency_options = DependencyOptions {
                name: name.clone(),
                version: version.clone(),
                source,
                dependency_type: if *dev {
                    DependencyType::Development
                } else if *build {
                    DependencyType::Build
                } else if *optional {
                    DependencyType::Optional
                } else {
                    DependencyType::Normal
                },
                feature_config,
                update_lockfile: !no_update,
            };
            
            let result = dependency::add_dependency(dependency_options)?;
            info!("パッケージの追加が完了しました: {}", result);
            
            // 依存関係グラフの検証
            info!("依存関係グラフの検証を実行中...");
            let validation_result = dependency::validate_dependency_graph()?;
            if validation_result.has_issues() {
                warn!("依存関係グラフに問題が見つかりました:");
                for issue in validation_result.issues {
                    warn!("  - {}", issue);
                }
            } else {
                info!("依存関係グラフの検証が完了しました。問題は見つかりませんでした。");
            }
            
            // セキュリティ監査の実行（オプション）
            if !*no_update {
                info!("新しい依存関係のセキュリティ監査を実行中...");
                let audit_result = security::audit_dependency(name, version.as_deref())?;
                if audit_result.has_vulnerabilities() {
                    warn!("セキュリティ脆弱性が見つかりました:");
                    for vuln in audit_result.vulnerabilities {
                        warn!("  - {}: {}", vuln.severity, vuln.description);
                        warn!("    影響するバージョン: {}", vuln.affected_versions);
                        if let Some(fix_version) = vuln.fixed_version {
                            warn!("    修正バージョン: {}", fix_version);
                        }
                    }
                } else {
                    info!("セキュリティ脆弱性は見つかりませんでした。");
                }
            }
        },
        Commands::Update { name, packages, features, all_features, no_default_features, dry_run, force, compatible, latest, workspace } => {
            // 更新対象の決定
            let update_targets = if let Some(pkg_name) = name {
                info!("パッケージの更新: {}", pkg_name);
                vec![pkg_name.clone()]
            } else if !packages.is_empty() {
                info!("指定されたパッケージの更新: {}", packages.join(", "));
                packages.clone()
            } else if *workspace {
                info!("ワークスペース内の全パッケージの更新");
                dependency::list_workspace_packages()?
            } else {
                info!("全パッケージの更新");
                vec![]
            };
            
            // 更新モードの決定
            let update_mode = if *latest {
                UpdateMode::Latest
            } else if *compatible {
                UpdateMode::Compatible
            } else {
                UpdateMode::Default
            };
            
            info!("更新モード: {}", match update_mode {
                UpdateMode::Latest => "最新バージョン（互換性を無視）",
                UpdateMode::Compatible => "互換性のある最新バージョン",
                UpdateMode::Default => "セマンティックバージョニングに従った更新",
            });
            
            // 機能フラグの処理
            let feature_config = FeatureConfig {
                specific_features: features.clone(),
                all_features: *all_features,
                no_default_features: *no_default_features,
            };
            
            // 更新オプションの構築
            let update_options = UpdateOptions {
                targets: update_targets,
                mode: update_mode,
                feature_config,
                dry_run: *dry_run,
                force: *force,
            };
            
            if *dry_run {
                info!("ドライラン: 実際の更新は行われません");
            }
            
            // 更新の実行
            let update_results = dependency::update_dependencies(update_options)?;
            
            // 結果の表示
            if update_results.is_empty() {
                info!("更新するパッケージはありませんでした");
            } else {
                info!("更新されたパッケージ:");
                for result in &update_results {
                    info!("  {} {} -> {}", result.name, result.old_version, result.new_version);
                    if !result.breaking_changes.is_empty() {
                        warn!("  破壊的変更の可能性:");
                        for change in &result.breaking_changes {
                            warn!("    - {}", change);
                        }
                    }
                }
                
                // 更新後の依存関係グラフの検証
                if !*dry_run {
                    info!("依存関係グラフの検証を実行中...");
                    let validation_result = dependency::validate_dependency_graph()?;
                    if validation_result.has_issues() {
                        warn!("依存関係グラフに問題が見つかりました:");
                        for issue in validation_result.issues {
                            warn!("  - {}", issue);
                        }
                    } else {
                        info!("依存関係グラフの検証が完了しました。問題は見つかりませんでした。");
                    }
                }
            }
            
            info!("更新が完了しました");
        },
        Commands::List => {
            info!("インストール済みパッケージ一覧:");
            let dependencies = dependency::list_dependencies()?;
            
            if dependencies.is_empty() {
                info!("  パッケージはインストールされていません");
            } else {
                for (name, version, audit) in dependencies {
                    let status_icon = match audit {
                        Some(SecurityAudit::Vulnerable(severity)) => {
                            match severity.as_str() {
                                "critical" => "🔴",
                                "high" => "🟠",
                                "medium" => "🟡",
                                "low" => "🟢",
                                _ => "⚠️",
                            }
                        },
                        Some(SecurityAudit::Outdated) => "📦",
                        Some(SecurityAudit::LicenseIssue) => "⚖️",
                        None => "✅",
                    };
                    
                    info!("  {} {} ({})", status_icon, name, version);
                    
                    if let Some(SecurityAudit::Vulnerable(severity)) = audit {
                        warn!("    セキュリティ脆弱性 ({})", severity);
                    } else if let Some(SecurityAudit::Outdated) = audit {
                        warn!("    新しいバージョンが利用可能です");
                    } else if let Some(SecurityAudit::LicenseIssue) = audit {
                        warn!("    ライセンスの互換性に問題があります");
                    }
                }
            }
        },
        Commands::Search { query, limit, sort, category, keyword, format, verbose, registry } => {
            info!("パッケージの検索: {}", query);
            
            // 検索オプションの構築
            let search_options = SearchOptions {
                query: query.clone(),
                limit: *limit,
                sort_by: match sort.as_deref() {
                    Some("downloads") => SortBy::Downloads,
                    Some("recent-downloads") => SortBy::RecentDownloads,
                    Some("recent-updates") => SortBy::RecentUpdates,
                    Some("relevance") => SortBy::Relevance,
                    _ => SortBy::Relevance,
                },
                categories: category.clone(),
                keywords: keyword.clone(),
                registry: registry.clone(),
                verbose: *verbose,
            };
            
            // 検索の実行
            let results = registry::search_packages(search_options)?;
            
            // 結果の表示
            match format.as_deref() {
                Some("json") => {
                    // JSON形式で出力
                    let json = serde_json::to_string_pretty(&results)?;
                    println!("{}", json);
                },
                Some("table") | _ => {
                    // テーブル形式で出力
                    info!("検索結果 ({} 件):", results.len());
                    
                    if results.is_empty() {
                        info!("  検索条件に一致するパッケージは見つかりませんでした");
                    } else {
                        // ヘッダーの表示
                        println!("{:<30} | {:<15} | {:<10} | {:<40}", "パッケージ名", "最新バージョン", "ダウンロード数", "説明");
                        println!("{}", "-".repeat(100));
                        
                        // 結果の表示
                        for package in results {
                            let description = if package.description.len() > 40 {
                                format!("{}...", &package.description[..37])
                            } else {
                                package.description.clone()
                            };
                            
                            println!("{:<30} | {:<15} | {:<10} | {:<40}",
                                package.name,
                                package.version,
                                package.downloads,
                                description
                            );
                            
                            if *verbose {
                                println!("  作者: {}", package.author);
                                println!("  ライセンス: {}", package.license);
                                println!("  カテゴリ: {}", package.categories.join(", "));
                                println!("  キーワード: {}", package.keywords.join(", "));
                                println!("  リポジトリ: {}", package.repository.unwrap_or_default());
                                println!();
                            }
                        }
                    }
                }
            }
        },
        Commands::Info { name, version, format, verbose, registry, dependencies, reverse_dependencies, downloads, vulnerabilities } => {
            info!("パッケージ情報の取得: {}", name);
            
            // 情報取得オプションの構築
            let info_options = PackageInfoOptions {
                name: name.clone(),
                version: version.clone(),
                registry: registry.clone(),
                include_dependencies: *dependencies,
                include_reverse_dependencies: *reverse_dependencies,
                include_download_stats: *downloads,
                include_vulnerabilities: *vulnerabilities,
                verbose: *verbose,
            };
            
            // パッケージ情報の取得
            let pkg_info = registry::get_package_info(info_options)?;
            
            // 結果の表示
            match format.as_deref() {
                Some("json") => {
                    // JSON形式で出力
                    let json = serde_json::to_string_pretty(&pkg_info)?;
                    println!("{}", json);
                },
                Some("markdown") => {
                    // Markdown形式で出力
                    println!("# {} v{}", pkg_info.name, pkg_info.version);
                    println!();
                    println!("{}", pkg_info.description);
                    println!();
                    println!("**作者:** {}", pkg_info.author);
                    println!("**ライセンス:** {}", pkg_info.license);
                    println!("**ダウンロード数:** {}", pkg_info.downloads);
                    println!();
                    
                    if !pkg_info.dependencies.is_empty() {
                        println!("## 依存関係");
                        for dep in &pkg_info.dependencies {
                            println!("- {}", dep);
                        }
                        println!();
                    }
                    
                    if !pkg_info.reverse_dependencies.is_empty() {
                        println!("## 逆依存関係");
                        for dep in &pkg_info.reverse_dependencies {
                            println!("- {}", dep);
                        }
                        println!();
                    }
                    
                    if !pkg_info.vulnerabilities.is_empty() {
                        println!("## セキュリティ脆弱性");
                        for vuln in &pkg_info.vulnerabilities {
                            println!("### {} ({})", vuln.id, vuln.severity);
                            println!("{}", vuln.description);
                            println!("**影響するバージョン:** {}", vuln.affected_versions);
                            if let Some(fix) = &vuln.fixed_version {
                                println!("**修正バージョン:** {}", fix);
                            }
                            println!();
                        }
                    }
                },
                _ => {
                    // 通常の表示形式
                    info!("パッケージ: {}", pkg_info.name);
                    info!("バージョン: {}", pkg_info.version);
                    info!("説明: {}", pkg_info.description);
                    info!("作者: {}", pkg_info.author);
                    info!("ライセンス: {}", pkg_info.license);
                    info!("ダウンロード数: {}", pkg_info.downloads);
                    
                    if !pkg_info.dependencies.is_empty() {
                        info!("依存関係:");
                        for dep in &pkg_info.dependencies {
                            info!("  {}", dep);
                        }
                    }
                    
                    if !pkg_info.reverse_dependencies.is_empty() {
                        info!("逆依存関係:");
                        for dep in &pkg_info.reverse_dependencies {
                            info!("  {}", dep);
                        }
                    }
                    
                    if !pkg_info.vulnerabilities.is_empty() {
                        warn!("セキュリティ脆弱性:");
                        for vuln in &pkg_info.vulnerabilities {
                            warn!("  {} ({})", vuln.id, vuln.severity);
                            warn!("    説明: {}", vuln.description);
                            warn!("    影響するバージョン: {}", vuln.affected_versions);
                            if let Some(fix) = &vuln.fixed_version {
                                warn!("    修正バージョン: {}", fix);
                            }
                        }
                    }
                }
            }
        },
        Commands::Publish { dry_run, no_verify, registry, token, no_confirm, path, allow_overwrite } => {
            info!("パッケージの公開を開始します...");
            
            // パッケージのパスを決定
            let package_path = path.clone().unwrap_or_else(|| PathBuf::from("."));
            info!("パッケージパス: {}", package_path.display());
            
            // パッケージの検証
            if !*no_verify {
                info!("パッケージの検証を実行中...");
                let verification_result = package::verify_package(&package_path)?;
                
                if verification_result.has_issues() {
                    error!("パッケージの検証に失敗しました:");
                    for issue in verification_result.issues {
                        error!("  - {}", issue);
                    }
                    return Err(anyhow!("パッケージの検証に失敗しました"));
                }
                
                info!("パッケージの検証が完了しました");
            } else {
                warn!("パッケージの検証をスキップします");
            }
            
            // 公開オプションの構築
            let publish_options = PublishOptions {
                package_path,
                registry: registry.clone(),
                token: token.clone(),
                dry_run: *dry_run,
                no_confirm: *no_confirm,
                allow_overwrite: *allow_overwrite,
            };
            
            // 確認プロンプト
            if !*no_confirm && !*dry_run {
                let package_info = package::get_package_metadata(&publish_options.package_path)?;
                info!("以下のパッケージを公開します:");
                info!("  名前: {}", package_info.name);
                info!("  バージョン: {}", package_info.version);
                info!("  説明: {}", package_info.description);
                
                if !confirm("パッケージを公開しますか？")? {
                    info!("パッケージの公開をキャンセルしました");
                    return Ok(());
                }
            }
            
            if *dry_run {
                info!("ドライラン: 実際の公開は行われません");
            }
            
            // パッケージの公開
            let publish_result = registry::publish_package(publish_options)?;
            
            if *dry_run {
                info!("ドライラン完了: パッケージは公開されていません");
            } else {
                info!("パッケージの公開が完了しました");
                info!("公開URL: {}", publish_result.package_url);
                info!("バージョン: {}", publish_result.version);
                info!("公開日時: {}", publish_result.published_at);
            }
        },
        Commands::Registry { command } => {
            match command {
                RegistryCommands::Add { name, url, token, default } => {
                    info!("レジストリの追加: {} ({})", name, url);
                    
                    let registry_config = RegistryConfig {
                        name: name.clone(),
                        url: url.clone(),
                        token: token.clone(),
                        is_default: *default,
                    };
                    
                    registry::add_registry(registry_config)?;
                    
                    if *default {
                        info!("デフォルトレジストリとして設定されました");
                    }
                    
                    info!("レジストリの追加が完了しました");
                },
                RegistryCommands::List => {
                    info!("登録済みレジストリ一覧:");
                    
                    let registries = registry::list_registries()?;
                    
                    if registries.is_empty() {
                        info!("  登録済みレジストリはありません");
                    } else {
                        for reg in registries {
                            let default_marker = if reg.is_default { " (デフォルト)" } else { "" };
                            info!("  {} - {}{}", reg.name, reg.url, default_marker);
                        }
                    }
                },
                RegistryCommands::Remove { name } => {
                    info!("レジストリの削除: {}", name);
                    
                    if confirm(&format!("レジストリ '{}' を削除しますか？", name))? {
                        registry::remove_registry(name)?;
                        info!("レジストリの削除が完了しました");
                    } else {
                        info!("レジストリの削除をキャンセルしました");
                    }
                },
                RegistryCommands::SetDefault { name } => {
                    info!("デフォルトレジストリの設定: {}", name);
                    registry::set_default_registry(name)?;
                    info!("デフォルトレジストリの設定が完了しました");
                },
                RegistryCommands::Login { name, token } => {
                    let registry_name = name.as_deref().unwrap_or("default");
                    info!("レジストリへのログイン: {}", registry_name);
                    
                    let token_value = if let Some(token_str) = token {
                        token_str.clone()
                    } else {
                        // トークンの入力を促す
                        rpassword::prompt_password("認証トークンを入力してください: ")?
                    };
                    
                    registry::login_to_registry(registry_name, &token_value)?;
                    info!("レジストリへのログインが完了しました");
                },
                RegistryCommands::Logout { name } => {
                    let registry_name = name.as_deref().unwrap_or("default");
                    info!("レジストリからのログアウト: {}", registry_name);
                    
                    if confirm(&format!("レジストリ '{}' からログアウトしますか？", registry_name))? {
                        registry::logout_from_registry(registry_name)?;
                        info!("レジストリからのログアウトが完了しました");
                    } else {
                        info!("レジストリからのログアウトをキャンセルしました");
                    }
                },
            }
        },
        Commands::Audit { security, license, dependencies, verbose } => {
            info!("パッケージの監査を開始します...");
            
            let audit_options = AuditOptions {
                check_security: *security || (!*security && !*license && !*dependencies),
                check_license: *license || (!*security && !*license && !*dependencies),
                check_dependencies: *dependencies || (!*security && !*license && !*dependencies),
                verbose: *verbose,
            };
            
            let audit_result = security::audit_package(audit_options)?;
            
            // 結果の表示
            if audit_result.vulnerabilities.is_empty() && 
               audit_result.license_issues.is_empty() && 
               audit_result.dependency_issues.is_empty() {
                info!("監査が完了しました。問題は見つかりませんでした。");
            } else {
                warn!("監査が完了しました。以下の問題が見つかりました:");
                
                if !audit_result.vulnerabilities.is_empty() {
                    warn!("セキュリティ脆弱性 ({} 件):", audit_result.vulnerabilities.len());
                    for vuln in &audit_result.vulnerabilities {
                        warn!("  {} - {} ({})", vuln.package_name, vuln.description, vuln.severity);
                        warn!("    影響するバージョン: {}", vuln.affected_versions);
                        if let Some(fix) = &vuln.fixed_version {
                            warn!("    修正バージョン: {}", fix);
                        }
                        if *verbose {
                            warn!("    詳細: {}", vuln.details);
                            if let Some(url) = &vuln.advisory_url {
                                warn!("    アドバイザリURL: {}", url);
                            }
                        }
                    }
                }
                
                if !audit_result.license_issues.is_empty() {
                    warn!("ライセンス問題 ({} 件):", audit_result.license_issues.len());
                    for issue in &audit_result.license_issues {
                        warn!("  {} - {}", issue.package_name, issue.description);
                        if *verbose {
                            warn!("    現在のライセンス: {}", issue.current_license);
                            warn!("    推奨ライセンス: {}", issue.recommended_license);
                            warn!("    詳細: {}", issue.details);
                        }
                    }
                }
                
                if !audit_result.dependency_issues.is_empty() {
                    warn!("依存関係問題 ({} 件):", audit_result.dependency_issues.len());
                    for issue in &audit_result.dependency_issues {
                        warn!("  {} - {}", issue.package_name, issue.description);
                        if *verbose {
                            warn!("    詳細: {}", issue.details);
                            if let Some(recommendation) = &issue.recommendation {
                                warn!("    推奨対応: {}", recommendation);
                            }
                        }
                    }
                }
            }
        },
        Commands::SecurityAudit { security, verbose } => {
            info!("パッケージのセキュリティ監査を開始します...");
            
            let audit_options = SecurityAuditOptions {
                verbose: *verbose,
            };
            
            let audit_result = security::security_audit_package(audit_options)?;
            
            // 結果の表示
            if audit_result.vulnerabilities.is_empty() {
                info!("セキュリティ監査が完了しました。脆弱性は見つかりませんでした。");
            } else {
                warn!("セキュリティ監査が完了しました。以下の脆弱性が見つかりました:");
                
                // 重要度別に脆弱性をグループ化
                let mut critical = Vec::new();
                let mut high = Vec::new();
                let mut medium = Vec::new();
                let mut low = Vec::new();
                let mut unknown = Vec::new();
                
                for vuln in &audit_result.vulnerabilities {
                    match vuln.severity.as_str() {
                        "critical" => critical.push(vuln),
                        "high" => high.push(vuln),
                        "medium" => medium.push(vuln),
                        "low" => low.push(vuln),
                        _ => unknown.push(vuln),
                    }
                }
                
                // 重要度別に表示
                if !critical.isEmpty() {
                    warn!("重大な脆弱性 ({} 件):", critical.len());
                    for vuln in critical {
                        display_vulnerability(vuln, *verbose);
                    }
                }
                
                if !high.isEmpty() {
                    warn!("高リスクの脆弱性 ({} 件):", high.len());
                    for vuln in high {
                        display_vulnerability(vuln, *verbose);
                    }
                }
                
                if !medium.isEmpty() {
                    warn!("中リスクの脆弱性 ({} 件):", medium.len());
                    for vuln in medium {
                        display_vulnerability(vuln, *verbose);
                    }
                }
                
                if !low.isEmpty() {
                    warn!("低リスクの脆弱性 ({} 件):", low.len());
                    for vuln in low {
                        display_vulnerability(vuln, *verbose);
                    }
                }
                
                if !unknown.isEmpty() {
                    warn!("不明な重要度の脆弱性 ({} 件):", unknown.len());
                    for vuln in unknown {
                        display_vulnerability(vuln, *verbose);
                    }
                }
                
                // 修正推奨事項
                info!("推奨対応:");
                info!("  - 影響を受ける依存関係を更新してください");
                info!("  - または、脆弱性の修正されたバージョンに更新してください");
            }
        },
        _ => {
            info!("コマンドを実行: {}", cli.command);
        }
    }
    
    Ok(())
}

/// ログ設定を初期化
fn setup_logging(verbose: bool, quiet: bool) -> Result<(), Box<dyn std::error::Error>> {
    // env_loggerを使用してログレベルを設定
    let mut builder = env_logger::Builder::from_env(env_logger::Env::default());
    
    // ログレベルを設定
    if verbose {
        builder.filter_level(log::LevelFilter::Debug);
        builder.init();
        debug!("詳細ログモードが有効になりました");
    } else if quiet {
        builder.filter_level(log::LevelFilter::Error);
        builder.init();
    } else {
        builder.filter_level(log::LevelFilter::Info);
        builder.init();
    }
    Ok(())
}

use std::fmt;
use std::error::Error as StdError;
use std::io;
use std::path::PathBuf;
use thiserror::Error;
use std::sync::Arc;
use std::collections::HashMap;
use semver::{Version, VersionReq};
use url::Url;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

/// パッケージマネージャーのエラー型
/// 
/// SwiftLightパッケージマネージャーで発生する可能性のあるすべてのエラーを表現します。
/// エラーは詳細な診断情報、修正提案、およびコンテキスト情報を含みます。
#[derive(Error, Debug, Clone)]
pub enum PackageError {
    /// IO操作に関連するエラー
    #[error("IO エラー: {source} ({operation})")]
    IOError {
        #[source]
        source: Arc<io::Error>,
        /// エラーが発生した操作の説明
        operation: String,
        /// 関連するファイルパス
        path: Option<PathBuf>,
        /// 推奨される解決策
        suggestions: Vec<String>,
    },
    
    /// パーサーエラー
    #[error("パーサーエラー: {message} (行: {line}, 列: {column})")]
    ParseError {
        /// エラーメッセージ
        message: String,
        /// エラーが発生したファイル
        file: Option<PathBuf>,
        /// エラーが発生した行番号
        line: usize,
        /// エラーが発生した列番号
        column: usize,
        /// 問題のあるコードスニペット
        snippet: Option<String>,
        /// 推奨される修正
        suggestions: Vec<String>,
    },
    
    /// ネットワークエラー
    #[error("ネットワークエラー: {message} - URL: {url:?}")]
    NetworkError {
        /// エラーメッセージ
        message: String,
        /// 問題のあったURL
        url: Option<Url>,
        /// HTTPステータスコード（該当する場合）
        status_code: Option<u16>,
        /// リトライ情報
        retry_info: Option<RetryInfo>,
        /// プロキシ設定の問題かどうか
        proxy_related: bool,
        /// 推奨される解決策
        suggestions: Vec<String>,
    },
    
    /// パッケージが見つからない
    #[error("パッケージ '{name}' が見つかりません{}", match version { Some(v) => format!(" (要求バージョン: {})", v), None => String::new() })]
    PackageNotFound {
        /// パッケージ名
        name: String,
        /// 要求されたバージョン（オプション）
        version: Option<String>,
        /// 検索されたレジストリ
        registries: Vec<String>,
        /// 類似のパッケージ名の提案
        similar_packages: Vec<String>,
        /// 推奨される解決策
        suggestions: Vec<String>,
    },
    
    /// バージョンが見つからない
    #[error("パッケージ '{name}' のバージョン '{version}' が見つかりません - 利用可能なバージョン: {available_versions:?}")]
    VersionNotFound {
        /// パッケージ名
        name: String,
        /// 要求されたバージョン
        version: String,
        /// 利用可能なバージョンのリスト
        available_versions: Vec<String>,
        /// 最新のバージョン
        latest_version: Option<String>,
        /// 推奨される解決策
        suggestions: Vec<String>,
    },
    
    /// 依存関係の解決エラー
    #[error("依存関係の解決エラー: {message}")]
    ResolutionError {
        /// エラーメッセージ
        message: String,
        /// 依存関係グラフの問題箇所
        dependency_graph: Option<DependencyGraphError>,
        /// バージョン制約の衝突
        version_conflicts: Vec<VersionConflict>,
        /// 循環依存関係
        circular_dependencies: Vec<Vec<String>>,
        /// 推奨される解決策
        suggestions: Vec<String>,
    },
    
    /// セキュリティエラー
    #[error("セキュリティエラー: {message}")]
    SecurityError {
        /// エラーメッセージ
        message: String,
        /// セキュリティ脆弱性の詳細
        vulnerability: Option<VulnerabilityInfo>,
        /// 署名検証の問題
        signature_issue: Option<SignatureIssue>,
        /// 権限の問題
        permission_issue: Option<PermissionIssue>,
        /// 推奨される解決策
        suggestions: Vec<String>,
        /// セキュリティアドバイザリURL
        advisory_url: Option<Url>,
    },
    
    /// 設定エラー
    #[error("設定エラー: {message}")]
    ConfigError {
        /// エラーメッセージ
        message: String,
        /// 問題のある設定ファイル
        config_file: Option<PathBuf>,
        /// 問題のある設定キー
        config_key: Option<String>,
        /// 無効な値
        invalid_value: Option<String>,
        /// 期待される値の形式
        expected_format: Option<String>,
        /// 推奨される解決策
        suggestions: Vec<String>,
    },
    
    /// レジストリエラー
    #[error("レジストリエラー: {message}")]
    RegistryError {
        /// エラーメッセージ
        message: String,
        /// 問題のあるレジストリURL
        registry_url: Option<Url>,
        /// 認証の問題
        authentication_issue: bool,
        /// レジストリのステータス情報
        registry_status: Option<RegistryStatus>,
        /// 推奨される解決策
        suggestions: Vec<String>,
    },
    
    /// ユーザー対話エラー
    #[error("ユーザー対話エラー: {message}")]
    InteractionError {
        /// エラーメッセージ
        message: String,
        /// 必要な入力の種類
        input_type: Option<String>,
        /// 推奨される解決策
        suggestions: Vec<String>,
    },
    
    /// ファイルシステムエラー
    #[error("ファイルシステムエラー: パス '{path}' - {message}")]
    FilesystemError {
        /// 問題のあるファイルパス
        path: PathBuf,
        /// エラーメッセージ
        message: String,
        /// ファイルシステムの操作種類
        operation: FilesystemOperation,
        /// ファイルの権限問題
        permission_issue: bool,
        /// ディスク容量の問題
        disk_space_issue: bool,
        /// 推奨される解決策
        suggestions: Vec<String>,
    },
    
    /// ビルドエラー
    #[error("ビルドエラー: {message}")]
    BuildError {
        /// エラーメッセージ
        message: String,
        /// ビルド対象のパッケージ
        package: String,
        /// ビルドコマンド
        build_command: Option<String>,
        /// ビルドログ
        build_log: Option<String>,
        /// 依存関係の問題
        dependency_issues: Vec<String>,
        /// 推奨される解決策
        suggestions: Vec<String>,
    },
    
    /// パッケージ検証エラー
    #[error("パッケージ検証エラー: {message}")]
    ValidationError {
        /// エラーメッセージ
        message: String,
        /// 検証に失敗したパッケージ
        package: String,
        /// 検証の種類
        validation_type: ValidationType,
        /// 検証の詳細
        details: HashMap<String, String>,
        /// 推奨される解決策
        suggestions: Vec<String>,
    },
    
    /// パッケージ公開エラー
    #[error("パッケージ公開エラー: {message}")]
    PublishError {
        /// エラーメッセージ
        message: String,
        /// 公開しようとしたパッケージ
        package: String,
        /// 公開先レジストリ
        registry: Option<String>,
        /// バージョン衝突
        version_conflict: bool,
        /// 権限の問題
        permission_issue: bool,
        /// 推奨される解決策
        suggestions: Vec<String>,
    },
    
    /// キャッシュエラー
    #[error("キャッシュエラー: {message}")]
    CacheError {
        /// エラーメッセージ
        message: String,
        /// キャッシュの場所
        cache_path: Option<PathBuf>,
        /// キャッシュ操作の種類
        operation: CacheOperation,
        /// キャッシュの破損
        corruption: bool,
        /// 推奨される解決策
        suggestions: Vec<String>,
    },
    
    /// ロックファイルエラー
    #[error("ロックファイルエラー: {message}")]
    LockfileError {
        /// エラーメッセージ
        message: String,
        /// ロックファイルのパス
        lockfile_path: Option<PathBuf>,
        /// ロックファイルの操作
        operation: LockfileOperation,
        /// ロックファイルの破損
        corruption: bool,
        /// 推奨される解決策
        suggestions: Vec<String>,
    },
    
    /// プラグインエラー
    #[error("プラグインエラー: {message}")]
    PluginError {
        /// エラーメッセージ
        message: String,
        /// プラグイン名
        plugin_name: String,
        /// プラグインバージョン
        plugin_version: Option<String>,
        /// プラグインの操作
        operation: PluginOperation,
        /// 推奨される解決策
        suggestions: Vec<String>,
    },
    
    /// ワークスペースエラー
    #[error("ワークスペースエラー: {message}")]
    WorkspaceError {
        /// エラーメッセージ
        message: String,
        /// ワークスペースのルートパス
        workspace_root: Option<PathBuf>,
        /// 問題のあるメンバーパッケージ
        member_package: Option<String>,
        /// 推奨される解決策
        suggestions: Vec<String>,
    },
    
    /// 並行処理エラー
    #[error("並行処理エラー: {message}")]
    ConcurrencyError {
        /// エラーメッセージ
        message: String,
        /// 並行操作の種類
        operation: String,
        /// デッドロックの可能性
        potential_deadlock: bool,
        /// 推奨される解決策
        suggestions: Vec<String>,
    },
    
    /// メモリ不足エラー
    #[error("メモリ不足エラー: {message}")]
    OutOfMemoryError {
        /// エラーメッセージ
        message: String,
        /// 要求されたメモリ量（バイト）
        requested_memory: Option<usize>,
        /// 利用可能なメモリ量（バイト）
        available_memory: Option<usize>,
        /// 推奨される解決策
        suggestions: Vec<String>,
    },
    
    /// タイムアウトエラー
    #[error("タイムアウトエラー: {message}")]
    TimeoutError {
        /// エラーメッセージ
        message: String,
        /// タイムアウトした操作
        operation: String,
        /// タイムアウト時間（ミリ秒）
        timeout_ms: u64,
        /// 推奨される解決策
        suggestions: Vec<String>,
    },
    
    /// その他のエラー
    #[error("{message}")]
    Other {
        /// エラーメッセージ
        message: String,
        /// エラーの詳細情報
        details: Option<HashMap<String, String>>,
        /// 推奨される解決策
        suggestions: Vec<String>,
    },
}

/// リトライ情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryInfo {
    /// リトライ回数
    pub attempts: usize,
    /// 最大リトライ回数
    pub max_attempts: usize,
    /// 次のリトライまでの待機時間（ミリ秒）
    pub next_retry_ms: u64,
    /// バックオフ戦略
    pub backoff_strategy: String,
}

/// 依存関係グラフエラー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraphError {
    /// 問題のあるパッケージ
    pub package: String,
    /// 問題の説明
    pub description: String,
    /// 依存関係パス
    pub dependency_path: Vec<String>,
}

/// バージョン衝突
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionConflict {
    /// パッケージ名
    pub package: String,
    /// 要求されたバージョン制約
    pub requested_constraints: HashMap<String, String>,
    /// 解決不可能な理由
    pub reason: String,
}

/// 脆弱性情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VulnerabilityInfo {
    /// 脆弱性ID
    pub id: String,
    /// 脆弱性の説明
    pub description: String,
    /// 深刻度（CVSS）
    pub severity: String,
    /// 影響を受けるバージョン
    pub affected_versions: String,
    /// 修正されたバージョン
    pub fixed_versions: Option<String>,
    /// 公開日
    pub published_date: DateTime<Utc>,
    /// アドバイザリURL
    pub advisory_url: Option<Url>,
}

/// 署名の問題
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureIssue {
    /// 問題の種類
    pub issue_type: SignatureIssueType,
    /// 期待された署名
    pub expected_signature: Option<String>,
    /// 実際の署名
    pub actual_signature: Option<String>,
    /// 署名者
    pub signer: Option<String>,
}

/// 署名問題の種類
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SignatureIssueType {
    /// 署名が見つからない
    Missing,
    /// 署名が無効
    Invalid,
    /// 署名が期限切れ
    Expired,
    /// 署名者が信頼されていない
    UntrustedSigner,
    /// その他の署名問題
    Other,
}

/// 権限の問題
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionIssue {
    /// 問題の種類
    pub issue_type: PermissionIssueType,
    /// 必要な権限
    pub required_permission: Option<String>,
    /// 現在の権限
    pub current_permission: Option<String>,
    /// 関連するリソース
    pub resource: Option<String>,
}

/// 権限問題の種類
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PermissionIssueType {
    /// アクセス拒否
    AccessDenied,
    /// 権限不足
    InsufficientPermissions,
    /// ファイルシステム権限
    FileSystemPermission,
    /// ネットワーク権限
    NetworkPermission,
    /// その他の権限問題
    Other,
}

/// レジストリステータス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStatus {
    /// レジストリが利用可能かどうか
    pub available: bool,
    /// レジストリの応答時間（ミリ秒）
    pub response_time_ms: Option<u64>,
    /// レジストリのステータスメッセージ
    pub status_message: Option<String>,
    /// 最後の確認時刻
    pub last_checked: DateTime<Utc>,
}

/// ファイルシステム操作
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FilesystemOperation {
    /// 読み取り
    Read,
    /// 書き込み
    Write,
    /// 作成
    Create,
    /// 削除
    Delete,
    /// 移動
    Move,
    /// コピー
    Copy,
    /// 権限変更
    ChangePermissions,
    /// その他の操作
    Other,
}

/// 検証の種類
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValidationType {
    /// マニフェスト検証
    Manifest,
    /// 構造検証
    Structure,
    /// 依存関係検証
    Dependencies,
    /// セキュリティ検証
    Security,
    /// ライセンス検証
    License,
    /// その他の検証
    Other,
}

/// キャッシュ操作
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CacheOperation {
    /// 読み取り
    Read,
    /// 書き込み
    Write,
    /// 更新
    Update,
    /// 削除
    Delete,
    /// クリーンアップ
    Cleanup,
    /// その他の操作
    Other,
}

/// ロックファイル操作
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LockfileOperation {
    /// 読み取り
    Read,
    /// 書き込み
    Write,
    /// 更新
    Update,
    /// 解析
    Parse,
    /// 生成
    Generate,
    /// その他の操作
    Other,
}

/// プラグイン操作
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginOperation {
    /// ロード
    Load,
    /// 実行
    Execute,
    /// インストール
    Install,
    /// アンインストール
    Uninstall,
    /// 更新
    Update,
    /// その他の操作
    Other,
}

impl PackageError {
    /// 新しいIOエラーを作成
    pub fn io_error(source: io::Error, operation: impl Into<String>, path: Option<PathBuf>) -> Self {
        PackageError::IOError {
            source: source.into(),
            operation: operation.into(),
            path,
            suggestions: vec![
                "ファイルの権限を確認してください".to_string(),
                "ディスク容量を確認してください".to_string(),
                "ファイルが存在するか確認してください".to_string(),
            ],
        }
    }

    /// 新しいパーサーエラーを作成
    pub fn parse_error(
        message: impl Into<String>,
        file: Option<PathBuf>,
        line: usize,
        column: usize,
        snippet: Option<String>,
    ) -> Self {
        PackageError::ParseError {
            message: message.into(),
            file,
            line,
            column,
            snippet,
            suggestions: vec![
                "構文を確認してください".to_string(),
                "ドキュメントを参照してください".to_string(),
            ],
        }
    }

    /// 新しいネットワークエラーを作成
    pub fn network_error(
        message: impl Into<String>,
        url: Option<Url>,
        status_code: Option<u16>,
    ) -> Self {
        let mut suggestions = vec![
            "ネットワーク接続を確認してください".to_string(),
            "プロキシ設定を確認してください".to_string(),
        ];

        if let Some(code) = status_code {
            match code {
                401 | 403 => suggestions.push("認証情報を確認してください".to_string()),
                404 => suggestions.push("URLが正しいか確認してください".to_string()),
                429 => suggestions.push("レート制限に達した可能性があります。しばらく待ってから再試行してください".to_string()),
                500..=599 => suggestions.push("サーバーエラーが発生しています。しばらく待ってから再試行してください".to_string()),
                _ => {}
            }
        }

        PackageError::NetworkError {
            message: message.into(),
            url,
            status_code,
            retry_info: Some(RetryInfo {
                attempts: 1,
                max_attempts: 3,
                next_retry_ms: 1000,
                backoff_strategy: "exponential".to_string(),
            }),
            proxy_related: false,
            suggestions,
        }
    }

    /// 新しいパッケージが見つからないエラーを作成
    pub fn package_not_found(
        name: impl Into<String>,
        version: Option<String>,
        registries: Vec<String>,
    ) -> Self {
        let name_str = name.into();
        PackageError::PackageNotFound {
            name: name_str.clone(),
            version,
            registries,
            similar_packages: vec![],  // 実際の実装では類似パッケージを検索
            suggestions: vec![
                "パッケージ名のスペルを確認してください".to_string(),
                "別のレジストリを試してください".to_string(),
                "パッケージが公開されているか確認してください".to_string(),
            ],
        }
    }

    /// 新しいバージョンが見つからないエラーを作成
    pub fn version_not_found(
        name: impl Into<String>,
        version: impl Into<String>,
        available_versions: Vec<String>,
    ) -> Self {
        let version_str = version.into();
        let mut suggestions = vec![
            "バージョン指定を確認してください".to_string(),
            format!("利用可能なバージョンのいずれかを使用してください: {}", available_versions.join(", ")),
        ];

        if let Some(latest) = available_versions.last() {
            suggestions.push(format!("最新バージョン '{}' を使用することを検討してください", latest));
        }

        PackageError::VersionNotFound {
            name: name.into(),
            version: version_str,
            available_versions,
            latest_version: available_versions.last().cloned(),
            suggestions,
        }
    }

    /// 新しい依存関係解決エラーを作成
    pub fn resolution_error(message: impl Into<String>) -> Self {
        PackageError::ResolutionError {
            message: message.into(),
            dependency_graph: None,
            version_conflicts: vec![],
            circular_dependencies: vec![],
            suggestions: vec![
                "依存関係のバージョン制約を緩和してみてください".to_string(),
                "互換性のあるバージョンを使用してください".to_string(),
                "依存関係グラフを確認してください".to_string(),
            ],
        }
    }

    /// 新しいセキュリティエラーを作成
    pub fn security_error(message: impl Into<String>) -> Self {
        PackageError::SecurityError {
            message: message.into(),
            vulnerability: None,
            signature_issue: None,
            permission_issue: None,
            suggestions: vec![
                "パッケージの信頼性を確認してください".to_string(),
                "最新バージョンに更新してください".to_string(),
                "セキュリティアドバイザリを確認してください".to_string(),
            ],
            advisory_url: None,
        }
    }

    /// 新しい設定エラーを作成
    pub fn config_error(
        message: impl Into<String>,
        config_file: Option<PathBuf>,
        config_key: Option<String>,
    ) -> Self {
        PackageError::ConfigError {
            message: message.into(),
            config_file,
            config_key,
            invalid_value: None,
            expected_format: None,
            suggestions: vec![
                "設定ファイルの構文を確認してください".to_string(),
                "ドキュメントを参照して正しい設定形式を確認してください".to_string(),
            ],
        }
    }

    /// 新しいレジストリエラーを作成
    pub fn registry_error(message: impl Into<String>, registry_url: Option<Url>) -> Self {
        PackageError::RegistryError {
            message: message.into(),
            registry_url,
            authentication_issue: false,
            registry_status: None,
            suggestions: vec![
                "レジストリの設定を確認してください".to_string(),
                "レジストリが利用可能か確認してください".to_string(),
                "認証情報を確認してください".to_string(),
            ],
        }
    }

    /// 新しいファイルシステムエラーを作成
    pub fn filesystem_error(
        path: impl Into<PathBuf>,
        message: impl Into<String>,
        operation: FilesystemOperation,
    ) -> Self {
        PackageError::FilesystemError {
            path: path.into(),
            message: message.into(),
            operation,
            permission_issue: false,
            disk_space_issue: false,
            suggestions: vec![
                "ファイルの権限を確認してください".to_string(),
                "ディスク容量を確認してください".to_string(),
                "ファイルパスが正しいか確認してください".to_string(),
            ],
        }
    }

    /// 新しいビルドエラーを作成
    pub fn build_error(message: impl Into<String>, package: impl Into<String>) -> Self {
        PackageError::BuildError {
            message: message.into(),
            package: package.into(),
            build_command: None,
            build_log: None,
            dependency_issues: vec![],
            suggestions: vec![
                "ビルド依存関係を確認してください".to_string(),
                "ビルドスクリプトを確認してください".to_string(),
                "コンパイラのバージョンを確認してください".to_string(),
            ],
        }
    }

    /// 新しいその他のエラーを作成
    pub fn other(message: impl Into<String>) -> Self {
        PackageError::Other {
            message: message.into(),
            details: None,
            suggestions: vec![
                "詳細なエラーメッセージを確認してください".to_string(),
                "ドキュメントを参照してください".to_string(),
                "サポートに問い合わせてください".to_string(),
            ],
        }
    }

    /// エラーの診断情報を取得
    pub fn get_diagnostic_info(&self) -> HashMap<String, String> {
        let mut info = HashMap::new();
        
        match self {
            PackageError::IOError { source, operation, path, .. } => {
                info.insert("error_type".to_string(), "io_error".to_string());
                info.insert("operation".to_string(), operation.clone());
                info.insert("error_details".to_string(), source.to_string());
                if let Some(p) = path {
                    info.insert("path".to_string(), p.display().to_string());
                }
            },
            PackageError::ParseError { message, file, line, column, .. } => {
                info.insert("error_type".to_string(), "parse_error".to_string());
                info.insert("message".to_string(), message.clone());
                info.insert("line".to_string(), line.to_string());
                info.insert("column".to_string(), column.to_string());
                if let Some(f) = file {
                    info.insert("file".to_string(), f.display().to_string());
                }
            },
            // 他のエラー型についても同様に実装
            _ => {
                info.insert("error_type".to_string(), "unknown".to_string());
                info.insert("message".to_string(), self.to_string());
            }
        }
        
        info
    }

    /// エラーの修正提案を取得
    pub fn get_suggestions(&self) -> Vec<String> {
        match self {
            PackageError::IOError { suggestions, .. } => suggestions.clone(),
            PackageError::ParseError { suggestions, .. } => suggestions.clone(),
            PackageError::NetworkError { suggestions, .. } => suggestions.clone(),
            PackageError::PackageNotFound { suggestions, .. } => suggestions.clone(),
            PackageError::VersionNotFound { suggestions, .. } => suggestions.clone(),
            PackageError::ResolutionError { suggestions, .. } => suggestions.clone(),
            PackageError::SecurityError { suggestions, .. } => suggestions.clone(),
            PackageError::ConfigError { suggestions, .. } => suggestions.clone(),
            PackageError::RegistryError { suggestions, .. } => suggestions.clone(),
            PackageError::InteractionError { suggestions, .. } => suggestions.clone(),
            PackageError::FilesystemError { suggestions, .. } => suggestions.clone(),
            PackageError::BuildError { suggestions, .. } => suggestions.clone(),
            PackageError::ValidationError { suggestions, .. } => suggestions.clone(),
            PackageError::PublishError { suggestions, .. } => suggestions.clone(),
            PackageError::CacheError { suggestions, .. } => suggestions.clone(),
            PackageError::LockfileError { suggestions, .. } => suggestions.clone(),
            PackageError::PluginError { suggestions, .. } => suggestions.clone(),
            PackageError::WorkspaceError { suggestions, .. } => suggestions.clone(),
            PackageError::ConcurrencyError { suggestions, .. } => suggestions.clone(),
            PackageError::OutOfMemoryError { suggestions, .. } => suggestions.clone(),
            PackageError::TimeoutError { suggestions, .. } => suggestions.clone(),
            PackageError::Other { suggestions, .. } => suggestions.clone(),
        }
    }
}

/// Result型のエイリアス
pub type Result<T> = std::result::Result<T, PackageError>; 
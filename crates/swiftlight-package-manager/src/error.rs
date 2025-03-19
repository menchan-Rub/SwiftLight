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
use std::result;
use anyhow;
use toml;
use serde_json;

/// パッケージマネージャーのエラー型
/// 
/// SwiftLightパッケージマネージャーで発生する可能性のあるすべてのエラーを表現します。
/// エラーは詳細な診断情報、修正提案、およびコンテキスト情報を含みます。
#[derive(Error, Debug, Clone)]
pub enum PackageError {
    /// I/Oエラー
    #[error("I/Oエラー: {0}")]
    IoError(String),

    /// マニフェストエラー
    #[error("マニフェストエラー: {0}")]
    Manifest(String),

    /// 依存関係エラー
    #[error("依存関係エラー: {0}")]
    Dependency(String),

    /// 解決エラー
    #[error("解決エラー: {0}")]
    Resolution(String),

    /// 構築エラー
    #[error("構築エラー: {0}")]
    Build(String),

    /// ネットワークエラー
    #[error("ネットワークエラー: {0}")]
    Network(String),

    /// レジストリエラー
    #[error("レジストリエラー: {0}")]
    Registry(String),

    /// バージョンエラー
    #[error("バージョンエラー: {0}")]
    Version(String),

    /// 構成エラー
    #[error("構成エラー: {0}")]
    Config(String),

    /// パッケージエラー
    #[error("パッケージエラー: {0}")]
    Package(String),

    /// ロックファイルエラー
    #[error("ロックファイルエラー: {0}")]
    Lockfile(String),

    /// セキュリティエラー
    #[error("セキュリティエラー: {0}")]
    Security(String),

    /// 検証エラー
    #[error("検証エラー: {0}")]
    Validation(String),

    /// ファイルシステムエラー
    #[error("ファイルシステムエラー: パス '{path}' - {message}")]
    FilesystemError { path: PathBuf, message: String },

    /// 内部エラー
    #[error("内部エラー: {0}")]
    Internal(String),

    /// ユーザーエラー
    #[error("ユーザーエラー: {0}")]
    User(String),

    /// クレートの依存関係エラー
    #[error("{0}")]
    Anyhow(String),

    /// TOMLシリアライズエラー
    #[error("TOMLシリアライズエラー: {0}")]
    TomlSer(#[from] toml::ser::Error),

    /// TOMLデシリアライズエラー
    #[error("TOMLデシリアライズエラー: {0}")]
    TomlDe(#[from] toml::de::Error),

    /// JSONシリアライズエラー
    #[error("JSONシリアライズエラー: {0}")]
    JsonSer(String),

    /// セマンティックバージョニングエラー
    #[error("セマンティックバージョニングエラー: {0}")]
    SemVer(String),

    /// URLパースエラー
    #[error("URLパースエラー: {0}")]
    Url(#[from] url::ParseError),

    /// HTTPエラー
    #[error("HTTPエラー: {0}")]
    Http(String),

    /// コマンドエラー
    #[error("コマンドエラー: {0}")]
    Command(String),

    /// タイムアウトエラー
    #[error("タイムアウトエラー: {0}")]
    Timeout(String),

    /// パーミッションエラー
    #[error("パーミッションエラー: {0}")]
    Permission(String),

    /// グローバルなロック取得エラー
    #[error("グローバルなロック取得エラー: {0}")]
    GlobalLock(String),

    /// プロジェクトロック取得エラー
    #[error("プロジェクトロック取得エラー: {0}")]
    ProjectLock(String),

    /// パースエラー
    #[error("パースエラー: {0}")]
    ParseError(String),

    /// セキュリティ監査エラー
    #[error("セキュリティ監査エラー: パッケージ {0} - {1}")]
    SecurityAuditError(String, String),
}

/// エラー種別
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// I/Oエラー
    Io,
    /// マニフェストエラー
    Manifest,
    /// 依存関係エラー
    Dependency,
    /// 解決エラー
    Resolution,
    /// 構築エラー
    Build,
    /// ネットワークエラー
    Network,
    /// レジストリエラー
    Registry,
    /// バージョンエラー
    Version,
    /// 構成エラー
    Config,
    /// パッケージエラー
    Package,
    /// ロックファイルエラー
    Lockfile,
    /// セキュリティエラー
    Security,
    /// 検証エラー
    Validation,
    /// ファイルシステムエラー
    FileSystem,
    /// 内部エラー
    Internal,
    /// ユーザーエラー
    User,
    /// HTTPエラー
    Http,
    /// コマンドエラー
    Command,
    /// タイムアウトエラー
    Timeout,
    /// パーミッションエラー
    Permission,
    /// ロックエラー
    Lock,
    /// その他
    Other,
}

impl PackageError {
    /// エラーの種類を取得
    pub fn kind(&self) -> ErrorKind {
        match self {
            PackageError::IoError(_) => ErrorKind::Io,
            PackageError::Manifest(_) => ErrorKind::Manifest,
            PackageError::Dependency(_) => ErrorKind::Dependency,
            PackageError::Resolution(_) => ErrorKind::Resolution,
            PackageError::Build(_) => ErrorKind::Build,
            PackageError::Network(_) => ErrorKind::Network,
            PackageError::Registry(_) => ErrorKind::Registry,
            PackageError::Version(_) => ErrorKind::Version,
            PackageError::Config(_) => ErrorKind::Config,
            PackageError::Package(_) => ErrorKind::Package,
            PackageError::Lockfile(_) => ErrorKind::Lockfile,
            PackageError::Security(_) => ErrorKind::Security,
            PackageError::SecurityAuditError(_, _) => ErrorKind::Security,
            PackageError::Validation(_) => ErrorKind::Validation,
            PackageError::FilesystemError { .. } => ErrorKind::FileSystem,
            PackageError::Internal(_) => ErrorKind::Internal,
            PackageError::User(_) => ErrorKind::User,
            PackageError::Anyhow(_) => ErrorKind::Other,
            PackageError::TomlSer(_) => ErrorKind::Other,
            PackageError::TomlDe(_) => ErrorKind::Other,
            PackageError::JsonSer(_) => ErrorKind::Other,
            PackageError::SemVer(_) => ErrorKind::Other,
            PackageError::Url(_) => ErrorKind::Other,
            PackageError::Http(_) => ErrorKind::Http,
            PackageError::Command(_) => ErrorKind::Command,
            PackageError::Timeout(_) => ErrorKind::Timeout,
            PackageError::Permission(_) => ErrorKind::Permission,
            PackageError::GlobalLock(_) | PackageError::ProjectLock(_) => ErrorKind::Lock,
            PackageError::ParseError(_) => ErrorKind::Other,
        }
    }

    /// ユーザーフレンドリーなエラーメッセージを取得
    pub fn user_message(&self) -> String {
        match self {
            PackageError::IoError(err) => format!("I/Oエラーが発生しました: {}", err),
            PackageError::Manifest(msg) => format!("マニフェストエラーが発生しました: {}", msg),
            PackageError::Dependency(msg) => format!("依存関係エラーが発生しました: {}", msg),
            PackageError::Resolution(msg) => format!("依存関係の解決中にエラーが発生しました: {}", msg),
            PackageError::Build(msg) => format!("ビルドエラーが発生しました: {}", msg),
            PackageError::Network(msg) => format!("ネットワークエラーが発生しました: {}", msg),
            PackageError::Registry(msg) => format!("レジストリエラーが発生しました: {}", msg),
            PackageError::Version(msg) => format!("バージョンエラーが発生しました: {}", msg),
            PackageError::Config(msg) => format!("設定エラーが発生しました: {}", msg),
            PackageError::Package(msg) => format!("パッケージエラーが発生しました: {}", msg),
            PackageError::Lockfile(msg) => format!("ロックファイルエラーが発生しました: {}", msg),
            PackageError::Security(msg) => format!("セキュリティエラーが発生しました: {}", msg),
            PackageError::SecurityAuditError(package, msg) => format!("パッケージ {} のセキュリティ監査中にエラーが発生しました: {}", package, msg),
            PackageError::Validation(msg) => format!("検証エラーが発生しました: {}", msg),
            PackageError::FilesystemError { path, message } => {
                format!("ファイルシステムエラーが発生しました: パス '{}': {}", path.display(), message)
            },
            PackageError::Internal(msg) => format!("内部エラーが発生しました: {}", msg),
            PackageError::User(msg) => msg.clone(),
            PackageError::Anyhow(err) => format!("エラーが発生しました: {}", err),
            PackageError::TomlSer(err) => format!("TOMLシリアライズエラーが発生しました: {}", err),
            PackageError::TomlDe(err) => format!("TOMLデシリアライズエラーが発生しました: {}", err),
            PackageError::JsonSer(err) => format!("JSONシリアライズエラーが発生しました: {}", err),
            PackageError::SemVer(err) => format!("セマンティックバージョニングエラーが発生しました: {}", err),
            PackageError::Url(err) => format!("URLパースエラーが発生しました: {}", err),
            PackageError::Http(msg) => format!("HTTPエラーが発生しました: {}", msg),
            PackageError::Command(msg) => format!("コマンドエラーが発生しました: {}", msg),
            PackageError::Timeout(msg) => format!("タイムアウトが発生しました: {}", msg),
            PackageError::Permission(msg) => format!("パーミッションエラーが発生しました: {}", msg),
            PackageError::GlobalLock(msg) => format!("グローバルなロック取得に失敗しました: {}", msg),
            PackageError::ProjectLock(msg) => format!("プロジェクトロック取得に失敗しました: {}", msg),
            PackageError::ParseError(msg) => format!("パースエラーが発生しました: {}", msg),
        }
    }

    /// エラーコード（エラー種別に基づく一意の文字列）を取得
    pub fn error_code(&self) -> String {
        let prefix = match self.kind() {
            ErrorKind::Io => "IO",
            ErrorKind::Manifest => "MANIFEST",
            ErrorKind::Dependency => "DEP",
            ErrorKind::Resolution => "RESOLVE",
            ErrorKind::Build => "BUILD",
            ErrorKind::Network => "NET",
            ErrorKind::Registry => "REG",
            ErrorKind::Version => "VER",
            ErrorKind::Config => "CFG",
            ErrorKind::Package => "PKG",
            ErrorKind::Lockfile => "LOCK",
            ErrorKind::Security => "SEC",
            ErrorKind::Validation => "VALID",
            ErrorKind::FileSystem => "FS",
            ErrorKind::Internal => "INT",
            ErrorKind::User => "USER",
            ErrorKind::Http => "HTTP",
            ErrorKind::Command => "CMD",
            ErrorKind::Timeout => "TIMEOUT",
            ErrorKind::Permission => "PERM",
            ErrorKind::Lock => "LOCK",
            ErrorKind::Other => "OTHER",
        };

        format!("SWPM_{}_001", prefix)
    }

    /// エラーが致命的かどうかを判定
    pub fn is_fatal(&self) -> bool {
        match self.kind() {
            ErrorKind::Io |
            ErrorKind::Internal |
            ErrorKind::FileSystem |
            ErrorKind::Permission |
            ErrorKind::Lock => true,
            _ => false,
        }
    }

    /// 再試行可能なエラーかどうかを判定
    pub fn is_retryable(&self) -> bool {
        match self.kind() {
            ErrorKind::Network |
            ErrorKind::Http |
            ErrorKind::Timeout |
            ErrorKind::Registry => true,
            _ => false,
        }
    }

    /// エラーを新しく作成（汎用）
    pub fn new<S: Into<String>>(msg: S) -> Self {
        PackageError::Internal(msg.into())
    }

    /// I/Oエラーを作成
    pub fn io<S: Into<String>>(msg: S) -> Self {
        PackageError::IoError(msg.into())
    }

    /// マニフェストエラーを作成
    pub fn manifest<S: Into<String>>(msg: S) -> Self {
        PackageError::Manifest(msg.into())
    }

    /// 依存関係エラーを作成
    pub fn dependency<S: Into<String>>(msg: S) -> Self {
        PackageError::Dependency(msg.into())
    }

    /// 解決エラーを作成
    pub fn resolution<S: Into<String>>(msg: S) -> Self {
        PackageError::Resolution(msg.into())
    }

    /// ビルドエラーを作成
    pub fn build<S: Into<String>>(msg: S) -> Self {
        PackageError::Build(msg.into())
    }

    /// ネットワークエラーを作成
    pub fn network<S: Into<String>>(msg: S) -> Self {
        PackageError::Network(msg.into())
    }

    /// レジストリエラーを作成
    pub fn registry<S: Into<String>>(msg: S) -> Self {
        PackageError::Registry(msg.into())
    }

    /// バージョンエラーを作成
    pub fn version<S: Into<String>>(msg: S) -> Self {
        PackageError::Version(msg.into())
    }

    /// 設定エラーを作成
    pub fn config<S: Into<String>>(msg: S) -> Self {
        PackageError::Config(msg.into())
    }

    /// パッケージエラーを作成
    pub fn package<S: Into<String>>(msg: S) -> Self {
        PackageError::Package(msg.into())
    }

    /// ファイルシステムエラーを作成
    pub fn filesystem<S: Into<String>, P: Into<PathBuf>>(path: P, msg: S) -> Self {
        PackageError::FilesystemError {
            path: path.into(),
            message: msg.into(),
        }
    }
}

/// SwiftLightパッケージマネージャーの結果型
pub type Result<T> = result::Result<T, PackageError>;

/// エラーの表示用ヘルパー
pub struct DisplayError<'a>(pub &'a PackageError);

impl<'a> fmt::Display for DisplayError<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "エラー {}: {}", self.0.error_code(), self.0.user_message())
    }
}

impl From<semver::Error> for PackageError {
    fn from(e: semver::Error) -> Self {
        PackageError::SemVer(e.to_string())
    }
}

impl From<anyhow::Error> for PackageError {
    fn from(e: anyhow::Error) -> Self {
        PackageError::Anyhow(e.to_string())
    }
} 
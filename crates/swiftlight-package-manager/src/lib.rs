//! SwiftLightパッケージマネージャー
//! 
//! このモジュールはSwiftLight言語のパッケージ管理システムを提供します。
//! 高度な依存関係解決、バージョン管理、パッケージの検証と配布を行います。
//! 
//! # 特徴
//! 
//! - 高速な依存関係解決アルゴリズム
//! - セマンティックバージョニングの完全サポート
//! - パッケージの暗号署名による検証
//! - 分散型レジストリシステム
//! - オフラインモードサポート
//! - プロジェクト固有の設定
//! - 高度なキャッシング戦略

use std::collections::{HashMap, HashSet, BTreeMap};
use std::error::Error;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature, Signer, Verifier};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use log::{debug, error, info, trace, warn};
use rand::rngs::OsRng;
use reqwest::{Client, StatusCode};
use semver::{Version, VersionReq};
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use sha2::{Sha256, Digest};
use tar::Archive;
use thiserror::Error;
use tokio::sync::Semaphore;
use toml::{self, Value};
use url::Url;

// バージョンのシリアライズサポート
mod semver_serialize {
    use semver::Version;
    use serde::{Serialize, Deserialize, Serializer, Deserializer};
    use std::str::FromStr;

    pub fn serialize<S>(version: &Version, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&version.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Version, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Version::from_str(&s).map_err(serde::de::Error::custom)
    }
    
    // バージョン要件のシリアライズサポート
    pub mod req {
        use semver::VersionReq;
        use serde::{Serialize, Deserialize, Serializer, Deserializer};
        use std::str::FromStr;

        pub fn serialize<S>(req: &VersionReq, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_str(&req.to_string())
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<VersionReq, D::Error>
        where
            D: Deserializer<'de>,
        {
            let s = String::deserialize(deserializer)?;
            VersionReq::from_str(&s).map_err(serde::de::Error::custom)
        }
    }
}

/// パッケージマネージャーのエラー型
#[derive(Error, Debug)]
pub enum PackageError {
    #[error("IOエラー: {0}")]
    Io(#[from] io::Error),
    
    #[error("パッケージが見つかりません: {0}")]
    PackageNotFound(String),
    
    #[error("バージョン解決エラー: {0}")]
    VersionResolution(String),
    
    #[error("依存関係の循環が検出されました: {0}")]
    CyclicDependency(String),
    
    #[error("パッケージの検証に失敗しました: {0}")]
    VerificationFailed(String),
    
    #[error("ネットワークエラー: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("パッケージの解析に失敗しました: {0}")]
    ParseError(String),
    
    #[error("TOMLエラー: {0}")]
    Toml(#[from] toml::de::Error),
    
    #[error("セマンティックバージョンエラー: {0}")]
    Semver(#[from] semver::Error),
    
    #[error("署名エラー: {0}")]
    Signature(String),
    
    #[error("キャッシュエラー: {0}")]
    Cache(String),
    
    #[error("設定エラー: {0}")]
    Config(String),
    
    #[error("レジストリエラー: {0}")]
    Registry(String),
    
    #[error("不正なパッケージ名: {0}")]
    InvalidPackageName(String),
    
    #[error("不正なパッケージバージョン: {0}")]
    InvalidPackageVersion(String),
    
    #[error("パッケージのビルドに失敗しました: {0}")]
    BuildFailed(String),
    
    #[error("パッケージのインストールに失敗しました: {0}")]
    InstallFailed(String),
    
    #[error("パッケージの公開に失敗しました: {0}")]
    PublishFailed(String),
    
    #[error("パッケージのアンインストールに失敗しました: {0}")]
    UninstallFailed(String),
    
    #[error("パッケージの更新に失敗しました: {0}")]
    UpdateFailed(String),
    
    #[error("パッケージのロックに失敗しました: {0}")]
    LockFailed(String),
    
    #[error("パッケージの解凍に失敗しました: {0}")]
    ExtractionFailed(String),
    
    #[error("パッケージの圧縮に失敗しました: {0}")]
    CompressionFailed(String),
    
    #[error("パッケージのハッシュ計算に失敗しました: {0}")]
    HashFailed(String),
    
    #[error("パッケージの依存関係が解決できません: {0}")]
    UnresolvableDependency(String),
    
    #[error("パッケージの互換性がありません: {0} と {1}")]
    IncompatiblePackages(String, String),
    
    #[error("パッケージのメタデータが不正です: {0}")]
    InvalidMetadata(String),
    
    #[error("パッケージの権限がありません: {0}")]
    PermissionDenied(String),
    
    #[error("タイムアウトエラー: {0}")]
    Timeout(String),
    
    #[error("その他のエラー: {0}")]
    Other(String),
}

/// パッケージの依存関係を表す構造体
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Dependency {
    /// パッケージ名
    pub name: String,
    
    /// バージョン要件
    #[serde(with = "semver_serialize::req")]
    pub version_req: VersionReq,
    
    /// オプションの依存関係かどうか
    #[serde(default)]
    pub optional: bool,
    
    /// 開発依存関係かどうか
    #[serde(default)]
    pub dev: bool,
    
    /// ビルド依存関係かどうか
    #[serde(default)]
    pub build: bool,
    
    /// 特定のターゲットのみに適用される依存関係かどうか
    #[serde(default)]
    pub target: Option<String>,
    
    /// 依存関係の特徴（フィーチャー）
    #[serde(default)]
    pub features: Vec<String>,
    
    /// デフォルトの特徴を使用するかどうか
    #[serde(default = "default_true")]
    pub default_features: bool,
    
    /// レジストリURL（デフォルトのレジストリを使用しない場合）
    #[serde(default)]
    pub registry: Option<String>,
    
    /// Gitリポジトリから取得する場合のURL
    #[serde(default)]
    pub git: Option<String>,
    
    /// Gitリポジトリのブランチ
    #[serde(default)]
    pub branch: Option<String>,
    
    /// Gitリポジトリのタグ
    #[serde(default)]
    pub tag: Option<String>,
    
    /// Gitリポジトリのコミットハッシュ
    #[serde(default)]
    pub rev: Option<String>,
    
    /// ローカルパスから取得する場合のパス
    #[serde(default)]
    pub path: Option<String>,
    
    /// パッケージの代替名（エイリアス）
    #[serde(default)]
    pub package: Option<String>,
    
    /// 依存関係の優先度（競合解決に使用）
    #[serde(default)]
    pub priority: i32,
}

fn default_true() -> bool {
    true
}

impl Dependency {
    /// 新しい依存関係を作成
    pub fn new<S: Into<String>>(name: S, version_req: VersionReq) -> Self {
        Dependency {
            name: name.into(),
            version_req,
            optional: false,
            dev: false,
            build: false,
            target: None,
            features: Vec::new(),
            default_features: true,
            registry: None,
            git: None,
            branch: None,
            tag: None,
            rev: None,
            path: None,
            package: None,
            priority: 0,
        }
    }
    
    /// 依存関係がオプションかどうかを設定
    pub fn optional(mut self, optional: bool) -> Self {
        self.optional = optional;
        self
    }
    
    /// 依存関係が開発用かどうかを設定
    pub fn dev(mut self, dev: bool) -> Self {
        self.dev = dev;
        self
    }
    
    /// 依存関係がビルド用かどうかを設定
    pub fn build(mut self, build: bool) -> Self {
        self.build = build;
        self
    }
    
    /// 依存関係のターゲットを設定
    pub fn target<S: Into<String>>(mut self, target: Option<S>) -> Self {
        self.target = target.map(|s| s.into());
        self
    }
    
    /// 依存関係の特徴を設定
    pub fn features(mut self, features: Vec<String>) -> Self {
        self.features = features;
        self
    }
    
    /// 依存関係のデフォルト特徴を使用するかどうかを設定
    pub fn default_features(mut self, default_features: bool) -> Self {
        self.default_features = default_features;
        self
    }
    
    /// 依存関係のレジストリURLを設定
    pub fn registry<S: Into<String>>(mut self, registry: Option<S>) -> Self {
        self.registry = registry.map(|s| s.into());
        self
    }
    
    /// 依存関係のGitリポジトリURLを設定
    pub fn git<S: Into<String>>(mut self, git: Option<S>) -> Self {
        self.git = git.map(|s| s.into());
        self
    }
    
    /// 依存関係のGitブランチを設定
    pub fn branch<S: Into<String>>(mut self, branch: Option<S>) -> Self {
        self.branch = branch.map(|s| s.into());
        self
    }
    
    /// 依存関係のGitタグを設定
    pub fn tag<S: Into<String>>(mut self, tag: Option<S>) -> Self {
        self.tag = tag.map(|s| s.into());
        self
    }
    
    /// 依存関係のGitリビジョンを設定
    pub fn rev<S: Into<String>>(mut self, rev: Option<S>) -> Self {
        self.rev = rev.map(|s| s.into());
        self
    }
    
    /// 依存関係のローカルパスを設定
    pub fn path<S: Into<String>>(mut self, path: Option<S>) -> Self {
        self.path = path.map(|s| s.into());
        self
    }
    
    /// 依存関係のパッケージ名エイリアスを設定
    pub fn package<S: Into<String>>(mut self, package: Option<S>) -> Self {
        self.package = package.map(|s| s.into());
        self
    }
    
    /// 依存関係の優先度を設定
    pub fn priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
    
    /// 依存関係のソースタイプを取得
    pub fn source_type(&self) -> DependencySourceType {
        if self.path.is_some() {
            DependencySourceType::Path
        } else if self.git.is_some() {
            DependencySourceType::Git
        } else {
            DependencySourceType::Registry
        }
    }
    
    /// 依存関係が指定されたバージョンと互換性があるかどうかを確認
    pub fn is_compatible_with(&self, version: &Version) -> bool {
        self.version_req.matches(version)
    }
    
    /// 依存関係の実際のパッケージ名を取得
    pub fn actual_name(&self) -> &str {
        self.package.as_deref().unwrap_or(&self.name)
    }
    
    /// 依存関係の識別子を取得
    pub fn identifier(&self) -> String {
        match self.source_type() {
            DependencySourceType::Registry => {
                format!("{}@{}", self.actual_name(), self.version_req)
            }
            DependencySourceType::Git => {
                let mut id = format!("{}@git:{}", self.actual_name(), self.git.as_ref().unwrap());
                if let Some(ref branch) = self.branch {
                    id.push_str(&format!("#branch={}", branch));
                } else if let Some(ref tag) = self.tag {
                    id.push_str(&format!("#tag={}", tag));
                } else if let Some(ref rev) = self.rev {
                    id.push_str(&format!("#rev={}", rev));
                }
                id
            }
            DependencySourceType::Path => {
                format!("{}@path:{}", self.actual_name(), self.path.as_ref().unwrap())
            }
        }
    }
}

/// 依存関係のソースタイプ
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencySourceType {
    /// レジストリから取得
    Registry,
    /// Gitリポジトリから取得
    Git,
    /// ローカルパスから取得
    Path,
}

/// パッケージのメタデータを表す構造体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    /// パッケージ名
    pub name: String,
    
    /// パッケージのバージョン
    #[serde(with = "semver_serialize")]
    pub version: Version,
    
    /// パッケージの説明
    #[serde(default)]
    pub description: Option<String>,
    
    /// パッケージの作者
    #[serde(default)]
    pub authors: Vec<String>,
    
    /// パッケージのライセンス
    #[serde(default)]
    pub license: Option<String>,
    
    /// パッケージのリポジトリURL
    #[serde(default)]
    pub repository: Option<String>,
    
    /// パッケージのホームページURL
    #[serde(default)]
    pub homepage: Option<String>,
    
    /// パッケージのドキュメントURL
    #[serde(default)]
    pub documentation: Option<String>,
    
    /// パッケージのキーワード
    #[serde(default)]
    pub keywords: Vec<String>,
    
    /// パッケージのカテゴリ
    #[serde(default)]
    pub categories: Vec<String>,
    
    /// パッケージの依存関係
    #[serde(default)]
    pub dependencies: HashMap<String, Dependency>,
    
    /// パッケージの開発依存関係
    #[serde(default)]
    pub dev_dependencies: HashMap<String, Dependency>,
    
    /// パッケージのビルド依存関係
    #[serde(default)]
    pub build_dependencies: HashMap<String, Dependency>,
    
    /// パッケージの特徴（フィーチャー）
    #[serde(default)]
    pub features: HashMap<String, Vec<String>>,
    
    /// パッケージのデフォルト特徴
    #[serde(default)]
    pub default_features: Vec<String>,
    
    /// パッケージの最小互換SwiftLightバージョン
    #[serde(default)]
    pub swiftlight: Option<String>,
    
    /// パッケージの公開日時
    #[serde(default)]
    pub published_at: Option<DateTime<Utc>>,
    
    /// パッケージのダウンロード数
    #[serde(default)]
    pub downloads: u64,
    
    /// パッケージのサイズ（バイト）
    #[serde(default)]
    pub size: u64,
    
    /// パッケージのSHA-256ハッシュ
    #[serde(default)]
    pub checksum: Option<String>,
    
    /// パッケージの署名
    #[serde(default)]
    pub signature: Option<String>,
    
    /// パッケージの公開者
    #[serde(default)]
    pub publisher: Option<String>,
    
    /// パッケージの公開者の公開鍵
    #[serde(default)]
    pub publisher_public_key: Option<String>,
    
    /// パッケージの非推奨フラグ
    #[serde(default)]
    pub deprecated: bool,
    
    /// パッケージの非推奨理由
    #[serde(default)]
    pub deprecation_reason: Option<String>,
    
    /// パッケージの代替パッケージ
    #[serde(default)]
    pub alternative: Option<String>,
    
    /// パッケージのYankedフラグ
    #[serde(default)]
    pub yanked: bool,
    
    /// パッケージのYanked理由
    #[serde(default)]
    pub yanked_reason: Option<String>,
    
    /// パッケージのメタデータ
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

impl PackageMetadata {
    /// 新しいパッケージメタデータを作成
    pub fn new<S: Into<String>>(name: S, version: Version) -> Self {
        PackageMetadata {
            name: name.into(),
            version,
            description: None,
            authors: Vec::new(),
            license: None,
            repository: None,
            homepage: None,
            documentation: None,
            keywords: Vec::new(),
            categories: Vec::new(),
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
            build_dependencies: HashMap::new(),
            features: HashMap::new(),
            default_features: Vec::new(),
            swiftlight: None,
            published_at: None,
            downloads: 0,
            size: 0,
            checksum: None,
            signature: None,
            publisher: None,
            publisher_public_key: None,
            deprecated: false,
            deprecation_reason: None,
            alternative: None,
            yanked: false,
            yanked_reason: None,
            metadata: HashMap::new(),
        }
    }
    
    /// パッケージの識別子を取得
    pub fn identifier(&self) -> String {
        format!("{}@{}", self.name, self.version)
    }
    
    /// パッケージの全ての依存関係を取得
    pub fn all_dependencies(&self) -> HashMap<String, &Dependency> {
        let mut all_deps = HashMap::new();
        
        for (name, dep) in &self.dependencies {
            all_deps.insert(name.clone(), dep);
        }
        
        for (name, dep) in &self.dev_dependencies {
            all_deps.insert(name.clone(), dep);
        }
        
        for (name, dep) in &self.build_dependencies {
            all_deps.insert(name.clone(), dep);
        }
        
        all_deps
    }
    
    /// パッケージの全ての依存関係を取得（開発依存関係を除く）
    pub fn runtime_dependencies(&self) -> HashMap<String, &Dependency> {
        let mut runtime_deps = HashMap::new();
        
        for (name, dep) in &self.dependencies {
            runtime_deps.insert(name.clone(), dep);
        }
        
        runtime_deps
    }
    
    /// パッケージの全ての依存関係を取得（ビルド依存関係を含む）
    pub fn build_time_dependencies(&self) -> HashMap<String, &Dependency> {
        let mut build_deps = HashMap::new();
        
        for (name, dep) in &self.dependencies {
            build_deps.insert(name.clone(), dep);
        }
        
        for (name, dep) in &self.build_dependencies {
            build_deps.insert(name.clone(), dep);
        }
        
        build_deps
    }
    
    /// パッケージのチェックサムを計算
    pub fn calculate_checksum(&self, data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        format!("{:x}", result)
    }
    
    /// パッケージの署名を検証
    pub fn verify_signature(&self, data: &[u8]) -> Result<bool, PackageError> {
        if let (Some(signature_str), Some(public_key_str)) = (&self.signature, &self.publisher_public_key) {
            let signature_bytes = hex::decode(signature_str)
                .map_err(|e| PackageError::Signature(format!("署名のデコードに失敗しました: {}", e)))?;
            
            let public_key_bytes = hex::decode(public_key_str)
                .map_err(|e| PackageError::Signature(format!("公開鍵のデコードに失敗しました: {}", e)))?;
            
            let signature = Signature::from_bytes(&signature_bytes)
                .map_err(|e| PackageError::Signature(format!("署名の解析に失敗しました: {}", e)))?;
            
            let public_key = PublicKey::from_bytes(&public_key_bytes)
                .map_err(|e| PackageError::Signature(format!("公開鍵の解析に失敗しました: {}", e)))?;
            
            Ok(public_key.verify(data, &signature).is_ok())
        } else {
            Err(PackageError::Signature("署名または公開鍵がありません".to_string()))
        }
    }
    
    /// パッケージの署名を生成
    pub fn sign_package(&mut self, data: &[u8], keypair: &Keypair) -> Result<(), PackageError> {
        let signature = keypair.sign(data);
        self.signature = Some(hex::encode(signature.to_bytes()));
        self.publisher_public_key = Some(hex::encode(keypair.public.to_bytes()));
        Ok(())
    }
    
    /// パッケージが非推奨かどうかを確認
    pub fn is_deprecated(&self) -> bool {
        self.deprecated
    }
    
    /// パッケージがYankedかどうかを確認
    pub fn is_yanked(&self) -> bool {
        self.yanked
    }
    
    /// パッケージが使用可能かどうかを確認
    pub fn is_available(&self) -> bool {
        !self.is_deprecated() && !self.is_yanked()
    }
    
    /// パッケージのメタデータをTOMLに変換
    pub fn to_toml(&self) -> Result<String, PackageError> {
        toml::to_string(self).map_err(|e| PackageError::ParseError(e.to_string()))
    }
    
    /// TOMLからパッケージのメタデータを解析
    pub fn from_toml(toml_str: &str) -> Result<Self, PackageError> {
        toml::from_str(toml_str).map_err(|e| PackageError::ParseError(e.to_string()))
    }
    
    /// パッケージのメタデータをJSONに変換
    pub fn to_json(&self) -> Result<String, PackageError> {
        serde_json::to_string(self).map_err(|e| PackageError::ParseError(e.to_string()))
    }
    
    /// JSONからパッケージのメタデータを解析
    pub fn from_json(json_str: &str) -> Result<Self, PackageError> {
        serde_json::from_str(json_str).map_err(|e| PackageError::ParseError(e.to_string()))
    }
}

/// パッケージマネージャーの設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageManagerConfig {
    /// デフォルトのレジストリURL
    #[serde(default = "default_registry_url")]
    pub default_registry: String,
    
    /// 追加のレジストリURL
    #[serde(default)]
    pub registries: HashMap<String, String>,
    
    /// キャッシュディレクトリ
    #[serde(default = "default_cache_dir")]
    pub cache_dir: PathBuf,
    
    /// キャッシュの有効期限（秒）
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl: u64,
    
    /// 並行ダウンロード数
    #[serde(default = "default_concurrent_downloads")]
    pub concurrent_downloads: usize,
    
    /// タイムアウト（秒）
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    
    /// リトライ回数
    #[serde(default = "default_retry_count")]
    pub retry_count: u32,
    
    /// リトライ間隔（秒）
    #[serde(default = "default_retry_interval")]
    pub retry_interval: u64,
    
    /// オフラインモード
    #[serde(default)]
    pub offline: bool,
    
    /// 署名検証を強制するかどうか
    #[serde(default = "default_true")]
    pub verify_signatures: bool,
    
    /// 自動更新を有効にするかどうか
    #[serde(default = "default_true")]
    pub auto_update: bool,
    
    /// 詳細なログを有効にするかどうか
    #[serde(default)]
    pub verbose: bool,
    
    /// プロキシURL
    #[serde(default)]
    pub proxy: Option<String>,
    
    /// 認証情報
    #[serde(default)]
    pub auth: HashMap<String, RegistryAuth>,
    
    /// ミラーURL
    #[serde(default)]
    pub mirrors: HashMap<String, Vec<String>>,
    
    /// 信頼できる公開鍵
    #[serde(default)]
    pub trusted_keys: Vec<String>,
    
    /// 信頼できる発行者
    #[serde(default)]
    pub trusted_publishers: Vec<String>,
    
    /// 自動クリーンアップを有効にするかどうか
    #[serde(default = "default_true")]
    pub auto_cleanup: bool,
    
    /// クリーンアップの閾値（バイト）
    #[serde(default = "default_cleanup_threshold")]
    pub cleanup_threshold: u64,
    
    /// 最大キャッシュサイズ（バイト）
    #[serde(default = "default_max_cache_size")]
    pub max_cache_size: u64,
    
    /// 追加の設定
    #[serde(default)]
    pub extra: HashMap<String, Value>,
}

fn default_registry_url() -> String {
    "https://registry.swiftlight.io".to_string()
}

fn default_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from(".cache"))
        .join("swiftlight")
}

fn default_cache_ttl() -> u64 {
    86400 // 24時間
}

fn default_concurrent_downloads() -> usize {
    4
}

fn default_timeout() -> u64 {
    30 // 30秒
}

fn default_retry_count() -> u32 {
    3
}

fn default_retry_interval() -> u64 {
    5 // 5秒
}

fn default_cleanup_threshold() -> u64 {
    1024 * 1024 * 1024 // 1GB
}

fn default_max_cache_size() -> u64 {
    5 * 1024 * 1024 * 1024 // 5GB
}

/// レジストリ認証情報
#[derive(Debug, Serialize, Deserialize)]
pub struct RegistryAuth {
    /// 認証トークン
    pub token: Option<String>,
    
    /// ユーザー名
    pub username: Option<String>,
    
    /// パスワード
    pub password: Option<String>,
    
    /// キーペア（署名用）
    #[serde(skip)]
    pub keypair: Option<Keypair>,
    
    /// 公開鍵（文字列形式）
    pub public_key: Option<String>,
    
    /// 最終認証日時
    pub last_auth: Option<DateTime<Utc>>,
    
    /// 認証有効期限
    pub expires_at: Option<DateTime<Utc>>,
}

impl Clone for RegistryAuth {
    fn clone(&self) -> Self {
        Self {
            token: self.token.clone(),
            username: self.username.clone(),
            password: self.password.clone(),
            keypair: None, // keypairはクローンできないため、Noneを設定
            public_key: self.public_key.clone(),
            last_auth: self.last_auth.clone(),
            expires_at: self.expires_at.clone(),
        }
    }
} 
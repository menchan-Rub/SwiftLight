use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use anyhow::{Result, anyhow, Context, bail};
use semver::{Version, VersionReq};
use serde::{Serialize, Deserialize};
use toml;
use url::Url;
use chrono::{DateTime, Utc};
use sha2::{Sha256, Digest};
use hex;
use regex::Regex;
use walkdir::WalkDir;

use crate::dependency::{Dependency, DependencyType, DependencySource};
use crate::error::PackageError;
use crate::validation::{ValidationError, ValidationResult, Validator};
use crate::config::Config;
use crate::registry::Registry;
use crate::resolver::DependencyResolver;

/// パッケージ情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    /// パッケージ名
    pub name: String,
    /// バージョン
    pub version: String,
    /// 作者情報
    pub authors: Vec<String>,
    /// 説明
    pub description: Option<String>,
    /// リポジトリURL
    pub repository: Option<String>,
    /// ライセンス
    pub license: Option<String>,
    /// キーワード
    pub keywords: Option<Vec<String>>,
    /// カテゴリ
    pub categories: Option<Vec<String>>,
    /// ドキュメントURL
    pub documentation: Option<String>,
    /// ホームページURL
    pub homepage: Option<String>,
    /// エディション
    pub edition: Option<String>,
    /// 公開するかどうか
    pub publish: Option<bool>,
    /// 最小互換バージョン
    pub rust_version: Option<String>,
    /// パッケージのメタデータ
    pub metadata: Option<HashMap<String, toml::Value>>,
    /// 作成日時
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// 更新日時
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    /// パッケージの説明（長文）
    pub readme: Option<String>,
    /// パッケージのアイコン
    pub icon: Option<String>,
    /// パッケージのスクリーンショット
    pub screenshots: Option<Vec<String>>,
    /// パッケージの言語
    pub language: Option<String>,
    /// パッケージのサポート情報
    pub support: Option<SupportInfo>,
    /// パッケージの安全性情報
    pub security: Option<SecurityInfo>,
    /// パッケージの互換性情報
    pub compatibility: Option<CompatibilityInfo>,
}

/// サポート情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportInfo {
    /// サポートメール
    pub email: Option<String>,
    /// サポートURL
    pub url: Option<String>,
    /// バグトラッカーURL
    pub issues: Option<String>,
    /// サポート期限
    pub end_of_life: Option<String>,
    /// サポートポリシー
    pub policy: Option<String>,
}

/// セキュリティ情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityInfo {
    /// セキュリティポリシーURL
    pub policy: Option<String>,
    /// 脆弱性報告先
    pub reporting: Option<String>,
    /// 署名検証キー
    pub signing_keys: Option<Vec<String>>,
    /// 監査レポートURL
    pub audit_reports: Option<Vec<String>>,
}

/// 互換性情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilityInfo {
    /// 互換性のあるプラットフォーム
    pub platforms: Option<Vec<String>>,
    /// 互換性のあるアーキテクチャ
    pub architectures: Option<Vec<String>>,
    /// 互換性のあるSwiftlightバージョン
    pub swiftlight_version: Option<String>,
    /// 互換性のある言語バージョン
    pub language_versions: Option<HashMap<String, String>>,
}

/// バイナリセクション
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryInfo {
    /// バイナリ名
    pub name: String,
    /// エントリーポイント
    pub path: Option<String>,
    /// 必須フィーチャー
    pub required_features: Option<Vec<String>>,
    /// ターゲットプラットフォーム
    pub target: Option<Vec<String>>,
    /// ビルドフラグ
    pub build_flags: Option<Vec<String>>,
    /// 環境変数
    pub env: Option<HashMap<String, String>>,
    /// バイナリの説明
    pub description: Option<String>,
    /// バイナリのバージョン（パッケージと異なる場合）
    pub version: Option<String>,
    /// バイナリのアイコン
    pub icon: Option<String>,
    /// バイナリのカテゴリ
    pub category: Option<String>,
    /// バイナリのエントリーポイント
    pub main: Option<String>,
    /// バイナリの依存関係
    pub dependencies: Option<HashMap<String, toml::Value>>,
}

/// ライブラリセクション
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryInfo {
    /// ライブラリ名
    pub name: Option<String>,
    /// ライブラリパス
    pub path: Option<String>,
    /// ライブラリタイプ
    pub crate_type: Option<Vec<String>>,
    /// ライブラリのドキュメント
    pub doc: Option<bool>,
    /// ライブラリのテスト
    pub test: Option<bool>,
    /// ライブラリのベンチマーク
    pub bench: Option<bool>,
    /// ライブラリのドキュメントプライベート項目
    pub doc_private: Option<bool>,
    /// ライブラリのプロシージャルマクロ
    pub proc_macro: Option<bool>,
    /// ライブラリのエディション
    pub edition: Option<String>,
    /// ライブラリのターゲットプラットフォーム
    pub target: Option<Vec<String>>,
    /// ライブラリのエクスポート
    pub exports: Option<Vec<String>>,
    /// ライブラリのインターフェース安定性
    pub stability: Option<String>,
    /// ライブラリのAPI互換性ポリシー
    pub api_compatibility: Option<String>,
}

/// フィーチャーセクション
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturesInfo {
    #[serde(flatten)]
    pub features: HashMap<String, Vec<String>>,
}

/// ビルドセクション
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildInfo {
    /// ビルドスクリプト
    pub script: Option<String>,
    /// ビルド依存関係
    pub dependencies: Option<HashMap<String, toml::Value>>,
    /// ビルドターゲット
    pub target: Option<String>,
    /// ビルド環境変数
    pub env: Option<HashMap<String, String>>,
    /// ビルドフック
    pub hooks: Option<BuildHooks>,
    /// ビルドプラグイン
    pub plugins: Option<Vec<String>>,
    /// ビルドキャッシュ設定
    pub cache: Option<BuildCacheConfig>,
    /// ビルドパラレル設定
    pub parallel: Option<bool>,
    /// ビルドタイムアウト
    pub timeout: Option<u64>,
    /// ビルドリトライ設定
    pub retry: Option<BuildRetryConfig>,
}

/// ビルドフック
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildHooks {
    /// ビルド前フック
    pub pre_build: Option<Vec<String>>,
    /// ビルド後フック
    pub post_build: Option<Vec<String>>,
    /// インストール前フック
    pub pre_install: Option<Vec<String>>,
    /// インストール後フック
    pub post_install: Option<Vec<String>>,
    /// テスト前フック
    pub pre_test: Option<Vec<String>>,
    /// テスト後フック
    pub post_test: Option<Vec<String>>,
    /// パッケージ前フック
    pub pre_package: Option<Vec<String>>,
    /// パッケージ後フック
    pub post_package: Option<Vec<String>>,
}

/// ビルドキャッシュ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildCacheConfig {
    /// キャッシュを有効にするか
    pub enabled: Option<bool>,
    /// キャッシュディレクトリ
    pub dir: Option<String>,
    /// キャッシュの有効期限（秒）
    pub ttl: Option<u64>,
    /// キャッシュサイズ制限（MB）
    pub max_size: Option<u64>,
    /// キャッシュ共有設定
    pub shared: Option<bool>,
}

/// ビルドリトライ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildRetryConfig {
    /// リトライ回数
    pub count: Option<u32>,
    /// リトライ間隔（秒）
    pub interval: Option<u64>,
    /// リトライ時のバックオフ係数
    pub backoff: Option<f64>,
}

/// 開発プロファイル
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileInfo {
    /// 最適化レベル
    pub opt_level: Option<i32>,
    /// デバッグ情報
    pub debug: Option<bool>,
    /// デバッグアサーション
    pub debug_assertions: Option<bool>,
    /// オーバーフローチェック
    pub overflow_checks: Option<bool>,
    /// バックトレース
    pub backtrace: Option<bool>,
    /// コード生成ユニット
    pub codegen_units: Option<u32>,
    /// リンカーフラグ
    pub lto: Option<bool>,
    /// パニック動作
    pub panic: Option<String>,
    /// インクリメンタルコンパイル
    pub incremental: Option<bool>,
    /// ランタイムアサーション
    pub runtime_assertions: Option<bool>,
    /// メモリサニタイザー
    pub memory_sanitizer: Option<bool>,
    /// アドレスサニタイザー
    pub address_sanitizer: Option<bool>,
    /// スレッドサニタイザー
    pub thread_sanitizer: Option<bool>,
    /// リークサニタイザー
    pub leak_sanitizer: Option<bool>,
    /// スタックプロテクション
    pub stack_protector: Option<bool>,
    /// プロファイリング
    pub profiling: Option<bool>,
    /// カバレッジ
    pub coverage: Option<bool>,
}

/// テストセクション
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestInfo {
    /// テストターゲット
    pub targets: Option<Vec<String>>,
    /// テスト除外
    pub exclude: Option<Vec<String>>,
    /// テストフラグ
    pub flags: Option<Vec<String>>,
    /// テスト環境変数
    pub env: Option<HashMap<String, String>>,
    /// テストタイムアウト（秒）
    pub timeout: Option<u64>,
    /// テスト並列実行数
    pub threads: Option<u32>,
    /// テストフィルター
    pub filter: Option<String>,
    /// テストフレームワーク
    pub framework: Option<String>,
    /// テストレポート形式
    pub report_format: Option<String>,
    /// テストレポート出力先
    pub report_output: Option<String>,
}

/// ベンチマークセクション
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkInfo {
    /// ベンチマークターゲット
    pub targets: Option<Vec<String>>,
    /// ベンチマーク除外
    pub exclude: Option<Vec<String>>,
    /// ベンチマークフラグ
    pub flags: Option<Vec<String>>,
    /// ベンチマーク環境変数
    pub env: Option<HashMap<String, String>>,
    /// ベンチマークタイムアウト（秒）
    pub timeout: Option<u64>,
    /// ベンチマーク繰り返し回数
    pub iterations: Option<u32>,
    /// ベンチマークウォームアップ回数
    pub warmup: Option<u32>,
    /// ベンチマークフレームワーク
    pub framework: Option<String>,
    /// ベンチマークレポート形式
    pub report_format: Option<String>,
    /// ベンチマークレポート出力先
    pub report_output: Option<String>,
}

/// ドキュメントセクション
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationInfo {
    /// ドキュメントターゲット
    pub targets: Option<Vec<String>>,
    /// ドキュメント除外
    pub exclude: Option<Vec<String>>,
    /// ドキュメントフラグ
    pub flags: Option<Vec<String>>,
    /// ドキュメント環境変数
    pub env: Option<HashMap<String, String>>,
    /// ドキュメント出力先
    pub output: Option<String>,
    /// ドキュメントテーマ
    pub theme: Option<String>,
    /// ドキュメントプライベート項目
    pub private_items: Option<bool>,
    /// ドキュメントテンプレート
    pub templates: Option<String>,
    /// ドキュメント追加ファイル
    pub additional_files: Option<Vec<String>>,
    /// ドキュメントフォーマット
    pub format: Option<String>,
}

/// パッケージセクション
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackagingInfo {
    /// パッケージ形式
    pub format: Option<Vec<String>>,
    /// パッケージ含めるファイル
    pub include: Option<Vec<String>>,
    /// パッケージ除外ファイル
    pub exclude: Option<Vec<String>>,
    /// パッケージメタデータ
    pub metadata: Option<HashMap<String, String>>,
    /// パッケージスクリプト
    pub scripts: Option<PackagingScripts>,
    /// パッケージ署名設定
    pub signing: Option<PackageSigning>,
    /// パッケージ圧縮設定
    pub compression: Option<String>,
    /// パッケージ出力先
    pub output_dir: Option<String>,
    /// パッケージ命名パターン
    pub naming_pattern: Option<String>,
}

/// パッケージスクリプト
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackagingScripts {
    /// パッケージ前スクリプト
    pub pre_package: Option<Vec<String>>,
    /// パッケージ後スクリプト
    pub post_package: Option<Vec<String>>,
    /// インストール前スクリプト
    pub pre_install: Option<Vec<String>>,
    /// インストール後スクリプト
    pub post_install: Option<Vec<String>>,
    /// アンインストール前スクリプト
    pub pre_uninstall: Option<Vec<String>>,
    /// アンインストール後スクリプト
    pub post_uninstall: Option<Vec<String>>,
}

/// パッケージ署名設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageSigning {
    /// 署名を有効にするか
    pub enabled: Option<bool>,
    /// 署名キー
    pub key: Option<String>,
    /// 署名証明書
    pub certificate: Option<String>,
    /// 署名アルゴリズム
    pub algorithm: Option<String>,
    /// 署名タイムスタンプサーバー
    pub timestamp_server: Option<String>,
}

/// マニフェスト
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// パッケージ情報
    pub package: PackageInfo,
    /// 依存関係
    #[serde(default)]
    pub dependencies: HashMap<String, toml::Value>,
    /// 開発依存関係
    #[serde(default, rename = "dev-dependencies")]
    pub dev_dependencies: HashMap<String, toml::Value>,
    /// ビルド依存関係
    #[serde(default, rename = "build-dependencies")]
    pub build_dependencies: HashMap<String, toml::Value>,
    /// ターゲット依存関係
    #[serde(default)]
    pub target: Option<HashMap<String, HashMap<String, toml::Value>>>,
    /// ライブラリセクション
    pub lib: Option<LibraryInfo>,
    /// バイナリセクション
    #[serde(default)]
    pub bin: Vec<BinaryInfo>,
    /// フィーチャーセクション
    #[serde(default)]
    pub features: HashMap<String, Vec<String>>,
    /// ワークスペース
    pub workspace: Option<WorkspaceInfo>,
    /// ビルド情報
    pub build: Option<BuildInfo>,
    /// プロファイル
    #[serde(default)]
    pub profile: HashMap<String, ProfileInfo>,
    /// メタデータ
    #[serde(default)]
    pub metadata: HashMap<String, toml::Value>,
    /// テスト設定
    pub test: Option<TestInfo>,
    /// ベンチマーク設定
    pub bench: Option<BenchmarkInfo>,
    /// ドキュメント設定
    pub doc: Option<DocumentationInfo>,
    /// パッケージング設定
    pub package_info: Option<PackagingInfo>,
    /// パッチセクション
    #[serde(default)]
    pub patch: HashMap<String, HashMap<String, toml::Value>>,
    /// リプレースセクション
    #[serde(default)]
    pub replace: HashMap<String, toml::Value>,
    /// プラグインセクション
    #[serde(default)]
    pub plugins: HashMap<String, toml::Value>,
    /// スクリプトセクション
    #[serde(default)]
    pub scripts: HashMap<String, String>,
    /// 環境変数セクション
    #[serde(default)]
    pub env: HashMap<String, String>,
}

/// ワークスペース情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    /// メンバー
    pub members: Vec<String>,
    /// 除外メンバー
    #[serde(default)]
    pub exclude: Vec<String>,
    /// 継承設定
    pub inheritance: Option<InheritanceInfo>,
    /// デフォルトメンバー
    pub default_members: Option<Vec<String>>,
    /// メタデータ
    pub metadata: Option<HashMap<String, toml::Value>>,
    /// 依存関係
    pub dependencies: Option<HashMap<String, toml::Value>>,
    /// ワークスペースレイアウト
    pub layout: Option<WorkspaceLayout>,
    /// ワークスペースリゾルバー設定
    pub resolver: Option<WorkspaceResolverConfig>,
}

/// ワークスペースレイアウト
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceLayout {
    /// パッケージディレクトリ
    pub packages_dir: Option<String>,
    /// ターゲットディレクトリ
    pub target_dir: Option<String>,
    /// キャッシュディレクトリ
    pub cache_dir: Option<String>,
    /// ドキュメントディレクトリ
    pub docs_dir: Option<String>,
}

/// ワークスペースリゾルバー設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceResolverConfig {
    /// リゾルバーバージョン
    pub version: Option<u32>,
    /// フィーチャーユニフィケーション
    pub features: Option<String>,
    /// 依存関係オーバーライド
    pub overrides: Option<HashMap<String, String>>,
}

/// 継承設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InheritanceInfo {
    /// 依存関係の継承
    pub dependencies: Option<bool>,
    /// 開発依存関係の継承
    #[serde(rename = "dev-dependencies")]
    pub dev_dependencies: Option<bool>,
    /// フィーチャーの継承
    pub features: Option<bool>,
    /// プロファイルの継承
    pub profile: Option<bool>,
    /// ビルド設定の継承
    pub build: Option<bool>,
    /// テスト設定の継承
    pub test: Option<bool>,
    /// ベンチマーク設定の継承
    pub bench: Option<bool>,
    /// ドキュメント設定の継承
    pub doc: Option<bool>,
    /// パッケージング設定の継承
    pub package: Option<bool>,
    /// 環境変数の継承
    pub env: Option<bool>,
    /// スクリプトの継承
    pub scripts: Option<bool>,
}

impl Manifest {
    /// 新しいマニフェストを作成
    pub fn new(name: &str, version: &str, authors: Vec<String>) -> Self {
        let now = Utc::now();
        
        Manifest {
            package: PackageInfo {
                name: name.to_string(),
                version: version.to_string(),
                authors,
                description: None,
                repository: None,
                license: None,
                keywords: None,
                categories: None,
                documentation: None,
                homepage: None,
                edition: Some("2022".to_string()),
                publish: Some(true),
                rust_version: None,
                metadata: None,
                created_at: Some(now),
                updated_at: Some(now),
                readme: None,
                icon: None,
                screenshots: None,
                language: None,
                support: None,
                security: None,
                compatibility: None,
            },
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
            build_dependencies: HashMap::new(),
            target: None,
            lib: None,
            bin: Vec::new(),
            features: HashMap::new(),
            workspace: None,
            build: None,
            profile: HashMap::new(),
            metadata: HashMap::new(),
            test: None,
            bench: None,
            doc: None,
            package_info: None,
            patch: HashMap::new(),
            replace: HashMap::new(),
            plugins: HashMap::new(),
            scripts: HashMap::new(),
            env: HashMap::new(),
        }
    }

    /// マニフェストを読み込む
    pub fn load(path: &Path) -> Result<Self> {
        let mut file = File::open(path)
            .with_context(|| format!("マニフェストを開けません: {}", path.display()))?;
        
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .with_context(|| format!("マニフェストを読み込めません: {}", path.display()))?;
        
        let manifest: Manifest = toml::from_str(&contents)
            .with_context(|| format!("マニフェストのパースに失敗しました: {}", path.display()))?;
        
        Ok(manifest)
    }

    /// マニフェストを保存
    pub fn save(&self, path: &Path) -> Result<()> {
        let contents = toml::to_string_pretty(&self)
            .with_context(|| "マニフェストのシリアライズに失敗しました")?;
        
        // ディレクトリが存在しない場合は作成
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("ディレクトリを作成できません: {}", parent.display()))?;
            }
        }
        
        let mut file = File::create(path)
            .with_context(|| format!("マニフェストを作成できません: {}", path.display()))?;
        
        file.write_all(contents.as_bytes())
            .with_context(|| format!("マニフェストに書き込めません: {}", path.display()))?;
        
        Ok(())
    }

    /// 依存関係を追加
    pub fn add_dependency(&mut self, name: &str, version: &str) {
        self.dependencies.insert(
            name.to_string(),
            toml::Value::String(version.to_string()),
        );
    }

    /// 依存関係をテーブルとして追加
    pub fn add_dependency_with_features(&mut self, name: &str, version: &str, features: Vec<String>) {
        let mut table = toml::value::Table::new();
        table.insert("version".to_string(), toml::Value::String(version.to_string()));
        table.insert("features".to_string(), toml::Value::Array(
            features.into_iter().map(toml::Value::String).collect()
        ));
        self.dependencies.insert(name.to_string(), toml::Value::Table(table));
    }

    /// 開発依存関係を追加
    pub fn add_dev_dependency(&mut self, name: &str, version: &str) {
        self.dev_dependencies.insert(
            name.to_string(),
            toml::Value::String(version.to_string()),
        );
    }

    /// ビルド依存関係を追加
    pub fn add_build_dependency(&mut self, name: &str, version: &str) {
        self.build_dependencies.insert(
            name.to_string(),
            toml::Value::String(version.to_string()),
        );
    }

    /// ターゲット依存関係を追加
    pub fn add_target_dependency(&mut self, target: &str, name: &str, version: &str) -> Result<()> {
        if self.target.is_none() {
            self.target = Some(HashMap::new());
        }
        
        let target_map = self.target.as_mut().unwrap();
        
        if !target_map.contains_key(target) {
            target_map.insert(target.to_string(), HashMap::new());
        }
        
        let deps = target_map.get_mut(target).unwrap();
        deps.insert(name.to_string(), toml::Value::String(version.to_string()));
        
        Ok(())
    }

    /// バイナリを追加
    pub fn add_binary(&mut self, name: &str, path: Option<&str>) {
        let bin = BinaryInfo {
            name: name.to_string(),
            path: path.map(|s| s.to_string()),
            required_features: None,
            target: None,
            build_flags: None,
            env: None,
            description: None,
            version: None,
            icon: None,
            category: None,
            main: None,
            dependencies: None,
        };
        self.bin.push(bin);
    }

    /// 詳細なバイナリを追加
    pub fn add_binary_with_details(&mut self, binary: BinaryInfo) {
        self.bin.push(binary);
    }

    /// ライブラリを設定
    pub fn set_library(&mut self, name: Option<&str>, path: Option<&str>, crate_type: Option<Vec<String>>) {
        let lib = LibraryInfo {
            name: name.map(|s| s.to_string()),
            path: path.map(|s| s.to_string()),
            crate_type,
            doc: None,
            test: None,
            bench: None,
            doc_private: None,
            proc_macro: None,
            edition: None,
            target: None,
            exports: None,
            stability: None,
            api_compatibility: None,
        };
        self.lib = Some(lib);
    }

    /// 詳細なライブラリを設定
    pub fn set_library_with_details(&mut self, library: LibraryInfo) {
        self.lib = Some(library);
    }

    /// フィーチャーを追加
    pub fn add_feature(&mut self, name: &str, dependencies: Vec<String>) {
        self.features.insert(name.to_string(), dependencies);
    }

    /// プロファイルを追加
    pub fn add_profile(&mut self, name: &str, profile: ProfileInfo) {
        self.profile.insert(name.to_string(), profile);
    }

    /// スクリプトを追加
    pub fn add_script(&mut self, name: &str, command: &str) {
        self.scripts.insert(name.to_string(), command.to_string());
    }

    /// 環境変数を追加
    /// バージョンを更新
    pub fn update_version(&mut self, version: &str) -> Result<()> {
        // バージョンの妥当性をチェック
        let _ = Version::parse(version)
            .with_context(|| format!("無効なバージョン: {}", version))?;
        
        self.package.version = version.to_string();
        Ok(())
    }

    /// 通常の依存関係を取得
    pub fn get_dependencies(&self) -> Result<Vec<Dependency>> {
        self.parse_dependencies(&self.dependencies, DependencyType::Normal)
    }

    /// 開発依存関係を取得
    pub fn get_dev_dependencies(&self) -> Result<Vec<Dependency>> {
        self.parse_dependencies(&self.dev_dependencies, DependencyType::Dev)
    }

    /// ビルド依存関係を取得
    pub fn get_build_dependencies(&self) -> Result<Vec<Dependency>> {
        self.parse_dependencies(&self.build_dependencies, DependencyType::Build)
    }

    /// すべての依存関係を取得
    pub fn get_all_dependencies(&self) -> Result<Vec<Dependency>> {
        let mut deps = Vec::new();
        
        deps.extend(self.get_dependencies()?);
        deps.extend(self.get_dev_dependencies()?);
        deps.extend(self.get_build_dependencies()?);
        
        Ok(deps)
    }

    // 依存関係をパース
    fn parse_dependencies(&self, deps_map: &HashMap<String, toml::Value>, dep_type: DependencyType) -> Result<Vec<Dependency>> {
        let mut result = Vec::new();
        
        for (name, value) in deps_map {
            match value {
                toml::Value::String(version) => {
                    let dep = Dependency::new(name, version)
                        .with_dep_type(dep_type.clone());
                    result.push(dep);
                },
                toml::Value::Table(table) => {
                    let version = match table.get("version") {
                        Some(toml::Value::String(v)) => v.clone(),
                        _ => "".to_string(),
                    };
                    
                    let mut dep = Dependency::new(name, &version)
                        .with_dep_type(dep_type.clone());
                    
                    // フィーチャーの解析
                    if let Some(toml::Value::Array(features)) = table.get("features") {
                        let mut feat_list = Vec::new();
                        for feat in features {
                            if let toml::Value::String(f) = feat {
                                feat_list.push(f.clone());
                            }
                        }
                        dep = dep.with_features(feat_list);
                    }
                    
                    // オプション依存関係の解析
                    if let Some(toml::Value::Boolean(optional)) = table.get("optional") {
                        if *optional {
                            dep = dep.as_optional();
                        }
                    }
                    
                    // ソースの解析
                    if let Some(toml::Value::String(path)) = table.get("path") {
                        dep = dep.with_source(DependencySource::Path { 
                            path: PathBuf::from(path) 
                        });
                    } else if let Some(toml::Value::String(git)) = table.get("git") {
                        let mut branch = None;
                        let mut tag = None;
                        let mut rev = None;
                        
                        if let Some(toml::Value::String(b)) = table.get("branch") {
                            branch = Some(b.clone());
                        }
                        
                        if let Some(toml::Value::String(t)) = table.get("tag") {
                            tag = Some(t.clone());
                        }
                        
                        if let Some(toml::Value::String(r)) = table.get("rev") {
                            rev = Some(r.clone());
                        }
                        
                        dep = dep.with_source(DependencySource::Git { 
                            url: git.clone(),
                            branch,
                            tag,
                            rev,
                        });
                    } else if let Some(toml::Value::String(registry)) = table.get("registry") {
                        dep = dep.with_source(DependencySource::Registry { 
                            url: registry.clone() 
                        });
                    } else {
                        dep = dep.with_source(DependencySource::Registry { 
                            url: "https://registry.swiftlight.io".to_string() 
                        });
                    }
                    
                    result.push(dep);
                },
                _ => {
                    return Err(anyhow!("依存関係のパースに失敗しました: {}", name));
                },
            }
        }
        
        Ok(result)
    }
}

/// 新しいマニフェストを作成
pub fn create_new_manifest(
    name: &str,
    version: &str,
    authors: Vec<String>,
    description: Option<String>,
    edition: Option<String>,
    license: Option<String>,
) -> Manifest {
    let mut manifest = Manifest::new(name, version, authors);
    
    if let Some(desc) = description {
        manifest.package.description = Some(desc);
    }
    
    if let Some(ed) = edition {
        manifest.package.edition = Some(ed);
    }
    
    if let Some(lic) = license {
        manifest.package.license = Some(lic);
    }
    
    manifest
}

/// マニフェストを検証
pub fn validate_manifest(manifest: &Manifest) -> Result<Vec<String>> {
    let mut warnings = Vec::new();
    
    // 必須フィールドの検証
    if manifest.package.name.is_empty() {
        return Err(anyhow!("パッケージ名が指定されていません"));
    }
    
    if manifest.package.version.is_empty() {
        return Err(anyhow!("バージョンが指定されていません"));
    }
    
    // バージョンの妥当性をチェック
    if let Err(e) = Version::parse(&manifest.package.version) {
        return Err(anyhow!("無効なバージョン: {}", e));
    }
    
    // ライブラリとバイナリの両方が存在するかチェック
    if manifest.lib.is_none() && manifest.bin.is_empty() {
        warnings.push("ライブラリもバイナリも指定されていません。少なくとも一つを指定してください。".to_string());
    }
    
    // 依存関係の循環参照チェック（実際の実装では複雑になりますが、ここではモックとして警告だけ出します）
    // ...
    
    Ok(warnings)
}

/// マニフェストのバージョンを更新
pub fn update_manifest_version(manifest_path: &Path, new_version: &str) -> Result<()> {
    let mut manifest = Manifest::load(manifest_path)?;
    manifest.update_version(new_version)?;
    manifest.save(manifest_path)?;
    
    Ok(())
}

/// マニフェストの存在確認
pub fn manifest_exists(project_dir: &Path) -> bool {
    let manifest_path = project_dir.join("swiftlight.toml");
    manifest_path.exists()
}

/// タスクを実行するためのユーティリティ関数
impl Dependency {
    /// 新しい依存関係を作成
    pub fn new(name: &str, version: &str) -> Self {
        use semver::VersionReq;
        
        let version_req = match VersionReq::parse(version) {
            Ok(v) => Some(v),
            Err(_) => None,
        };
        
        Dependency {
            name: name.to_string(),
            version_req,
            dep_type: DependencyType::Normal,
            source: DependencySource::Registry { 
                url: "https://registry.swiftlight.io".to_string() 
            },
            optional: false,
            features: Vec::new(),
            no_default_features: false,
        }
    }
    
    /// 依存関係タイプを設定
    pub fn with_dep_type(mut self, dep_type: DependencyType) -> Self {
        self.dep_type = dep_type;
        self
    }
    
    /// フィーチャーを設定
    pub fn with_features(mut self, features: Vec<String>) -> Self {
        self.features = features;
        self
    }
    
    /// オプション依存関係として設定
    pub fn as_optional(mut self) -> Self {
        self.optional = true;
        self
    }
    
    /// ソースを設定
    pub fn with_source(mut self, source: DependencySource) -> Self {
        self.source = source;
        self
    }
    
    /// デフォルトフィーチャーを無効化
    pub fn with_no_default_features(mut self) -> Self {
        self.no_default_features = true;
        self
    }
}
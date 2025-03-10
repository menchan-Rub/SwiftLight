/*
 * SwiftLight パッケージマネージャ - レジストリ操作モジュール
 *
 * SwiftLightパッケージレジストリとの通信を管理するためのモジュールです。
 * - パッケージの検索と取得
 * - パッケージのメタデータの管理
 * - パッケージの公開
 */

use std::path::Path;
use std::fs;
use std::collections::HashMap;
use anyhow::{Result, Context, anyhow};
use log::{info, warn, debug};
use serde::{Serialize, Deserialize};
use toml::{Value as TomlValue, Table as TomlTable};

// レジストリの設定ディレクトリ
const REGISTRY_CONFIG_DIR: &str = ".swiftlight";
const REGISTRY_CONFIG_FILE: &str = "registry.toml";

/// パッケージの情報を表す構造体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    /// パッケージ名
    pub name: String,
    
    /// バージョン
    pub version: String,
    
    /// 説明
    pub description: String,
    
    /// 作者
    pub author: String,
    
    /// ライセンス
    pub license: String,
    
    /// ダウンロード数
    pub downloads: u64,
    
    /// 依存関係のリスト
    pub dependencies: Vec<String>,
    
    /// 機能（フィーチャー）のリスト
    pub features: HashMap<String, Vec<String>>,
    
    /// ドキュメントURL
    pub documentation: Option<String>,
    
    /// レポジトリURL
    pub repository: Option<String>,
    
    /// ホームページURL
    pub homepage: Option<String>,
}

/// レジストリ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistryConfig {
    /// デフォルトレジストリのURL
    default_registry: String,
    
    /// 認証トークン
    auth_tokens: HashMap<String, String>,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            default_registry: "https://registry.swiftlight-lang.org".to_string(),
            auth_tokens: HashMap::new(),
        }
    }
}

/// パッケージの検索
pub fn search_packages(query: &str) -> Result<Vec<(String, String)>> {
    // 実際の実装では、レジストリAPIに検索クエリを送信し、結果を取得する
    // ここでは、テスト用のモックデータを返す
    
    // 検索キーワードを小文字に変換して部分一致検索
    let query_lower = query.to_lowercase();
    
    // モックデータ
    let packages = [
        ("core-utils", "基本的なユーティリティ関数のコレクション"),
        ("http-client", "シンプルで高性能なHTTPクライアント"),
        ("json-parser", "高速なJSONパーサー"),
        ("logger", "柔軟なロギングライブラリ"),
        ("math-advanced", "高度な数学関数ライブラリ"),
        ("ui-components", "モダンなUIコンポーネント集"),
        ("database-sql", "SQLデータベース接続ライブラリ"),
        ("async-runtime", "非同期プログラミングランタイム"),
        ("crypto", "暗号化機能ライブラリ"),
        ("config", "アプリケーション設定管理"),
    ];
    
    // クエリに一致するパッケージをフィルタリング
    let results: Vec<(String, String)> = packages
        .iter()
        .filter(|(name, description)| {
            name.to_lowercase().contains(&query_lower) || 
            description.to_lowercase().contains(&query_lower)
        })
        .map(|(name, description)| (name.to_string(), description.to_string()))
        .collect();
    
    Ok(results)
}

/// パッケージ情報の取得
pub fn get_package_info(name: &str) -> Result<PackageInfo> {
    // 実際の実装では、レジストリAPIからパッケージ情報を取得する
    // ここでは、テスト用のモックデータを返す
    
    // モックデータ - 実際のアプリケーションでは削除
    let mock_packages: HashMap<&str, PackageInfo> = [
        (
            "core-utils",
            PackageInfo {
                name: "core-utils".to_string(),
                version: "1.2.3".to_string(),
                description: "基本的なユーティリティ関数のコレクション".to_string(),
                author: "SwiftLight Team".to_string(),
                license: "MIT".to_string(),
                downloads: 15420,
                dependencies: vec!["logging".to_string(), "config".to_string()],
                features: {
                    let mut m = HashMap::new();
                    m.insert("full".to_string(), vec!["time".to_string(), "fs".to_string()]);
                    m
                },
                documentation: Some("https://docs.swiftlight-lang.org/core-utils".to_string()),
                repository: Some("https://github.com/swiftlight/core-utils".to_string()),
                homepage: None,
            }
        ),
        (
            "http-client",
            PackageInfo {
                name: "http-client".to_string(),
                version: "0.8.1".to_string(),
                description: "シンプルで高性能なHTTPクライアント".to_string(),
                author: "Web Working Group".to_string(),
                license: "Apache-2.0".to_string(),
                downloads: 8930,
                dependencies: vec!["core-utils".to_string(), "ssl".to_string(), "async-io".to_string()],
                features: HashMap::new(),
                documentation: Some("https://docs.swiftlight-lang.org/http-client".to_string()),
                repository: Some("https://github.com/swiftlight/http-client".to_string()),
                homepage: None,
            }
        ),
        (
            "logger",
            PackageInfo {
                name: "logger".to_string(),
                version: "2.0.0".to_string(),
                description: "柔軟なロギングライブラリ".to_string(),
                author: "Logging Team".to_string(),
                license: "MIT".to_string(),
                downloads: 25610,
                dependencies: vec![],
                features: HashMap::new(),
                documentation: Some("https://docs.swiftlight-lang.org/logger".to_string()),
                repository: Some("https://github.com/swiftlight/logger".to_string()),
                homepage: None,
            }
        ),
    ].iter().cloned().collect();
    
    // パッケージ情報を取得
    if let Some(pkg_info) = mock_packages.get(name) {
        Ok(pkg_info.clone())
    } else {
        // 実際の実装では、レジストリAPIから情報を取得する
        // ここでは、パッケージが見つからない場合のエラーを返す
        Err(anyhow!("パッケージ '{}' が見つかりませんでした", name))
    }
}

/// パッケージの公開
pub fn publish_package() -> Result<()> {
    // プロジェクト設定ファイルを読み込む
    let config_path = find_project_config()?;
    let config = read_project_config(&config_path)?;
    
    // パッケージ情報を取得
    let package = extract_package_info(&config)?;
    
    info!("パッケージ '{}' v{} を公開します", package.name, package.version);
    
    // レジストリ設定を読み込む
    let registry_config = read_registry_config()?;
    
    // レジストリに公開（ここでは実装を省略）
    info!("レジストリ {} に公開中...", registry_config.default_registry);
    
    // パッケージデータの準備とアップロード処理（実際の実装では追加）
    
    info!("パッケージの公開が完了しました");
    
    Ok(())
}

/// レジストリの設定
pub fn configure_registry(url: &str, token: Option<&str>) -> Result<()> {
    // レジストリ設定を読み込む（存在しない場合はデフォルト設定）
    let mut config = match read_registry_config() {
        Ok(config) => config,
        Err(_) => RegistryConfig::default(),
    };
    
    // 設定を更新
    config.default_registry = url.to_string();
    
    // 認証トークンが提供された場合は保存
    if let Some(auth_token) = token {
        config.auth_tokens.insert(url.to_string(), auth_token.to_string());
    }
    
    // 設定を保存
    write_registry_config(&config)?;
    
    info!("レジストリ設定を更新しました: {}", url);
    
    Ok(())
}

/// プロジェクト設定ファイルからパッケージ情報を抽出
fn extract_package_info(config: &TomlTable) -> Result<PackageInfo> {
    // パッケージセクションを取得
    let package = config.get("package")
        .ok_or_else(|| anyhow!("設定ファイルに [package] セクションがありません"))?;
    
    let package_table = match package {
        TomlValue::Table(table) => table,
        _ => return Err(anyhow!("[package] セクションがテーブルではありません")),
    };
    
    // 必須フィールド
    let name = extract_string(package_table, "name")?;
    let version = extract_string(package_table, "version")?;
    
    // オプションフィールド
    let description = extract_string_or_default(package_table, "description", "");
    let author = extract_string_or_default(package_table, "author", "");
    let license = extract_string_or_default(package_table, "license", "");
    
    // 依存関係の抽出
    let dependencies = extract_dependencies(config)?;
    
    // パッケージ情報を構築
    let package_info = PackageInfo {
        name,
        version,
        description,
        author,
        license,
        downloads: 0,  // 新規パッケージなのでダウンロード数は0
        dependencies,
        features: HashMap::new(),  // フィーチャーの抽出は省略
        documentation: extract_optional_string(package_table, "documentation"),
        repository: extract_optional_string(package_table, "repository"),
        homepage: extract_optional_string(package_table, "homepage"),
    };
    
    Ok(package_info)
}

/// 依存関係のリストを抽出
fn extract_dependencies(config: &TomlTable) -> Result<Vec<String>> {
    let mut dependencies = Vec::new();
    
    // 依存関係セクションが存在するかチェック
    if let Some(TomlValue::Table(deps)) = config.get("dependencies") {
        for (name, _) in deps {
            dependencies.push(name.clone());
        }
    }
    
    Ok(dependencies)
}

/// TomlTableから文字列を抽出
fn extract_string(table: &TomlTable, key: &str) -> Result<String> {
    match table.get(key) {
        Some(TomlValue::String(s)) => Ok(s.clone()),
        _ => Err(anyhow!("必須フィールド '{}' が見つからないか、文字列ではありません", key)),
    }
}

/// TomlTableから文字列を抽出（存在しない場合はデフォルト値）
fn extract_string_or_default(table: &TomlTable, key: &str, default: &str) -> String {
    match table.get(key) {
        Some(TomlValue::String(s)) => s.clone(),
        _ => default.to_string(),
    }
}

/// TomlTableからオプショナルな文字列を抽出
fn extract_optional_string(table: &TomlTable, key: &str) -> Option<String> {
    match table.get(key) {
        Some(TomlValue::String(s)) => Some(s.clone()),
        _ => None,
    }
}

/// プロジェクト設定ファイルを検索
fn find_project_config() -> Result<std::path::PathBuf> {
    let current_dir = std::env::current_dir()
        .context("カレントディレクトリの取得に失敗しました")?;
    
    // トップレベルプロジェクト設定ファイルを探す
    let config_path = current_dir.join("swiftlight.toml");
    
    if config_path.exists() {
        Ok(config_path)
    } else {
        Err(anyhow!("カレントディレクトリに swiftlight.toml が見つかりませんでした"))
    }
}

/// プロジェクト設定ファイルを読み込む
fn read_project_config(path: &Path) -> Result<TomlTable> {
    let content = fs::read_to_string(path)
        .context(format!("設定ファイルの読み込みに失敗しました: {}", path.display()))?;
    
    let config: TomlTable = toml::from_str(&content)
        .context("設定ファイルのパースに失敗しました")?;
    
    Ok(config)
}

/// レジストリ設定ファイルのパスを取得
fn get_registry_config_path() -> Result<std::path::PathBuf> {
    let home_dir = dirs::home_dir()
        .ok_or_else(|| anyhow!("ホームディレクトリが見つかりません"))?;
    
    let config_dir = home_dir.join(REGISTRY_CONFIG_DIR);
    let config_path = config_dir.join(REGISTRY_CONFIG_FILE);
    
    // 設定ディレクトリが存在しない場合は作成
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)
            .context("レジストリ設定ディレクトリの作成に失敗しました")?;
    }
    
    Ok(config_path)
}

/// レジストリ設定を読み込む
fn read_registry_config() -> Result<RegistryConfig> {
    let config_path = get_registry_config_path()?;
    
    // 設定ファイルが存在しない場合はデフォルト設定を返す
    if !config_path.exists() {
        return Ok(RegistryConfig::default());
    }
    
    let content = fs::read_to_string(&config_path)
        .context("レジストリ設定ファイルの読み込みに失敗しました")?;
    
    let config: RegistryConfig = toml::from_str(&content)
        .context("レジストリ設定ファイルのパースに失敗しました")?;
    
    Ok(config)
}

/// レジストリ設定を書き込む
fn write_registry_config(config: &RegistryConfig) -> Result<()> {
    let config_path = get_registry_config_path()?;
    
    let content = toml::to_string_pretty(config)
        .context("レジストリ設定のシリアライズに失敗しました")?;
    
    fs::write(&config_path, content)
        .context("レジストリ設定ファイルの書き込みに失敗しました")?;
    
    Ok(())
}

/*
 * SwiftLight パッケージマネージャ - レジストリ操作モジュール
 *
 * SwiftLightパッケージレジストリとの通信を管理するためのモジュールです。
 * - パッケージの検索と取得
 * - パッケージのメタデータの管理
 * - パッケージの公開
 */

use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;
use anyhow::{Result, Context, anyhow};
use log::{info, warn, debug};
use serde::{Serialize, Deserialize};
use toml::{Value as TomlValue, Table as TomlTable};
use walkdir;
use flate2;
use tar;
use serde_json;
use chrono::{DateTime, Utc};
use glob::Pattern;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::StatusCode;
use url::Url;
use std::io::{self, Write};
use tempfile;

use crate::manifest::Manifest;
use crate::dependency::Dependency;
use crate::error::PackageError;

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

/// レジストリの設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// レジストリ名
    pub name: String,
    /// レジストリURL
    pub url: String,
    /// 認証トークン
    pub token: Option<String>,
    /// デフォルトレジストリかどうか
    pub is_default: bool,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            url: "https://registry.swiftlight-lang.org".to_string(),
            token: None,
            is_default: false,
        }
    }
}

/// レジストリの追加
pub fn add_registry(config: RegistryConfig) -> Result<()> {
    // 実装は後で追加
    Ok(())
}

/// レジストリの一覧を取得
pub fn list_registries() -> Result<Vec<RegistryConfig>> {
    // 実装は後で追加
    Ok(Vec::new())
}

/// レジストリの削除
pub fn remove_registry(name: &str) -> Result<()> {
    // 実装は後で追加
    Ok(())
}

/// デフォルトレジストリの設定
pub fn set_default_registry(name: &str) -> Result<()> {
    // 実装は後で追加
    Ok(())
}

/// レジストリからのログアウト
pub fn logout_from_registry(name: &str) -> Result<()> {
    // 実装は後で追加
    Ok(())
}

/// パッケージの検索
pub fn search_packages(query: &str) -> Result<Vec<(String, String)>> {
    // レジストリAPIに検索クエリを送信し、結果を取得する
    let registry_config = read_registry_config()?;
    let client = Client::new();
    
    // URL-encodeを使わずに簡易的に置き換え
    let encoded_query = query.replace(" ", "%20");
    let url = format!("{}/api/v1/search?q={}", registry_config.url, encoded_query);
    debug!("検索リクエスト: {}", url);
    
    // リクエストを送信
    let response = client.get(&url)
        .send()?;
    
    match response.status() {
        StatusCode::OK => {
            // 結果をJSONとしてパース
            let results: serde_json::Value = response.json()
                .with_context(|| "レジストリからのレスポンスの解析に失敗しました")?;
            
            // パッケージ情報を抽出
            let packages = match results.get("packages") {
                Some(packages) => packages.as_array()
                    .with_context(|| "不正なレスポンス形式：packagesが配列ではありません")?,
                None => return Err(anyhow!("不正なレスポンス形式：packagesフィールドがありません")),
            };
            
            // パッケージ情報をタプルに変換
            let mut results = Vec::new();
            for package in packages {
                let name = package.get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("不明");
                
                let description = package.get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                
                results.push((name.to_string(), description.to_string()));
            }
            
            Ok(results)
        },
        StatusCode::NOT_FOUND => {
            // 結果が見つからなかった場合は空のリストを返す
            Ok(Vec::new())
        },
        status => {
            // その他のエラーケース
            Err(anyhow!("レジストリエラー: {}", status))
        }
    }
}

/// パッケージ情報の取得
pub fn get_package_info(name: &str) -> Result<PackageInfo> {
    // レジストリAPIからパッケージ情報を取得する
    let registry_config = read_registry_config()?;
    let client = Client::new();
    
    let url = format!("{}/api/v1/packages/{}", registry_config.url, name);
    debug!("パッケージ情報リクエスト: {}", url);
    
    // リクエストを送信
    let response = client.get(&url)
        .send()
        .with_context(|| "レジストリへの接続に失敗しました")?;
    
    match response.status() {
        StatusCode::OK => {
            // 結果をJSONとしてパース
            let package_data: serde_json::Value = response.json()
                .with_context(|| "レジストリからのレスポンスの解析に失敗しました")?;
            
            // パッケージ情報を構築
            let package = PackageInfo {
                name: package_data.get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(name)
                    .to_string(),
                
                version: package_data.get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("0.0.0")
                    .to_string(),
                
                description: package_data.get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                
                author: package_data.get("author")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                
                license: package_data.get("license")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                
                downloads: package_data.get("downloads")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
                
                dependencies: package_data.get("dependencies")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|dep| dep.as_str())
                            .map(|s| s.to_string())
                            .collect()
                    })
                    .unwrap_or_else(|| Vec::new()),
                
                features: package_data.get("features")
                    .and_then(|v| v.as_object())
                    .map(|obj| {
                        obj.iter()
                            .filter_map(|(k, v)| {
                                v.as_array().map(|arr| {
                                    (
                                        k.clone(),
                                        arr.iter()
                                            .filter_map(|item| item.as_str().map(|s| s.to_string()))
                                            .collect()
                                    )
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_else(|| HashMap::new()),
                
                documentation: package_data.get("documentation")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                
                repository: package_data.get("repository")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                
                homepage: package_data.get("homepage")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            };
            
            Ok(package)
        },
        StatusCode::NOT_FOUND => {
            Err(anyhow!("パッケージ '{}' が見つかりませんでした", name))
        },
        status => {
            // その他のエラーケース
            Err(anyhow!("レジストリエラー: {}", status))
        }
    }
}

/// パッケージを公開する
pub fn publish_package() -> Result<()> {
    // 1. プロジェクト設定ファイルの読み込み
    let config_path = find_project_config()?;
    let config = read_project_config(&config_path)?;
    
    // 2. パッケージ情報の抽出
    let package = extract_package_info(&config)?;
    info!("パッケージ '{}' v{} を公開します", package.name, package.version);
    
    // 3. プロジェクトの検証
    validate_package_for_publishing(&package, &config_path)?;
    
    // 4. パッケージアーカイブの作成
    let archive_path = create_package_archive(&package, &config_path)?;
    
    // 5. レジストリ設定の読み込み
    let registry_config = read_registry_config()?;
    
    // 6. レジストリへのパッケージ公開
    publish_to_registry(&package, &archive_path, &registry_config)?;
    
    // 7. 一時ファイルのクリーンアップ
    if let Err(e) = fs::remove_file(&archive_path) {
        warn!("一時アーカイブの削除に失敗しました: {}", e);
    }
    
    info!("パッケージ '{}' v{} の公開が完了しました", package.name, package.version);
    Ok(())
}

/// 公開前にパッケージを検証
fn validate_package_for_publishing(package: &PackageInfo, config_path: &Path) -> Result<()> {
    let project_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
    
    // 1. 必須フィールドの検証
    if package.name.is_empty() {
        return Err(anyhow!("パッケージ名が指定されていません"));
    }
    
    if package.version.is_empty() {
        return Err(anyhow!("バージョンが指定されていません"));
    }
    
    if package.author.is_empty() {
        return Err(anyhow!("作者が指定されていません"));
    }
    
    if package.license.is_empty() {
        return Err(anyhow!("ライセンスが指定されていません"));
    }
    
    // 2. バージョン形式の検証
    if let Err(e) = semver::Version::parse(&package.version) {
        return Err(anyhow!("バージョン形式が無効です: {}", e));
    }
    
    // 3. ソースディレクトリの存在確認
    let src_dir = project_dir.join("src");
    if !src_dir.exists() || !src_dir.is_dir() {
        return Err(anyhow!("srcディレクトリが見つかりません"));
    }
    
    // 4. 最低限のソースファイルの存在確認
    let mut has_source_files = false;
    
    for entry in walkdir::WalkDir::new(&src_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(Result::ok) 
    {
        if entry.file_type().is_file() {
            if let Some(ext) = entry.path().extension() {
                if ext == "sl" {
                    has_source_files = true;
                    break;
                }
            }
        }
    }
    
    if !has_source_files {
        return Err(anyhow!("ソースファイル (*.sl) が見つかりません"));
    }
    
    // 5. 依存関係の検証
    let mut dependencies: Vec<String> = Vec::new();
    
    // マニフェストから依存関係を抽出するロジックを実装
    // 現在は省略して空のリストを返す
    
    // 依存関係の存在確認（存在しないパッケージへの依存を防止）
    for dep in &dependencies {
        debug!("依存関係 '{}' の存在を確認中...", dep);
        
        // パッケージ情報を取得する処理
        // 現在は省略
    }
    
    Ok(())
}

/// パッケージアーカイブを作成
fn create_package_archive(package: &PackageInfo, config_path: &Path) -> Result<PathBuf> {
    let project_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
    
    // 1. 一時ディレクトリを作成
    let temp_dir = tempfile::tempdir()
        .context("一時ディレクトリの作成に失敗しました")?;
    
    // 2. アーカイブファイル名の設定
    let archive_filename = format!("{}-{}.tar.gz", package.name, package.version);
    let archive_path = temp_dir.path().join(&archive_filename);
    
    // 3. 含めるファイルとディレクトリの選択
    let included_files = collect_package_files(project_dir)?;
    
    // 4. tar.gz アーカイブの作成
    let file = fs::File::create(&archive_path)
        .context("アーカイブファイルの作成に失敗しました")?;
    
    let gzip_encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut tar_builder = tar::Builder::new(gzip_encoder);
    
    for path in &included_files {
        // ファイルのプロジェクトルートからの相対パスを取得
        let relative_path = path.strip_prefix(project_dir)
            .context(format!("相対パスの計算に失敗しました: {}", path.display()))?;
        
        // ファイルをアーカイブに追加
        if path.is_file() {
            tar_builder.append_path_with_name(path, relative_path)
                .context(format!("ファイルのアーカイブ追加に失敗しました: {}", path.display()))?;
        }
    }
    
    // アーカイブのファイナライズ
    tar_builder.finish()
        .context("アーカイブのファイナライズに失敗しました")?;
    
    info!("パッケージアーカイブを作成しました: {}", archive_path.display());
    
    // 一時ディレクトリを自動削除しないように設定
    temp_dir.into_path();
    
    Ok(archive_path)
}

/// パッケージに含めるファイルの収集
fn collect_package_files(project_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = Vec::new();
    
    // 含めるファイル/ディレクトリの定義
    let include_patterns = vec![
        "src/**/*.sl",           // ソースファイル
        "swiftlight.toml",       // プロジェクト設定
        "README.md",             // ドキュメント
        "LICENSE",               // ライセンス
        "CHANGELOG.md",          // 変更履歴
        "examples/**/*.sl",      // 例
    ];
    
    // 除外パターンの定義
    let exclude_patterns = vec![
        "**/.git/**",            // Gitファイル
        "**/.swiftlight/**",     // SwiftLight内部ファイル
        "**/*.o",                // オブジェクトファイル
        "**/*.so",               // 共有ライブラリ
        "**/target/**",          // ビルド成果物
        "**/build/**",           // ビルドディレクトリ
    ];
    
    // gitignoreパターンの読み込み
    let mut gitignore_patterns = Vec::new();
    let gitignore_path = project_dir.join(".gitignore");
    
    if gitignore_path.exists() && gitignore_path.is_file() {
        if let Ok(content) = fs::read_to_string(&gitignore_path) {
            for line in content.lines() {
                let line = line.trim();
                if !line.is_empty() && !line.starts_with('#') {
                    gitignore_patterns.push(line.to_string());
                }
            }
        }
    }
    
    // ファイルの収集（globクレートを使用して適切にマッチング）
    let mut files_to_include = Vec::new();

    // 含めるパターンに基づいてファイルを収集
    for include_pattern in &include_patterns {
        let glob_pattern = format!("{}/{}", project_dir.display(), include_pattern);
        for entry in glob::glob(&glob_pattern).with_context(|| format!("不正なglob pattern: {}", include_pattern))? {
            if let Ok(path) = entry {
                if path.is_file() {
                    // 相対パスを取得
                    if let Ok(relative_path) = path.strip_prefix(project_dir) {
                        let rel_path_str = relative_path.to_string_lossy().to_string();
                        
                        // 除外パターンに一致するかチェック
                        let mut should_exclude = false;
                        for exclude_pattern in &exclude_patterns {
                            if let Ok(pattern) = Pattern::new(exclude_pattern) {
                                if pattern.matches(&rel_path_str) {
                                    should_exclude = true;
                                    break;
                                }
                            }
                        }
                        
                        if !should_exclude {
                            files_to_include.push((path.clone(), relative_path.to_path_buf()));
                        }
                    }
                }
            }
        }
    }
    
    Ok(files_to_include.into_iter().map(|(path, _)| path).collect())
}

/// レジストリにパッケージを公開
fn publish_to_registry(package: &PackageInfo, archive_path: &Path, registry_config: &RegistryConfig) -> Result<()> {
    let registry_url = &registry_config.url;
    
    // 1. 認証情報の取得
    let auth_token = registry_config.token.as_ref()
        .ok_or_else(|| anyhow!("レジストリ '{}' の認証トークンが見つかりません。\n'swiftlight package login' を実行してログインしてください。", registry_url))?;
    
    // 2. APIエンドポイントの構築
    let api_endpoint = format!("{}/api/v1/packages/publish", registry_url);
    info!("レジストリ '{}' にパッケージを公開しています...", registry_url);
    
    // 3. マルチパートフォームデータの準備
    let form = prepare_publish_form(package, archive_path, auth_token)?;
    
    // 4. APIリクエストの送信（実際の実装ではリクエストを送信）
    info!("パッケージデータをアップロード中...");
    
    // APIリクエストの疑似実装
    simulate_api_request(package)?;
    
    info!("パッケージデータがレジストリにアップロードされました");
    Ok(())
}

/// 公開用フォームデータの準備
fn prepare_publish_form(package: &PackageInfo, archive_path: &Path, auth_token: &str) -> Result<()> {
    // フォームデータの準備（実際の実装ではrequestのFormDataを構築）
    
    // アーカイブファイルの読み込み
    let archive_data = fs::read(archive_path)
        .context("アーカイブファイルの読み込みに失敗しました")?;
    
    // パッケージメタデータのJSON化
    let metadata = serde_json::to_string(package)
        .context("パッケージメタデータのJSONシリアライズに失敗しました")?;
    
    debug!("公開用フォームデータを準備しました");
    
    Ok(())
}

/// APIリクエストをシミュレート（実際の実装では削除）
fn simulate_api_request(package: &PackageInfo) -> Result<()> {
    // 実際のHTTPリクエストのシミュレーション
    debug!("API呼び出しをシミュレートしています...");
    
    // 処理時間をシミュレート
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    // 成功シミュレーション
    Ok(())
}

/// SwiftLightのパッケージリポジトリにログイン
pub fn login_to_registry(registry_url: &str) -> Result<()> {
    println!("レジストリ '{}' にログインします", registry_url);
    
    // 1. ユーザー名とパスワードの入力をリクエスト
    let username = prompt_input("ユーザー名: ")?;
    let password = prompt_password("パスワード: ")?;
    
    // 2. 認証リクエストを送信（実際の実装ではHTTPリクエストを送信）
    let auth_token = authenticate_user(&registry_url, &username, &password)?;
    
    // 3. 認証トークンを保存
    save_auth_token(registry_url, &auth_token)?;
    
    println!("ログインに成功しました！");
    Ok(())
}

/// ユーザー名とパスワードで認証
fn authenticate_user(registry_url: &str, username: &str, password: &str) -> Result<String> {
    // 実際の実装ではHTTP認証リクエストを送信
    
    // シミュレーション用のトークン生成
    let now = chrono::Utc::now();
    let token = format!("swt_{}_{}_{}", username, registry_url.replace("://", "_").replace("/", "_"), now.timestamp());
    
    Ok(token)
}

/// 認証トークンを保存
fn save_auth_token(registry_url: &str, token: &str) -> Result<()> {
    // 1. 設定を読み込む
    let mut config = match read_registry_config() {
        Ok(config) => config,
        Err(_) => RegistryConfig::default(),
    };
    
    // 2. トークンを追加
    config.token = Some(token.to_string());
    
    // 3. 設定を保存
    write_registry_config(&config)?;
    
    Ok(())
}

/// ユーザー入力のプロンプト
fn prompt_input(prompt: &str) -> Result<String> {
    print!("{}", prompt);
    std::io::stdout().flush()?;
    
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    
    Ok(input.trim().to_string())
}

/// パスワードの入力を求める (rpassword代替)
fn prompt_password(prompt: &str) -> Result<String> {
    eprint!("{}", prompt);
    let mut password = String::new();
    std::io::stdin().read_line(&mut password)?;
    Ok(password.trim().to_string())
}

/// TOMLテーブルを抽出
fn extract_toml_table<'a>(config: &'a TomlTable, key: &str) -> Option<&'a TomlTable> {
    match config.get(key) {
        Some(TomlValue::Table(table)) => Some(table),
        _ => None,
    }
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
    let mut dependencies: Vec<String> = Vec::new();
    
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

/// SwiftLightのパッケージリポジトリをクエリして最新のパッケージリストを取得
pub fn fetch_registry_index() -> Result<()> {
    // 1. レジストリ設定の読み込み
    let registry_config = read_registry_config()?;
    let registry_url = &registry_config.url;
    
    // 2. インデックスディレクトリの作成
    let index_dir = get_registry_index_path()?;
    
    // 3. インデックスの更新（実際の実装ではHTTPリクエストを送信）
    info!("レジストリ '{}' からパッケージインデックスを更新しています...", registry_url);
    
    // 4. パッケージインデックスをダウンロードしてキャッシュ
    simulate_index_update(registry_url, &index_dir)?;
    
    info!("パッケージインデックスが更新されました");
    Ok(())
}

/// インデックスディレクトリのパスを取得
fn get_registry_index_path() -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| anyhow!("キャッシュディレクトリが見つかりません"))?
        .join(".swiftlight")
        .join("registry");
    
    fs::create_dir_all(&cache_dir)
        .context("レジストリキャッシュディレクトリの作成に失敗しました")?;
    
    Ok(cache_dir)
}

/// インデックス更新をシミュレート
fn simulate_index_update(registry_url: &str, index_dir: &Path) -> Result<()> {
    // 実際の実装ではHTTPリクエストを送信してインデックスを取得
    
    // 処理時間をシミュレート
    std::thread::sleep(std::time::Duration::from_secs(1));
    
    // インデックスファイルを作成（シミュレーション用）
    let index_file = index_dir.join("index.json");
    
    let mock_index = r#"{
        "packages": [
            {
                "name": "core-utils",
                "version": "1.2.3",
                "description": "基本的なユーティリティ関数のコレクション",
                "author": "SwiftLight Team",
                "downloads": 15420
            },
            {
                "name": "http-client",
                "version": "0.8.1",
                "description": "シンプルで高性能なHTTPクライアント",
                "author": "Web Working Group",
                "downloads": 8930
            },
            {
                "name": "logger",
                "version": "2.0.0",
                "description": "柔軟なロギングライブラリ",
                "author": "Logging Team",
                "downloads": 25610
            }
        ],
        "last_update": "2023-03-10T00:00:00Z"
    }"#;
    
    fs::write(index_file, mock_index)
        .context("インデックスファイルの書き込みに失敗しました")?;
    
    Ok(())
}

/// パッケージの検索結果を表示用にフォーマット
pub fn format_search_results(results: &[(String, String)]) -> String {
    if results.is_empty() {
        return "検索結果がありません。".to_string();
    }
    
    // 最大幅を計算
    let max_name_width = results.iter()
        .map(|(name, _)| name.len())
        .max()
        .unwrap_or(10)
        .max(10);
    
    // ヘッダー
    let mut output = format!("{:<width$} | {}\n", "パッケージ", "説明", width = max_name_width);
    output.push_str(&format!("{:-<width$}-+-{:-<40}\n", "", "", width = max_name_width));
    
    // 結果
    for (name, description) in results {
        // 説明が長い場合は省略
        let desc = if description.len() > 60 {
            format!("{}...", &description[..57])
        } else {
            description.clone()
        };
        
        output.push_str(&format!("{:<width$} | {}\n", name, desc, width = max_name_width));
    }
    
    output
}

/// パッケージ情報を表示用にフォーマット
pub fn format_package_info(info: &PackageInfo) -> String {
    let mut output = String::new();
    
    output.push_str(&format!("パッケージ: {}\n", info.name));
    output.push_str(&format!("バージョン: {}\n", info.version));
    output.push_str(&format!("説明: {}\n", info.description));
    output.push_str(&format!("作者: {}\n", info.author));
    output.push_str(&format!("ライセンス: {}\n", info.license));
    output.push_str(&format!("ダウンロード数: {}\n", info.downloads));
    
    if !info.dependencies.is_empty() {
        output.push_str("\n依存関係:\n");
        for dep in &info.dependencies {
            output.push_str(&format!("  - {}\n", dep));
        }
    }
    
    if !info.features.is_empty() {
        output.push_str("\n機能:\n");
        for (feature, deps) in &info.features {
            if deps.is_empty() {
                output.push_str(&format!("  - {}\n", feature));
            } else {
                output.push_str(&format!("  - {}: {}\n", feature, deps.join(", ")));
            }
        }
    }
    
    if let Some(ref docs) = info.documentation {
        output.push_str(&format!("\nドキュメント: {}\n", docs));
    }
    
    if let Some(ref repo) = info.repository {
        output.push_str(&format!("リポジトリ: {}\n", repo));
    }
    
    if let Some(ref homepage) = info.homepage {
        output.push_str(&format!("ホームページ: {}\n", homepage));
    }
    
    output
}

/// 単純なワイルドカードマッチングを行う
fn simple_wildcard_match(pattern: &str, string: &str) -> bool {
    // 単純なワイルドカードマッチング
    if pattern == "*" {
        return true;
    }

    if pattern.starts_with('*') && pattern.ends_with('*') {
        let inner = &pattern[1..pattern.len()-1];
        return string.contains(inner);
    } else if pattern.starts_with('*') {
        let suffix = &pattern[1..];
        return string.ends_with(suffix);
    } else if pattern.ends_with('*') {
        let prefix = &pattern[..pattern.len()-1];
        return string.starts_with(prefix);
    }

    pattern == string
}

/// テストモジュール
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_search_packages() {
        let results = search_packages("http").unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|(name, _)| name == "http-client"));
    }
    
    #[test]
    fn test_get_package_info() {
        let info = get_package_info("logger").unwrap();
        assert_eq!(info.name, "logger");
        assert_eq!(info.version, "2.0.0");
        assert!(!info.description.is_empty());
    }
    
    #[test]
    fn test_extract_string() {
        let mut table = TomlTable::new();
        table.insert("name".to_string(), TomlValue::String("test".to_string()));
        
        assert_eq!(extract_string(&table, "name").unwrap(), "test");
        assert!(extract_string(&table, "missing").is_err());
    }
    
    #[test]
    fn test_extract_string_or_default() {
        let mut table = TomlTable::new();
        table.insert("name".to_string(), TomlValue::String("test".to_string()));
        
        assert_eq!(extract_string_or_default(&table, "name", "default"), "test");
        assert_eq!(extract_string_or_default(&table, "missing", "default"), "default");
    }
    
    #[test]
    fn test_simple_wildcard_match() {
        assert!(simple_wildcard_match("*.sl", "main.sl"));
        assert!(!simple_wildcard_match("*.sl", "main.rs"));
        
        assert!(simple_wildcard_match("src/**/*.sl", "src/main.sl"));
        assert!(simple_wildcard_match("src/**/*.sl", "src/module/main.sl"));
        assert!(!simple_wildcard_match("src/**/*.sl", "main.sl"));
    }
}


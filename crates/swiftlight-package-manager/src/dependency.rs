/*
 * SwiftLight パッケージマネージャ - 依存関係管理モジュール
 *
 * SwiftLightプロジェクトの依存関係を管理するためのモジュールです。
 * - 依存関係の追加・削除・更新
 * - 依存関係グラフの解決
 * - バージョン制約の処理
 * - セキュリティ監査と脆弱性チェック
 */

use std::path::{Path, PathBuf};
use std::fs;
use std::collections::{HashMap, HashSet};
use anyhow::{Result, Context, anyhow, bail};
use log::{info, warn, debug, error};
use semver::{Version, VersionReq};
use serde::{Serialize, Deserialize};
use toml::{Value as TomlValue, Table as TomlTable};
use chrono::{DateTime, Utc};

use crate::registry::{get_package_info, PackageInfo, Registry};
use crate::security::{SecurityAudit, VulnerabilityInfo, audit_package};
use crate::cache::DependencyCache;
use crate::lock::{LockFile, LockEntry};

/// SwiftLightパッケージの依存関係を表す構造体
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Dependency {
    /// パッケージ名
    pub name: String,
    
    /// バージョン要件（例: "^1.0.0", ">=2.0.0, <3.0.0"）
    pub version_req: String,
    
    /// 開発依存かどうか
    pub dev: bool,
    
    /// 依存元パッケージからの機能要件
    pub features: Vec<String>,
    
    /// オプショナル依存かどうか
    pub optional: bool,
    
    /// パッケージのソース（レジストリURL、パス、Gitリポジトリなど）
    pub source: Option<DependencySource>,
    
    /// 依存関係の追加日時
    pub added_at: Option<DateTime<Utc>>,
    
    /// 最終更新日時
    pub updated_at: Option<DateTime<Utc>>,
    
    /// セキュリティ監査情報
    pub security_audit: Option<SecurityAudit>,
}

/// 依存関係のソース
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DependencySource {
    /// 標準レジストリ
    Registry(String),
    
    /// ローカルパス
    Path(PathBuf),
    
    /// Gitリポジトリ
    Git {
        /// リポジトリURL
        url: String,
        
        /// ブランチ、タグ、コミットハッシュ
        reference: Option<String>,
        
        /// サブディレクトリ
        subdir: Option<String>,
    },
    
    /// HTTP/HTTPS URL
    Url {
        /// URL
        url: String,
        
        /// チェックサム（セキュリティ検証用）
        checksum: Option<String>,
    },
}

/// 依存関係解決の結果
#[derive(Debug, Clone)]
pub struct ResolutionResult {
    /// 解決された依存関係マップ
    pub packages: HashMap<String, ResolvedPackage>,
    
    /// 解決中に検出された警告
    pub warnings: Vec<String>,
    
    /// 依存関係グラフ（キー: パッケージ名、値: 直接依存するパッケージ名のリスト）
    pub dependency_graph: HashMap<String, Vec<String>>,
}

/// 解決されたパッケージ情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedPackage {
    /// パッケージ情報
    pub info: PackageInfo,
    
    /// 解決されたバージョン
    pub resolved_version: Version,
    
    /// 有効化された機能
    pub activated_features: HashSet<String>,
    
    /// 依存元パッケージ
    pub dependent_packages: Vec<String>,
    
    /// セキュリティ監査結果
    pub security_audit: Option<SecurityAudit>,
}

impl Dependency {
    /// 新しい依存関係を作成
    pub fn new(name: String, version_req: String) -> Self {
        Self {
            name,
            version_req,
            dev: false,
            features: Vec::new(),
            optional: false,
            source: None,
            added_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            security_audit: None,
        }
    }
    
    /// 開発依存関係として設定
    pub fn as_dev(mut self) -> Self {
        self.dev = true;
        self
    }
    
    /// 機能を追加
    pub fn with_features(mut self, features: Vec<String>) -> Self {
        self.features = features;
        self
    }
    
    /// オプショナル依存として設定
    pub fn as_optional(mut self) -> Self {
        self.optional = true;
        self
    }
    
    /// ソースを設定
    pub fn with_source(mut self, source: DependencySource) -> Self {
        self.source = Some(source);
        self
    }
    
    /// セキュリティ監査を実行
    pub fn audit_security(&mut self) -> Result<()> {
        let audit_result = audit_package(&self.name, &self.version_req)?;
        self.security_audit = Some(audit_result);
        Ok(())
    }
}

/// プロジェクトの依存関係を追加
pub fn add_dependency(name: &str, version: Option<&str>, dev: bool) -> Result<String> {
    // カレントディレクトリのSwiftLightプロジェクト設定ファイルを読み込む
    let config_path = find_project_config()?;
    let mut config = read_project_config(&config_path)?;
    
    // 使用するバージョン要件の決定（指定がなければ最新バージョン）
    let version_req = match version {
        Some(ver) => ver.to_string(),
        None => {
            // パッケージ情報を取得して最新バージョンを使用
            let pkg_info = get_package_info(name)?;
            format!("^{}", pkg_info.version)
        }
    };
    
    // 依存関係の設定を構築
    let mut dep_config = TomlTable::new();
    dep_config.insert("version".to_string(), TomlValue::String(version_req.clone()));
    
    // 設定ファイルに依存関係を追加
    let section = if dev { "dev-dependencies" } else { "dependencies" };
    
    // 依存関係セクションがなければ作成
    let dependencies = config
        .entry(section.to_string())
        .or_insert(TomlValue::Table(TomlTable::new()));
    
    if let TomlValue::Table(ref mut deps_table) = dependencies {
        deps_table.insert(name.to_string(), TomlValue::Table(dep_config));
    } else {
        return Err(anyhow!("設定ファイルの形式が不正です：{}セクションがテーブルではありません", section));
    }
    
    // 設定ファイルを保存
    write_project_config(&config_path, &config)?;
    
    // ロックファイルを更新
    update_lock_file(&config_path)?;
    
    // セキュリティ監査を実行
    let audit_result = audit_package(name, &version_req)?;
    if let Some(vulnerabilities) = &audit_result.vulnerabilities {
        if !vulnerabilities.is_empty() {
            warn!("セキュリティ脆弱性が検出されました: {}", name);
            for vuln in vulnerabilities {
                warn!("  - {}: {}", vuln.id, vuln.description);
            }
        }
    }
    
    // 戻り値として追加された依存関係の情報を返す
    Ok(format!("{}@{}", name, version_req))
}

/// 依存関係を削除
pub fn remove_dependency(name: &str, dev: bool) -> Result<()> {
    // カレントディレクトリのSwiftLightプロジェクト設定ファイルを読み込む
    let config_path = find_project_config()?;
    let mut config = read_project_config(&config_path)?;
    
    // 削除対象のセクションを決定
    let section = if dev { "dev-dependencies" } else { "dependencies" };
    
    // 依存関係を削除
    let mut removed = false;
    
    if let Some(TomlValue::Table(ref mut deps_table)) = config.get_mut(section) {
        if deps_table.remove(name).is_some() {
            removed = true;
        }
    }
    
    if !removed {
        return Err(anyhow!("依存関係 '{}' が{}セクションに見つかりませんでした", 
            name, if dev { "開発" } else { "" }));
    }
    
    // 設定ファイルを保存
    write_project_config(&config_path, &config)?;
    
    // ロックファイルを更新
    update_lock_file(&config_path)?;
    
    info!("依存関係 '{}' を{}から削除しました", 
        name, if dev { "開発依存" } else { "依存関係" });
    
    Ok(())
}

/// 依存関係を更新
pub fn update_dependency(name: Option<&str>) -> Result<()> {
    // カレントディレクトリのSwiftLightプロジェクト設定ファイルを読み込む
    let config_path = find_project_config()?;
    let config = read_project_config(&config_path)?;
    
    // 更新対象の依存関係を特定
    let mut updated = false;
    
    // 依存関係セクションを抽出
    let sections = ["dependencies", "dev-dependencies"];
    
    for section in &sections {
        if let Some(TomlValue::Table(deps)) = config.get(*section) {
            // 特定のパッケージのみを更新
            if let Some(pkg_name) = name {
                if let Some(_) = deps.get(pkg_name) {
                    update_single_dependency(pkg_name, *section, &config_path)?;
                    updated = true;
                }
            } else {
                // すべての依存関係を更新
                for (pkg_name, _) in deps {
                    update_single_dependency(pkg_name, *section, &config_path)?;
                    updated = true;
                }
            }
        }
    }
    
    if !updated {
        if let Some(pkg_name) = name {
            return Err(anyhow!("依存関係 '{}' が見つかりませんでした", pkg_name));
        } else {
            info!("更新する依存関係がありません");
        }
    }
    
    // ロックファイルを更新
    update_lock_file(&config_path)?;
    
    Ok(())
}

/// 単一の依存関係を更新
fn update_single_dependency(name: &str, section: &str, config_path: &Path) -> Result<()> {
    // 実際の更新処理を実装（最新バージョンの取得など）
    let pkg_info = get_package_info(name)?;
    
    // 設定ファイルを読み込み
    let mut config = read_project_config(config_path)?;
    
    // 依存関係の更新
    if let Some(TomlValue::Table(ref mut section_table)) = config.get_mut(section) {
        if let Some(TomlValue::Table(ref mut dep_config)) = section_table.get_mut(name) {
            // バージョン要件を更新
            let new_version = format!("^{}", pkg_info.version);
            dep_config.insert("version".to_string(), TomlValue::String(new_version.clone()));
            
            // 更新日時を記録
            let now = Utc::now().to_rfc3339();
            dep_config.insert("updated_at".to_string(), TomlValue::String(now));
            
            info!("依存関係 '{}' を {} に更新しました", name, new_version);
            
            // セキュリティ監査を実行
            let audit_result = audit_package(name, &new_version)?;
            if let Some(vulnerabilities) = &audit_result.vulnerabilities {
                if !vulnerabilities.is_empty() {
                    warn!("セキュリティ脆弱性が検出されました: {}", name);
                    for vuln in vulnerabilities {
                        warn!("  - {}: {}", vuln.id, vuln.description);
                    }
                }
            }
        }
    }
    
    // 設定ファイルを保存
    write_project_config(config_path, &config)?;
    
    Ok(())
}

/// 依存関係の一覧を取得
pub fn list_dependencies() -> Result<Vec<(String, String, Option<SecurityAudit>)>> {
    // 依存関係のリストを格納するベクター
    let mut dependencies = Vec::new();
    
    // カレントディレクトリのSwiftLightプロジェクト設定ファイルを読み込む
    let config_path = find_project_config()?;
    let config = read_project_config(&config_path)?;
    
    // 依存関係セクションを抽出
    let sections = ["dependencies", "dev-dependencies"];
    
    for section in &sections {
        if let Some(TomlValue::Table(deps)) = config.get(*section) {
            for (name, config_value) in deps {
                let version = match config_value {
                    TomlValue::String(ver) => ver.clone(),
                    TomlValue::Table(table) => {
                        if let Some(TomlValue::String(ver)) = table.get("version") {
                            ver.clone()
                        } else {
                            "バージョン未指定".to_string()
                        }
                    },
                    _ => "バージョン未指定".to_string(),
                };
                
                let dep_type = if *section == "dev-dependencies" {
                    "(開発用)"
                } else {
                    ""
                };
                
                // セキュリティ監査を実行
                let audit_result = audit_package(name, &version).ok();
                
                dependencies.push((format!("{}{}", name, dep_type), version, audit_result));
            }
        }
    }
    
    Ok(dependencies)
}

/// 依存関係グラフを解決
pub fn resolve_dependencies() -> Result<ResolutionResult> {
    // 解決された依存関係グラフを格納するハッシュマップ
    let mut resolved_packages = HashMap::new();
    let mut dependency_graph = HashMap::new();
    let mut warnings = Vec::new();
    
    // カレントディレクトリのSwiftLightプロジェクト設定ファイルを読み込む
    let config_path = find_project_config()?;
    let config = read_project_config(&config_path)?;
    
    // 依存関係セクションを抽出
    let sections = ["dependencies", "dev-dependencies"];
    
    // ルート依存関係を収集
    let mut root_deps = Vec::new();
    
    for section in &sections {
        if let Some(TomlValue::Table(deps)) = config.get(*section) {
            for (name, _) in deps {
                root_deps.push(name.clone());
            }
        }
    }
    
    // 依存関係キャッシュを初期化
    let mut cache = DependencyCache::new();
    
    // 依存関係の再帰的解決
    for dep_name in root_deps {
        match resolve_dependency_recursive(
            &dep_name, 
            &mut resolved_packages, 
            &mut dependency_graph,
            &mut Vec::new(),
            &mut cache
        ) {
            Ok(_) => {},
            Err(e) => {
                warnings.push(format!("依存関係 '{}' の解決中にエラーが発生しました: {}", dep_name, e));
            }
        }
    }
    
    // ロックファイルを更新
    let lock_entries: Vec<LockEntry> = resolved_packages.iter()
        .map(|(name, pkg)| LockEntry {
            name: name.clone(),
            version: pkg.resolved_version.to_string(),
            checksum: pkg.info.checksum.clone(),
            source: pkg.info.source.clone(),
            dependencies: pkg.info.dependencies.clone(),
        })
        .collect();
    
    let lock_file = LockFile {
        version: "1.0".to_string(),
        packages: lock_entries,
        metadata: HashMap::new(),
    };
    
    let lock_path = config_path.parent().unwrap().join("swiftlight.lock");
    fs::write(&lock_path, toml::to_string_pretty(&lock_file)?)?;
    
    Ok(ResolutionResult {
        packages: resolved_packages,
        warnings,
        dependency_graph,
    })
}

/// 依存関係を再帰的に解決
fn resolve_dependency_recursive(
    name: &str,
    resolved: &mut HashMap<String, ResolvedPackage>,
    dependency_graph: &mut HashMap<String, Vec<String>>,
    dependency_path: &mut Vec<String>,
    cache: &mut DependencyCache,
) -> Result<()> {
    // 循環依存性のチェック
    if dependency_path.contains(&name.to_string()) {
        let cycle = dependency_path.clone();
        cycle.push(name.to_string());
        return Err(anyhow!("循環依存性が検出されました: {}", cycle.join(" -> ")));
    }
    
    // 既に解決済みの場合はスキップ
    if resolved.contains_key(name) {
        return Ok(());
    }
    
    // キャッシュからパッケージ情報を取得、なければレジストリから取得
    let pkg_info = if let Some(cached) = cache.get(name) {
        cached.clone()
    } else {
        let info = get_package_info(name)?;
        cache.insert(name, &info);
        info
    };
    
    // 依存関係パスに現在のパッケージを追加
    dependency_path.push(name.to_string());
    
    // 依存関係グラフにエントリを追加
    let deps = pkg_info.dependencies.clone();
    dependency_graph.insert(name.to_string(), deps.clone());
    
    // パッケージの依存関係を再帰的に解決
    for dep in &deps {
        resolve_dependency_recursive(dep, resolved, dependency_graph, dependency_path, cache)?;
    }
    
    // 依存関係パスから現在のパッケージを削除
    dependency_path.pop();
    
    // セキュリティ監査を実行
    let security_audit = audit_package(name, &pkg_info.version.to_string()).ok();
    
    // 解決済みリストに追加
    let resolved_pkg = ResolvedPackage {
        resolved_version: pkg_info.version.clone(),
        activated_features: pkg_info.features.iter().cloned().collect(),
        dependent_packages: dependency_path.clone(),
        info: pkg_info,
        security_audit,
    };
    
    resolved.insert(name.to_string(), resolved_pkg);
    
    Ok(())
}

/// ロックファイルを更新
fn update_lock_file(config_path: &Path) -> Result<()> {
    // 依存関係を解決
    let resolution = resolve_dependencies()?;
    
    // 警告があれば表示
    for warning in &resolution.warnings {
        warn!("{}", warning);
    }
    
    info!("ロックファイルを更新しました");
    
    Ok(())
}

/// 依存関係の互換性チェック
pub fn check_compatibility() -> Result<Vec<String>> {
    let mut issues = Vec::new();
    
    // 依存関係を解決
    let resolution = resolve_dependencies()?;
    
    // バージョン互換性の問題を検出
    let packages = &resolution.packages;
    let graph = &resolution.dependency_graph;
    
    for (pkg_name, deps) in graph {
        if let Some(pkg) = packages.get(pkg_name) {
            for dep_name in deps {
                if let Some(dep_pkg) = packages.get(dep_name) {
                    // バージョン要件を解析
                    if let Ok(req) = VersionReq::parse(&format!("^{}", pkg.info.version)) {
                        // 依存パッケージのバージョンが要件を満たすか確認
                        if !req.matches(&dep_pkg.resolved_version) {
                            issues.push(format!(
                                "互換性の問題: {} (v{}) は {} の要件 {} を満たしません",
                                dep_name, dep_pkg.resolved_version, pkg_name, req
                            ));
                        }
                    }
                }
            }
        }
    }
    
    Ok(issues)
}

/// 依存関係のセキュリティ監査
pub fn audit_dependencies() -> Result<Vec<VulnerabilityInfo>> {
    let mut vulnerabilities = Vec::new();
    
    // 依存関係の一覧を取得
    let dependencies = list_dependencies()?;
    
    for (name, version, _) in dependencies {
        // パッケージ名から開発依存の表記を削除
        let clean_name = name.replace("(開発用)", "").trim().to_string();
        
        // セキュリティ監査を実行
        match audit_package(&clean_name, &version) {
            Ok(audit) => {
                if let Some(vulns) = audit.vulnerabilities {
                    for vuln in vulns {
                        vulnerabilities.push(vuln);
                    }
                }
            },
            Err(e) => {
                warn!("パッケージ {} の監査中にエラーが発生しました: {}", clean_name, e);
            }
        }
    }
    
    Ok(vulnerabilities)
}

/// プロジェクト設定ファイルを検索
fn find_project_config() -> Result<PathBuf> {
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

/// プロジェクト設定ファイルを書き込む
fn write_project_config(path: &Path, config: &TomlTable) -> Result<()> {
    let content = toml::to_string_pretty(config)
        .context("設定のシリアライズに失敗しました")?;
    
    fs::write(path, content)
        .context(format!("設定ファイルの書き込みに失敗しました: {}", path.display()))?;
    
    Ok(())
}

/// 依存関係のクリーンアップ（未使用の依存関係を削除）
pub fn cleanup_dependencies() -> Result<Vec<String>> {
    let mut removed = Vec::new();
    
    // 依存関係を解決
    let resolution = resolve_dependencies()?;
    
    // 使用されていない依存関係を特定
    let config_path = find_project_config()?;
    let mut config = read_project_config(&config_path)?;
    
    let sections = ["dependencies", "dev-dependencies"];
    
    for section in &sections {
        if let Some(TomlValue::Table(ref mut deps_table)) = config.get_mut(section) {
            let mut to_remove = Vec::new();
            
            for (name, _) in deps_table.iter() {
                // 依存関係グラフに存在しないか、他のパッケージから参照されていない場合
                if !resolution.dependency_graph.contains_key(name) {
                    to_remove.push(name.clone());
                }
            }
            
            for name in &to_remove {
                deps_table.remove(name);
                removed.push(format!("{} ({})", name, if *section == "dev-dependencies" { "開発用" } else { "通常" }));
            }
        }
    }
    
    if !removed.is_empty() {
        // 設定ファイルを保存
        write_project_config(&config_path, &config)?;
        
        // ロックファイルを更新
        update_lock_file(&config_path)?;
    }
    
    Ok(removed)
}

/// 依存関係のバージョン制約を最適化
pub fn optimize_version_constraints() -> Result<Vec<(String, String, String)>> {
    let mut optimized = Vec::new();
    
    // 依存関係を解決
    let resolution = resolve_dependencies()?;
    
    // 設定ファイルを読み込む
    let config_path = find_project_config()?;
    let mut config = read_project_config(&config_path)?;
    
    let sections = ["dependencies", "dev-dependencies"];
    
    for section in &sections {
        if let Some(TomlValue::Table(ref mut deps_table)) = config.get_mut(section) {
            for (name, dep_value) in deps_table.iter_mut() {
                if let Some(pkg) = resolution.packages.get(name) {
                    let current_version = match dep_value {
                        TomlValue::String(ver) => ver.clone(),
                        TomlValue::Table(table) => {
                            if let Some(TomlValue::String(ver)) = table.get("version") {
                                ver.clone()
                            } else {
                                continue;
                            }
                        },
                        _ => continue,
                    };
                    
                    // 最適なバージョン制約を計算
                    let optimal_version = format!("^{}", pkg.resolved_version);
                    
                    if current_version != optimal_version {
                        match dep_value {
                            TomlValue::String(ver) => {
                                *ver = optimal_version.clone();
                            },
                            TomlValue::Table(table) => {
                                if let Some(ver) = table.get_mut("version") {
                                    *ver = TomlValue::String(optimal_version.clone());
                                }
                            },
                            _ => {},
                        }
                        
                        optimized.push((name.clone(), current_version, optimal_version));
                    }
                }
            }
        }
    }
    
    if !optimized.is_empty() {
        // 設定ファイルを保存
        write_project_config(&config_path, &config)?;
        
        // ロックファイルを更新
        update_lock_file(&config_path)?;
    }
    
    Ok(optimized)
}

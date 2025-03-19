use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use anyhow::{Result, anyhow, Context};
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use log::{info, warn, debug};
use chrono::{DateTime, Utc, FixedOffset};

use crate::config::Config;
use crate::manifest::Manifest;
use crate::dependency::{Dependency, DependencyGraph};
use crate::lockfile::Lockfile;

/// オフラインキャッシュのメタデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineCache {
    /// キャッシュのバージョン
    pub version: String,
    /// 最終更新日時
    pub last_updated: String,
    /// キャッシュのルートパス
    pub cache_path: PathBuf,
    /// キャッシュエントリ
    pub entries: HashMap<String, CacheEntry>,
}

/// キャッシュエントリ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// パッケージ名
    pub name: String,
    /// バージョン
    pub version: String,
    /// チェックサム
    pub checksum: String,
    /// キャッシュされたファイルへのパス
    pub file_path: PathBuf,
    /// キャッシュ日時
    pub cached_at: String,
    /// ソース情報
    pub source: String,
    /// 依存関係
    pub dependencies: Vec<String>,
}

/// オフラインモード設定
#[derive(Debug, Clone)]
pub struct OfflineConfig {
    /// オフラインモードが有効かどうか
    pub enabled: bool,
    /// キャッシュディレクトリ
    pub cache_dir: PathBuf,
    /// キャッシュの最大サイズ（MB）
    pub max_cache_size: Option<usize>,
    /// ミラーURLの使用
    pub use_mirrors: bool,
    /// ローカルレジストリの使用
    pub use_local_registry: bool,
}

/// オフラインモード
#[derive(Debug, Clone)]
pub struct OfflineMode {
    /// オフラインモードが有効かどうか
    pub enabled: bool,
    /// キャッシュディレクトリ
    pub cache_dir: PathBuf,
    /// キャッシュ設定
    pub cache: OfflineCache,
}

impl OfflineCache {
    /// 新しいオフラインキャッシュを作成
    pub fn new(cache_path: PathBuf) -> Self {
        OfflineCache {
            version: "1.0".to_string(),
            last_updated: chrono::Utc::now().to_rfc3339(),
            cache_path,
            entries: HashMap::new(),
        }
    }

    /// オフラインキャッシュを読み込む
    pub fn load(path: &Path) -> Result<Self> {
        let meta_path = path.join("cache.json");
        
        if !meta_path.exists() {
            return Ok(OfflineCache::new(path.to_path_buf()));
        }
        
        let mut file = File::open(&meta_path)
            .with_context(|| format!("キャッシュメタデータを開けません: {}", meta_path.display()))?;
        
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .with_context(|| format!("キャッシュメタデータを読み込めません: {}", meta_path.display()))?;
        
        let mut cache: OfflineCache = serde_json::from_str(&contents)
            .with_context(|| format!("キャッシュメタデータのパースに失敗しました: {}", meta_path.display()))?;
        
        // キャッシュパスを更新（ファイルから読み込んだパスは異なる環境である可能性がある）
        cache.cache_path = path.to_path_buf();
        
        Ok(cache)
    }

    /// オフラインキャッシュを保存
    pub fn save(&self) -> Result<()> {
        let meta_path = self.cache_path.join("cache.json");
        
        // ディレクトリを作成
        fs::create_dir_all(&self.cache_path)
            .with_context(|| format!("キャッシュディレクトリを作成できません: {}", self.cache_path.display()))?;
        
        let contents = serde_json::to_string_pretty(&self)
            .with_context(|| "キャッシュメタデータのシリアライズに失敗しました")?;
        
        let mut file = File::create(&meta_path)
            .with_context(|| format!("キャッシュメタデータを作成できません: {}", meta_path.display()))?;
        
        file.write_all(contents.as_bytes())
            .with_context(|| format!("キャッシュメタデータに書き込めません: {}", meta_path.display()))?;
        
        Ok(())
    }

    /// エントリを追加
    pub fn add_entry(&mut self, entry: CacheEntry) {
        let key = format!("{}-{}", entry.name, entry.version);
        self.entries.insert(key, entry);
        self.last_updated = chrono::Utc::now().to_rfc3339();
    }

    /// エントリを取得
    pub fn get_entry(&self, name: &str, version: &str) -> Option<&CacheEntry> {
        let key = format!("{}-{}", name, version);
        self.entries.get(&key)
    }

    /// エントリを削除
    pub fn remove_entry(&mut self, name: &str, version: &str) -> Option<CacheEntry> {
        let key = format!("{}-{}", name, version);
        let entry = self.entries.remove(&key);
        if entry.is_some() {
            self.last_updated = chrono::Utc::now().to_rfc3339();
        }
        entry
    }

    /// キャッシュサイズを取得
    pub fn get_cache_size(&self) -> Result<u64> {
        let mut total_size = 0;
        
        for entry in self.entries.values() {
            let full_path = self.cache_path.join(&entry.file_path);
            if full_path.exists() {
                total_size += fs::metadata(&full_path)?.len();
            }
        }
        
        Ok(total_size)
    }

    /// キャッシュのクリーンアップ
    pub fn cleanup(&mut self, max_size_mb: usize) -> Result<Vec<String>> {
        let max_size: u64 = (max_size_mb as u64) * 1024 * 1024; // MBをバイトに変換
        let mut current_size = self.get_cache_size()?;
        
        // キャッシュサイズが制限内の場合は何もしない
        if current_size <= max_size {
            return Ok(Vec::new());
        }
        
        // 古い順にソート
        let mut entries: Vec<_> = self.entries.iter().collect();
        entries.sort_by(|(_, a), (_, b)| {
            let a_date = chrono::DateTime::parse_from_rfc3339(&a.cached_at)
                .unwrap_or_else(|_| chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap());
            let b_date = chrono::DateTime::parse_from_rfc3339(&b.cached_at)
                .unwrap_or_else(|_| chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00Z").unwrap());
            a_date.cmp(&b_date) // 昇順（古い日付が先）
        });
        
        let mut removed = Vec::new();
        let mut keys_to_remove = Vec::new();
        
        // キャッシュサイズが上限を下回るまで削除
        for (key, entry) in entries {
            if current_size <= max_size {
                break;
            }
            
            // ファイルサイズを取得
            let file_path = &entry.file_path;
            if let Ok(metadata) = fs::metadata(file_path) {
                let file_size = metadata.len();
                
                // ファイルを削除
                if let Err(e) = fs::remove_file(file_path) {
                    eprintln!("キャッシュファイルの削除に失敗しました: {}", e);
                    continue;
                }
                
                // 現在のサイズを更新
                current_size = if current_size >= file_size {
                    current_size - file_size
                } else {
                    0
                };
                
                // エントリをコピーして保存
                let entry_to_remove = entry.clone();
                removed.push(format!("{}-{}", entry_to_remove.name, entry_to_remove.version));
                
                // 削除するキーを保存
                keys_to_remove.push(key.clone());
            }
        }
        
        // エントリを一括削除
        for key in keys_to_remove {
            self.entries.remove(&key);
        }
        
        self.last_updated = chrono::Utc::now().to_rfc3339();
        self.save()?;
        
        Ok(removed)
    }
}

impl OfflineMode {
    /// 新しいオフラインモードを作成
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        let cache = if cache_dir.exists() {
            OfflineCache::load(&cache_dir)?
        } else {
            OfflineCache::new(cache_dir.clone())
        };

        Ok(Self {
            enabled: false,
            cache_dir,
            cache,
        })
    }

    /// オフラインモードを有効化
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// オフラインモードを無効化
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// オフラインモードが有効かどうかを返します
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// キャッシュを更新
    pub fn update_cache(&mut self) -> Result<()> {
        self.cache.save()
    }
}

/// オフラインモードを初期化
pub fn init_offline_mode(config: &Config) -> Result<OfflineConfig> {
    let cache_dir = config.get_cache_dir().join("offline");
    
    let offline_config = OfflineConfig {
        enabled: config.get_bool("offline.enabled").unwrap_or(false),
        cache_dir,
        max_cache_size: config.get_usize("offline.max_cache_size"),
        use_mirrors: config.get_bool("offline.use_mirrors").unwrap_or(true),
        use_local_registry: config.get_bool("offline.use_local_registry").unwrap_or(false),
    };
    
    // キャッシュディレクトリを作成
    fs::create_dir_all(&offline_config.cache_dir)
        .with_context(|| format!("オフラインキャッシュディレクトリを作成できません: {}", offline_config.cache_dir.display()))?;
    
    Ok(offline_config)
}

/// オフラインキャッシュにパッケージを追加
pub fn add_package_to_cache(
    cache: &mut OfflineCache,
    name: &str,
    version: &str,
    package_path: &Path,
    source: &str,
    dependencies: &[String],
) -> Result<()> {
    // ファイルチェックサムを計算
    let mut file = File::open(package_path)
        .with_context(|| format!("パッケージファイルを開けません: {}", package_path.display()))?;
    
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];
    
    loop {
        let bytes_read = file.read(&mut buffer)
            .with_context(|| format!("パッケージファイルの読み込みに失敗しました: {}", package_path.display()))?;
            
        if bytes_read == 0 {
            break;
        }
        
        hasher.update(&buffer[..bytes_read]);
    }
    
    let checksum = format!("{:x}", hasher.finalize());
    
    // キャッシュエントリを作成
    let rel_path = PathBuf::from(format!("packages/{}-{}.tar.gz", name, version));
    let cache_path = cache.cache_path.join(&rel_path);
    
    // ディレクトリを作成
    fs::create_dir_all(cache_path.parent().unwrap())?;
    
    // ファイルをコピー
    fs::copy(package_path, &cache_path)
        .with_context(|| format!("パッケージファイルのコピーに失敗しました: {} -> {}", package_path.display(), cache_path.display()))?;
    
    // エントリを追加
    let entry = CacheEntry {
        name: name.to_string(),
        version: version.to_string(),
        checksum,
        file_path: rel_path,
        cached_at: chrono::Utc::now().to_rfc3339(),
        source: source.to_string(),
        dependencies: dependencies.to_vec(),
    };
    
    cache.add_entry(entry);
    cache.save()?;
    
    Ok(())
}

/// オフラインでの依存関係解決
pub fn resolve_dependencies_offline(
    cache: &OfflineCache,
    dependencies: &[Dependency],
    include_dev: bool,
) -> Result<DependencyGraph> {
    // オンラインのresolve_dependenciesを使用するがオフラインキャッシュを使用
    let graph = crate::dependency::resolve_dependencies(dependencies, include_dev)?;
    
    // 解決された依存関係がすべてキャッシュにあるかチェック
    for (id, dep) in &graph.nodes {
        let name_parts: Vec<_> = id.split('-').collect();
        if name_parts.len() < 2 {
            continue;
        }
        
        let name = name_parts[0];
        let version = name_parts[1..].join("-");
        
        if cache.get_entry(name, &version).is_none() {
            return Err(anyhow!("オフラインモードで依存関係 '{}-{}' が見つかりません", name, version));
        }
    }
    
    Ok(graph)
}

/// オフラインキャッシュからパッケージを取得
pub fn get_package_from_cache(
    cache: &OfflineCache,
    name: &str,
    version: &str,
    output_path: &Path,
) -> Result<PathBuf> {
    let entry = cache.get_entry(name, version)
        .ok_or_else(|| anyhow!("パッケージ '{}-{}' がキャッシュにありません", name, version))?;
    
    let cache_path = cache.cache_path.join(&entry.file_path);
    if !cache_path.exists() {
        return Err(anyhow!("キャッシュされたパッケージファイル '{}' が見つかりません", cache_path.display()));
    }
    
    // 出力ディレクトリを作成
    fs::create_dir_all(output_path.parent().unwrap())?;
    
    // ファイルをコピー
    fs::copy(&cache_path, output_path)
        .with_context(|| format!("パッケージファイルのコピーに失敗しました: {} -> {}", cache_path.display(), output_path.display()))?;
    
    Ok(output_path.to_path_buf())
}

/// オフラインモードの状態を確認
pub fn check_offline_availability(
    cache: &OfflineCache,
    manifest: &Manifest,
    lockfile: Option<&Lockfile>,
) -> Result<(bool, Vec<String>)> {
    let mut available = true;
    let mut missing = Vec::new();
    
    // ロックファイルがあれば、それに基づいてチェック
    if let Some(lock) = lockfile {
        for pkg in lock.get_all_packages() {
            if cache.get_entry(&pkg.name, &pkg.version).is_none() {
                available = false;
                missing.push(format!("{}-{}", pkg.name, pkg.version));
            }
        }
    } else {
        // ロックファイルがなければマニフェストの依存関係をチェック
        // これは厳密には正確でない（バージョン範囲に複数のバージョンがあり得るため）
        let deps = manifest.get_all_dependencies()?;
        
        for dep in deps {
            // 実際の実装では、レジストリから利用可能なバージョンを取得するが、
            // オフラインモードでは難しいため、ここではシンプルにする
            let version = match &dep.version_req {
                Some(v) => v.to_string(),
                None => "最新".to_string(),
            };
            
            // キャッシュにこの名前のパッケージがあるか確認
            let has_package = cache.entries.iter().any(|(key, _)| key.starts_with(&format!("{}-", dep.name)));
            
            if !has_package {
                available = false;
                missing.push(format!("{}-{}", dep.name, version));
            }
        }
    }
    
    Ok((available, missing))
}

/// オフラインモードでプロジェクトをビルド
pub fn build_project_offline(
    project_dir: &Path,
    config: &Config,
    cache: &OfflineCache,
) -> Result<()> {
    // マニフェストを読み込む
    let manifest_path = project_dir.join("swiftlight.toml");
    let manifest = Manifest::load(&manifest_path)?;
    
    // ロックファイルを読み込む
    let lockfile_path = project_dir.join("swiftlight.lock");
    let lockfile = if lockfile_path.exists() {
        Some(Lockfile::load(&lockfile_path)?)
    } else {
        None
    };
    
    // オフラインでビルド可能か確認
    let (available, missing) = check_offline_availability(cache, &manifest, lockfile.as_ref())?;
    if !available {
        return Err(anyhow!("オフラインモードでビルドできません。以下のパッケージがキャッシュにありません: {}", missing.join(", ")));
    }
    
    // 依存関係を解決
    let deps = manifest.get_dependencies()?;
    let graph = resolve_dependencies_offline(cache, &deps, false)?;
    
    // 依存関係を展開
    let deps_dir = project_dir.join("target").join("deps");
    fs::create_dir_all(&deps_dir)?;
    
    for (id, dep) in &graph.nodes {
        let name_parts: Vec<_> = id.split('-').collect();
        if name_parts.len() < 2 {
            continue;
        }
        
        let name = name_parts[0];
        let version = name_parts[1..].join("-");
        
        let output_path = deps_dir.join(format!("{}-{}.tar.gz", name, version));
        get_package_from_cache(cache, name, &version, &output_path)?;
    }
    
    // 実際のビルドプロセスを実行
    let options = crate::build::BuildOptions::default();
    crate::build::build_package(project_dir, &options, config)?;
    
    Ok(())
}

/// キャッシュに保存されたパッケージをソートする（新しいものが優先）
pub fn sort_cached_packages(cache: &mut OfflineCache) {
    // エントリを日付でソート
    let mut entries: Vec<_> = cache.entries.iter().collect();
    entries.sort_by(|(_, a), (_, b)| {
        // RFC3339からDateTimeを解析
        let a_date = match chrono::DateTime::parse_from_rfc3339(&a.cached_at) {
            Ok(dt) => dt,
            Err(_) => return std::cmp::Ordering::Equal, // エラーの場合は等しいとみなす
        };
        
        let b_date = match chrono::DateTime::parse_from_rfc3339(&b.cached_at) {
            Ok(dt) => dt,
            Err(_) => return std::cmp::Ordering::Equal, // エラーの場合は等しいとみなす
        };
        
        b_date.cmp(&a_date) // 降順（新しい日付が先）
    });
    
    // ソートした結果に基づいて新しいHashMapを作成
    let mut sorted_entries = HashMap::new();
    for (key, entry) in entries {
        sorted_entries.insert(key.clone(), entry.clone());
    }
    
    // 古いエントリをソートされた結果で置き換え
    cache.entries = sorted_entries;
} 
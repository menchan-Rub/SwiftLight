use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use anyhow::{Result, anyhow, Context};
use serde::{Serialize, Deserialize};

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

    /// キャッシュのクリーンアップ（指定サイズ以下にする）
    pub fn cleanup(&mut self, max_size_mb: usize) -> Result<Vec<String>> {
        let max_size = max_size_mb * 1024 * 1024;
        let current_size = self.get_cache_size()?;
        
        if current_size <= max_size as u64 {
            return Ok(Vec::new());  // サイズ内なので何もしない
        }
        
        // 日付でソートしたエントリを取得
        let mut entries: Vec<_> = self.entries.iter().collect();
        entries.sort_by(|a, b| {
            let a_date = chrono::DateTime::parse_from_rfc3339(&a.1.cached_at).unwrap_or_else(|_| chrono::DateTime::from_timestamp(0, 0).unwrap());
            let b_date = chrono::DateTime::parse_from_rfc3339(&b.1.cached_at).unwrap_or_else(|_| chrono::DateTime::from_timestamp(0, 0).unwrap());
            a_date.cmp(&b_date)
        });
        
        let mut removed = Vec::new();
        let mut current_size = current_size;
        
        // 古いエントリから削除していく
        for (key, entry) in entries {
            if current_size <= max_size as u64 {
                break;
            }
            
            let full_path = self.cache_path.join(&entry.file_path);
            if full_path.exists() {
                let file_size = fs::metadata(&full_path)?.len();
                fs::remove_file(&full_path)?;
                current_size -= file_size;
                
                self.entries.remove(key);
                removed.push(format!("{}-{}", entry.name, entry.version));
            }
        }
        
        self.last_updated = chrono::Utc::now().to_rfc3339();
        self.save()?;
        
        Ok(removed)
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
    
    let mut hasher = sha2::Sha256::new();
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
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use anyhow::{Result, anyhow, Context};
use semver::Version;
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use toml;

/// パッケージのロック情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedPackage {
    /// パッケージ名
    pub name: String,
    /// バージョン
    pub version: String,
    /// パッケージのチェックサム (SHA-256)
    pub checksum: String,
    /// 直接依存関係かどうか
    pub is_direct: bool,
    /// ソース情報（レジストリURL、Git URL、ローカルパス等）
    pub source: String,
    /// 依存関係
    pub dependencies: Vec<String>,
    /// ビルド依存関係
    pub build_dependencies: Vec<String>,
    /// 開発依存関係
    pub dev_dependencies: Vec<String>,
}

/// ロックファイル構造
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lockfile {
    /// スキーマバージョン
    pub version: String,
    /// 最終更新日時
    pub last_modified: String,
    /// ロックしたパッケージ
    pub packages: HashMap<String, LockedPackage>,
}

impl Lockfile {
    /// 新しいロックファイルを作成
    pub fn new() -> Self {
        Lockfile {
            version: "1.0".to_string(),
            last_modified: chrono::Utc::now().to_rfc3339(),
            packages: HashMap::new(),
        }
    }

    /// ロックファイルを読み込む
    pub fn load(path: &Path) -> Result<Self> {
        let mut file = File::open(path)
            .with_context(|| format!("ロックファイルを開けません: {}", path.display()))?;
        
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .with_context(|| format!("ロックファイルを読み込めません: {}", path.display()))?;
        
        let lockfile: Lockfile = toml::from_str(&contents)
            .with_context(|| format!("ロックファイルのパースに失敗しました: {}", path.display()))?;
        
        Ok(lockfile)
    }

    /// ロックファイルを保存
    pub fn save(&self, path: &Path) -> Result<()> {
        let contents = toml::to_string_pretty(&self)
            .with_context(|| "ロックファイルのシリアライズに失敗しました")?;
        
        let mut file = File::create(path)
            .with_context(|| format!("ロックファイルを作成できません: {}", path.display()))?;
        
        file.write_all(contents.as_bytes())
            .with_context(|| format!("ロックファイルに書き込めません: {}", path.display()))?;
        
        Ok(())
    }

    /// パッケージを追加
    pub fn add_package(&mut self, package: LockedPackage) {
        self.packages.insert(format!("{}-{}", package.name, package.version), package);
        self.last_modified = chrono::Utc::now().to_rfc3339();
    }

    /// パッケージを取得
    pub fn get_package(&self, name: &str, version: &str) -> Option<&LockedPackage> {
        self.packages.get(&format!("{}-{}", name, version))
    }

    /// パッケージの削除
    pub fn remove_package(&mut self, name: &str, version: &str) -> Option<LockedPackage> {
        let key = format!("{}-{}", name, version);
        let package = self.packages.remove(&key);
        if package.is_some() {
            self.last_modified = chrono::Utc::now().to_rfc3339();
        }
        package
    }

    /// 全パッケージの取得
    pub fn get_all_packages(&self) -> Vec<&LockedPackage> {
        self.packages.values().collect()
    }

    /// 直接依存パッケージの取得
    pub fn get_direct_dependencies(&self) -> Vec<&LockedPackage> {
        self.packages.values().filter(|p| p.is_direct).collect()
    }

    /// バージョンの更新
    pub fn update_version(&mut self, name: &str, old_version: &str, new_version: &str, new_checksum: &str) -> Result<()> {
        let old_key = format!("{}-{}", name, old_version);
        let new_key = format!("{}-{}", name, new_version);
        
        if let Some(mut package) = self.packages.remove(&old_key) {
            package.version = new_version.to_string();
            package.checksum = new_checksum.to_string();
            self.packages.insert(new_key, package);
            self.last_modified = chrono::Utc::now().to_rfc3339();
            Ok(())
        } else {
            Err(anyhow!("パッケージが見つかりません: {}-{}", name, old_version))
        }
    }

    /// ロックファイルの検証
    pub fn verify(&self, packages_dir: &Path) -> Result<bool> {
        for package in self.packages.values() {
            let package_path = packages_dir.join(format!("{}-{}.tar.gz", package.name, package.version));
            if !package_path.exists() {
                return Ok(false);
            }

            let checksum = calculate_checksum(&package_path)?;
            if checksum != package.checksum {
                return Ok(false);
            }
        }
        
        Ok(true)
    }
}

/// ファイルのSHA-256チェックサムを計算
fn calculate_checksum(path: &Path) -> Result<String> {
    let mut file = File::open(path)
        .with_context(|| format!("ファイルを開けません: {}", path.display()))?;
    
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];
    
    loop {
        let bytes_read = file.read(&mut buffer)
            .with_context(|| format!("ファイルの読み込みに失敗しました: {}", path.display()))?;
            
        if bytes_read == 0 {
            break;
        }
        
        hasher.update(&buffer[..bytes_read]);
    }
    
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// ロックファイルの存在確認
pub fn lockfile_exists(project_dir: &Path) -> bool {
    let lockfile_path = project_dir.join("swiftlight.lock");
    lockfile_path.exists()
}

/// ロックファイルの作成
pub fn create_lockfile(project_dir: &Path) -> Result<Lockfile> {
    let lockfile = Lockfile::new();
    let lockfile_path = project_dir.join("swiftlight.lock");
    lockfile.save(&lockfile_path)?;
    Ok(lockfile)
}

/// ロックファイルの読み込み
pub fn load_lockfile(project_dir: &Path) -> Result<Lockfile> {
    let lockfile_path = project_dir.join("swiftlight.lock");
    Lockfile::load(&lockfile_path)
}

/// ロックファイルのマージ
pub fn merge_lockfiles(base: &Lockfile, other: &Lockfile) -> Lockfile {
    let mut merged = base.clone();
    
    for (key, package) in &other.packages {
        if !merged.packages.contains_key(key) {
            merged.packages.insert(key.clone(), package.clone());
        }
    }
    
    merged.last_modified = chrono::Utc::now().to_rfc3339();
    merged
}

/// ロックファイルの差分を取得
pub fn diff_lockfiles(base: &Lockfile, other: &Lockfile) -> (Vec<LockedPackage>, Vec<LockedPackage>, Vec<(LockedPackage, LockedPackage)>) {
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut modified = Vec::new();
    
    // 追加されたパッケージを検出
    for (key, package) in &other.packages {
        if !base.packages.contains_key(key) {
            added.push(package.clone());
        }
    }
    
    // 削除されたパッケージを検出
    for (key, package) in &base.packages {
        if !other.packages.contains_key(key) {
            removed.push(package.clone());
        }
    }
    
    // 変更されたパッケージを検出
    for (key, base_package) in &base.packages {
        if let Some(other_package) = other.packages.get(key) {
            if base_package.checksum != other_package.checksum {
                modified.push((base_package.clone(), other_package.clone()));
            }
        }
    }
    
    (added, removed, modified)
} 
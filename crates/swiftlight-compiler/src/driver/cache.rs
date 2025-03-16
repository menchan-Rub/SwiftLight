//! コンパイルキャッシュモジュール
//!
//! コンパイル結果をキャッシュして再利用するためのモジュールです。

use std::path::{Path, PathBuf};
use std::fs;
use std::io::{self, Read, Write};
use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use std::sync::{Arc, Mutex, RwLock};
use std::hash::{Hash, Hasher};

/// キャッシュエントリのメタデータ
#[derive(Debug, Clone)]
pub struct CacheEntryMetadata {
    /// エントリの作成時間
    pub created_at: SystemTime,
    /// 最終アクセス時間
    pub last_accessed: SystemTime,
    /// アクセス回数
    pub access_count: u64,
    /// 対応するソースファイルの最終変更時間
    pub source_modified_at: SystemTime,
    /// エントリのファイルサイズ
    pub size_bytes: u64,
    /// コンパイルにかかった時間
    pub compile_time: Duration,
    /// 依存ファイルとそのハッシュ
    pub dependencies: HashMap<PathBuf, String>,
}

/// キャッシュエントリ
#[derive(Debug)]
pub struct CacheEntry {
    /// メタデータ
    pub metadata: CacheEntryMetadata,
    /// キャッシュされたデータのパス
    pub data_path: PathBuf,
    /// エントリのキー（通常はファイルのハッシュ）
    pub key: String,
    /// エントリの種類
    pub entry_type: CacheEntryType,
}

/// キャッシュエントリの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheEntryType {
    /// 字句解析結果
    LexerOutput,
    /// 構文解析結果
    ASTOutput,
    /// IR出力
    IROutput,
    /// オブジェクトファイル
    ObjectFile,
    /// 依存関係情報
    DependencyInfo,
    /// マクロ展開結果
    MacroExpansion,
    /// 型情報
    TypeInfo,
}

impl CacheEntryType {
    /// ファイル拡張子を取得
    pub fn extension(&self) -> &'static str {
        match self {
            CacheEntryType::LexerOutput => "lex",
            CacheEntryType::ASTOutput => "ast",
            CacheEntryType::IROutput => "ir",
            CacheEntryType::ObjectFile => "o",
            CacheEntryType::DependencyInfo => "dep",
            CacheEntryType::MacroExpansion => "macro",
            CacheEntryType::TypeInfo => "types",
        }
    }
    
    /// ディレクトリ名を取得
    pub fn directory(&self) -> &'static str {
        match self {
            CacheEntryType::LexerOutput => "lexer",
            CacheEntryType::ASTOutput => "ast",
            CacheEntryType::IROutput => "ir",
            CacheEntryType::ObjectFile => "obj",
            CacheEntryType::DependencyInfo => "deps",
            CacheEntryType::MacroExpansion => "macros",
            CacheEntryType::TypeInfo => "types",
        }
    }
}

/// コンパイルキャッシュ
pub struct CompilationCache {
    /// キャッシュのルートディレクトリ
    root_dir: PathBuf,
    /// キャッシュエントリのインデックス
    index: RwLock<HashMap<String, CacheEntry>>,
    /// キャッシュサイズの上限（バイト単位）
    max_size: u64,
    /// 現在のキャッシュサイズ（バイト単位）
    current_size: Arc<Mutex<u64>>,
    /// キャッシュヒット数
    hits: Arc<Mutex<u64>>,
    /// キャッシュミス数
    misses: Arc<Mutex<u64>>,
    /// キャッシュの有効期限（秒）
    expiration_time: u64,
    /// キャッシュが有効かどうか
    enabled: bool,
}

impl CompilationCache {
    /// 新しいコンパイルキャッシュを作成
    pub fn new<P: AsRef<Path>>(root_dir: P, max_size: u64, expiration_time: u64) -> io::Result<Self> {
        let root_dir = root_dir.as_ref().to_path_buf();
        
        // キャッシュディレクトリを作成
        if !root_dir.exists() {
            fs::create_dir_all(&root_dir)?;
        }
        
        // 各エントリタイプのディレクトリを作成
        for entry_type in &[
            CacheEntryType::LexerOutput,
            CacheEntryType::ASTOutput,
            CacheEntryType::IROutput,
            CacheEntryType::ObjectFile,
            CacheEntryType::DependencyInfo,
            CacheEntryType::MacroExpansion,
            CacheEntryType::TypeInfo,
        ] {
            let type_dir = root_dir.join(entry_type.directory());
            if !type_dir.exists() {
                fs::create_dir_all(&type_dir)?;
            }
        }
        
        // インデックスを読み込み
        let index = RwLock::new(HashMap::new());
        let current_size = Arc::new(Mutex::new(0));
        
        Ok(Self {
            root_dir,
            index,
            max_size,
            current_size,
            hits: Arc::new(Mutex::new(0)),
            misses: Arc::new(Mutex::new(0)),
            expiration_time,
            enabled: true,
        })
    }
    
    /// キャッシュから項目を取得
    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str, entry_type: CacheEntryType) -> Option<T> {
        if !self.enabled {
            return None;
        }
        
        let index = self.index.read().unwrap();
        let entry = index.get(key)?;
        
        // エントリが古くなっていないか確認
        let now = SystemTime::now();
        let created_duration = now.duration_since(entry.metadata.created_at).ok()?;
        if created_duration.as_secs() > self.expiration_time {
            return None;
        }
        
        // 依存関係が変更されていないか確認
        for (path, hash) in &entry.metadata.dependencies {
            if let Ok(metadata) = fs::metadata(path) {
                if let Ok(modified) = metadata.modified() {
                    if modified > entry.metadata.source_modified_at {
                        return None;
                    }
                }
            } else {
                return None;  // 依存ファイルが存在しない
            }
        }
        
        // キャッシュファイルを読み込み
        let file_path = self.get_cache_file_path(key, entry_type);
        let file = fs::File::open(file_path).ok()?;
        let reader = io::BufReader::new(file);
        
        // データをデシリアライズ
        let data: T = bincode::deserialize_from(reader).ok()?;
        
        // ヒット数を更新
        let mut hits = self.hits.lock().unwrap();
        *hits += 1;
        
        // メタデータを更新
        drop(index);
        let mut index = self.index.write().unwrap();
        if let Some(entry) = index.get_mut(key) {
            entry.metadata.last_accessed = SystemTime::now();
            entry.metadata.access_count += 1;
        }
        
        Some(data)
    }
    
    /// キャッシュに項目を格納
    pub fn put<T: serde::ser::Serialize>(
        &self,
        key: &str,
        data: &T,
        entry_type: CacheEntryType,
        source_path: &Path,
        compile_time: Duration,
        dependencies: HashMap<PathBuf, String>,
    ) -> io::Result<()> {
        if !self.enabled {
            return Ok(());
        }
        
        // 容量チェック
        let data_size = bincode::serialized_size(data).unwrap_or(0);
        if data_size > self.max_size {
            return Ok(());  // キャッシュサイズより大きいデータは保存しない
        }
        
        // キャッシュクリーンアップ
        self.ensure_capacity(data_size)?;
        
        // キャッシュファイルを書き込み
        let file_path = self.get_cache_file_path(key, entry_type);
        let file = fs::File::create(&file_path)?;
        let writer = io::BufWriter::new(file);
        bincode::serialize_into(writer, data).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        
        // メタデータを作成
        let source_modified_at = fs::metadata(source_path)?.modified()?;
        let metadata = CacheEntryMetadata {
            created_at: SystemTime::now(),
            last_accessed: SystemTime::now(),
            access_count: 0,
            source_modified_at,
            size_bytes: data_size,
            compile_time,
            dependencies,
        };
        
        // インデックスを更新
        let entry = CacheEntry {
            metadata,
            data_path: file_path,
            key: key.to_string(),
            entry_type,
        };
        
        let mut index = self.index.write().unwrap();
        index.insert(key.to_string(), entry);
        
        // 現在のキャッシュサイズを更新
        let mut current_size = self.current_size.lock().unwrap();
        *current_size += data_size;
        
        // ミス数を更新
        let mut misses = self.misses.lock().unwrap();
        *misses += 1;
        
        Ok(())
    }
    
    /// キャッシュから項目を削除
    pub fn remove(&self, key: &str) -> io::Result<()> {
        let mut index = self.index.write().unwrap();
        if let Some(entry) = index.remove(key) {
            // ファイルを削除
            if entry.data_path.exists() {
                fs::remove_file(&entry.data_path)?;
            }
            
            // サイズを更新
            let mut current_size = self.current_size.lock().unwrap();
            *current_size = current_size.saturating_sub(entry.metadata.size_bytes);
        }
        
        Ok(())
    }
    
    /// キャッシュを空にする
    pub fn clear(&self) -> io::Result<()> {
        // すべてのファイルを削除
        for entry_type in &[
            CacheEntryType::LexerOutput,
            CacheEntryType::ASTOutput,
            CacheEntryType::IROutput,
            CacheEntryType::ObjectFile,
            CacheEntryType::DependencyInfo,
            CacheEntryType::MacroExpansion,
            CacheEntryType::TypeInfo,
        ] {
            let type_dir = self.root_dir.join(entry_type.directory());
            if type_dir.exists() {
                for entry in fs::read_dir(&type_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_file() {
                        fs::remove_file(path)?;
                    }
                }
            }
        }
        
        // インデックスをクリア
        let mut index = self.index.write().unwrap();
        index.clear();
        
        // サイズをリセット
        let mut current_size = self.current_size.lock().unwrap();
        *current_size = 0;
        
        Ok(())
    }
    
    /// キャッシュファイルのパスを取得
    fn get_cache_file_path(&self, key: &str, entry_type: CacheEntryType) -> PathBuf {
        let dir = self.root_dir.join(entry_type.directory());
        dir.join(format!("{}.{}", key, entry_type.extension()))
    }
    
    /// キャッシュの容量を確保（必要に応じて古いエントリを削除）
    fn ensure_capacity(&self, required_size: u64) -> io::Result<()> {
        let current_size = *self.current_size.lock().unwrap();
        if current_size + required_size <= self.max_size {
            return Ok(());
        }
        
        // 削除する必要のあるサイズ
        let to_remove = (current_size + required_size) - self.max_size;
        
        // 最終アクセス時間の古い順にエントリをソート
        let mut index = self.index.write().unwrap();
        let mut entries: Vec<_> = index.values().collect();
        entries.sort_by(|a, b| a.metadata.last_accessed.cmp(&b.metadata.last_accessed));
        
        // 空き容量ができるまで古いエントリを削除
        let mut removed_size = 0;
        let mut removed_keys = Vec::new();
        
        for entry in entries {
            if removed_size >= to_remove {
                break;
            }
            
            // ファイルを削除
            if entry.data_path.exists() {
                fs::remove_file(&entry.data_path)?;
            }
            
            removed_size += entry.metadata.size_bytes;
            removed_keys.push(entry.key.clone());
        }
        
        // インデックスから削除したエントリを削除
        for key in removed_keys {
            index.remove(&key);
        }
        
        // サイズを更新
        let mut current_size = self.current_size.lock().unwrap();
        *current_size = current_size.saturating_sub(removed_size);
        
        Ok(())
    }
    
    /// キャッシュのヒット数を取得
    pub fn hit_count(&self) -> u64 {
        *self.hits.lock().unwrap()
    }
    
    /// キャッシュのミス数を取得
    pub fn miss_count(&self) -> u64 {
        *self.misses.lock().unwrap()
    }
    
    /// キャッシュのヒット率を取得
    pub fn hit_rate(&self) -> f64 {
        let hits = *self.hits.lock().unwrap();
        let misses = *self.misses.lock().unwrap();
        let total = hits + misses;
        
        if total == 0 {
            return 0.0;
        }
        
        hits as f64 / total as f64
    }
    
    /// キャッシュの現在のサイズを取得
    pub fn size(&self) -> u64 {
        *self.current_size.lock().unwrap()
    }
    
    /// キャッシュの使用率を取得
    pub fn usage_ratio(&self) -> f64 {
        let current_size = *self.current_size.lock().unwrap();
        current_size as f64 / self.max_size as f64
    }
    
    /// キャッシュの有効/無効を設定
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    
    /// キャッシュが有効かどうか
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// キャッシュキーの生成
pub fn generate_cache_key<T: Hash>(value: &T) -> String {
    use std::collections::hash_map::DefaultHasher;
    
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
} 
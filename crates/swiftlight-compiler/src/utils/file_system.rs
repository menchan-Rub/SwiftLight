// ファイルシステム操作を行うモジュール
// ファイルの読み書き、監視、仮想ファイルシステムなどを提供します

use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::collections::HashMap;
use std::time::{SystemTime, Duration};
use std::sync::{Arc, Mutex};

/// ファイル変更イベント
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChangeEvent {
    /// ファイルが作成された
    Created(PathBuf),
    /// ファイルが変更された
    Modified(PathBuf),
    /// ファイルが削除された
    Deleted(PathBuf),
}

/// ファイル変更監視
#[derive(Debug)]
pub struct FileWatcher {
    /// 監視対象のパス
    watched_paths: Vec<PathBuf>,
    /// ファイルの最終変更時間
    last_modified: HashMap<PathBuf, SystemTime>,
    /// 監視間隔（ミリ秒）
    poll_interval: Duration,
}

impl FileWatcher {
    /// 新しいファイル監視を作成
    pub fn new() -> Self {
        Self {
            watched_paths: Vec::new(),
            last_modified: HashMap::new(),
            poll_interval: Duration::from_millis(500),
        }
    }
    
    /// 監視対象のパスを追加
    pub fn watch<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let path = path.as_ref().to_path_buf();
        
        if path.exists() {
            let metadata = fs::metadata(&path)?;
            let modified = metadata.modified()?;
            
            self.last_modified.insert(path.clone(), modified);
            self.watched_paths.push(path);
        }
        
        Ok(())
    }
    
    /// 監視間隔を設定
    pub fn set_poll_interval(&mut self, interval_ms: u64) {
        self.poll_interval = Duration::from_millis(interval_ms);
    }
    
    /// 変更を検出
    pub fn check_for_changes(&mut self) -> io::Result<Vec<FileChangeEvent>> {
        let mut events = Vec::new();
        
        for path in &self.watched_paths {
            if !path.exists() {
                if self.last_modified.remove(path).is_some() {
                    events.push(FileChangeEvent::Deleted(path.clone()));
                }
                continue;
            }
            
            let metadata = fs::metadata(path)?;
            let modified = metadata.modified()?;
            
            if let Some(last_modified) = self.last_modified.get(path) {
                if modified > *last_modified {
                    events.push(FileChangeEvent::Modified(path.clone()));
                    self.last_modified.insert(path.clone(), modified);
                }
            } else {
                events.push(FileChangeEvent::Created(path.clone()));
                self.last_modified.insert(path.clone(), modified);
            }
        }
        
        Ok(events)
    }
    
    /// 監視を中止
    pub fn stop_watching<P: AsRef<Path>>(&mut self, path: P) {
        let path = path.as_ref().to_path_buf();
        self.watched_paths.retain(|p| p != &path);
        self.last_modified.remove(&path);
    }
    
    /// 全ての監視を中止
    pub fn stop_all(&mut self) {
        self.watched_paths.clear();
        self.last_modified.clear();
    }
}

/// 仮想ファイルシステム
/// メモリ上にファイルを保持し、実際のファイルシステムを使わずに読み書きできます
#[derive(Debug, Default)]
pub struct VirtualFileSystem {
    /// 仮想ファイルの内容
    files: HashMap<PathBuf, Vec<u8>>,
}

impl VirtualFileSystem {
    /// 新しい仮想ファイルシステムを作成
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }
    
    /// ファイルが存在するかどうかを確認
    pub fn exists<P: AsRef<Path>>(&self, path: P) -> bool {
        self.files.contains_key(&path.as_ref().to_path_buf())
    }
    
    /// ファイルを読み込み
    pub fn read<P: AsRef<Path>>(&self, path: P) -> io::Result<Vec<u8>> {
        let path = path.as_ref().to_path_buf();
        
        if let Some(content) = self.files.get(&path) {
            Ok(content.clone())
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, format!("仮想ファイルが見つかりません: {}", path.display())))
        }
    }
    
    /// ファイルを文字列として読み込み
    pub fn read_to_string<P: AsRef<Path>>(&self, path: P) -> io::Result<String> {
        let bytes = self.read(path)?;
        String::from_utf8(bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
    
    /// ファイルに書き込み
    pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(&mut self, path: P, contents: C) -> io::Result<()> {
        let path = path.as_ref().to_path_buf();
        
        // 親ディレクトリが存在することを確認
        if let Some(parent) = path.parent() {
            self.create_dir_all(parent)?;
        }
        
        self.files.insert(path, contents.as_ref().to_vec());
        Ok(())
    }
    
    /// ディレクトリを作成
    pub fn create_dir<P: AsRef<Path>>(&mut self, _path: P) -> io::Result<()> {
        // 仮想ファイルシステムでは特に何もしない
        Ok(())
    }
    
    /// ディレクトリを再帰的に作成
    pub fn create_dir_all<P: AsRef<Path>>(&mut self, _path: P) -> io::Result<()> {
        // 仮想ファイルシステムでは特に何もしない
        Ok(())
    }
    
    /// ファイルを削除
    pub fn remove_file<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let path = path.as_ref().to_path_buf();
        
        if self.files.remove(&path).is_none() {
            return Err(io::Error::new(io::ErrorKind::NotFound, format!("仮想ファイルが見つかりません: {}", path.display())));
        }
        
        Ok(())
    }
    
    /// ディレクトリを削除
    pub fn remove_dir<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let path = path.as_ref().to_path_buf();
        
        // このディレクトリ以下のファイルをすべて削除
        let prefix = format!("{}/", path.display());
        let to_remove: Vec<PathBuf> = self.files.keys()
            .filter(|p| p.starts_with(&path) || p.to_string_lossy().starts_with(&prefix))
            .cloned()
            .collect();
            
        for p in to_remove {
            self.files.remove(&p);
        }
        
        Ok(())
    }
    
    /// 実際のファイルシステムからファイルを読み込んで仮想ファイルシステムに追加
    pub fn import_from_real<P: AsRef<Path>, Q: AsRef<Path>>(&mut self, real_path: P, virtual_path: Q) -> io::Result<()> {
        let content = fs::read(real_path)?;
        self.write(virtual_path, content)
    }
    
    /// 仮想ファイルシステムのファイルを実際のファイルシステムに書き出し
    pub fn export_to_real<P: AsRef<Path>, Q: AsRef<Path>>(&self, virtual_path: P, real_path: Q) -> io::Result<()> {
        let content = self.read(virtual_path)?;
        
        // 親ディレクトリが存在することを確認
        if let Some(parent) = real_path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::write(real_path, content)
    }
    
    /// 全てのファイルをクリア
    pub fn clear(&mut self) {
        self.files.clear();
    }
    
    /// 仮想ファイルシステム内のファイル一覧を取得
    pub fn list_files(&self) -> Vec<PathBuf> {
        self.files.keys().cloned().collect()
    }
}

/// ファイルシステム操作ユーティリティ
#[derive(Debug, Default)]
pub struct FileSystem {
    /// 仮想ファイルシステム
    virtual_fs: Option<Arc<Mutex<VirtualFileSystem>>>,
}

impl FileSystem {
    /// 新しいファイルシステムユーティリティを作成
    pub fn new() -> Self {
        Self {
            virtual_fs: None,
        }
    }
    
    /// 仮想ファイルシステムを設定
    pub fn with_virtual_fs(virtual_fs: Arc<Mutex<VirtualFileSystem>>) -> Self {
        Self {
            virtual_fs: Some(virtual_fs),
        }
    }
    
    /// ファイルを読み込み
    pub fn read<P: AsRef<Path>>(&self, path: P) -> io::Result<Vec<u8>> {
        let path = path.as_ref();
        
        // 仮想ファイルシステムをチェック
        if let Some(vfs) = &self.virtual_fs {
            let vfs = vfs.lock().unwrap();
            if vfs.exists(path) {
                return vfs.read(path);
            }
        }
        
        // 実際のファイルシステムから読み込み
        fs::read(path)
    }
    
    /// ファイルを文字列として読み込み
    pub fn read_to_string<P: AsRef<Path>>(&self, path: P) -> io::Result<String> {
        let path = path.as_ref();
        
        // 仮想ファイルシステムをチェック
        if let Some(vfs) = &self.virtual_fs {
            let vfs = vfs.lock().unwrap();
            if vfs.exists(path) {
                return vfs.read_to_string(path);
            }
        }
        
        // 実際のファイルシステムから読み込み
        fs::read_to_string(path)
    }
    
    /// ファイルに書き込み
    pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(&self, path: P, contents: C) -> io::Result<()> {
        let path = path.as_ref();
        
        // 仮想ファイルシステムへの書き込み
        if let Some(vfs) = &self.virtual_fs {
            let mut vfs = vfs.lock().unwrap();
            vfs.write(path, contents.as_ref())?;
            return Ok(());
        }
        
        // 実際のファイルシステムに書き込み
        // 親ディレクトリが存在することを確認
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::write(path, contents)
    }
    
    /// ファイルをコピー
    pub fn copy<P: AsRef<Path>, Q: AsRef<Path>>(&self, from: P, to: Q) -> io::Result<u64> {
        let from = from.as_ref();
        let to = to.as_ref();
        
        // 仮想ファイルシステムの場合
        if let Some(vfs) = &self.virtual_fs {
            let mut vfs = vfs.lock().unwrap();
            
            if vfs.exists(from) {
                let content = vfs.read(from)?;
                vfs.write(to, &content)?;
                return Ok(content.len() as u64);
            }
        }
        
        // 実際のファイルシステムの場合
        // 親ディレクトリが存在することを確認
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::copy(from, to)
    }
    
    /// ディレクトリを作成
    pub fn create_dir<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let path = path.as_ref();
        
        // 仮想ファイルシステムの場合
        if let Some(vfs) = &self.virtual_fs {
            let mut vfs = vfs.lock().unwrap();
            return vfs.create_dir(path);
        }
        
        // 実際のファイルシステムの場合
        fs::create_dir(path)
    }
    
    /// ディレクトリを再帰的に作成
    pub fn create_dir_all<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let path = path.as_ref();
        
        // 仮想ファイルシステムの場合
        if let Some(vfs) = &self.virtual_fs {
            let mut vfs = vfs.lock().unwrap();
            return vfs.create_dir_all(path);
        }
        
        // 実際のファイルシステムの場合
        fs::create_dir_all(path)
    }
    
    /// ファイルを削除
    pub fn remove_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let path = path.as_ref();
        
        // 仮想ファイルシステムの場合
        if let Some(vfs) = &self.virtual_fs {
            let mut vfs = vfs.lock().unwrap();
            if vfs.exists(path) {
                return vfs.remove_file(path);
            }
        }
        
        // 実際のファイルシステムの場合
        fs::remove_file(path)
    }
    
    /// ディレクトリを削除
    pub fn remove_dir<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let path = path.as_ref();
        
        // 仮想ファイルシステムの場合
        if let Some(vfs) = &self.virtual_fs {
            let mut vfs = vfs.lock().unwrap();
            return vfs.remove_dir(path);
        }
        
        // 実際のファイルシステムの場合
        fs::remove_dir(path)
    }
    
    /// ディレクトリを再帰的に削除
    pub fn remove_dir_all<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let path = path.as_ref();
        
        // 仮想ファイルシステムの場合
        if let Some(vfs) = &self.virtual_fs {
            let mut vfs = vfs.lock().unwrap();
            return vfs.remove_dir(path);
        }
        
        // 実際のファイルシステムの場合
        fs::remove_dir_all(path)
    }
    
    /// ファイルの存在確認
    pub fn exists<P: AsRef<Path>>(&self, path: P) -> bool {
        let path = path.as_ref();
        
        // 仮想ファイルシステムの場合
        if let Some(vfs) = &self.virtual_fs {
            let vfs = vfs.lock().unwrap();
            if vfs.exists(path) {
                return true;
            }
        }
        
        // 実際のファイルシステムの場合
        path.exists()
    }
    
    /// ファイルがディレクトリかどうかを確認
    pub fn is_dir<P: AsRef<Path>>(&self, path: P) -> bool {
        let path = path.as_ref();
        
        // 仮想ファイルシステムは未実装なのでスキップ
        
        // 実際のファイルシステムの場合
        path.is_dir()
    }
    
    /// ファイルが通常ファイルかどうかを確認
    pub fn is_file<P: AsRef<Path>>(&self, path: P) -> bool {
        let path = path.as_ref();
        
        // 仮想ファイルシステムの場合
        if let Some(vfs) = &self.virtual_fs {
            let vfs = vfs.lock().unwrap();
            if vfs.exists(path) {
                return true; // 仮想ファイルシステムでは全て通常ファイルとして扱う
            }
        }
        
        // 実際のファイルシステムの場合
        path.is_file()
    }
} 
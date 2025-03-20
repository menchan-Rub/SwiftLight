// ハッシュ計算を行うモジュール
// 様々なハッシュアルゴリズムを提供します

use std::path::{Path, PathBuf};
use std::fs;
use std::io::{self, Read};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// ハッシュアルゴリズムの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    /// Rustのデフォルトハッシュ関数
    Default,
    /// FNV-1a
    Fnv1a,
    /// SHA-256
    Sha256,
    /// xxHash
    XxHash,
}

/// コンテンツハッシャー
#[derive(Debug, Clone)]
pub struct ContentHasher {
    /// ハッシュアルゴリズム
    algorithm: HashAlgorithm,
}

impl Default for ContentHasher {
    fn default() -> Self {
        Self::new(HashAlgorithm::Default)
    }
}

impl ContentHasher {
    /// 新しいコンテンツハッシャーを作成
    pub fn new(algorithm: HashAlgorithm) -> Self {
        Self {
            algorithm,
        }
    }
    
    /// ファイルの内容からハッシュ値を計算
    pub fn hash_file<P: AsRef<Path>>(&self, path: P) -> io::Result<String> {
        let path = path.as_ref();
        let mut file = fs::File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        
        Ok(self.hash_bytes(&buffer))
    }
    
    /// バイト列からハッシュ値を計算
    pub fn hash_bytes(&self, bytes: &[u8]) -> String {
        match self.algorithm {
            HashAlgorithm::Default => {
                let mut hasher = DefaultHasher::new();
                bytes.hash(&mut hasher);
                format!("{:016x}", hasher.finish())
            },
            HashAlgorithm::Fnv1a => {
                // FNV-1aハッシュの実装（簡易版）
                const FNV_PRIME: u64 = 1099511628211;
                const FNV_OFFSET_BASIS: u64 = 14695981039346656037;
                
                let mut hash = FNV_OFFSET_BASIS;
                for byte in bytes {
                    hash ^= *byte as u64;
                    hash = hash.wrapping_mul(FNV_PRIME);
                }
                
                format!("{:016x}", hash)
            },
            HashAlgorithm::Sha256 => {
                // 実際のコードではsha2クレートなどを使用
                format!("sha256_{}", self.hash_bytes_default(bytes))
            },
            HashAlgorithm::XxHash => {
                // 実際のコードではtwoxクレートなどを使用
                format!("xxhash_{}", self.hash_bytes_default(bytes))
            },
        }
    }
    
    /// 文字列からハッシュ値を計算
    pub fn hash_string(&self, s: &str) -> String {
        self.hash_bytes(s.as_bytes())
    }
    
    /// デフォルトのハッシュ関数でバイト列からハッシュ値を計算
    fn hash_bytes_default(&self, bytes: &[u8]) -> String {
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
    
    /// ディレクトリ内の全ファイルの内容を結合したハッシュ値を計算
    pub fn hash_directory<P: AsRef<Path>>(&self, dir: P) -> io::Result<String> {
        let dir = dir.as_ref();
        let mut combined_hash = String::new();
        
        if !dir.is_dir() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "指定されたパスはディレクトリではありません"));
        }
        
        let mut paths: Vec<_> = fs::read_dir(dir)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect();
            
        // パスをソートして再現性を確保
        paths.sort();
        
        for path in paths {
            if path.is_file() {
                let file_hash = self.hash_file(&path)?;
                combined_hash.push_str(&file_hash);
            } else if path.is_dir() {
                let subdir_hash = self.hash_directory(&path)?;
                combined_hash.push_str(&subdir_hash);
            }
        }
        
        Ok(self.hash_string(&combined_hash))
    }
} 
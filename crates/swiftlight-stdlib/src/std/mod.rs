//! # SwiftLight言語の標準モジュール
//! 
//! このモジュールは標準ライブラリの基本的な機能を提供します。
//! 主に、coreモジュールからの再エクスポートと、標準ライブラリ固有の機能を提供します。

// coreモジュールの全ての機能を再エクスポート
pub use crate::core::*;

/// エントリーポイント関数
/// 
/// SwiftLight言語のプログラムのメインエントリーポイントを定義します。
pub mod main {
    use crate::core::types::{Error, Result};

    /// メイン関数を実行
    pub fn run<F>(main_fn: F) -> Result<()>
    where
        F: FnOnce() -> Result<()>,
    {
        // メイン関数を実行し、結果を返す
        main_fn()
    }

    /// 引数付きのメイン関数を実行
    pub fn run_with_args<F>(main_fn: F, args: Vec<String>) -> Result<()>
    where
        F: FnOnce(Vec<String>) -> Result<()>,
    {
        // 引数付きでメイン関数を実行し、結果を返す
        main_fn(args)
    }
}

/// 環境変数アクセス
pub mod env {
    use crate::core::types::{Error, ErrorKind, Result};
    use crate::core::collections::HashMap;
    use std::env as StdEnv;

    /// 環境変数を取得
    pub fn get(key: &str) -> Result<String> {
        StdEnv::var(key).map_err(|e| {
            Error::new(
                ErrorKind::EnvError, 
                format!("環境変数'{}' の取得に失敗しました: {}", key, e)
            )
        })
    }

    /// 環境変数を設定
    pub fn set(key: &str, value: &str) -> Result<()> {
        StdEnv::set_var(key, value);
        Ok(())
    }

    /// 環境変数を削除
    pub fn remove(key: &str) {
        StdEnv::remove_var(key);
    }

    /// 全ての環境変数を取得
    pub fn vars() -> HashMap<String, String> {
        let std_vars = StdEnv::vars().collect::<std::collections::HashMap<String, String>>();
        let mut result = HashMap::new();
        for (key, value) in std_vars {
            result.insert(key, value);
        }
        result
    }

    /// カレントディレクトリを取得
    pub fn current_dir() -> Result<String> {
        StdEnv::current_dir()
            .map_err(|e| {
                Error::new(
                    ErrorKind::IOError, 
                    format!("カレントディレクトリの取得に失敗しました: {}", e)
                )
            })
            .map(|path| path.to_string_lossy().to_string())
    }

    /// カレントディレクトリを変更
    pub fn set_current_dir(path: &str) -> Result<()> {
        StdEnv::set_current_dir(path).map_err(|e| {
            Error::new(
                ErrorKind::IOError, 
                format!("カレントディレクトリの変更に失敗しました: {}", e)
            )
        })
    }
}

/// プロセス関連機能
pub mod process {
    use crate::core::types::{Error, ErrorKind, Result};
    
    /// 現在のプロセスを終了
    pub fn exit(code: i32) -> ! {
        std::process::exit(code)
    }
    
    /// スリープ（ミリ秒）
    pub fn sleep(milliseconds: u64) {
        std::thread::sleep(std::time::Duration::from_millis(milliseconds));
    }
    
    /// コマンドを実行
    pub fn command(cmd: &str, args: &[&str]) -> Result<(i32, String, String)> {
        let output = std::process::Command::new(cmd)
            .args(args)
            .output()
            .map_err(|e| {
                Error::new(
                    ErrorKind::ProcessError, 
                    format!("コマンド実行に失敗しました: {}", e)
                )
            })?;
        
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let status = output.status.code().unwrap_or(-1);
        
        Ok((status, stdout, stderr))
    }
    
    /// プロセスIDを取得
    pub fn pid() -> u32 {
        std::process::id()
    }
}

/// 時間関連機能
pub mod time {
    use crate::core::types::{Error, ErrorKind, Result};
    
    /// 時間の計測構造体
    pub struct Instant {
        inner: std::time::Instant,
    }
    
    impl Instant {
        /// 新しいインスタンスを作成
        pub fn now() -> Self {
            Self {
                inner: std::time::Instant::now(),
            }
        }
        
        /// 経過時間をミリ秒で取得
        pub fn elapsed_ms(&self) -> u64 {
            let elapsed = self.inner.elapsed();
            elapsed.as_secs() * 1000 + elapsed.subsec_millis() as u64
        }
        
        /// 経過時間をマイクロ秒で取得
        pub fn elapsed_us(&self) -> u64 {
            let elapsed = self.inner.elapsed();
            elapsed.as_secs() * 1_000_000 + elapsed.subsec_micros() as u64
        }
        
        /// 経過時間をナノ秒で取得
        pub fn elapsed_ns(&self) -> u64 {
            let elapsed = self.inner.elapsed();
            elapsed.as_secs() * 1_000_000_000 + elapsed.subsec_nanos() as u64
        }
    }
    
    /// 現在のUNIXタイムスタンプを取得（秒）
    pub fn unix_timestamp() -> Result<u64> {
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| {
                Error::new(
                    ErrorKind::TimeError, 
                    format!("システム時刻の取得に失敗しました: {}", e)
                )
            })?;
        
        Ok(time.as_secs())
    }
    
    /// 現在のUNIXタイムスタンプをミリ秒で取得
    pub fn unix_timestamp_ms() -> Result<u128> {
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| {
                Error::new(
                    ErrorKind::TimeError, 
                    format!("システム時刻の取得に失敗しました: {}", e)
                )
            })?;
        
        Ok(time.as_millis())
    }
}

/// ファイルシステム操作
pub mod fs {
    use crate::core::types::{Error, ErrorKind, Result};
    use crate::core::collections::Vec;
    use std::fs as StdFs;
    use std::io::{Read, Write};
    use std::path::Path;
    
    /// ファイルの内容を読み込む
    pub fn read_to_string(path: &str) -> Result<String> {
        StdFs::read_to_string(path).map_err(|e| {
            Error::new(
                ErrorKind::IOError, 
                format!("ファイルの読み込みに失敗しました: {}", e)
            )
        })
    }
    
    /// ファイルをバイト配列として読み込む
    pub fn read(path: &str) -> Result<Vec<u8>> {
        let mut file = StdFs::File::open(path).map_err(|e| {
            Error::new(
                ErrorKind::IOError, 
                format!("ファイルのオープンに失敗しました: {}", e)
            )
        })?;
        
        let mut std_buffer = std::vec::Vec::new();
        file.read_to_end(&mut std_buffer).map_err(|e| {
            Error::new(
                ErrorKind::IOError, 
                format!("ファイルの読み込みに失敗しました: {}", e)
            )
        })?;
        
        Ok(Vec::from(std_buffer))
    }
    
    /// ファイルに文字列を書き込む
    pub fn write(path: &str, contents: &str) -> Result<()> {
        StdFs::write(path, contents).map_err(|e| {
            Error::new(
                ErrorKind::IOError, 
                format!("ファイルの書き込みに失敗しました: {}", e)
            )
        })
    }
    
    /// ファイルにバイト配列を書き込む
    pub fn write_bytes(path: &str, contents: &[u8]) -> Result<()> {
        StdFs::write(path, contents).map_err(|e| {
            Error::new(
                ErrorKind::IOError, 
                format!("ファイルの書き込みに失敗しました: {}", e)
            )
        })
    }
    
    /// ファイルの存在確認
    pub fn exists(path: &str) -> bool {
        Path::new(path).exists()
    }
    
    /// ディレクトリを作成
    pub fn create_dir(path: &str) -> Result<()> {
        StdFs::create_dir(path).map_err(|e| {
            Error::new(
                ErrorKind::IOError, 
                format!("ディレクトリの作成に失敗しました: {}", e)
            )
        })
    }
    
    /// ディレクトリを再帰的に作成
    pub fn create_dir_all(path: &str) -> Result<()> {
        StdFs::create_dir_all(path).map_err(|e| {
            Error::new(
                ErrorKind::IOError, 
                format!("ディレクトリの作成に失敗しました: {}", e)
            )
        })
    }
    
    /// ファイルまたはディレクトリを削除
    pub fn remove(path: &str) -> Result<()> {
        let path_obj = Path::new(path);
        
        if path_obj.is_dir() {
            StdFs::remove_dir(path).map_err(|e| {
                Error::new(
                    ErrorKind::IOError, 
                    format!("ディレクトリの削除に失敗しました: {}", e)
                )
            })
        } else {
            StdFs::remove_file(path).map_err(|e| {
                Error::new(
                    ErrorKind::IOError, 
                    format!("ファイルの削除に失敗しました: {}", e)
                )
            })
        }
    }
    
    /// ディレクトリの内容を列挙
    pub fn read_dir(path: &str) -> Result<Vec<String>> {
        let entries = StdFs::read_dir(path).map_err(|e| {
            Error::new(
                ErrorKind::IOError, 
                format!("ディレクトリの読み込みに失敗しました: {}", e)
            )
        })?;
        
        let mut result = Vec::new();
        
        for entry in entries {
            let entry = entry.map_err(|e| {
                Error::new(
                    ErrorKind::IOError, 
                    format!("ディレクトリエントリの読み込みに失敗しました: {}", e)
                )
            })?;
            
            let path_str = entry.path().to_string_lossy().to_string();
            result.push(path_str);
        }
        
        Ok(result)
    }
} 
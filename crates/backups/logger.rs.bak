// ロギング機能を提供するモジュール
// コンパイラのプロセスやステップを詳細に記録するためのロガーを実装します

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};
use std::fmt;

/// ログレベル
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// トレース情報 (最も詳細)
    Trace,
    /// デバッグ情報
    Debug,
    /// 情報
    Info,
    /// 警告
    Warning,
    /// エラー
    Error,
    /// 致命的エラー (最も重大)
    Fatal,
}

impl LogLevel {
    /// ログレベルの文字列表現を取得
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warning => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Fatal => "FATAL",
        }
    }
    
    /// 文字列からログレベルを解析
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "TRACE" => Some(LogLevel::Trace),
            "DEBUG" => Some(LogLevel::Debug),
            "INFO" => Some(LogLevel::Info),
            "WARN" | "WARNING" => Some(LogLevel::Warning),
            "ERROR" => Some(LogLevel::Error),
            "FATAL" => Some(LogLevel::Fatal),
            _ => None,
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// ログエントリ
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// タイムスタンプ
    pub timestamp: SystemTime,
    /// ログレベル
    pub level: LogLevel,
    /// モジュール名
    pub module: String,
    /// メッセージ
    pub message: String,
}

impl LogEntry {
    /// 新しいログエントリを作成
    pub fn new<S: Into<String>>(level: LogLevel, module: S, message: S) -> Self {
        Self {
            timestamp: SystemTime::now(),
            level,
            module: module.into(),
            message: message.into(),
        }
    }
    
    /// ログエントリを文字列にフォーマット
    pub fn format(&self) -> String {
        let now = self.timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0))
            .as_secs();
            
        format!(
            "[{:010}] [{:5}] [{}] {}",
            now,
            self.level,
            self.module,
            self.message
        )
    }
}

impl fmt::Display for LogEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format())
    }
}

/// ログの出力先
#[derive(Debug, Clone)]
pub enum LogTarget {
    /// 標準出力
    Stdout,
    /// 標準エラー出力
    Stderr,
    /// ファイル
    File(PathBuf),
}

/// ロガーインターフェース
pub trait Logger: Send + Sync {
    /// ログメッセージを記録
    fn log(&self, entry: LogEntry);
    
    /// トレースメッセージを記録
    fn trace<M: Into<String>, S: Into<String>>(&self, module: M, message: S) {
        self.log(LogEntry::new(LogLevel::Trace, module, message));
    }
    
    /// デバッグメッセージを記録
    fn debug<M: Into<String>, S: Into<String>>(&self, module: M, message: S) {
        self.log(LogEntry::new(LogLevel::Debug, module, message));
    }
    
    /// 情報メッセージを記録
    fn info<M: Into<String>, S: Into<String>>(&self, module: M, message: S) {
        self.log(LogEntry::new(LogLevel::Info, module, message));
    }
    
    /// 警告メッセージを記録
    fn warning<M: Into<String>, S: Into<String>>(&self, module: M, message: S) {
        self.log(LogEntry::new(LogLevel::Warning, module, message));
    }
    
    /// エラーメッセージを記録
    fn error<M: Into<String>, S: Into<String>>(&self, module: M, message: S) {
        self.log(LogEntry::new(LogLevel::Error, module, message));
    }
    
    /// 致命的エラーメッセージを記録
    fn fatal<M: Into<String>, S: Into<String>>(&self, module: M, message: S) {
        self.log(LogEntry::new(LogLevel::Fatal, module, message));
    }
    
    /// パフォーマンスメトリックを記録
    fn performance<M: Into<String>, S: Into<String>>(&self, module: M, operation: S, duration: Duration) {
        let message = format!("Performance: {} took {:?}", operation.into(), duration);
        self.info(module, message);
    }
}

/// 複合ロガー（複数のロガーに出力）
#[derive(Default)]
pub struct CompositeLogger {
    loggers: Vec<Box<dyn Logger>>,
}

impl CompositeLogger {
    /// 新しい複合ロガーを作成
    pub fn new() -> Self {
        Self {
            loggers: Vec::new(),
        }
    }
    
    /// ロガーを追加
    pub fn add_logger<L: Logger + 'static>(&mut self, logger: L) {
        self.loggers.push(Box::new(logger));
    }
}

impl Logger for CompositeLogger {
    fn log(&self, entry: LogEntry) {
        for logger in &self.loggers {
            logger.log(entry.clone());
        }
    }
}

/// コンソールロガーの実装
pub struct ConsoleLogger {
    /// 最小ログレベル
    min_level: LogLevel,
    /// 出力先
    target: LogTarget,
}

impl ConsoleLogger {
    /// 新しいコンソールロガーを作成
    pub fn new(min_level: LogLevel, target: LogTarget) -> Self {
        Self {
            min_level,
            target,
        }
    }
    
    /// 標準出力に書き込むロガーを作成
    pub fn stdout(min_level: LogLevel) -> Self {
        Self::new(min_level, LogTarget::Stdout)
    }
    
    /// 標準エラー出力に書き込むロガーを作成
    pub fn stderr(min_level: LogLevel) -> Self {
        Self::new(min_level, LogTarget::Stderr)
    }
}

impl Logger for ConsoleLogger {
    fn log(&self, entry: LogEntry) {
        if entry.level < self.min_level {
            return;
        }
        
        let message = format!("{}\n", entry);
        match &self.target {
            LogTarget::Stdout => {
                let _ = io::stdout().write_all(message.as_bytes());
                let _ = io::stdout().flush();
            }
            LogTarget::Stderr => {
                let _ = io::stderr().write_all(message.as_bytes());
                let _ = io::stderr().flush();
            }
            LogTarget::File(path) => {
                if let Ok(mut file) = OpenOptions::new()
                    .write(true)
                    .append(true)
                    .create(true)
                    .open(path)
                {
                    let _ = file.write_all(message.as_bytes());
                    let _ = file.flush();
                }
            }
        }
    }
}

/// ファイルロガーの実装
pub struct FileLogger {
    /// 最小ログレベル
    min_level: LogLevel,
    /// ファイルパス
    file_path: PathBuf,
    /// ファイルハンドル
    file: Mutex<Option<File>>,
}

impl FileLogger {
    /// 新しいファイルロガーを作成
    pub fn new<P: AsRef<Path>>(min_level: LogLevel, file_path: P) -> io::Result<Self> {
        let file_path = file_path.as_ref().to_path_buf();
        
        // 親ディレクトリが存在しない場合は作成
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let file = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(&file_path)?;
            
        Ok(Self {
            min_level,
            file_path,
            file: Mutex::new(Some(file)),
        })
    }
}

impl Logger for FileLogger {
    fn log(&self, entry: LogEntry) {
        if entry.level < self.min_level {
            return;
        }
        
        let message = format!("{}\n", entry);
        let mut file_guard = match self.file.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        
        if let Some(file) = &mut *file_guard {
            let _ = file.write_all(message.as_bytes());
            let _ = file.flush();
        } else {
            // ファイルが閉じられている場合は再度開く
            if let Ok(new_file) = OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open(&self.file_path)
            {
                *file_guard = Some(new_file);
                if let Some(file) = &mut *file_guard {
                    let _ = file.write_all(message.as_bytes());
                    let _ = file.flush();
                }
            }
        }
    }
}

/// パフォーマンスモニター
pub struct PerformanceMonitor {
    /// ロガー
    logger: Arc<dyn Logger>,
    /// モジュール名
    module: String,
    /// 操作名
    operation: String,
    /// 開始時刻
    start_time: Instant,
}

impl PerformanceMonitor {
    /// 新しいパフォーマンスモニターを作成
    pub fn new<M: Into<String>, O: Into<String>>(
        logger: Arc<dyn Logger>,
        module: M,
        operation: O,
    ) -> Self {
        Self {
            logger,
            module: module.into(),
            operation: operation.into(),
            start_time: Instant::now(),
        }
    }
    
    /// 経過時間を報告
    pub fn report_elapsed(&self) {
        let elapsed = self.start_time.elapsed();
        self.logger.performance(&self.module, &self.operation, elapsed);
    }
}

impl Drop for PerformanceMonitor {
    fn drop(&mut self) {
        self.report_elapsed();
    }
} 
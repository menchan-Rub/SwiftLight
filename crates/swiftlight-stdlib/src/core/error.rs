//! # SwiftLight言語のエラー処理モジュール
//! 
//! このモジュールはSwiftLight言語のエラー処理機能を提供します。
//! エラーの型、エラーハンドリング機能、およびエラー関連のユーティリティが含まれています。

use std::fmt;
use std::error::Error as StdError;
use std::sync::Arc;
use std::panic::{self, Location};
use std::backtrace::Backtrace;
use std::io;
use std::convert::From;

use crate::core::types;

// types::Errorとtypes::ErrorKindの再エクスポート
pub use crate::core::types::{Error, ErrorKind};

/// 結果型のエイリアス
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// エラー情報を含むコンテキスト
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// エラーが発生したファイル名
    pub file: String,
    
    /// エラーが発生した行番号
    pub line: u32,
    
    /// エラーが発生した列番号
    pub column: u32,
    
    /// エラーが発生した関数名
    pub function: String,
    
    /// エラーに関連する詳細情報
    pub details: String,
}

impl ErrorContext {
    /// 新しいエラーコンテキストを作成
    pub fn new(
        file: impl Into<String>,
        line: u32,
        column: u32,
        function: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self {
            file: file.into(),
            line,
            column,
            function: function.into(),
            details: details.into(),
        }
    }
    
    /// 現在の位置情報からエラーコンテキストを作成
    pub fn current(details: impl Into<String>) -> Self {
        let location = Location::caller();
        Self {
            file: location.file().to_string(),
            line: location.line(),
            column: 0, // Rustは列情報を提供していないため0を設定
            function: "<unknown>".to_string(),
            details: details.into(),
        }
    }
}

/// スタックトレース情報
#[derive(Debug, Clone)]
pub struct StackTrace {
    frames: Vec<StackFrame>,
}

/// スタックフレーム情報
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// 関数名
    pub function: String,
    
    /// ファイル名
    pub file: String,
    
    /// 行番号
    pub line: u32,
}

impl StackTrace {
    /// 現在のスタックトレースを取得
    pub fn current() -> Self {
        let backtrace = Backtrace::capture();
        let mut frames = Vec::new();
        
        // バックトレースからスタックフレームを構築する
        // 注：これは単純化されたバージョンで、実際にはbacktraceクレートの機能を
        // より詳細に利用する必要があります
        frames.push(StackFrame {
            function: "current_function".to_string(),
            file: Location::caller().file().to_string(),
            line: Location::caller().line(),
        });
        
        Self { frames }
    }
    
    /// スタックトレースの文字列表現を取得
    pub fn to_string(&self) -> String {
        let mut result = String::new();
        
        for (i, frame) in self.frames.iter().enumerate() {
            result.push_str(&format!(
                "#{}: {} at {}:{}\n",
                i,
                frame.function,
                frame.file,
                frame.line
            ));
        }
        
        result
    }
}

/// エラーを表すトレイト
pub trait ErrorTrait: StdError + Send + Sync + 'static {
    /// エラーの種類を取得
    fn kind(&self) -> ErrorKind;
    
    /// エラーのコンテキストを取得
    fn context(&self) -> Option<&ErrorContext>;
    
    /// エラーを別のエラーにラップする
    fn wrap<E: ErrorTrait>(self, error: E) -> Error;
    
    /// スタックトレースを取得
    fn stack_trace(&self) -> Option<&StackTrace>;
}

impl ErrorTrait for Error {
    fn kind(&self) -> ErrorKind {
        self.kind()
    }
    
    fn context(&self) -> Option<&ErrorContext> {
        None
    }
    
    fn wrap<E: ErrorTrait>(self, error: E) -> Error {
        Error::new(
            self.kind(),
            format!("{}: {}", self.message(), error)
        )
    }
    
    fn stack_trace(&self) -> Option<&StackTrace> {
        None
    }
}

/// エラーを安全に処理する関数
pub fn try_catch<F, R, E>(f: F) -> Result<R, Error>
where
    F: FnOnce() -> Result<R, E>,
    E: Into<Error>,
{
    match f() {
        Ok(result) => Ok(result),
        Err(error) => Err(error.into()),
    }
}

/// パニックを捕捉してエラーに変換する関数
pub fn catch_panic<F, R>(f: F) -> Result<R, Error>
where
    F: FnOnce() -> R + panic::UnwindSafe,
{
    match panic::catch_unwind(f) {
        Ok(result) => Ok(result),
        Err(_) => Err(Error::new(
            ErrorKind::RuntimeError,
            "パニックが発生しました"
        )),
    }
}

/// Resultをラップして追加情報を付与する
pub fn with_context<T, E, F>(result: Result<T, E>, f: F) -> Result<T, Error>
where
    E: Into<Error>,
    F: FnOnce() -> String,
{
    match result {
        Ok(value) => Ok(value),
        Err(error) => {
            let error = error.into();
            Err(Error::new(
                error.kind(),
                format!("{}: {}", f(), error.message())
            ))
        }
    }
}

// 標準的なエラー型からのFrom実装
impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::new(
            ErrorKind::IOError,
            format!("IO Error: {}", error)
        )
    }
}

impl From<std::fmt::Error> for Error {
    fn from(_: std::fmt::Error) -> Self {
        Error::new(
            ErrorKind::RuntimeError,
            "フォーマットエラーが発生しました"
        )
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(error: std::str::Utf8Error) -> Self {
        Error::new(
            ErrorKind::ValueError,
            format!("UTF-8変換エラー: {}", error)
        )
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(error: std::num::ParseIntError) -> Self {
        Error::new(
            ErrorKind::ValueError,
            format!("整数解析エラー: {}", error)
        )
    }
}

impl From<std::num::ParseFloatError> for Error {
    fn from(error: std::num::ParseFloatError) -> Self {
        Error::new(
            ErrorKind::ValueError,
            format!("浮動小数点数解析エラー: {}", error)
        )
    }
}

/// エラーハンドリングのユーティリティ関数
pub mod util {
    use super::*;
    
    /// エラーの詳細情報を取得
    pub fn error_details(error: &Error) -> String {
        format!(
            "エラー: {:?}\nメッセージ: {}\n",
            error.kind(),
            error.message()
        )
    }
    
    /// 致命的なエラーを処理する関数
    pub fn handle_fatal_error(error: Error) -> ! {
        eprintln!("致命的なエラーが発生しました:");
        eprintln!("{}", error_details(&error));
        std::process::exit(1);
    }
}

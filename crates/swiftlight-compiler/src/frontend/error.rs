//! # SwiftLight コンパイラエラー処理
//! 
//! コンパイラのエラー処理機能を提供します。エラーの種類、位置情報、
//! 詳細なメッセージなどを含むエラー型を定義します。

use std::fmt;
use std::error::Error;
use std::result;

use crate::frontend::diagnostic::{Diagnostic, DiagnosticLevel};

/// SwiftLightコンパイラの結果型
pub type Result<T> = result::Result<T, CompilerError>;

/// エラーの種類を表す列挙型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /// 字句解析エラー
    Lexical,
    /// 構文解析エラー
    Syntax,
    /// 意味解析エラー
    Semantic,
    /// 型検査エラー
    Type,
    /// シンボル解決エラー
    SymbolResolution,
    /// コード生成エラー
    CodeGeneration,
    /// 最適化エラー
    Optimization,
    /// I/Oエラー
    IO,
    /// その他のエラー
    Other,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Lexical => write!(f, "字句解析エラー"),
            ErrorKind::Syntax => write!(f, "構文解析エラー"),
            ErrorKind::Semantic => write!(f, "意味解析エラー"),
            ErrorKind::Type => write!(f, "型検査エラー"),
            ErrorKind::SymbolResolution => write!(f, "シンボル解決エラー"),
            ErrorKind::CodeGeneration => write!(f, "コード生成エラー"),
            ErrorKind::Optimization => write!(f, "最適化エラー"),
            ErrorKind::IO => write!(f, "I/Oエラー"),
            ErrorKind::Other => write!(f, "その他のエラー"),
        }
    }
}

/// ソースコード内の位置情報
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    /// ファイル名
    pub file_name: String,
    /// 行番号（1から始まる）
    pub line: usize,
    /// 列番号（1から始まる）
    pub column: usize,
    /// 開始位置（バイトオフセット）
    pub start: usize,
    /// 終了位置（バイトオフセット）
    pub end: usize,
}

impl SourceLocation {
    /// 新しい位置情報を作成
    pub fn new(file_name: impl Into<String>, line: usize, column: usize, start: usize, end: usize) -> Self {
        Self {
            file_name: file_name.into(),
            line,
            column,
            start,
            end,
        }
    }
    
    /// 位置情報が未知であることを示す特殊な値
    pub fn unknown() -> Self {
        Self {
            file_name: "<unknown>".to_string(),
            line: 0,
            column: 0,
            start: 0,
            end: 0,
        }
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file_name, self.line, self.column)
    }
}

/// コンパイラエラー型
#[derive(Debug, Clone)]
pub struct CompilerError {
    /// エラーの種類
    pub kind: ErrorKind,
    /// エラーメッセージ
    pub message: String,
    /// ソースコード内の位置情報（オプション）
    pub location: Option<SourceLocation>,
    /// 原因となったエラー（オプション）
    pub cause: Option<Box<CompilerError>>,
    /// 関連する診断情報
    pub diagnostics: Vec<Diagnostic>,
}

impl CompilerError {
    /// 新しいコンパイラエラーを作成
    pub fn new(kind: ErrorKind, message: impl Into<String>, location: Option<SourceLocation>) -> Self {
        Self {
            kind,
            message: message.into(),
            location,
            cause: None,
            diagnostics: Vec::new(),
        }
    }
    
    /// 原因エラーを設定
    pub fn with_cause(mut self, cause: CompilerError) -> Self {
        self.cause = Some(Box::new(cause));
        self
    }
    
    /// 診断情報を追加
    pub fn add_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }
    
    /// 診断情報を設定して自身を返す
    pub fn with_diagnostic(mut self, diagnostic: Diagnostic) -> Self {
        self.add_diagnostic(diagnostic);
        self
    }
    
    /// 字句解析エラーを作成するヘルパーメソッド
    pub fn lexical_error(message: impl Into<String>, location: Option<SourceLocation>) -> Self {
        Self::new(ErrorKind::Lexical, message, location)
    }
    
    /// 構文解析エラーを作成するヘルパーメソッド
    pub fn syntax_error(message: impl Into<String>, location: Option<SourceLocation>) -> Self {
        Self::new(ErrorKind::Syntax, message, location)
    }
    
    /// 意味解析エラーを作成するヘルパーメソッド
    pub fn semantic_error(message: impl Into<String>, location: Option<SourceLocation>) -> Self {
        Self::new(ErrorKind::Semantic, message, location)
    }
    
    /// 型エラーを作成するヘルパーメソッド
    pub fn type_error(message: impl Into<String>, location: Option<SourceLocation>) -> Self {
        Self::new(ErrorKind::Type, message, location)
    }
}

impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref loc) = self.location {
            write!(f, "{} at {}: {}", self.kind, loc, self.message)?;
        } else {
            write!(f, "{}: {}", self.kind, self.message)?;
        }
        
        // 原因エラーがある場合は表示
        if let Some(ref cause) = self.cause {
            write!(f, "\n原因: {}", cause)?;
        }
        
        Ok(())
    }
}

impl Error for CompilerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause.as_ref().map(|e| e.as_ref() as &(dyn Error + 'static))
    }
}

/// テスト
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_creation() {
        let location = SourceLocation::new("test.swl", 10, 5, 100, 105);
        let error = CompilerError::syntax_error("不正な式です", Some(location.clone()));
        
        assert_eq!(error.kind, ErrorKind::Syntax);
        assert_eq!(error.message, "不正な式です");
        assert_eq!(error.location, Some(location));
    }
    
    #[test]
    fn test_error_with_cause() {
        let cause = CompilerError::lexical_error("不正な文字です", None);
        let error = CompilerError::syntax_error("式の解析に失敗しました", None)
            .with_cause(cause);
        
        assert!(error.cause.is_some());
        assert_eq!(error.cause.unwrap().kind, ErrorKind::Lexical);
    }
    
    #[test]
    fn test_error_display() {
        let location = SourceLocation::new("test.swl", 10, 5, 100, 105);
        let error = CompilerError::syntax_error("不正な式です", Some(location));
        
        assert_eq!(format!("{}", error), "構文解析エラー at test.swl:10:5: 不正な式です");
    }
} 
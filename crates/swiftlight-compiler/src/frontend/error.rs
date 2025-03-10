//! # エラー処理モジュール
//! 
//! SwiftLightコンパイラのエラー処理に関連する型定義を提供します。
//! ソースコードの位置情報や、コンパイル時のエラーメッセージを管理します。

use std::fmt;
use std::path::Path;
use std::result;
use std::error::Error as StdError;

/// コンパイラの結果型
pub type Result<T> = result::Result<T, CompilerError>;

/// ソースコード内の位置
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    /// ファイル名
    pub file_name: String,
    /// 行番号 (1から始まる)
    pub line: usize,
    /// 列番号 (1から始まる)
    pub column: usize,
    /// オフセット位置
    pub offset: usize,
    /// 領域の長さ
    pub length: usize,
}

impl SourceLocation {
    /// 新しい位置情報を作成
    pub fn new(file_name: &str, line: usize, column: usize, offset: usize, length: usize) -> Self {
        Self {
            file_name: file_name.to_string(),
            line,
            column,
            offset,
            length,
        }
    }
    
    /// ファイルパスから位置情報を作成
    pub fn from_path(path: &Path, line: usize, column: usize, offset: usize, length: usize) -> Self {
        Self {
            file_name: path.to_string_lossy().to_string(),
            line,
            column,
            offset,
            length,
        }
    }
    
    /// 単一の位置情報を作成
    pub fn at_point(file_name: &str, line: usize, column: usize, offset: usize) -> Self {
        Self::new(file_name, line, column, offset, 0)
    }
    
    /// 隣接する2つの位置情報を結合
    pub fn merge(&self, other: &SourceLocation) -> Self {
        assert_eq!(self.file_name, other.file_name, "異なるファイルの位置情報は結合できません");
        
        let start_offset = self.offset.min(other.offset);
        let end_offset = (self.offset + self.length).max(other.offset + other.length);
        
        let (start_line, start_column) = 
            if self.offset <= other.offset {
                (self.line, self.column)
            } else {
                (other.line, other.column)
            };
        
        Self {
            file_name: self.file_name.clone(),
            line: start_line,
            column: start_column,
            offset: start_offset,
            length: end_offset - start_offset,
        }
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file_name, self.line, self.column)
    }
}

/// エラーの種類
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /// 字句解析エラー
    Lexical,
    /// 構文解析エラー
    Syntax,
    /// 名前解決エラー
    NameResolution,
    /// 型チェックエラー
    Type,
    /// コード生成エラー
    CodeGeneration,
    /// 入出力エラー
    IO,
    /// その他の内部エラー
    Internal,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::Lexical => write!(f, "字句解析エラー"),
            ErrorKind::Syntax => write!(f, "構文解析エラー"),
            ErrorKind::NameResolution => write!(f, "名前解決エラー"),
            ErrorKind::Type => write!(f, "型エラー"),
            ErrorKind::CodeGeneration => write!(f, "コード生成エラー"),
            ErrorKind::IO => write!(f, "入出力エラー"),
            ErrorKind::Internal => write!(f, "内部エラー"),
        }
    }
}

/// コンパイラエラー
#[derive(Debug, Clone)]
pub struct CompilerError {
    /// エラーの種類
    pub kind: ErrorKind,
    /// エラーメッセージ
    pub message: String,
    /// エラーの発生位置
    pub location: Option<SourceLocation>,
    /// 詳細情報（追加メッセージなど）
    pub details: Option<String>,
    /// 関連するエラー（派生エラーやヒントなど）
    pub related: Vec<RelatedError>,
}

impl CompilerError {
    /// 新しいコンパイラエラーを作成
    pub fn new(kind: ErrorKind, message: String, location: Option<SourceLocation>, 
              details: Option<String>) -> Self {
        Self {
            kind,
            message,
            location,
            details,
            related: Vec::new(),
        }
    }
    
    /// 字句解析エラーを作成
    pub fn lexical_error(message: impl Into<String>, location: SourceLocation) -> Self {
        Self::new(
            ErrorKind::Lexical,
            message.into(),
            Some(location),
            None,
        )
    }
    
    /// 構文解析エラーを作成
    pub fn syntax_error(message: impl Into<String>, location: SourceLocation) -> Self {
        Self::new(
            ErrorKind::Syntax,
            message.into(),
            Some(location),
            None,
        )
    }
    
    /// 名前解決エラーを作成
    pub fn name_resolution_error(message: impl Into<String>, location: Option<SourceLocation>) -> Self {
        Self::new(
            ErrorKind::NameResolution,
            message.into(),
            location,
            None,
        )
    }
    
    /// 型チェックエラーを作成
    pub fn type_error(message: impl Into<String>, location: Option<SourceLocation>) -> Self {
        Self::new(
            ErrorKind::Type,
            message.into(),
            location,
            None,
        )
    }
    
    /// コード生成エラーを作成
    pub fn code_generation_error(message: impl Into<String>, location: Option<SourceLocation>) -> Self {
        Self::new(
            ErrorKind::CodeGeneration,
            message.into(),
            location,
            None,
        )
    }
    
    /// 入出力エラーを作成
    pub fn io_error(message: impl Into<String>) -> Self {
        Self::new(
            ErrorKind::IO,
            message.into(),
            None,
            None,
        )
    }
    
    /// 内部エラーを作成
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(
            ErrorKind::Internal,
            message.into(),
            None,
            None,
        )
    }
    
    /// 警告を作成
    pub fn warning(message: impl Into<String>, location: Option<SourceLocation>) -> Self {
        let mut error = Self::new(
            ErrorKind::Internal, // 仮の種類
            message.into(),
            location,
            None,
        );
        error.related.push(RelatedError::Warning {
            message: error.message.clone(),
            location: error.location.clone(),
        });
        error
    }
    
    /// 詳細情報を追加
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
    
    /// ヒント（提案）を追加
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.related.push(RelatedError::Hint {
            message: hint.into(),
            location: None,
        });
        self
    }
    
    /// 関連エラーを追加
    pub fn with_related(mut self, related: RelatedError) -> Self {
        self.related.push(related);
        self
    }
    
    /// 型エラーに対する型情報を追加
    pub fn with_type_info(mut self, expected: &str, found: &str) -> Self {
        self.details = Some(format!("期待された型: {}, 実際の型: {}", expected, found));
        self
    }
}

impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // エラーの基本情報
        if let Some(location) = &self.location {
            write!(f, "{} at {}: {}", self.kind, location, self.message)?;
        } else {
            write!(f, "{}: {}", self.kind, self.message)?;
        }
        
        // 詳細情報があれば追加
        if let Some(details) = &self.details {
            write!(f, "\n  詳細: {}", details)?;
        }
        
        // 関連エラーがあれば追加
        for related in &self.related {
            write!(f, "\n  {}", related)?;
        }
        
        Ok(())
    }
}

impl StdError for CompilerError {}

/// 関連エラー（ヒントやサブエラーなど）
#[derive(Debug, Clone)]
pub enum RelatedError {
    /// ヒント（対処方法など）
    Hint {
        /// ヒントのメッセージ
        message: String,
        /// 関連する位置
        location: Option<SourceLocation>,
    },
    /// メモ（補足情報）
    Note {
        /// メモのメッセージ
        message: String,
        /// 関連する位置
        location: Option<SourceLocation>,
    },
    /// 警告
    Warning {
        /// 警告メッセージ
        message: String,
        /// 関連する位置
        location: Option<SourceLocation>,
    },
    /// 関連エラー
    Related {
        /// エラーメッセージ
        message: String,
        /// 関連する位置
        location: Option<SourceLocation>,
    },
}

impl fmt::Display for RelatedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RelatedError::Hint { message, location } => {
                if let Some(loc) = location {
                    write!(f, "ヒント at {}: {}", loc, message)
                } else {
                    write!(f, "ヒント: {}", message)
                }
            },
            RelatedError::Note { message, location } => {
                if let Some(loc) = location {
                    write!(f, "メモ at {}: {}", loc, message)
                } else {
                    write!(f, "メモ: {}", message)
                }
            },
            RelatedError::Warning { message, location } => {
                if let Some(loc) = location {
                    write!(f, "警告 at {}: {}", loc, message)
                } else {
                    write!(f, "警告: {}", message)
                }
            },
            RelatedError::Related { message, location } => {
                if let Some(loc) = location {
                    write!(f, "関連 at {}: {}", loc, message)
                } else {
                    write!(f, "関連: {}", message)
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_source_location() {
        let loc = SourceLocation::new("test.swl", 10, 5, 100, 10);
        assert_eq!(loc.file_name, "test.swl");
        assert_eq!(loc.line, 10);
        assert_eq!(loc.column, 5);
        assert_eq!(loc.offset, 100);
        assert_eq!(loc.length, 10);
        
        assert_eq!(loc.to_string(), "test.swl:10:5");
    }
    
    #[test]
    fn test_source_location_merge() {
        let loc1 = SourceLocation::new("test.swl", 10, 5, 100, 10);
        let loc2 = SourceLocation::new("test.swl", 10, 15, 110, 10);
        
        let merged = loc1.merge(&loc2);
        assert_eq!(merged.file_name, "test.swl");
        assert_eq!(merged.line, 10);
        assert_eq!(merged.column, 5);
        assert_eq!(merged.offset, 100);
        assert_eq!(merged.length, 20);
    }
    
    #[test]
    fn test_compiler_error() {
        let location = SourceLocation::new("test.swl", 10, 5, 100, 10);
        let error = CompilerError::syntax_error("予期しないトークンです", location.clone());
        
        assert_eq!(error.kind, ErrorKind::Syntax);
        assert_eq!(error.message, "予期しないトークンです");
        assert_eq!(error.location, Some(location));
        
        // 追加情報付きのエラー
        let error = CompilerError::type_error("型が一致しません", Some(location.clone()))
            .with_details("int型とstring型は互換性がありません")
            .with_hint("string型に明示的にキャストしてください");
        
        assert_eq!(error.kind, ErrorKind::Type);
        assert_eq!(error.message, "型が一致しません");
        assert_eq!(error.location, Some(location));
        assert_eq!(error.details, Some("int型とstring型は互換性がありません".to_string()));
        assert_eq!(error.related.len(), 1);
    }
    
    #[test]
    fn test_error_display() {
        let location = SourceLocation::new("test.swl", 10, 5, 100, 10);
        let error = CompilerError::syntax_error("予期しないトークンです", location);
        
        let error_str = format!("{}", error);
        assert!(error_str.contains("構文解析エラー"));
        assert!(error_str.contains("test.swl:10:5"));
        assert!(error_str.contains("予期しないトークンです"));
    }
} 
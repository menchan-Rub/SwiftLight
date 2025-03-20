// エラー処理およびエラー報告のためのモジュール
// コンパイラ全体で使用されるエラータイプやエラーハンドリング機能を提供します

use std::fmt;
use std::error::Error;
use std::path::{Path, PathBuf};

/// エラーの重大度
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    /// 警告 - コンパイルは継続可能
    Warning,
    /// エラー - コンパイルは失敗する
    Error,
    /// 致命的エラー - すぐに終了する必要がある
    Fatal,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSeverity::Warning => write!(f, "警告"),
            ErrorSeverity::Error => write!(f, "エラー"),
            ErrorSeverity::Fatal => write!(f, "致命的エラー"),
        }
    }
}

/// エラーが発生した場所
#[derive(Debug, Clone)]
pub struct ErrorLocation {
    /// ファイルパス
    pub file: Option<PathBuf>,
    /// 行番号
    pub line: Option<usize>,
    /// 列番号
    pub column: Option<usize>,
}

impl ErrorLocation {
    /// 新しいエラー位置を作成
    pub fn new<P: AsRef<Path>>(file: Option<P>, line: Option<usize>, column: Option<usize>) -> Self {
        Self {
            file: file.map(|p| p.as_ref().to_path_buf()),
            line,
            column,
        }
    }

    /// ファイルのみのエラー位置を作成
    pub fn file_only<P: AsRef<Path>>(file: P) -> Self {
        Self {
            file: Some(file.as_ref().to_path_buf()),
            line: None,
            column: None,
        }
    }

    /// 位置情報なしのエラー位置を作成
    pub fn unknown() -> Self {
        Self {
            file: None,
            line: None,
            column: None,
        }
    }
}

impl fmt::Display for ErrorLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(file) = &self.file {
            write!(f, "{}", file.display())?;
            if let Some(line) = self.line {
                write!(f, ":{}:", line)?;
                if let Some(column) = self.column {
                    write!(f, "{}", column)?;
                }
            }
            Ok(())
        } else {
            write!(f, "<不明な位置>")
        }
    }
}

/// コンパイラエラー
#[derive(Debug, Clone)]
pub struct CompilerError {
    /// エラーコード
    pub code: String,
    /// エラーメッセージ
    pub message: String,
    /// エラーの発生箇所
    pub location: ErrorLocation,
    /// エラーの重大度
    pub severity: ErrorSeverity,
    /// 追加情報やヒント
    pub notes: Vec<String>,
    /// 根本原因となったエラー
    pub source: Option<Box<dyn Error + Send + Sync>>,
}

impl CompilerError {
    /// 新しいコンパイラエラーを作成
    pub fn new<S: Into<String>>(
        code: S,
        message: S,
        location: ErrorLocation,
        severity: ErrorSeverity,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            location,
            severity,
            notes: Vec::new(),
            source: None,
        }
    }

    /// エラーにヒントを追加
    pub fn with_note<S: Into<String>>(mut self, note: S) -> Self {
        self.notes.push(note.into());
        self
    }

    /// エラーに複数のヒントを追加
    pub fn with_notes<S: Into<String>>(mut self, notes: Vec<S>) -> Self {
        for note in notes {
            self.notes.push(note.into());
        }
        self
    }

    /// エラーの根本原因を設定
    pub fn with_source<E: Error + Send + Sync + 'static>(mut self, source: E) -> Self {
        self.source = Some(Box::new(source));
        self
    }

    /// 警告を作成
    pub fn warning<S: Into<String>>(code: S, message: S, location: ErrorLocation) -> Self {
        Self::new(code, message, location, ErrorSeverity::Warning)
    }

    /// エラーを作成
    pub fn error<S: Into<String>>(code: S, message: S, location: ErrorLocation) -> Self {
        Self::new(code, message, location, ErrorSeverity::Error)
    }

    /// 致命的エラーを作成
    pub fn fatal<S: Into<String>>(code: S, message: S, location: ErrorLocation) -> Self {
        Self::new(code, message, location, ErrorSeverity::Fatal)
    }
}

impl fmt::Display for CompilerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: [{}] {}", self.severity, self.code, self.message)?;
        if self.location.file.is_some() || self.location.line.is_some() {
            write!(f, " at {}", self.location)?;
        }
        
        for note in &self.notes {
            write!(f, "\n注意: {}", note)?;
        }
        
        if let Some(source) = &self.source {
            write!(f, "\n原因: {}", source)?;
        }
        
        Ok(())
    }
}

impl Error for CompilerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref() as &(dyn Error + 'static))
    }
}

/// コンパイル結果型
pub type CompilerResult<T> = Result<T, CompilerError>;

/// エラーハンドラー
pub trait ErrorHandler {
    /// エラーを報告
    fn report_error(&mut self, error: CompilerError);
    
    /// 警告を報告
    fn report_warning(&mut self, warning: CompilerError);
    
    /// エラーの合計数を取得
    fn error_count(&self) -> usize;
    
    /// 警告の合計数を取得
    fn warning_count(&self) -> usize;
    
    /// 報告されたエラーを全て取得
    fn get_errors(&self) -> &[CompilerError];
    
    /// 報告された警告を全て取得
    fn get_warnings(&self) -> &[CompilerError];
    
    /// 全てのエラーをクリア
    fn clear(&mut self);
}

/// 基本的なエラーハンドラーの実装
#[derive(Debug, Default)]
pub struct BasicErrorHandler {
    /// 報告されたエラー
    errors: Vec<CompilerError>,
    /// 報告された警告
    warnings: Vec<CompilerError>,
}

impl BasicErrorHandler {
    /// 新しいエラーハンドラーを作成
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

impl ErrorHandler for BasicErrorHandler {
    fn report_error(&mut self, error: CompilerError) {
        println!("{}", error);
        self.errors.push(error);
    }
    
    fn report_warning(&mut self, warning: CompilerError) {
        println!("{}", warning);
        self.warnings.push(warning);
    }
    
    fn error_count(&self) -> usize {
        self.errors.len()
    }
    
    fn warning_count(&self) -> usize {
        self.warnings.len()
    }
    
    fn get_errors(&self) -> &[CompilerError] {
        &self.errors
    }
    
    fn get_warnings(&self) -> &[CompilerError] {
        &self.warnings
    }
    
    fn clear(&mut self) {
        self.errors.clear();
        self.warnings.clear();
    }
} 
//! 診断情報モジュール
//! 
//! コンパイラの診断情報を管理し、整形して出力するためのユーティリティを提供します。

use std::io;
use std::fmt;
use std::collections::HashMap;

/// 診断情報のレベル
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    /// エラー
    Error,
    /// 警告
    Warning,
    /// 情報
    Info,
    /// ヒント
    Hint,
    /// 注意
    Note,
}

impl fmt::Display for DiagnosticLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiagnosticLevel::Error => write!(f, "エラー"),
            DiagnosticLevel::Warning => write!(f, "警告"),
            DiagnosticLevel::Info => write!(f, "情報"),
            DiagnosticLevel::Hint => write!(f, "ヒント"),
            DiagnosticLevel::Note => write!(f, "注意"),
        }
    }
}

/// 診断情報
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// レベル
    pub level: DiagnosticLevel,
    /// メッセージ
    pub message: String,
    /// コード
    pub code: Option<String>,
}

impl Diagnostic {
    /// 新しい診断情報を作成
    pub fn new(level: DiagnosticLevel, message: String) -> Self {
        Self {
            level,
            message,
            code: None,
        }
    }
    
    /// コードを設定
    pub fn with_code(mut self, code: String) -> Self {
        self.code = Some(code);
        self
    }
}

/// 診断情報エミッタ - 診断情報を出力する
pub struct DiagnosticEmitter {
    /// 診断情報
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticEmitter {
    /// 新しい診断エミッタを作成
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }
    
    /// 診断情報を発行
    pub fn emit(&self, diagnostic: Diagnostic) {
        // 実際の実装ではログに出力するなど
        match diagnostic.level {
            DiagnosticLevel::Error => log::error!("{}", diagnostic.message),
            DiagnosticLevel::Warning => log::warn!("{}", diagnostic.message),
            DiagnosticLevel::Info => log::info!("{}", diagnostic.message),
            DiagnosticLevel::Hint => log::debug!("{}", diagnostic.message),
            DiagnosticLevel::Note => log::debug!("{}", diagnostic.message),
        }
    }
    
    /// 診断情報を追加
    pub fn add_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }
    
    /// 診断情報を取得
    pub fn get_diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
    
    /// エラーが存在するかどうか
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.level == DiagnosticLevel::Error)
    }
    
    /// 警告が存在するかどうか
    pub fn has_warnings(&self) -> bool {
        self.diagnostics.iter().any(|d| d.level == DiagnosticLevel::Warning)
    }
    
    /// 診断情報をクリア
    pub fn clear(&mut self) {
        self.diagnostics.clear();
    }
}

impl Default for DiagnosticEmitter {
    fn default() -> Self {
        Self::new()
    }
} 
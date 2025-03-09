//! # コンパイラ診断情報
//! 
//! コンパイラの診断情報を表現するためのモジュールです。
//! エラー、警告、ヒントなどの診断メッセージを管理します。

use std::fmt;
use crate::frontend::error::SourceLocation;

/// 診断レベルを表す列挙型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticLevel {
    /// エラー: コンパイルを中断する重大な問題
    Error,
    /// 警告: コンパイルは続行するが、潜在的な問題
    Warning,
    /// 情報: 単なる情報提供
    Info,
    /// ヒント: 問題解決のための提案
    Hint,
    /// 注記: 追加の説明
    Note,
}

impl fmt::Display for DiagnosticLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiagnosticLevel::Error => write!(f, "エラー"),
            DiagnosticLevel::Warning => write!(f, "警告"),
            DiagnosticLevel::Info => write!(f, "情報"),
            DiagnosticLevel::Hint => write!(f, "ヒント"),
            DiagnosticLevel::Note => write!(f, "注記"),
        }
    }
}

/// 診断情報を表す構造体
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// 診断レベル
    pub level: DiagnosticLevel,
    /// 診断メッセージ
    pub message: String,
    /// ソースコード内の位置情報（オプション）
    pub location: Option<SourceLocation>,
    /// 問題のあるコードの範囲を強調表示するためのスニペット
    pub code_snippet: Option<String>,
    /// 修正案（オプション）
    pub suggestion: Option<String>,
    /// 関連する補足的な診断情報
    pub related: Vec<Diagnostic>,
}

impl Diagnostic {
    /// 新しい診断情報を作成
    pub fn new(
        level: DiagnosticLevel, 
        message: impl Into<String>,
        location: Option<SourceLocation>,
    ) -> Self {
        Self {
            level,
            message: message.into(),
            location,
            code_snippet: None,
            suggestion: None,
            related: Vec::new(),
        }
    }
    
    /// エラー診断を作成
    pub fn error(message: impl Into<String>, location: Option<SourceLocation>) -> Self {
        Self::new(DiagnosticLevel::Error, message, location)
    }
    
    /// 警告診断を作成
    pub fn warning(message: impl Into<String>, location: Option<SourceLocation>) -> Self {
        Self::new(DiagnosticLevel::Warning, message, location)
    }
    
    /// 情報診断を作成
    pub fn info(message: impl Into<String>, location: Option<SourceLocation>) -> Self {
        Self::new(DiagnosticLevel::Info, message, location)
    }
    
    /// ヒント診断を作成
    pub fn hint(message: impl Into<String>, location: Option<SourceLocation>) -> Self {
        Self::new(DiagnosticLevel::Hint, message, location)
    }
    
    /// 注記診断を作成
    pub fn note(message: impl Into<String>, location: Option<SourceLocation>) -> Self {
        Self::new(DiagnosticLevel::Note, message, location)
    }
    
    /// コードスニペットを設定
    pub fn with_code_snippet(mut self, snippet: impl Into<String>) -> Self {
        self.code_snippet = Some(snippet.into());
        self
    }
    
    /// 修正案を設定
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
    
    /// 関連する診断情報を追加
    pub fn add_related(&mut self, related: Diagnostic) {
        self.related.push(related);
    }
    
    /// 関連する診断情報を設定して自身を返す
    pub fn with_related(mut self, related: Diagnostic) -> Self {
        self.add_related(related);
        self
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref loc) = self.location {
            write!(f, "[{}] at {}: {}", self.level, loc, self.message)?;
        } else {
            write!(f, "[{}]: {}", self.level, self.message)?;
        }
        
        if let Some(ref snippet) = self.code_snippet {
            write!(f, "\n\n{}\n", snippet)?;
        }
        
        if let Some(ref suggestion) = self.suggestion {
            write!(f, "\n修正案: {}", suggestion)?;
        }
        
        for related in &self.related {
            write!(f, "\n  -> {}", related)?;
        }
        
        Ok(())
    }
}

/// 診断情報のコレクションを管理する構造体
#[derive(Debug, Default, Clone)]
pub struct DiagnosticManager {
    /// 蓄積された診断情報
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticManager {
    /// 新しい診断マネージャーを作成
    pub fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }
    
    /// 診断情報を追加
    pub fn add(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }
    
    /// エラー診断を追加
    pub fn add_error(&mut self, message: impl Into<String>, location: Option<SourceLocation>) {
        self.add(Diagnostic::error(message, location));
    }
    
    /// 警告診断を追加
    pub fn add_warning(&mut self, message: impl Into<String>, location: Option<SourceLocation>) {
        self.add(Diagnostic::warning(message, location));
    }
    
    /// 蓄積された診断情報のスライスを取得
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
    
    /// エラーがあるかどうかを判定
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| matches!(d.level, DiagnosticLevel::Error))
    }
    
    /// 診断情報をクリア
    pub fn clear(&mut self) {
        self.diagnostics.clear();
    }
    
    /// 診断情報の数を取得
    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }
    
    /// 診断情報が空かどうかを判定
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

/// テスト
#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::error::SourceLocation;
    
    #[test]
    fn test_diagnostic_creation() {
        let location = SourceLocation::new("test.swl", 10, 5, 100, 105);
        let diagnostic = Diagnostic::error("変数が定義されていません", Some(location.clone()));
        
        assert_eq!(diagnostic.level, DiagnosticLevel::Error);
        assert_eq!(diagnostic.message, "変数が定義されていません");
        assert_eq!(diagnostic.location, Some(location));
    }
    
    #[test]
    fn test_diagnostic_with_suggestion() {
        let diagnostic = Diagnostic::error("変数が定義されていません", None)
            .with_suggestion("変数を宣言してください: let x = 0");
        
        assert!(diagnostic.suggestion.is_some());
        assert_eq!(diagnostic.suggestion.unwrap(), "変数を宣言してください: let x = 0");
    }
    
    #[test]
    fn test_diagnostic_manager() {
        let mut manager = DiagnosticManager::new();
        
        manager.add_error("エラー1", None);
        manager.add_warning("警告1", None);
        
        assert_eq!(manager.len(), 2);
        assert!(manager.has_errors());
    }
} 
//! # パーサーエラー回復機構
//! 
//! 構文解析中のエラー発生時に回復するための機能を提供します。
//! エラーがあっても可能な限り解析を継続し、複数のエラーを一度に報告できるようにします。

use std::collections::HashSet;

use crate::frontend::error::{CompilerError, SourceLocation};
use crate::frontend::lexer::token::{Token, TokenKind};

/// エラー回復モード
pub enum RecoveryMode {
    /// 次のセミコロンまで飛ばす（文単位の回復）
    SkipToSemicolon,
    /// 次のブロック終了まで飛ばす（ブロック単位の回復）
    SkipToBlockEnd,
    /// 特定のトークンまで飛ばす
    SkipToToken(TokenKind),
    /// 指定されたトークンのいずれかまで飛ばす
    SkipToAnyToken(Vec<TokenKind>),
    /// 式の終わりまで飛ばす
    SkipToEndOfExpression,
    /// 文の終わりまで飛ばす
    SkipToEndOfStatement,
    /// 宣言の終わりまで飛ばす
    SkipToEndOfDeclaration,
}

/// 同期ポイント用トークン集合（エラー回復時に同期するトークン）
pub struct SyncSet {
    /// 同期に使用するトークンの集合
    tokens: HashSet<TokenKind>,
}

impl SyncSet {
    /// 新しい同期ポイント集合を作成
    pub fn new() -> Self {
        Self { tokens: HashSet::new() }
    }
    
    /// 同期ポイントとしてのトークンを追加
    pub fn add(&mut self, token: TokenKind) -> &mut Self {
        self.tokens.insert(token);
        self
    }
    
    /// 複数の同期ポイントを追加
    pub fn add_all(&mut self, tokens: &[TokenKind]) -> &mut Self {
        for token in tokens {
            self.tokens.insert(token.clone());
        }
        self
    }
    
    /// 指定したトークンが同期ポイントかどうかをチェック
    pub fn contains(&self, token: &TokenKind) -> bool {
        self.tokens.contains(token)
    }
}

impl Default for SyncSet {
    fn default() -> Self {
        let mut set = Self::new();
        // デフォルトの同期ポイントを設定
        set.add_all(&[
            TokenKind::Semicolon,
            TokenKind::RightBrace,
            TokenKind::KeywordFn,
            TokenKind::KeywordLet,
            TokenKind::KeywordVar,
            TokenKind::KeywordConst,
            TokenKind::KeywordStruct,
            TokenKind::KeywordEnum,
            TokenKind::KeywordTrait,
            TokenKind::KeywordImpl,
            TokenKind::KeywordType,
            TokenKind::KeywordImport,
            TokenKind::KeywordModule,
        ]);
        set
    }
}

/// パーサーエラー回復機能
pub struct ErrorRecovery {
    /// 現在のエラー回復モード
    mode: Option<RecoveryMode>,
    /// 回復中かどうか
    is_recovering: bool,
    /// これまでに報告されたエラー
    reported_errors: Vec<CompilerError>,
    /// エラー回復で使用する同期ポイント
    sync_set: SyncSet,
    /// パニックモード中か（エラー回復が難しい重大なエラー発生時）
    in_panic_mode: bool,
}

impl ErrorRecovery {
    /// 新しいエラー回復機能を作成
    pub fn new() -> Self {
        Self {
            mode: None,
            is_recovering: false,
            reported_errors: Vec::new(),
            sync_set: SyncSet::default(),
            in_panic_mode: false,
        }
    }
    
    /// エラーを報告し、回復モードを設定
    pub fn report_error(&mut self, error: CompilerError, mode: RecoveryMode) {
        self.reported_errors.push(error);
        self.mode = Some(mode);
        self.is_recovering = true;
    }
    
    /// 現在のトークンが同期ポイントかどうかを判定
    pub fn is_sync_point(&self, token: &TokenKind) -> bool {
        self.sync_set.contains(token)
    }
    
    /// 現在回復モードかどうかを取得
    pub fn is_recovering(&self) -> bool {
        self.is_recovering
    }
    
    /// パニックモードの状態を取得
    pub fn in_panic_mode(&self) -> bool {
        self.in_panic_mode
    }
    
    /// パニックモードを設定
    pub fn set_panic_mode(&mut self, panic_mode: bool) {
        self.in_panic_mode = panic_mode;
    }
    
    /// エラー回復モードをリセット
    pub fn reset_recovery(&mut self) {
        self.is_recovering = false;
        self.mode = None;
    }
    
    /// 現在のトークンで回復を試みる
    pub fn try_recover(&mut self, token: &Token) -> bool {
        if !self.is_recovering {
            return true;
        }
        
        match &self.mode {
            Some(RecoveryMode::SkipToSemicolon) => {
                if token.kind == TokenKind::Semicolon {
                    self.reset_recovery();
                    return true;
                }
            }
            Some(RecoveryMode::SkipToBlockEnd) => {
                if token.kind == TokenKind::RightBrace {
                    self.reset_recovery();
                    return true;
                }
            }
            Some(RecoveryMode::SkipToToken(target)) => {
                if &token.kind == target {
                    self.reset_recovery();
                    return true;
                }
            }
            Some(RecoveryMode::SkipToAnyToken(targets)) => {
                if targets.contains(&token.kind) {
                    self.reset_recovery();
                    return true;
                }
            }
            Some(RecoveryMode::SkipToEndOfExpression) => {
                // 式の終わりを示すトークンの集合
                let end_tokens = [
                    TokenKind::Semicolon,
                    TokenKind::Comma,
                    TokenKind::RightParen,
                    TokenKind::RightBrace,
                    TokenKind::RightBracket,
                ];
                
                if end_tokens.contains(&token.kind) {
                    self.reset_recovery();
                    return true;
                }
            }
            Some(RecoveryMode::SkipToEndOfStatement) => {
                // 文の終わりを示すトークンの集合
                if token.kind == TokenKind::Semicolon || token.kind == TokenKind::RightBrace {
                    self.reset_recovery();
                    return true;
                }
            }
            Some(RecoveryMode::SkipToEndOfDeclaration) => {
                // 宣言の終わりを示すトークンの集合
                let end_tokens = [
                    TokenKind::KeywordFn,
                    TokenKind::KeywordLet,
                    TokenKind::KeywordVar,
                    TokenKind::KeywordConst,
                    TokenKind::KeywordStruct,
                    TokenKind::KeywordEnum,
                    TokenKind::KeywordTrait,
                    TokenKind::KeywordImpl,
                    TokenKind::KeywordType,
                    TokenKind::KeywordImport,
                    TokenKind::KeywordModule,
                    TokenKind::RightBrace,
                    TokenKind::EOF,
                ];
                
                if end_tokens.contains(&token.kind) {
                    self.reset_recovery();
                    return true;
                }
            }
            None => {
                return true;
            }
        }
        
        // 同期ポイントでも回復
        if self.is_sync_point(&token.kind) {
            self.reset_recovery();
            return true;
        }
        
        // まだ回復できていない
        false
    }
    
    /// エラー回復で飛ばしたトークンとして不足しているものを挿入したトークンを生成
    pub fn synthesize_token(&self, kind: TokenKind, location: SourceLocation) -> Token {
        Token::new(kind, String::new(), location)
    }
    
    /// これまでに報告されたエラーを取得
    pub fn get_reported_errors(&self) -> &[CompilerError] {
        &self.reported_errors
    }
}

impl Default for ErrorRecovery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sync_set() {
        let mut set = SyncSet::new();
        set.add(TokenKind::Semicolon).add(TokenKind::RightBrace);
        
        assert!(set.contains(&TokenKind::Semicolon));
        assert!(set.contains(&TokenKind::RightBrace));
        assert!(!set.contains(&TokenKind::LeftBrace));
    }
    
    #[test]
    fn test_error_recovery() {
        let location = SourceLocation::new("test.swl", 1, 1, 0, 1);
        let token = Token::new(TokenKind::Semicolon, ";".to_string(), location.clone());
        
        let mut recovery = ErrorRecovery::new();
        assert!(!recovery.is_recovering());
        
        // エラーを報告
        let error = CompilerError::syntax_error("構文エラー", location.clone());
        recovery.report_error(error, RecoveryMode::SkipToSemicolon);
        
        assert!(recovery.is_recovering());
        
        // セミコロンで回復
        assert!(recovery.try_recover(&token));
        assert!(!recovery.is_recovering());
    }
    
    #[test]
    fn test_multiple_recovery_modes() {
        let location = SourceLocation::new("test.swl", 1, 1, 0, 1);
        
        let mut recovery = ErrorRecovery::new();
        let error = CompilerError::syntax_error("構文エラー", location.clone());
        
        // ブロック終了までスキップ
        recovery.report_error(error, RecoveryMode::SkipToBlockEnd);
        
        // セミコロンでは回復しない
        let semicolon = Token::new(TokenKind::Semicolon, ";".to_string(), location.clone());
        assert!(!recovery.try_recover(&semicolon));
        assert!(recovery.is_recovering());
        
        // 右中括弧で回復
        let right_brace = Token::new(TokenKind::RightBrace, "}".to_string(), location.clone());
        assert!(recovery.try_recover(&right_brace));
        assert!(!recovery.is_recovering());
    }
}

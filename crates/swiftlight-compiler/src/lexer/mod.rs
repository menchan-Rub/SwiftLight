use crate::core::types::{Error, Result};
use crate::core::collections::Vec;
use crate::core::source::{SourceLocation, SourceRange};
use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use std::thread;
use std::thread::ThreadPool;
use std::collections::HashSet;

/// トークンの種類を表す列挙型
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // キーワード
    Let,
    Fn,
    Return,
    If,
    Else,
    While,
    For,
    In,
    Match,
    Case,
    Default,
    Break,
    Continue,
    
    // 量子計算関連のキーワード
    Quantum,
    Qubit,
    Measure,
    Entangle,
    Superposition,
    Gate,
    Circuit,
    
    // 時相型関連のキーワード
    Temporal,
    Future,
    Past,
    Always,
    Eventually,
    Until,
    Since,
    
    // エフェクトシステム関連のキーワード
    Effect,
    Handle,
    Resume,
    Suspend,
    Pure,
    Impure,
    
    // リソース管理関連のキーワード
    Resource,
    Acquire,
    Release,
    Use,
    With,
    Dispose,
    
    // 識別子とリテラル
    Identifier(String),
    Integer(i64),
    Float(f64),
    String(String),
    Char(char),
    
    // 演算子
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Equal,
    EqualEqual,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    Not,
    
    // 量子演算子
    TensorProduct,
    QuantumOr,
    QuantumAnd,
    QuantumNot,
    
    // 時相演算子
    TemporalOr,
    TemporalAnd,
    TemporalNot,
    
    // エフェクト演算子
    EffectOr,
    EffectAnd,
    EffectNot,
    
    // 区切り文字
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Semicolon,
    Comma,
    Dot,
    Arrow,
    
    // 特殊
    EOF,
    Error,
}

/// トークンの種類に対応するハイライトスタイル
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HighlightStyle {
    /// キーワード
    Keyword,
    /// 識別子
    Identifier,
    /// 文字列リテラル
    String,
    /// 数値リテラル
    Number,
    /// 演算子
    Operator,
    /// 区切り文字
    Delimiter,
    /// コメント
    Comment,
    /// その他
    Other,
}

impl TokenKind {
    /// トークンの種類に対応するハイライトスタイルを取得
    pub fn highlight_style(&self) -> HighlightStyle {
        match self {
            // キーワード
            TokenKind::Let |
            TokenKind::Fn |
            TokenKind::Return |
            TokenKind::If |
            TokenKind::Else |
            TokenKind::While |
            TokenKind::For |
            TokenKind::In |
            TokenKind::Match |
            TokenKind::Case |
            TokenKind::Default |
            TokenKind::Break |
            TokenKind::Continue => HighlightStyle::Keyword,

            // 識別子
            TokenKind::Identifier(_) => HighlightStyle::Identifier,

            // 文字列リテラル
            TokenKind::String(_) => HighlightStyle::String,

            // 数値リテラル
            TokenKind::Integer(_) |
            TokenKind::Float(_) => HighlightStyle::Number,

            // 演算子
            TokenKind::Plus |
            TokenKind::Minus |
            TokenKind::Star |
            TokenKind::Slash |
            TokenKind::Percent |
            TokenKind::Equal |
            TokenKind::EqualEqual |
            TokenKind::NotEqual |
            TokenKind::Less |
            TokenKind::LessEqual |
            TokenKind::Greater |
            TokenKind::GreaterEqual |
            TokenKind::And |
            TokenKind::Or |
            TokenKind::Not => HighlightStyle::Operator,

            // 区切り文字
            TokenKind::LeftParen |
            TokenKind::RightParen |
            TokenKind::LeftBrace |
            TokenKind::RightBrace |
            TokenKind::LeftBracket |
            TokenKind::RightBracket |
            TokenKind::Semicolon |
            TokenKind::Comma |
            TokenKind::Dot |
            TokenKind::Arrow => HighlightStyle::Delimiter,

            // その他
            _ => HighlightStyle::Other,
        }
    }
}

/// ハイライト情報を表す構造体
#[derive(Debug, Clone)]
pub struct HighlightInfo {
    /// 開始位置
    pub start: usize,
    /// 終了位置
    pub end: usize,
    /// ハイライトスタイル
    pub style: HighlightStyle,
}

/// トークンを表す構造体
#[derive(Debug, Clone)]
pub struct Token {
    /// トークンの種類
    pub kind: TokenKind,
    /// トークンの位置情報
    pub range: SourceRange,
    /// トークンの文字列
    pub lexeme: String,
}

/// レキサーの最適化オプション
#[derive(Debug, Clone, Copy)]
pub struct LexerOptimizationOptions {
    /// トークンキャッシュを有効にするか
    pub enable_token_cache: bool,
    /// 並列レキサル解析を有効にするか
    pub enable_parallel_lexing: bool,
    /// キャッシュサイズ
    pub cache_size: usize,
    /// バッファサイズ
    pub buffer_size: usize,
}

impl Default for LexerOptimizationOptions {
    fn default() -> Self {
        Self {
            enable_token_cache: true,
            enable_parallel_lexing: false,
            cache_size: 1000,
            buffer_size: 4096,
        }
    }
}

/// トークンキャッシュ
#[derive(Debug, Clone)]
struct TokenCache {
    /// トークンのキャッシュ
    tokens: HashMap<usize, Token>,
    /// ハイライト情報のキャッシュ
    highlights: HashMap<usize, HighlightInfo>,
}

impl TokenCache {
    fn new() -> Self {
        Self {
            tokens: HashMap::new(),
            highlights: HashMap::new(),
        }
    }

    fn clear(&mut self) {
        self.tokens.clear();
        self.highlights.clear();
    }
}

/// レキサーの並列解析用のスレッドプール
#[derive(Debug)]
struct ParallelLexerPool {
    /// スレッドプール
    pool: ThreadPool,
    /// トークンの結果を格納するチャネル
    result_channel: (Sender<Vec<Token>>, Receiver<Vec<Token>>),
}

impl ParallelLexerPool {
    fn new(num_threads: usize) -> Self {
        Self {
            pool: ThreadPool::new(num_threads),
            result_channel: mpsc::channel(),
        }
    }
}

/// レキサーの拡張機能
#[derive(Debug)]
pub struct LexerExtensions {
    /// カスタムトークン定義
    custom_tokens: HashMap<String, TokenKind>,
    /// トークン変換ルール
    token_transforms: Vec<TokenTransform>,
    /// 言語拡張の設定
    language_extensions: HashSet<LanguageExtension>,
}

/// トークン変換ルール
#[derive(Debug, Clone)]
pub struct TokenTransform {
    /// 変換前のトークン
    from: TokenKind,
    /// 変換後のトークン
    to: TokenKind,
    /// 変換条件
    condition: Box<dyn Fn(&str) -> bool>,
}

/// レキサーを表す構造体
pub struct Lexer {
    /// ソースコード
    source: String,
    /// 現在の位置
    current: usize,
    /// 開始位置
    start: usize,
    /// 行番号
    line: usize,
    /// 列番号
    column: usize,
    /// ハイライト情報のリスト
    highlights: Vec<HighlightInfo>,
    /// 最適化オプション
    optimization_options: LexerOptimizationOptions,
    /// トークンキャッシュ
    token_cache: TokenCache,
    /// 並列レキサル解析用のスレッドプール
    parallel_pool: ParallelLexerPool,
    /// レキサーの拡張機能
    extensions: LexerExtensions,
}

impl Lexer {
    /// 新しいレキサーを作成
    pub fn new(source: String) -> Self {
        Self {
            source,
            current: 0,
            start: 0,
            line: 1,
            column: 1,
            highlights: Vec::new(),
            optimization_options: LexerOptimizationOptions::default(),
            token_cache: TokenCache::new(),
            parallel_pool: ParallelLexerPool::new(4),
            extensions: LexerExtensions {
                custom_tokens: HashMap::new(),
                token_transforms: Vec::new(),
                language_extensions: HashSet::new(),
            },
        }
    }
    
    /// ハイライト情報を取得
    pub fn get_highlights(&self) -> &[HighlightInfo] {
        &self.highlights
    }
    
    /// ハイライト情報をクリア
    pub fn clear_highlights(&mut self) {
        self.highlights.clear();
    }
    
    /// 次のトークンを取得
    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();
        self.start = self.current;
        
        let c = self.advance();
        let token = match c {
            Some(c) => match c {
                '(' => self.make_token(TokenKind::LeftParen),
                ')' => self.make_token(TokenKind::RightParen),
                '{' => self.make_token(TokenKind::LeftBrace),
                '}' => self.make_token(TokenKind::RightBrace),
                '[' => self.make_token(TokenKind::LeftBracket),
                ']' => self.make_token(TokenKind::RightBracket),
                ';' => self.make_token(TokenKind::Semicolon),
                ',' => self.make_token(TokenKind::Comma),
                '.' => self.make_token(TokenKind::Dot),
                '-' => {
                    if self.match_char('>') {
                        self.make_token(TokenKind::Arrow)
                    } else {
                        self.make_token(TokenKind::Minus)
                    }
                },
                '+' => self.make_token(TokenKind::Plus),
                '*' => self.make_token(TokenKind::Star),
                '/' => self.make_token(TokenKind::Slash),
                '%' => self.make_token(TokenKind::Percent),
                '=' => {
                    if self.match_char('=') {
                        self.make_token(TokenKind::EqualEqual)
                    } else {
                        self.make_token(TokenKind::Equal)
                    }
                },
                '!' => {
                    if self.match_char('=') {
                        self.make_token(TokenKind::NotEqual)
                    } else {
                        self.make_token(TokenKind::Not)
                    }
                },
                '<' => {
                    if self.match_char('=') {
                        self.make_token(TokenKind::LessEqual)
                    } else {
                        self.make_token(TokenKind::Less)
                    }
                },
                '>' => {
                    if self.match_char('=') {
                        self.make_token(TokenKind::GreaterEqual)
                    } else {
                        self.make_token(TokenKind::Greater)
                    }
                },
                '&' => {
                    if self.match_char('&') {
                        self.make_token(TokenKind::And)
                    } else {
                        self.error_token("Unexpected character")
                    }
                },
                '|' => {
                    if self.match_char('|') {
                        self.make_token(TokenKind::Or)
                    } else {
                        self.error_token("Unexpected character")
                    }
                },
                '"' => self.string(),
                '\'' => self.character(),
                c if c.is_alphabetic() || c == '_' => self.identifier(),
                c if c.is_digit(10) => self.number(),
                _ => self.error_token("Unexpected character"),
            },
            None => self.make_token(TokenKind::EOF),
        };

        // ハイライト情報を追加
        let style = token.kind.highlight_style();
        self.highlights.push(HighlightInfo {
            start: self.start,
            end: self.current,
            style,
        });

        token
    }
    
    /// 空白文字をスキップ
    fn skip_whitespace(&mut self) {
        loop {
            match self.peek() {
                Some(c) => match c {
                    ' ' | '\r' | '\t' => {
                        self.advance();
                    },
                    '\n' => {
                        self.line += 1;
                        self.column = 1;
                        self.advance();
                    },
                    '/' => {
                        if self.peek_next() == Some('/') {
                            // コメントをスキップ
                            while self.peek() != Some('\n') && self.peek().is_some() {
                                self.advance();
                            }
                        } else {
                            return;
                        }
                    },
                    _ => return,
                },
                None => return,
            }
        }
    }
    
    /// 文字列リテラルを解析
    fn string(&mut self) -> Token {
        while self.peek() != Some('"') && self.peek().is_some() {
            if self.peek() == Some('\n') {
                self.line += 1;
                self.column = 1;
            }
            self.advance();
        }
        
        if self.peek().is_none() {
            return self.error_token("Unterminated string");
        }
        
        self.advance(); // 閉じ引用符を消費
        
        let value = self.source[self.start + 1..self.current - 1].to_string();
        self.make_token(TokenKind::String(value))
    }
    
    /// 文字リテラルを解析
    fn character(&mut self) -> Token {
        self.advance(); // 開始引用符を消費
        
        let c = self.advance().unwrap_or('\0');
        
        if self.peek() != Some('\'') {
            return self.error_token("Unterminated character literal");
        }
        
        self.advance(); // 閉じ引用符を消費
        
        self.make_token(TokenKind::Char(c))
    }
    
    /// 数値リテラルを解析
    fn number(&mut self) -> Token {
        while self.peek().map_or(false, |c| c.is_digit(10)) {
            self.advance();
        }
        
        if self.peek() == Some('.') && self.peek_next().map_or(false, |c| c.is_digit(10)) {
            self.advance(); // 小数点を消費
            
            while self.peek().map_or(false, |c| c.is_digit(10)) {
                self.advance();
            }
        }
        
        let value = self.source[self.start..self.current].parse().unwrap();
        self.make_token(TokenKind::Float(value))
    }
    
    /// 識別子を解析
    fn identifier(&mut self) -> Token {
        while self.peek().map_or(false, |c| c.is_alphanumeric() || c == '_') {
            self.advance();
        }
        
        let text = &self.source[self.start..self.current];
        let kind = match text {
            "let" => TokenKind::Let,
            "fn" => TokenKind::Fn,
            "return" => TokenKind::Return,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "match" => TokenKind::Match,
            "case" => TokenKind::Case,
            "default" => TokenKind::Default,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            _ => TokenKind::Identifier(text.to_string()),
        };
        
        self.make_token(kind)
    }
    
    /// 次の文字を取得（進めない）
    fn peek(&self) -> Option<char> {
        self.source.chars().nth(self.current)
    }
    
    /// 次の次の文字を取得
    fn peek_next(&self) -> Option<char> {
        self.source.chars().nth(self.current + 1)
    }
    
    /// 次の文字を取得（進める）
    fn advance(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.current += 1;
            self.column += 1;
        }
        c
    }
    
    /// 文字をマッチ
    fn match_char(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }
    
    /// トークンを作成
    fn make_token(&self, kind: TokenKind) -> Token {
        let lexeme = self.source[self.start..self.current].to_string();
        let range = SourceRange::new(
            SourceLocation::new(self.line, self.column - lexeme.len()),
            SourceLocation::new(self.line, self.column),
        );
        
        Token { kind, range, lexeme }
    }
    
    /// エラートークンを作成
    fn error_token(&self, message: &str) -> Token {
        let range = SourceRange::new(
            SourceLocation::new(self.line, self.column),
            SourceLocation::new(self.line, self.column),
        );
        
        Token {
            kind: TokenKind::Error,
            range,
            lexeme: message.to_string(),
        }
    }

    /// 最適化オプションを設定
    pub fn set_optimization_options(&mut self, options: LexerOptimizationOptions) {
        self.optimization_options = options;
    }

    /// 言語拡張を有効化
    pub fn enable_extension(&mut self, extension: LanguageExtension) {
        self.extensions.language_extensions.insert(extension);
    }

    /// カスタムトークンを追加
    pub fn add_custom_token(&mut self, name: String, kind: TokenKind) {
        self.extensions.custom_tokens.insert(name, kind);
    }

    /// トークン変換ルールを追加
    pub fn add_token_transform(&mut self, from: TokenKind, to: TokenKind, condition: Box<dyn Fn(&str) -> bool>) {
        self.extensions.token_transforms.push(TokenTransform {
            from,
            to,
            condition,
        });
    }

    /// 並列レキサル解析を実行
    fn lex_parallel(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        let mut current_pos = 0;

        while current_pos < self.source.len() {
            let chunk_size = self.source.len() / self.optimization_options.num_threads;
            let end_pos = (current_pos + chunk_size).min(self.source.len());

            let source_chunk = self.source[current_pos..end_pos].to_string();
            let (tx, rx) = mpsc::channel();

            self.parallel_pool.pool.execute(move || {
                let mut chunk_lexer = Lexer::new(source_chunk);
                let result = chunk_lexer.tokenize();
                tx.send(result).unwrap();
            });

            match rx.recv() {
                Ok(Ok(chunk_tokens)) => tokens.extend(chunk_tokens),
                Ok(Err(e)) => return Err(e),
                Err(_) => return Err(Error::new("Parallel lexing failed".to_string())),
            }

            current_pos = end_pos;
        }

        Ok(tokens)
    }

    /// メモ化を使用してトークンを取得
    fn next_token_memoized(&mut self) -> Token {
        let pos = self.current;

        if let Some(cached_token) = self.token_cache.tokens.get(&pos) {
            return cached_token.clone();
        }

        let token = self.next_token();
        self.token_cache.tokens.insert(pos, token.clone());
        token
    }

    /// トークンを変換
    fn transform_token(&self, token: &Token) -> Token {
        for transform in &self.extensions.token_transforms {
            if token.kind == transform.from && (transform.condition)(&token.lexeme) {
                return Token {
                    kind: transform.to.clone(),
                    range: token.range.clone(),
                    lexeme: token.lexeme.clone(),
                };
            }
        }
        token.clone()
    }

    /// 言語拡張に基づいてトークンを解析
    fn lex_with_extensions(&mut self) -> Result<Vec<Token>> {
        if self.extensions.language_extensions.contains(&LanguageExtension::QuantumTypes) {
            self.lex_quantum_tokens()
        } else if self.extensions.language_extensions.contains(&LanguageExtension::TemporalTypes) {
            self.lex_temporal_tokens()
        } else {
            self.tokenize()
        }
    }

    /// 量子トークンを解析
    fn lex_quantum_tokens(&mut self) -> Result<Vec<Token>> {
        // 量子トークンの実装
        unimplemented!("Quantum tokens not implemented yet")
    }

    /// 時相トークンを解析
    fn lex_temporal_tokens(&mut self) -> Result<Vec<Token>> {
        // 時相トークンの実装
        unimplemented!("Temporal tokens not implemented yet")
    }

    /// すべてのトークンを取得
    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        let mut token = self.next_token();

        while token.kind != TokenKind::EOF {
            tokens.push(token);
            token = self.next_token();
        }

        tokens.push(token);
        Ok(tokens)
    }
} 
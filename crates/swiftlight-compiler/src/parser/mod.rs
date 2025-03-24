use crate::core::types::{Error, Result};
use crate::core::collections::Vec;
use crate::core::source::{SourceLocation, SourceRange};
use crate::lexer::{Lexer, Token, TokenKind};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use std::thread;
use std::thread::ThreadPool;

/// 式の種類を表す列挙型
#[derive(Debug, Clone)]
pub enum ExprKind {
    // リテラル
    Literal {
        value: LiteralValue,
        location: SourceLocation,
    },
    
    // 識別子
    Identifier {
        name: String,
        location: SourceLocation,
    },
    
    // 単項演算子
    Unary {
        operator: Token,
        right: Box<Expr>,
        location: SourceLocation,
    },
    
    // 二項演算子
    Binary {
        left: Box<Expr>,
        operator: Token,
        right: Box<Expr>,
        location: SourceLocation,
    },
    
    // 関数呼び出し
    Call {
        callee: Box<Expr>,
        arguments: Vec<Expr>,
        location: SourceLocation,
    },
    
    // メンバーアクセス
    Member {
        object: Box<Expr>,
        property: Token,
        location: SourceLocation,
    },
    
    // 配列アクセス
    Index {
        array: Box<Expr>,
        index: Box<Expr>,
        location: SourceLocation,
    },

    // 量子計算関連の式
    Quantum {
        kind: QuantumExprKind,
        location: SourceLocation,
    },

    // 時相型関連の式
    Temporal {
        kind: TemporalExprKind,
        location: SourceLocation,
    },

    // エフェクトシステム関連の式
    Effect {
        kind: EffectExprKind,
        location: SourceLocation,
    },

    // リソース管理関連の式
    Resource {
        kind: ResourceExprKind,
        location: SourceLocation,
    },
}

/// 量子計算関連の式の種類
#[derive(Debug, Clone)]
pub enum QuantumExprKind {
    QubitDeclaration {
        name: String,
        initial_state: Option<Box<Expr>>,
    },
    QuantumGate {
        gate_type: String,
        qubits: Vec<Box<Expr>>,
        parameters: Vec<Expr>,
    },
    Measurement {
        qubit: Box<Expr>,
        basis: Option<String>,
    },
    Entanglement {
        qubits: Vec<Box<Expr>>,
    },
    Superposition {
        qubit: Box<Expr>,
        phase: Option<Expr>,
    },
    QuantumCircuit {
        gates: Vec<QuantumExprKind>,
    },
}

/// 時相型関連の式の種類
#[derive(Debug, Clone)]
pub enum TemporalExprKind {
    Future {
        expression: Box<Expr>,
        time: Option<Box<Expr>>,
    },
    Past {
        expression: Box<Expr>,
        time: Option<Box<Expr>>,
    },
    Always {
        expression: Box<Expr>,
        interval: Option<(Box<Expr>, Box<Expr>)>,
    },
    Eventually {
        expression: Box<Expr>,
        interval: Option<(Box<Expr>, Box<Expr>)>,
    },
    Until {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Since {
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

/// エフェクトシステム関連の式の種類
#[derive(Debug, Clone)]
pub enum EffectExprKind {
    EffectDeclaration {
        name: String,
        parameters: Vec<String>,
        return_type: Option<TypeId>,
    },
    Handle {
        effect: Box<Expr>,
        handlers: Vec<(String, Box<Expr>)>,
    },
    Resume {
        value: Box<Expr>,
    },
    Suspend {
        effect: String,
        arguments: Vec<Expr>,
    },
    Pure {
        expression: Box<Expr>,
    },
    Impure {
        expression: Box<Expr>,
    },
}

/// リソース管理関連の式の種類
#[derive(Debug, Clone)]
pub enum ResourceExprKind {
    ResourceDeclaration {
        name: String,
        type_id: TypeId,
    },
    Acquire {
        resource: Box<Expr>,
    },
    Release {
        resource: Box<Expr>,
    },
    Use {
        resource: Box<Expr>,
        body: Box<Expr>,
    },
    With {
        resources: Vec<Box<Expr>>,
        body: Box<Expr>,
    },
    Dispose {
        resource: Box<Expr>,
    },
}

/// 式を表す構造体
#[derive(Debug, Clone)]
pub struct Expr {
    /// 式の種類
    pub kind: ExprKind,
    /// 式の型（後で型推論で設定）
    pub type_id: Option<TypeId>,
}

/// リテラル値を表す列挙型
#[derive(Debug, Clone)]
pub enum LiteralValue {
    Integer(i64),
    Float(f64),
    String(String),
    Char(char),
    Boolean(bool),
    Nil,
}

/// 文の種類を表す列挙型
#[derive(Debug, Clone)]
pub enum StmtKind {
    // 式文
    Expression {
        expression: Expr,
        location: SourceLocation,
    },
    
    // 変数宣言
    Var {
        name: Token,
        initializer: Option<Expr>,
        location: SourceLocation,
    },
    
    // 関数宣言
    Function {
        name: Token,
        parameters: Vec<Token>,
        body: Vec<Stmt>,
        location: SourceLocation,
    },
    
    // ブロック
    Block {
        statements: Vec<Stmt>,
        location: SourceLocation,
    },
    
    // if文
    If {
        condition: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
        location: SourceLocation,
    },
    
    // while文
    While {
        condition: Expr,
        body: Box<Stmt>,
        location: SourceLocation,
    },
    
    // for文
    For {
        initializer: Option<Box<Stmt>>,
        condition: Option<Expr>,
        increment: Option<Expr>,
        body: Box<Stmt>,
        location: SourceLocation,
    },
    
    // return文
    Return {
        value: Option<Expr>,
        location: SourceLocation,
    },
    
    // break文
    Break {
        location: SourceLocation,
    },
    
    // continue文
    Continue {
        location: SourceLocation,
    },
}

/// 文を表す構造体
#[derive(Debug, Clone)]
pub struct Stmt {
    /// 文の種類
    pub kind: StmtKind,
}

/// 補完候補の種類
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionKind {
    /// キーワード
    Keyword,
    /// 変数
    Variable,
    /// 関数
    Function,
    /// 型
    Type,
    /// モジュール
    Module,
    /// フィールド
    Field,
    /// メソッド
    Method,
}

/// 補完候補を表す構造体
#[derive(Debug, Clone)]
pub struct CompletionItem {
    /// 候補の文字列
    pub text: String,
    /// 候補の種類
    pub kind: CompletionKind,
    /// 候補の説明
    pub description: Option<String>,
}

/// パーサーのコンテキスト
#[derive(Debug, Clone)]
pub struct ParserContext {
    /// 現在のスコープ
    scope: Vec<HashMap<String, CompletionKind>>,
    /// 現在のモジュール
    current_module: Option<String>,
    /// インポートされたモジュール
    imported_modules: HashSet<String>,
}

impl ParserContext {
    /// 新しいコンテキストを作成
    pub fn new() -> Self {
        Self {
            scope: vec![HashMap::new()],
            current_module: None,
            imported_modules: HashSet::new(),
        }
    }

    /// 新しいスコープを追加
    pub fn push_scope(&mut self) {
        self.scope.push(HashMap::new());
    }

    /// スコープを削除
    pub fn pop_scope(&mut self) {
        if self.scope.len() > 1 {
            self.scope.pop();
        }
    }

    /// 識別子を登録
    pub fn register_identifier(&mut self, name: String, kind: CompletionKind) {
        if let Some(scope) = self.scope.last_mut() {
            scope.insert(name, kind);
        }
    }

    /// 補完候補を取得
    pub fn get_completions(&self, prefix: &str) -> Vec<CompletionItem> {
        let mut completions = Vec::new();

        // 現在のスコープから補完候補を収集
        for scope in self.scope.iter().rev() {
            for (name, kind) in scope {
                if name.starts_with(prefix) {
                    completions.push(CompletionItem {
                        text: name.clone(),
                        kind: kind.clone(),
                        description: None,
                    });
                }
            }
        }

        // キーワードの補完候補を追加
        for keyword in KEYWORDS {
            if keyword.starts_with(prefix) {
                completions.push(CompletionItem {
                    text: keyword.to_string(),
                    kind: CompletionKind::Keyword,
                    description: None,
                });
            }
        }

        completions
    }
}

/// パーサーの優先順位
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
enum Precedence {
    None,
    Assignment,  // =
    Or,         // or
    And,        // and
    Equality,   // == !=
    Comparison, // < > <= >=
    Term,       // + -
    Factor,     // * /
    Unary,      // ! -
    Call,       // . ()
    Primary,
}

impl Precedence {
    /// 次の優先順位を取得
    fn next(self) -> Self {
        match self {
            Self::None => Self::Assignment,
            Self::Assignment => Self::Or,
            Self::Or => Self::And,
            Self::And => Self::Equality,
            Self::Equality => Self::Comparison,
            Self::Comparison => Self::Term,
            Self::Term => Self::Factor,
            Self::Factor => Self::Unary,
            Self::Unary => Self::Call,
            Self::Call => Self::Primary,
            Self::Primary => self,
        }
    }
}

/// 演算子の優先順位を取得
fn get_precedence(kind: &TokenKind) -> Precedence {
    match kind {
        TokenKind::Equal => Precedence::Assignment,
        TokenKind::Or => Precedence::Or,
        TokenKind::And => Precedence::And,
        TokenKind::EqualEqual | TokenKind::NotEqual => Precedence::Equality,
        TokenKind::Less | TokenKind::LessEqual |
        TokenKind::Greater | TokenKind::GreaterEqual => Precedence::Comparison,
        TokenKind::Plus | TokenKind::Minus => Precedence::Term,
        TokenKind::Star | TokenKind::Slash => Precedence::Factor,
        TokenKind::Not => Precedence::Unary,
        TokenKind::Dot | TokenKind::LeftParen => Precedence::Call,
        _ => Precedence::None,
    }
}

/// パーサーの最適化オプション
#[derive(Debug, Clone, Copy)]
pub struct ParserOptimizationOptions {
    /// メモ化を有効にするか
    pub enable_memoization: bool,
    /// 並列解析を有効にするか
    pub enable_parallel_parsing: bool,
    /// キャッシュサイズ
    pub cache_size: usize,
    /// 再帰制限
    pub recursion_limit: usize,
}

impl Default for ParserOptimizationOptions {
    fn default() -> Self {
        Self {
            enable_memoization: true,
            enable_parallel_parsing: false,
            cache_size: 1000,
            recursion_limit: 1000,
        }
    }
}

/// パーサーのメモ化キャッシュ
#[derive(Debug, Clone)]
struct MemoizationCache {
    /// 式のキャッシュ
    expression_cache: HashMap<(usize, Precedence), Result<Expr>>,
    /// 文のキャッシュ
    statement_cache: HashMap<usize, Result<Stmt>>,
    /// 宣言のキャッシュ
    declaration_cache: HashMap<usize, Result<Stmt>>,
}

impl MemoizationCache {
    fn new() -> Self {
        Self {
            expression_cache: HashMap::new(),
            statement_cache: HashMap::new(),
            declaration_cache: HashMap::new(),
        }
    }

    fn clear(&mut self) {
        self.expression_cache.clear();
        self.statement_cache.clear();
        self.declaration_cache.clear();
    }
}

/// パーサーの並列解析用のスレッドプール
#[derive(Debug)]
struct ParallelParserPool {
    /// スレッドプール
    pool: ThreadPool,
    /// タスクの結果を格納するチャネル
    result_channel: (Sender<Result<Stmt>>, Receiver<Result<Stmt>>),
}

impl ParallelParserPool {
    fn new(num_threads: usize) -> Self {
        Self {
            pool: ThreadPool::new(num_threads),
            result_channel: mpsc::channel(),
        }
    }
}

/// パーサーの拡張機能
#[derive(Debug)]
pub struct ParserExtensions {
    /// カスタム演算子の定義
    custom_operators: HashMap<String, (Precedence, Associativity)>,
    /// マクロ定義
    macros: HashMap<String, MacroDefinition>,
    /// 言語拡張の設定
    language_extensions: HashSet<LanguageExtension>,
}

/// 言語拡張の種類
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum LanguageExtension {
    /// パターンマッチング
    PatternMatching,
    /// 型推論
    TypeInference,
    /// 依存型
    DependentTypes,
    /// 量子型
    QuantumTypes,
    /// 時相型
    TemporalTypes,
    /// エフェクトシステム
    EffectSystem,
    /// リソース管理
    ResourceManagement,
}

/// マクロ定義
#[derive(Debug, Clone)]
pub struct MacroDefinition {
    /// マクロ名
    name: String,
    /// パラメータ
    parameters: Vec<String>,
    /// 展開パターン
    pattern: String,
    /// 展開結果
    expansion: String,
}

/// パーサーを表す構造体
pub struct Parser {
    /// レキサー
    lexer: Lexer,
    /// 現在のトークン
    current: Token,
    /// 前のトークン
    previous: Token,
    /// エラーの有無
    had_error: bool,
    /// パニックモード
    panic_mode: bool,
    /// パーサーのコンテキスト
    context: ParserContext,
    /// 最適化オプション
    optimization_options: ParserOptimizationOptions,
    /// メモ化キャッシュ
    memoization_cache: MemoizationCache,
    /// 並列解析用のスレッドプール
    parallel_pool: ParallelParserPool,
    /// 言語拡張機能
    extensions: ParserExtensions,
}

impl Parser {
    /// 新しいパーサーを作成
    pub fn new(source: String) -> Self {
        let mut lexer = Lexer::new(source);
        let current = lexer.next_token();
        let previous = current.clone();
        
        Self {
            lexer,
            current,
            previous,
            had_error: false,
            panic_mode: false,
            context: ParserContext::new(),
            optimization_options: ParserOptimizationOptions::default(),
            memoization_cache: MemoizationCache::new(),
            parallel_pool: ParallelParserPool::new(4),
            extensions: ParserExtensions {
                custom_operators: HashMap::new(),
                macros: HashMap::new(),
                language_extensions: HashSet::new(),
            },
        }
    }
    
    /// 次のトークンを取得
    fn advance(&mut self) {
        self.previous = self.current.clone();
        self.current = self.lexer.next_token();
    }
    
    /// トークンを消費
    fn consume(&mut self, kind: TokenKind, message: &str) -> Result<Token> {
        if self.current.kind == kind {
            let token = self.current.clone();
            self.advance();
            Ok(token)
        } else {
            self.error_at_current(message);
            Err(Error::new("Parse error".to_string()))
        }
    }
    
    /// 現在のトークンでエラーを報告
    fn error_at_current(&mut self, message: &str) {
        self.error_at(&self.current, message);
    }
    
    /// 前のトークンでエラーを報告
    fn error(&mut self, message: &str) {
        self.error_at(&self.previous, message);
    }
    
    /// トークンでエラーを報告
    fn error_at(&mut self, token: &Token, message: &str) {
        if self.panic_mode {
            return;
        }
        
        self.panic_mode = true;
        self.had_error = true;
        
        eprintln!("[line {}] Error", token.range.start.line);
        
        if token.kind == TokenKind::EOF {
            eprint!(" at end");
        } else if token.kind == TokenKind::Error {
            // エラーメッセージは既にレキサーで設定されている
        } else {
            eprint!(" at '{}'", token.lexeme);
        }
        
        eprintln!(": {}", message);
    }
    
    /// 同期
    fn synchronize(&mut self) {
        self.panic_mode = false;
        
        while self.current.kind != TokenKind::EOF {
            if self.previous.kind == TokenKind::Semicolon {
                return;
            }
            
            match self.current.kind {
                TokenKind::Fn |
                TokenKind::Let |
                TokenKind::For |
                TokenKind::If |
                TokenKind::While |
                TokenKind::Return => return,
                _ => {}
            }
            
            self.advance();
        }
    }
    
    /// 式を解析
    pub fn parse_expression(&mut self) -> Result<Expr> {
        self.parse_precedence(Precedence::Assignment)
    }
    
    /// 優先順位に基づいて式を解析
    fn parse_precedence(&mut self, precedence: Precedence) -> Result<Expr> {
        self.advance();
        let mut expr = self.unary()?;

        while precedence <= get_precedence(&self.current.kind) {
            self.advance();
            expr = self.finish_call(expr)?;
        }

        Ok(expr)
    }
    
    /// 単項式を解析
    fn unary(&mut self) -> Result<Expr> {
        if self.match_token(&[
            TokenKind::Not,
            TokenKind::Minus,
        ]) {
            let operator = self.previous.clone();
            let right = self.unary()?;
            return Ok(Expr {
                kind: ExprKind::Unary {
                    operator,
                    right: Box::new(right),
                    location: operator.range.start,
                },
                type_id: None,
            });
        }

        self.primary()
    }
    
    /// 一次式を解析
    fn primary(&mut self) -> Result<Expr> {
        if self.match_token(&[TokenKind::False]) {
            return Ok(Expr {
                kind: ExprKind::Literal {
                    value: LiteralValue::Boolean(false),
                    location: self.previous.range.start,
                },
                type_id: None,
            });
        }
        if self.match_token(&[TokenKind::True]) {
            return Ok(Expr {
                kind: ExprKind::Literal {
                    value: LiteralValue::Boolean(true),
                    location: self.previous.range.start,
                },
                type_id: None,
            });
        }
        if self.match_token(&[TokenKind::Nil]) {
            return Ok(Expr {
                kind: ExprKind::Literal {
                    value: LiteralValue::Nil,
                    location: self.previous.range.start,
                },
                type_id: None,
            });
        }
        if self.match_token(&[TokenKind::Number(0.0)]) {
            return Ok(Expr {
                kind: ExprKind::Literal {
                    value: LiteralValue::Float(self.previous.lexeme.parse().unwrap()),
                    location: self.previous.range.start,
                },
                type_id: None,
            });
        }
        if self.match_token(&[TokenKind::String("".to_string())]) {
            return Ok(Expr {
                kind: ExprKind::Literal {
                    value: LiteralValue::String(self.previous.lexeme.clone()),
                    location: self.previous.range.start,
                },
                type_id: None,
            });
        }
        if self.match_token(&[TokenKind::Identifier("".to_string())]) {
            return Ok(Expr {
                kind: ExprKind::Identifier {
                    name: self.previous.lexeme.clone(),
                    location: self.previous.range.start,
                },
                type_id: None,
            });
        }
        if self.match_token(&[TokenKind::LeftParen]) {
            self.advance();
            let expr = self.parse_expression()?;
            self.consume(TokenKind::RightParen, "Expect ')' after expression")?;
            return Ok(expr);
        }

        self.error_at_current("Expect expression");
        Err(Error::new("Parse error".to_string()))
    }
    
    /// 関数呼び出しを解析
    fn finish_call(&mut self, callee: Expr) -> Result<Expr> {
        let mut arguments = Vec::new();
        
        if self.current.kind != TokenKind::RightParen {
            loop {
                if arguments.len() >= 255 {
                    self.error_at_current("Cannot have more than 255 arguments");
                    break;
                }
                
                arguments.push(self.parse_expression()?);
                
                if self.current.kind != TokenKind::Comma {
                    break;
                }
                
                self.advance();
            }
        }
        
        let paren = self.consume(TokenKind::RightParen, "Expect ')' after arguments")?;
        
        Ok(Expr {
            kind: ExprKind::Call {
                callee: Box::new(callee),
                arguments,
                location: paren.range.start,
            },
            type_id: None,
        })
    }
    
    /// 文を解析
    pub fn parse_statement(&mut self) -> Result<Stmt> {
        match self.current.kind {
            TokenKind::Let => self.parse_var_declaration(),
            TokenKind::Fn => self.parse_function_declaration(),
            TokenKind::If => self.parse_if_statement(),
            TokenKind::While => self.parse_while_statement(),
            TokenKind::For => self.parse_for_statement(),
            TokenKind::Return => self.parse_return_statement(),
            TokenKind::Break => self.parse_break_statement(),
            TokenKind::Continue => self.parse_continue_statement(),
            TokenKind::LeftBrace => self.parse_block_statement(),
            _ => self.parse_expression_statement(),
        }
    }
    
    /// 変数宣言を解析
    fn parse_var_declaration(&mut self) -> Result<Stmt> {
        let location = self.current.range.start;
        self.advance();
        
        let name = self.consume(TokenKind::Identifier("".to_string()), "Expect variable name")?;
        
        let initializer = if self.current.kind == TokenKind::Equal {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        self.consume(TokenKind::Semicolon, "Expect ';' after variable declaration")?;
        
        Ok(Stmt {
            kind: StmtKind::Var {
                name,
                initializer,
                location,
            },
        })
    }
    
    /// 関数宣言を解析
    fn parse_function_declaration(&mut self) -> Result<Stmt> {
        let location = self.current.range.start;
        self.advance();
        
        let name = self.consume(TokenKind::Identifier("".to_string()), "Expect function name")?;
        
        self.consume(TokenKind::LeftParen, "Expect '(' after function name")?;
        
        let mut parameters = Vec::new();
        if self.current.kind != TokenKind::RightParen {
            loop {
                if parameters.len() >= 255 {
                    self.error_at_current("Cannot have more than 255 parameters");
                    break;
                }
                
                parameters.push(self.consume(TokenKind::Identifier("".to_string()), "Expect parameter name")?);
                
                if self.current.kind != TokenKind::Comma {
                    break;
                }
                
                self.advance();
            }
        }
        
        self.consume(TokenKind::RightParen, "Expect ')' after parameters")?;
        self.consume(TokenKind::LeftBrace, "Expect '{' before function body")?;
        
        let mut body = Vec::new();
        while self.current.kind != TokenKind::RightBrace &&
              self.current.kind != TokenKind::EOF {
            body.push(self.parse_declaration()?);
        }
        
        self.consume(TokenKind::RightBrace, "Expect '}' after function body")?;
        
        Ok(Stmt {
            kind: StmtKind::Function {
                name,
                parameters,
                body,
                location,
            },
        })
    }
    
    /// 宣言を解析
    fn parse_declaration(&mut self) -> Result<Stmt> {
        if self.current.kind == TokenKind::Let {
            self.parse_var_declaration()
        } else if self.current.kind == TokenKind::Fn {
            self.parse_function_declaration()
        } else {
            self.parse_statement()
        }
    }
    
    /// if文を解析
    fn parse_if_statement(&mut self) -> Result<Stmt> {
        let location = self.current.range.start;
        self.advance();
        
        self.consume(TokenKind::LeftParen, "Expect '(' after 'if'")?;
        let condition = self.parse_expression()?;
        self.consume(TokenKind::RightParen, "Expect ')' after if condition")?;
        
        self.consume(TokenKind::LeftBrace, "Expect '{' before then branch")?;
        let then_branch = Box::new(self.parse_block_statement()?);
        
        let else_branch = if self.current.kind == TokenKind::Else {
            self.advance();
            self.consume(TokenKind::LeftBrace, "Expect '{' before else branch")?;
            Some(Box::new(self.parse_block_statement()?))
        } else {
            None
        };
        
        Ok(Stmt {
            kind: StmtKind::If {
                condition,
                then_branch,
                else_branch,
                location,
            },
        })
    }
    
    /// while文を解析
    fn parse_while_statement(&mut self) -> Result<Stmt> {
        let location = self.current.range.start;
        self.advance();
        
        self.consume(TokenKind::LeftParen, "Expect '(' after 'while'")?;
        let condition = self.parse_expression()?;
        self.consume(TokenKind::RightParen, "Expect ')' after condition")?;
        
        self.consume(TokenKind::LeftBrace, "Expect '{' before while body")?;
        let body = Box::new(self.parse_block_statement()?);
        
        Ok(Stmt {
            kind: StmtKind::While {
                condition,
                body,
                location,
            },
        })
    }
    
    /// for文を解析
    fn parse_for_statement(&mut self) -> Result<Stmt> {
        let location = self.current.range.start;
        self.advance();
        
        self.consume(TokenKind::LeftParen, "Expect '(' after 'for'")?;
        
        let initializer = if self.current.kind == TokenKind::Semicolon {
            None
        } else if self.current.kind == TokenKind::Let {
            Some(Box::new(self.parse_var_declaration()?))
        } else {
            Some(Box::new(self.parse_expression_statement()?))
        };
        
        let condition = if self.current.kind != TokenKind::Semicolon {
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        self.consume(TokenKind::Semicolon, "Expect ';' after loop condition")?;
        
        let increment = if self.current.kind != TokenKind::RightParen {
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        self.consume(TokenKind::RightParen, "Expect ')' after for clauses")?;
        
        self.consume(TokenKind::LeftBrace, "Expect '{' before for body")?;
        let body = Box::new(self.parse_block_statement()?);
        
        Ok(Stmt {
            kind: StmtKind::For {
                initializer,
                condition,
                increment,
                body,
                location,
            },
        })
    }
    
    /// return文を解析
    fn parse_return_statement(&mut self) -> Result<Stmt> {
        let location = self.current.range.start;
        self.advance();
        
        let value = if self.current.kind != TokenKind::Semicolon {
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        self.consume(TokenKind::Semicolon, "Expect ';' after return value")?;
        
        Ok(Stmt {
            kind: StmtKind::Return {
                value,
                location,
            },
        })
    }
    
    /// break文を解析
    fn parse_break_statement(&mut self) -> Result<Stmt> {
        let location = self.current.range.start;
        self.advance();
        
        self.consume(TokenKind::Semicolon, "Expect ';' after 'break'")?;
        
        Ok(Stmt {
            kind: StmtKind::Break {
                location,
            },
        })
    }
    
    /// continue文を解析
    fn parse_continue_statement(&mut self) -> Result<Stmt> {
        let location = self.current.range.start;
        self.advance();
        
        self.consume(TokenKind::Semicolon, "Expect ';' after 'continue'")?;
        
        Ok(Stmt {
            kind: StmtKind::Continue {
                location,
            },
        })
    }
    
    /// ブロック文を解析
    fn parse_block_statement(&mut self) -> Result<Stmt> {
        let location = self.current.range.start;
        let mut statements = Vec::new();
        
        while self.current.kind != TokenKind::RightBrace &&
              self.current.kind != TokenKind::EOF {
            statements.push(self.parse_declaration()?);
        }
        
        self.consume(TokenKind::RightBrace, "Expect '}' after block")?;
        
        Ok(Stmt {
            kind: StmtKind::Block {
                statements,
                location,
            },
        })
    }
    
    /// 式文を解析
    fn parse_expression_statement(&mut self) -> Result<Stmt> {
        let location = self.current.range.start;
        let expression = self.parse_expression()?;
        self.consume(TokenKind::Semicolon, "Expect ';' after expression")?;
        
        Ok(Stmt {
            kind: StmtKind::Expression {
                expression,
                location,
            },
        })
    }
    
    /// プログラムを解析
    pub fn parse_program(&mut self) -> Result<Vec<Stmt>> {
        let mut statements = Vec::new();
        
        while self.current.kind != TokenKind::EOF {
            statements.push(self.parse_declaration()?);
        }
        
        Ok(statements)
    }

    /// 補完候補を取得
    pub fn get_completions(&self, position: usize) -> Vec<CompletionItem> {
        // 現在のトークンまでのコンテキストを解析
        let mut context = self.context.clone();
        let mut parser = Parser::new(self.lexer.source[..position].to_string());
        parser.context = context;
        
        // 部分的な解析を実行
        while !parser.is_at_end() {
            parser.declaration();
        }

        // 補完候補を取得
        parser.context.get_completions("")
    }

    /// 最適化オプションを設定
    pub fn set_optimization_options(&mut self, options: ParserOptimizationOptions) {
        self.optimization_options = options;
    }

    /// 言語拡張を有効化
    pub fn enable_extension(&mut self, extension: LanguageExtension) {
        self.extensions.language_extensions.insert(extension);
    }

    /// カスタム演算子を追加
    pub fn add_custom_operator(&mut self, operator: String, precedence: Precedence, associativity: Associativity) {
        self.extensions.custom_operators.insert(operator, (precedence, associativity));
    }

    /// マクロを定義
    pub fn define_macro(&mut self, name: String, parameters: Vec<String>, pattern: String, expansion: String) {
        self.extensions.macros.insert(name, MacroDefinition {
            name,
            parameters,
            pattern,
            expansion,
        });
    }

    /// 並列解析を実行
    fn parse_parallel(&mut self) -> Result<Vec<Stmt>> {
        let mut statements = Vec::new();
        let mut current_pos = 0;

        while current_pos < self.lexer.source.len() {
            let chunk_size = self.lexer.source.len() / self.optimization_options.num_threads;
            let end_pos = (current_pos + chunk_size).min(self.lexer.source.len());

            let source_chunk = self.lexer.source[current_pos..end_pos].to_string();
            let (tx, rx) = mpsc::channel();

            self.parallel_pool.pool.execute(move || {
                let mut chunk_parser = Parser::new(source_chunk);
                let result = chunk_parser.parse_program();
                tx.send(result).unwrap();
            });

            match rx.recv() {
                Ok(Ok(chunk_statements)) => statements.extend(chunk_statements),
                Ok(Err(e)) => return Err(e),
                Err(_) => return Err(Error::new("Parallel parsing failed".to_string())),
            }

            current_pos = end_pos;
        }

        Ok(statements)
    }

    /// メモ化を使用して式を解析
    fn parse_expression_memoized(&mut self, precedence: Precedence) -> Result<Expr> {
        let pos = self.current.range.start.offset;
        let cache_key = (pos, precedence);

        if let Some(cached_result) = self.memoization_cache.expression_cache.get(&cache_key) {
            return cached_result.clone();
        }

        let result = self.parse_precedence(precedence);
        self.memoization_cache.expression_cache.insert(cache_key, result.clone());
        result
    }

    /// マクロを展開
    fn expand_macro(&self, name: &str, arguments: &[String]) -> Result<String> {
        if let Some(macro_def) = self.extensions.macros.get(name) {
            let mut expansion = macro_def.expansion.clone();
            
            for (param, arg) in macro_def.parameters.iter().zip(arguments.iter()) {
                expansion = expansion.replace(&format!("${}", param), arg);
            }

            Ok(expansion)
        } else {
            Err(Error::new(format!("Macro '{}' not found", name)))
        }
    }

    /// 言語拡張に基づいて式を解析
    fn parse_expression_with_extensions(&mut self) -> Result<Expr> {
        if self.extensions.language_extensions.contains(&LanguageExtension::PatternMatching) {
            self.parse_pattern_matching()
        } else if self.extensions.language_extensions.contains(&LanguageExtension::QuantumTypes) {
            self.parse_quantum_expression()
        } else if self.extensions.language_extensions.contains(&LanguageExtension::TemporalTypes) {
            self.parse_temporal_expression()
        } else {
            self.parse_expression()
        }
    }

    /// パターンマッチング式を解析
    fn parse_pattern_matching(&mut self) -> Result<Expr> {
        // パターンマッチングの実装
        unimplemented!("Pattern matching not implemented yet")
    }

    /// 量子式を解析
    fn parse_quantum_expression(&mut self) -> Result<Expr> {
        match self.current.kind {
            TokenKind::Qubit => {
                self.advance();
                let name = self.consume(TokenKind::Identifier("".to_string()), "Expected identifier after 'qubit'")?.lexeme;
                let initial_state = if self.match_char('=') {
                    Some(Box::new(self.parse_expression()?))
                } else {
                    None
                };
                Ok(Expr {
                    kind: ExprKind::Quantum {
                        kind: QuantumExprKind::QubitDeclaration {
                            name,
                            initial_state,
                        },
                        location: self.current.range.start,
                    },
                    type_id: None,
                })
            },
            TokenKind::Gate => {
                self.advance();
                let gate_type = self.consume(TokenKind::Identifier("".to_string()), "Expected gate type")?.lexeme;
                self.consume(TokenKind::LeftParen, "Expected '(' after gate type")?;
                let mut qubits = Vec::new();
                let mut parameters = Vec::new();
                loop {
                    qubits.push(Box::new(self.parse_expression()?));
                    if !self.match_char(',') {
                        break;
                    }
                }
                if self.match_char(';') {
                    loop {
                        parameters.push(self.parse_expression()?);
                        if !self.match_char(',') {
                            break;
                        }
                    }
                }
                self.consume(TokenKind::RightParen, "Expected ')' after gate arguments")?;
                Ok(Expr {
                    kind: ExprKind::Quantum {
                        kind: QuantumExprKind::QuantumGate {
                            gate_type,
                            qubits,
                            parameters,
                        },
                        location: self.current.range.start,
                    },
                    type_id: None,
                })
            },
            // ... 他の量子計算関連の式の解析 ...
            _ => Err(Error::new("Expected quantum expression", self.current.range)),
        }
    }

    /// 時相式を解析
    fn parse_temporal_expression(&mut self) -> Result<Expr> {
        match self.current.kind {
            TokenKind::Future => {
                self.advance();
                let expression = Box::new(self.parse_expression()?);
                let time = if self.match_char('@') {
                    Some(Box::new(self.parse_expression()?))
                } else {
                    None
                };
                Ok(Expr {
                    kind: ExprKind::Temporal {
                        kind: TemporalExprKind::Future {
                            expression,
                            time,
                        },
                        location: self.current.range.start,
                    },
                    type_id: None,
                })
            },
            // ... 他の時相型関連の式の解析 ...
            _ => Err(Error::new("Expected temporal expression", self.current.range)),
        }
    }

    fn parse_effect_expression(&mut self) -> Result<Expr> {
        match self.current.kind {
            TokenKind::Effect => {
                self.advance();
                let name = self.consume(TokenKind::Identifier("".to_string()), "Expected effect name")?.lexeme;
                self.consume(TokenKind::LeftParen, "Expected '(' after effect name")?;
                let mut parameters = Vec::new();
                if !self.check(TokenKind::RightParen) {
                    loop {
                        parameters.push(self.consume(TokenKind::Identifier("".to_string()), "Expected parameter name")?.lexeme);
                        if !self.match_char(',') {
                            break;
                        }
                    }
                }
                self.consume(TokenKind::RightParen, "Expected ')' after parameters")?;
                let return_type = if self.match_char(':') {
                    Some(self.parse_type()?)
                } else {
                    None
                };
                Ok(Expr {
                    kind: ExprKind::Effect {
                        kind: EffectExprKind::EffectDeclaration {
                            name,
                            parameters,
                            return_type,
                        },
                        location: self.current.range.start,
                    },
                    type_id: None,
                })
            },
            // ... 他のエフェクトシステム関連の式の解析 ...
            _ => Err(Error::new("Expected effect expression", self.current.range)),
        }
    }

    fn parse_resource_expression(&mut self) -> Result<Expr> {
        match self.current.kind {
            TokenKind::Resource => {
                self.advance();
                let name = self.consume(TokenKind::Identifier("".to_string()), "Expected resource name")?.lexeme;
                self.consume(TokenKind::Colon, "Expected ':' after resource name")?;
                let type_id = self.parse_type()?;
                Ok(Expr {
                    kind: ExprKind::Resource {
                        kind: ResourceExprKind::ResourceDeclaration {
                            name,
                            type_id,
                        },
                        location: self.current.range.start,
                    },
                    type_id: None,
                })
            },
            // ... 他のリソース管理関連の式の解析 ...
            _ => Err(Error::new("Expected resource expression", self.current.range)),
        }
    }
} 
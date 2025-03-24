//! # 構文解析器
//! 
//! SwiftLight言語の構文解析を担当するモジュールです。
//! 字句解析器からのトークン列を受け取り、言語の文法に基づいて
//! 抽象構文木（AST）を構築します。

use std::iter::Peekable;
use std::slice::Iter;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::frontend::ast::{
    self, BinaryOperator, EnumVariant, Expression, ExpressionKind,
    Identifier, Literal, LiteralKind, NodeId, Parameter, Program,
    Statement, StatementKind, StructField, TraitMethod, TypeAnnotation,
    UnaryOperator,
};
use crate::frontend::error::{CompilerError, ErrorKind, Result, SourceLocation};
use crate::frontend::lexer::{Token, TokenKind, Lexer};
use crate::frontend::source_map::SourceMap;
use crate::frontend::parser::error_recovery::{ErrorRecovery, RecoveryMode};

pub mod error;
pub mod expression;
pub mod statement;
pub mod declaration;
pub mod types;
pub mod ast;
pub mod error_recovery;
pub mod grammar;

/// インクリメンタルパースのための差分情報
#[derive(Debug, Clone)]
pub struct SourceDiff {
    /// 変更開始位置（バイトオフセット）
    pub start: usize,
    /// 変更前の長さ
    pub old_len: usize,
    /// 変更後の長さ
    pub new_len: usize,
    /// 変更されたテキスト
    pub new_text: String,
}

/// パースキャッシュ
#[derive(Debug, Default)]
pub struct ParseCache {
    /// 前回のパース結果
    ast_cache: HashMap<String, Arc<ast::Program>>,
    /// ファイルごとのトークンキャッシュ
    token_cache: HashMap<String, Vec<Token>>,
    /// パース時間の統計
    parse_times: HashMap<String, std::time::Duration>,
}

impl ParseCache {
    /// 新しいパースキャッシュを作成
    pub fn new() -> Self {
        Self {
            ast_cache: HashMap::new(),
            token_cache: HashMap::new(),
            parse_times: HashMap::new(),
        }
    }

    /// ASTをキャッシュに保存
    pub fn cache_ast(&mut self, file_path: &str, ast: Arc<ast::Program>) {
        self.ast_cache.insert(file_path.to_string(), ast);
    }

    /// トークンをキャッシュに保存
    pub fn cache_tokens(&mut self, file_path: &str, tokens: Vec<Token>) {
        self.token_cache.insert(file_path.to_string(), tokens);
    }

    /// パース時間を記録
    pub fn record_parse_time(&mut self, file_path: &str, duration: std::time::Duration) {
        self.parse_times.insert(file_path.to_string(), duration);
    }

    /// キャッシュ済みのASTを取得
    pub fn get_cached_ast(&self, file_path: &str) -> Option<Arc<ast::Program>> {
        self.ast_cache.get(file_path).cloned()
    }

    /// キャッシュ済みのトークンを取得
    pub fn get_cached_tokens(&self, file_path: &str) -> Option<&Vec<Token>> {
        self.token_cache.get(file_path)
    }

    /// パース時間の統計を取得
    pub fn get_parse_times(&self) -> &HashMap<String, std::time::Duration> {
        &self.parse_times
    }

    /// キャッシュをクリア
    pub fn clear(&mut self) {
        self.ast_cache.clear();
        self.token_cache.clear();
        // 統計情報は残す
    }

    /// 特定のファイルのキャッシュをクリア
    pub fn invalidate(&mut self, file_path: &str) {
        self.ast_cache.remove(file_path);
        self.token_cache.remove(file_path);
    }
}

/// パーサー
pub struct Parser {
    /// トークン列
    tokens: Vec<Token>,
    /// 現在のトークンインデックス
    current: usize,
    /// パースエラーの収集
    errors: Vec<CompilerError>,
    /// エラー回復モード
    panic_mode: bool,
    /// パニックモード中にスキップされたトークン数
    skipped_tokens: usize,
    /// キャッシュ
    cache: Arc<Mutex<ParseCache>>,
    /// 最後にパースしたファイルパス
    last_file_path: Option<String>,
    /// エラー回復機構
    error_recovery: ErrorRecovery,
}

impl Parser {
    /// 新しいパーサーを作成
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            current: 0,
            errors: Vec::new(),
            panic_mode: false,
            skipped_tokens: 0,
            cache: Arc::new(Mutex::new(ParseCache::new())),
            last_file_path: None,
            error_recovery: ErrorRecovery::new(),
        }
    }

    /// キャッシュ付きパーサーを作成
    pub fn with_cache(tokens: Vec<Token>, cache: Arc<Mutex<ParseCache>>) -> Self {
        Self {
            tokens,
            current: 0,
            errors: Vec::new(),
            panic_mode: false,
            skipped_tokens: 0,
            cache,
            last_file_path: None,
            error_recovery: ErrorRecovery::new(),
        }
    }

    /// インクリメンタルパースを行う
    pub fn parse_incremental(&mut self, source: &str, file_path: &str, diff: &SourceDiff) -> Result<Arc<ast::Program>> {
        let start_time = Instant::now();
        self.last_file_path = Some(file_path.to_string());

        // キャッシュからトークンと前回のASTを取得
        let (cached_tokens, cached_ast) = {
            let cache = self.cache.lock().unwrap();
            (
                cache.get_cached_tokens(file_path).cloned(),
                cache.get_cached_ast(file_path).clone()
            )
        };
        
        // 変更がなければキャッシュからASTを返す
        if diff.old_len == 0 && diff.new_len == 0 && cached_ast.is_some() {
            return Ok(cached_ast.unwrap());
        }
        
        // トークンの再利用または新規生成
        let tokens = if let Some(cached_tokens) = cached_tokens {
            // 差分に基づいてトークンを更新
            self.update_tokens(source, file_path, cached_tokens, diff)?
        } else {
            // 初回パースまたはキャッシュミスの場合は全体をトークナイズ
            let lexer = Lexer::new(source, file_path);
            lexer.collect::<Result<Vec<_>>>()?
        };
        
        // トークンをキャッシュに保存
        {
            let mut cache = self.cache.lock().unwrap();
            cache.cache_tokens(file_path, tokens.clone());
        }
        
        // トークンをパーサーにセット
        self.tokens = tokens;
        self.current = 0;
        self.errors.clear();
        self.panic_mode = false;
        self.skipped_tokens = 0;
        // エラー回復機構をリセット
        self.error_recovery = ErrorRecovery::new();
        
        // プログラムをパース
        let program = self.parse_program()?;
        let ast = Arc::new(program);
        
        // パース結果と時間をキャッシュに保存
        {
            let mut cache = self.cache.lock().unwrap();
            cache.cache_ast(file_path, ast.clone());
            cache.record_parse_time(file_path, start_time.elapsed());
        }
        
        Ok(ast)
    }

    /// 差分に基づいてトークンを更新
    fn update_tokens(&self, source: &str, file_path: &str, old_tokens: Vec<Token>, diff: &SourceDiff) -> Result<Vec<Token>> {
        // 差分がない場合は古いトークンをそのまま返す
        if diff.old_len == 0 && diff.new_len == 0 {
            return Ok(old_tokens);
        }
        
        // 変更された範囲に関連するトークンを特定
        let start_token_idx = self.find_token_at_position(&old_tokens, diff.start);
        let end_token_idx = self.find_token_at_position(&old_tokens, diff.start + diff.old_len);
        
        // 変更前後のバッファ領域（コンテキスト）を追加
        // 変更の影響が及ぶ可能性のある範囲を広めに取る
        let context_buffer = 5; // 前後5トークンをバッファとして追加
        let context_start = if start_token_idx > context_buffer {
            start_token_idx - context_buffer
        } else {
            0
        };
        
        let context_end = (end_token_idx + context_buffer).min(old_tokens.len());
        
        // 変更領域の前のトークン
        let prefix_tokens = old_tokens.iter().take(context_start).cloned().collect::<Vec<_>>();
        
        // 再解析する範囲を決定
        let reparse_start = if context_start > 0 {
            old_tokens[context_start].location.start
        } else {
            0
        };
        
        let reparse_end = if context_end < old_tokens.len() {
            old_tokens[context_end - 1].location.end
        } else {
            source.len()
        };
        
        // 変更を含むコンテキスト範囲のテキストを解析
        let changed_source = &source[reparse_start..reparse_end];
        let lexer = Lexer::new(changed_source, file_path);
        let mut new_tokens = lexer.collect::<Result<Vec<_>>>()?;
        
        // 位置情報を修正
        self.adjust_token_locations(&mut new_tokens, reparse_start);
        
        // 変更領域の後のトークン
        let mut suffix_tokens = old_tokens.iter().skip(context_end).cloned().collect::<Vec<_>>();
        
        // 位置情報を修正（変更によるオフセット）
        let offset = (reparse_end + new_tokens.last().map_or(0, |t| t.location.end - reparse_start)) as isize
                    - old_tokens[context_end - 1].location.end as isize;
        
        if offset != 0 {
            self.adjust_token_locations_with_offset(&mut suffix_tokens, offset);
        }
        
        // 全てのトークンを結合
        let mut result = Vec::new();
        result.extend(prefix_tokens);
        result.extend(new_tokens);
        result.extend(suffix_tokens);
        
        // トークン列の整合性チェック（デバッグ用）
        #[cfg(debug_assertions)]
        {
            for i in 1..result.len() {
                let prev = &result[i-1];
                let curr = &result[i];
                if prev.location.end > curr.location.start {
                    // トークンの位置が重複している場合は警告
                    eprintln!("Warning: Token positions overlap! prev={:?}, curr={:?}", prev, curr);
                }
            }
        }
        
        Ok(result)
    }

    /// 指定位置を含むトークンのインデックスを検索
    fn find_token_at_position(&self, tokens: &[Token], position: usize) -> usize {
        for (i, token) in tokens.iter().enumerate() {
            let start = token.location.start;
            let end = token.location.end;
            
            if position >= start && position <= end {
                return i;
            }
            
            // 位置がトークンの終端を超えた場合、次のトークンを検索
            if position < start {
                return i;
            }
        }
        
        // 位置がすべてのトークンの後にある場合
        tokens.len()
    }

    /// トークンの位置情報を調整（絶対位置）
    fn adjust_token_locations(&self, tokens: &mut [Token], base_offset: usize) {
        for token in tokens {
            let start = token.location.start + base_offset;
            let end = token.location.end + base_offset;
            
            // 新しい位置情報でトークンを更新
            token.location.start = start;
            token.location.end = end;
            // 注意: 行と列の情報は正確でない可能性があるが、
            // 多くの操作ではスタート位置のバイトオフセットが重要
        }
    }

    /// トークンの位置情報を調整（相対オフセット）
    fn adjust_token_locations_with_offset(&self, tokens: &mut [Token], offset: isize) {
        for token in tokens {
            if offset > 0 {
                token.location.start = token.location.start.checked_add(offset as usize).unwrap_or(token.location.start);
                token.location.end = token.location.end.checked_add(offset as usize).unwrap_or(token.location.end);
            } else if offset < 0 {
                let abs_offset = offset.abs() as usize;
                token.location.start = token.location.start.checked_sub(abs_offset).unwrap_or(token.location.start);
                token.location.end = token.location.end.checked_sub(abs_offset).unwrap_or(token.location.end);
            }
        }
    }

    /// パース結果の統計情報を取得
    pub fn get_statistics(&self) -> HashMap<String, String> {
        let mut stats = HashMap::new();
        
        if let Ok(cache) = self.cache.lock() {
            // パース時間の統計
            for (file, duration) in cache.get_parse_times() {
                stats.insert(
                    format!("parse_time:{}", file),
                    format!("{:.2?}", duration)
                );
            }
            
            // ファイルごとのAST統計
            for (file, ast) in &cache.ast_cache {
                stats.insert(
                    format!("ast_nodes:{}", file),
                    format!("{}", self.count_ast_nodes(ast))
                );
            }
            
            // トークン数
            for (file, tokens) in &cache.token_cache {
                stats.insert(
                    format!("tokens:{}", file),
                    format!("{}", tokens.len())
                );
            }
        }
        
        stats
    }

    /// AST内のノード数をカウント（簡易実装）
    fn count_ast_nodes(&self, program: &ast::Program) -> usize {
        // 実際にはASTをトラバースしてすべてのノードをカウントする必要がある
        // ここでは簡易的に宣言の数を返す
        program.declarations.len()
    }

    /// エラー発生時に回復して次の有効な構文境界まで進む
    fn synchronize(&mut self) {
        // 既にパニックモード中なら何もしない（再帰を防止）
        if self.panic_mode {
            return;
        }
        
        self.panic_mode = true;
        self.skipped_tokens = 0;
        
        // 現在のトークンに基づいて適切な回復モードを選択
        let recovery_mode = match self.peek_kind() {
            Some(TokenKind::LeftBrace) => RecoveryMode::SkipToBlockEnd,
            Some(TokenKind::Semicolon) => {
                // セミコロンまで既に来ている場合は、それを消費して回復完了
                self.advance();
                self.panic_mode = false;
                return;
            },
            Some(TokenKind::KeywordFn) | Some(TokenKind::KeywordLet) | 
            Some(TokenKind::KeywordVar) | Some(TokenKind::KeywordConst) |
            Some(TokenKind::KeywordStruct) | Some(TokenKind::KeywordEnum) |
            Some(TokenKind::KeywordTrait) | Some(TokenKind::KeywordImpl) |
            Some(TokenKind::KeywordType) | Some(TokenKind::KeywordImport) |
            Some(TokenKind::KeywordModule) => {
                // 宣言の開始点なら回復完了
                self.panic_mode = false;
                return;
            },
            _ => RecoveryMode::SkipToEndOfStatement,
        };
        
        // エラー回復モードを設定
        self.error_recovery.set_panic_mode(true);
        
        // 同期ポイントが見つかるまでトークンを消費
        while !self.is_at_end() {
            if let Some(token) = self.peek() {
                // 現在のトークンで回復できるか試みる
                if self.error_recovery.try_recover(token) {
                    break;
                }
                
                // 特定の同期ポイントに達したかチェック
                match token.kind {
                    TokenKind::Semicolon => {
                        self.advance(); // セミコロンを消費
                        break;
                    }
                    TokenKind::RightBrace => {
                        // ブロック終了なら回復（消費しない）
                        break;
                    }
                    TokenKind::KeywordFn | TokenKind::KeywordLet | 
                    TokenKind::KeywordVar | TokenKind::KeywordConst |
                    TokenKind::KeywordStruct | TokenKind::KeywordEnum |
                    TokenKind::KeywordTrait | TokenKind::KeywordImpl |
                    TokenKind::KeywordType | TokenKind::KeywordImport |
                    TokenKind::KeywordModule => {
                        // 新しい宣言の開始なら回復（消費しない）
                        break;
                    }
                    _ => {
                        // 同期ポイントでなければトークンを飛ばす
                        self.advance();
                        self.skipped_tokens += 1;
                    }
                }
            } else {
                break;
            }
        }
        
        // 回復完了
        self.panic_mode = false;
        self.error_recovery.set_panic_mode(false);
        
        // スキップされたトークン数をログに記録（開発時のデバッグ用）
        if self.skipped_tokens > 0 {
            // println!("Skipped {} tokens during error recovery", self.skipped_tokens);
            self.skipped_tokens = 0;
        }
    }

    /// プログラム（複数の文）を解析
    pub fn parse_program(&mut self) -> Result<Program> {
        // ファイルの終端に達するまで文をパース
        while !self.is_at_end() {
            match self.parse_declaration() {
                Ok(stmt) => self.program.add_statement(stmt),
                Err(err) => {
                    self.errors.push(err);
                    self.synchronize(); // エラーから回復
                }
            }
        }
        
        // エラーがあれば最初のエラーを返す、なければプログラムを返す
        if let Some(err) = self.errors.first() {
            Err(err.clone())
        } else {
            Ok(std::mem::take(&mut self.program))
        }
    }
    
    /// 宣言を解析（文の一種）
    fn parse_declaration(&mut self) -> Result<Statement> {
        // トークンの種類に基づいて適切な解析メソッドを呼び出す
        match self.peek_kind() {
            Some(TokenKind::Let) => self.parse_variable_declaration(),
            Some(TokenKind::Const) => self.parse_constant_declaration(),
            Some(TokenKind::Func) => self.parse_function_declaration(),
            Some(TokenKind::Struct) => self.parse_struct_declaration(),
            Some(TokenKind::Enum) => self.parse_enum_declaration(),
            Some(TokenKind::Trait) => self.parse_trait_declaration(),
            Some(TokenKind::Impl) => self.parse_impl_declaration(),
            Some(TokenKind::Import) => self.parse_import_declaration(),
            _ => self.parse_statement(),
        }
    }
    
    /// 変数宣言を解析
    pub fn parse_variable_declaration(&mut self) -> Result<Statement> {
        let start_loc = self.peek().map(|t| t.location.clone());
        self.consume(TokenKind::Let, "変数宣言には'let'キーワードが必要です")?;
        
        let name = match self.consume_identifier() {
            Some(ident) => ident,
            None => return Err(CompilerError::syntax_error(
                "変数名の識別子が必要です",
                self.peek().map(|t| t.location.clone()),
            )),
        };
        
        let type_annotation = if self.match_token(&TokenKind::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        
        let initializer = if self.match_token(&TokenKind::Equal) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        
        self.consume(&TokenKind::Semicolon, "変数宣言の後にはセミコロンが必要です")?;
        
        let node_id = self.next_id();
        let end_loc = self.peek().map(|t| t.location.clone()).or(start_loc.clone());
        let location = start_loc.zip(end_loc).map(|(start, end)| SourceLocation::new(start.file_name.clone(), start.line, start.column, end.line, end.column));
        
        Ok(Statement {
            id: node_id,
            location,
            kind: StatementKind::VariableDeclaration {
                name,
                type_annotation,
                initializer,
            },
        })
    }
    
    /// 定数宣言を解析
    pub fn parse_constant_declaration(&mut self) -> Result<Statement> {
        let start_loc = self.peek().map(|t| t.location.clone());
        self.consume(&TokenKind::Const, "定数宣言には'const'キーワードが必要です")?;
        
        let name = match self.consume_identifier() {
            Some(ident) => ident,
            None => return Err(CompilerError::syntax_error(
                "定数名の識別子が必要です",
                self.peek().map(|t| t.location.clone()),
            )),
        };
        
        let type_annotation = if self.match_token(&TokenKind::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        
        self.consume(&TokenKind::Equal, "定数宣言には初期化子が必要です")?;
        let value = Box::new(self.parse_expression()?);
        
        self.consume(&TokenKind::Semicolon, "定数宣言の後にはセミコロンが必要です")?;
        
        let node_id = self.next_id();
        let end_loc = self.peek().map(|t| t.location.clone()).or(start_loc.clone());
        let location = start_loc.zip(end_loc).map(|(start, end)| SourceLocation::new(start.file_name.clone(), start.line, start.column, end.line, end.column));
        
        Ok(Statement {
            id: node_id,
            location,
            kind: StatementKind::ConstantDeclaration {
                name,
                type_annotation,
                value,
            },
        })
    }
    
    /// 関数宣言を解析
    pub fn parse_function_declaration(&mut self) -> Result<Statement> {
        let start_loc = self.peek().map(|t| t.location.clone());
        self.consume(&TokenKind::Func, "関数宣言には'func'キーワードが必要です")?;
        
        let name = match self.consume_identifier() {
            Some(ident) => ident,
            None => return Err(CompilerError::syntax_error(
                "関数名の識別子が必要です",
                self.peek().map(|t| t.location.clone()),
            )),
        };
        
        self.consume(&TokenKind::LeftParen, "関数パラメータリストには'('が必要です")?;
        let parameters = self.parse_parameters()?;
        self.consume(&TokenKind::RightParen, "関数パラメータリストの終わりには')'が必要です")?;
        
        let return_type = if self.match_token(&TokenKind::Arrow) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        
        let body = self.parse_block_statement()?;
        
        let node_id = self.next_id();
        let end_loc = body.location.clone().or_else(|| self.peek().map(|t| t.location.clone())).or(start_loc.clone());
        let location = start_loc.zip(end_loc).map(|(start, end)| SourceLocation::new(start.file_name.clone(), start.line, start.column, end.line, end.column));
        
        Ok(Statement {
            id: node_id,
            location,
            kind: StatementKind::FunctionDeclaration {
                name,
                parameters,
                return_type,
                body: Box::new(body),
            },
        })
    }
    
    /// パラメータリストを解析
    fn parse_parameters(&mut self) -> Result<Vec<Parameter>> {
        let mut parameters = Vec::new();
        
        if !self.check(&TokenKind::RightParen) {
            loop {
                let param_name = match self.consume_identifier() {
                    Some(ident) => ident,
                    None => return Err(CompilerError::syntax_error(
                        "パラメータ名の識別子が必要です",
                        self.peek().map(|t| t.location.clone()),
                    )),
                };
                
                self.consume(&TokenKind::Colon, "パラメータには型注釈が必要です")?;
                let param_type = self.parse_type_annotation()?;
                
                // パラメータの位置情報を設定
                let param_start_loc = param_name.location.clone();
                let param_end_loc = param_type.location.clone().or_else(|| self.current_location());
                let param_location = param_start_loc.zip(param_end_loc).map(|(start, end)| 
                    SourceLocation::new(start.file_name.clone(), start.line, start.column, end.line, end.column));
                
                parameters.push(Parameter {
                    id: self.next_id(),
                    name: param_name,
                    type_annotation: param_type,
                    location: param_location,
                });
                
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
        }
        
        Ok(parameters)
    }
    
    /// 構造体宣言を解析
    pub fn parse_struct_declaration(&mut self) -> Result<Statement> {
        let start_loc = self.peek().map(|t| t.location.clone());
        self.consume(&TokenKind::Struct, "構造体宣言には'struct'キーワードが必要です")?;
        
        let name = match self.consume_identifier() {
            Some(ident) => ident,
            None => return Err(CompilerError::syntax_error(
                "構造体名の識別子が必要です",
                self.peek().map(|t| t.location.clone()),
            )),
        };
        
        self.consume(&TokenKind::LeftBrace, "構造体本体には'{'が必要です")?;
        let fields = self.parse_struct_fields()?;
        self.consume(&TokenKind::RightBrace, "構造体本体の終わりには'}'が必要です")?;
        
        let node_id = self.next_id();
        let end_loc = self.peek().map(|t| t.location.clone()).or(start_loc.clone());
        let location = start_loc.zip(end_loc).map(|(start, end)| SourceLocation::new(start.file_name.clone(), start.line, start.column, end.line, end.column));
        
        Ok(Statement {
            id: node_id,
            location,
            kind: StatementKind::StructDeclaration {
                name,
                fields,
            },
        })
    }
    
    /// 構造体フィールドリストを解析
    fn parse_struct_fields(&mut self) -> Result<Vec<StructField>> {
        let mut fields = Vec::new();
        
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            let field_name = match self.consume_identifier() {
                Some(ident) => ident,
                None => return Err(CompilerError::syntax_error(
                    "フィールド名の識別子が必要です",
                    self.peek().map(|t| t.location.clone()),
                )),
            };
            
            self.consume(&TokenKind::Colon, "フィールドには型注釈が必要です")?;
            let field_type = self.parse_type_annotation()?;
            self.consume(&TokenKind::Semicolon, "フィールド宣言の後にはセミコロンが必要です")?;
            
            fields.push(StructField {
                id: self.next_id(),
                name: field_name,
                type_annotation: field_type,
                location: self.current_location(),
            });
        }
        
        Ok(fields)
    }
    
    /// 列挙型宣言を解析
    pub fn parse_enum_declaration(&mut self) -> Result<Statement> {
        let start_loc = self.peek().map(|t| t.location.clone());
        self.consume(&TokenKind::Enum, "列挙型宣言には'enum'キーワードが必要です")?;
        
        let name = match self.consume_identifier() {
            Some(ident) => ident,
            None => return Err(CompilerError::syntax_error(
                "列挙型名の識別子が必要です",
                self.peek().map(|t| t.location.clone()),
            )),
        };
        
        self.consume(&TokenKind::LeftBrace, "列挙型本体には'{'が必要です")?;
        let variants = self.parse_enum_variants()?;
        self.consume(&TokenKind::RightBrace, "列挙型本体の終わりには'}'が必要です")?;
        
        let node_id = self.next_id();
        let end_loc = self.peek().map(|t| t.location.clone()).or(start_loc.clone());
        let location = start_loc.zip(end_loc).map(|(start, end)| SourceLocation::new(start.file_name.clone(), start.line, start.column, end.line, end.column));
        
        Ok(Statement {
            id: node_id,
            location,
            kind: StatementKind::EnumDeclaration {
                name,
                variants,
            },
        })
    }
    
    /// 列挙型バリアントリストを解析
    fn parse_enum_variants(&mut self) -> Result<Vec<EnumVariant>> {
        let mut variants = Vec::new();
        
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            let variant_name = match self.consume_identifier() {
                Some(ident) => ident,
                None => return Err(CompilerError::syntax_error(
                    "バリアント名の識別子が必要です",
                    self.peek().map(|t| t.location.clone()),
                )),
            };
            
            let associated_data = if self.match_token(&TokenKind::LeftParen) {
                let fields = self.parse_enum_variant_fields()?;
                self.consume(&TokenKind::RightParen, "バリアントフィールドリストの終わりには')'が必要です")?;
                Some(fields)
            } else {
                None
            };
            
            if self.match_token(&TokenKind::Equal) {
                let discriminant = self.parse_expression()?;
                variants.push(EnumVariant {
                    id: self.next_id(),
                    name: variant_name,
                    associated_data,
                    discriminant: Some(Box::new(discriminant)),
                    location: self.current_location(),
                });
            } else {
                variants.push(EnumVariant {
                    id: self.next_id(),
                    name: variant_name,
                    associated_data,
                    discriminant: None,
                    location: self.current_location(),
                });
            }
            
            if !self.match_token(&TokenKind::Comma) && !self.check(&TokenKind::RightBrace) {
                return Err(CompilerError::syntax_error(
                    "バリアント宣言の後にはカンマまたは'}'が必要です",
                    self.peek().map(|t| t.location.clone()),
                ));
            }
        }
        
        Ok(variants)
    }
    
    /// 列挙型バリアントのフィールドリストを解析
    fn parse_enum_variant_fields(&mut self) -> Result<Vec<TypeAnnotation>> {
        let mut fields = Vec::new();
        
        if !self.check(&TokenKind::RightParen) {
            loop {
                let field_type = self.parse_type_annotation()?;
                fields.push(field_type);
                
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
        }
        
        Ok(fields)
    }
    
    /// トレイト宣言を解析
    pub fn parse_trait_declaration(&mut self) -> Result<Statement> {
        let start_loc = self.peek().map(|t| t.location.clone());
        self.consume(&TokenKind::Trait, "トレイト宣言には'trait'キーワードが必要です")?;
        
        let name = match self.consume_identifier() {
            Some(ident) => ident,
            None => return Err(CompilerError::syntax_error(
                "トレイト名の識別子が必要です",
                self.peek().map(|t| t.location.clone()),
            )),
        };
        
        self.consume(&TokenKind::LeftBrace, "トレイト本体には'{'が必要です")?;
        let methods = self.parse_trait_methods()?;
        self.consume(&TokenKind::RightBrace, "トレイト本体の終わりには'}'が必要です")?;
        
        let node_id = self.next_id();
        let end_loc = self.peek().map(|t| t.location.clone()).or(start_loc.clone());
        let location = start_loc.zip(end_loc).map(|(start, end)| SourceLocation::new(start.file_name.clone(), start.line, start.column, end.line, end.column));
        
        Ok(Statement {
            id: node_id,
            location,
            kind: StatementKind::TraitDeclaration {
                name,
                methods,
            },
        })
    }
    
    /// トレイトメソッドリストを解析
    fn parse_trait_methods(&mut self) -> Result<Vec<TraitMethod>> {
        let mut methods = Vec::new();
        
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            let method_name = match self.consume_identifier() {
                Some(ident) => ident,
                None => return Err(CompilerError::syntax_error(
                    "メソッド名の識別子が必要です",
                    self.peek().map(|t| t.location.clone()),
                )),
            };
            
            self.consume(&TokenKind::LeftParen, "メソッドパラメータリストには'('が必要です")?;
            let parameters = self.parse_parameters()?;
            self.consume(&TokenKind::RightParen, "メソッドパラメータリストの終わりには')'が必要です")?;
            
            let return_type = if self.match_token(&TokenKind::Arrow) {
                Some(self.parse_type_annotation()?)
            } else {
                None
            };
            
            let body = if self.check(&TokenKind::LeftBrace) {
                Some(Box::new(self.parse_block_statement()?))
            } else {
                self.consume(&TokenKind::Semicolon, "メソッド宣言の後にはセミコロンが必要です")?;
                None
            };
            
            methods.push(TraitMethod {
                id: self.next_id(),
                name: method_name,
                parameters,
                return_type,
                body,
                location: self.current_location(),
            });
        }
        
        Ok(methods)
    }
    
    /// 実装宣言を解析
    pub fn parse_impl_declaration(&mut self) -> Result<Statement> {
        let start_loc = self.peek().map(|t| t.location.clone());
        self.consume(&TokenKind::Impl, "実装宣言には'impl'キーワードが必要です")?;
        
        let trait_name = if !self.check(&TokenKind::For) {
            match self.consume_identifier() {
                Some(ident) => Some(ident),
                None => return Err(CompilerError::syntax_error(
                    "トレイト名の識別子が必要です",
                    self.peek().map(|t| t.location.clone()),
                )),
            }
        } else {
            None
        };
        
        if trait_name.is_some() {
            self.consume(&TokenKind::For, "トレイト実装には'for'キーワードが必要です")?;
        }
        
        let target_type = self.parse_type_annotation()?;
        
        self.consume(&TokenKind::LeftBrace, "実装本体には'{'が必要です")?;
        let methods = self.parse_impl_methods()?;
        self.consume(&TokenKind::RightBrace, "実装本体の終わりには'}'が必要です")?;
        
        let node_id = self.next_id();
        let end_loc = self.peek().map(|t| t.location.clone()).or(start_loc.clone());
        let location = start_loc.zip(end_loc).map(|(start, end)| SourceLocation::new(start.file_name.clone(), start.line, start.column, end.line, end.column));
        
        Ok(Statement {
            id: node_id,
            location,
            kind: StatementKind::ImplDeclaration {
                trait_name,
                target_type,
                methods,
            },
        })
    }
    
    /// 実装メソッドリストを解析
    fn parse_impl_methods(&mut self) -> Result<Vec<TraitMethod>> {
        self.parse_trait_methods()
    }
    
    /// インポート宣言を解析
    pub fn parse_import_declaration(&mut self) -> Result<Statement> {
        let start_loc = self.peek().map(|t| t.location.clone());
        self.consume(&TokenKind::Import, "インポート宣言には'import'キーワードが必要です")?;
        
        let path = self.parse_import_path()?;
        self.consume(&TokenKind::Semicolon, "インポート宣言の後にはセミコロンが必要です")?;
        
        let node_id = self.next_id();
        let end_loc = self.peek().map(|t| t.location.clone()).or(start_loc.clone());
        let location = start_loc.zip(end_loc).map(|(start, end)| SourceLocation::new(start.file_name.clone(), start.line, start.column, end.line, end.column));
        
        Ok(Statement {
            id: node_id,
            location,
            kind: StatementKind::ImportDeclaration {
                path,
            },
        })
    }
    
    /// インポートパスを解析
    fn parse_import_path(&mut self) -> Result<Vec<Identifier>> {
        let mut path = Vec::new();
        
        loop {
            let segment = match self.consume_identifier() {
                Some(ident) => ident,
                None => return Err(CompilerError::syntax_error(
                    "インポートパスの識別子が必要です",
                    self.peek().map(|t| t.location.clone()),
                )),
            };
            
            path.push(segment);
            
            if !self.match_token(&TokenKind::Dot) {
                break;
            }
        }
        
        Ok(path)
    }
    
    /// 文を解析
    fn parse_statement(&mut self) -> Result<Statement> {
        // トークンの種類に基づいて適切な解析メソッドを呼び出す
        match self.peek_kind() {
            Some(TokenKind::LeftBrace) => self.parse_block_statement(),
            Some(TokenKind::If) => self.parse_if_statement(),
            Some(TokenKind::While) => self.parse_while_statement(),
            Some(TokenKind::For) => self.parse_for_statement(),
            Some(TokenKind::Return) => self.parse_return_statement(),
            Some(TokenKind::Break) => self.parse_break_statement(),
            Some(TokenKind::Continue) => self.parse_continue_statement(),
            _ => self.parse_expression_statement(),
        }
    }
    
    /// ブロック文を解析
    fn parse_block_statement(&mut self) -> Result<Statement> {
        let start_loc = self.peek().map(|t| t.location.clone());
        self.consume(&TokenKind::LeftBrace, "ブロックには'{'が必要です")?;
        
        let mut statements = Vec::new();
        
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            match self.parse_declaration() {
                Ok(stmt) => statements.push(stmt),
                Err(err) => {
                    self.errors.push(err);
                    self.synchronize();
                }
            }
        }
        
        self.consume(&TokenKind::RightBrace, "ブロックの終わりには'}'が必要です")?;
        
        let node_id = self.next_id();
        let end_loc = self.peek().map(|t| t.location.clone()).or(start_loc.clone());
        let location = start_loc.zip(end_loc).map(|(start, end)| SourceLocation::new(start.file_name.clone(), start.line, start.column, end.line, end.column));
        
        Ok(Statement {
            id: node_id,
            location,
            kind: StatementKind::Block {
                statements,
            },
        })
    }
    
    /// if文を解析
    pub fn parse_if_statement(&mut self) -> Result<Statement> {
        let start_loc = self.peek().map(|t| t.location.clone());
        self.consume(&TokenKind::If, "if文には'if'キーワードが必要です")?;
        
        let condition = Box::new(self.parse_expression()?);
        let then_branch = Box::new(self.parse_block_statement()?);
        
        let else_branch = if self.match_token(&TokenKind::Else) {
            if self.check(&TokenKind::If) {
                Some(Box::new(self.parse_if_statement()?))
            } else {
                Some(Box::new(self.parse_block_statement()?))
            }
        } else {
            None
        };
        
        let node_id = self.next_id();
        let end_loc = else_branch.as_ref()
            .and_then(|stmt| stmt.location.clone())
            .or_else(|| then_branch.location.clone())
            .or_else(|| self.peek().map(|t| t.location.clone()))
            .or(start_loc.clone());
        let location = start_loc.zip(end_loc).map(|(start, end)| SourceLocation::new(start.file_name.clone(), start.line, start.column, end.line, end.column));
        
        Ok(Statement {
            id: node_id,
            location,
            kind: StatementKind::IfStmt {
                condition,
                then_branch,
                else_branch,
            },
        })
    }
    
    /// while文を解析
    pub fn parse_while_statement(&mut self) -> Result<Statement> {
        let start_loc = self.peek().map(|t| t.location.clone());
        self.consume(&TokenKind::While, "while文には'while'キーワードが必要です")?;
        
        let condition = Box::new(self.parse_expression()?);
        let body = Box::new(self.parse_block_statement()?);
        
        let node_id = self.next_id();
        let end_loc = body.location.clone().or_else(|| self.peek().map(|t| t.location.clone())).or(start_loc.clone());
        let location = start_loc.zip(end_loc).map(|(start, end)| SourceLocation::new(start.file_name.clone(), start.line, start.column, end.line, end.column));
        
        Ok(Statement {
            id: node_id,
            location,
            kind: StatementKind::WhileStmt {
                condition,
                body,
            },
        })
    }
    
    /// for文を解析
    pub fn parse_for_statement(&mut self) -> Result<Statement> {
        let start_loc = self.peek().map(|t| t.location.clone());
        self.consume(&TokenKind::For, "for文には'for'キーワードが必要です")?;
        
        // for-each形式（for i in collection）か、C形式（for (init; cond; incr)）かを判断
        let next_token = self.peek().map(|t| &t.kind);
        
        if next_token == Some(&TokenKind::LeftParen) {
            // C形式の3部構成for文
            self.advance(); // '('を消費
            
            // 初期化部
            let initializer = if !self.check(&TokenKind::Semicolon) {
                if self.check(&TokenKind::Let) || self.check(&TokenKind::Const) {
                    // 変数宣言
                    Some(Box::new(if self.check(&TokenKind::Let) {
                        self.parse_variable_declaration()?
                    } else {
                        self.parse_constant_declaration()?
                    }))
                } else {
                    // 式文
                    let expr = self.parse_expression()?;
                    self.consume(&TokenKind::Semicolon, "for文の初期化部の後にはセミコロンが必要です")?;
                    Some(Box::new(Statement {
                        id: self.next_id(),
                        location: expr.location.clone(),
                        kind: StatementKind::ExpressionStmt {
                            expression: Box::new(expr),
                        },
                    }))
                }
            } else {
                self.consume(&TokenKind::Semicolon, "for文の初期化部の後にはセミコロンが必要です")?;
                None
            };
            
            // 条件部
            let condition = if !self.check(&TokenKind::Semicolon) {
                Some(Box::new(self.parse_expression()?))
            } else {
                None
            };
            self.consume(&TokenKind::Semicolon, "for文の条件部の後にはセミコロンが必要です")?;
            
            // 増分部
            let increment = if !self.check(&TokenKind::RightParen) {
                Some(Box::new(self.parse_expression()?))
            } else {
                None
            };
            self.consume(&TokenKind::RightParen, "C形式for文の終わりには')'が必要です")?;
            
            // 本体
            let body = Box::new(self.parse_block_statement()?);
            
            let node_id = self.next_id();
            let end_loc = body.location.clone().or_else(|| self.peek().map(|t| t.location.clone())).or(start_loc.clone());
            let location = start_loc.zip(end_loc).map(|(start, end)| 
                SourceLocation::new(start.file_name.clone(), start.line, start.column, end.line, end.column));
            
            Ok(Statement {
                id: node_id,
                location,
                kind: StatementKind::ForStmt {
                    initializer,
                    condition,
                    increment,
                    body,
                },
            })
        } else {
            // for-each形式
            let variable = match self.consume_identifier() {
                Some(ident) => ident,
                None => return Err(CompilerError::syntax_error(
                    "イテレータ変数の識別子が必要です",
                    self.peek().map(|t| t.location.clone()),
                )),
            };
            
            // 必要に応じて型注釈を解析
            let type_annotation = if self.check(&TokenKind::Colon) {
                self.advance(); // ':'を消費
                Some(self.parse_type_annotation()?)
            } else {
                None
            };
            
            // 可変宣言のフラグ
            let is_mutable = if self.check(&TokenKind::Mut) {
                self.advance(); // 'mut'を消費
                true
            } else {
                false
            };
            
            self.consume(&TokenKind::In, "for-each文には'in'キーワードが必要です")?;
            
            let iterable = Box::new(self.parse_expression()?);
            let body = Box::new(self.parse_block_statement()?);
            
            let node_id = self.next_id();
            let end_loc = body.location.clone().or_else(|| self.peek().map(|t| t.location.clone())).or(start_loc.clone());
            let location = start_loc.zip(end_loc).map(|(start, end)| 
                SourceLocation::new(start.file_name.clone(), start.line, start.column, end.line, end.column));
            
            Ok(Statement {
                id: node_id,
                location,
                kind: StatementKind::ForStmtEach {
                    variable: ast::VariableDeclaration {
                        id: self.next_id(),
                        name: variable,
                        is_mutable,
                        type_annotation,
                        initializer: None,
                        location: None,
                    },
                    iterable,
                    body,
                },
            })
        }
    }
    
    /// return文を解析
    pub fn parse_return_statement(&mut self) -> Result<Statement> {
        let start_loc = self.peek().map(|t| t.location.clone());
        self.consume(&TokenKind::Return, "return文には'return'キーワードが必要です")?;
        
        let expr = if !self.check(&TokenKind::Semicolon) {
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };
        
        self.consume(&TokenKind::Semicolon, "return文の後にはセミコロンが必要です")?;
        
        let node_id = self.next_id();
        let end_loc = self.peek().map(|t| t.location.clone()).or(start_loc.clone());
        let location = start_loc.zip(end_loc).map(|(start, end)| SourceLocation::new(start.file_name.clone(), start.line, start.column, end.line, end.column));
        
        Ok(Statement {
            id: node_id,
            location,
            kind: StatementKind::ReturnStmt {
                expression: expr,
            },
        })
    }
    
    /// break文を解析
    pub fn parse_break_statement(&mut self) -> Result<Statement> {
        let start_loc = self.peek().map(|t| t.location.clone());
        self.consume(&TokenKind::Break, "break文には'break'キーワードが必要です")?;
        self.consume(&TokenKind::Semicolon, "break文の後にはセミコロンが必要です")?;
        
        let node_id = self.next_id();
        let end_loc = self.peek().map(|t| t.location.clone()).or(start_loc.clone());
        let location = start_loc.zip(end_loc).map(|(start, end)| SourceLocation::new(start.file_name.clone(), start.line, start.column, end.line, end.column));
        
        Ok(Statement {
            id: node_id,
            location,
            kind: StatementKind::BreakStmt,
        })
    }
    
    /// continue文を解析
    pub fn parse_continue_statement(&mut self) -> Result<Statement> {
        let start_loc = self.peek().map(|t| t.location.clone());
        self.consume(&TokenKind::Continue, "continue文には'continue'キーワードが必要です")?;
        self.consume(&TokenKind::Semicolon, "continue文の後にはセミコロンが必要です")?;
        
        let node_id = self.next_id();
        let end_loc = self.peek().map(|t| t.location.clone()).or(start_loc.clone());
        let location = start_loc.zip(end_loc).map(|(start, end)| SourceLocation::new(start.file_name.clone(), start.line, start.column, end.line, end.column));
        
        Ok(Statement {
            id: node_id,
            location,
            kind: StatementKind::ContinueStmt,
        })
    }
    
    /// 式文を解析
    pub fn parse_expression_statement(&mut self) -> Result<Statement> {
        let expr = self.parse_expression()?;
        self.consume(&TokenKind::Semicolon, "式文の後にはセミコロンが必要です")?;
        
        let node_id = self.next_id();
        let location = expr.location.clone();
        
        Ok(Statement {
            id: node_id,
            location,
            kind: StatementKind::ExpressionStmt {
                expression: Box::new(expr),
            },
        })
    }
    
    /// 式を解析
    fn parse_expression(&mut self) -> Result<Expression> {
        self.parse_expression_with_precedence(0)
    }
    
    /// 優先順位を考慮した式の解析（Precedence Climbing法）
    fn parse_expression_with_precedence(&mut self, precedence: u8) -> Result<Expression> {
        // 左辺値（プレフィックス式）を解析
        let mut left = self.parse_prefix_expression()?;
        
        // 次のトークンが二項演算子であり、その優先順位が現在の優先順位より高い間、
        // 二項演算式を構築する
        while let Some(op_token) = self.peek() {
            if let Some(op) = self.get_binary_operator(op_token) {
                let op_precedence = self.get_operator_precedence(&op);
                
                if op_precedence <= precedence {
                    break;
                }
                
                self.advance(); // 演算子を消費
                
                // 優先順位を1上げて右辺値を解析（右結合演算子の場合は同じ優先順位を使用）
                let right_precedence = if self.is_right_associative(&op) {
                    op_precedence
                } else {
                    op_precedence + 1
                };
                
                let right = self.parse_expression_with_precedence(right_precedence)?;
                
                // 二項演算式を構築
                let start_loc = left.location.clone();
                let end_loc = right.location.clone();
                let location = start_loc.zip(end_loc).map(|(start, end)| 
                    SourceLocation::new(start.file_name.clone(), start.line, start.column, end.line, end.column));
                
                left = Expression {
                    id: self.next_id(),
                    location,
                    kind: ExpressionKind::BinaryOp {
                        operator: op,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                };
            } else if self.check(&TokenKind::LeftParen) && can_be_function_call(&left) {
                // 関数呼び出し式
                left = self.parse_call_expression(left)?;
            } else if self.check(&TokenKind::LeftBracket) {
                // 配列アクセス式
                left = self.parse_index_access_expression(left)?;
            } else if self.check(&TokenKind::Dot) {
                // メンバーアクセス式
                left = self.parse_member_access_expression(left)?;
            } else if self.check(&TokenKind::As) {
                // キャスト式
                left = self.parse_cast_expression(left)?;
            } else {
                break;
            }
        }
        
        Ok(left)
    }
    
    /// プレフィックス式を解析（単項演算子、リテラル、識別子、括弧式など）
    fn parse_prefix_expression(&mut self) -> Result<Expression> {
        if let Some(token) = self.peek() {
            match &token.kind {
                TokenKind::Plus | TokenKind::Minus | TokenKind::Not | TokenKind::BitwiseNot => {
                    // 単項演算子
                    let start_loc = Some(token.location.clone());
                    self.advance(); // 演算子を消費
                    
                    let operator = match token.kind {
                        TokenKind::Plus => UnaryOperator::Plus,
                        TokenKind::Minus => UnaryOperator::Minus,
                        TokenKind::Not => UnaryOperator::Not,
                        TokenKind::BitwiseNot => UnaryOperator::BitNot,
                        _ => unreachable!(),
                    };
                    
                    // オペランドを解析（単項演算子は高い優先順位を持つ）
                    let operand = self.parse_expression_with_precedence(100)?;
                    let end_loc = operand.location.clone();
                    let location = start_loc.zip(end_loc).map(|(start, end)| 
                        SourceLocation::new(start.file_name.clone(), start.line, start.column, end.line, end.column));
                    
                    Ok(Expression {
                        id: self.next_id(),
                        location,
                        kind: ExpressionKind::UnaryOp {
                            operator,
                            operand: Box::new(operand),
                        },
                    })
                },
                TokenKind::LeftParen => {
                    // 括弧式
                    let start_loc = Some(token.location.clone());
                    self.advance(); // '('を消費
                    
                    let expr = self.parse_expression()?;
                    
                    self.consume(&TokenKind::RightParen, "式の終わりには')'が必要です")?;
                    
                    Ok(expr) // 括弧式は括弧の中の式そのもの
                },
                TokenKind::LeftBrace => {
                    // ブロック式またはオブジェクトリテラル
                    self.parse_block_expression()
                },
                TokenKind::LeftBracket => {
                    // 配列リテラル
                    self.parse_array_literal()
                },
                TokenKind::If => {
                    // if式
                    self.parse_if_expression()
                },
                TokenKind::Match => {
                    // match式
                    self.parse_match_expression()
                },
                TokenKind::Fn => {
                    // ラムダ式
                    self.parse_lambda_expression()
                },
                TokenKind::True | TokenKind::False => {
                    // 真偽値リテラル
                    let value = token.kind == TokenKind::True;
                    self.advance(); // 'true'または'false'を消費
                    
                    Ok(Expression {
                        id: self.next_id(),
                        location: Some(token.location.clone()),
                        kind: ExpressionKind::Literal(Literal {
                            kind: LiteralKind::Boolean(value),
                        }),
                    })
                },
                TokenKind::Integer(value) => {
                    // 整数リテラル
                    let loc = token.location.clone();
                    self.advance(); // 整数値を消費
                    
                    Ok(Expression {
                        id: self.next_id(),
                        location: Some(loc),
                        kind: ExpressionKind::Literal(Literal {
                            kind: LiteralKind::Integer(*value),
                        }),
                    })
                },
                TokenKind::Float(value) => {
                    // 浮動小数点数リテラル
                    let loc = token.location.clone();
                    self.advance(); // 浮動小数点値を消費
                    
                    Ok(Expression {
                        id: self.next_id(),
                        location: Some(loc),
                        kind: ExpressionKind::Literal(Literal {
                            kind: LiteralKind::Float(*value),
                        }),
                    })
                },
                TokenKind::String(value) => {
                    // 文字列リテラル
                    let loc = token.location.clone();
                    let value_clone = value.clone();
                    self.advance(); // 文字列値を消費
                    
                    Ok(Expression {
                        id: self.next_id(),
                        location: Some(loc),
                        kind: ExpressionKind::Literal(Literal {
                            kind: LiteralKind::String(value_clone),
                        }),
                    })
                },
                TokenKind::Char(value) => {
                    // 文字リテラル
                    let loc = token.location.clone();
                    let value_clone = *value;
                    self.advance(); // 文字値を消費
                    
                    Ok(Expression {
                        id: self.next_id(),
                        location: Some(loc),
                        kind: ExpressionKind::Literal(Literal {
                            kind: LiteralKind::Char(value_clone),
                        }),
                    })
                },
                TokenKind::Identifier(_) => {
                    // 識別子
                    if let Some(ident) = self.consume_identifier() {
                        Ok(Expression {
                            id: self.next_id(),
                            location: ident.location.clone(),
                            kind: ExpressionKind::Identifier(ident),
                        })
                    } else {
                        Err(CompilerError::syntax_error(
                            "識別子が必要です",
                            Some(token.location.clone()),
                        ))
                    }
                },
                _ => {
                    Err(CompilerError::syntax_error(
                        "有効な式が必要です",
                        Some(token.location.clone()),
                    ))
                }
            }
        } else {
            Err(CompilerError::syntax_error(
                "式が必要ですが、トークンがありません",
                None,
            ))
        }
    }
    
    /// 関数呼び出し式を解析
    fn parse_call_expression(&mut self, callee: Expression) -> Result<Expression> {
        let start_loc = callee.location.clone();
        self.consume(&TokenKind::LeftParen, "関数呼び出しには'('が必要です")?;
        
        let mut arguments = Vec::new();
        
        // 引数リストを解析
        if !self.check(&TokenKind::RightParen) {
            loop {
                let arg = self.parse_expression()?;
                arguments.push(arg);
                
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
        }
        
        let end_token = self.consume(&TokenKind::RightParen, "関数呼び出しの終わりには')'が必要です")?;
        let end_loc = Some(end_token.location.clone());
        let location = start_loc.zip(end_loc).map(|(start, end)| 
            SourceLocation::new(start.file_name.clone(), start.line, start.column, end.line, end.column));
        
        Ok(Expression {
            id: self.next_id(),
            location,
            kind: ExpressionKind::Call {
                callee: Box::new(callee),
                arguments,
            },
        })
    }
    
    /// インデックスアクセス式を解析
    fn parse_index_access_expression(&mut self, array: Expression) -> Result<Expression> {
        let start_loc = array.location.clone();
        self.consume(&TokenKind::LeftBracket, "配列アクセスには'['が必要です")?;
        
        let index = self.parse_expression()?;
        
        let end_token = self.consume(&TokenKind::RightBracket, "配列アクセスの終わりには']'が必要です")?;
        let end_loc = Some(end_token.location.clone());
        let location = start_loc.zip(end_loc).map(|(start, end)| 
            SourceLocation::new(start.file_name.clone(), start.line, start.column, end.line, end.column));
        
        Ok(Expression {
            id: self.next_id(),
            location,
            kind: ExpressionKind::IndexAccess {
                array: Box::new(array),
                index: Box::new(index),
            },
        })
    }
    
    /// メンバーアクセス式を解析
    fn parse_member_access_expression(&mut self, object: Expression) -> Result<Expression> {
        let start_loc = object.location.clone();
        self.consume(&TokenKind::Dot, "メンバーアクセスには'.'が必要です")?;
        
        let member = match self.consume_identifier() {
            Some(ident) => ident,
            None => return Err(CompilerError::syntax_error(
                "メンバー名の識別子が必要です",
                self.peek().map(|t| t.location.clone()),
            )),
        };
        
        let end_loc = member.location.clone();
        let location = start_loc.zip(end_loc).map(|(start, end)| 
            SourceLocation::new(start.file_name.clone(), start.line, start.column, end.line, end.column));
        
        Ok(Expression {
            id: self.next_id(),
            location,
            kind: ExpressionKind::MemberAccess {
                object: Box::new(object),
                member,
            },
        })
    }
    
    /// キャスト式を解析
    fn parse_cast_expression(&mut self, expr: Expression) -> Result<Expression> {
        let start_loc = expr.location.clone();
        self.consume(&TokenKind::As, "キャスト式には'as'キーワードが必要です")?;
        
        let target_type = self.parse_type_annotation()?;
        
        let end_loc = target_type.location.clone();
        let location = start_loc.zip(end_loc).map(|(start, end)| 
            SourceLocation::new(start.file_name.clone(), start.line, start.column, end.line, end.column));
        
        Ok(Expression {
            id: self.next_id(),
            location,
            kind: ExpressionKind::Cast {
                expression: Box::new(expr),
                target_type,
            },
        })
    }
    
    /// 配列リテラルを解析
    fn parse_array_literal(&mut self) -> Result<Expression> {
        let start_token = self.consume(&TokenKind::LeftBracket, "配列リテラルには'['が必要です")?;
        let start_loc = Some(start_token.location.clone());
        
        let mut elements = Vec::new();
        
        // 要素リストを解析
        if !self.check(&TokenKind::RightBracket) {
            loop {
                let element = self.parse_expression()?;
                elements.push(element);
                
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
        }
        
        let end_token = self.consume(&TokenKind::RightBracket, "配列リテラルの終わりには']'が必要です")?;
        let end_loc = Some(end_token.location.clone());
        let location = start_loc.zip(end_loc).map(|(start, end)| 
            SourceLocation::new(start.file_name.clone(), start.line, start.column, end.line, end.column));
        
        Ok(Expression {
            id: self.next_id(),
            location,
            kind: ExpressionKind::ArrayLiteral {
                elements,
            },
        })
    }
    
    /// ブロック式を解析
    fn parse_block_expression(&mut self) -> Result<Expression> {
        let block_stmt = self.parse_block_statement()?;
        
        Ok(Expression {
            id: self.next_id(),
            location: block_stmt.location.clone(),
            kind: ExpressionKind::Block {
                statements: match block_stmt.kind {
                    StatementKind::Block { statements } => statements,
                    _ => unreachable!(),
                },
            },
        })
    }
    
    /// if式を解析
    fn parse_if_expression(&mut self) -> Result<Expression> {
        let if_stmt = self.parse_if_statement()?;
        
        if let StatementKind::IfStmt { condition, then_branch, else_branch } = if_stmt.kind {
            Ok(Expression {
                id: self.next_id(),
                location: if_stmt.location.clone(),
                kind: ExpressionKind::If {
                    condition,
                    then_branch,
                    else_branch,
                },
            })
        } else {
            unreachable!()
        }
    }
    
    /// match式を解析
    fn parse_match_expression(&mut self) -> Result<Expression> {
        let start_token = self.consume(&TokenKind::Match, "match式には'match'キーワードが必要です")?;
        let start_loc = Some(start_token.location.clone());
        
        // マッチ対象の式を解析
        let scrutinee = Box::new(self.parse_expression()?);
        
        self.consume(&TokenKind::LeftBrace, "match式の本体には'{'が必要です")?;
        
        let mut arms = Vec::new();
        
        // マッチアームを解析
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            let arm_start_loc = self.peek().map(|t| t.location.clone());
            
            // パターンを解析
            let pattern = self.parse_pattern()?;
            
            // ガード式（オプション）
            let guard = if self.match_token(&TokenKind::If) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            
            self.consume(&TokenKind::FatArrow, "マッチアームには'=>'が必要です")?;
            
            // アームの本体（式）を解析
            let expression = self.parse_expression()?;
            
            // 省略可能なカンマ
            self.match_token(&TokenKind::Comma);
            
            // アームの位置情報
            let arm_end_loc = expression.location.clone().or(arm_start_loc.clone());
            let arm_location = arm_start_loc.zip(arm_end_loc).map(|(start, end)| 
                SourceLocation::new(start.file_name.clone(), start.line, start.column, end.line, end.column));
                
            // Spanに変換
            let span = arm_location.map(|loc| {
                Span {
                    start: 0, // 実際はファイル内の絶対位置を計算する必要があります
                    end: 0,   // 同上
                }
            }).unwrap_or(Span { start: 0, end: 0 });
            
            arms.push(MatchArm {
                pattern,
                guard,
                expression,
                span,
            });
        }
        
        let end_token = self.consume(&TokenKind::RightBrace, "match式の終わりには'}'が必要です")?;
        let end_loc = Some(end_token.location.clone());
        let location = start_loc.zip(end_loc).map(|(start, end)| 
            SourceLocation::new(start.file_name.clone(), start.line, start.column, end.line, end.column));
        
        // Spanに変換
        let span = location.map(|loc| {
            Span {
                start: 0, // 実際はファイル内の絶対位置を計算する必要があります
                end: 0,   // 同上
            }
        }).unwrap_or(Span { start: 0, end: 0 });
        
        Ok(Expression {
            id: self.next_id(),
            kind: ExpressionKind::Match { 
                scrutinee, 
                arms,
            },
            location,
        })
    }
    
    /// パターンを解析
    fn parse_pattern(&mut self) -> Result<Pattern> {
        if let Some(token) = self.peek() {
            match &token.kind {
                TokenKind::Underscore => {
                    // ワイルドカードパターン
                    let token = self.advance();
                    let span = Span { start: 0, end: 0 }; // 適切なSpanを作成
                    return Ok(Pattern::Wildcard(span));
                },
                TokenKind::Identifier(_) => {
                    // 識別子パターン（変数バインディング）
                    let ident = self.consume_identifier().unwrap();
                    
                    if self.check(&TokenKind::LeftBrace) {
                        // 構造体パターン
                        return self.parse_struct_pattern(ident);
                    } else if self.check(&TokenKind::DoubleColon) {
                        // 列挙型パターン
                        return self.parse_enum_pattern(vec![ident]);
                    } else {
                        // 識別子パターン
                        return Ok(Pattern::Identifier(ident));
                    }
                },
                TokenKind::LeftParen => {
                    // タプルパターン
                    return self.parse_tuple_pattern();
                },
                TokenKind::LeftBracket => {
                    // 配列パターン
                    return self.parse_array_pattern();
                },
                TokenKind::Ampersand => {
                    // 参照パターン
                    return self.parse_reference_pattern();
                },
                // リテラルパターン
                TokenKind::Integer(_) | TokenKind::Float(_) | 
                TokenKind::String(_) | TokenKind::True | TokenKind::False | 
                TokenKind::Null => {
                    let literal = self.parse_literal()?;
                    return Ok(Pattern::Literal(literal));
                },
                _ => {
                    return Err(CompilerError::syntax_error(
                        "有効なパターンが必要です",
                        Some(token.location.clone()),
                    ));
                }
            }
        } else {
            return Err(CompilerError::syntax_error(
                "パターンが必要ですが、トークンがありません",
                None,
            ));
        }
    }
    
    /// 構造体パターンを解析
    fn parse_struct_pattern(&mut self, name: Identifier) -> Result<Pattern> {
        let start_loc = name.location.clone();
        self.consume(&TokenKind::LeftBrace, "構造体パターンには'{'が必要です")?;
        
        let mut fields = Vec::new();
        
        if !self.check(&TokenKind::RightBrace) {
            loop {
                // フィールド名
                let field_name = self.consume_identifier().ok_or_else(|| {
                    CompilerError::syntax_error(
                        "フィールド名が必要です",
                        self.peek().map(|t| t.location.clone()),
                    )
                })?;
                
                // : パターン
                let field_pattern = if self.match_token(&TokenKind::Colon) {
                    self.parse_pattern()?
                } else {
                    // 省略形 (x) は x: x と同じ
                    Pattern::Identifier(field_name.clone())
                };
                
                fields.push((field_name, field_pattern));
                
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
                
                // 末尾のカンマを許容
                if self.check(&TokenKind::RightBrace) {
                    break;
                }
            }
        }
        
        let end_token = self.consume(&TokenKind::RightBrace, "構造体パターンの終わりには'}'が必要です")?;
        
        // Spanを作成
        let span = Span {
            start: 0, // 適切な値を設定
            end: 0,   // 適切な値を設定
        };
        
        Ok(Pattern::Struct {
            name,
            fields,
            span,
        })
    }
    
    /// 列挙型パターンを解析
    fn parse_enum_pattern(&mut self, mut path: Vec<Identifier>) -> Result<Pattern> {
        // パスの解析
        while self.match_token(&TokenKind::DoubleColon) {
            let segment = self.consume_identifier().ok_or_else(|| {
                CompilerError::syntax_error(
                    "列挙型パスのセグメントが必要です",
                    self.peek().map(|t| t.location.clone()),
                )
            })?;
            
            path.push(segment);
        }
        
        // 最後のセグメントがバリアント名
        let variant = path.pop().unwrap();
        
        // ペイロードがある場合
        let payload = if self.check(&TokenKind::LeftParen) {
            self.advance(); // '('を消費
            
            let payload_pattern = self.parse_pattern()?;
            
            self.consume(&TokenKind::RightParen, "列挙型パターンのペイロードの終わりには')'が必要です")?;
            
            Some(Box::new(payload_pattern))
        } else {
            None
        };
        
        // Spanを作成
        let span = Span {
            start: 0, // 適切な値を設定
            end: 0,   // 適切な値を設定
        };
        
        Ok(Pattern::Enum {
            path,
            variant,
            payload,
            span,
        })
    }
    
    /// タプルパターンを解析
    fn parse_tuple_pattern(&mut self) -> Result<Pattern> {
        self.consume(&TokenKind::LeftParen, "タプルパターンには'('が必要です")?;
        
        let mut elements = Vec::new();
        
        if !self.check(&TokenKind::RightParen) {
            loop {
                let element = self.parse_pattern()?;
                elements.push(element);
                
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
                
                // 末尾のカンマを許容
                if self.check(&TokenKind::RightParen) {
                    break;
                }
            }
        }
        
        self.consume(&TokenKind::RightParen, "タプルパターンの終わりには')'が必要です")?;
        
        // Spanを作成
        let span = Span {
            start: 0, // 適切な値を設定
            end: 0,   // 適切な値を設定
        };
        
        Ok(Pattern::Tuple {
            elements,
            span,
        })
    }
    
    /// 配列パターンを解析
    fn parse_array_pattern(&mut self) -> Result<Pattern> {
        self.consume(&TokenKind::LeftBracket, "配列パターンには'['が必要です")?;
        
        let mut elements = Vec::new();
        let mut rest = None;
        
        if !self.check(&TokenKind::RightBracket) {
            loop {
                // スプレッド演算子があるか確認
                if self.match_token(&TokenKind::DotDotDot) {
                    // 残りの要素を表すパターン
                    if !self.check(&TokenKind::RightBracket) && !self.check(&TokenKind::Comma) {
                        rest = Some(Box::new(self.parse_pattern()?));
                    }
                    
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                } else {
                    // 通常の要素パターン
                    let element = self.parse_pattern()?;
                    elements.push(element);
                    
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                    
                    // 末尾のカンマを許容
                    if self.check(&TokenKind::RightBracket) {
                        break;
                    }
                }
            }
        }
        
        self.consume(&TokenKind::RightBracket, "配列パターンの終わりには']'が必要です")?;
        
        // Note: 現在のASTにはArrayパターンがないようなので、Tupleパターンに変換して返す
        // 実際のコードベースではArray型を追加するか、適切な処理に修正する必要があります
        let span = Span {
            start: 0,
            end: 0,
        };
        
        Ok(Pattern::Tuple {
            elements,
            span,
        })
    }
    
    /// 参照パターンを解析
    fn parse_reference_pattern(&mut self) -> Result<Pattern> {
        self.consume(&TokenKind::Ampersand, "参照パターンには'&'が必要です")?;
        
        // 可変参照かどうかを確認
        let mutable = self.match_token(&TokenKind::Mut);
        
        let inner = Box::new(self.parse_pattern()?);
        
        // Spanを作成
        let span = Span {
            start: 0, // 適切な値を設定
            end: 0,   // 適切な値を設定
        };
        
        Ok(Pattern::Reference {
            pattern: inner,
            mutable,
            span,
        })
    }
}
pub mod context_parser;

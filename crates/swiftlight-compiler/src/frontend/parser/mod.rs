//! # 構文解析器
//! 
//! SwiftLight言語の構文解析を担当するモジュールです。
//! 字句解析器からのトークン列を受け取り、言語の文法に基づいて
//! 抽象構文木（AST）を構築します。

use std::iter::Peekable;
use std::slice::Iter;

use crate::frontend::ast::{
    self, BinaryOperator, EnumVariant, Expression, ExpressionKind,
    Identifier, Literal, LiteralKind, NodeId, Parameter, Program,
    Statement, StatementKind, StructField, TraitMethod, TypeAnnotation,
    UnaryOperator,
};
use crate::frontend::error::{CompilerError, ErrorKind, Result, SourceLocation};
use crate::frontend::lexer::{Token, TokenKind};

pub mod error;
pub mod expression;
pub mod statement;
pub mod declaration;
pub mod types;

/// 構文解析器（Parser）
pub struct Parser<'a> {
    /// トークン列
    tokens: Peekable<Iter<'a, Token>>,
    /// 現在の位置（インデックス）
    position: usize,
    /// 終了位置（トークン列の長さ）
    end: usize,
    /// プログラム（構築中のAST）
    program: Program,
    /// 解析中に発生したエラー
    errors: Vec<CompilerError>,
    /// パニックモード（エラー回復用）
    panic_mode: bool,
}

impl<'a> Parser<'a> {
    /// 新しい構文解析器を作成
    pub fn new(tokens: &'a [Token], file_name: &str) -> Self {
        Self {
            tokens: tokens.iter().peekable(),
            position: 0,
            end: tokens.len(),
            program: Program::new(file_name),
            errors: Vec::new(),
            panic_mode: false,
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
                
                parameters.push(Parameter {
                    id: self.next_id(),
                    name: param_name,
                    type_annotation: param_type,
                    location: None, // TODO: 位置情報を設定
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
                location: None, // TODO: 位置情報を設定
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
                    location: None, // TODO: 位置情報を設定
                });
            } else {
                variants.push(EnumVariant {
                    id: self.next_id(),
                    name: variant_name,
                    associated_data,
                    discriminant: None,
                    location: None, // TODO: 位置情報を設定
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
                location: None, // TODO: 位置情報を設定
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
            kind: StatementKind::If {
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
            kind: StatementKind::While {
                condition,
                body,
            },
        })
    }
    
    /// for文を解析
    pub fn parse_for_statement(&mut self) -> Result<Statement> {
        let start_loc = self.peek().map(|t| t.location.clone());
        self.consume(&TokenKind::For, "for文には'for'キーワードが必要です")?;
        
        let iterator = match self.consume_identifier() {
            Some(ident) => ident,
            None => return Err(CompilerError::syntax_error(
                "イテレータ変数の識別子が必要です",
                self.peek().map(|t| t.location.clone()),
            )),
        };
        
        self.consume_token(Token::For)?;
        let iterator = self.consume_identifier().ok_or(ParserError::ExpectedIdentifier)?;
        self.consume_token(Token::In)?;
        let iterable = self.parse_expression()?;
        let body = self.parse_block_statement()?;
        Ok(Statement::For { iterator, iterable, body })
    }
    
    /// return文を解析
    pub fn parse_return_statement(&mut self) -> Result<Statement> {
        self.consume_token(Token::Return)?;
        let expr = if !self.check_token(Token::Semicolon) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        self.consume_token(Token::Semicolon)?;
        Ok(Statement::Return(expr))
    }
    
    /// break文を解析
    pub fn parse_break_statement(&mut self) -> Result<Statement> {
        self.consume_token(Token::Break)?;
        self.consume_token(Token::Semicolon)?;
        Ok(Statement::Break)
    }
    
    /// continue文を解析
    pub fn parse_continue_statement(&mut self) -> Result<Statement> {
        self.consume_token(Token::Continue)?;
        self.consume_token(Token::Semicolon)?;
        Ok(Statement::Continue)
    }
    
    /// 式文を解析
    pub fn parse_expression_statement(&mut self) -> Result<Statement> {
        let expr = self.parse_expression()?;
        self.consume_token(Token::Semicolon)?;
        Ok(Statement::ExpressionStatement(Box::new(expr)))
    }
    
    /// 式を解析
    fn parse_expression(&mut self) -> Result<Expression> {
        // TODO: 実装
        todo!("式の解析")
    }
    
    /// 型注釈を解析
    pub fn parse_type_annotation(&mut self) -> Result<TypeAnnotation> {
        if let Some(ident) = self.consume_identifier() {
            Ok(TypeAnnotation::Simple(ident))
        } else {
            Err(ParserError::ExpectedType)
        }
    }
    
    /// トークンを消費（次のトークンに進む）
    fn advance(&mut self) -> Option<&Token> {
        if !self.is_at_end() {
            self.position += 1;
            self.tokens.next()
        } else {
            None
        }
    }
    
    /// 現在のトークンを取得（消費せず）
    fn peek(&self) -> Option<&Token> {
        self.tokens.peek().copied()
    }
    
    /// 現在のトークンの種類を取得（消費せず）
    fn peek_kind(&self) -> Option<&TokenKind> {
        self.peek().map(|token| &token.kind)
    }
    
    /// ファイル終端に達したかどうかを判定
    fn is_at_end(&self) -> bool {
        self.position >= self.end || self.peek_kind() == Some(&TokenKind::Eof)
    }
    
    /// 期待するトークンの種類と一致するかチェックし、一致すれば消費
    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if let Some(token_kind) = self.peek_kind() {
            if std::mem::discriminant(token_kind) == std::mem::discriminant(kind) {
                self.advance();
                return true;
            }
        }
        false
    }
    
    /// 期待するトークンを消費し、一致しなければエラー
    fn consume(&mut self, kind: &TokenKind, message: &str) -> Result<&Token> {
        if let Some(token) = self.peek() {
            if std::mem::discriminant(&token.kind) == std::mem::discriminant(kind) {
                return Ok(self.advance().unwrap());
            }
            
            let error = CompilerError::syntax_error(
                message,
                Some(token.location.clone()),
            );
            Err(error)
        } else {
            let location = if let Some(last) = self.tokens.clone().last() {
                Some(last.location.clone())
            } else {
                None
            };
            
            let error = CompilerError::syntax_error(
                message,
                location,
            );
            Err(error)
        }
    }
    
    /// エラー回復（パニックモードからの同期）
    fn synchronize(&mut self) {
        self.panic_mode = false;
        
        while let Some(token) = self.peek() {
            // 文の終わりに到達したら同期完了
            if let TokenKind::Semicolon = token.kind {
                self.advance();
                return;
            }
            
            // 次の文や宣言の開始と思われるトークンに到達したら同期完了
            match token.kind {
                TokenKind::Func | TokenKind::Let | TokenKind::Const |
                TokenKind::If | TokenKind::While | TokenKind::For |
                TokenKind::Return | TokenKind::Struct | TokenKind::Enum |
                TokenKind::Trait | TokenKind::Impl | TokenKind::Import => {
                    return;
                }
                _ => {
                    self.advance();
                }
            }
        }
    }
    
    /// 新しいノードIDを生成
    fn next_id(&mut self) -> NodeId {
        self.program.next_id()
    }
}

/// テスト
#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::lexer;
    
    #[test]
    fn test_empty_program() {
        let source = "";
        let tokens = lexer::tokenize(source, "test.swl").unwrap();
        let mut parser = Parser::new(&tokens, "test.swl");
        
        let program = parser.parse_program().unwrap();
        assert_eq!(program.len(), 0);
    }
    
    // TODO: 他のテストケースを追加
}

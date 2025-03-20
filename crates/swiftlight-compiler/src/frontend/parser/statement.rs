//! # 文の構文解析
//! 
//! SwiftLight言語の文（ブロック、制御構造、式文など）の
//! 構文解析を担当するモジュールです。

use crate::frontend::ast::{Expression, Statement, StatementKind};
use crate::frontend::error::Result;
use crate::frontend::lexer::TokenKind;

use super::Parser;
use super::error;

impl<'a> Parser<'a> {
    /// 文のリストを解析
    pub(crate) fn parse_statement_list(&mut self, terminator: &TokenKind) -> Result<Vec<Statement>> {
        let mut statements = Vec::new();
        
        while !self.is_at_end() && !self.check(terminator) {
            statements.push(self.parse_statement()?);
        }
        
        Ok(statements)
    }
    
    /// 文を解析
    pub(crate) fn parse_statement(&mut self) -> Result<Statement> {
        if self.match_token(&TokenKind::LeftBrace) {
            // 先読みした左中括弧を戻す
            self.unget_token();
            return self.parse_block();
        } else if self.match_token(&TokenKind::If) {
            self.unget_token();
            return self.parse_if_statement();
        } else if self.match_token(&TokenKind::While) {
            self.unget_token();
            return self.parse_while_statement();
        } else if self.match_token(&TokenKind::For) {
            self.unget_token();
            return self.parse_for_statement();
        } else if self.match_token(&TokenKind::Return) {
            self.unget_token();
            return self.parse_return_statement();
        } else if self.match_token(&TokenKind::Break) {
            self.unget_token();
            return self.parse_break_statement();
        } else if self.match_token(&TokenKind::Continue) {
            self.unget_token();
            return self.parse_continue_statement();
        } else if self.match_token(&TokenKind::Let) {
            self.unget_token();
            return self.parse_variable_declaration();
        } else if self.match_token(&TokenKind::Match) {
            self.unget_token();
            return self.parse_match_statement();
        } else if self.match_token(&TokenKind::Loop) {
            self.unget_token();
            return self.parse_loop_statement();
        } else {
            // その他は式文として解析
            return self.parse_expression_statement();
        }
    }
    
    /// ブロック文を解析 ({ ... })
    pub(crate) fn parse_block(&mut self) -> Result<Statement> {
        let open_brace = self.consume(&TokenKind::LeftBrace, "ブロックの開始には '{' が必要です")?;
        
        let mut statements = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }
        
        let close_brace = self.consume(&TokenKind::RightBrace, "ブロックの終了には '}' が必要です")?;
        
        let id = self.next_id();
        Ok(Statement::new(
            StatementKind::Block(statements),
            id,
            Some(open_brace.location.clone()),
        ))
    }
    
    /// if文を解析
    pub(crate) fn parse_if_statement(&mut self) -> Result<Statement> {
        let if_token = self.consume(&TokenKind::If, "if文には 'if' キーワードが必要です")?;
        
        // 条件式の括弧は省略可能（Swift風の構文）
        let condition = if self.match_token(&TokenKind::LeftParen) {
            let expr = self.parse_expression()?;
            self.consume(&TokenKind::RightParen, "条件式の後に ')' が必要です")?;
            expr
        } else {
            self.parse_expression()?
        };
        
        // then節のブロックまたは文
        let then_branch = if self.check(&TokenKind::LeftBrace) {
            self.parse_block()?
        } else {
            self.parse_statement()?
        };
        
        // else節（オプション）
        let else_branch = if self.match_token(&TokenKind::Else) {
            let else_statement = if self.check(&TokenKind::If) {
                // else if の場合
                self.parse_if_statement()?
            } else if self.check(&TokenKind::LeftBrace) {
                // else { ... } の場合
                self.parse_block()?
            } else {
                // 単一文の場合
                self.parse_statement()?
            };
            
            Some(Box::new(else_statement))
        } else {
            None
        };
        
        let id = self.next_id();
        Ok(Statement::new(
            StatementKind::IfStmt {
                condition,
                then_branch: Box::new(then_branch),
                else_branch,
            },
            id,
            Some(if_token.location.clone()),
        ))
    }
    
    /// while文を解析
    pub(crate) fn parse_while_statement(&mut self) -> Result<Statement> {
        let while_token = self.consume(&TokenKind::While, "while文には 'while' キーワードが必要です")?;
        
        // 条件式の括弧は省略可能
        let condition = if self.match_token(&TokenKind::LeftParen) {
            let expr = self.parse_expression()?;
            self.consume(&TokenKind::RightParen, "条件式の後に ')' が必要です")?;
            expr
        } else {
            self.parse_expression()?
        };
        
        // ループ本体
        let body = if self.check(&TokenKind::LeftBrace) {
            self.parse_block()?
        } else {
            self.parse_statement()?
        };
        
        let id = self.next_id();
        Ok(Statement::new(
            StatementKind::WhileStmt {
                condition,
                body: Box::new(body),
            },
            id,
            Some(while_token.location.clone()),
        ))
    }
    
    /// for文を解析
    pub(crate) fn parse_for_statement(&mut self) -> Result<Statement> {
        let for_token = self.consume(&TokenKind::For, "for文には 'for' キーワードが必要です")?;
        
        // イテレータ変数
        let variable = self.parse_identifier()?;
        
        // 'in' キーワード
        self.consume(&TokenKind::In, "forループには 'in' キーワードが必要です")?;
        
        // イテレート対象
        let iterable = self.parse_expression()?;
        
        // ループ本体
        let body = if self.check(&TokenKind::LeftBrace) {
            self.parse_block()?
        } else {
            self.parse_statement()?
        };
        
        let id = self.next_id();
        Ok(Statement::new(
            StatementKind::ForStmt {
                variable,
                iterable,
                body: Box::new(body),
            },
            id,
            Some(for_token.location.clone()),
        ))
    }
    
    /// loop文を解析（無限ループ）
    pub(crate) fn parse_loop_statement(&mut self) -> Result<Statement> {
        let loop_token = self.consume(&TokenKind::Loop, "loop文には 'loop' キーワードが必要です")?;
        
        // ループ本体
        let body = if self.check(&TokenKind::LeftBrace) {
            self.parse_block()?
        } else {
            self.parse_statement()?
        };
        
        let id = self.next_id();
        Ok(Statement::new(
            StatementKind::Loop {
                body: Box::new(body),
            },
            id,
            Some(loop_token.location.clone()),
        ))
    }
    
    /// match文を解析
    pub(crate) fn parse_match_statement(&mut self) -> Result<Statement> {
        let match_token = self.consume(&TokenKind::Match, "match文には 'match' キーワードが必要です")?;
        
        // マッチ対象の式
        let value = self.parse_expression()?;
        
        // マッチブロック
        self.consume(&TokenKind::LeftBrace, "match文には '{' が必要です")?;
        
        let mut arms = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            // パターン
            let pattern = self.parse_pattern()?;
            
            // => または -> 演算子
            if !self.match_token(&TokenKind::FatArrow) && !self.match_token(&TokenKind::Arrow) {
                return Err(error::syntax_error(
                    self.peek().location.clone(),
                    "パターンの後には '=>' または '->' が必要です"
                ));
            }
            
            // 結果式またはブロック
            let result = if self.check(&TokenKind::LeftBrace) {
                self.parse_block()?
            } else {
                let expr = self.parse_expression()?;
                // セミコロンは省略可能
                self.match_token(&TokenKind::Semicolon);
                
                let id = self.next_id();
                Statement::new(
                    StatementKind::ExpressionStmt(expr.clone()),
                    id,
                    expr.location.clone(),
                )
            };
            
            arms.push((pattern, result));
            
            // カンマは省略可能
            self.match_token(&TokenKind::Comma);
        }
        
        self.consume(&TokenKind::RightBrace, "match文の終了には '}' が必要です")?;
        
        let id = self.next_id();
        Ok(Statement::new(
            StatementKind::Match {
                value,
                arms,
            },
            id,
            Some(match_token.location.clone()),
        ))
    }
    
    /// 変数宣言を解析
    pub(crate) fn parse_variable_declaration(&mut self) -> Result<Statement> {
        let let_token = self.consume(&TokenKind::Let, "変数宣言には 'let' キーワードが必要です")?;
        
        // 変数名
        let name = self.parse_identifier()?;
        
        // 型注釈（オプション）
        let type_annotation = if self.match_token(&TokenKind::Colon) {
            Some(self.parse_type_expression()?)
        } else {
            None
        };
        
        // 初期化式（オプション）
        let initializer = if self.match_token(&TokenKind::Equal) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        // セミコロンは省略可能
        self.match_token(&TokenKind::Semicolon);
        
        let id = self.next_id();
        Ok(Statement::new(
            StatementKind::VariableDeclaration {
                name,
                type_annotation,
                initializer,
                is_mutable: false, // letはイミュータブル
            },
            id,
            Some(let_token.location.clone()),
        ))
    }
    
    /// return文を解析
    pub(crate) fn parse_return_statement(&mut self) -> Result<Statement> {
        let return_token = self.consume(&TokenKind::Return, "return文には 'return' キーワードが必要です")?;
        
        // 戻り値（オプション）
        let value = if self.check(&TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expression()?)
        };
        
        // セミコロンは省略可能
        self.match_token(&TokenKind::Semicolon);
        
        let id = self.next_id();
        Ok(Statement::new(
            StatementKind::ReturnStmt(value),
            id,
            Some(return_token.location.clone()),
        ))
    }
    
    /// break文を解析
    pub(crate) fn parse_break_statement(&mut self) -> Result<Statement> {
        let break_token = self.consume(&TokenKind::Break, "break文には 'break' キーワードが必要です")?;
        
        // セミコロンは省略可能
        self.match_token(&TokenKind::Semicolon);
        
        let id = self.next_id();
        Ok(Statement::new(
            StatementKind::BreakStmt,
            id,
            Some(break_token.location.clone()),
        ))
    }
    
    /// continue文を解析
    pub(crate) fn parse_continue_statement(&mut self) -> Result<Statement> {
        let continue_token = self.consume(&TokenKind::Continue, "continue文には 'continue' キーワードが必要です")?;
        
        // セミコロンは省略可能
        self.match_token(&TokenKind::Semicolon);
        
        let id = self.next_id();
        Ok(Statement::new(
            StatementKind::ContinueStmt,
            id,
            Some(continue_token.location.clone()),
        ))
    }
    
    /// 式文を解析
    pub(crate) fn parse_expression_statement(&mut self) -> Result<Statement> {
        let expr = self.parse_expression()?;
        
        // セミコロンは省略可能
        self.match_token(&TokenKind::Semicolon);
        
        let id = self.next_id();
        Ok(Statement::new(
            StatementKind::ExpressionStmt(expr.clone()),
            id,
            expr.location.clone(),
        ))
    }
    
    /// パターンを解析（match文で使用）
    fn parse_pattern(&mut self) -> Result<Expression> {
        // 現在はリテラルパターンと識別子パターンのみサポート
        // 将来的には構造体パターン、タプルパターン、範囲パターンなどを追加
        if self.check(&TokenKind::Underscore) {
            // ワイルドカードパターン
            let token = self.advance();
            let id = self.next_id();
            Ok(Expression::new(
                ExpressionKind::Wildcard,
                id,
                Some(token.location.clone()),
            ))
        } else if self.check_literal() {
            // リテラルパターン
            self.parse_literal()
        } else {
            // 識別子パターン（変数バインディング）
            self.parse_identifier()
        }
    }
    
    /// 型式を解析
    fn parse_type_expression(&mut self) -> Result<Expression> {
        // 基本型名
        let type_name = self.parse_identifier()?;
        
        // ジェネリック型パラメータ（オプション）
        if self.match_token(&TokenKind::Less) {
            let mut type_args = Vec::new();
            
            // 最初の型引数
            type_args.push(self.parse_type_expression()?);
            
            // 残りの型引数（カンマ区切り）
            while self.match_token(&TokenKind::Comma) {
                type_args.push(self.parse_type_expression()?);
            }
            
            self.consume(&TokenKind::Greater, "ジェネリック型パラメータの終了には '>' が必要です")?;
            
            let id = self.next_id();
            Ok(Expression::new(
                ExpressionKind::GenericType {
                    base_type: Box::new(type_name),
                    type_args,
                },
                id,
                type_name.location.clone(),
            ))
        } else {
            // 単純な型名
            Ok(type_name)
        }
    }
}

/// テスト
#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::lexer;
    use crate::frontend::ast::ExpressionKind;
    
    #[test]
    fn test_parse_expression_statement() {
        let source = "42;";
        let tokens = lexer::tokenize(source, "test.swl").unwrap();
        let mut parser = Parser::new(&tokens, "test.swl");
        
        let stmt = parser.parse_statement().unwrap();
        match &stmt.kind {
            StatementKind::ExpressionStmt(expr) => {
                match &expr.kind {
                    ExpressionKind::Literal(_) => {},
                    _ => panic!("Expected literal expression"),
                }
            },
            _ => panic!("Expected expression statement"),
        }
    }
    
    #[test]
    fn test_parse_block() {
        let source = "{ 42; 10; }";
        let tokens = lexer::tokenize(source, "test.swl").unwrap();
        let mut parser = Parser::new(&tokens, "test.swl");
        
        let stmt = parser.parse_statement().unwrap();
        match &stmt.kind {
            StatementKind::Block(statements) => {
                assert_eq!(statements.len(), 2);
            },
            _ => panic!("Expected block statement"),
        }
    }
    
    #[test]
    fn test_parse_if_statement() {
        let source = "if (x > 0) { return 1; } else { return 0; }";
        let tokens = lexer::tokenize(source, "test.swl").unwrap();
        let mut parser = Parser::new(&tokens, "test.swl");
        
        let stmt = parser.parse_statement().unwrap();
        match &stmt.kind {
            StatementKind::IfStmt { condition: _, then_branch: _, else_branch } => {
                assert!(else_branch.is_some());
            },
            _ => panic!("Expected if statement"),
        }
    }
    
    #[test]
    fn test_parse_variable_declaration() {
        let source = "let x: int = 42;";
        let tokens = lexer::tokenize(source, "test.swl").unwrap();
        let mut parser = Parser::new(&tokens, "test.swl");
        
        let stmt = parser.parse_statement().unwrap();
        match &stmt.kind {
            StatementKind::VariableDeclaration { name, type_annotation, initializer, is_mutable } => {
                assert!(type_annotation.is_some());
                assert!(initializer.is_some());
                assert_eq!(*is_mutable, false);
            },
            _ => panic!("Expected variable declaration"),
        }
    }
    
    #[test]
    fn test_parse_match_statement() {
        let source = "match x { 1 => 10, 2 => 20, _ => 0 }";
        let tokens = lexer::tokenize(source, "test.swl").unwrap();
        let mut parser = Parser::new(&tokens, "test.swl");
        
        let stmt = parser.parse_statement().unwrap();
        match &stmt.kind {
            StatementKind::Match { value: _, arms } => {
                assert_eq!(arms.len(), 3);
            },
            _ => panic!("Expected match statement"),
        }
    }
    
    #[test]
    fn test_parse_loop_statement() {
        let source = "loop { if x > 10 { break; } x = x + 1; }";
        let tokens = lexer::tokenize(source, "test.swl").unwrap();
        let mut parser = Parser::new(&tokens, "test.swl");
        
        let stmt = parser.parse_statement().unwrap();
        match &stmt.kind {
            StatementKind::Loop { body } => {
                match &body.kind {
                    StatementKind::Block(_) => {},
                    _ => panic!("Expected block as loop body"),
                }
            },
            _ => panic!("Expected loop statement"),
        }
    }
} 
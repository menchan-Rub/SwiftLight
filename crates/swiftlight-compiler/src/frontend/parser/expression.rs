//! # 式の構文解析
//! 
//! SwiftLight言語の式（リテラル、演算子、関数呼び出しなど）の
//! 構文解析を担当するモジュールです。

use crate::frontend::ast::{
    BinaryOperator, Expression, ExpressionKind, Identifier, Literal,
    LiteralKind, Statement, TypeAnnotation, UnaryOperator,
};
use crate::frontend::error::{CompilerError, Result};
use crate::frontend::lexer::{Token, TokenKind};

use super::Parser;
use super::error;

impl<'a> Parser<'a> {
    /// 式を解析
    pub fn parse_expression(&mut self) -> Result<Expression> {
        self.parse_assignment()
    }
    
    /// 代入式を解析
    fn parse_assignment(&mut self) -> Result<Expression> {
        let expr = self.parse_conditional()?;
        
        // 代入演算子を検出した場合
        if self.match_token(&TokenKind::Equal) {
            let equals_token = self.peek().unwrap();
            let value = self.parse_assignment()?;
            let id = self.next_id();
            
            // 左辺が有効な代入先（変数や添字アクセスなど）かチェック
            match &expr.kind {
                ExpressionKind::Identifier(_) |
                ExpressionKind::MemberAccess { .. } |
                ExpressionKind::IndexAccess { .. } => {
                    let assignment = ExpressionKind::Assignment {
                        left: Box::new(expr),
                        right: Box::new(value),
                    };
                    Ok(Expression::new(assignment, id, Some(equals_token.location.clone())))
                },
                _ => {
                    let error = error::syntax_error(
                        "不正な代入先です",
                        equals_token,
                    );
                    Err(error)
                }
            }
        } else {
            Ok(expr)
        }
    }
    
    /// 条件式（三項演算子）を解析
    fn parse_conditional(&mut self) -> Result<Expression> {
        let expr = self.parse_logical_or()?;
        
        // 条件演算子 (? :) の検出
        if self.match_token(&TokenKind::Question) {
            let question_token = self.peek().unwrap();
            let then_branch = self.parse_expression()?;
            
            // : (コロン) を消費
            self.consume(&TokenKind::Colon, "条件式の \":\" が必要です")?;
            
            let else_branch = self.parse_conditional()?;
            let id = self.next_id();
            
            let conditional = ExpressionKind::Conditional {
                condition: Box::new(expr),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
            };
            
            Ok(Expression::new(conditional, id, Some(question_token.location.clone())))
        } else {
            Ok(expr)
        }
    }
    
    /// 論理OR式を解析
    fn parse_logical_or(&mut self) -> Result<Expression> {
        let mut expr = self.parse_logical_and()?;
        
        // || 演算子の連続を処理
        while self.match_token(&TokenKind::PipePipe) {
            let operator_token = self.peek().unwrap();
            let right = self.parse_logical_and()?;
            let id = self.next_id();
            
            expr = Expression::new(
                ExpressionKind::Binary {
                    op: BinaryOperator::LogicalOr,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                id,
                Some(operator_token.location.clone()),
            );
        }
        
        Ok(expr)
    }
    
    /// 論理AND式を解析
    fn parse_logical_and(&mut self) -> Result<Expression> {
        let mut expr = self.parse_equality()?;
        
        // && 演算子の連続を処理
        while self.match_token(&TokenKind::AmpersandAmpersand) {
            let operator_token = self.peek().unwrap();
            let right = self.parse_equality()?;
            let id = self.next_id();
            
            expr = Expression::new(
                ExpressionKind::Binary {
                    op: BinaryOperator::LogicalAnd,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                id,
                Some(operator_token.location.clone()),
            );
        }
        
        Ok(expr)
    }
    
    /// 等価比較式を解析
    fn parse_equality(&mut self) -> Result<Expression> {
        let mut expr = self.parse_comparison()?;
        
        // == と != 演算子の連続を処理
        while self.match_token(&TokenKind::EqualEqual) || self.match_token(&TokenKind::BangEqual) {
            let operator_token = self.peek().unwrap();
            let op = match operator_token.kind {
                TokenKind::EqualEqual => BinaryOperator::Equal,
                TokenKind::BangEqual => BinaryOperator::NotEqual,
                _ => unreachable!(),
            };
            
            let right = self.parse_comparison()?;
            let id = self.next_id();
            
            expr = Expression::new(
                ExpressionKind::Binary {
                    op,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                id,
                Some(operator_token.location.clone()),
            );
        }
        
        Ok(expr)
    }
    
    /// 比較式を解析
    fn parse_comparison(&mut self) -> Result<Expression> {
        let mut expr = self.parse_bitwise_or()?;
        
        // <, >, <=, >= 演算子の連続を処理
        while self.match_token(&TokenKind::Less) || 
              self.match_token(&TokenKind::Greater) ||
              self.match_token(&TokenKind::LessEqual) || 
              self.match_token(&TokenKind::GreaterEqual) {
            
            let operator_token = self.peek().unwrap();
            let op = match operator_token.kind {
                TokenKind::Less => BinaryOperator::LessThan,
                TokenKind::Greater => BinaryOperator::GreaterThan,
                TokenKind::LessEqual => BinaryOperator::LessThanEqual,
                TokenKind::GreaterEqual => BinaryOperator::GreaterThanEqual,
                _ => unreachable!(),
            };
            
            let right = self.parse_bitwise_or()?;
            let id = self.next_id();
            
            expr = Expression::new(
                ExpressionKind::Binary {
                    op,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                id,
                Some(operator_token.location.clone()),
            );
        }
        
        Ok(expr)
    }
    
    /// ビットOR式を解析
    fn parse_bitwise_or(&mut self) -> Result<Expression> {
        let mut expr = self.parse_bitwise_xor()?;
        
        // | 演算子の連続を処理
        while self.match_token(&TokenKind::Pipe) {
            let operator_token = self.peek().unwrap();
            let right = self.parse_bitwise_xor()?;
            let id = self.next_id();
            
            expr = Expression::new(
                ExpressionKind::Binary {
                    op: BinaryOperator::BitwiseOr,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                id,
                Some(operator_token.location.clone()),
            );
        }
        
        Ok(expr)
    }
    
    /// ビットXOR式を解析
    fn parse_bitwise_xor(&mut self) -> Result<Expression> {
        let mut expr = self.parse_bitwise_and()?;
        
        // ^ 演算子の連続を処理
        while self.match_token(&TokenKind::Caret) {
            let operator_token = self.peek().unwrap();
            let right = self.parse_bitwise_and()?;
            let id = self.next_id();
            
            expr = Expression::new(
                ExpressionKind::Binary {
                    op: BinaryOperator::BitwiseXor,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                id,
                Some(operator_token.location.clone()),
            );
        }
        
        Ok(expr)
    }
    
    /// ビットAND式を解析
    fn parse_bitwise_and(&mut self) -> Result<Expression> {
        let mut expr = self.parse_shift()?;
        
        // & 演算子の連続を処理
        while self.match_token(&TokenKind::Ampersand) {
            let operator_token = self.peek().unwrap();
            let right = self.parse_shift()?;
            let id = self.next_id();
            
            expr = Expression::new(
                ExpressionKind::Binary {
                    op: BinaryOperator::BitwiseAnd,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                id,
                Some(operator_token.location.clone()),
            );
        }
        
        Ok(expr)
    }
    
    /// シフト式を解析
    fn parse_shift(&mut self) -> Result<Expression> {
        let mut expr = self.parse_term()?;
        
        // << と >> 演算子の連続を処理
        while self.match_token(&TokenKind::LessLess) || self.match_token(&TokenKind::GreaterGreater) {
            let operator_token = self.peek().unwrap();
            let op = match operator_token.kind {
                TokenKind::LessLess => BinaryOperator::LeftShift,
                TokenKind::GreaterGreater => BinaryOperator::RightShift,
                _ => unreachable!(),
            };
            
            let right = self.parse_term()?;
            let id = self.next_id();
            
            expr = Expression::new(
                ExpressionKind::Binary {
                    op,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                id,
                Some(operator_token.location.clone()),
            );
        }
        
        Ok(expr)
    }
    
    /// 項を解析（加算・減算）
    fn parse_term(&mut self) -> Result<Expression> {
        let mut expr = self.parse_factor()?;
        
        // + と - 演算子の連続を処理
        while self.match_token(&TokenKind::Plus) || self.match_token(&TokenKind::Minus) {
            let operator_token = self.peek().unwrap();
            let op = match operator_token.kind {
                TokenKind::Plus => BinaryOperator::Add,
                TokenKind::Minus => BinaryOperator::Subtract,
                _ => unreachable!(),
            };
            
            let right = self.parse_factor()?;
            let id = self.next_id();
            
            expr = Expression::new(
                ExpressionKind::Binary {
                    op,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                id,
                Some(operator_token.location.clone()),
            );
        }
        
        Ok(expr)
    }
    
    /// 因子を解析（乗算・除算・剰余）
    fn parse_factor(&mut self) -> Result<Expression> {
        let mut expr = self.parse_power()?;
        
        // *, /, % 演算子の連続を処理
        while self.match_token(&TokenKind::Star) || 
              self.match_token(&TokenKind::Slash) || 
              self.match_token(&TokenKind::Percent) {
            
            let operator_token = self.peek().unwrap();
            let op = match operator_token.kind {
                TokenKind::Star => BinaryOperator::Multiply,
                TokenKind::Slash => BinaryOperator::Divide,
                TokenKind::Percent => BinaryOperator::Modulo,
                _ => unreachable!(),
            };
            
            let right = self.parse_power()?;
            let id = self.next_id();
            
            expr = Expression::new(
                ExpressionKind::Binary {
                    op,
                    left: Box::new(expr),
                    right: Box::new(right),
                },
                id,
                Some(operator_token.location.clone()),
            );
        }
        
        Ok(expr)
    }
    
    /// べき乗式を解析
    fn parse_power(&mut self) -> Result<Expression> {
        let expr = self.parse_unary()?;
        
        // ** 演算子（べき乗）を処理
        if self.match_token(&TokenKind::DoubleStar) {
            let operator_token = self.peek().unwrap();
            // べき乗は右結合なので再帰的に処理
            let right = self.parse_power()?;
            let id = self.next_id();
            
            let power_expr = ExpressionKind::Binary {
                op: BinaryOperator::Power,
                left: Box::new(expr),
                right: Box::new(right),
            };
            
            Ok(Expression::new(power_expr, id, Some(operator_token.location.clone())))
        } else {
            Ok(expr)
        }
    }
    
    /// 単項式を解析
    fn parse_unary(&mut self) -> Result<Expression> {
        // 単項演算子 (+, -, !, ~) を処理
        if self.match_token(&TokenKind::Plus) || 
           self.match_token(&TokenKind::Minus) || 
           self.match_token(&TokenKind::Bang) || 
           self.match_token(&TokenKind::Tilde) {
            
            let operator_token = self.peek().unwrap();
            let op = match operator_token.kind {
                TokenKind::Plus => UnaryOperator::Plus,
                TokenKind::Minus => UnaryOperator::Minus,
                TokenKind::Bang => UnaryOperator::Not,
                TokenKind::Tilde => UnaryOperator::BitwiseNot,
                _ => unreachable!(),
            };
            
            let operand = self.parse_unary()?;
            let id = self.next_id();
            
            let unary_expr = ExpressionKind::Unary {
                op,
                operand: Box::new(operand),
            };
            
            Ok(Expression::new(unary_expr, id, Some(operator_token.location.clone())))
        } else {
            self.parse_call()
        }
    }
    
    /// 関数呼び出しとメンバアクセスを解析
    fn parse_call(&mut self) -> Result<Expression> {
        let mut expr = self.parse_primary()?;
        
        loop {
            if self.match_token(&TokenKind::LeftParen) {
                // 関数呼び出し
                expr = self.finish_call(expr)?;
            } else if self.match_token(&TokenKind::Dot) {
                // メンバアクセス (obj.member)
                let dot_token = self.peek().unwrap();
                
                // メンバ名（識別子）を消費
                let member_token = self.consume(&TokenKind::Identifier("".to_string()), "プロパティ名が必要です")?;
                let id = self.next_id();
                
                if let TokenKind::Identifier(name) = &member_token.kind {
                    let member = Identifier::new(name.clone(), Some(member_token.location.clone()));
                    let member_access = ExpressionKind::MemberAccess {
                        object: Box::new(expr),
                        member,
                    };
                    
                    expr = Expression::new(member_access, id, Some(dot_token.location.clone()));
                } else {
                    return Err(error::syntax_error(
                        "プロパティ名が必要です",
                        member_token,
                    ));
                }
            } else if self.match_token(&TokenKind::LeftBracket) {
                // インデックスアクセス (array[index])
                let bracket_token = self.peek().unwrap();
                
                // インデックス式を解析
                let index = self.parse_expression()?;
                
                // 閉じ括弧を消費
                self.consume(&TokenKind::RightBracket, "インデックスアクセスの \"]\" が必要です")?;
                
                let id = self.next_id();
                let index_access = ExpressionKind::IndexAccess {
                    array: Box::new(expr),
                    index: Box::new(index),
                };
                
                expr = Expression::new(index_access, id, Some(bracket_token.location.clone()));
            } else {
                break;
            }
        }
        
        Ok(expr)
    }
    
    /// 関数呼び出しの引数リストを解析
    fn finish_call(&mut self, callee: Expression) -> Result<Expression> {
        let paren_token = self.peek().unwrap();
        let mut arguments = Vec::new();
        
        // 引数がない場合
        if !self.check(&TokenKind::RightParen) {
            // 最初の引数を解析
            arguments.push(self.parse_expression()?);
            
            // カンマ区切りの引数リストを処理
            while self.match_token(&TokenKind::Comma) {
                // 最大引数数チェック
                if arguments.len() >= 255 {
                    let comma_token = self.peek().unwrap();
                    return Err(error::syntax_error(
                        "関数の引数は255個までです",
                        comma_token,
                    ));
                }
                
                arguments.push(self.parse_expression()?);
            }
        }
        
        // 閉じ括弧を消費
        self.consume(&TokenKind::RightParen, "関数呼び出しの \")\" が必要です")?;
        
        let id = self.next_id();
        let call = ExpressionKind::Call {
            callee: Box::new(callee),
            arguments,
        };
        
        Ok(Expression::new(call, id, Some(paren_token.location.clone())))
    }
    
    /// 基本式を解析（リテラル、グループ化、識別子など）
    fn parse_primary(&mut self) -> Result<Expression> {
        if let Some(token) = self.peek() {
            // 現在のトークンに基づいて適切な式を解析
            match &token.kind {
                // リテラル
                TokenKind::True => {
                    self.advance();
                    let id = self.next_id();
                    let literal = Literal::boolean(true, Some(token.location.clone()));
                    Ok(Expression::new(ExpressionKind::Literal(literal), id, Some(token.location.clone())))
                },
                TokenKind::False => {
                    self.advance();
                    let id = self.next_id();
                    let literal = Literal::boolean(false, Some(token.location.clone()));
                    Ok(Expression::new(ExpressionKind::Literal(literal), id, Some(token.location.clone())))
                },
                TokenKind::Nil => {
                    self.advance();
                    let id = self.next_id();
                    let literal = Literal::nil(Some(token.location.clone()));
                    Ok(Expression::new(ExpressionKind::Literal(literal), id, Some(token.location.clone())))
                },
                TokenKind::IntLiteral(value) => {
                    self.advance();
                    let id = self.next_id();
                    // パースして値を取得
                    let int_value = if value.starts_with("0x") || value.starts_with("0X") {
                        i64::from_str_radix(&value[2..], 16)
                    } else if value.starts_with("0o") || value.starts_with("0O") {
                        i64::from_str_radix(&value[2..], 8)
                    } else if value.starts_with("0b") || value.starts_with("0B") {
                        i64::from_str_radix(&value[2..], 2)
                    } else {
                        value.parse::<i64>()
                    }.unwrap_or(0); // エラー時はデフォルト値
                    
                    let literal = Literal::integer(int_value, Some(token.location.clone()));
                    Ok(Expression::new(ExpressionKind::Literal(literal), id, Some(token.location.clone())))
                },
                TokenKind::FloatLiteral(value) => {
                    self.advance();
                    let id = self.next_id();
                    let float_value = value.parse::<f64>().unwrap_or(0.0); // エラー時はデフォルト値
                    let literal = Literal::float(float_value, Some(token.location.clone()));
                    Ok(Expression::new(ExpressionKind::Literal(literal), id, Some(token.location.clone())))
                },
                TokenKind::StringLiteral(value) => {
                    self.advance();
                    let id = self.next_id();
                    let literal = Literal::string(value.clone(), Some(token.location.clone()));
                    Ok(Expression::new(ExpressionKind::Literal(literal), id, Some(token.location.clone())))
                },
                TokenKind::CharLiteral(value) => {
                    self.advance();
                    let id = self.next_id();
                    let literal = Literal::character(*value, Some(token.location.clone()));
                    Ok(Expression::new(ExpressionKind::Literal(literal), id, Some(token.location.clone())))
                },
                
                // 識別子
                TokenKind::Identifier(name) => {
                    self.advance();
                    let id = self.next_id();
                    let identifier = Identifier::new(name.clone(), Some(token.location.clone()));
                    Ok(Expression::new(ExpressionKind::Identifier(identifier), id, Some(token.location.clone())))
                },
                
                // グループ化式
                TokenKind::LeftParen => {
                    self.advance(); // '(' を消費
                    let expr = self.parse_expression()?;
                    self.consume(&TokenKind::RightParen, "式の後に ')' が必要です")?;
                    Ok(expr)
                },
                
                // 配列リテラル
                TokenKind::LeftBracket => {
                    self.advance(); // '[' を消費
                    let start_token = token;
                    let mut elements = Vec::new();
                    
                    // 空の配列の場合
                    if !self.check(&TokenKind::RightBracket) {
                        // 最初の要素を解析
                        elements.push(self.parse_expression()?);
                        
                        // カンマ区切りの要素を処理
                        while self.match_token(&TokenKind::Comma) {
                            elements.push(self.parse_expression()?);
                        }
                    }
                    
                    // 閉じ括弧を消費
                    self.consume(&TokenKind::RightBracket, "配列リテラルの \"]\" が必要です")?;
                    
                    let id = self.next_id();
                    let array = ExpressionKind::ArrayLiteral(elements);
                    Ok(Expression::new(array, id, Some(start_token.location.clone())))
                },
                
                // オブジェクトリテラル
                TokenKind::LeftBrace => {
                    self.advance(); // '{' を消費
                    let start_token = token;
                    let mut properties = Vec::new();
                    
                    // 空のオブジェクトの場合
                    if !self.check(&TokenKind::RightBrace) {
                        // 最初のプロパティを解析
                        let key = self.parse_identifier()?;
                        self.consume(&TokenKind::Colon, "オブジェクトリテラルのキーの後に ':' が必要です")?;
                        let value = self.parse_expression()?;
                        properties.push((key, value));
                        
                        // カンマ区切りのプロパティを処理
                        while self.match_token(&TokenKind::Comma) {
                            let key = self.parse_identifier()?;
                            self.consume(&TokenKind::Colon, "オブジェクトリテラルのキーの後に ':' が必要です")?;
                            let value = self.parse_expression()?;
                            properties.push((key, value));
                        }
                    }
                    
                    // 閉じ括弧を消費
                    self.consume(&TokenKind::RightBrace, "オブジェクトリテラルの \"}\" が必要です")?;
                    
                    let id = self.next_id();
                    let object = ExpressionKind::ObjectLiteral(properties);
                    Ok(Expression::new(object, id, Some(start_token.location.clone())))
                },
                
                // それ以外の場合はエラー
                _ => Err(error::invalid_expression_error(token)),
            }
        } else {
            // トークンがない場合（ファイル終端）
            let error = CompilerError::syntax_error(
                "式が必要ですが、ファイルが終了しました",
                None,
            );
            Err(error)
        }
    }
    
    /// 識別子を解析
    fn parse_identifier(&mut self) -> Result<Identifier> {
        let token = self.peek().unwrap();
        
        if let TokenKind::Identifier(name) = &token.kind {
            self.advance();
            Ok(Identifier::new(name.clone(), Some(token.location.clone())))
        } else {
            Err(error::syntax_error(
                "識別子が必要です",
                token,
            ))
        }
    }
    
    /// 現在のトークンの種類をチェック（消費せず）
    fn check(&self, kind: &TokenKind) -> bool {
        if let Some(token_kind) = self.peek_kind() {
            std::mem::discriminant(token_kind) == std::mem::discriminant(kind)
        } else {
            false
        }
    }
}

/// テスト
#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::lexer;
    
    #[test]
    fn test_parse_literal() {
        let source = "42";
        let tokens = lexer::tokenize(source, "test.swl").unwrap();
        let mut parser = Parser::new(&tokens, "test.swl");
        
        let expr = parser.parse_expression().unwrap();
        match &expr.kind {
            ExpressionKind::Literal(lit) => {
                match &lit.kind {
                    LiteralKind::Integer(value) => assert_eq!(*value, 42),
                    _ => panic!("Expected integer literal"),
                }
            },
            _ => panic!("Expected literal expression"),
        }
    }
    
    #[test]
    fn test_parse_binary_expression() {
        let source = "2 + 3 * 4";
        let tokens = lexer::tokenize(source, "test.swl").unwrap();
        let mut parser = Parser::new(&tokens, "test.swl");
        
        let expr = parser.parse_expression().unwrap();
        match &expr.kind {
            ExpressionKind::Binary { op, left, right } => {
                assert_eq!(*op, BinaryOperator::Add);
                
                // 左辺は整数リテラル 2
                match &left.kind {
                    ExpressionKind::Literal(lit) => {
                        match &lit.kind {
                            LiteralKind::Integer(value) => assert_eq!(*value, 2),
                            _ => panic!("Expected integer literal"),
                        }
                    },
                    _ => panic!("Expected literal expression"),
                }
                
                // 右辺は 3 * 4
                match &right.kind {
                    ExpressionKind::Binary { op, left, right } => {
                        assert_eq!(*op, BinaryOperator::Multiply);
                        
                        // 3
                        match &left.kind {
                            ExpressionKind::Literal(lit) => {
                                match &lit.kind {
                                    LiteralKind::Integer(value) => assert_eq!(*value, 3),
                                    _ => panic!("Expected integer literal"),
                                }
                            },
                            _ => panic!("Expected literal expression"),
                        }
                        
                        // 4
                        match &right.kind {
                            ExpressionKind::Literal(lit) => {
                                match &lit.kind {
                                    LiteralKind::Integer(value) => assert_eq!(*value, 4),
                                    _ => panic!("Expected integer literal"),
                                }
                            },
                            _ => panic!("Expected literal expression"),
                        }
                    },
                    _ => panic!("Expected binary expression"),
                }
            },
            _ => panic!("Expected binary expression"),
        }
    }
} 
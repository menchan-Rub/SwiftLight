//! # 型注釈の構文解析
//! 
//! SwiftLight言語の型注釈（基本型、ジェネリック型、関数型など）の
//! 構文解析を担当するモジュールです。

use crate::frontend::ast::{Identifier, TypeAnnotation};
use crate::frontend::error::Result;
use crate::frontend::lexer::TokenKind;

use super::Parser;
use super::error;

impl<'a> Parser<'a> {
    /// 型注釈を解析
    pub(crate) fn parse_type(&mut self) -> Result<TypeAnnotation> {
        let type_expr = self.parse_union_type()?;
        Ok(type_expr)
    }
    
    /// ユニオン型を解析 (T | U)
    fn parse_union_type(&mut self) -> Result<TypeAnnotation> {
        let mut types = vec![self.parse_intersection_type()?];
        
        // | 演算子の連続を処理
        while self.match_token(&TokenKind::Pipe) {
            types.push(self.parse_intersection_type()?);
        }
        
        // 単一の型の場合はそのまま返す
        if types.len() == 1 {
            Ok(types.remove(0))
        } else {
            Ok(TypeAnnotation::Union(types))
        }
    }
    
    /// 交差型を解析 (T & U)
    fn parse_intersection_type(&mut self) -> Result<TypeAnnotation> {
        let mut types = vec![self.parse_primary_type()?];
        
        // & 演算子の連続を処理
        while self.match_token(&TokenKind::Ampersand) {
            types.push(self.parse_primary_type()?);
        }
        
        // 単一の型の場合はそのまま返す
        if types.len() == 1 {
            Ok(types.remove(0))
        } else {
            Ok(TypeAnnotation::Intersection(types))
        }
    }
    
    /// 基本型を解析
    fn parse_primary_type(&mut self) -> Result<TypeAnnotation> {
        // 現在のトークンに基づいて型を解析
        match self.peek_kind() {
            // 括弧で囲まれた型や関数型 ((A, B) -> C)
            Some(TokenKind::LeftParen) => {
                self.advance(); // '(' を消費
                
                // 空の括弧はユニット型として処理
                if self.match_token(&TokenKind::RightParen) {
                    // 関数型かどうか
                    if self.match_token(&TokenKind::Arrow) {
                        // () -> T 形式の関数型
                        let return_type = Box::new(self.parse_type()?);
                        Ok(TypeAnnotation::Function {
                            parameters: Vec::new(),
                            return_type,
                        })
                    } else {
                        // ユニット型 () は空のタプル型として表現
                        Ok(TypeAnnotation::Tuple(Vec::new()))
                    }
                } else {
                    // 最初の型を解析
                    let first_type = self.parse_type()?;
                    
                    if self.match_token(&TokenKind::Comma) {
                        // (A, B, ...) 形式のタプル型
                        let mut types = vec![first_type];
                        
                        // カンマ区切りの型リスト
                        while !self.check(&TokenKind::RightParen) && !self.is_at_end() {
                            types.push(self.parse_type()?);
                            
                            // リストの続きがあるか確認
                            if !self.match_token(&TokenKind::Comma) {
                                break;
                            }
                        }
                        
                        self.consume(&TokenKind::RightParen, "タプル型の終了には ')' が必要です")?;
                        
                        // 関数型かどうか
                        if self.match_token(&TokenKind::Arrow) {
                            // (A, B, ...) -> T 形式の関数型
                            let return_type = Box::new(self.parse_type()?);
                            Ok(TypeAnnotation::Function {
                                parameters: types,
                                return_type,
                            })
                        } else {
                            // タプル型
                            Ok(TypeAnnotation::Tuple(types))
                        }
                    } else if self.match_token(&TokenKind::RightParen) {
                        // 関数型かどうか
                        if self.match_token(&TokenKind::Arrow) {
                            // (A) -> T 形式の関数型
                            // 注: 単一のパラメータの場合は括弧は省略可能だが、ここでは明示的に使われている
                            let return_type = Box::new(self.parse_type()?);
                            Ok(TypeAnnotation::Function {
                                parameters: vec![first_type],
                                return_type,
                            })
                        } else {
                            // (A) は単にグループ化されたA
                            Ok(first_type)
                        }
                    } else {
                        // 予期しないトークン
                        let token = self.peek().unwrap();
                        Err(error::syntax_error(
                            "タプル型または関数型が不正です",
                            token,
                        ))
                    }
                }
            },
            
            // 配列型 [T]
            Some(TokenKind::LeftBracket) => {
                self.advance(); // '[' を消費
                let element_type = Box::new(self.parse_type()?);
                self.consume(&TokenKind::RightBracket, "配列型の終了には ']' が必要です")?;
                
                // オプショナル修飾子を処理
                self.parse_optional_modifier(TypeAnnotation::Array(element_type))
            },
            
            // 識別子型 (Int, String など)
            Some(TokenKind::Identifier(_)) => {
                let name = self.parse_identifier()?;
                
                // ジェネリック型か確認 (例: Array<T>)
                if self.match_token(&TokenKind::Less) {
                    let mut arguments = Vec::new();
                    
                    // 最初の型引数
                    arguments.push(self.parse_type()?);
                    
                    // カンマ区切りの型引数リスト
                    while self.match_token(&TokenKind::Comma) {
                        arguments.push(self.parse_type()?);
                    }
                    
                    self.consume(&TokenKind::Greater, "ジェネリック型の終了には '>' が必要です")?;
                    
                    let base = Box::new(TypeAnnotation::Named(name));
                    let generic_type = TypeAnnotation::Generic { base, arguments };
                    
                    // オプショナル修飾子を処理
                    self.parse_optional_modifier(generic_type)
                } else {
                    // 通常の名前付き型
                    let named_type = TypeAnnotation::Named(name);
                    
                    // オプショナル修飾子を処理
                    self.parse_optional_modifier(named_type)
                }
            },
            
            // 依存型 (依存型の構文解析)
            Some(TokenKind::Dependent) => {
                self.advance(); // 'dependent' キーワードを消費
                self.consume(&TokenKind::LeftParen, "依存型の開始には '(' が必要です")?;
                
                // 依存変数名
                let var_name = self.parse_identifier()?;
                
                self.consume(&TokenKind::Colon, "依存型の変数と型の区切りには ':' が必要です")?;
                
                // 依存変数の型
                let var_type = Box::new(self.parse_type()?);
                
                self.consume(&TokenKind::RightParen, "依存型の変数定義の終了には ')' が必要です")?;
                self.consume(&TokenKind::Arrow, "依存型の本体には '->' が必要です")?;
                
                // 依存型の本体
                let body = Box::new(self.parse_type()?);
                
                Ok(TypeAnnotation::Dependent {
                    var_name,
                    var_type,
                    body,
                })
            },
            
            // 参照型 (&T, &mut T)
            Some(TokenKind::Ampersand) => {
                self.advance(); // '&' を消費
                
                // 可変参照かどうか
                let is_mutable = self.match_token(&TokenKind::Mut);
                
                // 参照先の型
                let target_type = Box::new(self.parse_primary_type()?);
                
                // ライフタイム注釈があるか確認
                let lifetime = if self.match_token(&TokenKind::Lifetime) {
                    let lifetime_token = self.previous().unwrap();
                    if let TokenKind::Lifetime(name) = &lifetime_token.kind {
                        Some(name.clone())
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                Ok(TypeAnnotation::Reference {
                    target_type,
                    is_mutable,
                    lifetime,
                })
            },
            
            // その他の型（エラー）
            _ => {
                let token = self.peek().unwrap();
                Err(error::invalid_type_error(token))
            },
        }
    }
    
    /// 型のオプショナル修飾子を処理
    fn parse_optional_modifier(&mut self, base_type: TypeAnnotation) -> Result<TypeAnnotation> {
        if self.match_token(&TokenKind::Question) {
            Ok(TypeAnnotation::Optional(Box::new(base_type)))
        } else {
            Ok(base_type)
        }
    }
}

/// テスト
#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::lexer;
    
    #[test]
    fn test_parse_simple_type() {
        let source = "Int";
        let tokens = lexer::tokenize(source, "test.swl").unwrap();
        let mut parser = Parser::new(&tokens, "test.swl");
        
        let ty = parser.parse_type().unwrap();
        match &ty {
            TypeAnnotation::Named(ident) => assert_eq!(ident.name, "Int"),
            _ => panic!("Expected named type"),
        }
    }
    
    #[test]
    fn test_parse_array_type() {
        let source = "[String]";
        let tokens = lexer::tokenize(source, "test.swl").unwrap();
        let mut parser = Parser::new(&tokens, "test.swl");
        
        let ty = parser.parse_type().unwrap();
        match &ty {
            TypeAnnotation::Array(element) => {
                match &**element {
                    TypeAnnotation::Named(ident) => assert_eq!(ident.name, "String"),
                    _ => panic!("Expected named type"),
                }
            },
            _ => panic!("Expected array type"),
        }
    }
    
    #[test]
    fn test_parse_generic_type() {
        let source = "HashMap<String, Int>";
        let tokens = lexer::tokenize(source, "test.swl").unwrap();
        let mut parser = Parser::new(&tokens, "test.swl");
        
        let ty = parser.parse_type().unwrap();
        match &ty {
            TypeAnnotation::Generic { base, arguments } => {
                match &**base {
                    TypeAnnotation::Named(ident) => assert_eq!(ident.name, "HashMap"),
                    _ => panic!("Expected named type"),
                }
                assert_eq!(arguments.len(), 2);
            },
            _ => panic!("Expected generic type"),
        }
    }
    
    #[test]
    fn test_parse_function_type() {
        let source = "(Int, String) -> Bool";
        let tokens = lexer::tokenize(source, "test.swl").unwrap();
        let mut parser = Parser::new(&tokens, "test.swl");
        
        let ty = parser.parse_type().unwrap();
        match &ty {
            TypeAnnotation::Function { parameters, return_type } => {
                assert_eq!(parameters.len(), 2);
                match &**return_type {
                    TypeAnnotation::Named(ident) => assert_eq!(ident.name, "Bool"),
                    _ => panic!("Expected named type"),
                }
            },
            _ => panic!("Expected function type"),
        }
    }
    
    #[test]
    fn test_parse_optional_type() {
        let source = "String?";
        let tokens = lexer::tokenize(source, "test.swl").unwrap();
        let mut parser = Parser::new(&tokens, "test.swl");
        
        let ty = parser.parse_type().unwrap();
        match &ty {
            TypeAnnotation::Optional(inner) => {
                match &**inner {
                    TypeAnnotation::Named(ident) => assert_eq!(ident.name, "String"),
                    _ => panic!("Expected named type"),
                }
            },
            _ => panic!("Expected optional type"),
        }
    }
    
    #[test]
    fn test_parse_reference_type() {
        let source = "&mut String";
        let tokens = lexer::tokenize(source, "test.swl").unwrap();
        let mut parser = Parser::new(&tokens, "test.swl");
        
        let ty = parser.parse_type().unwrap();
        match &ty {
            TypeAnnotation::Reference { target_type, is_mutable, lifetime } => {
                assert!(*is_mutable);
                assert!(lifetime.is_none());
                match &**target_type {
                    TypeAnnotation::Named(ident) => assert_eq!(ident.name, "String"),
                    _ => panic!("Expected named type"),
                }
            },
            _ => panic!("Expected reference type"),
        }
    }
    
    #[test]
    fn test_parse_dependent_type() {
        let source = "dependent(n: Int) -> Array<Int>";
        let tokens = lexer::tokenize(source, "test.swl").unwrap();
        let mut parser = Parser::new(&tokens, "test.swl");
        
        let ty = parser.parse_type().unwrap();
        match &ty {
            TypeAnnotation::Dependent { var_name, var_type, body } => {
                assert_eq!(var_name.name, "n");
                match &**var_type {
                    TypeAnnotation::Named(ident) => assert_eq!(ident.name, "Int"),
                    _ => panic!("Expected named type for var_type"),
                }
                match &**body {
                    TypeAnnotation::Generic { base, arguments: _ } => {
                        match &**base {
                            TypeAnnotation::Named(ident) => assert_eq!(ident.name, "Array"),
                            _ => panic!("Expected named type for base"),
                        }
                    },
                    _ => panic!("Expected generic type for body"),
                }
            },
            _ => panic!("Expected dependent type"),
        }
    }
    
    #[test]
    fn test_parse_union_type() {
        let source = "Int | String | Bool";
        let tokens = lexer::tokenize(source, "test.swl").unwrap();
        let mut parser = Parser::new(&tokens, "test.swl");
        
        let ty = parser.parse_type().unwrap();
        match &ty {
            TypeAnnotation::Union(types) => {
                assert_eq!(types.len(), 3);
            },
            _ => panic!("Expected union type"),
        }
    }
    
    #[test]
    fn test_parse_intersection_type() {
        let source = "Comparable & Hashable & Equatable";
        let tokens = lexer::tokenize(source, "test.swl").unwrap();
        let mut parser = Parser::new(&tokens, "test.swl");
        
        let ty = parser.parse_type().unwrap();
        match &ty {
            TypeAnnotation::Intersection(types) => {
                assert_eq!(types.len(), 3);
            },
            _ => panic!("Expected intersection type"),
        }
    }
}
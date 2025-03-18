//! # 宣言の構文解析
//! 
//! SwiftLight言語の宣言（変数宣言、関数宣言、構造体定義など）の
//! 構文解析を担当するモジュールです。

use crate::frontend::ast::{
    EnumVariant, Parameter, Statement, StatementKind,
    StructField, TraitMethod,
};
use crate::frontend::error::Result;
use crate::frontend::lexer::TokenKind;

use super::Parser;
use super::error;

impl<'a> Parser<'a> {
    /// 宣言を解析
    pub(crate) fn parse_declaration(&mut self) -> Result<Statement> {
        if self.match_token(&TokenKind::Let) {
            self.backup();
            return self.parse_variable_declaration();
        } else if self.match_token(&TokenKind::Const) {
            self.backup();
            return self.parse_constant_declaration();
        } else if self.match_token(&TokenKind::Func) {
            self.backup();
            return self.parse_function_declaration();
        } else if self.match_token(&TokenKind::Struct) {
            self.backup();
            return self.parse_struct_declaration();
        } else if self.match_token(&TokenKind::Enum) {
            self.backup();
            return self.parse_enum_declaration();
        } else if self.match_token(&TokenKind::Trait) {
            self.backup();
            return self.parse_trait_declaration();
        } else if self.match_token(&TokenKind::Impl) {
            self.backup();
            return self.parse_impl_declaration();
        } else if self.match_token(&TokenKind::Import) {
            self.backup();
            return self.parse_import_declaration();
        }

        Err(error::syntax_error(
            "宣言が期待されていますが、見つかりませんでした",
            self.peek().unwrap(),
        ))
    }

    /// 変数宣言を解析
    pub(crate) fn parse_variable_declaration(&mut self) -> Result<Statement> {
        let let_token = self.consume(&TokenKind::Let, "変数宣言には 'let' キーワードが必要です")?;
        
        // 可変変数かどうか
        let is_mutable = self.match_token(&TokenKind::Mut);
        
        // 変数名（識別子トークンを直接消費）
        let name = self.consume_identifier("変数名が必要です")?;
        
        // 型注釈（オプション）
        let type_annotation = if self.match_token(&TokenKind::Colon) {
            Some(self.parse_type()?)
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
        let var_decl = StatementKind::VariableDeclaration(
            VariableData {
                keyword_location: let_token.location.clone(),
                name,
                type_annotation,
                initializer,
                is_mutable,
            }
        );
        
        Ok(Statement::new(var_decl, id, Some(let_token.location.clone())))
    }
    /// 定数宣言を解析
    pub(crate) fn parse_constant_declaration(&mut self) -> Result<Statement> {
        let const_token = self.consume(&TokenKind::Const, "定数宣言には 'const' キーワードが必要です")?;
        
        // 定数名
        let name = self.parse_identifier()?;
        
        // 型注釈（オプション）
        let type_annotation = if self.match_token(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        
        // 初期化式（必須）
        self.consume(&TokenKind::Equal, "定数宣言には初期化式が必要です")?;
        let initializer = self.parse_expression()?;
        
        // セミコロンは省略可能
        self.match_token(&TokenKind::Semicolon);
        let id = self.next_id();
        let const_decl = StatementKind::ConstantDeclaration(
            ConstantData {
                keyword_location: const_token.location.clone(),
                name,
                type_annotation,
                initializer,
            }
        );
        
        Ok(Statement::new(const_decl, id, Some(const_token.location.clone())))
    }
    
    /// 関数宣言を解析
    pub(crate) fn parse_function_declaration(&mut self) -> Result<Statement> {
        let func_token = self.consume(&TokenKind::Func, "関数宣言には 'func' キーワードが必要です")?;
        
        // 関数名
        let name = self.parse_identifier()?;
        
        // パラメータリスト
        self.consume(&TokenKind::LeftParen, "関数宣言にはパラメータリストが必要です")?;
        let params = self.parse_parameters()?;
        self.consume(&TokenKind::RightParen, "パラメータリストの終了には ')' が必要です")?;
        
        // 戻り値の型（オプション）
        let return_type = if self.match_token(&TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };
        
        // 関数本体
        let body = if self.peek().map_or(false, |t| t.kind == TokenKind::LeftBrace) {
            self.parse_block()?
        } else {
            return Err(error::syntax_error(
                "関数本体には '{' が必要です",
                self.peek().unwrap(),
            ));
        };
        
        let id = self.next_id();
        let func_decl = StatementKind::FunctionDeclaration {
            name,
            params,
            return_type,
            body: Box::new(body),
        };
        
        Ok(Statement::new(func_decl, id, Some(func_token.location.clone())))
    }
    
    /// パラメータリストを解析
    fn parse_parameters(&mut self) -> Result<Vec<Parameter>> {
        let mut params = Vec::new();
        // 空のパラメータリスト
        if self.peek().map_or(false, |t| t.kind == TokenKind::RightParen) {
            return Ok(params);
        }
        
        // 最初のパラメータ
        params.push(self.parse_parameter()?);
        
        // カンマ区切りのパラメータリスト
        while self.match_token(&TokenKind::Comma) {
            params.push(self.parse_parameter()?);
        }
        
        Ok(params)
    }
    
    /// 単一のパラメータを解析
    fn parse_parameter(&mut self) -> Result<Parameter> {
        // パラメータ名
        let name = self.parse_identifier()?;
        
        // 可変パラメータかどうか
        let is_variadic = self.match_token(&TokenKind::Ellipsis);
        
        // 型注釈
        self.consume(&TokenKind::Colon, "パラメータには型注釈が必要です")?;
        let type_annotation = Some(self.parse_type()?);
        
        // デフォルト値（オプション）
        let default_value = if self.match_token(&TokenKind::Equal) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        Ok(Parameter::new(
            name.clone(),
            type_annotation,
            default_value,
            is_variadic,
            name.location.clone(),
        ))
    }
    
    /// 構造体宣言を解析
    pub(crate) fn parse_struct_declaration(&mut self) -> Result<Statement> {
        let struct_token = self.consume(&TokenKind::Struct, "構造体宣言には 'struct' キーワードが必要です")?;
        
        // 構造体名
        let name = self.parse_identifier()?;
        
        // 構造体本体
        self.consume(&TokenKind::LeftBrace, "構造体宣言には '{' が必要です")?;
        
        // フィールドのリスト
        let mut fields = Vec::new();
        while !self.is_at_end() && self.peek().kind != TokenKind::RightBrace {
            fields.push(self.parse_struct_field()?);
            
            // カンマまたはセミコロンは省略可能
            let _ = self.match_token(&TokenKind::Comma) || self.match_token(&TokenKind::Semicolon);
        }
        
        self.consume(&TokenKind::RightBrace, "構造体宣言の終了には '}' が必要です")?;
        
        let id = self.next_id();
        let struct_decl = StatementKind::StructDeclaration {
            name,
            fields,
        };
        
        Ok(Statement::new(struct_decl, id, Some(struct_token.location.clone())))
    }
    
    /// 構造体のフィールドを解析
    fn parse_struct_field(&mut self) -> Result<StructField> {
        // フィールド名
        let name = self.parse_identifier()?;
        
        // 型注釈
        self.consume(&TokenKind::Colon, "フィールドには型注釈が必要です")?;
        let type_annotation = self.parse_type()?;
        
        // デフォルト値（オプション）
        let default_value = if self.match_token(&TokenKind::Equal) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        
        Ok(StructField::new(
            name.clone(),
            type_annotation,
            default_value,
            name.location.clone(),
        ))
    }
    
    /// 列挙型宣言を解析
    pub(crate) fn parse_enum_declaration(&mut self) -> Result<Statement> {
        let enum_token = self.consume(&TokenKind::Enum, "列挙型宣言には 'enum' キーワードが必要です")?;
        
        // 列挙型名
        let name = self.parse_identifier()?;
        
        // 列挙型本体
        self.consume(&TokenKind::LeftBrace, "列挙型宣言には '{' が必要です")?;
        
        // バリアントのリスト
        let mut variants = Vec::new();
        while self.parser.peek().kind != TokenKind::RightBrace && !self.parser.is_at_end() {
            variants.push(self.parse_enum_variant()?);
            
            // カンマまたはセミコロンは省略可能
            let _ = self.match_token(&TokenKind::Comma) || self.match_token(&TokenKind::Semicolon);
        }
        
        self.consume(&TokenKind::RightBrace, "列挙型宣言の終了には '}' が必要です")?;
        
        let id = self.next_id();
        let enum_decl = StatementKind::EnumDeclaration {
            name,
            variants,
        };
        
        Ok(Statement::new(enum_decl, id, Some(enum_token.location.clone())))
    }
    
    /// 列挙型のバリアントを解析
    fn parse_enum_variant(&mut self) -> Result<EnumVariant> {
        // バリアント名
        let name = self.parse_identifier()?;
        
        // 関連値（オプション）
        let associated_values = if self.match_token(&TokenKind::LeftParen) {
            let mut types = Vec::new();
            
            // 空の関連値リスト
            if self.parser.peek().kind != TokenKind::RightParen {
                // 最初の型
                types.push(self.parse_type()?);
                
                // カンマ区切りの型リスト
                while self.match_token(&TokenKind::Comma) {
                    types.push(self.parse_type()?);
                }
            }
            
            self.consume(&TokenKind::RightParen, "関連値リストの終了には ')' が必要です")?;
            
            Some(types)
        } else {
            None
        };
        
        Ok(EnumVariant::new(
            name.clone(),
            associated_values,
            name.location.clone(),
        ))
    }
    
    /// トレイト宣言を解析
    pub(crate) fn parse_trait_declaration(&mut self) -> Result<Statement> {
        let trait_token = self.consume(&TokenKind::Trait, "トレイト宣言には 'trait' キーワードが必要です")?;
        
        // トレイト名
        let name = self.parse_identifier()?;
        
        // トレイト本体
        self.consume(&TokenKind::LeftBrace, "トレイト宣言には '{' が必要です")?;
        
        // メソッド宣言のリスト
        let mut methods = Vec::new();
        while self.parser.peek().kind != TokenKind::RightBrace && !self.is_at_end() {
            methods.push(self.parse_trait_method()?);
        }
        
        self.consume(&TokenKind::RightBrace, "トレイト宣言の終了には '}' が必要です")?;
        
        let id = self.next_id();
        let trait_decl = StatementKind::TraitDeclaration {
            name,
            methods,
        };
        
        Ok(Statement::new(trait_decl, id, Some(trait_token.location.clone())))
    }
    
    /// トレイトのメソッド宣言を解析
    fn parse_trait_method(&mut self) -> Result<TraitMethod> {
        // 'func' キーワード
        self.consume(&TokenKind::Func, "メソッド宣言には 'func' キーワードが必要です")?;
        
        // メソッド名
        let name = self.parse_identifier()?;
        
        // パラメータリスト
        self.consume(&TokenKind::LeftParen, "メソッド宣言にはパラメータリストが必要です")?;
        let params = self.parse_parameters()?;
        self.consume(&TokenKind::RightParen, "パラメータリストの終了には ')' が必要です")?;
        
        // 戻り値の型（オプション）
        let return_type = if self.match_token(&TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };
        // デフォルト実装（オプション）
        let default_impl = if self.parser.peek().kind == TokenKind::LeftBrace {
            let body = self.parse_block()?;
            Some(Box::new(body))
        } else {
            // セミコロンは省略可能
            self.match_token(&TokenKind::Semicolon);
            None
        };
        
        Ok(TraitMethod::new(
            name.clone(),
            params,
            return_type,
            default_impl,
            name.location.clone(),
        ))
    }
    
    /// トレイト実装宣言を解析
    pub(crate) fn parse_impl_declaration(&mut self) -> Result<Statement> {
        let impl_token = self.consume(&TokenKind::Impl, "impl宣言には 'impl' キーワードが必要です")?;
        // トレイト名（オプション）
        let trait_name = if self.parser.peek().kind != TokenKind::For 
            && self.parser.peek().kind != TokenKind::LeftBrace 
        {
            Some(self.parse_identifier()?)
        } else {
            None
        };
        
        // 'for' キーワード（オプション、トレイト実装の場合）
        self.match_token(&TokenKind::For);
        
        // 実装対象の型
        let target_type = self.parse_type()?;
        
        // 実装本体
        self.consume(&TokenKind::LeftBrace, "impl宣言には '{' が必要です")?;
        
        // メソッド実装のリスト
        let mut methods = Vec::new();
        while self.parser.peek().kind != TokenKind::RightBrace && !self.is_at_end() {
            // 各メソッドは関数宣言として解析
            methods.push(self.parse_function_declaration()?);
        }
        self.consume(&TokenKind::RightBrace, "impl宣言の終了には '}' が必要です")?;
        
        let id = self.next_id();
        let impl_decl = StatementKind::ImplDeclaration {
            target_type,
            trait_name,
            methods,
        };
        
        Ok(Statement::new(impl_decl, id, Some(impl_token.location.clone())))
    }
    
    /// インポート宣言を解析
    pub(crate) fn parse_import_declaration(&mut self) -> Result<Statement> {
        let import_token = self.consume(&TokenKind::Import, "import宣言には 'import' キーワードが必要です")?;
        
        // パスのリスト
        let mut path = Vec::new();
        
        // 最初のパス要素
        path.push(self.parse_identifier()?);
        
        // ドット区切りのパス
        while self.match_token(&TokenKind::Dot) {
            path.push(self.parse_identifier()?);
        }
        
        // エイリアス（オプション）
        let alias = if self.match_token(&TokenKind::As) {
            Some(self.parse_identifier()?)
        } else {
            None
        };
        
        // セミコロンは省略可能
        self.match_token(&TokenKind::Semicolon);
        
        let id = self.next_id();
        let import_decl = StatementKind::Import {
            path,
            alias,
        };
        
        Ok(Statement::new(import_decl, id, Some(import_token.location.clone())))
    }
}

/// テスト
#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::lexer;
    
    #[test]
    fn test_parse_variable_declaration() {
        let source = "let x: Int = 42;";
        let tokens = lexer::tokenize(source, "test.swl").unwrap();
        let mut parser = Parser::new(&tokens, "test.swl");
        
        let stmt = parser.parse_declaration().unwrap();
        match &stmt.kind {
            StatementKind::VariableDeclaration(variable_decl) => {
                assert!(!variable_decl.is_mutable, "デフォルトでミュータブルでないことを確認");
                assert_eq!(variable_decl.decl.identifier.name, "x");
                assert!(variable_decl.decl.type_annotation.as_ref().expect("型注釈が存在する必要があります").is_resolved(),
                    "型注釈が解決されていることを確認");
                assert!(variable_decl.decl.initializer.as_ref().expect("初期化式が存在する必要があります").is_evaluable(),
                    "評価可能な初期化式が存在することを確認");
            },
            _ => panic!("Expected variable declaration"),
        }
    }
    #[test]
    fn test_parse_function_declaration() {
        let source = "func add(a: Int, b: Int) -> Int { return a + b; }";
        let tokens = lexer::tokenize(source, "test.swl").unwrap();
        let mut parser = Parser::new(&tokens, "test.swl");
        
        let stmt = parser.parse_declaration().unwrap();
        match &stmt.kind {
            StatementKind::FunctionDeclaration { name, params, return_type, body: _ } => {
                assert_eq!(name.name, "add");
                assert_eq!(params.len(), 2);
                assert!(return_type.is_some());
            },
            _ => panic!("Expected function declaration"),
        }
    }
    
    #[test]
    fn test_parse_struct_declaration() {
        let source = "struct Point { x: Int, y: Int }";
        let tokens = lexer::tokenize(source, "test.swl").unwrap();
        let mut parser = Parser::new(&tokens, "test.swl");
        
        let stmt = parser.parse_declaration().unwrap();
        match &stmt.kind {
            StatementKind::StructDeclaration { name, fields } => {
                assert_eq!(name.name, "Point");
                assert_eq!(fields.len(), 2);
            },
            _ => panic!("Expected struct declaration"),
        }
    }
} 
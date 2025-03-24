//! # LSP構文ハイライト機能
//!
//! このモジュールは、Language Server Protocol (LSP)の
//! semanticTokens機能を実装し、SwiftLight言語のエディタサポートを強化します。

use dashmap::DashMap;
use swiftlight_compiler::{
    frontend::{
        lexer::{Lexer, Token},
        parser::{Parser, ast},
        syntax_highlight::{SyntaxHighlighter, SyntaxRangeInfo, HighlightKind},
    },
    diagnostics::DiagnosticEmitter,
};
use lsp_types::{
    SemanticToken, SemanticTokensOptions, SemanticTokensLegend,
    SemanticTokenType, SemanticTokenModifier, SemanticTokens,
};
use std::{
    collections::HashMap,
    sync::Arc,
};

/// LSPの構文ハイライト実装
pub struct LspSyntaxHighlighter {
    /// ファイルパスとその構文情報のマップ
    pub document_syntax: DashMap<String, DocumentSyntax>,
    /// トークンタイプからLSPトークンタイプへのマッピング
    token_type_map: HashMap<HighlightKind, u32>,
    /// LSPトークンの凡例
    pub legend: SemanticTokensLegend,
    /// 診断エミッタ
    diagnostics: Arc<DiagnosticEmitter>,
}

/// ドキュメントの構文情報
#[derive(Debug, Clone)]
pub struct DocumentSyntax {
    /// ソーステキスト
    pub text: String,
    /// 構文範囲情報
    pub ranges: Vec<SyntaxRangeInfo>,
    /// 最終更新タイムスタンプ
    pub timestamp: std::time::SystemTime,
}

impl LspSyntaxHighlighter {
    /// 新しいLSP構文ハイライタを作成
    pub fn new(diagnostics: Arc<DiagnosticEmitter>) -> Self {
        let (legend, token_type_map) = Self::create_legend();
        
        Self {
            document_syntax: DashMap::new(),
            token_type_map,
            legend,
            diagnostics,
        }
    }
    
    /// LSPトークン凡例を作成
    fn create_legend() -> (SemanticTokensLegend, HashMap<HighlightKind, u32>) {
        // トークンタイプの定義
        let token_types = vec![
            SemanticTokenType::KEYWORD,
            SemanticTokenType::FUNCTION,
            SemanticTokenType::METHOD,
            SemanticTokenType::VARIABLE,
            SemanticTokenType::PARAMETER,
            SemanticTokenType::PROPERTY,
            SemanticTokenType::TYPE,
            SemanticTokenType::CLASS,
            SemanticTokenType::STRUCT,
            SemanticTokenType::INTERFACE,
            SemanticTokenType::ENUM,
            SemanticTokenType::ENUM_MEMBER,
            SemanticTokenType::STRING,
            SemanticTokenType::NUMBER,
            SemanticTokenType::COMMENT,
            SemanticTokenType::OPERATOR,
            SemanticTokenType::NAMESPACE,
            SemanticTokenType::TYPE_PARAMETER,
            SemanticTokenType::MACRO,
        ];
        
        // トークン修飾子の定義
        let token_modifiers = vec![
            SemanticTokenModifier::DECLARATION,
            SemanticTokenModifier::DEFINITION,
            SemanticTokenModifier::READONLY,
            SemanticTokenModifier::STATIC,
            SemanticTokenModifier::DEPRECATED,
            SemanticTokenModifier::ABSTRACT,
            SemanticTokenModifier::ASYNC,
            SemanticTokenModifier::MODIFICATION,
            SemanticTokenModifier::DOCUMENTATION,
            SemanticTokenModifier::DEFAULT_LIBRARY,
        ];
        
        // ハイライト種類からLSPトークンタイプへのマッピングを作成
        let mut token_type_map = HashMap::new();
        token_type_map.insert(HighlightKind::Keyword, 0);
        token_type_map.insert(HighlightKind::FunctionName, 1);
        token_type_map.insert(HighlightKind::Method, 2);
        token_type_map.insert(HighlightKind::VariableName, 3);
        token_type_map.insert(HighlightKind::Parameter, 4);
        token_type_map.insert(HighlightKind::Property, 5);
        token_type_map.insert(HighlightKind::TypeName, 6);
        token_type_map.insert(HighlightKind::UserType, 7);
        token_type_map.insert(HighlightKind::UserType, 8);
        token_type_map.insert(HighlightKind::LibraryType, 9);
        token_type_map.insert(HighlightKind::PrimitiveType, 10);
        token_type_map.insert(HighlightKind::ConstantName, 11);
        token_type_map.insert(HighlightKind::StringLiteral, 12);
        token_type_map.insert(HighlightKind::NumericLiteral, 13);
        token_type_map.insert(HighlightKind::Comment, 14);
        token_type_map.insert(HighlightKind::Operator, 15);
        token_type_map.insert(HighlightKind::Module, 16);
        token_type_map.insert(HighlightKind::TypeName, 17);
        token_type_map.insert(HighlightKind::Macro, 18);
        
        let legend = SemanticTokensLegend {
            token_types,
            token_modifiers,
        };
        
        (legend, token_type_map)
    }
    
    /// LSP構文ハイライト機能のオプションを取得
    pub fn get_options(&self) -> SemanticTokensOptions {
        SemanticTokensOptions {
            legend: self.legend.clone(),
            range: Some(true),
            full: Some(true),
        }
    }
    
    /// ドキュメントの構文ハイライトを更新
    pub fn update_document(&self, uri: &str, content: &str) {
        // ソースコードを字句解析
        let mut lexer = Lexer::new(content);
        let tokens_result = lexer.tokenize();
        
        match tokens_result {
            Ok(tokens) => {
                // 構文解析してASTを取得
                let mut parser = Parser::new(tokens.clone());
                let ast_result = parser.parse();
                
                let syntax_highlighter = SyntaxHighlighter::new(tokens);
                
                // ASTが得られた場合は、それを使って詳細な構文情報を生成
                let syntax_highlighter = if let Ok(ast) = ast_result {
                    syntax_highlighter.with_ast(ast)
                } else {
                    syntax_highlighter
                };
                
                // 構文範囲情報を生成
                let ranges = syntax_highlighter.generate_syntax_ranges();
                
                // ドキュメント構文情報を保存
                self.document_syntax.insert(uri.to_string(), DocumentSyntax {
                    text: content.to_string(),
                    ranges,
                    timestamp: std::time::SystemTime::now(),
                });
            },
            Err(err) => {
                // 字句解析でエラーが発生した場合は、エラーを診断エミッタに追加
                self.diagnostics.emit_error(
                    &format!("構文ハイライト更新中にエラーが発生しました: {}", err),
                    uri,
                );
            }
        }
    }
    
    /// LSPセマンティックトークンを生成
    pub fn generate_semantic_tokens(&self, uri: &str) -> Option<SemanticTokens> {
        if let Some(doc_syntax) = self.document_syntax.get(uri) {
            let mut tokens = Vec::new();
            let text = &doc_syntax.text;
            
            // 構文範囲情報からLSPセマンティックトークンを生成
            let mut current_line = 0;
            let mut current_char = 0;
            
            // 行と文字のオフセットを計算するためのヘルパー
            let calculate_position = |offset: usize| -> (u32, u32) {
                let prefix = &text[..offset];
                let lines: Vec<&str> = prefix.split('\n').collect();
                let line = (lines.len() - 1) as u32;
                let character = if lines.len() > 0 {
                    lines.last().unwrap().len() as u32
                } else {
                    0
                };
                (line, character)
            };
            
            // 範囲情報をソート（行、文字位置順）
            let mut sorted_ranges = doc_syntax.ranges.clone();
            sorted_ranges.sort_by(|a, b| {
                let (a_line, a_char) = calculate_position(a.start);
                let (b_line, b_char) = calculate_position(b.start);
                (a_line, a_char).cmp(&(b_line, b_char))
            });
            
            // LSPセマンティックトークンを生成
            for range in sorted_ranges {
                let (line, character) = calculate_position(range.start);
                let delta_line = line - current_line;
                let delta_start = if delta_line == 0 {
                    character - current_char
                } else {
                    character
                };
                
                // トークンタイプのマッピング
                let token_type = self.token_type_map.get(&range.kind).cloned().unwrap_or(0);
                
                // トークン修飾子（今回は0）
                let token_modifiers_bitset = 0;
                
                // トークンの長さ
                let length = (range.end - range.start) as u32;
                
                // トークンを追加
                tokens.push(SemanticToken {
                    delta_line,
                    delta_start,
                    length,
                    token_type,
                    token_modifiers_bitset,
                });
                
                // 現在位置を更新
                current_line = line;
                current_char = character;
            }
            
            // LSPセマンティックトークンを返す
            Some(SemanticTokens {
                result_id: Some(format!("{:?}", doc_syntax.timestamp.elapsed().unwrap_or_default())),
                data: tokens,
            })
        } else {
            None
        }
    }
}

/// エディタの構文ハイライト設定に関するヘルパー関数
pub fn get_editor_textmate_scopes() -> HashMap<String, String> {
    let mut scopes = HashMap::new();
    
    // SwiftLight言語のTextMateスコープマッピング
    scopes.insert("keyword".to_string(), "keyword.control.swiftlight".to_string());
    scopes.insert("function".to_string(), "entity.name.function.swiftlight".to_string());
    scopes.insert("method".to_string(), "entity.name.function.method.swiftlight".to_string());
    scopes.insert("variable".to_string(), "variable.other.swiftlight".to_string());
    scopes.insert("parameter".to_string(), "variable.parameter.swiftlight".to_string());
    scopes.insert("property".to_string(), "variable.other.property.swiftlight".to_string());
    scopes.insert("type".to_string(), "entity.name.type.swiftlight".to_string());
    scopes.insert("class".to_string(), "entity.name.type.class.swiftlight".to_string());
    scopes.insert("struct".to_string(), "entity.name.type.struct.swiftlight".to_string());
    scopes.insert("interface".to_string(), "entity.name.type.interface.swiftlight".to_string());
    scopes.insert("enum".to_string(), "entity.name.type.enum.swiftlight".to_string());
    scopes.insert("enumMember".to_string(), "variable.other.enummember.swiftlight".to_string());
    scopes.insert("string".to_string(), "string.quoted.swiftlight".to_string());
    scopes.insert("number".to_string(), "constant.numeric.swiftlight".to_string());
    scopes.insert("comment".to_string(), "comment.line.swiftlight".to_string());
    scopes.insert("operator".to_string(), "keyword.operator.swiftlight".to_string());
    scopes.insert("namespace".to_string(), "entity.name.namespace.swiftlight".to_string());
    scopes.insert("typeParameter".to_string(), "entity.name.type.parameter.swiftlight".to_string());
    scopes.insert("macro".to_string(), "entity.name.function.macro.swiftlight".to_string());
    
    scopes
} 
 
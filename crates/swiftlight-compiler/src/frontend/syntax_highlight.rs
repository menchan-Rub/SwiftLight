//! # 構文ハイライト用データ提供モジュール
//!
//! このモジュールでは、エディタやIDEが構文ハイライトを正確に行うためのデータを生成します。
//! レキサーとパーサーが生成した情報を使用し、効率的に構文情報をエクスポートします。

use std::collections::{HashMap, HashSet};
use crate::frontend::lexer::token::{Token, TokenKind};
use crate::frontend::parser::grammar::{HighlightKind, SyntaxHighlight, token_to_highlight_kind};
use crate::frontend::parser::ast;
use crate::frontend::error::{Result, CompilerError, ErrorKind};
use crate::frontend::source_map::Span;

/// コード範囲の構文情報
#[derive(Debug, Clone)]
pub struct SyntaxRangeInfo {
    /// 開始位置（バイトオフセット）
    pub start: usize,
    /// 終了位置（バイトオフセット）
    pub end: usize,
    /// ハイライトの種類
    pub kind: HighlightKind,
    /// セマンティックタグ（オプション）
    pub semantic_tag: Option<String>,
    /// 関連する識別子情報
    pub symbol_info: Option<SymbolInfo>,
}

/// シンボル情報
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    /// シンボル名
    pub name: String,
    /// シンボルの種類（関数、変数、型など）
    pub kind: String,
    /// シンボルの定義位置
    pub definition_location: Option<Span>,
    /// シンボルの型（可能であれば）
    pub type_name: Option<String>,
    /// シンボルのドキュメント（可能であれば）
    pub documentation: Option<String>,
}

/// 構文ハイライト生成器
#[derive(Debug)]
pub struct SyntaxHighlighter {
    /// トークン列
    tokens: Vec<Token>,
    /// AST
    ast: Option<ast::Program>,
    /// セマンティック情報
    semantic_info: HashMap<String, String>,
    /// シンボル情報
    symbol_table: HashMap<String, SymbolInfo>,
}

impl SyntaxHighlighter {
    /// 新しい構文ハイライト生成器を作成
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            ast: None,
            semantic_info: HashMap::new(),
            symbol_table: HashMap::new(),
        }
    }

    /// ASTを設定
    pub fn with_ast(mut self, ast: ast::Program) -> Self {
        self.ast = Some(ast);
        self
    }

    /// セマンティック情報を設定
    pub fn with_semantic_info(mut self, semantic_info: HashMap<String, String>) -> Self {
        self.semantic_info = semantic_info;
        self
    }

    /// シンボル情報を設定
    pub fn with_symbol_table(mut self, symbol_table: HashMap<String, SymbolInfo>) -> Self {
        self.symbol_table = symbol_table;
        self
    }

    /// 構文ハイライト情報を生成
    pub fn generate_highlights(&self) -> Vec<SyntaxHighlight> {
        let mut highlights = self.tokens.iter().map(|token| {
            SyntaxHighlight {
                token: token.clone(),
                highlight_kind: token_to_highlight_kind(token),
                semantic_info: None,
            }
        }).collect::<Vec<_>>();

        // セマンティック情報があれば追加
        if !self.semantic_info.is_empty() {
            self.update_highlights_with_semantics(&mut highlights);
        }

        highlights
    }

    /// セマンティクス情報に基づいてハイライト情報を更新
    fn update_highlights_with_semantics(&self, highlights: &mut [SyntaxHighlight]) {
        for highlight in highlights {
            if highlight.highlight_kind == HighlightKind::Identifier {
                if let TokenKind::Identifier(name) = &highlight.token.kind {
                    if let Some(kind) = self.semantic_info.get(name) {
                        // 識別子の種類に基づいてハイライト種類を更新
                        highlight.highlight_kind = match kind.as_str() {
                            "function" => HighlightKind::FunctionName,
                            "method" => HighlightKind::Method,
                            "variable" => HighlightKind::VariableName,
                            "constant" => HighlightKind::ConstantName,
                            "parameter" => HighlightKind::Parameter,
                            "type" => HighlightKind::TypeName,
                            "userType" => HighlightKind::UserType,
                            "libraryType" => HighlightKind::LibraryType,
                            "primitiveType" => HighlightKind::PrimitiveType,
                            "property" => HighlightKind::Property,
                            "module" => HighlightKind::Module,
                            _ => HighlightKind::Identifier,
                        };
                        highlight.semantic_info = Some(kind.clone());
                    }
                }
            }
        }
    }

    /// 構文範囲情報を生成
    pub fn generate_syntax_ranges(&self) -> Vec<SyntaxRangeInfo> {
        let mut ranges = Vec::new();

        // まずトークンベースの構文情報を追加
        for token in &self.tokens {
            let kind = token_to_highlight_kind(token);
            let mut semantic_tag = None;
            let mut symbol_info = None;

            // 識別子の場合はセマンティック情報を追加
            if let TokenKind::Identifier(name) = &token.kind {
                semantic_tag = self.semantic_info.get(name).cloned();
                symbol_info = self.symbol_table.get(name).cloned();
            }

            ranges.push(SyntaxRangeInfo {
                start: token.location.start,
                end: token.location.end,
                kind,
                semantic_tag,
                symbol_info,
            });
        }

        // ASTがあれば、さらに詳細な構文情報を追加
        if let Some(ref ast) = self.ast {
            self.enhance_ranges_with_ast(&mut ranges, ast);
        }

        ranges
    }

    /// ASTを使って構文範囲情報を強化する
    fn enhance_ranges_with_ast(&self, ranges: &mut Vec<SyntaxRangeInfo>, ast: &ast::Program) {
        // ASTノードをトラバースして構文情報を補完
        // 実際の実装ではASTの各ノードタイプに対応した処理を行います
        // ここでは簡単な概要のみ示します

        // 関数宣言のハイライト強化
        for function in &ast.functions {
            // 関数名ハイライト
            if let Some(name_start) = function.name_span.map(|span| span.start) {
                if let Some(name_end) = function.name_span.map(|span| span.end) {
                    ranges.push(SyntaxRangeInfo {
                        start: name_start,
                        end: name_end,
                        kind: HighlightKind::FunctionName,
                        semantic_tag: Some("function".to_string()),
                        symbol_info: function.name.as_ref().map(|name| SymbolInfo {
                            name: name.clone(),
                            kind: "function".to_string(),
                            definition_location: function.name_span,
                            type_name: None, // 関数の型情報があれば追加
                            documentation: None, // ドキュメントコメントがあれば追加
                        }),
                    });
                }
            }

            // パラメータのハイライト
            for param in &function.parameters {
                if let Some(param_span) = param.name_span {
                    ranges.push(SyntaxRangeInfo {
                        start: param_span.start,
                        end: param_span.end,
                        kind: HighlightKind::Parameter,
                        semantic_tag: Some("parameter".to_string()),
                        symbol_info: Some(SymbolInfo {
                            name: param.name.clone(),
                            kind: "parameter".to_string(),
                            definition_location: Some(param_span),
                            type_name: param.type_annotation.as_ref().map(|t| t.to_string()),
                            documentation: None,
                        }),
                    });
                }
            }

            // 関数本体内の変数やその他の要素も同様に処理
            // ...
        }

        // 型宣言のハイライト強化
        for type_decl in &ast.types {
            // 型名ハイライト
            if let Some(name_span) = type_decl.name_span {
                ranges.push(SyntaxRangeInfo {
                    start: name_span.start,
                    end: name_span.end,
                    kind: HighlightKind::TypeName,
                    semantic_tag: Some("type".to_string()),
                    symbol_info: Some(SymbolInfo {
                        name: type_decl.name.clone(),
                        kind: "type".to_string(),
                        definition_location: Some(name_span),
                        type_name: None,
                        documentation: None,
                    }),
                });
            }

            // 型のメンバー、ジェネリックパラメータなども同様に処理
            // ...
        }

        // 変数宣言のハイライト強化
        for var_decl in &ast.variables {
            if let Some(name_span) = var_decl.name_span {
                let kind = if var_decl.is_constant {
                    HighlightKind::ConstantName
                } else {
                    HighlightKind::VariableName
                };

                let symbol_kind = if var_decl.is_constant {
                    "constant"
                } else {
                    "variable"
                };

                ranges.push(SyntaxRangeInfo {
                    start: name_span.start,
                    end: name_span.end,
                    kind,
                    semantic_tag: Some(symbol_kind.to_string()),
                    symbol_info: Some(SymbolInfo {
                        name: var_decl.name.clone(),
                        kind: symbol_kind.to_string(),
                        definition_location: Some(name_span),
                        type_name: var_decl.type_annotation.as_ref().map(|t| t.to_string()),
                        documentation: None,
                    }),
                });
            }
        }

        // その他のASTノードも同様に処理
        // ...
    }

    /// 指定されたファイル位置でホバーすると表示する情報を取得
    pub fn get_hover_info(&self, position: usize) -> Option<String> {
        // 位置に一致するシンボルを探す
        for (name, info) in &self.symbol_table {
            if let Some(loc) = &info.definition_location {
                if position >= loc.start && position <= loc.end {
                    let mut result = format!("**{}**", name);
                    
                    if let Some(type_name) = &info.type_name {
                        result.push_str(&format!(": {}", type_name));
                    }
                    
                    result.push_str(&format!("\n\n_種類: {}_", info.kind));
                    
                    if let Some(doc) = &info.documentation {
                        result.push_str(&format!("\n\n{}", doc));
                    }
                    
                    return Some(result);
                }
            }
        }
        
        // トークンベースの情報でフォールバック
        for token in &self.tokens {
            if position >= token.location.start && position <= token.location.end {
                match &token.kind {
                    TokenKind::Identifier(name) => {
                        return Some(format!("識別子: {}", name));
                    }
                    TokenKind::IntLiteral(value) => {
                        return Some(format!("整数リテラル: {}", value));
                    }
                    TokenKind::FloatLiteral(value) => {
                        return Some(format!("浮動小数点リテラル: {}", value));
                    }
                    TokenKind::StringLiteral(value) => {
                        return Some(format!("文字列リテラル: {:?}", value));
                    }
                    _ => {
                        return Some(format!("トークン: {:?}", token.kind));
                    }
                }
            }
        }
        
        None
    }

    /// 指定されたファイル位置での定義位置を取得
    pub fn get_definition_location(&self, position: usize) -> Option<Span> {
        // 位置に一致するトークンを探す
        for token in &self.tokens {
            if position >= token.location.start && position <= token.location.end {
                // 識別子の場合
                if let TokenKind::Identifier(name) = &token.kind {
                    // シンボルテーブルに登録されていれば定義位置を返す
                    if let Some(info) = self.symbol_table.get(name) {
                        return info.definition_location;
                    }
                }
                break;
            }
        }
        
        None
    }
}

/// 構文ハイライト情報をJSON形式に変換
pub fn syntax_highlights_to_json(highlights: &[SyntaxHighlight]) -> String {
    let mut json = String::from("[\n");
    
    for (i, highlight) in highlights.iter().enumerate() {
        json.push_str(&format!(
            "  {{\"start\": {}, \"end\": {}, \"kind\": \"{:?}\"{}}}", 
            highlight.token.location.start,
            highlight.token.location.end,
            highlight.highlight_kind,
            if let Some(ref info) = highlight.semantic_info {
                format!(", \"semantic\": \"{}\"", info)
            } else {
                String::new()
            }
        ));
        
        if i < highlights.len() - 1 {
            json.push_str(",\n");
        } else {
            json.push_str("\n");
        }
    }
    
    json.push_str("]\n");
    json
}

/// 構文範囲情報をJSON形式に変換
pub fn syntax_ranges_to_json(ranges: &[SyntaxRangeInfo]) -> String {
    let mut json = String::from("[\n");
    
    for (i, range) in ranges.iter().enumerate() {
        let mut entry = format!(
            "  {{\"start\": {}, \"end\": {}, \"kind\": \"{:?}\"", 
            range.start,
            range.end,
            range.kind
        );
        
        if let Some(ref tag) = range.semantic_tag {
            entry.push_str(&format!(", \"semantic_tag\": \"{}\"", tag));
        }
        
        if let Some(ref symbol) = range.symbol_info {
            entry.push_str(&format!(", \"symbol\": {{\"name\": \"{}\", \"kind\": \"{}\"}}", 
                symbol.name, symbol.kind));
        }
        
        entry.push_str("}");
        
        if i < ranges.len() - 1 {
            entry.push_str(",\n");
        } else {
            entry.push_str("\n");
        }
        
        json.push_str(&entry);
    }
    
    json.push_str("]\n");
    json
} 
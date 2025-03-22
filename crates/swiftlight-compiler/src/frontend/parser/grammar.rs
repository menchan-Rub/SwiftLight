//! # SwiftLight言語の文法規則
//! 
//! このモジュールでは、SwiftLight言語の文法規則を定義します。
//! パーサーの実装で使用される構文規則とプロダクションを提供します。

use crate::frontend::lexer::token::TokenKind;
use std::collections::{HashMap, HashSet};
use crate::frontend::lexer::token::{Token, TokenKind};
use crate::frontend::error::{CompilerError, Result, SourceLocation};
use crate::frontend::parser::{ast, error::ParserError};

/// 優先順位レベル（低いほど優先度が低い）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    /// 最低優先度（割り当て、範囲など）
    Lowest = 0,
    /// 代入（=, +=, -=, *=, /=, %=, &=, |=, ^=, <<=, >>=）
    Assignment = 1,
    /// 範囲 (.., ..=)
    Range = 2,
    /// 条件式 (三項演算子)
    Conditional = 3,
    /// 論理OR (||)
    LogicalOr = 4,
    /// 論理AND (&&)
    LogicalAnd = 5,
    /// 等価比較 (==, !=)
    Equality = 6,
    /// 比較 (<, >, <=, >=)
    Comparison = 7,
    /// ビット演算OR, XOR (|, ^)
    BitwiseOr = 8,
    /// ビット演算AND (&)
    BitwiseAnd = 9,
    /// シフト (<<, >>)
    Shift = 10,
    /// 加算・減算 (+, -)
    Term = 11,
    /// 乗算・除算・剰余 (*, /, %)
    Factor = 12,
    /// 単項演算子 (!, -, ~, &, *, ++, --)
    Unary = 13,
    /// メソッド呼び出し、添え字アクセス、メンバーアクセス (obj.method(), array[index], obj.field)
    Call = 14,
    /// 基本式（リテラル、識別子、括弧で囲まれた式など）
    Primary = 15,
}

impl Precedence {
    /// トークンから優先順位を取得
    pub fn from_token(token: &TokenKind) -> Self {
        match token {
            // 代入演算子
            TokenKind::Equal | 
            TokenKind::PlusEqual | TokenKind::MinusEqual | 
            TokenKind::StarEqual | TokenKind::SlashEqual | 
            TokenKind::PercentEqual | 
            TokenKind::AmpersandEqual | TokenKind::PipeEqual | TokenKind::CaretEqual |
            TokenKind::LeftShiftEqual | TokenKind::RightShiftEqual => Precedence::Assignment,
            
            // 範囲演算子
            TokenKind::Range | TokenKind::RangeInclusive => Precedence::Range,
            
            // 論理演算子
            TokenKind::PipePipe => Precedence::LogicalOr,
            TokenKind::AmpersandAmpersand => Precedence::LogicalAnd,
            
            // 等価演算子
            TokenKind::EqualEqual | TokenKind::BangEqual => Precedence::Equality,
            
            // 比較演算子
            TokenKind::Less | TokenKind::Greater | 
            TokenKind::LessEqual | TokenKind::GreaterEqual => Precedence::Comparison,
            
            // ビット演算子
            TokenKind::Pipe | TokenKind::Caret => Precedence::BitwiseOr,
            TokenKind::Ampersand => Precedence::BitwiseAnd,
            TokenKind::LeftShift | TokenKind::RightShift => Precedence::Shift,
            
            // 算術演算子
            TokenKind::Plus | TokenKind::Minus => Precedence::Term,
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Precedence::Factor,
            
            // 関数呼び出し、配列アクセス、メンバーアクセス
            TokenKind::LeftParen | TokenKind::LeftBracket | TokenKind::Dot => Precedence::Call,
            
            // その他
            _ => Precedence::Lowest,
        }
    }
}

/// 二項演算子のトークンかどうかを判定
pub fn is_binary_operator(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::Plus | TokenKind::Minus | 
        TokenKind::Star | TokenKind::Slash | TokenKind::Percent |
        TokenKind::EqualEqual | TokenKind::BangEqual |
        TokenKind::Less | TokenKind::Greater |
        TokenKind::LessEqual | TokenKind::GreaterEqual |
        TokenKind::AmpersandAmpersand | TokenKind::PipePipe |
        TokenKind::Ampersand | TokenKind::Pipe | TokenKind::Caret |
        TokenKind::LeftShift | TokenKind::RightShift |
        TokenKind::Range | TokenKind::RangeInclusive |
        TokenKind::Equal | TokenKind::PlusEqual | TokenKind::MinusEqual |
        TokenKind::StarEqual | TokenKind::SlashEqual | TokenKind::PercentEqual |
        TokenKind::AmpersandEqual | TokenKind::PipeEqual | TokenKind::CaretEqual |
        TokenKind::LeftShiftEqual | TokenKind::RightShiftEqual
    )
}

/// 単項演算子のトークンかどうかを判定
pub fn is_unary_operator(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::Plus | TokenKind::Minus | 
        TokenKind::Bang | TokenKind::Tilde |
        TokenKind::Ampersand | TokenKind::Star |
        TokenKind::PlusPlus | TokenKind::MinusMinus
    )
}

/// 単項前置演算子のトークンかどうかを判定
pub fn is_prefix_operator(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::Plus | TokenKind::Minus | 
        TokenKind::Bang | TokenKind::Tilde |
        TokenKind::Ampersand | TokenKind::Star |
        TokenKind::PlusPlus | TokenKind::MinusMinus
    )
}

/// 単項後置演算子のトークンかどうかを判定
pub fn is_postfix_operator(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::PlusPlus | TokenKind::MinusMinus
    )
}

/// 文を開始するトークンかどうかを判定
pub fn is_statement_start(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::KeywordLet | TokenKind::KeywordVar | TokenKind::KeywordConst |
        TokenKind::KeywordIf | TokenKind::KeywordWhile | TokenKind::KeywordFor |
        TokenKind::KeywordReturn | TokenKind::KeywordBreak | TokenKind::KeywordContinue |
        TokenKind::LeftBrace | TokenKind::Semicolon |
        TokenKind::KeywordTry | TokenKind::KeywordThrow | TokenKind::KeywordAsync |
        TokenKind::KeywordMatch
    )
}

/// 式を開始するトークンかどうかを判定
pub fn is_expression_start(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::Identifier | 
        TokenKind::IntLiteral | TokenKind::FloatLiteral | 
        TokenKind::StringLiteral | TokenKind::CharLiteral |
        TokenKind::KeywordTrue | TokenKind::KeywordFalse | TokenKind::KeywordNil |
        TokenKind::LeftParen | TokenKind::LeftBracket | TokenKind::LeftBrace |
        TokenKind::KeywordSelf | TokenKind::KeywordSuper |
        TokenKind::Plus | TokenKind::Minus | TokenKind::Bang | TokenKind::Tilde |
        TokenKind::PlusPlus | TokenKind::MinusMinus |
        TokenKind::Ampersand | TokenKind::Star |
        TokenKind::KeywordIf | TokenKind::KeywordMatch | 
        TokenKind::KeywordTry | TokenKind::KeywordThrow | 
        TokenKind::KeywordAsync | TokenKind::KeywordAwait
    )
}

/// 宣言を開始するトークンかどうかを判定
pub fn is_declaration_start(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::KeywordFn | TokenKind::KeywordLet | TokenKind::KeywordVar |
        TokenKind::KeywordConst | TokenKind::KeywordStruct | TokenKind::KeywordEnum |
        TokenKind::KeywordTrait | TokenKind::KeywordImpl | TokenKind::KeywordType |
        TokenKind::KeywordImport | TokenKind::KeywordModule |
        TokenKind::KeywordPub | TokenKind::KeywordAsync | TokenKind::KeywordUnsafe
    )
}

/// 型を開始するトークンかどうかを判定
pub fn is_type_start(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::Identifier | 
        TokenKind::KeywordSelf |
        TokenKind::LeftParen | TokenKind::LeftBracket |
        TokenKind::Ampersand | TokenKind::Star
    )
}

/// 文の終わりを示すトークンかどうかを判定
pub fn is_statement_end(token: &TokenKind) -> bool {
    token == &TokenKind::Semicolon || token == &TokenKind::RightBrace
}

/// ブロックの終わりを示すトークンかどうかを判定
pub fn is_block_end(token: &TokenKind) -> bool {
    token == &TokenKind::RightBrace
}

/// 式の終わりを示すトークンかどうかを判定
pub fn is_expression_end(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::Semicolon | TokenKind::Comma | 
        TokenKind::RightParen | TokenKind::RightBracket | TokenKind::RightBrace |
        TokenKind::Colon | TokenKind::Arrow | TokenKind::FatArrow
    )
}

/// 型の終わりを示すトークンかどうかを判定
pub fn is_type_end(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::Semicolon | TokenKind::Comma | 
        TokenKind::RightParen | TokenKind::RightBracket | TokenKind::RightBrace |
        TokenKind::Equal | TokenKind::LeftBrace |
        TokenKind::Colon | TokenKind::Arrow
    )
}

/// リテラルを表すトークンかどうかを判定
pub fn is_literal(token: &TokenKind) -> bool {
    matches!(token,
        TokenKind::IntLiteral | TokenKind::FloatLiteral | 
        TokenKind::StringLiteral | TokenKind::CharLiteral |
        TokenKind::KeywordTrue | TokenKind::KeywordFalse | TokenKind::KeywordNil
    )
}

/// 文法規則の種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GrammarRuleKind {
    // 構造体宣言
    StructDeclaration,
    // 列挙型宣言
    EnumDeclaration,
    // 関数宣言
    FunctionDeclaration,
    // クラス宣言
    ClassDeclaration,
    // トレイト宣言
    TraitDeclaration,
    // インターフェース宣言
    InterfaceDeclaration,
    // 型宣言
    TypeDeclaration,
    // 変数宣言
    VariableDeclaration,
    // 定数宣言
    ConstantDeclaration,
    // if文
    IfStatement,
    // for文
    ForStatement,
    // while文
    WhileStatement,
    // match文
    MatchStatement,
    // try文
    TryStatement,
    // return文
    ReturnStatement,
    // 式
    Expression,
    // ブロック
    Block,
    // モジュール宣言
    ModuleDeclaration,
    // インポート宣言
    ImportDeclaration,
    // その他
    Other,
}

/// 自動補完コンテキスト
#[derive(Debug, Clone)]
pub struct CompletionContext {
    /// 補完位置
    pub position: usize,
    /// 現在の文法規則
    pub rule: GrammarRuleKind,
    /// 期待されるトークン種類
    pub expected_tokens: HashSet<TokenKind>,
    /// 補完前の部分識別子（あれば）
    pub partial_identifier: Option<String>,
    /// 現在スコープで使用可能なシンボル
    pub available_symbols: HashMap<String, String>,
    /// 親コンテキスト
    pub parent_context: Option<Box<CompletionContext>>,
}

impl CompletionContext {
    /// 新しい補完コンテキストを作成
    pub fn new(position: usize, rule: GrammarRuleKind) -> Self {
        Self {
            position,
            rule,
            expected_tokens: HashSet::new(),
            partial_identifier: None,
            available_symbols: HashMap::new(),
            parent_context: None,
        }
    }

    /// 期待トークンを追加
    pub fn expect_token(&mut self, kind: TokenKind) {
        self.expected_tokens.insert(kind);
    }

    /// 期待トークン群を追加
    pub fn expect_tokens(&mut self, kinds: &[TokenKind]) {
        for kind in kinds {
            self.expected_tokens.insert(kind.clone());
        }
    }

    /// 部分識別子を設定
    pub fn set_partial_identifier(&mut self, identifier: &str) {
        self.partial_identifier = Some(identifier.to_string());
    }

    /// 使用可能なシンボルを追加
    pub fn add_symbol(&mut self, name: &str, kind: &str) {
        self.available_symbols.insert(name.to_string(), kind.to_string());
    }

    /// 使用可能なシンボルを追加（一括）
    pub fn add_symbols(&mut self, symbols: HashMap<String, String>) {
        self.available_symbols.extend(symbols);
    }

    /// 親コンテキストを設定
    pub fn set_parent(&mut self, parent: CompletionContext) {
        self.parent_context = Some(Box::new(parent));
    }

    /// 全ての期待トークンを取得（親コンテキストも含む）
    pub fn all_expected_tokens(&self) -> HashSet<TokenKind> {
        let mut result = self.expected_tokens.clone();
        if let Some(parent) = &self.parent_context {
            result.extend(parent.all_expected_tokens());
        }
        result
    }

    /// 全ての使用可能なシンボルを取得（親コンテキストも含む）
    pub fn all_available_symbols(&self) -> HashMap<String, String> {
        let mut result = self.available_symbols.clone();
        if let Some(parent) = &self.parent_context {
            // 親スコープのシンボル（同名の場合は現在のスコープが優先）
            for (name, kind) in parent.all_available_symbols() {
                if !result.contains_key(&name) {
                    result.insert(name, kind);
                }
            }
        }
        result
    }
}

/// 構文ハイライト情報
#[derive(Debug, Clone)]
pub struct SyntaxHighlight {
    /// トークン
    pub token: Token,
    /// ハイライトの種類
    pub highlight_kind: HighlightKind,
    /// セマンティック情報（オプション）
    pub semantic_info: Option<String>,
}

/// ハイライトの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HighlightKind {
    /// キーワード
    Keyword,
    /// 識別子
    Identifier,
    /// 型名
    TypeName,
    /// 関数名
    FunctionName,
    /// 変数名
    VariableName,
    /// 定数名
    ConstantName,
    /// 数値リテラル
    NumericLiteral,
    /// 文字列リテラル
    StringLiteral,
    /// 文字リテラル
    CharLiteral,
    /// コメント
    Comment,
    /// ドキュメントコメント
    DocComment,
    /// 演算子
    Operator,
    /// 区切り記号
    Delimiter,
    /// マクロ
    Macro,
    /// 注釈/デコレータ
    Annotation,
    /// エラー
    Error,
    /// ユーザー定義型
    UserType,
    /// ライブラリ型
    LibraryType,
    /// プリミティブ型
    PrimitiveType,
    /// ラベル
    Label,
    /// プロパティ
    Property,
    /// メソッド
    Method,
    /// パラメータ
    Parameter,
    /// モジュール
    Module,
    /// その他
    Other,
}

/// トークンからハイライト種類を決定
pub fn token_to_highlight_kind(token: &Token) -> HighlightKind {
    match &token.kind {
        // キーワード
        TokenKind::Let | TokenKind::Var | TokenKind::Const | TokenKind::Func |
        TokenKind::Class | TokenKind::Struct | TokenKind::Enum | TokenKind::Interface |
        TokenKind::Trait | TokenKind::Impl | TokenKind::Type | TokenKind::If |
        TokenKind::Else | TokenKind::For | TokenKind::While | TokenKind::Do |
        TokenKind::Break | TokenKind::Continue | TokenKind::Return | TokenKind::Yield |
        TokenKind::Match | TokenKind::Case | TokenKind::Default | TokenKind::Switch |
        TokenKind::Try | TokenKind::Catch | TokenKind::Throw | TokenKind::Pub |
        TokenKind::Private | TokenKind::Protected | TokenKind::Internal | TokenKind::Static |
        TokenKind::Async | TokenKind::Await | TokenKind::Mut | TokenKind::Ref |
        TokenKind::Unsafe | TokenKind::Module | TokenKind::Import | TokenKind::Export |
        TokenKind::As | TokenKind::From | TokenKind::Where | TokenKind::Inline |
        TokenKind::Extern | TokenKind::Sizeof | TokenKind::Typeof | TokenKind::In |
        TokenKind::Is | TokenKind::SelfLower | TokenKind::SelfUpper | TokenKind::Super |
        TokenKind::Pure | TokenKind::Dependent | TokenKind::Forall | TokenKind::Exists |
        TokenKind::Operator | TokenKind::Precedence | TokenKind::Associativity |
        TokenKind::Protocol | TokenKind::Extension | TokenKind::Typealias |
        TokenKind::Meta | TokenKind::Guard | TokenKind::Defer => HighlightKind::Keyword,

        // リテラル
        TokenKind::IntLiteral(_) | TokenKind::FloatLiteral(_) => HighlightKind::NumericLiteral,
        TokenKind::StringLiteral(_) => HighlightKind::StringLiteral,
        TokenKind::CharLiteral(_) => HighlightKind::CharLiteral,
        TokenKind::TrueLiteral | TokenKind::FalseLiteral | TokenKind::NilLiteral => HighlightKind::Keyword,

        // コメント
        TokenKind::Comment(_) => HighlightKind::Comment,
        TokenKind::DocComment(_) => HighlightKind::DocComment,

        // 識別子（デフォルトで識別子として扱い、セマンティクス分析で修正）
        TokenKind::Identifier(_) => HighlightKind::Identifier,

        // 演算子
        TokenKind::Plus | TokenKind::Minus | TokenKind::Star | TokenKind::Slash |
        TokenKind::Percent | TokenKind::Caret | TokenKind::Ampersand | TokenKind::Pipe |
        TokenKind::Tilde | TokenKind::Bang | TokenKind::Equal | TokenKind::Less |
        TokenKind::Greater | TokenKind::PlusEqual | TokenKind::MinusEqual |
        TokenKind::StarEqual | TokenKind::SlashEqual | TokenKind::PercentEqual |
        TokenKind::CaretEqual | TokenKind::AmpersandEqual | TokenKind::PipeEqual |
        TokenKind::TildeEqual | TokenKind::BangEqual | TokenKind::EqualEqual |
        TokenKind::LessEqual | TokenKind::GreaterEqual | TokenKind::AmpersandAmpersand |
        TokenKind::PipePipe | TokenKind::PlusPlus | TokenKind::MinusMinus |
        TokenKind::LeftShift | TokenKind::RightShift | TokenKind::Question |
        TokenKind::QuestionDot | TokenKind::QuestionQuestion => HighlightKind::Operator,

        // 区切り記号
        TokenKind::LeftParen | TokenKind::RightParen | TokenKind::LeftBrace |
        TokenKind::RightBrace | TokenKind::LeftBracket | TokenKind::RightBracket |
        TokenKind::Comma | TokenKind::Dot | TokenKind::Semicolon | TokenKind::Colon |
        TokenKind::DoubleColon | TokenKind::Arrow | TokenKind::FatArrow |
        TokenKind::DotDot | TokenKind::DotDotDot => HighlightKind::Delimiter,

        // 特殊なもの
        TokenKind::At => HighlightKind::Annotation,
        TokenKind::Hash => HighlightKind::Macro,
        
        // その他
        _ => HighlightKind::Other,
    }
}

/// トークン列から構文ハイライト情報を生成
pub fn generate_highlights(tokens: &[Token]) -> Vec<SyntaxHighlight> {
    tokens.iter().map(|token| {
        SyntaxHighlight {
            token: token.clone(),
            highlight_kind: token_to_highlight_kind(token),
            semantic_info: None,
        }
    }).collect()
}

/// セマンティクス情報に基づいてハイライト情報を更新
pub fn update_highlights_with_semantics(
    highlights: &mut [SyntaxHighlight],
    identifiers: &HashMap<String, String>,
) {
    for highlight in highlights {
        if highlight.highlight_kind == HighlightKind::Identifier {
            if let TokenKind::Identifier(name) = &highlight.token.kind {
                if let Some(kind) = identifiers.get(name) {
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

/// 特定の位置での自動補完コンテキストを生成
pub fn generate_completion_context(
    tokens: &[Token],
    position: usize,
    ast: &ast::Program,
) -> CompletionContext {
    let mut context = CompletionContext::new(position, GrammarRuleKind::Other);

    // 1. 位置に最も近いトークンを見つける
    let nearest_token_index = find_nearest_token(tokens, position);

    // 2. 文法コンテキストを判定
    if let Some(idx) = nearest_token_index {
        let rule = determine_grammar_rule(tokens, idx);
        context.rule = rule;
        
        // 3. トークンシーケンスに基づいて期待されるトークンを設定
        set_expected_tokens(&mut context, tokens, idx);
        
        // 4. 部分識別子があれば設定
        if position > 0 {
            extract_partial_identifier(tokens, idx, position, &mut context);
        }
    }

    // 5. AST解析から使用可能なシンボルを収集
    collect_available_symbols(&mut context, ast, position);

    context
}

/// 位置に最も近いトークンのインデックスを見つける
fn find_nearest_token(tokens: &[Token], position: usize) -> Option<usize> {
    // 位置を含むトークンを検索
    for (i, token) in tokens.iter().enumerate() {
        if position >= token.location.start && position <= token.location.end {
            return Some(i);
        }
    }
    
    // 位置の直前のトークンを検索
    if !tokens.is_empty() {
        let mut best_idx = 0;
        let mut best_distance = usize::MAX;
        
        for (i, token) in tokens.iter().enumerate() {
            if token.location.end <= position {
                let distance = position - token.location.end;
                if distance < best_distance {
                    best_distance = distance;
                    best_idx = i;
                }
            }
        }
        
        if best_distance != usize::MAX {
            return Some(best_idx);
        }
    }
    
    None
}

/// 文法規則を判定
fn determine_grammar_rule(tokens: &[Token], idx: usize) -> GrammarRuleKind {
    // トークンシーケンスに基づいて文法規則を判定
    // ここでは簡単な実装のみを示す。実際にはより複雑な解析が必要
    
    // 現在のトークンから遡って文脈を判定
    for i in (0..=idx.min(tokens.len() - 1)).rev() {
        match tokens[i].kind {
            TokenKind::Struct => return GrammarRuleKind::StructDeclaration,
            TokenKind::Enum => return GrammarRuleKind::EnumDeclaration,
            TokenKind::Func => return GrammarRuleKind::FunctionDeclaration,
            TokenKind::Class => return GrammarRuleKind::ClassDeclaration,
            TokenKind::Interface => return GrammarRuleKind::InterfaceDeclaration,
            TokenKind::Trait => return GrammarRuleKind::TraitDeclaration,
            TokenKind::Type => return GrammarRuleKind::TypeDeclaration,
            TokenKind::Let | TokenKind::Var | TokenKind::Const => return GrammarRuleKind::VariableDeclaration,
            TokenKind::If => return GrammarRuleKind::IfStatement,
            TokenKind::For => return GrammarRuleKind::ForStatement,
            TokenKind::While => return GrammarRuleKind::WhileStatement,
            TokenKind::Match => return GrammarRuleKind::MatchStatement,
            TokenKind::Try => return GrammarRuleKind::TryStatement,
            TokenKind::Return => return GrammarRuleKind::ReturnStatement,
            TokenKind::Module => return GrammarRuleKind::ModuleDeclaration,
            TokenKind::Import => return GrammarRuleKind::ImportDeclaration,
            TokenKind::LeftBrace => {
                // ブロックの前のトークンをチェック
                if i > 0 {
                    match tokens[i-1].kind {
                        TokenKind::RightParen => {
                            // 括弧の前のトークンをチェック
                            if i > 1 {
                                match tokens[i-2].kind {
                                    TokenKind::If => return GrammarRuleKind::IfStatement,
                                    TokenKind::For => return GrammarRuleKind::ForStatement,
                                    TokenKind::While => return GrammarRuleKind::WhileStatement,
                                    _ => {}
                                }
                            }
                            return GrammarRuleKind::Block;
                        }
                        _ => {}
                    }
                }
                return GrammarRuleKind::Block;
            }
            _ => {}
        }
    }
    
    // デフォルトでは式を返す
    GrammarRuleKind::Expression
}

/// 期待されるトークンを設定
fn set_expected_tokens(context: &mut CompletionContext, tokens: &[Token], idx: usize) {
    match context.rule {
        GrammarRuleKind::StructDeclaration => {
            context.expect_tokens(&[
                TokenKind::Identifier(String::new()),
                TokenKind::LeftBrace,
                TokenKind::RightBrace,
                TokenKind::Where,
                TokenKind::Colon,
            ]);
        },
        GrammarRuleKind::FunctionDeclaration => {
            context.expect_tokens(&[
                TokenKind::Identifier(String::new()),
                TokenKind::LeftParen,
                TokenKind::RightParen,
                TokenKind::Arrow,
                TokenKind::LeftBrace,
                TokenKind::Colon,
                TokenKind::Where,
            ]);
        },
        GrammarRuleKind::VariableDeclaration => {
            context.expect_tokens(&[
                TokenKind::Identifier(String::new()),
                TokenKind::Colon,
                TokenKind::Equal,
                TokenKind::Semicolon,
            ]);
        },
        GrammarRuleKind::Block => {
            context.expect_tokens(&[
                TokenKind::Let,
                TokenKind::Var,
                TokenKind::Const,
                TokenKind::If,
                TokenKind::For,
                TokenKind::While,
                TokenKind::Match,
                TokenKind::Return,
                TokenKind::Break,
                TokenKind::Continue,
                TokenKind::Identifier(String::new()),
                TokenKind::RightBrace,
            ]);
        },
        GrammarRuleKind::IfStatement => {
            // if文の現在位置に応じて異なるトークンが期待される
            let mut found_if = false;
            let mut found_paren = false;
            let mut found_block = false;
            
            for i in (0..=idx).rev() {
                match tokens[i].kind {
                    TokenKind::If => {
                        found_if = true;
                        break;
                    },
                    TokenKind::LeftParen => found_paren = true,
                    TokenKind::RightParen => found_paren = true,
                    TokenKind::LeftBrace => found_block = true,
                    TokenKind::RightBrace => found_block = true,
                    _ => {}
                }
            }
            
            if found_if && !found_paren {
                context.expect_token(TokenKind::LeftParen);
            } else if found_paren && !found_block {
                context.expect_token(TokenKind::LeftBrace);
            } else if found_block {
                context.expect_token(TokenKind::Else);
            }
        },
        // 他の文法規則も同様に実装
        _ => {
            // デフォルトでは基本的な式トークンを期待
            context.expect_tokens(&[
                TokenKind::Identifier(String::new()),
                TokenKind::IntLiteral(0),
                TokenKind::FloatLiteral(0.0),
                TokenKind::StringLiteral(String::new()),
                TokenKind::TrueLiteral,
                TokenKind::FalseLiteral,
                TokenKind::NilLiteral,
                TokenKind::LeftParen,
                TokenKind::LeftBracket,
            ]);
        }
    }
}

/// 部分識別子を抽出
fn extract_partial_identifier(
    tokens: &[Token],
    idx: usize,
    position: usize,
    context: &mut CompletionContext
) {
    if idx < tokens.len() {
        let token = &tokens[idx];
        
        // トークンが識別子で、ポジションがそのトークン内にある場合
        if let TokenKind::Identifier(name) = &token.kind {
            if position >= token.location.start && position <= token.location.end {
                let prefix_len = position - token.location.start;
                if prefix_len < name.len() {
                    context.set_partial_identifier(&name[0..prefix_len]);
                } else {
                    context.set_partial_identifier(name);
                }
            }
        }
        // トークンの直後にある場合は空の部分識別子
        else if position == token.location.end + 1 {
            context.set_partial_identifier("");
        }
    }
}

/// 使用可能なシンボルを収集
fn collect_available_symbols(
    context: &mut CompletionContext,
    ast: &ast::Program,
    position: usize
) {
    // この実装は簡略化されています。
    // 実際の実装では、ASTを深く解析して現在のスコープ内で利用可能な
    // すべてのシンボルを収集する必要があります。
    
    // 基本的なシンボルを追加（サンプル）
    context.add_symbol("println", "function");
    context.add_symbol("String", "libraryType");
    context.add_symbol("Int", "primitiveType");
    context.add_symbol("Float", "primitiveType");
    context.add_symbol("Bool", "primitiveType");
    context.add_symbol("Array", "libraryType");
    context.add_symbol("HashMap", "libraryType");
    context.add_symbol("Option", "libraryType");
    context.add_symbol("Result", "libraryType");
    
    // プログラム内の宣言をスキャン
    for decl in &ast.declarations {
        match decl {
            ast::Declaration::Function(func) => {
                context.add_symbol(&func.name, "function");
            },
            ast::Declaration::Struct(struct_decl) => {
                context.add_symbol(&struct_decl.name, "userType");
            },
            ast::Declaration::Enum(enum_decl) => {
                context.add_symbol(&enum_decl.name, "userType");
            },
            ast::Declaration::Variable(var_decl) => {
                if var_decl.location.start < position {
                    let symbol_kind = if var_decl.is_constant {
                        "constant"
                    } else {
                        "variable"
                    };
                    context.add_symbol(&var_decl.name, symbol_kind);
                }
            },
            ast::Declaration::Class(class_decl) => {
                context.add_symbol(&class_decl.name, "userType");
            },
            ast::Declaration::Interface(interface_decl) => {
                context.add_symbol(&interface_decl.name, "userType");
            },
            ast::Declaration::TypeAlias(type_alias) => {
                context.add_symbol(&type_alias.name, "userType");
            },
            ast::Declaration::Module(module_decl) => {
                context.add_symbol(&module_decl.name, "module");
            },
            // その他の宣言タイプも追加
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_precedence_ordering() {
        assert!(Precedence::Primary > Precedence::Call);
        assert!(Precedence::Call > Precedence::Unary);
        assert!(Precedence::Unary > Precedence::Factor);
        assert!(Precedence::Factor > Precedence::Term);
        assert!(Precedence::Term > Precedence::Shift);
        assert!(Precedence::Shift > Precedence::BitwiseAnd);
        assert!(Precedence::BitwiseAnd > Precedence::BitwiseOr);
        assert!(Precedence::BitwiseOr > Precedence::Comparison);
        assert!(Precedence::Comparison > Precedence::Equality);
        assert!(Precedence::Equality > Precedence::LogicalAnd);
        assert!(Precedence::LogicalAnd > Precedence::LogicalOr);
        assert!(Precedence::LogicalOr > Precedence::Conditional);
        assert!(Precedence::Conditional > Precedence::Range);
        assert!(Precedence::Range > Precedence::Assignment);
        assert!(Precedence::Assignment > Precedence::Lowest);
    }
    
    #[test]
    fn test_precedence_from_token() {
        assert_eq!(Precedence::from_token(&TokenKind::Plus), Precedence::Term);
        assert_eq!(Precedence::from_token(&TokenKind::Minus), Precedence::Term);
        assert_eq!(Precedence::from_token(&TokenKind::Star), Precedence::Factor);
        assert_eq!(Precedence::from_token(&TokenKind::Slash), Precedence::Factor);
        assert_eq!(Precedence::from_token(&TokenKind::Percent), Precedence::Factor);
        assert_eq!(Precedence::from_token(&TokenKind::EqualEqual), Precedence::Equality);
        assert_eq!(Precedence::from_token(&TokenKind::BangEqual), Precedence::Equality);
        assert_eq!(Precedence::from_token(&TokenKind::Less), Precedence::Comparison);
        assert_eq!(Precedence::from_token(&TokenKind::Greater), Precedence::Comparison);
        assert_eq!(Precedence::from_token(&TokenKind::LessEqual), Precedence::Comparison);
        assert_eq!(Precedence::from_token(&TokenKind::GreaterEqual), Precedence::Comparison);
        assert_eq!(Precedence::from_token(&TokenKind::AmpersandAmpersand), Precedence::LogicalAnd);
        assert_eq!(Precedence::from_token(&TokenKind::PipePipe), Precedence::LogicalOr);
        assert_eq!(Precedence::from_token(&TokenKind::Equal), Precedence::Assignment);
        assert_eq!(Precedence::from_token(&TokenKind::PlusEqual), Precedence::Assignment);
        assert_eq!(Precedence::from_token(&TokenKind::LeftParen), Precedence::Call);
        assert_eq!(Precedence::from_token(&TokenKind::LeftBracket), Precedence::Call);
        assert_eq!(Precedence::from_token(&TokenKind::Dot), Precedence::Call);
    }
    
    #[test]
    fn test_is_binary_operator() {
        assert!(is_binary_operator(&TokenKind::Plus));
        assert!(is_binary_operator(&TokenKind::Minus));
        assert!(is_binary_operator(&TokenKind::Star));
        assert!(is_binary_operator(&TokenKind::Slash));
        assert!(is_binary_operator(&TokenKind::Percent));
        assert!(is_binary_operator(&TokenKind::EqualEqual));
        assert!(is_binary_operator(&TokenKind::BangEqual));
        assert!(is_binary_operator(&TokenKind::Less));
        assert!(is_binary_operator(&TokenKind::Greater));
        assert!(is_binary_operator(&TokenKind::LessEqual));
        assert!(is_binary_operator(&TokenKind::GreaterEqual));
        assert!(is_binary_operator(&TokenKind::AmpersandAmpersand));
        assert!(is_binary_operator(&TokenKind::PipePipe));
        assert!(is_binary_operator(&TokenKind::Equal));
        
        assert!(!is_binary_operator(&TokenKind::LeftParen));
        assert!(!is_binary_operator(&TokenKind::RightParen));
        assert!(!is_binary_operator(&TokenKind::Semicolon));
        assert!(!is_binary_operator(&TokenKind::Identifier));
    }
    
    #[test]
    fn test_is_unary_operator() {
        assert!(is_unary_operator(&TokenKind::Plus));
        assert!(is_unary_operator(&TokenKind::Minus));
        assert!(is_unary_operator(&TokenKind::Bang));
        assert!(is_unary_operator(&TokenKind::Tilde));
        assert!(is_unary_operator(&TokenKind::PlusPlus));
        assert!(is_unary_operator(&TokenKind::MinusMinus));
        
        assert!(!is_unary_operator(&TokenKind::Star)); // これは単項でも二項でもある
        assert!(!is_unary_operator(&TokenKind::EqualEqual));
        assert!(!is_unary_operator(&TokenKind::LeftParen));
    }
    
    #[test]
    fn test_statement_start() {
        assert!(is_statement_start(&TokenKind::KeywordLet));
        assert!(is_statement_start(&TokenKind::KeywordVar));
        assert!(is_statement_start(&TokenKind::KeywordIf));
        assert!(is_statement_start(&TokenKind::KeywordWhile));
        assert!(is_statement_start(&TokenKind::KeywordFor));
        assert!(is_statement_start(&TokenKind::KeywordReturn));
        assert!(is_statement_start(&TokenKind::LeftBrace));
        
        assert!(!is_statement_start(&TokenKind::RightBrace));
        assert!(!is_statement_start(&TokenKind::Identifier));
        assert!(!is_statement_start(&TokenKind::IntLiteral));
    }
}

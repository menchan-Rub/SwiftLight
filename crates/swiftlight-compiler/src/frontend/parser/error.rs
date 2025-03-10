//! # 構文解析エラー処理
//! 
//! 構文解析時に発生するエラーを処理するためのユーティリティを提供します。
//! このモジュールは、構文解析中に発生する様々な種類のエラーを生成し、
//! 適切な診断情報を付加するための関数群を提供します。

use crate::frontend::error::{CompilerError, ErrorKind, SourceLocation};
use crate::frontend::lexer::{Token, TokenKind};
use crate::frontend::diagnostic::{Diagnostic, DiagnosticLevel};

/// 期待されるトークンの説明を生成
///
/// # 引数
/// * `expected` - 期待されるトークンの説明
/// * `found` - 実際に見つかったトークン
///
/// # 戻り値
/// フォーマットされたエラーメッセージ
pub fn expected_token_message(expected: &str, found: &Token) -> String {
    format!("'{}'が期待されましたが、'{}'が見つかりました", expected, token_to_string(found))
}

/// 期待される複数のトークンのいずれかの説明を生成
///
/// # 引数
/// * `expected` - 期待されるトークンの説明の配列
/// * `found` - 実際に見つかったトークン
///
/// # 戻り値
/// フォーマットされたエラーメッセージ
pub fn expected_one_of_message(expected: &[&str], found: &Token) -> String {
    let expected_str = expected.join("', '");
    format!("'{}'のいずれかが期待されましたが、'{}'が見つかりました", expected_str, token_to_string(found))
}

/// 構文エラーを作成
///
/// # 引数
/// * `message` - エラーメッセージ
/// * `token` - エラーが発生したトークン
///
/// # 戻り値
/// 構文エラーを表すCompilerError
pub fn syntax_error(message: impl Into<String>, token: &Token) -> CompilerError {
    let message = message.into();
    let error = CompilerError::syntax_error(
        message.clone(),
        Some(token.location.clone()),
    );
    
    // 追加の診断情報を提供
    let diagnostic = Diagnostic::error(
        format!("構文エラー: {}", message),
        Some(token.location.clone()),
    ).with_suggestion(format!("正しい構文を確認してください"));
    
    error.with_diagnostic(diagnostic)
}

/// 不正な式エラーを作成
///
/// # 引数
/// * `token` - エラーが発生したトークン
///
/// # 戻り値
/// 不正な式エラーを表すCompilerError
pub fn invalid_expression_error(token: &Token) -> CompilerError {
    let error = syntax_error(
        format!("不正な式です: {}", token_to_string(token)),
        token,
    );
    
    // 追加の診断情報
    let diagnostic = Diagnostic::note(
        "有効な式の例: 変数、リテラル、演算子を含む式、関数呼び出しなど",
        None,
    );
    
    error.with_diagnostic(diagnostic)
}

/// 不正な文エラーを作成
///
/// # 引数
/// * `token` - エラーが発生したトークン
///
/// # 戻り値
/// 不正な文エラーを表すCompilerError
pub fn invalid_statement_error(token: &Token) -> CompilerError {
    let error = syntax_error(
        format!("不正な文です: {}", token_to_string(token)),
        token,
    );
    
    // 追加の診断情報
    let diagnostic = Diagnostic::note(
        "有効な文の例: 変数宣言、代入、制御フロー文、関数呼び出しなど",
        None,
    );
    
    error.with_diagnostic(diagnostic)
}

/// 不正な型エラーを作成
///
/// # 引数
/// * `token` - エラーが発生したトークン
///
/// # 戻り値
/// 不正な型エラーを表すCompilerError
pub fn invalid_type_error(token: &Token) -> CompilerError {
    let error = syntax_error(
        format!("不正な型です: {}", token_to_string(token)),
        token,
    );
    
    // 追加の診断情報
    let diagnostic = Diagnostic::note(
        "有効な型の例: 基本型(int, float, string)、配列型、ユーザー定義型など",
        None,
    );
    
    error.with_diagnostic(diagnostic)
}

/// 不正な宣言エラーを作成
///
/// # 引数
/// * `token` - エラーが発生したトークン
///
/// # 戻り値
/// 不正な宣言エラーを表すCompilerError
pub fn invalid_declaration_error(token: &Token) -> CompilerError {
    let error = syntax_error(
        format!("不正な宣言です: {}", token_to_string(token)),
        token,
    );
    
    // 追加の診断情報
    let diagnostic = Diagnostic::note(
        "有効な宣言の例: 変数宣言、関数宣言、型宣言など",
        None,
    );
    
    error.with_diagnostic(diagnostic)
}

/// 予期しないトークンエラーを作成
///
/// # 引数
/// * `token` - 予期しないトークン
///
/// # 戻り値
/// 予期しないトークンエラーを表すCompilerError
pub fn unexpected_token_error(token: &Token) -> CompilerError {
    let error = syntax_error(
        format!("予期しないトークンです: {}", token_to_string(token)),
        token,
    );
    
    // 追加の診断情報
    let diagnostic = Diagnostic::note(
        "このコンテキストでは別のトークンが期待されています",
        None,
    );
    
    error.with_diagnostic(diagnostic)
}

/// 未終了エラーを作成
///
/// # 引数
/// * `what` - 未終了の構造物の説明（例: "ブロック", "文字列"）
/// * `start` - 構造物の開始トークン
/// * `current` - 現在のトークン（エラーが検出された位置）
///
/// # 戻り値
/// 未終了エラーを表すCompilerError
pub fn unterminated_error(what: &str, start: &Token, current: &Token) -> CompilerError {
    let error = CompilerError::syntax_error(
        format!("未終了の{}です", what),
        Some(current.location.clone()),
    );
    
    // 開始位置の診断情報
    let start_diagnostic = Diagnostic::note(
        format!("{}はここから始まります", what),
        Some(start.location.clone()),
    );
    
    // 現在位置の診断情報
    let current_diagnostic = Diagnostic::error(
        format!("{}が終了していません", what),
        Some(current.location.clone()),
    ).with_suggestion(format!("{}を適切に終了してください", what));
    
    error.with_diagnostic(start_diagnostic).with_diagnostic(current_diagnostic)
}

/// ファイル終端エラーを作成
///
/// # 引数
/// * `message` - エラーメッセージ
/// * `last_token` - 最後に処理されたトークン
///
/// # 戻り値
/// ファイル終端エラーを表すCompilerError
pub fn eof_error(message: impl Into<String>, last_token: &Token) -> CompilerError {
    let message = message.into();
    let mut location = last_token.location.clone();
    // 位置を最後のトークンの直後に調整
    location.column += last_token.lexeme.len();
    
    let error = CompilerError::syntax_error(
        message.clone(),
        Some(location.clone()),
    );
    
    // 追加の診断情報
    let diagnostic = Diagnostic::error(
        format!("ファイルが予期せず終了しました: {}", message),
        Some(location),
    ).with_suggestion("ファイルの内容を確認し、必要な構文要素を追加してください");
    
    error.with_diagnostic(diagnostic)
}

/// 不足しているトークンエラーを作成
///
/// # 引数
/// * `expected` - 期待されるトークンの説明
/// * `location` - エラーの位置
///
/// # 戻り値
/// 不足しているトークンエラーを表すCompilerError
pub fn missing_token_error(expected: &str, location: SourceLocation) -> CompilerError {
    let error = CompilerError::syntax_error(
        format!("'{}'が不足しています", expected),
        Some(location.clone()),
    );
    
    // 追加の診断情報
    let diagnostic = Diagnostic::error(
        format!("'{}'が必要です", expected),
        Some(location),
    ).with_suggestion(format!("'{}'を追加してください", expected));
    
    error.with_diagnostic(diagnostic)
}

/// 構文の不一致エラーを作成
///
/// # 引数
/// * `expected` - 期待される構文の説明
/// * `found` - 実際に見つかったトークン
///
/// # 戻り値
/// 構文の不一致エラーを表すCompilerError
pub fn syntax_mismatch_error(expected: &str, found: &Token) -> CompilerError {
    let error = syntax_error(
        format!("{}が期待されましたが、{}が見つかりました", expected, token_to_string(found)),
        found,
    );
    
    // 追加の診断情報
    let diagnostic = Diagnostic::note(
        format!("正しい構文: {}", expected),
        None,
    );
    
    error.with_diagnostic(diagnostic)
}

/// トークンの文字列表現を取得
///
/// # 引数
/// * `token` - 文字列表現を取得するトークン
///
/// # 戻り値
/// トークンの人間が読める文字列表現
fn token_to_string(token: &Token) -> String {
    match &token.kind {
        TokenKind::Identifier(name) => format!("識別子 '{}'", name),
        TokenKind::IntLiteral(val) => format!("整数リテラル {}", val),
        TokenKind::FloatLiteral(val) => format!("浮動小数点数リテラル {}", val),
        TokenKind::StringLiteral(val) => format!("文字列リテラル \"{}\"", val),
        TokenKind::CharLiteral(c) => format!("文字リテラル '{}'", c),
        TokenKind::BoolLiteral(b) => format!("真偽値リテラル {}", if *b { "true" } else { "false" }),
        TokenKind::Keyword(kw) => format!("キーワード '{}'", kw),
        _ => format!("{}", token.kind),
    }
}
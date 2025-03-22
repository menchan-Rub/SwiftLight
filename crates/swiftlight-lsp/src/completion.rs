// SwiftLight言語のLSP補完モジュール
//
// このモジュールはSwiftLightのLSPサーバーが提供するコード補完機能を実装します。
// エディタ上でのコード入力時に適切な候補を提示します。

use std::sync::{Arc, Mutex};

use anyhow::Result;
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse, Documentation,
    InsertTextFormat, MarkupContent, MarkupKind, Position, Range, TextEdit,
};

use swiftlight_compiler::{
    frontend::{
        lexer::{tokenize, Token},
        parser,
        source_map::SourceMap,
    },
    frontend::semantic::{
        name_resolution::NameResolver,
        type_checker::TypeChecker,
    },
};

use crate::ServerState;

/// コード補完を処理
pub async fn handle_completion(
    state: Arc<Mutex<ServerState>>,
    params: CompletionParams,
) -> Result<Option<CompletionResponse>> {
    let uri = &params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;
    
    // ドキュメントの取得
    let content = {
        let state = state.lock().unwrap();
        match state.get_document(uri) {
            Some(doc) => doc.content.clone(),
            None => {
                log::warn!("補完を行うドキュメントが見つかりません: {}", uri);
                return Ok(None);
            }
        }
    };
    
    // URIをファイルパスに変換
    let file_path = match uri.to_file_path() {
        Ok(path) => path,
        Err(_) => {
            log::warn!("URIをファイルパスに変換できません: {}", uri);
            return Ok(None);
        }
    };
    
    // ファイル名の取得
    let file_name = file_path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown.swl");
    
    // 現在の位置のコンテキストを解析
    let completion_context = analyze_completion_context(&content, position)?;
    
    // コンテキストに基づいて補完候補を生成
    let completion_items = match completion_context {
        CompletionContext::Empty => {
            // 空の状態では、キーワードや基本的な構文要素を提案
            get_keyword_completions()
        },
        CompletionContext::AfterDot(expr) => {
            // ドット演算子の後では、メンバーアクセスを提案
            get_member_completions(&expr, &content, file_name)
        },
        CompletionContext::AfterDoubleColon(typename) => {
            // 二重コロンの後では、静的メンバーやEnum値を提案
            get_static_completions(&typename, &content, file_name)
        },
        CompletionContext::TypeAnnotation => {
            // 型アノテーションのコンテキストでは、型名を提案
            get_type_completions(&content, file_name)
        },
        CompletionContext::Import => {
            // importステートメントでは、モジュール名を提案
            get_module_completions(&state)
        },
        CompletionContext::Normal => {
            // 通常のコンテキストでは、ローカル変数、関数、型などを提案
            let mut completions = get_local_completions(&content, position, file_name);
            completions.extend(get_keyword_completions());
            completions
        },
    };
    
    Ok(Some(CompletionResponse::Array(completion_items)))
}

/// 補完のコンテキスト
enum CompletionContext {
    /// 空のドキュメントまたは行の先頭
    Empty,
    /// ドット演算子の後（メンバーアクセス）
    AfterDot(String),
    /// 二重コロンの後（静的メンバーアクセス）
    AfterDoubleColon(String),
    /// 型アノテーションのコンテキスト
    TypeAnnotation,
    /// インポートステートメント
    Import,
    /// 通常のコンテキスト
    Normal,
}

/// 補完コンテキストを解析
fn analyze_completion_context(content: &str, position: Position) -> Result<CompletionContext> {
    let line_idx = position.line as usize;
    let lines: Vec<&str> = content.lines().collect();
    
    if line_idx >= lines.len() {
        return Ok(CompletionContext::Empty);
    }
    
    let line = lines[line_idx];
    let char_idx = position.character as usize;
    
    if char_idx == 0 || line.is_empty() {
        return Ok(CompletionContext::Empty);
    }
    
    let line_prefix = &line[..char_idx.min(line.len())];
    
    // ドット演算子の後かチェック
    if let Some(prefix) = line_prefix.strip_suffix('.') {
        // 前の部分から式を抽出
        let expr = extract_expression(prefix);
        return Ok(CompletionContext::AfterDot(expr));
    }
    
    // 二重コロンの後かチェック
    if let Some(prefix) = line_prefix.strip_suffix("::") {
        // 前の部分から型名を抽出
        let typename = extract_typename(prefix);
        return Ok(CompletionContext::AfterDoubleColon(typename));
    }
    
    // 型アノテーションのコンテキストかチェック
    if line_prefix.contains(':') && !line_prefix.contains('=') {
        return Ok(CompletionContext::TypeAnnotation);
    }
    
    // インポートステートメントかチェック
    if line_prefix.trim_start().starts_with("import ") || 
       line_prefix.trim_start().starts_with("use ") {
        return Ok(CompletionContext::Import);
    }
    
    // それ以外は通常のコンテキスト
    Ok(CompletionContext::Normal)
}

/// 式を抽出（単純な実装）
fn extract_expression(text: &str) -> String {
    // 最後の識別子を取得（実際の実装ではASTを使うべき）
    let mut result = String::new();
    let mut chars = text.chars().rev();
    
    // 最初の非英数字で終了
    while let Some(c) = chars.next() {
        if c.is_alphanumeric() || c == '_' {
            result.push(c);
        } else {
            break;
        }
    }
    
    // 逆順にしたので戻す
    result.chars().rev().collect()
}

/// 型名を抽出（単純な実装）
fn extract_typename(text: &str) -> String {
    extract_expression(text) // 簡易実装では同じロジック
}

/// キーワード補完の取得
fn get_keyword_completions() -> Vec<CompletionItem> {
    let keywords = [
        ("let", "変数宣言"),
        ("var", "可変変数宣言"),
        ("const", "定数宣言"),
        ("func", "関数宣言"),
        ("class", "クラス宣言"),
        ("struct", "構造体宣言"),
        ("enum", "列挙型宣言"),
        ("interface", "インターフェース宣言"),
        ("trait", "トレイト宣言"),
        ("impl", "実装ブロック"),
        ("if", "条件分岐"),
        ("else", "else節"),
        ("for", "forループ"),
        ("while", "whileループ"),
        ("return", "関数からの戻り値"),
        ("import", "モジュールのインポート"),
        ("use", "要素の使用宣言"),
        ("pub", "公開アクセス修飾子"),
        ("private", "private修飾子"),
        ("mut", "可変修飾子"),
        ("static", "静的修飾子"),
    ];
    
    keywords.iter().map(|(keyword, doc)| {
        CompletionItem {
            label: keyword.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: None,
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::MARKDOWN,
                value: format!("**{}**\n\n{}", keyword, doc),
            })),
            deprecated: None,
            preselect: None,
            sort_text: None,
            filter_text: None,
            insert_text: Some(keyword.to_string()),
            insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
            text_edit: None,
            additional_text_edits: None,
            commit_characters: None,
            command: None,
            data: None,
            tags: None,
        }
    }).collect()
}

/// メンバー補完の取得
fn get_member_completions(expr: &str, content: &str, file_name: &str) -> Vec<CompletionItem> {
    // 実装では型推論を使って式の型を特定し、そのメンバーを提案する
    // ここでは簡易版
    match expr {
        "String" => {
            // 文字列型のメソッド例
            vec![
                create_method_completion("length", "文字列の長さを返す", "() -> Int"),
                create_method_completion("substring", "部分文字列を取得", "(start: Int, end: Int) -> String"),
                create_method_completion("replace", "文字列を置換", "(target: String, replacement: String) -> String"),
                create_method_completion("isEmpty", "文字列が空かどうかを返す", "() -> Bool"),
            ]
        },
        "Array" => {
            // 配列型のメソッド例
            vec![
                create_method_completion("count", "配列の要素数を返す", "() -> Int"),
                create_method_completion("append", "要素を追加", "(element: T) -> void"),
                create_method_completion("remove", "要素を削除", "(at: Int) -> T"),
                create_method_completion("isEmpty", "配列が空かどうかを返す", "() -> Bool"),
            ]
        },
        // その他の型
        _ => vec![],
    }
}

/// 静的メンバー補完の取得
fn get_static_completions(typename: &str, content: &str, file_name: &str) -> Vec<CompletionItem> {
    // 実装では型の静的メンバーを解析して提案
    // ここでは簡易版
    match typename {
        "Math" => {
            vec![
                create_completion("PI", "円周率の値", CompletionItemKind::CONSTANT),
                create_completion("E", "自然対数の底", CompletionItemKind::CONSTANT),
                create_method_completion("sqrt", "平方根を計算", "(value: Float) -> Float"),
                create_method_completion("abs", "絶対値を計算", "(value: Int) -> Int"),
            ]
        },
        "System" => {
            vec![
                create_method_completion("print", "標準出力に出力", "(message: String) -> void"),
                create_method_completion("exit", "プログラムを終了", "(code: Int) -> void"),
                create_completion("VERSION", "システムのバージョン", CompletionItemKind::CONSTANT),
            ]
        },
        // その他の型
        _ => vec![],
    }
}

/// 型名補完の取得
fn get_type_completions(content: &str, file_name: &str) -> Vec<CompletionItem> {
    // 基本型とよく使われる型の補完
    let types = [
        ("Int", "整数型"),
        ("Float", "浮動小数点型"),
        ("Double", "倍精度浮動小数点型"),
        ("Bool", "真偽値型"),
        ("String", "文字列型"),
        ("Char", "文字型"),
        ("Array", "配列型"),
        ("Dictionary", "辞書型"),
        ("Optional", "オプショナル型"),
        ("Result", "結果型"),
        ("Void", "空の戻り値型"),
    ];
    
    types.iter().map(|(type_name, doc)| {
        CompletionItem {
            label: type_name.to_string(),
            kind: Some(CompletionItemKind::CLASS),
            detail: Some(format!("型: {}", type_name)),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::MARKDOWN,
                value: format!("**{}**\n\n{}", type_name, doc),
            })),
            deprecated: None,
            preselect: None,
            sort_text: None,
            filter_text: None,
            insert_text: Some(type_name.to_string()),
            insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
            text_edit: None,
            additional_text_edits: None,
            commit_characters: None,
            command: None,
            data: None,
            tags: None,
        }
    }).collect()
}

/// モジュール名補完の取得
fn get_module_completions(state: &Arc<Mutex<ServerState>>) -> Vec<CompletionItem> {
    // 標準ライブラリのモジュール
    let std_modules = [
        ("core", "コア機能モジュール"),
        ("std", "標準機能モジュール"),
        ("io", "入出力モジュール"),
        ("math", "数学関数モジュール"),
        ("collections", "コレクションモジュール"),
        ("time", "時間関連モジュール"),
        ("fs", "ファイルシステムモジュール"),
        ("net", "ネットワークモジュール"),
        ("gui", "グラフィカルインターフェースモジュール"),
        ("concurrent", "並行処理モジュール"),
    ];
    
    std_modules.iter().map(|(module_name, doc)| {
        CompletionItem {
            label: module_name.to_string(),
            kind: Some(CompletionItemKind::MODULE),
            detail: Some(format!("モジュール: {}", module_name)),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::MARKDOWN,
                value: format!("**{}**\n\n{}", module_name, doc),
            })),
            deprecated: None,
            preselect: None,
            sort_text: None,
            filter_text: None,
            insert_text: Some(module_name.to_string()),
            insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
            text_edit: None,
            additional_text_edits: None,
            commit_characters: None,
            command: None,
            data: None,
            tags: None,
        }
    }).collect()
}

/// ローカル補完の取得
fn get_local_completions(content: &str, position: Position, file_name: &str) -> Vec<CompletionItem> {
    // 現在のスコープでのローカル変数や関数を解析して提供
    // 実際の実装ではシンボルテーブルを使用すべき
    
    // 簡易実装: 行から変数宣言を検出
    let lines: Vec<&str> = content.lines().collect();
    let mut completions = Vec::new();
    
    for (i, line) in lines.iter().enumerate() {
        if i as u32 >= position.line {
            break;
        }
        
        // 変数宣言の検出（簡易実装）
        if let Some(var_name) = extract_variable_declaration(line) {
            completions.push(create_completion(
                &var_name,
                "ローカル変数",
                CompletionItemKind::VARIABLE,
            ));
        }
        
        // 関数宣言の検出（簡易実装）
        if let Some(func_name) = extract_function_declaration(line) {
            completions.push(create_completion(
                &func_name,
                "関数",
                CompletionItemKind::FUNCTION,
            ));
        }
    }
    
    completions
}

/// 変数宣言を検出（簡易実装）
fn extract_variable_declaration(line: &str) -> Option<String> {
    let line = line.trim();
    if line.starts_with("let ") || line.starts_with("var ") || line.starts_with("const ") {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let var_name = parts[1].trim_end_matches(':');
            return Some(var_name.to_string());
        }
    }
    None
}

/// 関数宣言を検出（簡易実装）
fn extract_function_declaration(line: &str) -> Option<String> {
    let line = line.trim();
    if line.starts_with("fn ") || line.starts_with("func ") {
        let rest = &line[3..]; // "fn "の後
        if let Some(paren_pos) = rest.find('(') {
            let func_name = rest[..paren_pos].trim();
            return Some(func_name.to_string());
        }
    }
    None
}

/// 補完アイテムの作成ヘルパー
fn create_completion(
    label: &str,
    detail: &str,
    kind: CompletionItemKind,
) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(kind),
        detail: Some(detail.to_string()),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::MARKDOWN,
            value: format!("**{}**\n\n{}", label, detail),
        })),
        deprecated: None,
        preselect: None,
        sort_text: None,
        filter_text: None,
        insert_text: Some(label.to_string()),
        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
        text_edit: None,
        additional_text_edits: None,
        commit_characters: None,
        command: None,
        data: None,
        tags: None,
    }
}

/// メソッド補完アイテムの作成ヘルパー
fn create_method_completion(
    name: &str,
    doc: &str,
    signature: &str,
) -> CompletionItem {
    let snippet = if signature == "() -> void" || signature == "() -> Void" {
        format!("{}()", name)
    } else {
        format!("{}($1)", name)
    };
    
    CompletionItem {
        label: name.to_string(),
        kind: Some(CompletionItemKind::METHOD),
        detail: Some(signature.to_string()),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::MARKDOWN,
            value: format!("**{}**{}\n\n{}", name, signature, doc),
        })),
        deprecated: None,
        preselect: None,
        sort_text: None,
        filter_text: None,
        insert_text: Some(snippet),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        text_edit: None,
        additional_text_edits: None,
        commit_characters: None,
        command: None,
        data: None,
        tags: None,
    }
}

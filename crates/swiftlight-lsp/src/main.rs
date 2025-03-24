// SwiftLight言語のLSPサーバー
// 
// このモジュールはLanguage Server Protocol実装のエントリーポイントを提供します。
// LSPを通じてエディタ（VSCode、Vimなど）とSwiftLightコンパイラを接続します。

use std::{
    collections::HashMap,
    error::Error,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use anyhow::{anyhow, Result};
use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::{
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, PublishDiagnostics,
    },
    request::{Completion, GotoDefinition, HoverRequest, References},
    CompletionItem, CompletionOptions, CompletionParams, CompletionResponse, Diagnostic,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverParams, InitializeParams,
    InitializeResult, InitializedParams, Location, OneOf, Position, PublishDiagnosticsParams, Range,
    ReferenceParams, ServerCapabilities, TextDocumentContentChangeEvent, TextDocumentIdentifier,
    TextDocumentItem, TextDocumentPositionParams, TextDocumentSyncCapability, TextDocumentSyncKind,
    Url, WorkDoneProgressOptions,
};
use serde_json::Value;
use tokio::runtime::Runtime;

mod completion;
mod diagnostics;
pub mod syntax_highlight;

// SwiftLightドキュメントの情報を保持する構造体
struct DocumentState {
    uri: Url,
    version: i32,
    content: String,
}

// LSPサーバーの状態
struct ServerState {
    documents: HashMap<Url, DocumentState>,
    workspace_root: Option<PathBuf>,
    syntax_highlighter: syntax_highlight::LspSyntaxHighlighter,
}

impl ServerState {
    fn new() -> Self {
        let diagnostics = Arc::new(diagnostics::DiagnosticEmitter::new());
        
        Self {
            documents: HashMap::new(),
            workspace_root: None,
            syntax_highlighter: syntax_highlight::LspSyntaxHighlighter::new(diagnostics.clone()),
        }
    }

    // ドキュメントの追加または更新
    fn update_document(&mut self, uri: Url, version: i32, content: String) {
        self.documents.insert(
            uri.clone(),
            DocumentState {
                uri,
                version,
                content,
            },
        );
    }

    // ドキュメントの削除
    fn remove_document(&mut self, uri: &Url) {
        self.documents.remove(uri);
    }

    // ドキュメントの取得
    fn get_document(&self, uri: &Url) -> Option<&DocumentState> {
        self.documents.get(uri)
    }
}

fn main() -> Result<(), Box<dyn Error + Sync + Send>> {
    // ロギングの初期化
    env_logger::init();
    
    // LSP接続の確立
    log::info!("SwiftLight LSPサーバーを起動しています...");
    let (connection, io_threads) = Connection::stdio();

    // サーバー状態の初期化
    let server_state = Arc::new(Mutex::new(ServerState::new()));
    
    // 非同期ランタイムの作成
    let rt = Runtime::new()?;
    
    // メインループ
    let main_loop = async {
        // クライアントからの初期化リクエストを待機
        let (initialize_id, initialize_params) = connection.initialize_start()?;
        let params: InitializeParams = serde_json::from_value(initialize_params)?;
        
        // ワークスペースルートの設定
        if let Some(root_uri) = &params.root_uri {
            let root_path = root_uri.to_file_path().map_err(|_| anyhow!("無効なURI"))?;
            log::info!("ワークスペースルート: {:?}", root_path);
            server_state.lock().unwrap().workspace_root = Some(root_path);
        }
        
        // サーバー機能の設定
        let capabilities = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::INCREMENTAL,
            )),
            completion_provider: Some(CompletionOptions {
                resolve_provider: Some(true),
                trigger_characters: Some(vec![".".to_string(), "::".to_string()]),
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: Some(false),
                },
                all_commit_characters: None,
                completion_item: None,
            }),
            hover_provider: Some(OneOf::Left(true)),
            definition_provider: Some(OneOf::Left(true)),
            references_provider: Some(OneOf::Left(true)),
            semantic_tokens_provider: Some(
                SemanticTokensServerCapabilities::SemanticTokensOptions(
                    SemanticTokensOptions {
                        legend: SemanticTokensLegend {
                            token_types: vec![
                                SemanticTokenType::KEYWORD,
                                SemanticTokenType::FUNCTION,
                                // その他のトークンタイプ
                            ],
                            token_modifiers: vec![
                                SemanticTokenModifier::DECLARATION,
                                SemanticTokenModifier::DEFINITION,
                                // その他の修飾子
                            ],
                        },
                        range: Some(true),
                        full: Some(true),
                    }
                )
            ),
            ..ServerCapabilities::default()
        };
        
        // 初期化完了レスポンスの送信
        let initialize_result = InitializeResult {
            capabilities,
            server_info: Some(lsp_types::ServerInfo {
                name: "swiftlight-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        };
        
        let initialize_result = serde_json::to_value(initialize_result)?;
        connection.initialize_finish(initialize_id, initialize_result)?;
        
        // メインメッセージループ
        log::info!("LSPサーバーの初期化が完了しました。メッセージの処理を開始します。");
        handle_messages(connection, server_state).await?;
        
        Ok::<(), anyhow::Error>(())
    };
    
    // ブロッキングしないランタイムで実行
    match rt.block_on(main_loop) {
        Ok(_) => log::info!("LSPサーバーが正常に終了しました。"),
        Err(e) => log::error!("LSPサーバーエラー: {}", e),
    }
    
    // I/Oスレッドの終了を待機
    io_threads.join()?;
    
    Ok(())
}

// LSPメッセージの処理
async fn handle_messages(
    connection: Connection,
    state: Arc<Mutex<ServerState>>,
) -> Result<()> {
    loop {
        let message = match connection.receiver.recv() {
            Ok(message) => message,
            Err(e) => {
                log::error!("メッセージ受信エラー: {}", e);
                break;
            }
        };
        
        match message {
            Message::Request(request) => {
                handle_request(&connection, state.clone(), request).await?;
            }
            Message::Response(_) => {
                // 現在はレスポンスを特に処理しない
            }
            Message::Notification(notification) => {
                handle_notification(&connection, state.clone(), notification).await?;
            }
        }
    }
    
    Ok(())
}

// リクエストの処理
async fn handle_request(
    connection: &Connection,
    state: Arc<Mutex<ServerState>>,
    request: Request,
) -> Result<()> {
    log::debug!("リクエスト受信: {:?} - {}", request.id, request.method);
    
    match request.method.as_str() {
        // コード補完
        "textDocument/completion" => {
            let params: CompletionParams = serde_json::from_value(request.params)?;
            let result = completion::handle_completion(state, params).await?;
            let response = Response::new_ok(request.id, result);
            connection.sender.send(Message::Response(response))?;
        }
        
        // 定義へ移動
        "textDocument/definition" => {
            let params: GotoDefinitionParams = serde_json::from_value(request.params)?;
            let result = handle_goto_definition(state, params).await?;
            let response = Response::new_ok(request.id, result);
            connection.sender.send(Message::Response(response))?;
        }
        
        // ホバー情報
        "textDocument/hover" => {
            let params: HoverParams = serde_json::from_value(request.params)?;
            let result = handle_hover(state, params).await?;
            let response = Response::new_ok(request.id, result);
            connection.sender.send(Message::Response(response))?;
        }
        
        // 参照検索
        "textDocument/references" => {
            let params: ReferenceParams = serde_json::from_value(request.params)?;
            let result = handle_references(state, params).await?;
            let response = Response::new_ok(request.id, result);
            connection.sender.send(Message::Response(response))?;
        }
        
        // セマンティックトークンリクエスト
        "textDocument/semanticTokens/full" => {
            let params: SemanticTokensParams = serde_json::from_value(request.params)?;
            let result = semantic_tokens_request(&state, params);
            let response = Response::new_ok(request.id, result);
            connection.sender.send(Message::Response(response))?;
        }
        
        // セマンティックトークンリクエスト（範囲指定）
        "textDocument/semanticTokens/range" => {
            let params: SemanticTokensRangeParams = serde_json::from_value(request.params)?;
            let full_params = SemanticTokensParams {
                text_document: params.text_document,
                work_done_progress_params: params.work_done_progress_params,
            };
            let result = semantic_tokens_request(&state, full_params);
            let response = Response::new_ok(request.id, result);
            connection.sender.send(Message::Response(response))?;
        }
        
        // 未実装のリクエスト
        _ => {
            log::warn!("未実装のリクエスト: {}", request.method);
            let response = Response::new_err(
                request.id,
                lsp_server::ErrorCode::MethodNotFound as i32,
                format!("Method not implemented: {}", request.method),
            );
            connection.sender.send(Message::Response(response))?;
        }
    }
    
    Ok(())
}

// 通知の処理
async fn handle_notification(
    connection: &Connection,
    state: Arc<Mutex<ServerState>>,
    notification: Notification,
) -> Result<()> {
    log::debug!("通知受信: {}", notification.method);
    
    match notification.method.as_str() {
        // 初期化完了通知
        "initialized" => {
            let _params: InitializedParams = serde_json::from_value(notification.params)?;
            log::info!("クライアントの初期化が完了しました");
        }
        
        // ドキュメントが開かれた通知
        "textDocument/didOpen" => {
            let params: DidOpenTextDocumentParams = serde_json::from_value(notification.params)?;
            handle_document_open(connection, state, params).await?;
        }
        
        // ドキュメントが変更された通知
        "textDocument/didChange" => {
            let params: DidChangeTextDocumentParams = serde_json::from_value(notification.params)?;
            handle_document_change(connection, state, params).await?;
        }
        
        // ドキュメントが閉じられた通知
        "textDocument/didClose" => {
            let params: DidCloseTextDocumentParams = serde_json::from_value(notification.params)?;
            handle_document_close(connection, state, params).await?;
        }
        
        // その他の通知
        _ => {
            log::debug!("未処理の通知: {}", notification.method);
        }
    }
    
    Ok(())
}

// ドキュメントが開かれた時の処理
async fn handle_document_open(
    connection: &Connection,
    state: Arc<Mutex<ServerState>>,
    params: DidOpenTextDocumentParams,
) -> Result<()> {
    let text_document = params.text_document;
    
    log::info!(
        "ドキュメントが開かれました: {} (version: {})",
        text_document.uri, text_document.version
    );
    
    // ドキュメント状態の更新
    {
        let mut state = state.lock().unwrap();
        state.update_document(
            text_document.uri.clone(),
            text_document.version,
            text_document.text.clone(),
        );
    }
    
    // 診断結果の生成と送信
    let diagnostics = diagnostics::get_diagnostics(state.clone(), &text_document.uri).await?;
    send_diagnostics(connection, text_document.uri, diagnostics)?;
    
    // 構文ハイライトも更新
    state.syntax_highlighter.update_document(&text_document.uri.to_string(), &text_document.text);
    
    Ok(())
}

// ドキュメントが変更された時の処理
async fn handle_document_change(
    connection: &Connection,
    state: Arc<Mutex<ServerState>>,
    params: DidChangeTextDocumentParams,
) -> Result<()> {
    let document_id = params.text_document;
    let changes = params.content_changes;
    
    log::debug!(
        "ドキュメントが変更されました: {} (version: {})",
        document_id.uri, document_id.version
    );
    
    // ドキュメントの取得と更新
    let mut content = {
        let state = state.lock().unwrap();
        match state.get_document(&document_id.uri) {
            Some(doc) => doc.content.clone(),
            None => {
                log::warn!("未知のドキュメントが変更されました: {}", document_id.uri);
                return Ok(());
            }
        }
    };
    
    // 変更の適用
    for change in changes {
        if let Some(range) = change.range {
            // 部分的な変更
            let start_pos = position_to_index(&content, range.start)?;
            let end_pos = position_to_index(&content, range.end)?;
            
            content.replace_range(start_pos..end_pos, &change.text);
        } else {
            // 全体の置換
            content = change.text;
        }
    }
    
    // ドキュメント状態の更新
    {
        let mut state = state.lock().unwrap();
        state.update_document(
            document_id.uri.clone(),
            document_id.version.unwrap_or(0),
            content,
        );
    }
    
    // 診断結果の更新と送信
    let diagnostics = diagnostics::get_diagnostics(state.clone(), &document_id.uri).await?;
    send_diagnostics(connection, document_id.uri, diagnostics)?;
    
    // 構文ハイライトも更新
    state.syntax_highlighter.update_document(&document_id.uri.to_string(), &content);
    
    Ok(())
}

// ドキュメントが閉じられた時の処理
async fn handle_document_close(
    connection: &Connection,
    state: Arc<Mutex<ServerState>>,
    params: DidCloseTextDocumentParams,
) -> Result<()> {
    let document_id = params.text_document;
    
    log::info!("ドキュメントが閉じられました: {}", document_id.uri);
    
    // ドキュメントの削除
    {
        let mut state = state.lock().unwrap();
        state.remove_document(&document_id.uri);
    }
    
    // 診断結果のクリア
    send_diagnostics(connection, document_id.uri, vec![])?;
    
    Ok(())
}

// 定義へ移動機能の処理
async fn handle_goto_definition(
    state: Arc<Mutex<ServerState>>,
    params: GotoDefinitionParams,
) -> Result<Option<GotoDefinitionResponse>> {
    let _position = params.text_document_position_params.position;
    let _uri = params.text_document_position_params.text_document.uri;
    
    // 仮実装: 実際のコンパイラと統合後に完全実装する
    // 現在は空のレスポンスを返す
    Ok(None)
}

// ホバー情報の処理
async fn handle_hover(
    state: Arc<Mutex<ServerState>>,
    params: HoverParams,
) -> Result<Option<Hover>> {
    let _position = params.text_document_position_params.position;
    let _uri = params.text_document_position_params.text_document.uri;
    
    // 仮実装: 実際のコンパイラと統合後に完全実装する
    // 現在は空のレスポンスを返す
    Ok(None)
}

// 参照検索の処理
async fn handle_references(
    state: Arc<Mutex<ServerState>>,
    params: ReferenceParams,
) -> Result<Option<Vec<Location>>> {
    let _position = params.text_document_position.position;
    let _uri = params.text_document_position.text_document.uri;
    let _include_declaration = params.context.include_declaration;
    
    // 仮実装: 実際のコンパイラと統合後に完全実装する
    // 現在は空のレスポンスを返す
    Ok(None)
}

// 診断結果の送信
fn send_diagnostics(
    connection: &Connection,
    uri: Url,
    diagnostics: Vec<Diagnostic>,
) -> Result<()> {
    let params = PublishDiagnosticsParams {
        uri,
        diagnostics,
        version: None,
    };
    
    let notification = Notification::new(
        PublishDiagnostics::METHOD.to_string(),
        serde_json::to_value(params)?,
    );
    
    connection.sender.send(Message::Notification(notification))?;
    Ok(())
}

// 位置をインデックスに変換
fn position_to_index(text: &str, position: Position) -> Result<usize> {
    let mut line = 0;
    let mut column = 0;
    let mut index = 0;
    
    for c in text.chars() {
        if line == position.line as usize && column == position.character as usize {
            return Ok(index);
        }
        
        if c == '\n' {
            line += 1;
            column = 0;
        } else {
            column += 1;
        }
        
        index += c.len_utf8();
    }
    
    if line == position.line as usize && column == position.character as usize {
        return Ok(index);
    }
    
    Err(anyhow!("無効な位置: {:?}", position))
}

// セマンティックトークンリクエストのハンドラ
fn semantic_tokens_request(state: &ServerState, params: SemanticTokensParams) -> Result<Option<SemanticTokens>, ResponseError> {
    let uri = params.text_document.uri.to_string();
    
    Ok(state.syntax_highlighter.generate_semantic_tokens(&uri))
}

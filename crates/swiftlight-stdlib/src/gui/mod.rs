//! # SwiftLight言語のGUIモジュール
//! 
//! このモジュールはグラフィカルユーザーインターフェース（GUI）の作成と
//! 管理のための機能を提供します。
//! 
//! ウィンドウ、ボタン、テキストフィールドなどの基本的なUIコンポーネントと
//! レイアウト管理が含まれています。

use crate::core::types::{Error, ErrorKind, Result};
use crate::core::collections::{Vec, HashMap};
use std::rc::Rc;
use std::cell::{RefCell, Cell};

/// 色を表す構造体
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    /// 赤成分 (0.0-1.0)
    pub r: f32,
    /// 緑成分 (0.0-1.0)
    pub g: f32,
    /// 青成分 (0.0-1.0)
    pub b: f32,
    /// アルファ成分 (0.0-1.0)
    pub a: f32,
}

impl Color {
    /// 新しい色を作成
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
    
    /// RGBから新しい色を作成（アルファは1.0）
    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::new(r, g, b, 1.0)
    }
    
    /// 16進数文字列から色を作成
    pub fn from_hex(hex: &str) -> Result<Self> {
        let hex = hex.trim_start_matches('#');
        
        if hex.len() != 6 && hex.len() != 8 {
            return Err(Error::new(
                ErrorKind::InvalidArgument,
                "16進数色コードは6桁または8桁である必要があります"
            ));
        }
        
        let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| {
            Error::new(
                ErrorKind::InvalidArgument,
                "不正な16進数色コードです"
            )
        })? as f32 / 255.0;
        
        let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| {
            Error::new(
                ErrorKind::InvalidArgument,
                "不正な16進数色コードです"
            )
        })? as f32 / 255.0;
        
        let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| {
            Error::new(
                ErrorKind::InvalidArgument,
                "不正な16進数色コードです"
            )
        })? as f32 / 255.0;
        
        let a = if hex.len() == 8 {
            u8::from_str_radix(&hex[6..8], 16).map_err(|_| {
                Error::new(
                    ErrorKind::InvalidArgument,
                    "不正な16進数色コードです"
                )
            })? as f32 / 255.0
        } else {
            1.0
        };
        
        Ok(Self::new(r, g, b, a))
    }
    
    /// 定義済みの色: 黒
    pub fn black() -> Self {
        Self::rgb(0.0, 0.0, 0.0)
    }
    
    /// 定義済みの色: 白
    pub fn white() -> Self {
        Self::rgb(1.0, 1.0, 1.0)
    }
    
    /// 定義済みの色: 赤
    pub fn red() -> Self {
        Self::rgb(1.0, 0.0, 0.0)
    }
    
    /// 定義済みの色: 緑
    pub fn green() -> Self {
        Self::rgb(0.0, 1.0, 0.0)
    }
    
    /// 定義済みの色: 青
    pub fn blue() -> Self {
        Self::rgb(0.0, 0.0, 1.0)
    }
    
    /// 定義済みの色: 黄色
    pub fn yellow() -> Self {
        Self::rgb(1.0, 1.0, 0.0)
    }
    
    /// 定義済みの色: マゼンタ
    pub fn magenta() -> Self {
        Self::rgb(1.0, 0.0, 1.0)
    }
    
    /// 定義済みの色: シアン
    pub fn cyan() -> Self {
        Self::rgb(0.0, 1.0, 1.0)
    }
    
    /// 定義済みの色: 透明
    pub fn transparent() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0)
    }
    
    /// 16進数文字列に変換
    pub fn to_hex(&self) -> String {
        let r = (self.r * 255.0) as u8;
        let g = (self.g * 255.0) as u8;
        let b = (self.b * 255.0) as u8;
        let a = (self.a * 255.0) as u8;
        
        if self.a < 1.0 {
            format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a)
        } else {
            format!("#{:02x}{:02x}{:02x}", r, g, b)
        }
    }
}

/// 位置とサイズを表す構造体
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// X座標
    pub x: f32,
    /// Y座標
    pub y: f32,
    /// 幅
    pub width: f32,
    /// 高さ
    pub height: f32,
}

impl Rect {
    /// 新しい矩形を作成
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }
    
    /// 位置を設定
    pub fn with_position(mut self, x: f32, y: f32) -> Self {
        self.x = x;
        self.y = y;
        self
    }
    
    /// サイズを設定
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }
    
    /// 矩形が点を含むかチェック
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.width &&
        y >= self.y && y <= self.y + self.height
    }
    
    /// 矩形同士が重なっているかチェック
    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.width &&
        self.x + self.width > other.x &&
        self.y < other.y + other.height &&
        self.y + self.height > other.y
    }
}

/// イベント種別
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventType {
    /// マウスボタンが押された
    MouseDown,
    /// マウスボタンが離された
    MouseUp,
    /// マウスが移動した
    MouseMove,
    /// マウスホイールが回転した
    MouseWheel,
    /// キーが押された
    KeyDown,
    /// キーが離された
    KeyUp,
    /// ウィンドウがリサイズされた
    Resize,
    /// ウィンドウが閉じられようとしている
    Close,
    /// ウィンドウがフォーカスを得た
    Focus,
    /// ウィンドウがフォーカスを失った
    Blur,
}

/// イベントデータ
#[derive(Debug, Clone)]
pub struct Event {
    /// イベントの種別
    pub event_type: EventType,
    /// イベント発生時のX座標
    pub x: f32,
    /// イベント発生時のY座標
    pub y: f32,
    /// キーコード（キーイベントの場合）
    pub key_code: Option<u32>,
    /// マウスボタン（マウスイベントの場合）
    pub button: Option<u8>,
    /// 押されているモディファイアキー
    pub modifiers: u8,
}

impl Event {
    /// 新しいイベントを作成
    pub fn new(event_type: EventType) -> Self {
        Self {
            event_type,
            x: 0.0,
            y: 0.0,
            key_code: None,
            button: None,
            modifiers: 0,
        }
    }
    
    /// 位置情報を設定
    pub fn with_position(mut self, x: f32, y: f32) -> Self {
        self.x = x;
        self.y = y;
        self
    }
    
    /// キーコードを設定
    pub fn with_key_code(mut self, key_code: u32) -> Self {
        self.key_code = Some(key_code);
        self
    }
    
    /// マウスボタンを設定
    pub fn with_button(mut self, button: u8) -> Self {
        self.button = Some(button);
        self
    }
    
    /// モディファイアキーを設定
    pub fn with_modifiers(mut self, modifiers: u8) -> Self {
        self.modifiers = modifiers;
        self
    }
}

/// イベントハンドラ
pub type EventHandler = Box<dyn Fn(&Event) -> bool>;

/// UIコンポーネントのトレイト
pub trait Component {
    /// コンポーネントを描画
    fn draw(&self, ctx: &mut DrawContext);
    
    /// コンポーネントの領域を取得
    fn get_bounds(&self) -> Rect;
    
    /// コンポーネントの領域を設定
    fn set_bounds(&mut self, bounds: Rect);
    
    /// イベントを処理
    fn handle_event(&mut self, event: &Event) -> bool;
    
    /// イベントハンドラを追加
    fn add_event_handler(&mut self, event_type: EventType, handler: EventHandler);
}

/// 描画コンテキスト
pub struct DrawContext {
    // 実際の実装では描画APIの詳細が必要
    pub(crate) _internal: (),
}

impl DrawContext {
    /// 新しい描画コンテキストを作成
    pub(crate) fn new() -> Self {
        Self { _internal: () }
    }
    
    /// 矩形を描画
    pub fn draw_rect(&mut self, rect: Rect, color: Color) {
        // 実際の実装では描画APIの呼び出しを行う
        // 現在はスタブ実装
        println!("矩形を描画: {:?}, 色: {:?}", rect, color);
    }
    
    /// 線を描画
    pub fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: Color, width: f32) {
        // 実際の実装では描画APIの呼び出しを行う
        // 現在はスタブ実装
        println!("線を描画: ({}, {}) → ({}, {}), 色: {:?}, 太さ: {}", x1, y1, x2, y2, color, width);
    }
    
    /// テキストを描画
    pub fn draw_text(&mut self, text: &str, x: f32, y: f32, color: Color, size: f32) {
        // 実際の実装では描画APIの呼び出しを行う
        // 現在はスタブ実装
        println!("テキストを描画: '{}' at ({}, {}), 色: {:?}, サイズ: {}", text, x, y, color, size);
    }
}

/// ウィンドウ
pub struct Window {
    title: String,
    bounds: Rect,
    background_color: Color,
    components: Vec<Box<dyn Component>>,
    event_handlers: HashMap<EventType, Vec<EventHandler>>,
    visible: bool,
}

impl Window {
    /// 新しいウィンドウを作成
    pub fn new(title: &str, width: f32, height: f32) -> Self {
        Self {
            title: title.to_string(),
            bounds: Rect::new(0.0, 0.0, width, height),
            background_color: Color::white(),
            components: Vec::new(),
            event_handlers: HashMap::new(),
            visible: false,
        }
    }
    
    /// コンポーネントを追加
    pub fn add_component<C: Component + 'static>(&mut self, component: C) {
        self.components.push(Box::new(component));
    }
    
    /// ウィンドウを表示
    pub fn show(&mut self) {
        self.visible = true;
        // 実際の実装ではウィンドウシステムの呼び出しを行う
        println!("ウィンドウを表示: {}", self.title);
    }
    
    /// ウィンドウを閉じる
    pub fn close(&mut self) {
        self.visible = false;
        // 実際の実装ではウィンドウシステムの呼び出しを行う
        println!("ウィンドウを閉じる: {}", self.title);
    }
    
    /// ウィンドウのタイトルを設定
    pub fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
        // 実際の実装ではウィンドウシステムの呼び出しを行う
        println!("ウィンドウタイトルを変更: {}", self.title);
    }
    
    /// ウィンドウの背景色を設定
    pub fn set_background_color(&mut self, color: Color) {
        self.background_color = color;
    }
    
    /// イベントハンドラを追加
    pub fn add_event_handler(&mut self, event_type: EventType, handler: EventHandler) {
        let handlers = self.event_handlers.entry(event_type).or_insert_with(Vec::new);
        handlers.push(handler);
    }
    
    /// イベントをディスパッチ
    pub fn dispatch_event(&mut self, event: &Event) -> bool {
        // まずコンポーネントにイベントを渡す
        for component in self.components.iter_mut() {
            if component.handle_event(event) {
                return true;
            }
        }
        
        // 次にウィンドウ自身のハンドラを呼び出す
        let handlers = self.event_handlers.get(&event.event_type);
        if let Some(handlers) = handlers {
            for handler in handlers.iter() {
                if handler(event) {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// ウィンドウとそのコンテンツを描画
    pub fn draw(&self) {
        let mut ctx = DrawContext::new();
        
        // 背景を描画
        ctx.draw_rect(self.bounds, self.background_color);
        
        // 各コンポーネントを描画
        for i in 0..self.components.len() {
            if let Some(component) = self.components.get(i) {
                component.draw(&mut ctx);
            }
        }
    }
}

/// ボタン
pub struct Button {
    text: String,
    bounds: Rect,
    background_color: Color,
    text_color: Color,
    event_handlers: HashMap<EventType, Vec<EventHandler>>,
    enabled: bool,
}

impl Button {
    /// 新しいボタンを作成
    pub fn new(text: &str, x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            text: text.to_string(),
            bounds: Rect::new(x, y, width, height),
            background_color: Color::new(0.9, 0.9, 0.9, 1.0),
            text_color: Color::black(),
            event_handlers: HashMap::new(),
            enabled: true,
        }
    }
    
    /// ボタンのテキストを設定
    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
    }
    
    /// ボタンの背景色を設定
    pub fn set_background_color(&mut self, color: Color) {
        self.background_color = color;
    }
    
    /// ボタンのテキスト色を設定
    pub fn set_text_color(&mut self, color: Color) {
        self.text_color = color;
    }
    
    /// ボタンの有効/無効を設定
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

impl Component for Button {
    fn draw(&self, ctx: &mut DrawContext) {
        // ボタンの背景を描画
        ctx.draw_rect(self.bounds, self.background_color);
        
        // ボタンのテキストを描画（中央揃え）
        let text_x = self.bounds.x + self.bounds.width / 2.0;
        let text_y = self.bounds.y + self.bounds.height / 2.0;
        ctx.draw_text(&self.text, text_x, text_y, self.text_color, 12.0);
    }
    
    fn get_bounds(&self) -> Rect {
        self.bounds
    }
    
    fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
    }
    
    fn handle_event(&mut self, event: &Event) -> bool {
        if !self.enabled {
            return false;
        }
        
        match event.event_type {
            EventType::MouseDown => {
                if self.bounds.contains(event.x, event.y) {
                    let handlers = self.event_handlers.get(&EventType::MouseDown);
                    if let Some(handlers) = handlers {
                        for handler in handlers.iter() {
                            if handler(event) {
                                return true;
                            }
                        }
                    }
                }
            },
            _ => {}
        }
        
        false
    }
    
    fn add_event_handler(&mut self, event_type: EventType, handler: EventHandler) {
        let handlers = self.event_handlers.entry(event_type).or_insert_with(Vec::new);
        handlers.push(handler);
    }
}

/// テキストフィールド
pub struct TextField {
    text: String,
    bounds: Rect,
    background_color: Color,
    text_color: Color,
    event_handlers: HashMap<EventType, Vec<EventHandler>>,
    enabled: bool,
    focused: bool,
}

impl TextField {
    /// 新しいテキストフィールドを作成
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            text: String::new(),
            bounds: Rect::new(x, y, width, height),
            background_color: Color::white(),
            text_color: Color::black(),
            event_handlers: HashMap::new(),
            enabled: true,
            focused: false,
        }
    }
    
    /// テキストを設定
    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
    }
    
    /// テキストを取得
    pub fn text(&self) -> &str {
        &self.text
    }
    
    /// 背景色を設定
    pub fn set_background_color(&mut self, color: Color) {
        self.background_color = color;
    }
    
    /// テキスト色を設定
    pub fn set_text_color(&mut self, color: Color) {
        self.text_color = color;
    }
    
    /// 有効/無効を設定
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

impl Component for TextField {
    fn draw(&self, ctx: &mut DrawContext) {
        // テキストフィールドの背景を描画
        ctx.draw_rect(self.bounds, self.background_color);
        
        // テキストを描画
        ctx.draw_text(&self.text, self.bounds.x + 5.0, self.bounds.y + self.bounds.height / 2.0, self.text_color, 12.0);
        
        // フォーカスされている場合は枠線を描画
        if self.focused {
            let border_color = Color::rgb(0.0, 0.5, 1.0);
            let x = self.bounds.x;
            let y = self.bounds.y;
            let w = self.bounds.width;
            let h = self.bounds.height;
            
            // 上辺
            ctx.draw_line(x, y, x + w, y, border_color, 1.0);
            // 右辺
            ctx.draw_line(x + w, y, x + w, y + h, border_color, 1.0);
            // 下辺
            ctx.draw_line(x, y + h, x + w, y + h, border_color, 1.0);
            // 左辺
            ctx.draw_line(x, y, x, y + h, border_color, 1.0);
        }
    }
    
    fn get_bounds(&self) -> Rect {
        self.bounds
    }
    
    fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
    }
    
    fn handle_event(&mut self, event: &Event) -> bool {
        if !self.enabled {
            return false;
        }
        
        match event.event_type {
            EventType::MouseDown => {
                let was_focused = self.focused;
                self.focused = self.bounds.contains(event.x, event.y);
                
                if self.focused != was_focused {
                    // フォーカス状態が変化した
                    return true;
                }
            },
            EventType::KeyDown => {
                if self.focused {
                    if let Some(key_code) = event.key_code {
                        // 実際の実装ではキーコードに応じた処理を行う
                        // 例えば、バックスペースやエンターキーなどの特殊キーの処理
                        println!("キー入力: {}", key_code);
                        
                        // ここでは単純化のため、キーコードを文字として追加
                        if key_code >= 32 && key_code <= 126 {
                            self.text.push(char::from_u32(key_code).unwrap_or('?'));
                            return true;
                        }
                    }
                }
            },
            _ => {}
        }
        
        let handlers = self.event_handlers.get(&event.event_type);
        if let Some(handlers) = handlers {
            for handler in handlers.iter() {
                if handler(event) {
                    return true;
                }
            }
        }
        
        false
    }
    
    fn add_event_handler(&mut self, event_type: EventType, handler: EventHandler) {
        let handlers = self.event_handlers.entry(event_type).or_insert_with(Vec::new);
        handlers.push(handler);
    }
}

/// ラベル
pub struct Label {
    text: String,
    bounds: Rect,
    text_color: Color,
    event_handlers: HashMap<EventType, Vec<EventHandler>>,
}

impl Label {
    /// 新しいラベルを作成
    pub fn new(text: &str, x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            text: text.to_string(),
            bounds: Rect::new(x, y, width, height),
            text_color: Color::black(),
            event_handlers: HashMap::new(),
        }
    }
    
    /// テキストを設定
    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
    }
    
    /// テキスト色を設定
    pub fn set_text_color(&mut self, color: Color) {
        self.text_color = color;
    }
}

impl Component for Label {
    fn draw(&self, ctx: &mut DrawContext) {
        // テキストを描画
        ctx.draw_text(&self.text, self.bounds.x, self.bounds.y + self.bounds.height / 2.0, self.text_color, 12.0);
    }
    
    fn get_bounds(&self) -> Rect {
        self.bounds
    }
    
    fn set_bounds(&mut self, bounds: Rect) {
        self.bounds = bounds;
    }
    
    fn handle_event(&mut self, event: &Event) -> bool {
        let handlers = self.event_handlers.get(&event.event_type);
        if let Some(handlers) = handlers {
            for handler in handlers.iter() {
                if handler(event) {
                    return true;
                }
            }
        }
        
        false
    }
    
    fn add_event_handler(&mut self, event_type: EventType, handler: EventHandler) {
        let handlers = self.event_handlers.entry(event_type).or_insert_with(Vec::new);
        handlers.push(handler);
    }
}

/// レイアウト
pub struct Layout {
    components: Vec<Box<dyn Component>>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    background_color: Option<Color>,
    border_color: Option<Color>,
}

impl Layout {
    /// 新しいレイアウトを作成
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            components: Vec::new(),
            x,
            y,
            width,
            height,
            background_color: None,
            border_color: None,
        }
    }
    
    /// コンポーネントを追加
    pub fn add_component<C: Component + 'static>(&mut self, component: C) {
        self.components.push(Box::new(component));
    }
    
    /// 背景色を設定
    pub fn set_background_color(&mut self, color: Color) {
        self.background_color = Some(color);
    }
    
    /// 境界線の色を設定
    pub fn set_border_color(&mut self, color: Color) {
        self.border_color = Some(color);
    }
} 
//! # SwiftLight言語のGUIモジュール
//! 
//! このモジュールはグラフィカルユーザーインターフェース（GUI）の作成と
//! 管理のための機能を提供します。
//! 
//! ウィンドウ、ボタン、テキストフィールドなどの基本的なUIコンポーネントと
//! レイアウト管理が含まれています。

use crate::core::types::{Error, ErrorKind, Result};
use crate::core::collections::{Vec, HashMap};
use std::sync::{Arc, Mutex};
use lazy_static::lazy_static;

lazy_static! {
    static ref WINDOW_SYSTEM: Mutex<WindowSystem> = Mutex::new(WindowSystem::new());
}

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
    
    /// 定義済みの色: グレー
    pub fn gray() -> Self {
        Self::rgb(0.5, 0.5, 0.5)
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
    /// キャンバス
    pub(crate) canvas: Canvas,
    /// フォントキャッシュ
    pub(crate) font_cache: HashMap<String, FontHandle>,
    /// デフォルトフォント
    pub(crate) default_font: FontHandle,
}

/// 描画キャンバス
pub struct Canvas {
    /// 幅
    width: f32,
    /// 高さ
    height: f32,
    /// ピクセルバッファ (RGBA形式)
    buffer: Vec<u32>,
    /// 再描画が必要な領域
    dirty_regions: Vec<Rect>,
}

/// フォントハンドル
#[derive(Debug, Clone)]
pub struct FontHandle {
    /// フォントID
    id: usize,
    /// フォント名
    name: String,
    /// フォントサイズ
    size: f32,
}

impl DrawContext {
    /// 新しい描画コンテキストを作成
    pub(crate) fn new(width: f32, height: f32) -> Self {
        let canvas = Canvas {
            width,
            height,
            buffer: std::vec::Vec::with_capacity((width as usize) * (height as usize)).into(),
            dirty_regions: vec![],
        };
        
        let default_font = FontHandle {
            id: 0,
            name: "Default".to_string(),
            size: 12.0,
        };
        
        let mut font_cache = HashMap::new();
        font_cache.insert("Default".to_string(), default_font.clone());
        
        Self { 
            canvas,
            font_cache,
            default_font,
        }
    }
    
    /// 矩形を描画
    pub fn draw_rect(&mut self, rect: Rect, color: Color) {
        // 矩形の境界をクリップ
        let x_start = rect.x.max(0.0) as usize;
        let y_start = rect.y.max(0.0) as usize;
        let x_end = (rect.x + rect.width).min(self.canvas.width) as usize;
        let y_end = (rect.y + rect.height).min(self.canvas.height) as usize;
        
        // RGBAカラーに変換
        let rgba = self.color_to_rgba(color);
        
        // バッファにピクセルを書き込み
        for y in y_start..y_end {
            for x in x_start..x_end {
                let idx = y * self.canvas.width as usize + x;
                if idx < self.canvas.buffer.len() {
                    self.canvas.buffer[idx] = rgba;
                }
            }
        }
        
        // 変更領域を記録
        self.canvas.dirty_regions.push(rect);
    }
    
    /// 線を描画
    pub fn draw_line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, color: Color, width: f32) {
        // Bresenhamのアルゴリズムを使用して線を描画
        let rgba = self.color_to_rgba(color);
        let half_width = (width / 2.0).ceil() as i32;
        
        // 線の始点と終点
        let (x1, y1) = (x1 as i32, y1 as i32);
        let (x2, y2) = (x2 as i32, y2 as i32);
        
        // X方向とY方向の差
        let dx = (x2 - x1).abs();
        let dy = (y2 - y1).abs();
        
        // 増分方向
        let sx = if x1 < x2 { 1 } else { -1 };
        let sy = if y1 < y2 { 1 } else { -1 };
        
        // エラー累積初期値
        let mut err = dx - dy;
        let mut x = x1;
        let mut y = y1;
        
        let canvas_width = self.canvas.width as i32;
        let canvas_height = self.canvas.height as i32;
        
        // 線を太くするために周囲のピクセルも描画
        while x != x2 || y != y2 {
            // 現在のピクセルとその周囲を描画
            for wy in -half_width..=half_width {
                for wx in -half_width..=half_width {
                    let px = x + wx;
                    let py = y + wy;
                    
                    // キャンバス境界内かチェック
                    if px >= 0 && px < canvas_width && py >= 0 && py < canvas_height {
                        let idx = (py as usize) * self.canvas.width as usize + (px as usize);
                        if idx < self.canvas.buffer.len() {
                            self.canvas.buffer[idx] = rgba;
                        }
                    }
                }
            }
            
            // Bresenhamアルゴリズムによる次のピクセル計算
            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
        
        // 変更領域を記録
        let line_rect = Rect::new(
            f32::min(x1 as f32, x2 as f32) - width,
            f32::min(y1 as f32, y2 as f32) - width,
            f32::abs(x2 as f32 - x1 as f32) + width * 2.0,
            f32::abs(y2 as f32 - y1 as f32) + width * 2.0
        );
        self.canvas.dirty_regions.push(line_rect);
    }
    
    /// テキストを描画
    pub fn draw_text(&mut self, text: &str, x: f32, y: f32, color: Color, size: f32) {
        // フォントサイズに合わせたフォントを取得またはキャッシュから読み込み
        let font_key = format!("Default_{}", size);
        let font = if let Some(font) = self.font_cache.get(&font_key) {
            font.clone()
        } else {
            let font = FontHandle {
                id: self.font_cache.len(),
                name: "Default".to_string(),
                size,
            };
            self.font_cache.insert(font_key, font.clone());
            font
        };
        
        // テキストの描画サイズを計算
        let text_width = text.len() as f32 * size * 0.6;
        let text_height = size;
        
        // テキスト背景用のバッファを準備
        let rgba = self.color_to_rgba(color);
        
        // フォントレンダリング（簡略化版）
        let char_width = size * 0.6;
        let mut cursor_x = x;
        
        for c in text.chars() {
            // 各文字を描画（簡略化）
            self.draw_char(c, cursor_x, y, color, size, &font);
            cursor_x += char_width;
        }
        
        // 変更領域を記録
        let text_rect = Rect::new(x, y - text_height / 2.0, text_width, text_height);
        self.canvas.dirty_regions.push(text_rect);
    }
    
    /// 文字を描画
    fn draw_char(&mut self, c: char, x: f32, y: f32, color: Color, size: f32, font: &FontHandle) {
        // フォントレンダリングの簡略化版
        // 実際のアプリケーションではフォントレンダリングライブラリを使用
        
        let rgba = self.color_to_rgba(color);
        let char_width = size * 0.6;
        let char_height = size;
        
        // 単純なビットマップ文字を描画
        let glyph = self.get_simple_glyph(c);
        let scale_x = char_width / 8.0;
        let scale_y = char_height / 8.0;
        
        for row in 0..8 {
            for col in 0..8 {
                if glyph[row][col] {
                    let px = x + col as f32 * scale_x;
                    let py = y - char_height / 2.0 + row as f32 * scale_y;
                    
                    let px_start = px as usize;
                    let py_start = py as usize;
                    let px_end = (px + scale_x).ceil() as usize;
                    let py_end = (py + scale_y).ceil() as usize;
                    
                    for draw_y in py_start..py_end {
                        for draw_x in px_start..px_end {
                            if draw_x < self.canvas.width as usize && draw_y < self.canvas.height as usize {
                                let idx = draw_y * self.canvas.width as usize + draw_x;
                                if idx < self.canvas.buffer.len() {
                                    self.canvas.buffer[idx] = rgba;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// 文字のビットマップを取得（簡略化）
    fn get_simple_glyph(&self, c: char) -> [[bool; 8]; 8] {
        // 簡単なビットマップフォント（アスキー文字のみ）
        // 実際のアプリケーションではフォントレンダリングライブラリを使用
        match c {
            'A' => [
                [false, false, true, true, true, true, false, false],
                [false, true, false, false, false, false, true, false],
                [true, false, false, false, false, false, false, true],
                [true, false, false, false, false, false, false, true],
                [true, true, true, true, true, true, true, true],
                [true, false, false, false, false, false, false, true],
                [true, false, false, false, false, false, false, true],
                [true, false, false, false, false, false, false, true],
            ],
            // 他の文字も同様に定義（省略）
            _ => [
                [false, false, false, false, false, false, false, false],
                [false, false, true, true, true, true, false, false],
                [false, true, false, false, false, false, true, false],
                [true, false, false, false, false, false, false, true],
                [true, false, false, false, false, false, false, true],
                [true, false, false, false, false, false, false, true],
                [false, true, false, false, false, false, true, false],
                [false, false, true, true, true, true, false, false],
            ],
        }
    }
    
    /// Colorを32ビットRGBA値に変換
    fn color_to_rgba(&self, color: Color) -> u32 {
        let r = (color.r * 255.0) as u8;
        let g = (color.g * 255.0) as u8;
        let b = (color.b * 255.0) as u8;
        let a = (color.a * 255.0) as u8;
        
        ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    }
    
    /// 描画バッファをフラッシュ
    pub(crate) fn flush(&mut self) -> &[Rect] {
        &self.canvas.dirty_regions
    }
    
    /// 変更領域をクリア
    pub(crate) fn clear_dirty_regions(&mut self) {
        self.canvas.dirty_regions.clear();
    }
    
    /// 円を描画
    pub fn draw_circle(&mut self, center_x: f32, center_y: f32, radius: f32, color: Color, stroke_width: f32) {
        // 円の境界をクリップ
        let x_start = (center_x - radius).max(0.0) as usize;
        let y_start = (center_y - radius).max(0.0) as usize;
        let x_end = (center_x + radius).min(self.canvas.width) as usize;
        let y_end = (center_y + radius).min(self.canvas.height) as usize;
        
        // RGBAカラーに変換
        let rgba = self.color_to_rgba(color);
        
        // 円を描画
        for y in y_start..y_end {
            for x in x_start..x_end {
                // 現在のピクセルが円の内部にあるかチェック
                let dx = x as f32 - center_x;
                let dy = y as f32 - center_y;
                let distance = (dx * dx + dy * dy).sqrt();
                
                if distance <= radius {
                    // ストローク幅を考慮
                    if stroke_width > 0.0 {
                        if distance >= radius - stroke_width {
                            let idx = y * self.canvas.width as usize + x;
                            if idx < self.canvas.buffer.len() {
                                self.canvas.buffer[idx] = rgba;
                            }
                        }
                    } else {
                        let idx = y * self.canvas.width as usize + x;
                        if idx < self.canvas.buffer.len() {
                            self.canvas.buffer[idx] = rgba;
                        }
                    }
                }
            }
        }
        
        // 変更領域を記録
        let circle_rect = Rect::new(
            center_x - radius,
            center_y - radius,
            radius * 2.0,
            radius * 2.0
        );
        self.canvas.dirty_regions.push(circle_rect);
    }
}

/// ウィンドウシステムインターフェース
pub(crate) struct WindowSystem {
    windows: HashMap<usize, WindowHandle>,
    next_window_id: usize,
}

/// ウィンドウハンドル
struct WindowHandle {
    id: usize,
    title: String,
    bounds: Rect,
    visible: bool,
    context: DrawContext,
}

impl WindowSystem {
    /// 新しいウィンドウシステムを作成
    pub fn new() -> Self {
        Self {
            windows: HashMap::new(),
            next_window_id: 1,
        }
    }
    
    /// ウィンドウを作成
    pub fn create_window(&mut self, title: &str, width: f32, height: f32) -> usize {
        let id = self.next_window_id;
        self.next_window_id += 1;
        
        let window = WindowHandle {
            id,
            title: title.to_string(),
            bounds: Rect::new(0.0, 0.0, width, height),
            visible: false,
            context: DrawContext::new(width, height),
        };
        
        self.windows.insert(id, window);
        id
    }
    
    /// ウィンドウを表示
    pub fn show_window(&mut self, id: usize) -> bool {
        if let Some(window) = self.windows.get_mut(&id) {
            window.visible = true;
            // プラットフォーム固有のウィンドウシステム呼び出し
            println!("ウィンドウを表示: {}", window.title);
            true
        } else {
            false
        }
    }
    
    /// ウィンドウを閉じる
    pub fn close_window(&mut self, id: usize) -> bool {
        if let Some(window) = self.windows.get_mut(&id) {
            window.visible = false;
            // プラットフォーム固有のウィンドウシステム呼び出し
            println!("ウィンドウを閉じる: {}", window.title);
            true
        } else {
            false
        }
    }
    
    /// ウィンドウのタイトルを設定
    pub fn set_window_title(&mut self, id: usize, title: &str) -> bool {
        if let Some(window) = self.windows.get_mut(&id) {
            window.title = title.to_string();
            // プラットフォーム固有のウィンドウシステム呼び出し
            println!("ウィンドウタイトルを変更: {}", window.title);
            true
        } else {
            false
        }
    }
    
    /// 描画コンテキストを取得
    pub fn get_draw_context(&mut self, id: usize) -> Option<&mut DrawContext> {
        if let Some(window) = self.windows.get_mut(&id) {
            Some(&mut window.context)
        } else {
            None
        }
    }
    
    /// ウィンドウを更新
    pub fn update_window(&mut self, id: usize) -> bool {
        if let Some(window) = self.windows.get_mut(&id) {
            // 変更領域を取得
            let dirty_regions = window.context.flush();
            
            // プラットフォーム固有のウィンドウ更新呼び出し
            if !dirty_regions.is_empty() {
                println!("ウィンドウを更新: {} (変更領域: {})", window.title, dirty_regions.len());
                window.context.clear_dirty_regions();
            }
            
            true
        } else {
            false
        }
    }
}

// グローバルウィンドウシステムインスタンス
lazy_static! {
    static ref WINDOW_SYSTEM: Mutex<WindowSystem> = Mutex::new(WindowSystem::new());
}

/// ウィンドウ
pub struct Window {
    title: String,
    bounds: Rect,
    background_color: Color,
    components: Vec<Box<dyn Component>>,
    event_handlers: HashMap<EventType, Vec<EventHandler>>,
    visible: bool,
    window_id: usize,
}

impl Window {
    /// 新しいウィンドウを作成
    pub fn new(title: &str, width: f32, height: f32) -> Self {
        let mut ws = WINDOW_SYSTEM.lock().unwrap();
        let window_id = ws.create_window(title, width, height);
        
        Self {
            title: title.to_string(),
            bounds: Rect::new(0.0, 0.0, width, height),
            background_color: Color::white(),
            components: Vec::new(),
            event_handlers: HashMap::new(),
            visible: false,
            window_id,
        }
    }
    
    /// ウィンドウを表示
    pub fn show(&mut self) {
        self.visible = true;
        let mut ws = WINDOW_SYSTEM.lock().unwrap();
        ws.show_window(self.window_id);
        drop(ws);
        self.draw(); // 初回描画
    }
    
    /// ウィンドウを閉じる
    pub fn close(&mut self) {
        self.visible = false;
        let mut ws = WINDOW_SYSTEM.lock().unwrap();
        ws.close_window(self.window_id);
    }
    
    /// ウィンドウのタイトルを設定
    pub fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
        let mut ws = WINDOW_SYSTEM.lock().unwrap();
        ws.set_window_title(self.window_id, title);
    }
    
    /// ウィンドウとそのコンテンツを描画
    pub fn draw(&self) {
        let mut ws = WINDOW_SYSTEM.lock().unwrap();
        if let Some(ctx) = ws.get_draw_context(self.window_id) {
            // 背景を描画
            ctx.draw_rect(self.bounds, self.background_color);
            
            // 各コンポーネントを描画
            for component in &self.components {
                component.draw(ctx);
            }
            
            // ウィンドウを更新
            drop(ctx);
            ws.update_window(self.window_id);
        }
    }
    
    /// コンポーネントを追加
    pub fn add_component<C: Component + 'static>(&mut self, component: C) {
        self.components.push(Box::new(component));
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
    
    /// テーマを適用
    pub fn apply_theme(&mut self, theme: &Theme) {
        for component in &mut self.components {
            if let Some(themed_component) = component.as_any().downcast_ref::<dyn ThemedComponent>() {
                themed_component.apply_theme(theme);
            }
        }
    }

    /// テーマ変更ハンドラを追加
    pub fn add_theme_change_handler<F: Fn(&Theme) + 'static>(&mut self, handler: F) {
        let mut theme_manager = THEME_MANAGER.lock().unwrap();
        theme_manager.add_theme_change_handler(handler);
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

impl ThemedComponent for Button {
    fn apply_theme(&mut self, theme: &Theme) {
        let style = &theme.components().button;
        self.background_color = style.background;
        self.text_color = style.text;
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

impl ThemedComponent for TextField {
    fn apply_theme(&mut self, theme: &Theme) {
        let style = &theme.components().text_field;
        self.background_color = style.background;
        self.text_color = style.text;
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

impl ThemedComponent for Label {
    fn apply_theme(&mut self, theme: &Theme) {
        let style = &theme.components().label;
        self.text_color = style.text;
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
    
    /// レイアウトの領域を取得
    pub fn get_bounds(&self) -> Rect {
        Rect::new(self.x, self.y, self.width, self.height)
    }
    
    /// レイアウトの領域を設定
    pub fn set_bounds(&mut self, bounds: Rect) {
        self.x = bounds.x;
        self.y = bounds.y;
        self.width = bounds.width;
        self.height = bounds.height;
    }
}

impl Component for Layout {
    fn draw(&self, ctx: &mut DrawContext) {
        // 背景を描画
        if let Some(color) = self.background_color {
            ctx.draw_rect(self.get_bounds(), color);
        }
        
        // 境界線を描画
        if let Some(color) = self.border_color {
            let bounds = self.get_bounds();
            ctx.draw_line(bounds.x, bounds.y, bounds.x + bounds.width, bounds.y, color, 1.0);
            ctx.draw_line(bounds.x + bounds.width, bounds.y, bounds.x + bounds.width, bounds.y + bounds.height, color, 1.0);
            ctx.draw_line(bounds.x + bounds.width, bounds.y + bounds.height, bounds.x, bounds.y + bounds.height, color, 1.0);
            ctx.draw_line(bounds.x, bounds.y + bounds.height, bounds.x, bounds.y, color, 1.0);
        }
        
        // 子コンポーネントを描画
        for component in &self.components {
            component.draw(ctx);
        }
    }
    
    fn get_bounds(&self) -> Rect {
        Rect::new(self.x, self.y, self.width, self.height)
    }
    
    fn set_bounds(&mut self, bounds: Rect) {
        self.x = bounds.x;
        self.y = bounds.y;
        self.width = bounds.width;
        self.height = bounds.height;
    }
    
    fn handle_event(&mut self, event: &Event) -> bool {
        // イベントがレイアウトの範囲内かチェック
        if !self.get_bounds().contains(event.x, event.y) {
            return false;
        }
        
        // 子コンポーネントにイベントを伝播
        for component in &mut self.components {
            if component.handle_event(event) {
                return true;
            }
        }
        
        false
    }
    
    fn add_event_handler(&mut self, event_type: EventType, handler: EventHandler) {
        // レイアウト自体のイベントハンドラは実装しない
        // 子コンポーネントのイベントハンドリングに依存
    }
}

/// チェックボックスコンポーネント
pub struct CheckBox {
    text: String,
    bounds: Rect,
    checked: bool,
    background_color: Color,
    text_color: Color,
    event_handlers: HashMap<EventType, Vec<EventHandler>>,
    enabled: bool,
}

impl CheckBox {
    pub fn new(text: &str, x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            text: text.to_string(),
            bounds: Rect::new(x, y, width, height),
            checked: false,
            background_color: Color::white(),
            text_color: Color::black(),
            event_handlers: HashMap::new(),
            enabled: true,
        }
    }
    
    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
    }
    
    pub fn set_checked(&mut self, checked: bool) {
        self.checked = checked;
    }
    
    pub fn is_checked(&self) -> bool {
        self.checked
    }
    
    pub fn set_background_color(&mut self, color: Color) {
        self.background_color = color;
    }
    
    pub fn set_text_color(&mut self, color: Color) {
        self.text_color = color;
    }
    
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

impl Component for CheckBox {
    fn draw(&self, ctx: &mut DrawContext) {
        // チェックボックスの外枠を描画
        let box_size = 20.0;
        let box_rect = Rect::new(
            self.bounds.x,
            self.bounds.y,
            box_size,
            box_size
        );
        ctx.draw_rect(box_rect, self.background_color);
        
        // チェックマークを描画
        if self.checked {
            let check_color = if self.enabled {
                Color::from_hex("#4285f4").unwrap()
            } else {
                Color::gray()
            };
            
            // チェックマークの線を描画
            ctx.draw_line(
                box_rect.x + 5.0,
                box_rect.y + 10.0,
                box_rect.x + 9.0,
                box_rect.y + 15.0,
                check_color,
                2.0
            );
            ctx.draw_line(
                box_rect.x + 9.0,
                box_rect.y + 15.0,
                box_rect.x + 15.0,
                box_rect.y + 7.0,
                check_color,
                2.0
            );
        }
        
        // テキストを描画
        let text_x = box_rect.x + box_size + 5.0;
        let text_y = box_rect.y + box_size / 2.0;
        ctx.draw_text(
            &self.text,
            text_x,
            text_y,
            if self.enabled { self.text_color } else { Color::gray() },
            16.0
        );
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
                    self.checked = !self.checked;
                    
                    // イベントハンドラを呼び出し
                    if let Some(handlers) = self.event_handlers.get(&EventType::MouseDown) {
                        for handler in handlers {
                            handler(event);
                        }
                    }
                    return true;
                }
                false
            },
            _ => false,
        }
    }
    
    fn add_event_handler(&mut self, event_type: EventType, handler: EventHandler) {
        self.event_handlers.entry(event_type)
            .or_insert_with(Vec::new)
            .push(handler);
    }
}

impl ThemedComponent for CheckBox {
    fn apply_theme(&mut self, theme: &Theme) {
        let style = &theme.components().checkbox;
        self.background_color = style.background;
        self.text_color = style.text;
    }
}

/// ラジオボタンコンポーネント
pub struct RadioButton {
    text: String,
    bounds: Rect,
    checked: bool,
    group: String,
    background_color: Color,
    text_color: Color,
    event_handlers: HashMap<EventType, Vec<EventHandler>>,
    enabled: bool,
}

impl RadioButton {
    pub fn new(text: &str, group: &str, x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            text: text.to_string(),
            bounds: Rect::new(x, y, width, height),
            checked: false,
            group: group.to_string(),
            background_color: Color::white(),
            text_color: Color::black(),
            event_handlers: HashMap::new(),
            enabled: true,
        }
    }
    
    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_string();
    }
    
    pub fn set_checked(&mut self, checked: bool) {
        self.checked = checked;
    }
    
    pub fn is_checked(&self) -> bool {
        self.checked
    }
    
    pub fn get_group(&self) -> &str {
        &self.group
    }
    
    pub fn set_background_color(&mut self, color: Color) {
        self.background_color = color;
    }
    
    pub fn set_text_color(&mut self, color: Color) {
        self.text_color = color;
    }
    
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

impl Component for RadioButton {
    fn draw(&self, ctx: &mut DrawContext) {
        // ラジオボタンの外枠を描画
        let button_size = 20.0;
        let button_rect = Rect::new(
            self.bounds.x,
            self.bounds.y,
            button_size,
            button_size
        );
        
        // 円を描画
        ctx.draw_circle(
            button_rect.x + button_size / 2.0,
            button_rect.y + button_size / 2.0,
            button_size / 2.0,
            self.background_color,
            2.0
        );
        
        // 選択状態の円を描画
        if self.checked {
            let check_color = if self.enabled {
                Color::from_hex("#4285f4").unwrap()
            } else {
                Color::gray()
            };
            
            ctx.draw_circle(
                button_rect.x + button_size / 2.0,
                button_rect.y + button_size / 2.0,
                button_size / 4.0,
                check_color,
                0.0
            );
        }
        
        // テキストを描画
        let text_x = button_rect.x + button_size + 5.0;
        let text_y = button_rect.y + button_size / 2.0;
        ctx.draw_text(
            &self.text,
            text_x,
            text_y,
            if self.enabled { self.text_color } else { Color::gray() },
            16.0
        );
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
                    self.checked = true;
                    
                    // イベントハンドラを呼び出し
                    if let Some(handlers) = self.event_handlers.get(&EventType::MouseDown) {
                        for handler in handlers {
                            handler(event);
                        }
                    }
                    return true;
                }
                false
            },
            _ => false,
        }
    }
    
    fn add_event_handler(&mut self, event_type: EventType, handler: EventHandler) {
        self.event_handlers.entry(event_type)
            .or_insert_with(Vec::new)
            .push(handler);
    }
}

impl ThemedComponent for RadioButton {
    fn apply_theme(&mut self, theme: &Theme) {
        let style = &theme.components().radio_button;
        self.background_color = style.background;
        self.text_color = style.text;
    }
}

/// フレックスボックスの方向
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FlexDirection {
    /// 水平方向（左から右）
    Row,
    /// 垂直方向（上から下）
    Column,
    /// 水平方向（右から左）
    RowReverse,
    /// 垂直方向（下から上）
    ColumnReverse,
}

/// フレックスボックスの配置
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FlexAlign {
    /// 開始位置に配置
    Start,
    /// 中央に配置
    Center,
    /// 終了位置に配置
    End,
    /// 均等に配置
    SpaceBetween,
    /// 均等に配置（両端に余白なし）
    SpaceAround,
    /// 均等に配置（両端に余白あり）
    SpaceEvenly,
}

/// フレックスボックスレイアウトを表す構造体
pub struct FlexLayout {
    components: Vec<Box<dyn Component>>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    direction: FlexDirection,
    align: FlexAlign,
    spacing: f32,
    background_color: Option<Color>,
    border_color: Option<Color>,
}

impl FlexLayout {
    /// 新しいフレックスボックスレイアウトを作成
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            components: Vec::new(),
            x,
            y,
            width,
            height,
            direction: FlexDirection::Row,
            align: FlexAlign::Start,
            spacing: 0.0,
            background_color: None,
            border_color: None,
        }
    }

    /// コンポーネントを追加
    pub fn add_component<C: Component + 'static>(&mut self, component: C) {
        self.components.push(Box::new(component));
        self.update_layout();
    }

    /// 方向を設定
    pub fn set_direction(&mut self, direction: FlexDirection) {
        self.direction = direction;
        self.update_layout();
    }

    /// 配置を設定
    pub fn set_align(&mut self, align: FlexAlign) {
        self.align = align;
        self.update_layout();
    }

    /// 間隔を設定
    pub fn set_spacing(&mut self, spacing: f32) {
        self.spacing = spacing;
        self.update_layout();
    }

    /// 背景色を設定
    pub fn set_background_color(&mut self, color: Color) {
        self.background_color = Some(color);
    }

    /// 境界線の色を設定
    pub fn set_border_color(&mut self, color: Color) {
        self.border_color = Some(color);
    }

    /// レイアウトの領域を取得
    pub fn get_bounds(&self) -> Rect {
        Rect::new(self.x, self.y, self.width, self.height)
    }

    /// レイアウトの領域を設定
    pub fn set_bounds(&mut self, bounds: Rect) {
        self.x = bounds.x;
        self.y = bounds.y;
        self.width = bounds.width;
        self.height = bounds.height;
        self.update_layout();
    }

    /// レイアウトを更新
    fn update_layout(&mut self) {
        if self.components.is_empty() {
            return;
        }

        let total_spacing = self.spacing * (self.components.len() - 1) as f32;
        let available_space = match self.direction {
            FlexDirection::Row | FlexDirection::RowReverse => self.width,
            FlexDirection::Column | FlexDirection::ColumnReverse => self.height,
        };

        let total_size = available_space - total_spacing;
        let component_size = total_size / self.components.len() as f32;

        let mut current_pos = match self.direction {
            FlexDirection::Row => self.x,
            FlexDirection::RowReverse => self.x + self.width - component_size,
            FlexDirection::Column => self.y,
            FlexDirection::ColumnReverse => self.y + self.height - component_size,
        };

        for component in &mut self.components {
            let bounds = match self.direction {
                FlexDirection::Row => Rect::new(
                    current_pos,
                    self.y,
                    component_size,
                    self.height,
                ),
                FlexDirection::RowReverse => Rect::new(
                    current_pos,
                    self.y,
                    component_size,
                    self.height,
                ),
                FlexDirection::Column => Rect::new(
                    self.x,
                    current_pos,
                    self.width,
                    component_size,
                ),
                FlexDirection::ColumnReverse => Rect::new(
                    self.x,
                    current_pos,
                    self.width,
                    component_size,
                ),
            };
            component.set_bounds(bounds);

            current_pos += match self.direction {
                FlexDirection::Row => component_size + self.spacing,
                FlexDirection::RowReverse => -(component_size + self.spacing),
                FlexDirection::Column => component_size + self.spacing,
                FlexDirection::ColumnReverse => -(component_size + self.spacing),
            };
        }
    }
}

impl Component for FlexLayout {
    fn draw(&self, ctx: &mut DrawContext) {
        // 背景を描画
        if let Some(color) = self.background_color {
            ctx.draw_rect(self.get_bounds(), color);
        }

        // 境界線を描画
        if let Some(color) = self.border_color {
            let bounds = self.get_bounds();
            ctx.draw_line(bounds.x, bounds.y, bounds.x + bounds.width, bounds.y, color, 1.0);
            ctx.draw_line(bounds.x + bounds.width, bounds.y, bounds.x + bounds.width, bounds.y + bounds.height, color, 1.0);
            ctx.draw_line(bounds.x + bounds.width, bounds.y + bounds.height, bounds.x, bounds.y + bounds.height, color, 1.0);
            ctx.draw_line(bounds.x, bounds.y + bounds.height, bounds.x, bounds.y, color, 1.0);
        }

        // 子コンポーネントを描画
        for component in &self.components {
            component.draw(ctx);
        }
    }

    fn get_bounds(&self) -> Rect {
        Rect::new(self.x, self.y, self.width, self.height)
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.set_bounds(bounds);
    }

    fn handle_event(&mut self, event: &Event) -> bool {
        // イベントがレイアウトの領域内にあるか確認
        if !self.get_bounds().contains(event.x, event.y) {
            return false;
        }

        // 子コンポーネントにイベントを伝播
        for component in &mut self.components {
            if component.handle_event(event) {
                return true;
            }
        }

        false
    }

    fn add_event_handler(&mut self, event_type: EventType, handler: EventHandler) {
        // フレックスボックスレイアウト自体のイベントハンドラは実装しない
        // 子コンポーネントのイベントハンドリングに依存
    }
}

/// アンカーレイアウトを表す構造体
pub struct AnchorLayout {
    components: Vec<(Box<dyn Component>, Anchor)>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    background_color: Option<Color>,
    border_color: Option<Color>,
}

/// アンカーの位置を表す構造体
#[derive(Debug, Clone)]
pub struct Anchor {
    /// 左端からの距離（Noneの場合は左端に固定）
    pub left: Option<f32>,
    /// 右端からの距離（Noneの場合は右端に固定）
    pub right: Option<f32>,
    /// 上端からの距離（Noneの場合は上端に固定）
    pub top: Option<f32>,
    /// 下端からの距離（Noneの場合は下端に固定）
    pub bottom: Option<f32>,
    /// 幅（Noneの場合は左右のアンカーから計算）
    pub width: Option<f32>,
    /// 高さ（Noneの場合は上下のアンカーから計算）
    pub height: Option<f32>,
}

impl Anchor {
    /// 新しいアンカーを作成
    pub fn new() -> Self {
        Self {
            left: None,
            right: None,
            top: None,
            bottom: None,
            width: None,
            height: None,
        }
    }

    /// 左端からの距離を設定
    pub fn with_left(mut self, left: f32) -> Self {
        self.left = Some(left);
        self
    }

    /// 右端からの距離を設定
    pub fn with_right(mut self, right: f32) -> Self {
        self.right = Some(right);
        self
    }

    /// 上端からの距離を設定
    pub fn with_top(mut self, top: f32) -> Self {
        self.top = Some(top);
        self
    }

    /// 下端からの距離を設定
    pub fn with_bottom(mut self, bottom: f32) -> Self {
        self.bottom = Some(bottom);
        self
    }

    /// 幅を設定
    pub fn with_width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// 高さを設定
    pub fn with_height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }
}

impl AnchorLayout {
    /// 新しいアンカーレイアウトを作成
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
    pub fn add_component<C: Component + 'static>(&mut self, component: C, anchor: Anchor) {
        self.components.push((Box::new(component), anchor));
        self.update_layout();
    }

    /// 背景色を設定
    pub fn set_background_color(&mut self, color: Color) {
        self.background_color = Some(color);
    }

    /// 境界線の色を設定
    pub fn set_border_color(&mut self, color: Color) {
        self.border_color = Some(color);
    }

    /// レイアウトの領域を取得
    pub fn get_bounds(&self) -> Rect {
        Rect::new(self.x, self.y, self.width, self.height)
    }

    /// レイアウトの領域を設定
    pub fn set_bounds(&mut self, bounds: Rect) {
        self.x = bounds.x;
        self.y = bounds.y;
        self.width = bounds.width;
        self.height = bounds.height;
        self.update_layout();
    }

    /// レイアウトを更新
    fn update_layout(&mut self) {
        for (component, anchor) in &mut self.components {
            let mut x = self.x;
            let mut y = self.y;
            let mut width = self.width;
            let mut height = self.height;

            // 水平方向の位置とサイズを計算
            if let Some(left) = anchor.left {
                x = self.x + left;
            }
            if let Some(right) = anchor.right {
                width = self.x + self.width - right - x;
            }
            if let Some(w) = anchor.width {
                width = w;
            }

            // 垂直方向の位置とサイズを計算
            if let Some(top) = anchor.top {
                y = self.y + top;
            }
            if let Some(bottom) = anchor.bottom {
                height = self.y + self.height - bottom - y;
            }
            if let Some(h) = anchor.height {
                height = h;
            }

            component.set_bounds(Rect::new(x, y, width, height));
        }
    }
}

impl Component for AnchorLayout {
    fn draw(&self, ctx: &mut DrawContext) {
        // 背景を描画
        if let Some(color) = self.background_color {
            ctx.draw_rect(self.get_bounds(), color);
        }

        // 境界線を描画
        if let Some(color) = self.border_color {
            let bounds = self.get_bounds();
            ctx.draw_line(bounds.x, bounds.y, bounds.x + bounds.width, bounds.y, color, 1.0);
            ctx.draw_line(bounds.x + bounds.width, bounds.y, bounds.x + bounds.width, bounds.y + bounds.height, color, 1.0);
            ctx.draw_line(bounds.x + bounds.width, bounds.y + bounds.height, bounds.x, bounds.y + bounds.height, color, 1.0);
            ctx.draw_line(bounds.x, bounds.y + bounds.height, bounds.x, bounds.y, color, 1.0);
        }

        // 子コンポーネントを描画
        for (component, _) in &self.components {
            component.draw(ctx);
        }
    }

    fn get_bounds(&self) -> Rect {
        Rect::new(self.x, self.y, self.width, self.height)
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.set_bounds(bounds);
    }

    fn handle_event(&mut self, event: &Event) -> bool {
        // イベントがレイアウトの領域内にあるか確認
        if !self.get_bounds().contains(event.x, event.y) {
            return false;
        }

        // 子コンポーネントにイベントを伝播
        for (component, _) in &mut self.components {
            if component.handle_event(event) {
                return true;
            }
        }

        false
    }

    fn add_event_handler(&mut self, event_type: EventType, handler: EventHandler) {
        // アンカーレイアウト自体のイベントハンドラは実装しない
        // 子コンポーネントのイベントハンドリングに依存
    }
}

/// テーマを表す構造体
#[derive(Debug, Clone)]
pub struct Theme {
    /// テーマ名
    name: String,
    /// カラーパレット
    colors: ColorPalette,
    /// フォントスタイル
    fonts: FontStyles,
    /// コンポーネントスタイル
    components: ComponentStyles,
}

/// カラーパレットを表す構造体
#[derive(Debug, Clone)]
pub struct ColorPalette {
    /// 背景色
    background: Color,
    /// 前景色
    foreground: Color,
    /// アクセント色
    accent: Color,
    /// エラー色
    error: Color,
    /// 警告色
    warning: Color,
    /// 成功色
    success: Color,
    /// 無効色
    disabled: Color,
    /// 境界線色
    border: Color,
    /// ホバー色
    hover: Color,
    /// 選択色
    selected: Color,
}

/// フォントスタイルを表す構造体
#[derive(Debug, Clone)]
pub struct FontStyles {
    /// デフォルトフォント
    default: FontStyle,
    /// 見出しフォント
    heading: FontStyle,
    /// ボタンフォント
    button: FontStyle,
    /// ラベルフォント
    label: FontStyle,
    /// 入力フォント
    input: FontStyle,
}

/// フォントスタイルを表す構造体
#[derive(Debug, Clone)]
pub struct FontStyle {
    /// フォント名
    name: String,
    /// フォントサイズ
    size: f32,
    /// 太字かどうか
    bold: bool,
    /// イタリックかどうか
    italic: bool,
    /// 下線かどうか
    underline: bool,
}

/// コンポーネントスタイルを表す構造体
#[derive(Debug, Clone)]
pub struct ComponentStyles {
    /// ボタンスタイル
    button: ButtonStyle,
    /// テキストフィールドスタイル
    text_field: TextFieldStyle,
    /// ラベルスタイル
    label: LabelStyle,
    /// チェックボックススタイル
    checkbox: CheckBoxStyle,
    /// ラジオボタンスタイル
    radio_button: RadioButtonStyle,
}

/// ボタンスタイルを表す構造体
#[derive(Debug, Clone)]
pub struct ButtonStyle {
    /// 背景色
    background: Color,
    /// テキスト色
    text: Color,
    /// 境界線色
    border: Color,
    /// 境界線の太さ
    border_width: f32,
    /// 角丸の半径
    corner_radius: f32,
    /// パディング
    padding: f32,
    /// ホバー時の背景色
    hover_background: Color,
    /// ホバー時のテキスト色
    hover_text: Color,
    /// ホバー時の境界線色
    hover_border: Color,
    /// 無効時の背景色
    disabled_background: Color,
    /// 無効時のテキスト色
    disabled_text: Color,
    /// 無効時の境界線色
}

/// テーマを適用できるコンポーネントのトレイト
pub trait ThemedComponent: Component {
    /// テーマを適用
    fn apply_theme(&mut self, theme: &Theme);
}

impl ThemedComponent for Button {
    fn apply_theme(&mut self, theme: &Theme) {
        let style = &theme.components().button;
        self.background_color = style.background;
        self.text_color = style.text;
    }
}

impl ThemedComponent for TextField {
    fn apply_theme(&mut self, theme: &Theme) {
        let style = &theme.components().text_field;
        self.background_color = style.background;
        self.text_color = style.text;
    }
}

impl ThemedComponent for Label {
    fn apply_theme(&mut self, theme: &Theme) {
        let style = &theme.components().label;
        self.text_color = style.text;
    }
}

impl ThemedComponent for CheckBox {
    fn apply_theme(&mut self, theme: &Theme) {
        let style = &theme.components().checkbox;
        self.background_color = style.background;
        self.text_color = style.text;
    }
}

impl ThemedComponent for RadioButton {
    fn apply_theme(&mut self, theme: &Theme) {
        let style = &theme.components().radio_button;
        self.background_color = style.background;
        self.text_color = style.text;
    }
}

/// テーママネージャー
pub struct ThemeManager {
    /// 現在のテーマ
    current_theme: Theme,
    /// テーマの変更を監視するハンドラ
    theme_change_handlers: Vec<Box<dyn Fn(&Theme)>>,
}

impl ThemeManager {
    /// 新しいテーママネージャーを作成
    pub fn new() -> Self {
        Self {
            current_theme: Theme::new("Default"),
            theme_change_handlers: Vec::new(),
        }
    }

    /// 現在のテーマを取得
    pub fn current_theme(&self) -> &Theme {
        &self.current_theme
    }

    /// テーマを設定
    pub fn set_theme(&mut self, theme: Theme) {
        self.current_theme = theme;
        self.notify_theme_change();
    }

    /// テーマ変更ハンドラを追加
    pub fn add_theme_change_handler<F: Fn(&Theme) + 'static>(&mut self, handler: F) {
        self.theme_change_handlers.push(Box::new(handler));
    }

    /// テーマ変更を通知
    fn notify_theme_change(&self) {
        for handler in &self.theme_change_handlers {
            handler(&self.current_theme);
        }
    }
}

// グローバルテーママネージャーインスタンス
lazy_static! {
    static ref THEME_MANAGER: Mutex<ThemeManager> = Mutex::new(ThemeManager::new());
}

/// ウィンドウにテーマを適用
impl Window {
    /// テーマを適用
    pub fn apply_theme(&mut self, theme: &Theme) {
        for component in &mut self.components {
            if let Some(themed_component) = component.as_any().downcast_ref::<dyn ThemedComponent>() {
                themed_component.apply_theme(theme);
            }
        }
    }

    /// テーマ変更ハンドラを追加
    pub fn add_theme_change_handler<F: Fn(&Theme) + 'static>(&mut self, handler: F) {
        let mut theme_manager = THEME_MANAGER.lock().unwrap();
        theme_manager.add_theme_change_handler(handler);
    }
}

// ... existing code ...
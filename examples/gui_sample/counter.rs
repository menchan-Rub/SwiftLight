// SwiftLight GUI カウンターアプリケーション
// イベント駆動型GUIの例

extern crate swiftlight_stdlib;

use std::sync::{Arc, Mutex};
use swiftlight_stdlib::gui::{
    Window, Button, Label, Color, Rect, EventType, Event, Component
};

fn main() {
    // ウィンドウを作成
    let mut window = Window::new("カウンターアプリ", 300.0, 200.0);
    window.set_background_color(Color::from_hex("#f0f0f0").unwrap());
    
    // カウンター値を共有するための変数
    let counter = Arc::new(Mutex::new(0));
    
    // タイトルラベル
    let title_label = Label::new("カウンター", 100.0, 30.0, 100.0, 30.0);
    
    // カウント表示ラベル
    let count_display = Arc::new(Mutex::new(Label::new("0", 140.0, 70.0, 20.0, 30.0)));
    let count_label_clone = Arc::clone(&count_display);
    
    // 増加ボタン
    let mut increase_button = Button::new("+", 90.0, 120.0, 40.0, 40.0);
    increase_button.set_background_color(Color::from_hex("#4CAF50").unwrap());
    increase_button.set_text_color(Color::white());
    
    // 減少ボタン
    let mut decrease_button = Button::new("-", 170.0, 120.0, 40.0, 40.0);
    decrease_button.set_background_color(Color::from_hex("#F44336").unwrap());
    decrease_button.set_text_color(Color::white());
    
    // 増加ボタンのイベントハンドラ
    let counter_inc = Arc::clone(&counter);
    let count_display_inc = Arc::clone(&count_display);
    increase_button.add_event_handler(EventType::MouseDown, Box::new(move |_| {
        let mut count = counter_inc.lock().unwrap();
        *count += 1;
        
        let mut label = count_display_inc.lock().unwrap();
        label.set_text(&count.to_string());
        
        println!("カウンター: {}", *count);
        true
    }));
    
    // 減少ボタンのイベントハンドラ
    let counter_dec = Arc::clone(&counter);
    let count_display_dec = Arc::clone(&count_display);
    decrease_button.add_event_handler(EventType::MouseDown, Box::new(move |_| {
        let mut count = counter_dec.lock().unwrap();
        if *count > 0 {
            *count -= 1;
        }
        
        let mut label = count_display_dec.lock().unwrap();
        label.set_text(&count.to_string());
        
        println!("カウンター: {}", *count);
        true
    }));
    
    // コンポーネントをウィンドウに追加
    window.add_component(title_label);
    window.add_component(*count_label_clone.lock().unwrap());
    window.add_component(increase_button);
    window.add_component(decrease_button);
    
    // ウィンドウを表示
    window.show();
    
    // メインループ
    println!("カウンターアプリが起動しました。Ctrl+Cで終了してください。");
    loop {
        // イベント処理を模擬
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
} 
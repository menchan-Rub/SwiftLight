// SwiftLight GUI サンプルアプリケーション
// シンプルなフォームアプリケーションの例

extern crate swiftlight_stdlib;

use swiftlight_stdlib::gui::{
    Window, Button, TextField, Label, Color, Rect, EventType, Event, Component, Layout
};

fn main() {
    // ウィンドウを作成
    let mut window = Window::new("SwiftLight GUIサンプル", 400.0, 300.0);
    
    // 背景色を設定
    window.set_background_color(Color::from_hex("#f5f5f5").unwrap());
    
    // タイトルラベルを追加
    let title_label = Label::new("ユーザー登録フォーム", 20.0, 20.0, 360.0, 30.0);
    window.add_component(title_label);
    
    // 名前ラベルとテキストフィールド
    let name_label = Label::new("名前:", 20.0, 70.0, 80.0, 25.0);
    let mut name_field = TextField::new(110.0, 70.0, 270.0, 25.0);
    
    // メールアドレスラベルとテキストフィールド
    let email_label = Label::new("メールアドレス:", 20.0, 110.0, 80.0, 25.0);
    let mut email_field = TextField::new(110.0, 110.0, 270.0, 25.0);
    
    // パスワードラベルとテキストフィールド
    let password_label = Label::new("パスワード:", 20.0, 150.0, 80.0, 25.0);
    let mut password_field = TextField::new(110.0, 150.0, 270.0, 25.0);
    
    // 送信ボタン
    let mut submit_button = Button::new("登録", 150.0, 200.0, 100.0, 30.0);
    submit_button.set_background_color(Color::from_hex("#4285f4").unwrap());
    submit_button.set_text_color(Color::white());
    
    // イベントハンドラを設定
    submit_button.add_event_handler(EventType::MouseDown, Box::new(move |event| {
        println!("フォームが送信されました");
        true
    }));
    
    // すべてのコンポーネントをウィンドウに追加
    window.add_component(name_label);
    window.add_component(name_field);
    window.add_component(email_label);
    window.add_component(email_field);
    window.add_component(password_label);
    window.add_component(password_field);
    window.add_component(submit_button);
    
    // ウィンドウを表示
    window.show();
    
    // メインループ（実際の実装ではイベントループを処理）
    println!("アプリケーションが起動しました。Ctrl+Cで終了してください。");
    loop {
        // イベント処理を模擬（実際の実装ではOSからのイベントを処理）
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
} 
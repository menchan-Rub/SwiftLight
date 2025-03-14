//! SwiftLight言語の標準ライブラリ
//! 
//! この標準ライブラリはSwiftLight言語の基本機能を提供します。
//! 複数のモジュールから構成されており、それぞれが特定の機能領域をカバーしています。

/// コアモジュール：言語の基本機能を提供
pub mod core;

/// 標準モジュール：一般的なユーティリティ機能を提供
pub mod std;

/// 数学関連の機能を提供するモジュール
pub mod math;

/// FFI（外部関数インターフェース）を提供するモジュール
pub mod ffi;

/// GUIプログラミング関連の機能を提供するモジュール
pub mod gui;

/// WebAssembly関連の機能を提供するモジュール
pub mod wasm;

/// ライブラリのバージョン情報
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// ライブラリ初期化関数
pub fn initialize() {
    // 標準ライブラリの初期化処理
    // 現時点では特に何もしない
    println!("SwiftLight標準ライブラリを初期化しました。バージョン: {}", VERSION);
}

/// 言語機能のバージョン検証関数
/// 
/// 特定の言語機能が現在の実装で利用可能かどうかを確認する
pub fn has_feature(feature_name: &str) -> bool {
    match feature_name {
        "async" => true,
        "traits" => true,
        "generics" => true,
        "pattern_matching" => true,
        "ownership" => true,
        "borrowing" => true,
        "const_generics" => false,
        "dependent_types" => false,
        _ => false,
    }
}

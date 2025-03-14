// SwiftLight ライブラリの基本テンプレート
// このファイルは `swiftlight new --lib` コマンドによって生成されました

// 標準ライブラリをインポート
import std.io;

/// ライブラリの公開モジュール
pub mod my_library {
    /// 公開API関数の例
    pub fn hello() -> string {
        return "こんにちは、SwiftLightライブラリの世界へ！";
    }
    
    /// 計算用のユーティリティ関数
    pub fn add(a: i32, b: i32) -> i32 {
        return a + b;
    }
    
    /// 文字列処理関数
    pub fn repeat(text: string, count: i32) -> string {
        let result = "";
        for i in 0..count {
            result += text;
        }
        return result;
    }
    
    // 内部ヘルパー関数（非公開）
    fn internal_helper() -> string {
        return "この関数はライブラリ内部でのみアクセス可能です";
    }
} 
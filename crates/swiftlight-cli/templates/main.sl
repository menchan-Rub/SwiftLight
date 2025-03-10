// SwiftLight プログラムの基本テンプレート
// このファイルは `swiftlight new` コマンドによって生成されました

// 標準ライブラリをインポート
import std.io;
import std.system;

// メイン関数（プログラムのエントリーポイント）
fn main() -> i32 {
    // 標準出力にメッセージを表示
    io.println("Hello, SwiftLight!");
    
    // サンプルの変数宣言
    let message = "プログラミングの世界へようこそ！";
    io.println(message);
    
    // 数値を使った例
    let answer = 42;
    io.println("生命、宇宙、そして万物についての究極の疑問の答え: {}", answer);
    
    // 戻り値（0は正常終了を意味する）
    return 0;
}

// 別の関数の例
fn greet(name: string) -> string {
    return "こんにちは、" + name + "さん！";
} 
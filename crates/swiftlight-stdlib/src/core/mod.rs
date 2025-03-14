//! # SwiftLight言語のコアモジュール
//! 
//! このモジュールはSwiftLight言語の基本的な機能と型を提供します。
//! 言語機能の中核となる部分を実装しており、他のモジュールの基盤となります。

/// 基本的なデータ型の定義モジュール
pub mod types;

/// エラー処理関連の機能を提供するモジュール
pub mod error;

/// イテレータとコレクション関連の機能を提供するモジュール
pub mod iter;

/// コレクション型を提供するモジュール
pub mod collections;

/// メモリ管理関連の機能を提供するモジュール
pub mod memory;

// 標準プリミティブ型の再エクスポート
pub use self::types::{
    Int, Int8, Int16, Int32, Int64,
    UInt, UInt8, UInt16, UInt32, UInt64,
    Float32, Float64,
    Bool, Char, String, Array, Optional, Result,
};

// エラー関連の再エクスポート
pub use self::error::{Error, ErrorKind, Result as ErrorResult};

// イテレータ関連の再エクスポート
pub use self::iter::{Iterator, IntoIterator, IteratorExt};

// コレクション関連の再エクスポート
pub use self::collections::{Vec, HashMap, HashSet, BTreeMap, BTreeSet};

/// 名前空間を提供する構造体
/// 
/// 名前空間内に型や関数を配置するために使用します
pub struct Namespace;

impl Namespace {
    /// 名前空間の情報を文字列で返す
    pub fn info() -> &'static str {
        "SwiftLight言語のコア名前空間"
    }
}

/// プログラムのエントリーポイント情報
pub struct EntryPoint;

impl EntryPoint {
    /// メイン関数の実行
    pub fn run<F, R>(main_fn: F) -> R
    where
        F: FnOnce() -> R,
    {
        // 初期化処理があればここで実行
        let result = main_fn();
        // 終了処理があればここで実行
        result
    }
}

/// コア機能のバージョン情報を取得
pub fn version() -> &'static str {
    "0.1.0"
}

/// デバッグ情報出力用の関数
pub fn debug_print(message: &str) {
    println!("[SwiftLight Core Debug]: {}", message);
} 
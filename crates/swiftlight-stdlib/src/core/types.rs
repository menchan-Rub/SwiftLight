//! # SwiftLight言語の基本データ型
//! 
//! このモジュールでは、SwiftLight言語で使用される基本的なデータ型を定義します。
//! これらの型は言語の型システムの基盤となるもので、様々な場面で使用されます。

use std::fmt;
use std::cmp::PartialEq;
use std::convert::{From, Into};

/// 符号付き整数型（デフォルトは32ビット）
pub type Int = i32;

/// 8ビット符号付き整数型
pub type Int8 = i8;

/// 16ビット符号付き整数型
pub type Int16 = i16;

/// 32ビット符号付き整数型
pub type Int32 = i32;

/// 64ビット符号付き整数型
pub type Int64 = i64;

/// 符号なし整数型（デフォルトは32ビット）
pub type UInt = u32;

/// 8ビット符号なし整数型
pub type UInt8 = u8;

/// 16ビット符号なし整数型
pub type UInt16 = u16;

/// 32ビット符号なし整数型
pub type UInt32 = u32;

/// 64ビット符号なし整数型
pub type UInt64 = u64;

/// 32ビット浮動小数点型
pub type Float32 = f32;

/// 64ビット浮動小数点型
pub type Float64 = f64;

/// 論理値型
pub type Bool = bool;

/// 文字型
pub type Char = char;

/// 文字列型
pub type String = std::string::String;

/// 配列型
pub type Array<T> = Vec<T>;

/// オプション型（値が存在するかしないかを表現）
pub type Optional<T> = Option<T>;

/// 結果型（成功か失敗かを表現）
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// 標準エラー型
#[derive(Debug, Clone)]
pub struct Error {
    kind: ErrorKind,
    message: String,
}

/// エラー種別
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// 値エラー：型や値に関連するエラー
    ValueError,
    /// 範囲エラー：配列や文字列のインデックスが範囲外
    RangeError,
    /// 型エラー：型の不一致や変換エラー
    TypeError,
    /// 名前エラー：存在しない名前を参照
    NameError,
    /// 入出力エラー：I/O操作での失敗
    IOError,
    /// メモリエラー：メモリ確保や管理の問題
    MemoryError,
    /// 実行時エラー：実行中の一般的なエラー
    RuntimeError,
    /// 構文エラー：文法誤りがある場合
    SyntaxError,
    /// オーバーフローエラー：数値計算での溢れ
    OverflowError,
    /// 未実装エラー：実装されていない機能
    NotImplementedError,
    /// 環境変数関連エラー
    EnvError,
    /// プロセス関連エラー
    ProcessError,
    /// 時間関連エラー
    TimeError,
    /// FFI関連エラー
    FFIError,
    /// 引数無効エラー
    InvalidArgument,
    /// その他のエラー
    Other,
}

impl Error {
    /// 新しいエラーを作成
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
    
    /// エラーの種類を取得
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
    
    /// エラーメッセージを取得
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for Error {}

/// SwiftLightの型を表すトレイト
pub trait Type: 'static + Sized + fmt::Debug + Clone + PartialEq + Send + Sync {
    /// 型名を取得
    fn type_name() -> &'static str;
    
    /// 値の文字列表現を取得
    fn to_string(&self) -> String;
    
    /// デフォルト値を取得
    fn default_value() -> Self;
}

// 基本型に対するTypeトレイトの実装
macro_rules! impl_type_for_primitives {
    ($type:ty, $name:expr, $default:expr) => {
        impl Type for $type {
            fn type_name() -> &'static str {
                $name
            }
            
            fn to_string(&self) -> String {
                format!("{}", self)
            }
            
            fn default_value() -> Self {
                $default
            }
        }
    };
}

impl_type_for_primitives!(i8, "Int8", 0);
impl_type_for_primitives!(i16, "Int16", 0);
impl_type_for_primitives!(i32, "Int32", 0);
impl_type_for_primitives!(i64, "Int64", 0);

impl_type_for_primitives!(u8, "UInt8", 0);
impl_type_for_primitives!(u16, "UInt16", 0);
impl_type_for_primitives!(u32, "UInt32", 0);
impl_type_for_primitives!(u64, "UInt64", 0);

impl_type_for_primitives!(f32, "Float32", 0.0);
impl_type_for_primitives!(f64, "Float64", 0.0);

impl_type_for_primitives!(bool, "Bool", false);
impl_type_for_primitives!(char, "Char", '\0');

impl Type for String {
    fn type_name() -> &'static str {
        "String"
    }
    
    fn to_string(&self) -> String {
        self.clone()
    }
    
    fn default_value() -> Self {
        String::new()
    }
}

/// 型エイリアスの取得
pub fn get_type_alias(type_name: &str) -> Option<&'static str> {
    match type_name {
        "int" => Some("Int32"),
        "uint" => Some("UInt32"),
        "float" => Some("Float64"),
        "double" => Some("Float64"),
        "boolean" => Some("Bool"),
        "character" => Some("Char"),
        "str" => Some("String"),
        _ => None,
    }
}

/// 型変換を行う関数
pub fn convert<T, U>(value: T) -> Result<U, Error>
where
    T: Type,
    U: Type + From<T>,
{
    Ok(U::from(value))
}

/// 型チェックを行う関数
pub fn is_same_type<T, U>() -> bool
where
    T: Type,
    U: Type,
{
    T::type_name() == U::type_name()
}

/// 型の互換性をチェックする関数
pub fn is_compatible<T, U>() -> bool
where
    T: Type,
    U: Type,
{
    // 同じ型なら互換性がある
    if is_same_type::<T, U>() {
        return true;
    }
    
    // 型変換のルールに基づいて互換性をチェック
    match (T::type_name(), U::type_name()) {
        // 整数型の互換性
        ("Int8", "Int16") | ("Int8", "Int32") | ("Int8", "Int64") |
        ("Int16", "Int32") | ("Int16", "Int64") |
        ("Int32", "Int64") => true,
        
        // 浮動小数点型の互換性
        ("Float32", "Float64") => true,
        
        // 整数型から浮動小数点型への互換性
        ("Int8", "Float32") | ("Int8", "Float64") |
        ("Int16", "Float32") | ("Int16", "Float64") |
        ("Int32", "Float32") | ("Int32", "Float64") |
        ("Int64", "Float64") => true,
        
        // 符号なし整数型の互換性
        ("UInt8", "UInt16") | ("UInt8", "UInt32") | ("UInt8", "UInt64") |
        ("UInt16", "UInt32") | ("UInt16", "UInt64") |
        ("UInt32", "UInt64") => true,
        
        // デフォルトでは互換性なし
        _ => false,
    }
}

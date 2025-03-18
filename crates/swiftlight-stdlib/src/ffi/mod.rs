//! # SwiftLight言語の外部関数インターフェース（FFI）モジュール
//! 
//! このモジュールはSwiftLight言語と他の言語（主にC言語）との間の
//! 相互運用性を提供します。
//! 
//! 外部ライブラリのロード、外部関数の呼び出し、データ変換などの機能を含みます。

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void, c_int};
use crate::core::types::{Error, ErrorKind, Result};
use crate::core::collections::Vec;

/// 外部ライブラリへのハンドル
pub struct Library {
    inner: libloading::Library,
}

impl Library {
    /// 指定した名前のライブラリをロード
    pub fn load(name: &str) -> Result<Self> {
        let lib = unsafe {
            libloading::Library::new(name).map_err(|e| {
                Error::new(
                    ErrorKind::FFIError,
                    format!("ライブラリのロードに失敗しました: {}", e)
                )
            })?
        };
        
        Ok(Self { inner: lib })
    }
    
    /// シンボルを取得
    pub unsafe fn get<T>(&self, symbol: &str) -> Result<libloading::Symbol<T>> {
        let symbol_cstr = CString::new(symbol).map_err(|e| {
            Error::new(
                ErrorKind::FFIError,
                format!("シンボル名の変換に失敗しました: {}", e)
            )
        })?;
        
        self.inner.get(symbol_cstr.as_bytes()).map_err(|e| {
            Error::new(
                ErrorKind::FFIError,
                format!("シンボルの取得に失敗しました: {}", e)
            )
        })
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        // ライブラリは自動的にクローズされます
    }
}

/// C文字列との変換
pub mod string {
    use super::*;
    
    /// SwiftLight文字列をC文字列に変換
    pub fn to_c_string(s: &str) -> Result<CString> {
        CString::new(s).map_err(|e| {
            Error::new(
                ErrorKind::FFIError,
                format!("C文字列への変換に失敗しました: {}", e)
            )
        })
    }
    
    /// C文字列をSwiftLight文字列に変換
    pub unsafe fn from_c_string(s: *const c_char) -> Result<String> {
        if s.is_null() {
            return Err(Error::new(
                ErrorKind::FFIError,
                "NULL文字列ポインタです"
            ));
        }
        
        CStr::from_ptr(s).to_str().map(String::from).map_err(|e| {
            Error::new(
                ErrorKind::FFIError,
                format!("C文字列からの変換に失敗しました: {}", e)
            )
        })
    }
    
    /// C文字列の配列をSwiftLight文字列のベクターに変換
    pub unsafe fn from_c_string_array(arr: *const *const c_char, len: usize) -> Result<Vec<String>> {
        if arr.is_null() {
            return Err(Error::new(
                ErrorKind::FFIError,
                "NULL配列ポインタです"
            ));
        }
        
        let mut result = Vec::new();
        for i in 0..len {
            let ptr = *arr.add(i);
            let s = from_c_string(ptr)?;
            result.push(s);
        }
        
        Ok(result)
    }
}

/// CコンパチブルなCallback型
pub mod callback {
    use super::*;
    
    /// 汎用的なC互換コールバック型
    pub type Callback = extern "C" fn(*const c_void) -> c_int;
    
    /// コールバックと引数の登録
    pub struct CallbackRegistration {
        callback: Callback,
        data: *mut c_void,
    }
    
    impl CallbackRegistration {
        /// 新しいコールバック登録を作成
        pub fn new(callback: Callback, data: *mut c_void) -> Self {
            Self { callback, data }
        }
        
        /// コールバックを実行
        pub unsafe fn invoke(&self) -> i32 {
            (self.callback)(self.data)
        }
    }
}

/// C ABIを使用した関数エクスポート
/// 
/// SwiftLight関数をC言語からアクセス可能な形式でエクスポートするためのマクロ
#[macro_export]
macro_rules! export_c_function {
    (fn $name:ident($($arg:ident: $type:ty),*) -> $ret:ty $body:block) => {
        #[no_mangle]
        pub extern "C" fn $name($($arg: $type),*) -> $ret {
            $body
        }
    };
}

/// 共有メモリ管理
pub mod memory {
    use super::*;
    
    /// 外部から受け取ったメモリを管理
    pub struct ExternalMemory {
        ptr: *mut c_void,
        size: usize,
        free_fn: Option<extern "C" fn(*mut c_void)>,
    }
    
    impl ExternalMemory {
        /// 外部メモリを取得
        pub unsafe fn new(ptr: *mut c_void, size: usize) -> Self {
            Self {
                ptr,
                size,
                free_fn: None,
            }
        }
        
        /// 解放関数付きで外部メモリを取得
        pub unsafe fn with_free_fn(
            ptr: *mut c_void,
            size: usize,
            free_fn: extern "C" fn(*mut c_void)
        ) -> Self {
            Self {
                ptr,
                size,
                free_fn: Some(free_fn),
            }
        }
        
        /// ポインタを取得
        pub fn as_ptr(&self) -> *const c_void {
            self.ptr as *const c_void
        }
        
        /// 可変ポインタを取得
        pub fn as_mut_ptr(&mut self) -> *mut c_void {
            self.ptr
        }
        
        /// サイズを取得
        pub fn size(&self) -> usize {
            self.size
        }
        
        /// バイト配列に変換
        pub unsafe fn as_bytes(&self) -> &[u8] {
            std::slice::from_raw_parts(self.ptr as *const u8, self.size)
        }
        
        /// 可変バイト配列に変換
        pub unsafe fn as_bytes_mut(&mut self) -> &mut [u8] {
            std::slice::from_raw_parts_mut(self.ptr as *mut u8, self.size)
        }
    }
    
    impl Drop for ExternalMemory {
        fn drop(&mut self) {
            unsafe {
                if let Some(free_fn) = self.free_fn {
                    free_fn(self.ptr);
                }
            }
        }
    }
}

/// 型変換ユーティリティ
pub mod convert {
    
    
    /// SwiftLightのバイト配列をCのバイト配列に変換
    pub fn bytes_to_c(bytes: &[u8]) -> (*const u8, usize) {
        (bytes.as_ptr(), bytes.len())
    }
    
    /// SwiftLightの数値配列をCの配列に変換
    pub fn slice_to_c<T>(slice: &[T]) -> (*const T, usize) {
        (slice.as_ptr(), slice.len())
    }
    
    /// CスタイルのEnum値をRust Enumに変換するためのトレイト
    pub trait FromFFI<T> {
        fn from_ffi(value: T) -> Option<Self> where Self: Sized;
        fn to_ffi(&self) -> T;
    }
} 
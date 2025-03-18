//! # SwiftLight言語のメモリ管理モジュール
//! 
//! このモジュールはメモリ管理と割り当てに関連する機能を提供します。
//! メモリの割り当て、解放、参照カウント、およびその他のメモリ関連機能が含まれています。

use std::mem;
use std::alloc::{self, Layout};
use std::ptr::{self, NonNull};
use std::marker::PhantomData;
use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::core::types::{Error, ErrorKind};

/// メモリアロケーションエラーの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationErrorKind {
    /// メモリ不足エラー
    OutOfMemory,
    /// アラインメントエラー
    AlignmentError,
    /// 無効なサイズエラー
    InvalidSize,
    /// その他のエラー
    Other,
}

/// メモリアロケーションエラー
#[derive(Debug)]
pub struct AllocationError {
    kind: AllocationErrorKind,
    message: String,
}

impl AllocationError {
    /// 新しいアロケーションエラーを作成
    pub fn new(kind: AllocationErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
    
    /// エラーの種類を取得
    pub fn kind(&self) -> AllocationErrorKind {
        self.kind
    }
    
    /// エラーメッセージを取得
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for AllocationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "メモリ割り当てエラー: {:?} - {}", self.kind, self.message)
    }
}

impl std::error::Error for AllocationError {}

impl From<AllocationError> for Error {
    fn from(error: AllocationError) -> Self {
        Error::new(
            ErrorKind::MemoryError,
            format!("メモリ割り当てに失敗しました: {}", error)
        )
    }
}

/// 基本的なメモリアロケーター
pub struct Allocator;

impl Allocator {
    /// 指定したサイズとアラインメントでメモリを割り当てる
    pub unsafe fn allocate(size: usize, align: usize) -> Result<NonNull<u8>, AllocationError> {
        if size == 0 {
            return Err(AllocationError::new(
                AllocationErrorKind::InvalidSize,
                "サイズが0のメモリは割り当てられません"
            ));
        }
        
        if !align.is_power_of_two() {
            return Err(AllocationError::new(
                AllocationErrorKind::AlignmentError,
                "アラインメントは2のべき乗である必要があります"
            ));
        }
        
        let layout = Layout::from_size_align(size, align)
            .map_err(|_| AllocationError::new(
                AllocationErrorKind::AlignmentError,
                "無効なサイズまたはアラインメント"
            ))?;
        
        let ptr = alloc::alloc(layout);
        
        if ptr.is_null() {
            Err(AllocationError::new(
                AllocationErrorKind::OutOfMemory,
                "メモリの割り当てに失敗しました"
            ))
        } else {
            Ok(NonNull::new_unchecked(ptr))
        }
    }
    
    /// 割り当てられたメモリを解放する
    pub unsafe fn deallocate(ptr: NonNull<u8>, size: usize, align: usize) {
        if size == 0 {
            return;
        }
        
        let layout = Layout::from_size_align_unchecked(size, align);
        alloc::dealloc(ptr.as_ptr(), layout);
    }
    
    /// 割り当てられたメモリのサイズを変更する
    pub unsafe fn reallocate(
        ptr: NonNull<u8>,
        old_size: usize,
        old_align: usize,
        new_size: usize,
    ) -> Result<NonNull<u8>, AllocationError> {
        if new_size == 0 {
            Self::deallocate(ptr, old_size, old_align);
            return Err(AllocationError::new(
                AllocationErrorKind::InvalidSize,
                "サイズが0のメモリは割り当てられません"
            ));
        }
        
        let old_layout = Layout::from_size_align_unchecked(old_size, old_align);
        let new_layout = Layout::from_size_align(new_size, old_align)
            .map_err(|_| AllocationError::new(
                AllocationErrorKind::AlignmentError,
                "無効なサイズまたはアラインメント"
            ))?;
        
        let ptr = alloc::realloc(ptr.as_ptr(), old_layout, new_size);
        
        if ptr.is_null() {
            Err(AllocationError::new(
                AllocationErrorKind::OutOfMemory,
                "メモリの再割り当てに失敗しました"
            ))
        } else {
            Ok(NonNull::new_unchecked(ptr))
        }
    }
}

/// ボックス型（ヒープに割り当てられたオブジェクト）
pub struct Box<T> {
    ptr: NonNull<T>,
    phantom: PhantomData<T>,
}

impl<T> Box<T> {
    /// 新しいボックスを作成し、値を移動する
    pub fn new(value: T) -> Self {
        let ptr = unsafe {
            let layout = Layout::new::<T>();
            let ptr = match Allocator::allocate(layout.size(), layout.align()) {
                Ok(ptr) => ptr,
                Err(_) => std::alloc::handle_alloc_error(layout),
            };
            let ptr = ptr.as_ptr() as *mut T;
            ptr::write(ptr, value);
            NonNull::new_unchecked(ptr)
        };
        
        Self {
            ptr,
            phantom: PhantomData,
        }
    }
    
    /// ボックスから値を取り出す
    pub fn into_inner(self) -> T {
        let value = unsafe { ptr::read(self.ptr.as_ptr()) };
        // Boxを所有権システムから削除して、デストラクタが呼ばれないようにする
        mem::forget(self);
        value
    }
    
    /// ボックスを生ポインタに変換し、所有権を放棄する
    pub fn into_raw(b: Self) -> *mut T {
        let ptr = b.ptr.as_ptr();
        mem::forget(b);
        ptr
    }
    
    /// 生ポインタからボックスを作成する
    /// 
    /// # Safety
    /// 
    /// ポインタはBoxのinto_rawから取得したものでなければならない
    pub unsafe fn from_raw(raw: *mut T) -> Self {
        Self {
            ptr: NonNull::new_unchecked(raw),
            phantom: PhantomData,
        }
    }
}

impl<T> Drop for Box<T> {
    fn drop(&mut self) {
        unsafe {
            // 値を解放
            ptr::drop_in_place(self.ptr.as_ptr());
            
            // メモリを解放
            let layout = Layout::new::<T>();
            Allocator::deallocate(
                NonNull::new_unchecked(self.ptr.as_ptr() as *mut u8),
                layout.size(),
                layout.align()
            );
        }
    }
}

impl<T> std::ops::Deref for Box<T> {
    type Target = T;
    
    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> std::ops::DerefMut for Box<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut() }
    }
}

/// 参照カウントボックス
/// 
/// 複数の所有権を持つことができるスマートポインタ
pub struct Rc<T> {
    ptr: NonNull<RcInner<T>>,
    phantom: PhantomData<RcInner<T>>,
}

struct RcInner<T> {
    value: T,
    ref_count: AtomicUsize,
}

impl<T> Rc<T> {
    /// 新しい参照カウントボックスを作成する
    pub fn new(value: T) -> Self {
        // RcInnerを確保
        let inner = RcInner {
            value,
            ref_count: AtomicUsize::new(1),
        };
        
        // ヒープにRcInnerを配置
        let ptr = Box::new(inner);
        let ptr = Box::into_raw(ptr);
        
        Self {
            ptr: unsafe { NonNull::new_unchecked(ptr) },
            phantom: PhantomData,
        }
    }
    
    /// 参照カウントを取得
    pub fn strong_count(this: &Self) -> usize {
        let inner = unsafe { this.ptr.as_ref() };
        inner.ref_count.load(Ordering::SeqCst)
    }
}

impl<T> Clone for Rc<T> {
    fn clone(&self) -> Self {
        let inner = unsafe { self.ptr.as_ref() };
        // 参照カウントをインクリメント
        let old_count = inner.ref_count.fetch_add(1, Ordering::SeqCst);
        
        // カウントのオーバーフローをチェック
        if old_count > isize::MAX as usize {
            std::process::abort();
        }
        
        Self {
            ptr: self.ptr,
            phantom: PhantomData,
        }
    }
}

impl<T> Drop for Rc<T> {
    fn drop(&mut self) {
        let inner = unsafe { self.ptr.as_ref() };
        // 参照カウントをデクリメント
        if inner.ref_count.fetch_sub(1, Ordering::SeqCst) != 1 {
            return;
        }
        
        // 最後の参照なので、値を解放
        let _ = unsafe { Box::from_raw(self.ptr.as_ptr()) };
    }
}

impl<T> std::ops::Deref for Rc<T> {
    type Target = T;
    
    fn deref(&self) -> &Self::Target {
        let inner = unsafe { self.ptr.as_ref() };
        &inner.value
    }
}

/// メモリリーク検出機能
pub struct LeakDetector {
    allocations: RefCell<Vec<(*mut u8, usize)>>,
}

impl LeakDetector {
    /// 新しいリーク検出器を作成
    pub fn new() -> Self {
        Self {
            allocations: RefCell::new(Vec::new()),
        }
    }
    
    /// メモリ割り当てを記録
    pub fn track_allocation(&self, ptr: *mut u8, size: usize) {
        let mut allocations = self.allocations.borrow_mut();
        allocations.push((ptr, size));
    }
    
    /// メモリ解放を記録
    pub fn track_deallocation(&self, ptr: *mut u8) {
        let mut allocations = self.allocations.borrow_mut();
        if let Some(index) = allocations.iter().position(|&(p, _)| p == ptr) {
            allocations.remove(index);
        }
    }
    
    /// リークしたメモリの合計サイズを取得
    pub fn get_leaked_bytes(&self) -> usize {
        let allocations = self.allocations.borrow();
        allocations.iter().map(|&(_, size)| size).sum()
    }
    
    /// リークしているメモリブロックの数を取得
    pub fn get_leaked_blocks(&self) -> usize {
        let allocations = self.allocations.borrow();
        allocations.len()
    }
    
    /// リーク情報をレポート
    pub fn report_leaks(&self) -> String {
        let allocations = self.allocations.borrow();
        if allocations.is_empty() {
            return "メモリリークは検出されませんでした。".to_string();
        }
        
        let mut report = format!("{}個のメモリリークが検出されました。合計: {}バイト\n", 
                                allocations.len(), 
                                allocations.iter().map(|&(_, size)| size).sum::<usize>());
        
        for (i, &(ptr, size)) in allocations.iter().enumerate() {
            report.push_str(&format!("リーク #{}: アドレス: {:?}, サイズ: {}バイト\n", 
                                    i + 1, ptr, size));
        }
        
        report
    }
}

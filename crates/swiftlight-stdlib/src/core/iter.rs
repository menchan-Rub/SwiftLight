//! # SwiftLight言語のイテレータモジュール
//! 
//! このモジュールはコレクションを反復処理するためのイテレータシステムを提供します。
//! イテレータトレイトとイテレータのユーティリティ機能が含まれています。

use std::iter::{self, Iterator as StdIterator};
use std::collections::VecDeque;
use std::cmp::{min, max};
use std::fmt::Debug;

use crate::core::types::{Error, ErrorKind};

/// イテレータトレイト
///
/// コレクションの要素を順番に処理するための基本的なインターフェースを提供します。
pub trait Iterator: Sized {
    /// イテレータが生成する要素の型
    type Item;
    
    /// 次の要素を取得する
    fn next(&mut self) -> Option<Self::Item>;
    
    /// 残りの要素の数のヒントを提供
    /// 
    /// 正確な値を返す必要はありませんが、可能な限り正確なヒントを提供することが推奨されます。
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
    
    /// イテレータをベクターに変換
    fn collect_vec(self) -> Vec<Self::Item>
    where
        Self::Item: Sized,
    {
        let (min_size, _) = self.size_hint();
        let mut result = Vec::with_capacity(min_size);
        self.for_each(|item| result.push(item));
        result
    }
    
    /// 各要素に関数を適用するイテレータを作成
    fn map<B, F>(self, f: F) -> Map<Self, F>
    where
        F: FnMut(Self::Item) -> B,
    {
        Map::new(self, f)
    }
    
    /// 条件を満たす要素だけを含むイテレータを作成
    fn filter<P>(self, predicate: P) -> Filter<Self, P>
    where
        P: FnMut(&Self::Item) -> bool,
    {
        Filter::new(self, predicate)
    }
    
    /// イテレータから最初のn個の要素だけを含むイテレータを作成
    fn take(self, n: usize) -> Take<Self> {
        Take::new(self, n)
    }
    
    /// イテレータから最初のn個の要素をスキップするイテレータを作成
    fn skip(self, n: usize) -> Skip<Self> {
        Skip::new(self, n)
    }
    
    /// 連続する同じ要素を1つだけ含むイテレータを作成
    fn dedup(self) -> Dedup<Self>
    where
        Self::Item: PartialEq,
    {
        Dedup::new(self)
    }
    
    /// イテレータの全要素に関数を適用
    fn for_each<F>(mut self, mut f: F)
    where
        F: FnMut(Self::Item),
    {
        while let Some(item) = self.next() {
            f(item);
        }
    }
    
    /// 合計を計算
    fn sum(mut self) -> Self::Item
    where
        Self::Item: Default + std::ops::Add<Output = Self::Item>,
    {
        let mut sum = Self::Item::default();
        while let Some(item) = self.next() {
            sum = sum + item;
        }
        sum
    }
    
    /// 最大値を取得
    fn max(mut self) -> Option<Self::Item>
    where
        Self::Item: Ord,
    {
        self.next().map(|first| {
            self.fold(first, |max, item| if item > max { item } else { max })
        })
    }
    
    /// 最小値を取得
    fn min(mut self) -> Option<Self::Item>
    where
        Self::Item: Ord,
    {
        self.next().map(|first| {
            self.fold(first, |min, item| if item < min { item } else { min })
        })
    }
    
    /// 初期値と関数を使って値を畳み込む
    fn fold<B, F>(mut self, init: B, mut f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        let mut accum = init;
        while let Some(item) = self.next() {
            accum = f(accum, item);
        }
        accum
    }
    
    /// 全ての要素が条件を満たすかどうかを確認
    fn all<F>(mut self, mut predicate: F) -> bool
    where
        F: FnMut(Self::Item) -> bool,
    {
        while let Some(item) = self.next() {
            if !predicate(item) {
                return false;
            }
        }
        true
    }
    
    /// いずれかの要素が条件を満たすかどうかを確認
    fn any<F>(mut self, mut predicate: F) -> bool
    where
        F: FnMut(Self::Item) -> bool,
    {
        while let Some(item) = self.next() {
            if predicate(item) {
                return true;
            }
        }
        false
    }
    
    /// 要素数をカウント
    fn count(mut self) -> usize {
        let mut count = 0;
        while let Some(_) = self.next() {
            count += 1;
        }
        count
    }
    
    /// イテレータの要素から新しいイテレータを作成するイテレータを平坦化
    fn flat_map<U, F>(self, f: F) -> FlatMap<Self, U, F>
    where
        F: FnMut(Self::Item) -> U,
        U: IntoIterator,
    {
        FlatMap::new(self, f)
    }
    
    /// イテレータを列挙する（インデックスと値のペアにする）
    fn enumerate(self) -> Enumerate<Self> {
        Enumerate::new(self)
    }
    
    /// イテレータの要素をペアにする
    fn zip<U>(self, other: U) -> Zip<Self, U::IntoIter>
    where
        U: IntoIterator,
    {
        Zip::new(self, other.into_iter())
    }
    
    /// イテレータから最初のn個の要素をスキップし、次のm個の要素を含むイテレータを作成
    fn slice(self, start: usize, len: usize) -> Slice<Self> {
        Slice::new(self, start, len)
    }
}

/// コレクションからイテレータを作成するトレイト
pub trait IntoIterator {
    /// イテレータの要素の型
    type Item;
    
    /// イテレータの型
    type IntoIter: Iterator<Item = Self::Item>;
    
    /// コレクションからイテレータを作成
    fn into_iter(self) -> Self::IntoIter;
}

/// イテレータ拡張トレイト
/// 
/// 追加のイテレータユーティリティ関数を提供します
pub trait IteratorExt: Iterator {
    /// 要素をチャンクに分割するイテレータを作成
    fn chunks(self, chunk_size: usize) -> Chunks<Self>
    where
        Self: Sized,
    {
        assert!(chunk_size > 0, "チャンクサイズは正の値である必要があります");
        Chunks::new(self, chunk_size)
    }
    
    /// 要素をウィンドウに分割するイテレータを作成
    fn windows(self, window_size: usize) -> Windows<Self>
    where
        Self: Sized,
        Self::Item: Clone,
    {
        assert!(window_size > 0, "ウィンドウサイズは正の値である必要があります");
        Windows::new(self, window_size)
    }
    
    /// イテレータをインターリーブする
    fn interleave<U>(self, other: U) -> Interleave<Self, U::IntoIter>
    where
        Self: Sized,
        U: IntoIterator<Item = Self::Item>,
    {
        Interleave::new(self, other.into_iter())
    }
    
    /// イテレータを交互に取得する
    fn alternate<U>(self, other: U) -> Alternate<Self, U::IntoIter>
    where
        Self: Sized,
        U: IntoIterator<Item = Self::Item>,
    {
        Alternate::new(self, other.into_iter())
    }
}

// 標準のイテレータに対するIteratorトレイトの実装
impl<I: StdIterator> Iterator for I {
    type Item = I::Item;
    
    fn next(&mut self) -> Option<Self::Item> {
        StdIterator::next(self)
    }
    
    fn size_hint(&self) -> (usize, Option<usize>) {
        StdIterator::size_hint(self)
    }
}

// 標準のイテレータに対するIteratorExtトレイトの実装
impl<I: Iterator> IteratorExt for I {}

// ここから各イテレータアダプタの実装
// マップイテレータ
pub struct Map<I, F> {
    iter: I,
    func: F,
}

impl<I, F> Map<I, F> {
    fn new(iter: I, func: F) -> Self {
        Self { iter, func }
    }
}

impl<I, F, B> Iterator for Map<I, F>
where
    I: Iterator,
    F: FnMut(I::Item) -> B,
{
    type Item = B;
    
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(&mut self.func)
    }
    
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

// フィルターイテレータ
pub struct Filter<I, P> {
    iter: I,
    predicate: P,
}

impl<I, P> Filter<I, P> {
    fn new(iter: I, predicate: P) -> Self {
        Self { iter, predicate }
    }
}

impl<I, P> Iterator for Filter<I, P>
where
    I: Iterator,
    P: FnMut(&I::Item) -> bool,
{
    type Item = I::Item;
    
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(item) = self.iter.next() {
            if (self.predicate)(&item) {
                return Some(item);
            }
        }
        None
    }
    
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, upper) = self.iter.size_hint();
        (0, upper)
    }
}

// 先頭n要素のイテレータ
pub struct Take<I> {
    iter: I,
    remaining: usize,
}

impl<I> Take<I> {
    fn new(iter: I, n: usize) -> Self {
        Self {
            iter,
            remaining: n,
        }
    }
}

impl<I: Iterator> Iterator for Take<I> {
    type Item = I::Item;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        self.iter.next()
    }
    
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lower, upper) = self.iter.size_hint();
        let lower = min(lower, self.remaining);
        let upper = upper.map(|u| min(u, self.remaining));
        (lower, upper)
    }
}

// スキップイテレータ
pub struct Skip<I> {
    iter: I,
    n: usize,
}

impl<I> Skip<I> {
    fn new(iter: I, n: usize) -> Self {
        Self { iter, n }
    }
}

impl<I: Iterator> Iterator for Skip<I> {
    type Item = I::Item;
    
    fn next(&mut self) -> Option<Self::Item> {
        while self.n > 0 {
            if self.iter.next().is_none() {
                return None;
            }
            self.n -= 1;
        }
        self.iter.next()
    }
    
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lower, upper) = self.iter.size_hint();
        let lower = lower.saturating_sub(self.n);
        let upper = upper.map(|u| u.saturating_sub(self.n));
        (lower, upper)
    }
}

// 重複排除イテレータ
pub struct Dedup<I: Iterator> {
    iter: I,
    last: Option<I::Item>,
}

impl<I: Iterator> Dedup<I> {
    fn new(iter: I) -> Self {
        Self { iter, last: None }
    }
}

impl<I: Iterator> Iterator for Dedup<I>
where
    I::Item: PartialEq + Clone,
{
    type Item = I::Item;
    
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(item) = self.iter.next() {
            if let Some(ref last) = self.last {
                if *last == item {
                    continue;
                }
            }
            self.last = Some(item.clone());
            return self.last.clone();
        }
        None
    }
}

// フラットマップイテレータ
pub struct FlatMap<I: Iterator, U, F>
where
    U: IntoIterator,
    F: FnMut(I::Item) -> U,
{
    iter: I,
    func: F,
    current: Option<U::IntoIter>,
}

impl<I: Iterator, U, F> FlatMap<I, U, F>
where
    U: IntoIterator,
    F: FnMut(I::Item) -> U,
{
    fn new(iter: I, func: F) -> Self {
        Self {
            iter,
            func,
            current: None,
        }
    }
}

impl<I, U, F> Iterator for FlatMap<I, U, F>
where
    I: Iterator,
    F: FnMut(I::Item) -> U,
    U: IntoIterator,
{
    type Item = U::Item;
    
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(ref mut current) = self.current {
                if let Some(item) = current.next() {
                    return Some(item);
                }
                self.current = None;
            }
            
            let next_item = self.iter.next()?;
            self.current = Some((self.func)(next_item).into_iter());
        }
    }
}

// 列挙イテレータ
pub struct Enumerate<I> {
    iter: I,
    index: usize,
}

impl<I> Enumerate<I> {
    fn new(iter: I) -> Self {
        Self { iter, index: 0 }
    }
}

impl<I: Iterator> Iterator for Enumerate<I> {
    type Item = (usize, I::Item);
    
    fn next(&mut self) -> Option<Self::Item> {
        let item = self.iter.next()?;
        let index = self.index;
        self.index += 1;
        Some((index, item))
    }
    
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

// ジップイテレータ
pub struct Zip<A, B> {
    a: A,
    b: B,
}

impl<A, B> Zip<A, B> {
    fn new(a: A, b: B) -> Self {
        Self { a, b }
    }
}

impl<A, B> Iterator for Zip<A, B>
where
    A: Iterator,
    B: Iterator,
{
    type Item = (A::Item, B::Item);
    
    fn next(&mut self) -> Option<Self::Item> {
        match (self.a.next(), self.b.next()) {
            (Some(a), Some(b)) => Some((a, b)),
            _ => None,
        }
    }
    
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (a_lower, a_upper) = self.a.size_hint();
        let (b_lower, b_upper) = self.b.size_hint();
        
        let lower = min(a_lower, b_lower);
        let upper = match (a_upper, b_upper) {
            (Some(a), Some(b)) => Some(min(a, b)),
            _ => None,
        };
        
        (lower, upper)
    }
}

// スライスイテレータ
pub struct Slice<I: Iterator> {
    iter: I,
    start: usize,
    remaining: usize,
    started: bool,
}

impl<I: Iterator> Slice<I> {
    fn new(iter: I, start: usize, len: usize) -> Self {
        Self { 
            iter, 
            start,
            remaining: len,
            started: false,
        }
    }
}

impl<I: Iterator> Iterator for Slice<I> {
    type Item = I::Item;
    
    fn next(&mut self) -> Option<Self::Item> {
        // スタート位置までスキップする
        if !self.started {
            for _ in 0..self.start {
                if self.iter.next().is_none() {
                    return None;
                }
            }
            self.started = true;
        }
        
        // 残りの長さが0ならNoneを返す
        if self.remaining == 0 {
            return None;
        }
        
        // 次の要素を取得し、残りの長さを減らす
        let next = self.iter.next();
        if next.is_some() {
            self.remaining -= 1;
        }
        next
    }
    
    fn size_hint(&self) -> (usize, Option<usize>) {
        if !self.started {
            let (lower, upper) = self.iter.size_hint();
            let lower = lower.saturating_sub(self.start);
            let upper = upper.map(|u| u.saturating_sub(self.start));
            (0, upper.map(|u| std::cmp::min(u, self.remaining)))
        } else {
            (0, Some(self.remaining))
        }
    }
}

// チャンクイテレータ
pub struct Chunks<I> {
    iter: I,
    chunk_size: usize,
}

impl<I> Chunks<I> {
    fn new(iter: I, chunk_size: usize) -> Self {
        Self { iter, chunk_size }
    }
}

impl<I: Iterator> Iterator for Chunks<I> {
    type Item = Vec<I::Item>;
    
    fn next(&mut self) -> Option<Self::Item> {
        let mut chunk = Vec::with_capacity(self.chunk_size);
        for _ in 0..self.chunk_size {
            match self.iter.next() {
                Some(item) => chunk.push(item),
                None if chunk.is_empty() => return None,
                None => break,
            }
        }
        Some(chunk)
    }
}

// ウィンドウイテレータ
pub struct Windows<I>
where
    I: Iterator,
    I::Item: Clone,
{
    iter: I,
    window: VecDeque<I::Item>,
    window_size: usize,
    done: bool,
}

impl<I> Windows<I>
where
    I: Iterator,
    I::Item: Clone,
{
    fn new(mut iter: I, window_size: usize) -> Self {
        let mut window = VecDeque::with_capacity(window_size);
        for _ in 0..window_size {
            if let Some(item) = iter.next() {
                window.push_back(item);
            } else {
                break;
            }
        }
        let done = window.len() < window_size;
        Self {
            iter,
            window,
            window_size,
            done,
        }
    }
}

impl<I> Iterator for Windows<I>
where
    I: Iterator,
    I::Item: Clone,
{
    type Item = Vec<I::Item>;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.done || self.window.is_empty() {
            return None;
        }
        
        let result: Vec<_> = self.window.iter().cloned().collect();
        
        if let Some(item) = self.iter.next() {
            self.window.pop_front();
            self.window.push_back(item);
        } else {
            self.done = true;
        }
        
        Some(result)
    }
}

// インターリーブイテレータ
pub struct Interleave<A, B> {
    a: A,
    b: B,
    next_a: bool,  // 変数名を変更して意図を明確に
}

impl<A, B> Interleave<A, B> {
    fn new(a: A, b: B) -> Self {
        Self {
            a,
            b,
            next_a: true,
        }
    }
}

impl<A, B> Iterator for Interleave<A, B>
where
    A: Iterator,
    B: Iterator<Item = A::Item>,
{
    type Item = A::Item;
    
    fn next(&mut self) -> Option<Self::Item> {
        // 条件分岐の実装を単純化して型の不一致を解消
        if self.next_a {
            self.next_a = false;
            match self.a.next() {
                Some(item) => Some(item),
                None => self.b.next(),
            }
        } else {
            self.next_a = true;
            match self.b.next() {
                Some(item) => Some(item),
                None => self.a.next(),
            }
        }
    }
    
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (a_lower, a_upper) = self.a.size_hint();
        let (b_lower, b_upper) = self.b.size_hint();
        
        let lower = a_lower + b_lower;
        let upper = match (a_upper, b_upper) {
            (Some(a), Some(b)) => Some(a + b),
            _ => None,
        };
        
        (lower, upper)
    }
}

// 交互イテレータ
pub struct Alternate<A, B> {
    a: A,
    b: B,
    a_next: bool,
}

impl<A, B> Alternate<A, B> {
    fn new(a: A, b: B) -> Self {
        Self {
            a,
            b,
            a_next: true,
        }
    }
}

impl<A, B> Iterator for Alternate<A, B>
where
    A: Iterator,
    B: Iterator<Item = A::Item>,
{
    type Item = A::Item;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.a_next {
            self.a_next = false;
            match self.a.next() {
                Some(item) => Some(item),
                None => {
                    self.a_next = false;
                    self.b.next()
                }
            }
        } else {
            self.a_next = true;
            match self.b.next() {
                Some(item) => Some(item),
                None => {
                    self.a_next = true;
                    self.a.next()
                }
            }
        }
    }
}

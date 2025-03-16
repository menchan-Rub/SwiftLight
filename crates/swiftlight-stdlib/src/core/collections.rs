//! # SwiftLight言語のコレクションモジュール
//! 
//! このモジュールはSwiftLight言語で使用する基本的なコレクション型を提供します。
//! ベクター、マップ、セットなどのデータ構造が含まれています。

use std::collections::{HashMap as StdHashMap, HashSet as StdHashSet};
use std::collections::{BTreeMap as StdBTreeMap, BTreeSet as StdBTreeSet};
use std::hash::{Hash, Hasher};
use std::fmt::{self, Debug};
use std::ops::{Index, IndexMut};
use std::cmp::Ordering;

use crate::core::types::{Error, ErrorKind};
use std::iter::Iterator as StdIterator;

/// 可変長配列型
/// 
/// 要素の追加、削除、アクセスが効率的なベクター型を提供します。
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Vec<T> {
    /// 内部データ
    inner: std::vec::Vec<T>,
}

impl<T> Vec<T> {
    /// 新しい空のベクターを作成
    pub fn new() -> Self {
        Self {
            inner: std::vec::Vec::new(),
        }
    }
    
    /// 指定した容量で新しいベクターを作成
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: std::vec::Vec::with_capacity(capacity),
        }
    }
    
    /// ベクターに要素を追加
    pub fn push(&mut self, value: T) {
        self.inner.push(value);
    }
    
    /// ベクターの末尾から要素を取り出し
    pub fn pop(&mut self) -> Option<T> {
        self.inner.pop()
    }
    
    /// 指定したインデックスの要素を取得
    pub fn get(&self, index: usize) -> Option<&T> {
        self.inner.get(index)
    }
    
    /// 指定したインデックスの要素を可変で取得
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.inner.get_mut(index)
    }
    
    /// ベクターの長さを取得
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    
    /// ベクターが空かどうかを確認
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    
    /// ベクターの容量を取得
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }
    
    /// ベクターの容量を最適化
    pub fn shrink_to_fit(&mut self) {
        self.inner.shrink_to_fit();
    }
    
    /// 指定したインデックスに要素を挿入
    pub fn insert(&mut self, index: usize, element: T) {
        self.inner.insert(index, element);
    }
    
    /// 指定したインデックスの要素を削除して返す
    pub fn remove(&mut self, index: usize) -> T {
        self.inner.remove(index)
    }
    
    /// ベクターをスライスに変換
    pub fn as_slice(&self) -> &[T] {
        self.inner.as_slice()
    }
    
    /// ベクターを可変スライスに変換
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.inner.as_mut_slice()
    }
    
    /// ベクターをクリア
    pub fn clear(&mut self) {
        self.inner.clear();
    }
    
    /// ベクターの要素をイテレート
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.inner.iter()
    }
    
    /// ベクターの要素を可変でイテレート
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.inner.iter_mut()
    }
    
    /// 条件を満たす要素のインデックスを取得
    pub fn find_index<F>(&self, predicate: F) -> Option<usize>
    where
        F: FnMut(&T) -> bool,
    {
        self.inner.iter().position(predicate)
    }
    
    /// ベクターを結合
    pub fn concat(&mut self, other: &Vec<T>) 
    where
        T: Clone,
    {
        self.inner.extend_from_slice(&other.inner);
    }
    
    /// 要素がベクターに含まれているかチェック
    pub fn contains(&self, element: &T) -> bool
    where
        T: PartialEq,
    {
        self.inner.contains(element)
    }
    
    /// 内部ベクターへの可変参照を取得
    pub fn inner_mut(&mut self) -> &mut std::vec::Vec<T> {
        &mut self.inner
    }
    
    /// 内部ベクターへの参照を取得
    pub fn inner(&self) -> &std::vec::Vec<T> {
        &self.inner
    }
}

impl<T> Default for Vec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Debug> Debug for Vec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<T> Index<usize> for Vec<T> {
    type Output = T;
    
    fn index(&self, index: usize) -> &Self::Output {
        &self.inner[index]
    }
}

impl<T> IndexMut<usize> for Vec<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.inner[index]
    }
}

impl<T> From<std::vec::Vec<T>> for Vec<T> {
    fn from(vec: std::vec::Vec<T>) -> Self {
        Self { inner: vec }
    }
}

impl<T> Into<std::vec::Vec<T>> for Vec<T> {
    fn into(self) -> std::vec::Vec<T> {
        self.inner
    }
}

/// ハッシュマップ型
/// 
/// キーと値のペアを効率的に格納するハッシュテーブルを実装します。
#[derive(Clone, PartialEq, Eq)]
pub struct HashMap<K: Eq + Hash, V> {
    /// 内部データ
    inner: StdHashMap<K, V>,
}

impl<K, V> HashMap<K, V>
where
    K: Eq + Hash,
{
    /// 新しい空のハッシュマップを作成
    pub fn new() -> Self {
        Self {
            inner: StdHashMap::new(),
        }
    }
    
    /// 指定した容量で新しいハッシュマップを作成
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: StdHashMap::with_capacity(capacity),
        }
    }

    /// 要素を挿入
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.inner.insert(key, value)
    }
    
    /// キーに対応する値を取得
    pub fn get(&self, key: &K) -> Option<&V> {
        self.inner.get(key)
    }
    
    /// キーに対応する値を可変で取得
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.inner.get_mut(key)
    }
    
    /// キーに対応する要素を削除
    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.inner.remove(key)
    }
    
    /// マップの長さを取得
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    
    /// マップが空かどうかを確認
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    
    /// キーが存在するかどうかを確認
    pub fn contains_key(&self, key: &K) -> bool {
        self.inner.contains_key(key)
    }
    
    /// マップをクリア
    pub fn clear(&mut self) {
        self.inner.clear();
    }
    
    /// キーのイテレータを取得
    pub fn keys(&self) -> std::collections::hash_map::Keys<'_, K, V> {
        self.inner.keys()
    }
    
    /// 値のイテレータを取得
    pub fn values(&self) -> std::collections::hash_map::Values<'_, K, V> {
        self.inner.values()
    }
    
    /// 値の可変イテレータを取得
    pub fn values_mut(&mut self) -> std::collections::hash_map::ValuesMut<'_, K, V> {
        self.inner.values_mut()
    }
    
    /// エントリーのイテレータを取得
    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, K, V> {
        self.inner.iter()
    }
    
    /// エントリーの可変イテレータを取得
    pub fn iter_mut(&mut self) -> std::collections::hash_map::IterMut<'_, K, V> {
        self.inner.iter_mut()
    }
}

impl<K, V> Default for HashMap<K, V>
where
    K: Eq + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Debug + Eq + Hash, V: Debug> Debug for HashMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

/// ハッシュマップのエントリ
pub struct Entry<'a, K: Eq + Hash + Clone, V> {
    key: K,
    map: &'a mut HashMap<K, V>,
}

impl<'a, K: Eq + Hash + Clone, V> Entry<'a, K, V> {
    /// キーが存在しない場合に指定された値を挿入し、値への可変参照を返す
    pub fn or_insert_with<F>(self, default: F) -> &'a mut V
    where
        F: FnOnce() -> V,
    {
        let key_clone = self.key.clone();
        
        if !self.map.contains_key(&key_clone) {
            let value = default();
            self.map.insert(self.key, value);
        } else {
            // 存在する場合でも、キーをクローンしてゲットする必要があります
        }
        
        // キーが存在することが保証されているため、unwrap()しても安全
        self.map.get_mut(&key_clone).unwrap()
    }
    
    /// キーが存在しない場合にデフォルト値を挿入し、値への可変参照を返す
    pub fn or_insert(self, default: V) -> &'a mut V {
        self.or_insert_with(|| default)
    }
}

/// ハッシュセット型
/// 
/// 一意な要素を効率的に格納するハッシュテーブルベースのセットを実装します。
#[derive(Clone, PartialEq, Eq)]
pub struct HashSet<T: Eq + Hash> {
    /// 内部データ
    inner: StdHashSet<T>,
}

impl<T> HashSet<T>
where
    T: Eq + Hash,
{
    /// 新しい空のハッシュセットを作成
    pub fn new() -> Self {
        Self {
            inner: StdHashSet::new(),
        }
    }
    
    /// 指定した容量で新しいハッシュセットを作成
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: StdHashSet::with_capacity(capacity),
        }
    }
    
    /// 要素を挿入
    pub fn insert(&mut self, value: T) -> bool {
        self.inner.insert(value)
    }
    
    /// 要素が含まれているかどうかを確認
    pub fn contains(&self, value: &T) -> bool {
        self.inner.contains(value)
    }
    
    /// 要素を削除
    pub fn remove(&mut self, value: &T) -> bool {
        self.inner.remove(value)
    }
    
    /// セットの長さを取得
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    
    /// セットが空かどうかを確認
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    
    /// セットをクリア
    pub fn clear(&mut self) {
        self.inner.clear();
    }
    
    /// セットの要素をイテレート
    pub fn iter(&self) -> std::collections::hash_set::Iter<'_, T> {
        self.inner.iter()
    }
    
    /// 和集合を計算
    pub fn union(&self, other: &HashSet<T>) -> HashSet<T>
    where
        T: Clone,
    {
        let mut result = self.clone();
        for item in other.iter() {
            result.insert(item.clone());
        }
        result
    }
    
    /// 積集合を計算
    pub fn intersection(&self, other: &HashSet<T>) -> HashSet<T>
    where
        T: Clone,
    {
        let mut result = HashSet::new();
        for item in self.iter() {
            if other.contains(item) {
                result.insert(item.clone());
            }
        }
        result
    }
    
    /// 差集合を計算
    pub fn difference(&self, other: &HashSet<T>) -> HashSet<T>
    where
        T: Clone,
    {
        let mut result = HashSet::new();
        for item in self.iter() {
            if !other.contains(item) {
                result.insert(item.clone());
            }
        }
        result
    }
}

impl<T> Default for HashSet<T>
where
    T: Eq + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Debug + Eq + Hash> Debug for HashSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

/// Bツリーマップ型
/// 
/// キーの順序でソートされたマップを提供します。
#[derive(Clone, PartialEq, Eq)]
pub struct BTreeMap<K, V> {
    /// 内部データ
    inner: StdBTreeMap<K, V>,
}

impl<K, V> BTreeMap<K, V>
where
    K: Ord,
{
    /// 新しい空のBTreeMapを作成
    pub fn new() -> Self {
        Self {
            inner: StdBTreeMap::new(),
        }
    }
    
    /// 要素を挿入
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.inner.insert(key, value)
    }
    
    /// キーに対応する値を取得
    pub fn get(&self, key: &K) -> Option<&V> {
        self.inner.get(key)
    }
    
    /// キーに対応する値を可変で取得
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.inner.get_mut(key)
    }
    
    /// キーに対応する要素を削除
    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.inner.remove(key)
    }
    
    /// マップの長さを取得
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    
    /// マップが空かどうかを確認
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    
    /// キーが存在するかどうかを確認
    pub fn contains_key(&self, key: &K) -> bool {
        self.inner.contains_key(key)
    }
    
    /// マップをクリア
    pub fn clear(&mut self) {
        self.inner.clear();
    }
    
    /// キーのイテレータを取得
    pub fn keys(&self) -> std::collections::btree_map::Keys<'_, K, V> {
        self.inner.keys()
    }
    
    /// 値のイテレータを取得
    pub fn values(&self) -> std::collections::btree_map::Values<'_, K, V> {
        self.inner.values()
    }
    
    /// 値の可変イテレータを取得
    pub fn values_mut(&mut self) -> std::collections::btree_map::ValuesMut<'_, K, V> {
        self.inner.values_mut()
    }
    
    /// エントリーのイテレータを取得
    pub fn iter(&self) -> std::collections::btree_map::Iter<'_, K, V> {
        self.inner.iter()
    }
    
    /// エントリーの可変イテレータを取得
    pub fn iter_mut(&mut self) -> std::collections::btree_map::IterMut<'_, K, V> {
        self.inner.iter_mut()
    }
    
    /// 最小のキーを持つエントリを取得
    pub fn first_key_value(&self) -> Option<(&K, &V)> {
        self.inner.first_key_value()
    }
    
    /// 最大のキーを持つエントリを取得
    pub fn last_key_value(&self) -> Option<(&K, &V)> {
        self.inner.last_key_value()
    }
}

impl<K, V> Default for BTreeMap<K, V>
where
    K: Ord,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Debug, V: Debug> Debug for BTreeMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

/// Bツリーセット型
/// 
/// 要素の順序でソートされたセットを提供します。
#[derive(Clone, PartialEq, Eq)]
pub struct BTreeSet<T> {
    /// 内部データ
    inner: StdBTreeSet<T>,
}

impl<T> BTreeSet<T>
where
    T: Ord,
{
    /// 新しい空のBTreeSetを作成
    pub fn new() -> Self {
        Self {
            inner: StdBTreeSet::new(),
        }
    }
    
    /// 要素を挿入
    pub fn insert(&mut self, value: T) -> bool {
        self.inner.insert(value)
    }
    
    /// 要素が含まれているかどうかを確認
    pub fn contains(&self, value: &T) -> bool {
        self.inner.contains(value)
    }
    
    /// 要素を削除
    pub fn remove(&mut self, value: &T) -> bool {
        self.inner.remove(value)
    }
    
    /// セットの長さを取得
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    
    /// セットが空かどうかを確認
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    
    /// セットをクリア
    pub fn clear(&mut self) {
        self.inner.clear();
    }
    
    /// セットの要素をイテレート
    pub fn iter(&self) -> std::collections::btree_set::Iter<'_, T> {
        self.inner.iter()
    }
    
    /// 最小の要素を取得
    pub fn first(&self) -> Option<&T> {
        self.inner.first()
    }
    
    /// 最大の要素を取得
    pub fn last(&self) -> Option<&T> {
        self.inner.last()
    }
    
    /// 和集合を計算
    pub fn union(&self, other: &BTreeSet<T>) -> BTreeSet<T>
    where
        T: Clone,
    {
        let mut result = self.clone();
        for item in other.iter() {
            result.insert(item.clone());
        }
        result
    }
    
    /// 積集合を計算
    pub fn intersection(&self, other: &BTreeSet<T>) -> BTreeSet<T>
    where
        T: Clone,
    {
        let mut result = BTreeSet::new();
        for item in self.iter() {
            if other.contains(item) {
                result.insert(item.clone());
            }
        }
        result
    }
    
    /// 差集合を計算
    pub fn difference(&self, other: &BTreeSet<T>) -> BTreeSet<T>
    where
        T: Clone,
    {
        let mut result = BTreeSet::new();
        for item in self.iter() {
            if !other.contains(item) {
                result.insert(item.clone());
            }
        }
        result
    }
}

impl<T> Default for BTreeSet<T>
where
    T: Ord,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Debug + Ord> Debug for BTreeSet<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

// Clone制約が必要な機能を別のimplブロックで定義
impl<K, V> HashMap<K, V>
where
    K: Eq + Hash + Clone,
{
    /// キーに対して新しい値を挿入または既存の値を更新するためのエントリを取得
    pub fn entry(&mut self, key: K) -> Entry<K, V> {
        Entry {
            key,
            map: self,
        }
    }
}

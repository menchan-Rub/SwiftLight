// SwiftLight Type System - Traits
// トレイトシステム実装

//! # SwiftLight型システム - トレイト
//! 
//! このモジュールでは、SwiftLight言語のトレイトシステムを実装します。
//! トレイトは型の振る舞いを定義するインターフェースであり、
//! 多相性とコードの再利用を可能にする重要な機能です。
//!
//! 主な機能:
//! - トレイト定義と実装
//! - トレイト境界と型制約
//! - 関連型と関連定数
//! - デフォルト実装
//! - トレイト継承
//! - トレイト特殊化
//! - 自動実装（derive）

use std::collections::{HashMap, HashSet, BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;
use std::cell::RefCell;
use std::rc::Rc;

// 同一モジュール内の型をインポート
use super::{Type, TypeId, TypeError, TraitBound, TypeRegistry};
use super::types::{TypeDefinition, MethodSignature, MethodDefinition, TypeFlags, Visibility};

// 現時点では未実装のモジュールからのインポートをコメントアウト
// 必要に応じて実装後に有効化する
// use crate::frontend::source_map::SourceLocation;
// use crate::frontend::error::{Result, ErrorKind};
// use crate::frontend::ast;

// 一時的な型定義（コンパイルエラー回避用）
pub type SourceLocation = (usize, usize);
pub type Result<T> = std::result::Result<T, String>;
pub enum ErrorKind {
    TypeError,
    ParseError,
    CompileError,
}

/// トレイト定義
#[derive(Debug, Clone)]
pub struct TraitDefinition {
    /// トレイトの名前
    pub name: String,
    
    /// トレイトのID
    pub id: TypeId,
    
    /// トレイトが定義されているモジュールパス
    pub module_path: Vec<String>,
    
    /// トレイトの可視性
    pub visibility: Visibility,
    
    /// トレイトのジェネリックパラメータ
    pub generic_params: Vec<super::types::GenericParamDefinition>,
    
    /// スーパートレイト（継承するトレイト）
    pub super_traits: Vec<TraitBound>,
    
    /// トレイトメソッド
    pub methods: HashMap<String, MethodSignature>,
    
    /// デフォルト実装を持つメソッド
    pub default_methods: HashMap<String, MethodDefinition>,
    
    /// 関連型
    pub associated_types: HashMap<String, super::types::AssociatedTypeDefinition>,
    
    /// 関連定数
    pub associated_constants: HashMap<String, AssociatedConstantDefinition>,
    
    /// トレイトのマーカー属性
    pub marker: bool,
    
    /// トレイトの自動導出可能性
    pub auto_derivable: bool,
    
    /// トレイトの安全性フラグ
    pub is_unsafe: bool,
    
    /// トレイトの条件付き実装制約
    pub where_clauses: Vec<TraitWhereClause>,
    
    /// トレイトのドキュメントコメント
    pub doc_comment: Option<String>,
    
    /// トレイトの定義位置
    pub location: SourceLocation,
    
    /// トレイトのメタデータ
    pub metadata: TraitMetadata,
}

/// トレイト実装
#[derive(Debug, Clone)]
pub struct TraitImplementation {
    /// 実装されるトレイト
    pub trait_id: TypeId,
    
    /// 実装する型
    pub for_type: TypeId,
    
    /// 型パラメータ（ジェネリック実装の場合）
    pub type_params: Vec<TypeId>,
    
    /// 関連型の実装
    pub associated_types: HashMap<String, TypeId>,
    
    /// 関連定数の実装
    pub associated_constants: HashMap<String, ConstantValue>,
    
    /// メソッドの実装
    pub methods: HashMap<String, MethodImplementation>,
    
    /// 条件付き実装の制約
    pub where_clauses: Vec<TraitWhereClause>,
    
    /// 実装の安全性フラグ
    pub is_unsafe: bool,
    
    /// 実装の自動導出フラグ
    pub is_derived: bool,
    
    /// 実装の定義位置
    pub location: SourceLocation,
    
    /// 実装のメタデータ
    pub metadata: ImplementationMetadata,
}

/// メソッド実装
#[derive(Debug, Clone)]
pub struct MethodImplementation {
    /// メソッド名
    pub name: String,
    
    /// メソッドシグネチャ
    pub signature: MethodSignature,
    
    /// メソッド本体（実装）
    pub body: Option<Arc<dyn std::any::Any + Send + Sync>>,
    
    /// 実装の定義位置
    pub location: SourceLocation,
    
    /// 条件付き実装のための条件
    pub conditions: Vec<ImplementationCondition>,
}

/// メソッド実装のメタデータ
#[derive(Debug, Clone, Default)]
pub struct MethodMetadata {
    /// デフォルト実装かどうか
    pub is_default_implementation: bool,
    
    /// 最適化レベル
    pub optimization_level: OptimizationLevel,
    
    /// 安全性チェック
    pub safety_checks: SafetyChecks,
    
    /// プラットフォーム特有のヒント
    pub platform_hints: Vec<String>,
    
    /// パフォーマンス上重要かどうか
    pub performance_critical: bool,
    
    /// 実行コンテキスト
    pub execution_context: ExecutionContext,
}

/// 実装条件
#[derive(Debug, Clone)]
pub enum ImplementationCondition {
    /// トレイト境界条件
    TraitBound(TraitBound),
    
    /// 型等価性条件
    TypeEquals(TypeId, TypeId),
    
    /// カスタム条件
    Custom(String),
}

/// 最適化レベル
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OptimizationLevel {
    /// デバッグ（最適化なし）
    Debug,
    
    /// 通常の最適化
    #[default]
    Normal,
    
    /// 積極的な最適化
    Aggressive,
}

/// 安全性チェック
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SafetyChecks {
    /// すべてのチェックを実行
    #[default]
    Full,
    
    /// 基本的なチェックのみ
    Basic,
    
    /// チェックなし（unsafe）
    None,
}

/// 実行コンテキスト
#[derive(Debug, Clone, Default)]
pub enum ExecutionContext {
    /// 通常のコンテキスト
    #[default]
    Normal,
    
    /// 非同期コンテキスト
    Async,
    
    /// 並列コンテキスト
    Parallel,
    
    /// システムレベルコンテキスト
    System,
}

impl MethodMetadata {
    /// 新しいデフォルトのメタデータを作成
    pub fn new() -> Self {
        Self::default()
    }
}

/// 関連定数の定義
#[derive(Debug, Clone)]
pub struct AssociatedConstantDefinition {
    /// 定数名
    pub name: String,
    
    /// 定数の型
    pub type_id: TypeId,
    
    /// デフォルト値（オプション）
    pub default_value: Option<ConstantValue>,
    
    /// 定数の可視性
    pub visibility: Visibility,
    
    /// 定数のドキュメントコメント
    pub doc_comment: Option<String>,
    
    /// 定数の定義位置
    pub location: SourceLocation,
}

/// 定数値
#[derive(Debug, Clone)]
pub enum ConstantValue {
    /// 整数値
    Integer(i64),
    
    /// 浮動小数点値
    Float(f64),
    
    /// 文字列値
    String(String),
    
    /// ブール値
    Boolean(bool),
    
    /// 文字値
    Char(char),
    
    /// 配列値
    Array(Vec<ConstantValue>),
    
    /// タプル値
    Tuple(Vec<ConstantValue>),
    
    /// 構造体値
    Struct {
        name: String,
        fields: HashMap<String, ConstantValue>,
    },
    
    /// 列挙型値
    Enum {
        name: String,
        variant: String,
        payload: Option<Box<ConstantValue>>,
    },
    
    /// 単位値（Unit）
    Unit,
    
    /// 型
    Type(TypeId),
    
    /// 複合式（コンパイル時計算が必要なもの）
    ComputedExpression(Arc<dyn std::any::Any + Send + Sync>),
}

/// トレイトのWhere句
#[derive(Debug, Clone)]
pub struct TraitWhereClause {
    /// 制約が適用される型
    pub type_id: TypeId,
    
    /// 型に適用される制約
    pub bounds: Vec<TraitBound>,
    
    /// 制約の定義位置
    pub location: SourceLocation,
}

/// トレイトのメタデータ
#[derive(Debug, Clone, Default)]
pub struct TraitMetadata {
    /// トレイトの安定性レベル
    pub stability: super::types::StabilityLevel,
    
    /// トレイトの非推奨フラグ
    pub deprecated: Option<String>,
    
    /// トレイトの実験的フラグ
    pub experimental: bool,
    
    /// カスタムメタデータ
    pub custom: HashMap<String, String>,
}

/// 実装のメタデータ
#[derive(Debug, Clone, Default)]
pub struct ImplementationMetadata {
    /// 実装の安定性レベル
    pub stability: super::types::StabilityLevel,
    
    /// 実装の非推奨フラグ
    pub deprecated: Option<String>,
    
    /// 実装の実験的フラグ
    pub experimental: bool,
    
    /// カスタムメタデータ
    pub custom: HashMap<String, String>,
}

/// トレイトコヒーレンスチェッカー
/// トレイト実装の一貫性を確保するためのコンポーネント
pub struct TraitCoherenceChecker {
    /// すべてのトレイト実装のレジストリ
    implementations: HashMap<TypeId, Vec<TraitImplementation>>,
    
    /// オーファン実装のレジストリ
    orphan_implementations: HashMap<TypeId, Vec<TraitImplementation>>,
    
    /// トレイト定義のレジストリ
    trait_definitions: HashMap<TypeId, TraitDefinition>,
    
    /// 型レジストリへの参照
    type_registry: Arc<RefCell<TypeRegistry>>,
}

impl TraitCoherenceChecker {
    /// 新しいトレイトコヒーレンスチェッカーを作成
    pub fn new(type_registry: Arc<RefCell<TypeRegistry>>) -> Self {
        TraitCoherenceChecker {
            implementations: HashMap::new(),
            orphan_implementations: HashMap::new(),
            trait_definitions: HashMap::new(),
            type_registry,
        }
    }
    
    /// トレイト定義を登録
    pub fn register_trait_definition(&mut self, definition: TraitDefinition) -> Result<()> {
        if self.trait_definitions.contains_key(&definition.id) {
            return Err(ErrorKind::DuplicateDefinition(
                format!("トレイト '{}' は既に定義されています", definition.name),
                definition.location,
            ).into());
        }
        
        self.trait_definitions.insert(definition.id, definition);
        Ok(())
    }
    
    /// トレイト実装を登録
    pub fn register_implementation(&mut self, implementation: TraitImplementation) -> Result<()> {
        // トレイト定義が存在するか確認
        if !self.trait_definitions.contains_key(&implementation.trait_id) {
            return Err(ErrorKind::UndefinedType(
                format!("実装されるトレイトが定義されていません: {:?}", implementation.trait_id),
                implementation.location,
            ).into());
        }
        
        // オーファン実装ルールをチェック
        let is_orphan = self.is_orphan_implementation(&implementation);
        
        // 既存の実装と矛盾しないか確認（コヒーレンスチェック）
        self.check_coherence(&implementation)?;
        
        // 実装をレジストリに追加
        if is_orphan {
            let impls = self.orphan_implementations
                .entry(implementation.trait_id)
                .or_insert_with(Vec::new);
            impls.push(implementation);
        } else {
            let impls = self.implementations
                .entry(implementation.trait_id)
                .or_insert_with(Vec::new);
            impls.push(implementation);
        }
        
        Ok(())
    }
    
    /// 実装がオーファン実装かどうかをチェック
    fn is_orphan_implementation(&self, implementation: &TraitImplementation) -> bool {
        // オーファン実装ルール：
        // - トレイトが現在のクレートで定義されていない、かつ
        // - 実装対象の型が現在のクレートで定義されていない
        // この場合、オーファン実装となる
        
        // 簡略化のため、常にfalseを返す実装
        // 実際には型やトレイトの定義元クレートの情報を使用する必要がある
        false
    }
    
    /// 実装のコヒーレンスをチェック
    fn check_coherence(&self, implementation: &TraitImplementation) -> Result<()> {
        // 既存の実装と重複しないことを確認
        if let Some(impls) = self.implementations.get(&implementation.trait_id) {
            for existing_impl in impls {
                if self.implementations_overlap(existing_impl, implementation) {
                    return Err(ErrorKind::TypeCheckError(
                        format!("トレイト実装が既存の実装と重複しています"),
                        implementation.location,
                    ).into());
                }
            }
        }
        
        // オーファン実装についても同様にチェック
        if let Some(impls) = self.orphan_implementations.get(&implementation.trait_id) {
            for existing_impl in impls {
                if self.implementations_overlap(existing_impl, implementation) {
                    return Err(ErrorKind::TypeCheckError(
                        format!("トレイト実装が既存のオーファン実装と重複しています"),
                        implementation.location,
                    ).into());
                }
            }
        }
        
        Ok(())
    }
    
    /// 2つの実装が重複するかどうかをチェック
    fn implementations_overlap(&self, impl1: &TraitImplementation, impl2: &TraitImplementation) -> bool {
        // 同じ型に対する同じトレイトの実装は重複
        if impl1.for_type == impl2.for_type && impl1.trait_id == impl2.trait_id {
            return true;
        }
        
        // 詳細なチェックは型の特殊化関係なども考慮する必要がある
        // 型パラメータと where 句も考慮した詳細な実装が必要
        
        false
    }
    
    /// 型がトレイトを実装しているかチェック
    pub fn check_trait_implemented(&self, type_id: TypeId, trait_id: TypeId) -> bool {
        if let Some(impls) = self.implementations.get(&trait_id) {
            for implementation in impls {
                if implementation.for_type == type_id {
                    return true;
                }
            }
        }
        
        if let Some(impls) = self.orphan_implementations.get(&trait_id) {
            for implementation in impls {
                if implementation.for_type == type_id {
                    return true;
                }
            }
        }
        
        false
    }
}

/// トレイトの自動導出
pub struct TraitDeriver {
    /// 導出可能なトレイトの登録
    derivable_traits: HashMap<String, Box<dyn TraitDerivation>>,
    
    /// 型レジストリへの参照
    type_registry: Arc<RefCell<TypeRegistry>>,
}

impl TraitDeriver {
    /// 新しいトレイト導出機能を作成
    pub fn new(type_registry: Arc<RefCell<TypeRegistry>>) -> Self {
        let mut deriver = TraitDeriver {
            derivable_traits: HashMap::new(),
            type_registry,
        };
        
        // 標準の導出可能トレイトを登録
        deriver.register_standard_derivations();
        
        deriver
    }
    
    /// 標準の導出可能トレイトを登録
    fn register_standard_derivations(&mut self) {
        // 例: Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash などの実装を登録
        // self.register_derivation("Clone", Box::new(CloneDerivation {}));
        // 詳細な実装はここでは省略
    }
    
    /// カスタム導出を登録
    pub fn register_derivation(&mut self, trait_name: &str, derivation: Box<dyn TraitDerivation>) {
        self.derivable_traits.insert(trait_name.to_string(), derivation);
    }
    
    /// トレイトを導出
    pub fn derive_trait(&self, trait_name: &str, type_def: &TypeDefinition) -> Result<TraitImplementation> {
        if let Some(derivation) = self.derivable_traits.get(trait_name) {
            derivation.derive(type_def, &self.type_registry)
        } else {
            Err(ErrorKind::TypeCheckError(
                format!("トレイト '{}' は自動導出できません", trait_name),
                type_def.location,
            ).into())
        }
    }
}

/// トレイト導出のトレイト
pub trait TraitDerivation: Send + Sync {
    /// トレイトを導出する
    fn derive(&self, type_def: &TypeDefinition, type_registry: &Arc<RefCell<TypeRegistry>>) -> Result<TraitImplementation>;
}

/// トレイト解決機能
/// 型がトレイトを実装しているかを解決するためのコンポーネント
pub struct TraitResolver {
    /// コヒーレンスチェッカーへの参照
    coherence_checker: Arc<RefCell<TraitCoherenceChecker>>,
    
    /// 型レジストリへの参照
    type_registry: Arc<RefCell<TypeRegistry>>,
    
    /// 解決キャッシュ
    cache: HashMap<(TypeId, TypeId), bool>,
}

impl TraitResolver {
    /// 新しいトレイト解決機能を作成
    pub fn new(
        coherence_checker: Arc<RefCell<TraitCoherenceChecker>>,
        type_registry: Arc<RefCell<TypeRegistry>>,
    ) -> Self {
        TraitResolver {
            coherence_checker,
            type_registry,
            cache: HashMap::new(),
        }
    }
    
    /// 型がトレイトを実装しているかを解決
    pub fn resolve(&mut self, type_id: TypeId, trait_id: TypeId) -> Result<bool> {
        // キャッシュをチェック
        if let Some(&result) = self.cache.get(&(type_id, trait_id)) {
            return Ok(result);
        }
        
        // 直接的な実装をチェック
        let direct_impl = self.coherence_checker.borrow().check_trait_implemented(type_id, trait_id);
        if direct_impl {
            self.cache.insert((type_id, trait_id), true);
            return Ok(true);
        }
        
        // 間接的な実装（スーパートレイトを通じた実装）をチェック
        let trait_def_opt = self.get_trait_definition(trait_id);
        if let Some(trait_def) = trait_def_opt {
            for super_trait in &trait_def.super_traits {
                if self.resolve(type_id, super_trait.trait_id)? {
                    self.cache.insert((type_id, trait_id), true);
                    return Ok(true);
                }
            }
        }
        
        // 実装が見つからない
        self.cache.insert((type_id, trait_id), false);
        Ok(false)
    }
    
    /// トレイト定義を取得
    fn get_trait_definition(&self, trait_id: TypeId) -> Option<TraitDefinition> {
        self.coherence_checker.borrow().trait_definitions.get(&trait_id).cloned()
    }
    
    /// キャッシュをクリア
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

/// トレイト特殊化マネージャー
/// 特定の型に対するトレイト実装の特殊化を管理するコンポーネント
pub struct TraitSpecializationManager {
    /// 特殊化されたトレイト実装
    specialized_implementations: HashMap<(TypeId, TypeId), Vec<SpecializedImplementation>>,
    
    /// 特殊化の優先度情報
    specialization_priorities: HashMap<TypeId, i32>,
    
    /// 型レジストリへの参照
    type_registry: Arc<RefCell<TypeRegistry>>,
    
    /// トレイト解決機能への参照
    resolver: Arc<RefCell<TraitResolver>>,
}

/// 特殊化された実装
#[derive(Debug, Clone)]
pub struct SpecializedImplementation {
    /// 特殊化対象のトレイト実装
    pub implementation: TraitImplementation,
    
    /// 特殊化の優先度（高いほど優先される）
    pub priority: i32,
    
    /// 特殊化の条件（None の場合は無条件）
    pub condition: Option<SpecializationCondition>,
}

/// 特殊化条件
#[derive(Debug, Clone)]
pub enum SpecializationCondition {
    /// トレイト境界による条件
    TraitBound(TraitBound),
    
    /// 型等価性による条件
    TypeEquals(TypeId),
    
    /// 型クラスによる条件
    TypeClass {
        class_id: TypeId,
        parameters: Vec<TypeId>,
    },
    
    /// 論理積条件（AND）
    And(Vec<SpecializationCondition>),
    
    /// 論理和条件（OR）
    Or(Vec<SpecializationCondition>),
    
    /// 否定条件（NOT）
    Not(Box<SpecializationCondition>),
}

impl TraitSpecializationManager {
    /// 新しい特殊化マネージャーを作成
    pub fn new(
        type_registry: Arc<RefCell<TypeRegistry>>,
        resolver: Arc<RefCell<TraitResolver>>,
    ) -> Self {
        TraitSpecializationManager {
            specialized_implementations: HashMap::new(),
            specialization_priorities: HashMap::new(),
            type_registry,
            resolver,
        }
    }
    
    /// 特殊化実装を登録
    pub fn register_specialized_implementation(
        &mut self,
        implementation: TraitImplementation,
        priority: i32,
        condition: Option<SpecializationCondition>,
    ) -> Result<()> {
        let key = (implementation.trait_id, implementation.for_type);
        
        let specialized_impl = SpecializedImplementation {
            implementation,
            priority,
            condition,
        };
        
        let impls = self.specialized_implementations
            .entry(key)
            .or_insert_with(Vec::new);
        
        // 優先度に基づいて挿入位置を決定（降順）
        let insert_pos = impls.binary_search_by(|probe| {
            specialized_impl.priority.cmp(&probe.priority).reverse()
        }).unwrap_or_else(|e| e);
        
        impls.insert(insert_pos, specialized_impl);
        
        Ok(())
    }
    
    /// 型とトレイトに対して最も特殊化された実装を取得
    pub fn get_most_specialized_implementation(
        &self,
        type_id: TypeId,
        trait_id: TypeId,
    ) -> Option<&TraitImplementation> {
        let key = (trait_id, type_id);
        
        if let Some(impls) = self.specialized_implementations.get(&key) {
            for specialized_impl in impls {
                if self.condition_satisfied(&specialized_impl.condition, type_id) {
                    return Some(&specialized_impl.implementation);
                }
            }
        }
        
        None
    }
    
    /// 特殊化条件が満たされているかを評価
    fn condition_satisfied(&self, condition: &Option<SpecializationCondition>, type_id: TypeId) -> bool {
        match condition {
            None => true, // 条件がない場合は常に満たされる
            Some(cond) => self.evaluate_condition(cond, type_id),
        }
    }
    
    /// 特殊化条件を評価
    fn evaluate_condition(&self, condition: &SpecializationCondition, type_id: TypeId) -> bool {
        match condition {
            SpecializationCondition::TraitBound(trait_bound) => {
                // トレイト境界が満たされているかをチェック
                let mut resolver = self.resolver.borrow_mut();
                resolver.resolve(type_id, trait_bound.trait_id).unwrap_or(false)
            },
            
            SpecializationCondition::TypeEquals(other_type_id) => {
                // 型が等しいかをチェック
                type_id == *other_type_id
            },
            
            SpecializationCondition::TypeClass { class_id, parameters } => {
                // 型クラスの条件は現状実装なし
                // 実際には型クラスのメンバーシップチェックなどが必要
                false
            },
            
            SpecializationCondition::And(conditions) => {
                // 全ての条件が満たされる必要がある
                conditions.iter().all(|c| self.evaluate_condition(c, type_id))
            },
            
            SpecializationCondition::Or(conditions) => {
                // いずれかの条件が満たされれば良い
                conditions.iter().any(|c| self.evaluate_condition(c, type_id))
            },
            
            SpecializationCondition::Not(condition) => {
                // 条件の否定
                !self.evaluate_condition(condition, type_id)
            },
        }
    }
    
    /// トレイト実装の特殊化優先度を設定
    pub fn set_specialization_priority(&mut self, trait_id: TypeId, priority: i32) {
        self.specialization_priorities.insert(trait_id, priority);
    }
    
    /// トレイト実装の特殊化優先度を取得
    pub fn get_specialization_priority(&self, trait_id: TypeId) -> i32 {
        *self.specialization_priorities.get(&trait_id).unwrap_or(&0)
    }
}

/// トレイトの実装を構築するためのビルダー
pub struct TraitImplementationBuilder {
    /// 実装対象のトレイト
    trait_id: TypeId,
    
    /// 実装する型
    for_type: TypeId,
    
    /// 型パラメータ
    type_params: Vec<TypeId>,
    
    /// 関連型の実装
    associated_types: HashMap<String, TypeId>,
    
    /// 関連定数の実装
    associated_constants: HashMap<String, ConstantValue>,
    
    /// メソッドの実装
    methods: HashMap<String, MethodImplementation>,
    
    /// 条件付き実装の制約
    where_clauses: Vec<TraitWhereClause>,
    
    /// 実装の安全性フラグ
    is_unsafe: bool,
    
    /// 実装の自動導出フラグ
    is_derived: bool,
    
    /// 実装の定義位置
    location: SourceLocation,
    
    /// 実装のメタデータ
    metadata: ImplementationMetadata,
    
    /// 型レジストリへの参照
    type_registry: Arc<RefCell<TypeRegistry>>,
    
    /// トレイト定義への参照
    trait_definition: Option<TraitDefinition>,
}

impl TraitImplementationBuilder {
    /// 新しいトレイト実装ビルダーを作成
    pub fn new(
        trait_id: TypeId,
        for_type: TypeId,
        type_registry: Arc<RefCell<TypeRegistry>>,
        location: SourceLocation,
    ) -> Self {
        TraitImplementationBuilder {
            trait_id,
            for_type,
            type_params: Vec::new(),
            associated_types: HashMap::new(),
            associated_constants: HashMap::new(),
            methods: HashMap::new(),
            where_clauses: Vec::new(),
            is_unsafe: false,
            is_derived: false,
            location,
            metadata: ImplementationMetadata::default(),
            type_registry,
            trait_definition: None,
        }
    }
    
    /// トレイト定義を設定
    pub fn with_trait_definition(mut self, trait_definition: TraitDefinition) -> Self {
        self.trait_definition = Some(trait_definition);
        self
    }
    
    /// 型パラメータを追加
    pub fn add_type_param(mut self, param: TypeId) -> Self {
        self.type_params.push(param);
        self
    }
    
    /// 型パラメータを設定
    pub fn with_type_params(mut self, params: Vec<TypeId>) -> Self {
        self.type_params = params;
        self
    }
    
    /// 関連型の実装を追加
    pub fn add_associated_type(mut self, name: &str, type_id: TypeId) -> Self {
        self.associated_types.insert(name.to_string(), type_id);
        self
    }
    
    /// 関連定数の実装を追加
    pub fn add_associated_constant(mut self, name: &str, value: ConstantValue) -> Self {
        self.associated_constants.insert(name.to_string(), value);
        self
    }
    
    /// メソッドの実装を追加
    pub fn add_method(mut self, method: MethodImplementation) -> Self {
        self.methods.insert(method.name.clone(), method);
        self
    }
    
    /// Where句を追加
    pub fn add_where_clause(mut self, clause: TraitWhereClause) -> Self {
        self.where_clauses.push(clause);
        self
    }
    
    /// 安全でない実装として設定
    pub fn unsafe_impl(mut self) -> Self {
        self.is_unsafe = true;
        self
    }
    
    /// 自動導出された実装として設定
    pub fn derived(mut self) -> Self {
        self.is_derived = true;
        self
    }
    
    /// メタデータを設定
    pub fn with_metadata(mut self, metadata: ImplementationMetadata) -> Self {
        self.metadata = metadata;
        self
    }
    
    /// デフォルト実装からメソッドを埋める
    /// 
    /// トレイト定義に含まれるデフォルト実装を使用して、まだ実装されていないメソッドを自動的に埋めます。
    /// このプロセスでは以下の高度な処理を行います：
    /// - 型パラメータの適切な置換
    /// - 関連型の解決
    /// - コンテキスト依存の最適化
    /// - 特殊化された実装の選択
    /// - 条件付き実装の評価
    pub fn fill_default_methods(mut self) -> Result<Self> {
        if let Some(trait_def) = &self.trait_definition {
            let type_context = TypeSubstitutionContext::new()
                .with_self_type(self.for_type.clone())
                .with_type_params(&self.type_params)
                .with_associated_types(&self.associated_types);
            
            // デフォルトメソッドがあり、まだ実装されていないメソッドに適用
            for (name, default_method) in &trait_def.default_methods {
                if !self.methods.contains_key(name) {
                    // 型パラメータと関連型の置換を行う
                    let substituted_signature = type_context.substitute_signature(&default_method.signature)?;
                    let substituted_body = type_context.substitute_body(&default_method.implementation)?;
                    
                    // 実行コンテキストに基づいた最適化
                    let optimized_body = if self.metadata.enable_optimizations {
                        self.optimize_method_body(substituted_body, &substituted_signature)?
                    } else {
                        substituted_body
                    };
                    
                    // 特殊化されたプラットフォーム固有の実装があれば選択
                    let final_body = self.select_specialized_implementation(
                        name, 
                        optimized_body, 
                        &trait_def.specialized_implementations
                    )?;
                    
                    // メソッド実装の構築
                    let method_impl = MethodImplementation {
                        name: name.clone(),
                        signature: substituted_signature,
                        body: final_body,
                        location: self.location.clone(),
                        conditions: Vec::new(),
                    };
                    
                    // 条件付き実装の評価
                    if self.evaluate_conditional_implementation(&method_impl)? {
                        self.methods.insert(name.clone(), method_impl);
                        
                        // 依存メソッドの自動追加
                        if let Some(dependencies) = trait_def.method_dependencies.get(name) {
                            for dep_name in dependencies {
                                if !self.methods.contains_key(dep_name) && trait_def.default_methods.contains_key(dep_name) {
                                    // 再帰的に依存メソッドも追加（無限ループ防止のため深さ制限を設ける）
                                    self = self.add_dependent_method(dep_name, trait_def, &type_context, 0)?;
                                }
                            }
                        }
                    }
                }
            }
            
            // デフォルト実装の整合性検証
            self.validate_default_implementations(trait_def)?;
            
            // 実装されたメソッド間の相互作用を最適化
            if self.metadata.enable_advanced_optimizations {
                self = self.optimize_method_interactions()?;
            }
        }
        
        Ok(self)
    }
    
    /// 依存メソッドを追加する（再帰呼び出し用ヘルパーメソッド）
    fn add_dependent_method(
        mut self, 
        method_name: &str, 
        trait_def: &TraitDefinition, 
        type_context: &TypeSubstitutionContext,
        depth: usize
    ) -> Result<Self> {
        // 再帰の深さ制限（無限ループ防止）
        const MAX_RECURSION_DEPTH: usize = 10;
        if depth > MAX_RECURSION_DEPTH {
            return Err(ErrorKind::TypeSystemError(
                format!("メソッド依存関係の再帰が深すぎます: '{}'", method_name),
                self.location.clone()
            ).into());
        }
        
        if let Some(default_method) = trait_def.default_methods.get(method_name) {
            let substituted_signature = type_context.substitute_signature(&default_method.signature)?;
            let substituted_body = type_context.substitute_body(&default_method.implementation)?;
            
            let method_impl = MethodImplementation {
                name: method_name.to_string(),
                signature: substituted_signature,
                body: substituted_body,
                location: self.location.clone(),
                conditions: Vec::new(),
            };
            
            self.methods.insert(method_name.to_string(), method_impl);
            
            // さらに依存関係があれば再帰的に追加
            if let Some(dependencies) = trait_def.method_dependencies.get(method_name) {
                for dep_name in dependencies {
                    if !self.methods.contains_key(dep_name) && trait_def.default_methods.contains_key(dep_name) {
                        self = self.add_dependent_method(dep_name, trait_def, type_context, depth + 1)?;
                    }
                }
            }
        }
        
        Ok(self)
    }
    
    /// メソッド本体を最適化する
    fn optimize_method_body(&self, body: MethodBody, signature: &MethodSignature) -> Result<MethodBody> {
        // 最適化レベルに応じた処理
        match self.metadata.optimization_level {
            OptimizationLevel::None => Ok(body),
            OptimizationLevel::Basic => {
                // 基本的な最適化（定数畳み込み、不要なコード除去など）
                let optimizer = BasicMethodOptimizer::new(self.for_type.clone());
                optimizer.optimize(body, signature)
            },
            OptimizationLevel::Advanced => {
                // 高度な最適化（インライン化、ループ最適化など）
                let mut optimizer = AdvancedMethodOptimizer::new(
                    self.for_type.clone(),
                    &self.type_params,
                    &self.associated_types
                );
                optimizer.set_target_platform(&self.metadata.platform_hints);
                optimizer.optimize(body, signature)
            },
            OptimizationLevel::Aggressive => {
                // 積極的な最適化（特殊化、SIMD自動ベクトル化など）
                let mut optimizer = AggressiveMethodOptimizer::new(
                    self.for_type.clone(),
                    &self.type_params,
                    &self.associated_types,
                    &self.metadata
                );
                optimizer.set_execution_context(&self.metadata.execution_context);
                optimizer.optimize(body, signature)
            }
        }
    }
    
    /// 特殊化された実装を選択する
    fn select_specialized_implementation(
        &self,
        method_name: &str,
        default_body: MethodBody,
        specialized_impls: &HashMap<String, Vec<SpecializedMethodImplementation>>
    ) -> Result<MethodBody> {
        if let Some(specializations) = specialized_impls.get(method_name) {
            // 現在の実行コンテキストに最適な特殊化を選択
            for spec in specializations {
                if self.matches_specialization_criteria(spec) {
                    // 型パラメータを置換した特殊化実装を返す
                    let type_context = TypeSubstitutionContext::new()
                        .with_self_type(self.for_type.clone())
                        .with_type_params(&self.type_params);
                    
                    return type_context.substitute_body(&spec.implementation);
                }
            }
        }
        
        // 適切な特殊化が見つからなければデフォルト実装を使用
        Ok(default_body)
    }
    
    /// 特殊化条件に一致するかを評価
    fn matches_specialization_criteria(&self, spec: &SpecializedMethodImplementation) -> bool {
        // プラットフォーム条件の評価
        let platform_match = match &spec.platform {
            Some(platform) => self.metadata.platform_hints.contains(platform),
            None => true
        };
        
        // ハードウェア機能の評価
        let hardware_match = match &spec.hardware_features {
            Some(features) => {
                features.iter().all(|feature| {
                    self.metadata.available_hardware_features.contains(feature)
                })
            },
            None => true
        };
        
        // 型条件の評価
        let type_match = match &spec.type_constraints {
            Some(constraints) => {
                // 型制約の評価ロジック（簡略化）
                true // 実際には型制約を評価するコードが必要
            },
            None => true
        };
        
        platform_match && hardware_match && type_match
    }
    
    /// 条件付き実装を評価する
    /// コンパイル時条件や実行環境に基づいて条件付き実装が適用可能かを評価する
    fn evaluate_conditional_implementation(&self, method: &MethodImplementation) -> Result<bool> {
        // 条件がない場合は常に適用可能
        if method.conditions.is_empty() {
            return Ok(true);
        }
        
        // 各条件を評価
        for condition in &method.conditions {
            match condition {
                ImplementationCondition::CompileTimeExpr(expr) => {
                    // コンパイル時式の評価
                    let type_context = TypeSubstitutionContext::new()
                        .with_self_type(self.for_type.clone())
                        .with_type_params(&self.type_params);
                    
                    let evaluated_expr = self.metadata.compiler_context.evaluate_const_expr(
                        expr, 
                        &type_context,
                        &self.metadata.compile_time_constants
                    )?;
                    
                    // 評価結果がfalseなら条件を満たさない
                    if !evaluated_expr.as_bool()? {
                        return Ok(false);
                    }
                },
                ImplementationCondition::TypePredicate(predicate) => {
                    // 型述語の評価
                    let resolver = TypePredicateResolver::new(
                        &self.for_type,
                        &self.type_params,
                        &self.metadata.type_registry
                    );
                    
                    if !resolver.evaluate_predicate(predicate)? {
                        return Ok(false);
                    }
                },
                ImplementationCondition::RuntimeFeature(feature) => {
                    // 実行時機能の評価
                    if !self.metadata.runtime_features.supports_feature(feature) {
                        return Ok(false);
                    }
                },
                ImplementationCondition::MemoryModel(model) => {
                    // メモリモデルの評価
                    if !self.metadata.memory_model_compatibility.is_compatible_with(model) {
                        return Ok(false);
                    }
                },
                ImplementationCondition::Conjunction(subconditions) => {
                    // 全ての条件がtrueである必要がある
                    let mut temp_impl = self.clone();
                    for subcond in subconditions {
                        let method_with_condition = MethodImplementation {
                            conditions: vec![subcond.clone()],
                            ..method.clone()
                        };
                        
                        if !temp_impl.evaluate_conditional_implementation(&method_with_condition)? {
                            return Ok(false);
                        }
                    }
                },
                ImplementationCondition::Disjunction(subconditions) => {
                    // 少なくとも1つの条件がtrueである必要がある
                    let mut any_true = false;
                    let mut temp_impl = self.clone();
                    
                    for subcond in subconditions {
                        let method_with_condition = MethodImplementation {
                            conditions: vec![subcond.clone()],
                            ..method.clone()
                        };
                        
                        if temp_impl.evaluate_conditional_implementation(&method_with_condition)? {
                            any_true = true;
                            break;
                        }
                    }
                    
                    if !any_true {
                        return Ok(false);
                    }
                },
                ImplementationCondition::Negation(subcondition) => {
                    // 条件の否定
                    let method_with_condition = MethodImplementation {
                        conditions: vec![*subcondition.clone()],
                        ..method.clone()
                    };
                    
                    if self.evaluate_conditional_implementation(&method_with_condition)? {
                        return Ok(false);
                    }
                }
            }
        }
        
        // すべての条件を満たした
        Ok(true)
    }
    
    /// デフォルト実装の整合性を検証
    /// トレイト内のデフォルト実装間の整合性をチェックし、循環依存や型の不一致などを検出する
    fn validate_default_implementations(&self, trait_def: &TraitDefinition) -> Result<()> {
        // 依存グラフの構築
        let mut dependency_graph = DependencyGraph::new();
        
        // 各デフォルト実装をグラフに追加
        for (method_name, method_impl) in &trait_def.default_methods {
            dependency_graph.add_node(method_name.clone());
            
            // メソッド本体から依存関係を抽出
            let dependencies = self.extract_method_dependencies(&method_impl.body)?;
            
            // 依存関係をグラフに追加
            for dep in dependencies {
                if trait_def.methods.contains_key(&dep) {
                    dependency_graph.add_edge(method_name.clone(), dep);
                }
            }
        }
        
        // 循環依存のチェック
        if let Some(cycle) = dependency_graph.find_cycle() {
            return Err(ErrorKind::TypeCheckError(
                format!("デフォルト実装に循環依存が検出されました: {}", 
                    cycle.join(" -> ")),
                self.location.clone(),
            ).into());
        }
        
        // 型の整合性チェック
        for (method_name, method_impl) in &trait_def.default_methods {
            let method_sig = trait_def.methods.get(method_name)
                .ok_or_else(|| ErrorKind::InternalError(
                    format!("メソッド署名が見つかりません: {}", method_name)
                ))?;
            
            // 型コンテキストを作成
            let type_context = TypeSubstitutionContext::new()
                .with_self_type(self.for_type.clone())
                .with_type_params(&self.type_params);
            
            // メソッド本体の型チェック
            let body_type = self.metadata.type_checker.check_method_body(
                &method_impl.body,
                &type_context,
                &method_sig.return_type
            )?;
            
            // 戻り値の型が一致するか確認
            if !self.metadata.type_checker.is_subtype(&body_type, &method_sig.return_type)? {
                return Err(ErrorKind::TypeCheckError(
                    format!("メソッド '{}' のデフォルト実装の戻り値型 '{}' が期待される型 '{}' と一致しません",
                        method_name, body_type, method_sig.return_type),
                    method_impl.location.clone(),
                ).into());
            }
            
            // パラメータの使用法が正しいか確認
            self.validate_parameter_usage(&method_impl.body, &method_sig.parameters)?;
        }
        
        // 実装の網羅性チェック
        self.check_exhaustiveness(trait_def)?;
        
        // 特殊化の整合性チェック
        self.validate_specializations(trait_def)?;
        
        Ok(())
    }
    
    /// メソッド間の相互作用を最適化
    /// メソッド間の呼び出しパターンを分析し、全体最適化を適用する
    fn optimize_method_interactions(mut self) -> Result<Self> {
        // 呼び出しグラフの構築
        let call_graph = self.build_method_call_graph()?;
        
        // ホットパスの特定
        let hot_paths = call_graph.identify_hot_paths(&self.metadata.profile_data);
        
        // 共通部分の抽出
        self = self.extract_common_code_patterns(call_graph.clone())?;
        
        // 相互再帰の最適化
        self = self.optimize_mutual_recursion(call_graph.clone())?;
        
        // ホットパスの最適化
        for path in hot_paths {
            self = self.optimize_hot_path(path)?;
        }
        
        // インライン化の機会を特定
        let inline_candidates = self.identify_inline_candidates(call_graph.clone())?;
        
        // 適切なメソッドをインライン化
        for (caller, callee) in inline_candidates {
            self = self.inline_method_call(caller, callee)?;
        }
        
        // 未使用メソッドの最適化
        self = self.optimize_unused_methods(call_graph)?;
        
        // 並列実行の機会を特定
        self = self.identify_parallelization_opportunities()?;
        
        // メモリアクセスパターンの最適化
        self = self.optimize_memory_access_patterns()?;
        
        // 条件分岐の最適化
        self = self.optimize_conditional_branches()?;
        
        // 型特化の適用
        self = self.apply_type_specializations()?;
        
        // 最終的な検証
        self.validate_optimizations()?;
        
        Ok(self)
    }
    
    /// メソッド呼び出しグラフを構築する
    fn build_method_call_graph(&self) -> Result<MethodCallGraph> {
        let mut graph = MethodCallGraph::new();
        
        // 各メソッドをグラフに追加
        for (method_name, method_impl) in &self.methods {
            graph.add_node(method_name.clone());
            
            // メソッド本体から呼び出し関係を抽出
            let calls = self.extract_method_calls(&method_impl.body)?;
            
            // 呼び出し関係をグラフに追加
            for callee in calls {
                if self.methods.contains_key(&callee) {
                    graph.add_edge(method_name.clone(), callee, CallType::Direct);
                } else if let Some(trait_def) = &self.trait_definition {
                    if trait_def.default_methods.contains_key(&callee) {
                        graph.add_edge(method_name.clone(), callee, CallType::Default);
                    }
                }
            }
        }
        
        // 呼び出し頻度情報を追加
        if let Some(profile_data) = &self.metadata.profile_data {
            graph.annotate_with_profile_data(profile_data);
        }
        
        Ok(graph)
    }
    
    /// 共通コードパターンを抽出する
    fn extract_common_code_patterns(mut self, call_graph: MethodCallGraph) -> Result<Self> {
        // コードパターン検出器を初期化
        let pattern_detector = CodePatternDetector::new(&self.methods);
        
        // 共通パターンを検出
        let common_patterns = pattern_detector.detect_common_patterns()?;
        
        // 閾値以上出現するパターンを抽出
        for pattern in common_patterns.iter().filter(|p| p.occurrence_count >= 2) {
            // 新しいヘルパーメソッド名を生成
            let helper_name = format!("__extracted_helper_{}", self.next_helper_id());
            
            // ヘルパーメソッドの本体を作成
            let helper_body = pattern.create_helper_method()?;
            
            // ヘルパーメソッドのシグネチャを作成
            let helper_sig = pattern.create_helper_signature()?;
            
            // ヘルパーメソッドを追加
            self.methods.insert(helper_name.clone(), MethodImplementation {
                name: helper_name.clone(),
                signature: helper_sig,
                body: helper_body,
                location: self.location.clone(),
                conditions: Vec::new(),
            });
            
            // 元のメソッドを更新してヘルパーを使用するように
            for (method_name, method_impl) in &mut self.methods {
                if pattern.appears_in(method_name) {
                    let updated_body = pattern.replace_with_helper_call(
                        &method_impl.body,
                        &helper_name
                    )?;
                    
                    self.methods.get_mut(method_name).unwrap().body = updated_body;
                }
            }
        }
        
        Ok(self)
    }
    
    /// 相互再帰を最適化する
    fn optimize_mutual_recursion(mut self, call_graph: MethodCallGraph) -> Result<Self> {
        // 相互再帰グループを特定
        let recursive_groups = call_graph.identify_strongly_connected_components();
        
        for group in recursive_groups {
            if group.len() <= 1 {
                continue; // 単一メソッドの再帰は別途処理
            }
            
            // 末尾再帰最適化の適用
            self = self.apply_tail_recursion_optimization(&group)?;
            
            // 再帰展開の適用
            self = self.apply_recursion_unrolling(&group)?;
            
            // メモ化の適用
            self = self.apply_memoization(&group)?;
            
            // トランポリン変換の適用（スタックオーバーフロー防止）
            self = self.apply_trampoline_transformation(&group)?;
        }
        
        Ok(self)
    }
    
    /// ホットパスを最適化する
    fn optimize_hot_path(&self, path: Vec<String>) -> Result<Self> {
        let mut optimized = self.clone();
        
        // パス上のメソッドを特殊化
        for method_name in &path {
            if let Some(method) = self.methods.get(method_name) {
                // 実行時プロファイルに基づく最適化
                let optimized_method = self.metadata.optimizer.optimize_for_hot_path(
                    method,
                    &self.metadata.profile_data
                )?;
                
                // 最適化されたメソッドで置き換え
                optimized.methods.insert(method_name.clone(), optimized_method);
            }
        }
        
        // パス全体の融合最適化
        if path.len() >= 2 {
            optimized = optimized.apply_path_fusion(&path)?;
        }
        
        Ok(optimized)
    }
    
    /// インライン化候補を特定する
    fn identify_inline_candidates(&self, call_graph: MethodCallGraph) -> Result<Vec<(String, String)>> {
        let mut candidates = Vec::new();
        
        // 呼び出し頻度に基づいてインライン候補を特定
        for (caller, callees) in call_graph.get_all_edges() {
            for (callee, call_type) in callees {
                // 呼び出し頻度が高い場合
                if call_graph.get_call_frequency(&caller, &callee) > self.metadata.inline_threshold {
                    // 呼び出されるメソッドのサイズが小さい場合
                    if let Some(callee_method) = self.methods.get(&callee) {
                        let callee_size = self.estimate_method_size(callee_method);
                        
                        if callee_size <= self.metadata.max_inline_size {
                            candidates.push((caller.clone(), callee.clone()));
                        }
                    }
                }
            }
        }
        
        Ok(candidates)
    }
    /// トレイト実装を構築
    pub fn build(self) -> Result<TraitImplementation> {
        // トレイト定義が存在する場合は実装の検証を行う
        if let Some(trait_def) = &self.trait_definition {
            self.validate_implementation(trait_def)?;
        }
        
        Ok(TraitImplementation {
            trait_id: self.trait_id,
            for_type: self.for_type,
            type_params: self.type_params,
            associated_types: self.associated_types,
            associated_constants: self.associated_constants,
            methods: self.methods,
            where_clauses: self.where_clauses,
            is_unsafe: self.is_unsafe,
            is_derived: self.is_derived,
            location: self.location,
            metadata: self.metadata,
        })
    }
    
    /// 実装が有効かどうかを検証
    fn validate_implementation(&self, trait_def: &TraitDefinition) -> Result<()> {
        // 必須メソッドがすべて実装されているか確認
        for (name, method_sig) in &trait_def.methods {
            if !trait_def.default_methods.contains_key(name) && !self.methods.contains_key(name) {
                return Err(ErrorKind::TypeCheckError(
                    format!("トレイト '{}' のメソッド '{}' が実装されていません", trait_def.name, name),
                    self.location.clone(),
                ).into());
            }
        }
        
        // 関連型がすべて実装されているか確認
        for (name, _) in &trait_def.associated_types {
            if !self.associated_types.contains_key(name) {
                return Err(ErrorKind::TypeCheckError(
                    format!("トレイト '{}' の関連型 '{}' が実装されていません", trait_def.name, name),
                    self.location.clone(),
                ).into());
            }
        }
        
        // 関連定数がすべて実装されているか確認
        for (name, const_def) in &trait_def.associated_constants {
            if !self.associated_constants.contains_key(name) && const_def.default_value.is_none() {
                return Err(ErrorKind::TypeCheckError(
                    format!("トレイト '{}' の関連定数 '{}' が実装されていません", trait_def.name, name),
                    self.location.clone(),
                ).into());
            }
        }
        
        Ok(())
    }
}

/// トレイトの汎用機能を提供するユーティリティ
pub struct TraitUtils {
    /// 型レジストリへの参照
    type_registry: Arc<RefCell<TypeRegistry>>,
    
    /// トレイト解決機能への参照
    resolver: Arc<RefCell<TraitResolver>>,
}
impl TraitUtils {
    /// 新しいトレイトユーティリティを作成（スレッドセーフな設計）
    pub fn new(
        type_registry: Arc<RefCell<TypeRegistry>>,
        resolver: Arc<RefCell<TraitResolver>>,
    ) -> Self {
        TraitUtils {
            type_registry,
            resolver,
        }
    }
    
    /// 型がトレイトを実装しているか多層チェック
    pub fn implements_trait(&self, type_id: TypeId, trait_id: TypeId) -> Result<bool> {
        // キャッシュ付きの再帰的解決を実行
        self.resolver.borrow_mut().resolve_with_cache(type_id, trait_id, &mut HashMap::new())
    }
    
    /// トレイトメソッドを完全解決（デフォルト実装含む）
    pub fn lookup_trait_method(
        &self,
        type_id: TypeId,
        trait_id: TypeId,
        method_name: &str,
    ) -> Result<Option<MethodImplementation>> {
        let trait_impl = self.get_trait_implementation(type_id, trait_id)?;
        let trait_def = self.type_registry.borrow().get_trait(trait_id)?;
        
        // 実装メソッド -> デフォルトメソッド -> スーパートレイトの順で検索
        trait_impl.methods.get(method_name)
            .or_else(|| trait_def.default_methods.get(method_name))
            .or_else(|| self.check_supertraits(trait_id, |t| self.lookup_trait_method(type_id, t, method_name)))
            .map(|m| {
                let mut m = m.clone();
                self.instantiate_generics(&mut m.generic_params, type_id, trait_id)?;
                Ok(m)
            })
            .transpose()
    }
    
    /// 関連型を完全解決（型推論と再帰的解決を含む）
    pub fn resolve_associated_type(
        &self,
        type_id: TypeId,
        trait_id: TypeId,
        assoc_type_name: &str,
    ) -> Result<Option<TypeId>> {
        let trait_impl = self.get_trait_implementation(type_id, trait_id)?;
        let assoc_type = trait_impl.associated_types.get(assoc_type_name)
            .ok_or_else(|| ErrorKind::TypeCheckError(
                format!("関連型 '{}' の解決に失敗", assoc_type_name),
                SourceLocation::default()
            ))?;
        
        // 関連型のジェネリックインスタンス化
        self.resolve_type_with_context(*assoc_type, type_id, trait_id)
    }
    
    /// 関連定数を完全解決（定数畳み込み最適化付き）
    pub fn resolve_associated_constant(
        &self,
        type_id: TypeId,
        trait_id: TypeId,
        const_name: &str,
    ) -> Result<Option<ConstantValue>> {
        let trait_impl = self.get_trait_implementation(type_id, trait_id)?;
        let trait_def = self.type_registry.borrow().get_trait(trait_id)?;
        
        // 定数解決パイプライン
        let value = trait_impl.associated_constants.get(const_name)
            .or_else(|| trait_def.associated_constants.get(const_name).and_then(|c| c.default_value.as_ref()))
            .map(|v| self.fold_constant(v.clone()))
            .transpose()?;
        
        // 定数伝播最適化
        value.map(|v| self.propagate_constant(v, type_id)).transpose()
    }
    
    /// トレイト境界の完全検証（関連型制約含む）
    pub fn check_trait_bounds(
        &self,
        type_id: TypeId,
        bounds: &[TraitBound],
    ) -> Result<bool> {
        bounds.iter().try_fold(true, |acc, bound| {
            Ok(acc && self.check_trait_bound(type_id, bound)?)
        })
    }
    
    // 非公開ヘルパー関数群
    fn get_trait_implementation(&self, type_id: TypeId, trait_id: TypeId) -> Result<TraitImplementation> {
        self.resolver.borrow()
            .coherence_checker
            .borrow()
            .get_implementation(type_id, trait_id)
            .ok_or_else(|| ErrorKind::TypeCheckError(
                format!("トレイト実装が見つかりません: {} for {}", trait_id, type_id),
                SourceLocation::default()
            ).into())
    }
    
    fn check_trait_bound(&self, type_id: TypeId, bound: &TraitBound) -> Result<bool> {
        if !self.implements_trait(type_id, bound.trait_id)? {
            return Ok(false);
        }
        
        // 関連型制約の検証
        bound.associated_type_bounds.iter().try_fold(true, |acc, (name, expected)| {
            let actual = self.resolve_associated_type(type_id, bound.trait_id, name)?;
            Ok(acc && actual.map(|t| t == *expected).unwrap_or(false))
        })
    }
    
    fn resolve_type_with_context(&self, ty: TypeId, context: TypeId, trait_id: TypeId) -> Result<TypeId> {
        // 型解決パイプライン（ジェネリック置換、型推論、依存型解決）
        self.type_registry.borrow()
            .resolve_with_context(ty, context, Some(trait_id))
            .map_err(|e| e.into())
    }
    
    fn fold_constant(&self, value: ConstantValue) -> Result<ConstantValue> {
        // 定数畳み込みエンジン（コンパイル時計算）
        self.type_registry.borrow()
            .constant_folder
            .fold(value)
    }
    
    fn propagate_constant(&self, value: ConstantValue, ty: TypeId) -> Result<ConstantValue> {
        // 型依存の定数最適化
        self.type_registry.borrow()
            .constant_propagator
            .propagate(value, ty)
    }
    
    fn check_supertraits<F, T>(&self, trait_id: TypeId, mut f: F) -> Option<T> 
    where
        F: FnMut(TypeId) -> Result<Option<T>>
    {
        self.type_registry.borrow()
            .get_trait(trait_id)
            .ok()?
            .supertraits
            .iter()
            .find_map(|st| f(*st).transpose().ok())
    }
    
    fn instantiate_generics(
        &self,
        generics: &mut Option<Vec<GenericParameter>>,
        type_id: TypeId,
        trait_id: TypeId
    ) -> Result<()> {
        // ジェネリックパラメータの具体化
        if let Some(params) = generics {
            let substitutions = self.type_registry.borrow()
                .get_generic_mapping(type_id, trait_id)?;
            
            for param in params {
                if let Some(ty) = substitutions.get(&param.name) {
                    param.resolved_type = Some(*ty);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // コンパイラ内部テスト
    fn setup_test_env() -> (Arc<RefCell<TypeRegistry>>, Arc<RefCell<TraitResolver>>, Arc<RefCell<TraitCoherenceChecker>>) {
        let type_registry = Arc::new(RefCell::new(TypeRegistry::new()));
        let coherence_checker = Arc::new(RefCell::new(TraitCoherenceChecker::new(type_registry.clone())));
        let resolver = Arc::new(RefCell::new(TraitResolver::new(coherence_checker.clone(), type_registry.clone())));
        
        (type_registry, resolver, coherence_checker)
    }
    
    #[test]
    fn test_trait_implementation_builder() {
        let (type_registry, _, _) = setup_test_env();
        
        // テスト用のダミー型IDとソースロケーション
        let trait_id = TypeId::new(100);
        let for_type = TypeId::new(200);
        let location = SourceLocation::default(); // ダミーの位置情報
        
        // トレイト実装ビルダーのテスト
        let builder = TraitImplementationBuilder::new(trait_id, for_type, type_registry.clone(), location.clone());
        
        // メソッド実装を追加
        let method_impl = MethodImplementation {
            name: "test_method".to_string(),
            signature: MethodSignature {
                name: "test_method".to_string(),
                params: Vec::new(),
                return_type: TypeId::VOID,
                visibility: Visibility::Public,
                is_static: false,
                is_async: false,
                is_unsafe: false,
                is_abstract: false,
                is_virtual: false,
                is_override: false,
                is_pure: false,
                generic_params: None,
                effects: Vec::new(),
            },
            body: None,
            location: location.clone(),
            conditions: Vec::new(),
        };
        
        let builder = builder.add_method(method_impl);
        
        // 関連型を追加
        let assoc_type_id = TypeId::new(300);
        let builder = builder.add_associated_type("AssocType", assoc_type_id);
        
        // 実装を構築
        let result = builder.build();
        assert!(result.is_ok());
        
        let implementation = result.unwrap();
        assert_eq!(implementation.trait_id, trait_id);
        assert_eq!(implementation.for_type, for_type);
        assert!(implementation.methods.contains_key("test_method"));
        assert!(implementation.associated_types.contains_key("AssocType"));
        assert_eq!(implementation.associated_types["AssocType"], assoc_type_id);
    }
    
    #[test]
    fn test_trait_specialization() {
        let (type_registry, resolver, _) = setup_test_env();
        
        let mut specialization_manager = TraitSpecializationManager::new(
            type_registry.clone(),
            resolver.clone(),
        );
        
        // テスト用のダミー型IDとソースロケーション
        let trait_id = TypeId::new(100);
        let for_type = TypeId::new(200);
        let location = SourceLocation::default(); // ダミーの位置情報
        
        // 基本実装
        let base_impl = TraitImplementation {
            trait_id,
            for_type,
            type_params: Vec::new(),
            associated_types: HashMap::new(),
            associated_constants: HashMap::new(),
            methods: HashMap::new(),
            where_clauses: Vec::new(),
            is_unsafe: false,
            is_derived: false,
            location: location.clone(),
            metadata: ImplementationMetadata::default(),
        };
        
        // 特殊化条件なしで登録
        let result = specialization_manager.register_specialized_implementation(
            base_impl,
            0, // 優先度 0
            None, // 条件なし
        );
        
        assert!(result.is_ok());
        
        // 特殊化された実装を取得
        let impl_opt = specialization_manager.get_most_specialized_implementation(for_type, trait_id);
        assert!(impl_opt.is_some());
        
        let retrieved_impl = impl_opt.unwrap();
        assert_eq!(retrieved_impl.trait_id, trait_id);
        assert_eq!(retrieved_impl.for_type, for_type);
    }
} 
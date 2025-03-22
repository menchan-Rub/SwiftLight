// SwiftLight Type System - Effects
// エフェクトシステムの実装

//! # SwiftLight効果システム
//! 
//! 関数の副作用を型レベルで追跡し、純粋性や副作用の伝播を管理するシステムを提供します。
//! このモジュールにより、副作用の静的解析や関数合成の安全性が保証されます。

use std::collections::{HashMap, HashSet, BTreeSet};
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};

use crate::frontend::ast::{Expr, ExprKind};
use crate::frontend::error::{CompilerError, ErrorKind, Result, SourceLocation};
use crate::typesystem::{
    Type, TypeId, TypeRegistry, Symbol, Kind, 
    TypeError, TypeManager,
};

/// 効果の種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectKind {
    /// IO操作（ファイル、コンソール、ネットワークなど）
    IO,
    
    /// 状態変更（ミュータブルな状態の変更）
    Mutation,
    
    /// 例外発生
    Exception,
    
    /// 非決定性（乱数など）
    NonDeterminism,
    
    /// リソース操作（メモリ、ファイルハンドルなど）
    Resource,
    
    /// 並行性（スレッド、非同期処理など）
    Concurrency,
    
    /// 時間依存（現在時刻の取得など）
    TimeDependency,
    
    /// 量子効果（量子測定など）
    Quantum,
    
    /// グローバル状態アクセス
    GlobalAccess,
}

/// 効果ラベル
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EffectLabel {
    /// 効果の種類
    pub kind: EffectKind,
    
    /// 効果の対象（オプション）
    pub target: Option<Symbol>,
    
    /// 効果の詳細情報
    pub info: Option<EffectInfo>,
}

/// 効果の詳細情報
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EffectInfo {
    /// リソース効果の詳細
    Resource {
        /// リソースの獲得
        acquisition: bool,
        
        /// リソースの解放
        release: bool,
    },
    
    /// IO効果の詳細
    IO {
        /// 読み取り操作
        read: bool,
        
        /// 書き込み操作
        write: bool,
    },
    
    /// 状態変更効果の詳細
    Mutation {
        /// 変更するフィールド（オプション）
        field: Option<Symbol>,
    },
    
    /// 例外効果の詳細
    Exception {
        /// 例外の型
        exception_type: Symbol,
    },
    
    /// 量子効果の詳細
    Quantum {
        /// 測定操作
        measurement: bool,
        
        /// エンタングルメント操作
        entanglement: bool,
    },
}

/// 効果セット
#[derive(Debug, Clone)]
pub struct EffectSet {
    /// 効果のコレクション
    pub effects: HashSet<EffectLabel>,
    
    /// 効果の階層関係
    pub hierarchy: HashMap<EffectKind, HashSet<EffectLabel>>,
}

/// 関数型に対する効果注釈
#[derive(Debug, Clone)]
pub struct EffectAnnotation {
    /// 効果セット
    pub effects: EffectSet,
    
    /// 多相的効果変数
    pub effect_vars: Vec<EffectVar>,
    
    /// 効果上限（境界）
    pub upper_bound: Option<EffectSet>,
}

/// 効果変数
#[derive(Debug, Clone)]
pub struct EffectVar {
    /// 変数名
    pub name: Symbol,
    
    /// 変数の上限（境界）
    pub upper_bound: Option<EffectSet>,
}

/// 効果制約
#[derive(Debug, Clone)]
pub enum EffectConstraint {
    /// 効果サブタイピング制約
    Subeffect(EffectSet, EffectSet),
    
    /// 効果等価制約
    EffectEqual(EffectSet, EffectSet),
    
    /// 効果上限制約
    UpperBound(EffectVar, EffectSet),
    
    /// 効果不在制約（特定の効果が存在しないことを表明）
    Absence(EffectKind, Option<Symbol>),
}

/// 効果推論エンジン
pub struct EffectInference {
    /// 効果変数のマップ
    effect_vars: HashMap<Symbol, EffectVar>,
    
    /// 効果制約
    constraints: Vec<EffectConstraint>,
    
    /// 解決済み効果変数
    solved_vars: HashMap<Symbol, EffectSet>,
    
    /// 関数シグネチャの効果マップ
    function_effects: HashMap<Symbol, EffectAnnotation>,
    
    /// 型レジストリへの参照
    type_registry: Arc<TypeRegistry>,
}

impl EffectLabel {
    /// 新しい効果ラベルを作成
    pub fn new(kind: EffectKind, target: Option<Symbol>, info: Option<EffectInfo>) -> Self {
        Self { kind, target, info }
    }
    
    /// IO効果を作成
    pub fn io(target: Option<Symbol>, read: bool, write: bool) -> Self {
        Self {
            kind: EffectKind::IO,
            target,
            info: Some(EffectInfo::IO { read, write }),
        }
    }
    
    /// ミューテーション効果を作成
    pub fn mutation(target: Option<Symbol>, field: Option<Symbol>) -> Self {
        Self {
            kind: EffectKind::Mutation,
            target,
            info: Some(EffectInfo::Mutation { field }),
        }
    }
    
    /// 例外効果を作成
    pub fn exception(exception_type: Symbol) -> Self {
        Self {
            kind: EffectKind::Exception,
            target: None,
            info: Some(EffectInfo::Exception { exception_type }),
        }
    }
    
    /// リソース効果を作成
    pub fn resource(target: Option<Symbol>, acquisition: bool, release: bool) -> Self {
        Self {
            kind: EffectKind::Resource,
            target,
            info: Some(EffectInfo::Resource { acquisition, release }),
        }
    }
    
    /// 量子効果を作成
    pub fn quantum(target: Option<Symbol>, measurement: bool, entanglement: bool) -> Self {
        Self {
            kind: EffectKind::Quantum,
            target,
            info: Some(EffectInfo::Quantum { measurement, entanglement }),
        }
    }
    
    /// 非決定性効果を作成
    pub fn non_determinism() -> Self {
        Self {
            kind: EffectKind::NonDeterminism,
            target: None,
            info: None,
        }
    }
    
    /// 並行性効果を作成
    pub fn concurrency(target: Option<Symbol>) -> Self {
        Self {
            kind: EffectKind::Concurrency,
            target,
            info: None,
        }
    }
    
    /// 時間依存効果を作成
    pub fn time_dependency() -> Self {
        Self {
            kind: EffectKind::TimeDependency,
            target: None,
            info: None,
        }
    }
    
    /// グローバルアクセス効果を作成
    pub fn global_access(target: Symbol) -> Self {
        Self {
            kind: EffectKind::GlobalAccess,
            target: Some(target),
            info: None,
        }
    }
}

impl EffectSet {
    /// 新しい空の効果セットを作成
    pub fn new() -> Self {
        Self {
            effects: HashSet::new(),
            hierarchy: HashMap::new(),
        }
    }
    
    /// 効果を追加
    pub fn add_effect(&mut self, effect: EffectLabel) {
        self.effects.insert(effect.clone());
        
        // 階層にも追加
        self.hierarchy
            .entry(effect.kind)
            .or_insert_with(HashSet::new)
            .insert(effect);
    }
    
    /// 効果セットを結合
    pub fn union(&self, other: &EffectSet) -> EffectSet {
        let mut result = self.clone();
        
        for effect in &other.effects {
            result.add_effect(effect.clone());
        }
        
        result
    }
    
    /// 特定の種類の効果が含まれるかチェック
    pub fn contains_kind(&self, kind: EffectKind) -> bool {
        self.hierarchy.contains_key(&kind)
    }
    
    /// 特定の効果ラベルが含まれるかチェック
    pub fn contains(&self, effect: &EffectLabel) -> bool {
        self.effects.contains(effect)
    }
    
    /// 特定の種類の効果をすべて取得
    pub fn get_effects_by_kind(&self, kind: EffectKind) -> Vec<EffectLabel> {
        if let Some(effects) = self.hierarchy.get(&kind) {
            effects.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }
    
    /// このセットが別のセットのサブセットであるかチェック
    pub fn is_subset_of(&self, other: &EffectSet) -> bool {
        for effect in &self.effects {
            if !other.contains(effect) {
                return false;
            }
        }
        true
    }
    
    /// 純粋かどうかチェック（効果が存在しない）
    pub fn is_pure(&self) -> bool {
        self.effects.is_empty()
    }
}

impl EffectAnnotation {
    /// 新しい効果注釈を作成
    pub fn new() -> Self {
        Self {
            effects: EffectSet::new(),
            effect_vars: Vec::new(),
            upper_bound: None,
        }
    }
    
    /// 効果を追加
    pub fn add_effect(&mut self, effect: EffectLabel) {
        self.effects.add_effect(effect);
    }
    
    /// 効果変数を追加
    pub fn add_effect_var(&mut self, var: EffectVar) {
        self.effect_vars.push(var);
    }
    
    /// 上限を設定
    pub fn set_upper_bound(&mut self, bound: EffectSet) {
        self.upper_bound = Some(bound);
    }
    
    /// 効果注釈を結合
    pub fn combine(&self, other: &EffectAnnotation) -> Self {
        let mut result = self.clone();
        
        // 効果を結合
        result.effects = self.effects.union(&other.effects);
        
        // 効果変数を結合
        for var in &other.effect_vars {
            if !result.effect_vars.iter().any(|v| v.name == var.name) {
                result.effect_vars.push(var.clone());
            }
        }
        
        // 上限を更新（両方の上限がある場合は交差を取る）
        if let (Some(bound1), Some(bound2)) = (&self.upper_bound, &other.upper_bound) {
            // ここでは簡略化のため、単に両方の効果を含む新しいセットを作成
            let mut combined_bound = EffectSet::new();
            for effect in &bound1.effects {
                combined_bound.add_effect(effect.clone());
            }
            for effect in &bound2.effects {
                combined_bound.add_effect(effect.clone());
            }
            result.upper_bound = Some(combined_bound);
        } else if self.upper_bound.is_some() {
            result.upper_bound = self.upper_bound.clone();
        } else {
            result.upper_bound = other.upper_bound.clone();
        }
        
        result
    }
}

impl EffectInference {
    /// 新しい効果推論エンジンを作成
    pub fn new(type_registry: Arc<TypeRegistry>) -> Self {
        Self {
            effect_vars: HashMap::new(),
            constraints: Vec::new(),
            solved_vars: HashMap::new(),
            function_effects: HashMap::new(),
            type_registry,
        }
    }
    
    /// 関数定義から効果を推論
    pub fn infer_function_effects(&mut self, function_name: Symbol, body_expr: &TypeId) -> Result<EffectAnnotation> {
        // この関数はAST解析に基づいて関数本体から効果を推論する
        // （実際の実装ではAST構造を解析する必要がある）
        
        // 簡易実装として空の効果注釈を返す
        let mut annotation = EffectAnnotation::new();
        
        // 効果を収集する詳細な実装
        // （簡易実装のため省略）
        
        // 関数効果マップに保存
        self.function_effects.insert(function_name, annotation.clone());
        
        Ok(annotation)
    }
    
    /// 式から効果を推論
    pub fn infer_expression_effects(&mut self, expr: &TypeId) -> Result<EffectSet> {
        // この関数は式から効果を推論する
        // （実際の実装ではAST構造を解析する必要がある）
        
        // 簡易実装として空の効果セットを返す
        Ok(EffectSet::new())
    }
    
    /// 効果変数を作成
    pub fn create_effect_var(&mut self, name: Symbol, upper_bound: Option<EffectSet>) -> EffectVar {
        let var = EffectVar {
            name,
            upper_bound,
        };
        
        self.effect_vars.insert(name, var.clone());
        var
    }
    
    /// 効果制約を追加
    pub fn add_constraint(&mut self, constraint: EffectConstraint) {
        self.constraints.push(constraint);
    }
    
    /// 効果制約を解決
    pub fn solve_constraints(&mut self) -> Result<()> {
        let mut progress = true;
        
        // 制約が解決されなくなるまで繰り返す
        while progress {
            progress = false;
            
            let constraints = self.constraints.clone();
            for constraint in &constraints {
                match constraint {
                    EffectConstraint::Subeffect(sub, sup) => {
                        // サブ効果制約を解決
                        if !sub.is_subset_of(sup) {
                            return Err(CompilerError::new(
                                ErrorKind::TypeError,
                                "効果制約違反: サブ効果関係が成立しません".to_string(),
                                SourceLocation::default(),
                            ));
                        }
                    },
                    
                    EffectConstraint::EffectEqual(e1, e2) => {
                        // 効果等価制約を解決
                        if !e1.is_subset_of(e2) || !e2.is_subset_of(e1) {
                            return Err(CompilerError::new(
                                ErrorKind::TypeError,
                                "効果制約違反: 効果の等価性が成立しません".to_string(),
                                SourceLocation::default(),
                            ));
                        }
                    },
                    
                    EffectConstraint::UpperBound(var, bound) => {
                        // 上限制約を解決
                        if let Some(solved) = self.solved_vars.get(&var.name) {
                            if !solved.is_subset_of(bound) {
                                return Err(CompilerError::new(
                                    ErrorKind::TypeError,
                                    format!("効果制約違反: 変数 {} の効果が上限を超えています", var.name.as_str()),
                                    SourceLocation::default(),
                                ));
                            }
                        } else {
                            // 変数をバインド
                            self.solved_vars.insert(var.name, bound.clone());
                            progress = true;
                        }
                    },
                    
                    EffectConstraint::Absence(kind, target) => {
                        // 効果不在制約を解決
                        for (var_name, effect_set) in &self.solved_vars {
                            if effect_set.contains_kind(*kind) {
                                let matching_effects = effect_set.get_effects_by_kind(*kind);
                                
                                // ターゲットが指定されている場合は、そのターゲットの効果がないか確認
                                if let Some(target_sym) = target {
                                    for effect in matching_effects {
                                        if let Some(effect_target) = &effect.target {
                                            if effect_target == target_sym {
                                                return Err(CompilerError::new(
                                                    ErrorKind::TypeError,
                                                    format!("効果制約違反: 変数 {} に禁止された効果 {:?}({}) が含まれています", 
                                                            var_name.as_str(), kind, target_sym.as_str()),
                                                    SourceLocation::default(),
                                                ));
                                            }
                                        }
                                    }
                                } else {
                                    // ターゲットが指定されていない場合は、その種類の効果がないか確認
                                    return Err(CompilerError::new(
                                        ErrorKind::TypeError,
                                        format!("効果制約違反: 変数 {} に禁止された効果 {:?} が含まれています", 
                                                var_name.as_str(), kind),
                                        SourceLocation::default(),
                                    ));
                                }
                            }
                        }
                    },
                }
            }
        }
        
        Ok(())
    }
    
    /// 関数呼び出しの効果を推論
    pub fn infer_call_effects(&mut self, 
                             function: Symbol,
                             args: &[TypeId]) -> Result<EffectSet> {
        // 関数の効果注釈を取得
        if let Some(annotation) = self.function_effects.get(&function) {
            // 実際の実装では引数の効果を考慮して効果を計算する必要がある
            // （簡易実装のため省略）
            return Ok(annotation.effects.clone());
        }
        
        // 関数の効果が不明な場合、保守的にIO効果を仮定
        let mut effects = EffectSet::new();
        effects.add_effect(EffectLabel::io(None, true, true));
        
        Ok(effects)
    }
    
    /// 型から効果を抽出（関数型の場合）
    pub fn extract_effects_from_type(&self, type_id: TypeId) -> Result<Option<EffectAnnotation>> {
        let resolved_type = self.type_registry.resolve(type_id);
        
        // 実際の実装では型構造を解析する必要がある
        // （簡易実装のため省略）
        
        Ok(None)
    }
    
    /// 純粋性チェック
    pub fn check_purity(&self, effects: &EffectSet) -> bool {
        effects.is_pure()
    }
    
    /// 関数合成の効果を計算
    pub fn compose_effects(&self, f: &EffectSet, g: &EffectSet) -> EffectSet {
        f.union(g)
    }
    
    /// 関数の効果注釈を取得
    pub fn get_function_effects(&self, function: Symbol) -> Option<EffectAnnotation> {
        self.function_effects.get(&function).cloned()
    }
}

/// 純粋関数であることを示す効果セット
pub fn pure_effect() -> EffectSet {
    EffectSet::new()
}

/// IO効果セットを作成
pub fn io_effect() -> EffectSet {
    let mut effect_set = EffectSet::new();
    effect_set.add_effect(EffectLabel::io(None, true, true));
    effect_set
}

/// イミュータブル（読み取り専用）IO効果セットを作成
pub fn read_only_io_effect() -> EffectSet {
    let mut effect_set = EffectSet::new();
    effect_set.add_effect(EffectLabel::io(None, true, false));
    effect_set
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // TODO: テストケースを実装
    
    #[test]
    fn test_effect_set_operations() {
        let mut set1 = EffectSet::new();
        set1.add_effect(EffectLabel::io(None, true, false));
        
        let mut set2 = EffectSet::new();
        set2.add_effect(EffectLabel::mutation(None, None));
        
        let combined = set1.union(&set2);
        assert_eq!(combined.effects.len(), 2);
        assert!(combined.contains_kind(EffectKind::IO));
        assert!(combined.contains_kind(EffectKind::Mutation));
    }
} 
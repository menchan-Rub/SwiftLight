//! # SwiftLightリニア型システム
//!
//! リニア型とアフィン型によるリソース管理システムを提供します。
//! このモジュールにより、変数の使用回数を静的に追跡し、メモリ安全性を保証します。

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;

use crate::frontend::error::{CompilerError, ErrorKind, Result, SourceLocation};
use crate::typesystem::{
    Type, TypeId, TypeRegistry, Symbol, Kind, 
    TypeConstraint, TypeCheckContext,
};

/// リソース使用モード
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceMode {
    /// 線形リソース（正確に1回使用される）
    Linear,
    
    /// アフィンリソース（0回または1回使用される）
    Affine,
    
    /// 通常リソース（制限なし）
    Unrestricted,
    
    /// 関連リソース（他のリソースに依存）
    Related(ResourceId),
}

/// リソース識別子
pub type ResourceId = usize;

/// リソース状態
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceState {
    /// 未使用
    Unused,
    
    /// 部分的に使用（ビュー、参照など）
    PartiallyUsed,
    
    /// 移動済み
    Moved,
    
    /// 破棄済み
    Dropped,
}

/// リソース使用追跡
#[derive(Debug, Clone)]
pub struct ResourceTracker {
    /// 次のリソースID
    next_resource_id: ResourceId,
    
    /// リソースモード
    resource_modes: HashMap<ResourceId, ResourceMode>,
    
    /// リソース状態
    resource_states: HashMap<ResourceId, ResourceState>,
    
    /// シンボルへのリソースマッピング
    symbol_resources: HashMap<Symbol, ResourceId>,
    
    /// 型へのリソースマッピング
    type_resources: HashMap<TypeId, ResourceId>,
    
    /// 借用関係（所有者 -> 借用者のリスト）
    borrows: HashMap<ResourceId, Vec<ResourceId>>,
    
    /// 所有関係（借用者 -> 所有者）
    owners: HashMap<ResourceId, ResourceId>,
}

/// リソース借用モード
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorrowMode {
    /// 共有借用（読み取り専用）
    Shared,
    
    /// 排他的借用（読み書き）
    Exclusive,
    
    /// 借用なし（所有権移動）
    Move,
}

/// リソース割り当て
#[derive(Debug, Clone)]
pub struct ResourceAllocation {
    /// リソースID
    pub id: ResourceId,
    
    /// リソースモード
    pub mode: ResourceMode,
    
    /// 関連する型
    pub type_id: Option<TypeId>,
    
    /// 割り当て位置
    pub location: SourceLocation,
}

/// リニア型チェッカー
pub struct LinearTypeChecker {
    /// リソース追跡
    resource_tracker: ResourceTracker,
    
    /// 型レジストリ
    type_registry: Arc<TypeRegistry>,
    
    /// スコープスタック
    scopes: Vec<Scope>,
}

/// スコープ情報
#[derive(Debug, Clone)]
struct Scope {
    /// スコープ内のリソース
    resources: HashSet<ResourceId>,
    
    /// スコープの名前（オプション）
    name: Option<Symbol>,
    
    /// スコープの親
    parent: Option<usize>,
}

impl ResourceTracker {
    /// 新しいリソーストラッカーを作成
    pub fn new() -> Self {
        Self {
            next_resource_id: 0,
            resource_modes: HashMap::new(),
            resource_states: HashMap::new(),
            symbol_resources: HashMap::new(),
            type_resources: HashMap::new(),
            borrows: HashMap::new(),
            owners: HashMap::new(),
        }
    }
    
    /// 新しいリソースを割り当て
    pub fn allocate_resource(&mut self, mode: ResourceMode) -> ResourceId {
        let id = self.next_resource_id;
        self.next_resource_id += 1;
        
        self.resource_modes.insert(id, mode);
        self.resource_states.insert(id, ResourceState::Unused);
        
        id
    }
    
    /// リソースをシンボルに関連付け
    pub fn associate_symbol(&mut self, resource_id: ResourceId, symbol: Symbol) {
        self.symbol_resources.insert(symbol, resource_id);
    }
    
    /// リソースを型に関連付け
    pub fn associate_type(&mut self, resource_id: ResourceId, type_id: TypeId) {
        self.type_resources.insert(type_id, resource_id);
    }
    
    /// リソースを使用としてマーク
    pub fn mark_used(&mut self, resource_id: ResourceId) -> Result<()> {
        let state = self.resource_states.get(&resource_id)
            .ok_or_else(|| CompilerError::new(
                ErrorKind::TypeError,
                format!("リソースID {} が見つかりません", resource_id),
                SourceLocation::default(),
            ))?;
        
        let mode = self.resource_modes.get(&resource_id)
            .ok_or_else(|| CompilerError::new(
                ErrorKind::TypeError,
                format!("リソースID {} のモードが見つかりません", resource_id),
                SourceLocation::default(),
            ))?;
        
        // 現在の状態に基づいて使用をチェック
        match state {
            ResourceState::Unused => {
                // 未使用リソースは使用可能
                self.resource_states.insert(resource_id, ResourceState::Moved);
                Ok(())
            },
            
            ResourceState::PartiallyUsed => {
                // 部分的に使用されたリソースは、モードに依存
                match mode {
                    ResourceMode::Linear => {
                        return Err(CompilerError::new(
                            ErrorKind::TypeError,
                            format!("線形リソース {} は既に部分的に使用されています", resource_id),
                            SourceLocation::default(),
                        ));
                    },
                    ResourceMode::Affine => {
                        self.resource_states.insert(resource_id, ResourceState::Moved);
                        Ok(())
                    },
                    ResourceMode::Unrestricted => {
                        // 無制限リソースは複数回使用可能
                        Ok(())
                    },
                    ResourceMode::Related(owner_id) => {
                        // 所有者の状態に依存
                        if let Some(owner_state) = self.resource_states.get(&owner_id) {
                            match owner_state {
                                ResourceState::Moved | ResourceState::Dropped => {
                                    return Err(CompilerError::new(
                                        ErrorKind::TypeError,
                                        format!("関連リソース {} の所有者 {} は既に使用されています", 
                                                resource_id, owner_id),
                                        SourceLocation::default(),
                                    ));
                                },
                                _ => {
                                    self.resource_states.insert(resource_id, ResourceState::Moved);
                                    Ok(())
                                },
                            }
                        } else {
                            return Err(CompilerError::new(
                                ErrorKind::TypeError,
                                format!("関連リソース {} の所有者 {} が見つかりません", 
                                        resource_id, owner_id),
                                SourceLocation::default(),
                            ));
                        }
                    },
                }
            },
            
            ResourceState::Moved => {
                // 既に移動されたリソースは再利用不可（線形・アフィン）
                match mode {
                    ResourceMode::Linear | ResourceMode::Affine | ResourceMode::Related(_) => {
                        return Err(CompilerError::new(
                            ErrorKind::TypeError,
                            format!("リソース {} は既に使用されています", resource_id),
                            SourceLocation::default(),
                        ));
                    },
                    ResourceMode::Unrestricted => {
                        // 無制限リソースは複数回使用可能
                        Ok(())
                    },
                }
            },
            
            ResourceState::Dropped => {
                // 破棄されたリソースは再利用不可
                return Err(CompilerError::new(
                    ErrorKind::TypeError,
                    format!("リソース {} は既に破棄されています", resource_id),
                    SourceLocation::default(),
                ));
            },
        }
    }
    
    /// リソースを部分的に使用としてマーク（ビュー、参照など）
    pub fn mark_partially_used(&mut self, resource_id: ResourceId) -> Result<()> {
        let state = self.resource_states.get(&resource_id)
            .ok_or_else(|| CompilerError::new(
                ErrorKind::TypeError,
                format!("リソースID {} が見つかりません", resource_id),
                SourceLocation::default(),
            ))?;
        
        // 現在の状態に基づいて部分使用をチェック
        match state {
            ResourceState::Unused => {
                // 未使用リソースは部分使用可能
                self.resource_states.insert(resource_id, ResourceState::PartiallyUsed);
                Ok(())
            },
            
            ResourceState::PartiallyUsed => {
                // 既に部分使用されているリソースは、モードに依存
                let mode = self.resource_modes.get(&resource_id)
                    .ok_or_else(|| CompilerError::new(
                        ErrorKind::TypeError,
                        format!("リソースID {} のモードが見つかりません", resource_id),
                        SourceLocation::default(),
                    ))?;
                
                match mode {
                    ResourceMode::Unrestricted => Ok(()),
                    _ => {
                        return Err(CompilerError::new(
                            ErrorKind::TypeError,
                            format!("リソース {} は既に部分的に使用されています", resource_id),
                            SourceLocation::default(),
                        ));
                    },
                }
            },
            
            ResourceState::Moved => {
                // 既に移動されたリソースは部分使用不可
                return Err(CompilerError::new(
                    ErrorKind::TypeError,
                    format!("リソース {} は既に移動されています", resource_id),
                    SourceLocation::default(),
                ));
            },
            
            ResourceState::Dropped => {
                // 破棄されたリソースは部分使用不可
                return Err(CompilerError::new(
                    ErrorKind::TypeError,
                    format!("リソース {} は既に破棄されています", resource_id),
                    SourceLocation::default(),
                ));
            },
        }
    }
    
    /// リソースを借用
    pub fn borrow_resource(&mut self, 
                          owner_id: ResourceId, 
                          mode: BorrowMode) -> Result<ResourceId> {
        // 所有者の状態をチェック
        let owner_state = self.resource_states.get(&owner_id)
            .ok_or_else(|| CompilerError::new(
                ErrorKind::TypeError,
                format!("リソースID {} が見つかりません", owner_id),
                SourceLocation::default(),
            ))?;
        
        match owner_state {
            ResourceState::Unused | ResourceState::PartiallyUsed => {
                // 未使用または部分使用のリソースは借用可能
                
                // 新しいリソースを作成
                let borrowed_id = self.allocate_resource(ResourceMode::Related(owner_id));
                
                // 借用関係を記録
                self.borrows
                    .entry(owner_id)
                    .or_insert_with(Vec::new)
                    .push(borrowed_id);
                    
                self.owners.insert(borrowed_id, owner_id);
                
                // 排他的借用の場合は所有者を部分使用としてマーク
                if mode == BorrowMode::Exclusive {
                    self.mark_partially_used(owner_id)?;
                }
                
                Ok(borrowed_id)
            },
            
            ResourceState::Moved => {
                // 移動済みリソースは借用不可
                return Err(CompilerError::new(
                    ErrorKind::TypeError,
                    format!("リソース {} は既に移動されているため借用できません", owner_id),
                    SourceLocation::default(),
                ));
            },
            
            ResourceState::Dropped => {
                // 破棄済みリソースは借用不可
                return Err(CompilerError::new(
                    ErrorKind::TypeError,
                    format!("リソース {} は既に破棄されているため借用できません", owner_id),
                    SourceLocation::default(),
                ));
            },
        }
    }
    
    /// リソースを破棄
    pub fn drop_resource(&mut self, resource_id: ResourceId) -> Result<()> {
        // リソースの状態をチェック
        let state = self.resource_states.get(&resource_id)
            .ok_or_else(|| CompilerError::new(
                ErrorKind::TypeError,
                format!("リソースID {} が見つかりません", resource_id),
                SourceLocation::default(),
            ))?;
        
        match state {
            ResourceState::Moved | ResourceState::Dropped => {
                // 既に移動または破棄されたリソースは再破棄不可
                return Err(CompilerError::new(
                    ErrorKind::TypeError,
                    format!("リソース {} は既に使用または破棄されています", resource_id),
                    SourceLocation::default(),
                ));
            },
            
            _ => {
                // 借用されているリソースをチェック
                if let Some(borrowed_ids) = self.borrows.get(&resource_id) {
                    if !borrowed_ids.is_empty() {
                        let active_borrows: Vec<_> = borrowed_ids.iter()
                            .filter(|id| {
                                let state = self.resource_states.get(id).unwrap_or(&ResourceState::Dropped);
                                *state != ResourceState::Dropped
                            })
                            .collect();
                            
                        if !active_borrows.is_empty() {
                            return Err(CompilerError::new(
                                ErrorKind::TypeError,
                                format!("リソース {} は貸し出し中のため破棄できません", resource_id),
                                SourceLocation::default(),
                            ));
                        }
                    }
                }
                
                // リソースを破棄済みとしてマーク
                self.resource_states.insert(resource_id, ResourceState::Dropped);
                Ok(())
            },
        }
    }
    
    /// リソースの状態を取得
    pub fn get_resource_state(&self, resource_id: ResourceId) -> Option<ResourceState> {
        self.resource_states.get(&resource_id).copied()
    }
    
    /// リソースのモードを取得
    pub fn get_resource_mode(&self, resource_id: ResourceId) -> Option<ResourceMode> {
        self.resource_modes.get(&resource_id).copied()
    }
    
    /// シンボルに関連するリソースを取得
    pub fn get_symbol_resource(&self, symbol: &Symbol) -> Option<ResourceId> {
        self.symbol_resources.get(symbol).copied()
    }
    
    /// 型に関連するリソースを取得
    pub fn get_type_resource(&self, type_id: TypeId) -> Option<ResourceId> {
        self.type_resources.get(&type_id).copied()
    }
}

impl LinearTypeChecker {
    /// 新しいリニア型チェッカーを作成
    pub fn new(type_registry: Arc<TypeRegistry>) -> Self {
        Self {
            resource_tracker: ResourceTracker::new(),
            type_registry,
            scopes: vec![Scope {
                resources: HashSet::new(),
                name: None,
                parent: None,
            }],
        }
    }
    
    /// 現在のスコープにリソースを追加
    pub fn add_resource_to_scope(&mut self, resource_id: ResourceId) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.resources.insert(resource_id);
        }
    }
    
    /// 新しいスコープを開始
    pub fn enter_scope(&mut self, name: Option<Symbol>) {
        let parent = if self.scopes.is_empty() {
            None
        } else {
            Some(self.scopes.len() - 1)
        };
        
        self.scopes.push(Scope {
            resources: HashSet::new(),
            name,
            parent,
        });
    }
    
    /// 現在のスコープを終了
    pub fn exit_scope(&mut self) -> Result<()> {
        if self.scopes.len() <= 1 {
            return Err(CompilerError::new(
                ErrorKind::TypeError,
                "グローバルスコープを終了しようとしています".to_string(),
                SourceLocation::default(),
            ));
        }
        
        let scope = self.scopes.pop()
            .ok_or_else(|| CompilerError::new(
                ErrorKind::TypeError,
                "スコープが見つかりません".to_string(),
                SourceLocation::default(),
            ))?;
        
        // スコープ内の未使用リソースをチェック
        for &resource_id in &scope.resources {
            let mode = self.resource_tracker.get_resource_mode(resource_id)
                .ok_or_else(|| CompilerError::new(
                    ErrorKind::TypeError,
                    format!("リソースID {} のモードが見つかりません", resource_id),
                    SourceLocation::default(),
                ))?;
                
            let state = self.resource_tracker.get_resource_state(resource_id)
                .ok_or_else(|| CompilerError::new(
                    ErrorKind::TypeError,
                    format!("リソースID {} の状態が見つかりません", resource_id),
                    SourceLocation::default(),
                ))?;
                
            // リニアリソースが未使用のままスコープを抜けるとエラー
            if mode == ResourceMode::Linear && state == ResourceState::Unused {
                return Err(CompilerError::new(
                    ErrorKind::TypeError,
                    format!("線形リソース {} が未使用のままスコープを抜けます", resource_id),
                    SourceLocation::default(),
                ));
            }
        }
        
        Ok(())
    }
    
    /// リニア型の使用をチェック
    pub fn check_linear_use(&mut self, type_id: TypeId, location: SourceLocation) -> Result<()> {
        let resource_id = self.resource_tracker.get_type_resource(type_id)
            .ok_or_else(|| CompilerError::new(
                ErrorKind::TypeError,
                format!("型 {} に関連するリソースが見つかりません", 
                        self.type_registry.debug_type(type_id)),
                location,
            ))?;
            
        self.resource_tracker.mark_used(resource_id)
    }
    
    /// リニア型の借用をチェック
    pub fn check_linear_borrow(&mut self, 
                              type_id: TypeId, 
                              borrow_mode: BorrowMode,
                              location: SourceLocation) -> Result<TypeId> {
        let resource_id = self.resource_tracker.get_type_resource(type_id)
            .ok_or_else(|| CompilerError::new(
                ErrorKind::TypeError,
                format!("型 {} に関連するリソースが見つかりません", 
                        self.type_registry.debug_type(type_id)),
                location,
            ))?;
            
        let borrowed_id = self.resource_tracker.borrow_resource(resource_id, borrow_mode)?;
        
        // 借用の参照型を作成
        let ref_type = if borrow_mode == BorrowMode::Shared {
            Type::Ref {
                target: type_id,
                mutable: false,
            }
        } else {
            Type::Ref {
                target: type_id,
                mutable: true,
            }
        };
        
        let ref_type_id = self.type_registry.register_type(ref_type);
        
        // 作成した参照型をリソースに関連付け
        self.resource_tracker.associate_type(borrowed_id, ref_type_id);
        
        Ok(ref_type_id)
    }
    
    /// シンボルに新しいリソースを割り当て
    pub fn allocate_symbol_resource(&mut self, 
                                   symbol: Symbol, 
                                   mode: ResourceMode,
                                   type_id: TypeId,
                                   location: SourceLocation) -> Result<ResourceId> {
        // 新しいリソースを割り当て
        let resource_id = self.resource_tracker.allocate_resource(mode);
        
        // シンボルと型に関連付け
        self.resource_tracker.associate_symbol(resource_id, symbol);
        self.resource_tracker.associate_type(resource_id, type_id);
        
        // 現在のスコープに追加
        self.add_resource_to_scope(resource_id);
        
        Ok(resource_id)
    }
    
    /// 型からリソースモードを決定
    pub fn resource_mode_for_type(&self, type_id: TypeId) -> ResourceMode {
        let ty = self.type_registry.resolve(type_id);
        
        match ty {
            // 基本型は無制限
            Type::Primitive(_) => ResourceMode::Unrestricted,
            
            // リファレンス型はアフィン
            Type::Ref { .. } => ResourceMode::Affine,
            
            // 線形注釈を持つ型は線形
            Type::Annotated { base, annotation } => {
                if let Some(TypeLinearAnnotation::Linear) = annotation.linear {
                    ResourceMode::Linear
                } else if let Some(TypeLinearAnnotation::Affine) = annotation.linear {
                    ResourceMode::Affine
                } else {
                    self.resource_mode_for_type(*base)
                }
            },
            
            // 型変数は（保守的に）線形
            Type::TypeVar { .. } => ResourceMode::Linear,
            
            // 型アプリケーションは構築子に依存
            Type::Application { constructor, .. } => {
                self.resource_mode_for_type(constructor)
            },
            
            // デフォルトは無制限
            _ => ResourceMode::Unrestricted,
        }
    }
    
    /// スコープ終了時に未使用リソースをチェック
    pub fn check_unused_resources(&self) -> Result<()> {
        for scope in &self.scopes {
            for &resource_id in &scope.resources {
                let mode = self.resource_tracker.get_resource_mode(resource_id)
                    .ok_or_else(|| CompilerError::new(
                        ErrorKind::TypeError,
                        format!("リソースID {} のモードが見つかりません", resource_id),
                        SourceLocation::default(),
                    ))?;
                    
                let state = self.resource_tracker.get_resource_state(resource_id)
                    .ok_or_else(|| CompilerError::new(
                        ErrorKind::TypeError,
                        format!("リソースID {} の状態が見つかりません", resource_id),
                        SourceLocation::default(),
                    ))?;
                    
                // リニアリソースが未使用のままだとエラー
                if mode == ResourceMode::Linear && state == ResourceState::Unused {
                    return Err(CompilerError::new(
                        ErrorKind::TypeError,
                        format!("線形リソース {} が未使用のままプログラムが終了します", resource_id),
                        SourceLocation::default(),
                    ));
                }
            }
        }
        
        Ok(())
    }
}

/// 型のリニア注釈
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeLinearAnnotation {
    /// 線形型
    Linear,
    
    /// アフィン型
    Affine,
    
    /// 無制限型
    Unrestricted,
}

/// 型注釈
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TypeAnnotation {
    /// リニア型注釈
    pub linear: Option<TypeLinearAnnotation>,
    
    /// 他の注釈...
}

/// TypeのAnnotated変種を定義するために拡張
impl Type {
    /// 注釈付き型を作成
    pub fn annotated(base: TypeId, annotation: TypeAnnotation) -> Self {
        Type::Annotated {
            base,
            annotation,
        }
    }
    
    /// 線形型として注釈
    pub fn linear(base: TypeId) -> Self {
        let annotation = TypeAnnotation {
            linear: Some(TypeLinearAnnotation::Linear),
            ..Default::default()
        };
        
        Type::annotated(base, annotation)
    }
    
    /// アフィン型として注釈
    pub fn affine(base: TypeId) -> Self {
        let annotation = TypeAnnotation {
            linear: Some(TypeLinearAnnotation::Affine),
            ..Default::default()
        };
        
        Type::annotated(base, annotation)
    }
    
    /// 無制限型として注釈
    pub fn unrestricted(base: TypeId) -> Self {
        let annotation = TypeAnnotation {
            linear: Some(TypeLinearAnnotation::Unrestricted),
            ..Default::default()
        };
        
        Type::annotated(base, annotation)
    }
}

/// 型チェックコンテキストとの統合
impl TypeCheckContext {
    /// リニア型チェッカーをコンテキストに追加
    pub fn with_linear_checker(mut self, checker: LinearTypeChecker) -> Self {
        self.linear_checker = Some(checker);
        self
    }
    
    /// リニア型の使用をチェック
    pub fn check_linear_use(&mut self, type_id: TypeId, location: SourceLocation) -> Result<()> {
        if let Some(checker) = &mut self.linear_checker {
            checker.check_linear_use(type_id, location)
        } else {
            // リニアチェッカーがない場合は何もしない
            Ok(())
        }
    }
    
    /// リニア型の借用をチェック
    pub fn check_linear_borrow(&mut self, 
                              type_id: TypeId, 
                              borrow_mode: BorrowMode,
                              location: SourceLocation) -> Result<TypeId> {
        if let Some(checker) = &mut self.linear_checker {
            checker.check_linear_borrow(type_id, borrow_mode, location)
        } else {
            // リニアチェッカーがない場合は参照型を作成するだけ
            let ref_type = if borrow_mode == BorrowMode::Shared {
                Type::Ref {
                    target: type_id,
                    mutable: false,
                }
            } else {
                Type::Ref {
                    target: type_id,
                    mutable: true,
                }
            };
            
            Ok(self.type_registry.register_type(ref_type))
        }
    }
    
    /// スコープを開始
    pub fn enter_linear_scope(&mut self, name: Option<Symbol>) {
        if let Some(checker) = &mut self.linear_checker {
            checker.enter_scope(name);
        }
    }
    
    /// スコープを終了
    pub fn exit_linear_scope(&mut self) -> Result<()> {
        if let Some(checker) = &mut self.linear_checker {
            checker.exit_scope()
        } else {
            Ok(())
        }
    }
    
    /// シンボルに新しいリソースを割り当て
    pub fn allocate_linear_resource(&mut self, 
                                   symbol: Symbol, 
                                   mode: ResourceMode,
                                   type_id: TypeId,
                                   location: SourceLocation) -> Result<ResourceId> {
        if let Some(checker) = &mut self.linear_checker {
            checker.allocate_symbol_resource(symbol, mode, type_id, location)
        } else {
            // リニアチェッカーがない場合はダミーIDを返す
            Ok(0)
        }
    }
    
    /// 型からリソースモードを決定
    pub fn resource_mode_for_type(&self, type_id: TypeId) -> ResourceMode {
        if let Some(checker) = &self.linear_checker {
            checker.resource_mode_for_type(type_id)
        } else {
            // リニアチェッカーがない場合はすべての型を無制限として扱う
            ResourceMode::Unrestricted
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // TODO: テストケースを実装
    
    #[test]
    fn test_resource_tracker_basic() {
        let mut tracker = ResourceTracker::new();
        
        // リソースを割り当て
        let linear_id = tracker.allocate_resource(ResourceMode::Linear);
        let affine_id = tracker.allocate_resource(ResourceMode::Affine);
        let unrestricted_id = tracker.allocate_resource(ResourceMode::Unrestricted);
        
        // リソースモードをチェック
        assert_eq!(tracker.get_resource_mode(linear_id), Some(ResourceMode::Linear));
        assert_eq!(tracker.get_resource_mode(affine_id), Some(ResourceMode::Affine));
        assert_eq!(tracker.get_resource_mode(unrestricted_id), Some(ResourceMode::Unrestricted));
        
        // リソース状態をチェック
        assert_eq!(tracker.get_resource_state(linear_id), Some(ResourceState::Unused));
        assert_eq!(tracker.get_resource_state(affine_id), Some(ResourceState::Unused));
        assert_eq!(tracker.get_resource_state(unrestricted_id), Some(ResourceState::Unused));
    }
} 
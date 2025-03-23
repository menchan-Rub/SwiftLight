//! # モジュール依存グラフ
//!
//! モジュール間の依存関係を表現するためのグラフ構造と、
//! 依存関係の解析、循環依存の検出、およびトポロジカルソートを提供します。

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;

use crate::frontend::error::{CompilerError, ErrorKind, Result, SourceLocation};
use super::ModuleId;

/// モジュール依存グラフ
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// ノード (モジュールID) のリスト
    nodes: HashSet<ModuleId>,
    
    /// エッジ: モジュールIDから依存モジュールIDへのマッピング
    edges: HashMap<ModuleId, HashSet<ModuleId>>,
    
    /// 逆エッジ: モジュールIDから、それに依存しているモジュールIDへのマッピング
    reverse_edges: HashMap<ModuleId, HashSet<ModuleId>>,
    
    /// 循環依存に含まれるノード
    cycle_nodes: HashSet<ModuleId>,
    
    /// 検出された循環依存パス
    cycles: Vec<Vec<ModuleId>>,
}

/// 依存グラフのトラバーサル状態
#[derive(Debug, Clone, PartialEq, Eq)]
enum VisitState {
    /// 未訪問
    NotVisited,
    
    /// 訪問中（サイクル検出のため）
    Visiting,
    
    /// 訪問済み
    Visited,
}

impl DependencyGraph {
    /// 新しい依存グラフを作成
    pub fn new() -> Self {
        Self {
            nodes: HashSet::new(),
            edges: HashMap::new(),
            reverse_edges: HashMap::new(),
            cycle_nodes: HashSet::new(),
            cycles: Vec::new(),
        }
    }
    
    /// ノード（モジュール）を追加
    pub fn add_node(&mut self, id: ModuleId) {
        self.nodes.insert(id.clone());
        self.edges.entry(id.clone()).or_insert_with(HashSet::new);
        self.reverse_edges.entry(id).or_insert_with(HashSet::new);
    }
    
    /// 依存関係（エッジ）を追加
    /// 
    /// `from` は `to` に依存する
    pub fn add_dependency(&mut self, from: ModuleId, to: ModuleId) {
        // 必要に応じてノードを追加
        if !self.nodes.contains(&from) {
            self.add_node(from.clone());
        }
        if !self.nodes.contains(&to) {
            self.add_node(to.clone());
        }
        
        // エッジを追加
        self.edges.get_mut(&from).unwrap().insert(to.clone());
        self.reverse_edges.get_mut(&to).unwrap().insert(from);
    }
    
    /// 依存関係を取得
    pub fn get_dependencies(&self, id: &ModuleId) -> &HashSet<ModuleId> {
        self.edges.get(id).unwrap_or(&EMPTY_DEPS)
    }
    
    /// 逆依存関係を取得（このモジュールに依存しているモジュール）
    pub fn get_dependents(&self, id: &ModuleId) -> &HashSet<ModuleId> {
        self.reverse_edges.get(id).unwrap_or(&EMPTY_DEPS)
    }
    
    /// ノードの数を取得
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    
    /// エッジの数を取得
    pub fn edge_count(&self) -> usize {
        self.edges.values().map(|deps| deps.len()).sum()
    }
    
    /// 循環依存をチェック
    pub fn check_cycles(&mut self) -> Result<bool> {
        self.cycle_nodes.clear();
        self.cycles.clear();
        
        let mut visit_state: HashMap<ModuleId, VisitState> = self.nodes.iter()
            .map(|id| (id.clone(), VisitState::NotVisited))
            .collect();
        
        let mut path: Vec<ModuleId> = Vec::new();
        let mut has_cycles = false;
        
        // 各ノードから深さ優先探索
        for id in self.nodes.clone() {
            if visit_state[&id] == VisitState::NotVisited {
                if self.dfs_check_cycle(&id, &mut visit_state, &mut path, &mut has_cycles)? {
                    has_cycles = true;
                }
            }
        }
        
        Ok(has_cycles)
    }
    
    /// 深さ優先探索で循環依存をチェック
    fn dfs_check_cycle(
        &mut self, 
        current: &ModuleId,
        visit_state: &mut HashMap<ModuleId, VisitState>,
        path: &mut Vec<ModuleId>,
        has_cycles: &mut bool,
    ) -> Result<bool> {
        // 現在のノードを訪問中としてマーク
        visit_state.insert(current.clone(), VisitState::Visiting);
        path.push(current.clone());
        
        // 依存先を探索
        for dep in self.get_dependencies(current).clone() {
            match visit_state[&dep] {
                VisitState::NotVisited => {
                    // 未訪問なら再帰的に探索
                    if self.dfs_check_cycle(&dep, visit_state, path, has_cycles)? {
                        *has_cycles = true;
                    }
                },
                VisitState::Visiting => {
                    // 訪問中のノードに遭遇 = 循環依存を検出
                    
                    // 循環パスを抽出
                    let mut cycle_start_idx = 0;
                    for (i, id) in path.iter().enumerate() {
                        if id == &dep {
                            cycle_start_idx = i;
                            break;
                        }
                    }
                    
                    let cycle: Vec<ModuleId> = path[cycle_start_idx..].to_vec();
                    
                    // 循環パスを記録
                    self.cycles.push(cycle.clone());
                    
                    // 循環に含まれるノードを記録
                    for id in &cycle {
                        self.cycle_nodes.insert(id.clone());
                    }
                    
                    *has_cycles = true;
                    
                    // 詳細なエラーメッセージを作成
                    let cycle_str = cycle.iter()
                        .map(|id| id.to_string())
                        .collect::<Vec<_>>()
                        .join(" -> ");
                    
                    return Err(CompilerError::new(
                        ErrorKind::ModuleSystem,
                        format!("循環依存が検出されました: {}", cycle_str),
                        SourceLocation::default(),
                    ));
                },
                VisitState::Visited => {
                    // 既に訪問済みなので何もしない
                },
            }
        }
        
        // 現在のノードを訪問済みとしてマーク
        visit_state.insert(current.clone(), VisitState::Visited);
        path.pop();
        
        Ok(*has_cycles)
    }
    
    /// ノードが循環依存に含まれているかチェック
    pub fn is_in_cycle(&self, id: &ModuleId) -> bool {
        self.cycle_nodes.contains(id)
    }
    
    /// トポロジカルソート（依存順に並び替え）
    pub fn topological_sort(&self) -> Result<Vec<&ModuleId>> {
        let mut result = Vec::new();
        let mut visit_state: HashMap<&ModuleId, VisitState> = self.nodes.iter()
            .map(|id| (id, VisitState::NotVisited))
            .collect();
        
        // 各ノードから深さ優先探索
        for id in self.nodes.iter() {
            if visit_state[id] == VisitState::NotVisited {
                self.dfs_topological_sort(id, &mut visit_state, &mut result)?;
            }
        }
        
        // 結果を反転（後入れ先出しなので）
        result.reverse();
        
        Ok(result)
    }
    
    /// 深さ優先探索でトポロジカルソート
    fn dfs_topological_sort<'a>(
        &'a self,
        current: &'a ModuleId,
        visit_state: &mut HashMap<&'a ModuleId, VisitState>,
        result: &mut Vec<&'a ModuleId>,
    ) -> Result<()> {
        // 現在のノードを訪問中としてマーク
        visit_state.insert(current, VisitState::Visiting);
        
        // 依存先を探索
        for dep in self.get_dependencies(current) {
            if let Some(dep_state) = visit_state.get(dep) {
                match dep_state {
                    VisitState::NotVisited => {
                        // 未訪問なら再帰的に探索
                        self.dfs_topological_sort(dep, visit_state, result)?;
                    },
                    VisitState::Visiting => {
                        // 訪問中のノードに遭遇 = 循環依存があるため
                        // トポロジカルソートは不可能
                        return Err(CompilerError::new(
                            ErrorKind::ModuleSystem,
                            "循環依存が存在するためトポロジカルソートができません".to_string(),
                            SourceLocation::default(),
                        ));
                    },
                    VisitState::Visited => {
                        // 既に訪問済みなので何もしない
                    },
                }
            }
        }
        
        // 現在のノードを訪問済みとしてマーク
        visit_state.insert(current, VisitState::Visited);
        
        // 結果に追加
        result.push(current);
        
        Ok(())
    }
    
    /// 到達可能なノードを取得
    pub fn reachable_nodes(&self, start: &ModuleId) -> HashSet<ModuleId> {
        let mut result = HashSet::new();
        let mut queue = VecDeque::new();
        
        queue.push_back(start.clone());
        result.insert(start.clone());
        
        while let Some(current) = queue.pop_front() {
            for dep in self.get_dependencies(&current) {
                if !result.contains(dep) {
                    result.insert(dep.clone());
                    queue.push_back(dep.clone());
                }
            }
        }
        
        result
    }
    
    /// 逆方向に到達可能なノードを取得（依存元を辿る）
    pub fn reverse_reachable_nodes(&self, start: &ModuleId) -> HashSet<ModuleId> {
        let mut result = HashSet::new();
        let mut queue = VecDeque::new();
        
        queue.push_back(start.clone());
        result.insert(start.clone());
        
        while let Some(current) = queue.pop_front() {
            for dep in self.get_dependents(&current) {
                if !result.contains(dep) {
                    result.insert(dep.clone());
                    queue.push_back(dep.clone());
                }
            }
        }
        
        result
    }
    
    /// 依存グラフをDOT形式で出力（Graphviz用）
    pub fn to_dot(&self) -> String {
        let mut result = String::from("digraph dependencies {\n");
        
        // ノード定義
        for id in &self.nodes {
            let node_name = format!("\"{}\"", id);
            let attributes = if self.cycle_nodes.contains(id) {
                "[color=red,style=filled,fillcolor=pink]"
            } else {
                "[color=blue]"
            };
            
            result.push_str(&format!("    {} {};\n", node_name, attributes));
        }
        
        // エッジ定義
        for (from, deps) in &self.edges {
            let from_name = format!("\"{}\"", from);
            
            for to in deps {
                let to_name = format!("\"{}\"", to);
                
                let attributes = if self.cycle_nodes.contains(from) && self.cycle_nodes.contains(to) {
                    "[color=red,penwidth=2.0]"
                } else {
                    "[color=black]"
                };
                
                result.push_str(&format!("    {} -> {} {};\n", from_name, to_name, attributes));
            }
        }
        
        result.push_str("}\n");
        result
    }
}

/// 空の依存セット（定数）
static EMPTY_DEPS: HashSet<ModuleId> = HashSet::new();

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typesystem::Symbol;
    
    fn create_module_id(name: &str) -> ModuleId {
        ModuleId::Absolute(vec![Symbol::intern(name)])
    }
    
    #[test]
    fn test_simple_dependency() {
        let mut graph = DependencyGraph::new();
        
        let a = create_module_id("A");
        let b = create_module_id("B");
        
        graph.add_node(a.clone());
        graph.add_node(b.clone());
        graph.add_dependency(a.clone(), b.clone());
        
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
        assert!(graph.get_dependencies(&a).contains(&b));
        assert!(graph.get_dependents(&b).contains(&a));
    }
    
    #[test]
    fn test_cycle_detection() {
        let mut graph = DependencyGraph::new();
        
        let a = create_module_id("A");
        let b = create_module_id("B");
        let c = create_module_id("C");
        
        graph.add_node(a.clone());
        graph.add_node(b.clone());
        graph.add_node(c.clone());
        
        graph.add_dependency(a.clone(), b.clone());
        graph.add_dependency(b.clone(), c.clone());
        
        // この時点ではサイクルはない
        assert_eq!(graph.check_cycles().unwrap(), false);
        
        // サイクルを作る
        graph.add_dependency(c.clone(), a.clone());
        
        // サイクルが検出されるはず
        let result = graph.check_cycles();
        assert!(result.is_err());
        
        // 全ノードがサイクルに含まれることを確認
        assert!(graph.is_in_cycle(&a));
        assert!(graph.is_in_cycle(&b));
        assert!(graph.is_in_cycle(&c));
    }
    
    #[test]
    fn test_topological_sort() {
        let mut graph = DependencyGraph::new();
        
        let a = create_module_id("A");
        let b = create_module_id("B");
        let c = create_module_id("C");
        let d = create_module_id("D");
        
        graph.add_node(a.clone());
        graph.add_node(b.clone());
        graph.add_node(c.clone());
        graph.add_node(d.clone());
        
        graph.add_dependency(a.clone(), b.clone());
        graph.add_dependency(a.clone(), c.clone());
        graph.add_dependency(b.clone(), d.clone());
        graph.add_dependency(c.clone(), d.clone());
        
        let sorted = graph.topological_sort().unwrap();
        
        // Dは他に依存していないため最初に来る
        assert_eq!(sorted[0], &d);
        
        // Aは最後に来る (全てのモジュールに直接・間接的に依存)
        assert_eq!(sorted[3], &a);
        
        // BとCはどちらもDにだけ依存しているため、
        // どちらが先でも構わないがAの前でなければならない
        assert!(
            (sorted[1] == &b && sorted[2] == &c) ||
            (sorted[1] == &c && sorted[2] == &b)
        );
    }
    
    #[test]
    fn test_reachable_nodes() {
        let mut graph = DependencyGraph::new();
        
        let a = create_module_id("A");
        let b = create_module_id("B");
        let c = create_module_id("C");
        let d = create_module_id("D");
        let e = create_module_id("E");
        
        graph.add_node(a.clone());
        graph.add_node(b.clone());
        graph.add_node(c.clone());
        graph.add_node(d.clone());
        graph.add_node(e.clone());
        
        graph.add_dependency(a.clone(), b.clone());
        graph.add_dependency(a.clone(), c.clone());
        graph.add_dependency(b.clone(), d.clone());
        graph.add_dependency(c.clone(), e.clone());
        
        let reachable = graph.reachable_nodes(&a);
        
        // Aから全てのノードに到達可能
        assert_eq!(reachable.len(), 5);
        assert!(reachable.contains(&a));
        assert!(reachable.contains(&b));
        assert!(reachable.contains(&c));
        assert!(reachable.contains(&d));
        assert!(reachable.contains(&e));
        
        // Bからは B, D のみ到達可能
        let reachable_from_b = graph.reachable_nodes(&b);
        assert_eq!(reachable_from_b.len(), 2);
        assert!(reachable_from_b.contains(&b));
        assert!(reachable_from_b.contains(&d));
        assert!(!reachable_from_b.contains(&a));
        assert!(!reachable_from_b.contains(&c));
        assert!(!reachable_from_b.contains(&e));
    }
    
    #[test]
    fn test_reverse_reachable_nodes() {
        let mut graph = DependencyGraph::new();
        
        let a = create_module_id("A");
        let b = create_module_id("B");
        let c = create_module_id("C");
        let d = create_module_id("D");
        
        graph.add_node(a.clone());
        graph.add_node(b.clone());
        graph.add_node(c.clone());
        graph.add_node(d.clone());
        
        graph.add_dependency(a.clone(), b.clone());
        graph.add_dependency(b.clone(), d.clone());
        graph.add_dependency(c.clone(), d.clone());
        
        // Dに依存しているノードを探索
        let reverse_reachable = graph.reverse_reachable_nodes(&d);
        
        assert_eq!(reverse_reachable.len(), 4);
        assert!(reverse_reachable.contains(&a));
        assert!(reverse_reachable.contains(&b));
        assert!(reverse_reachable.contains(&c));
        assert!(reverse_reachable.contains(&d));
    }
} 
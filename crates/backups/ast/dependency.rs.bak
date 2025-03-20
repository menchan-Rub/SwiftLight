// 依存関係を管理するモジュール
// コンパイル単位間の依存関係の分析と管理を行います

use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet, VecDeque};
use crate::frontend::error::{CompilerError, ErrorKind, Result};

/// 依存関係の種類
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyType {
    /// インポート（使用のみ）
    Import,
    /// 継承（親クラスなど）
    Inheritance,
    /// 実装（インターフェースなど）
    Implementation,
    /// コンポジション（フィールド型など）
    Composition,
}

/// 依存関係グラフのノード
#[derive(Debug, Clone)]
pub struct DependencyNode {
    /// モジュール名
    pub name: String,
    /// ソースファイルパス
    pub source_path: PathBuf,
    /// 依存しているモジュール（名前と依存の種類）
    pub dependencies: HashMap<String, DependencyType>,
    /// このノードに依存しているモジュール
    pub dependents: HashSet<String>,
}

impl DependencyNode {
    /// 新しい依存関係ノードを作成
    pub fn new<P: AsRef<Path>>(name: String, source_path: P) -> Self {
        Self {
            name,
            source_path: source_path.as_ref().to_path_buf(),
            dependencies: HashMap::new(),
            dependents: HashSet::new(),
        }
    }

    /// 依存関係を追加
    pub fn add_dependency(&mut self, module_name: String, dep_type: DependencyType) {
        self.dependencies.insert(module_name, dep_type);
    }

    /// 依存元を追加
    pub fn add_dependent(&mut self, module_name: String) {
        self.dependents.insert(module_name);
    }
}

/// 依存関係グラフ
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// モジュールノード
    nodes: HashMap<String, DependencyNode>,
}

impl DependencyGraph {
    /// 新しい依存関係グラフを作成
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    /// モジュールを追加
    pub fn add_module<P: AsRef<Path>>(&mut self, module_name: String, source_path: P, dependencies: HashSet<String>) {
        let mut node = DependencyNode::new(module_name.clone(), source_path);
        
        // 依存関係を設定
        for dep in dependencies {
            node.add_dependency(dep.clone(), DependencyType::Import);
            
            // 依存先のノードに依存元を追加
            if let Some(dep_node) = self.nodes.get_mut(&dep) {
                dep_node.add_dependent(module_name.clone());
            }
        }
        
        self.nodes.insert(module_name, node);
    }

    /// モジュールのソースパスを取得
    pub fn get_source_path(&self, module_name: &str) -> Option<PathBuf> {
        self.nodes.get(module_name).map(|node| node.source_path.clone())
    }

    /// コンパイル順序を取得（トポロジカルソート）
    pub fn get_compilation_order(&self) -> Vec<String> {
        // 入次数（依存元の数）を計算
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        
        for node in self.nodes.values() {
            in_degree.entry(node.name.clone()).or_insert(0);
            
            for dep in node.dependencies.keys() {
                *in_degree.entry(dep.clone()).or_insert(0) += 1;
            }
        }
        
        // 入次数が0のノードからキューに追加
        let mut queue: VecDeque<String> = VecDeque::new();
        for (node_name, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(node_name.clone());
            }
        }
        
        // トポロジカルソート
        let mut result = Vec::new();
        
        while let Some(node_name) = queue.pop_front() {
            result.push(node_name.clone());
            
            if let Some(node) = self.nodes.get(&node_name) {
                for dep_name in node.dependents.iter() {
                    if let Some(count) = in_degree.get_mut(dep_name) {
                        *count -= 1;
                        if *count == 0 {
                            queue.push_back(dep_name.clone());
                        }
                    }
                }
            }
        }
        
        // 出力の順序を反転（依存先から順に）
        result.reverse();
        result
    }

    /// 循環依存関係の検出
    pub fn detect_cycles(&self) -> Option<Vec<String>> {
        // 各ノードの訪問状態
        #[derive(PartialEq, Eq, Clone, Copy)]
        enum VisitState {
            NotVisited,
            InProgress,
            Completed,
        }
        
        let mut visit_state: HashMap<String, VisitState> = HashMap::new();
        let mut path: Vec<String> = Vec::new();
        let mut cycle: Option<Vec<String>> = None;
        
        // DFSで循環を検出
        for node_name in self.nodes.keys() {
            if visit_state.get(node_name) != Some(&VisitState::Completed) {
                if Self::dfs_detect_cycle(node_name, &self.nodes, &mut visit_state, &mut path, &mut cycle) {
                    return cycle;
                }
            }
        }
        
        None
    }
    
    // DFSで循環検出のヘルパー関数
    fn dfs_detect_cycle(
        node_name: &str,
        nodes: &HashMap<String, DependencyNode>,
        visit_state: &mut HashMap<String, VisitState>,
        path: &mut Vec<String>,
        cycle: &mut Option<Vec<String>>,
    ) -> bool {
        // 訪問中なら循環を検出
        if visit_state.get(node_name) == Some(&VisitState::InProgress) {
            // 現在のパスから循環部分を抽出
            let start_idx = path.iter().position(|n| n == node_name).unwrap();
            let cycle_path = path[start_idx..].to_vec();
            *cycle = Some(cycle_path);
            return true;
        }
        
        // すでに処理済みならスキップ
        if visit_state.get(node_name) == Some(&VisitState::Completed) {
            return false;
        }
        
        // 訪問中としてマーク
        visit_state.insert(node_name.to_string(), VisitState::InProgress);
        path.push(node_name.to_string());
        
        // 依存先を再帰的に探索
        if let Some(node) = nodes.get(node_name) {
            for dep_name in node.dependencies.keys() {
                if Self::dfs_detect_cycle(dep_name, nodes, visit_state, path, cycle) {
                    return true;
                }
            }
        }
        
        // パスから削除して完了としてマーク
        path.pop();
        visit_state.insert(node_name.to_string(), VisitState::Completed);
        
        false
    }

    // 並列コンパイル可能なグループを取得
    pub fn parallel_groups(&self) -> Vec<Vec<usize>> {
        let mut result = Vec::new();
        // 実装は省略（実際のプロジェクトでは依存関係に基づいて並列処理可能なグループを特定）
        result
    }

    // トポロジカルソート
    pub fn topological_sort(&self) -> Result<Vec<String>> {
        if let Some(cycle) = self.detect_cycles() {
            return Err(CompilerError::new(
                ErrorKind::CircularDependency,
                format!("循環依存関係が検出されました: {}", cycle.join(" -> ")),
                None
            ));
        }
        
        Ok(self.get_compilation_order())
    }
} 
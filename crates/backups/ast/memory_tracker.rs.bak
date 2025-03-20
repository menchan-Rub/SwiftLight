// メモリ使用量を追跡するモジュール
// コンパイラのメモリ使用状況を監視し、メモリリークを検出します

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};

/// メモリ使用状況のスナップショット
#[derive(Debug, Clone)]
pub struct MemoryUsageSnapshot {
    /// 計測時刻
    pub timestamp: Instant,
    /// 合計メモリ使用量（バイト単位）
    pub total_bytes: usize,
    /// コンポーネント別メモリ使用量
    pub component_bytes: HashMap<String, usize>,
    /// 前回のスナップショットからの差分（バイト単位）
    pub delta_bytes: isize,
}

impl MemoryUsageSnapshot {
    /// 新しいメモリ使用状況スナップショットを作成
    fn new(total_bytes: usize, component_bytes: HashMap<String, usize>, delta_bytes: isize) -> Self {
        Self {
            timestamp: Instant::now(),
            total_bytes,
            component_bytes,
            delta_bytes,
        }
    }
}

impl fmt::Display for MemoryUsageSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "メモリ使用状況:")?;
        writeln!(f, "  合計: {} バイト ({:.2} MB)", self.total_bytes, self.total_bytes as f64 / 1_048_576.0)?;
        
        if self.delta_bytes != 0 {
            let sign = if self.delta_bytes > 0 { "+" } else { "" };
            writeln!(f, "  変化: {}{} バイト ({:.2} MB)", sign, self.delta_bytes, self.delta_bytes as f64 / 1_048_576.0)?;
        }
        
        writeln!(f, "  コンポーネント別:")?;
        let mut components: Vec<_> = self.component_bytes.iter().collect();
        components.sort_by(|a, b| b.1.cmp(a.1)); // 使用量の多い順にソート
        
        for (component, bytes) in components {
            writeln!(f, "    {}: {} バイト ({:.2} MB)", component, bytes, *bytes as f64 / 1_048_576.0)?;
        }
        
        Ok(())
    }
}

/// メモリトラッカー
#[derive(Debug, Clone)]
pub struct MemoryTracker {
    /// 現在のメモリ使用量（バイト単位）
    current_bytes: Arc<Mutex<usize>>,
    /// コンポーネント別メモリ使用量
    component_bytes: Arc<Mutex<HashMap<String, usize>>>,
    /// 履歴
    history: Arc<Mutex<Vec<MemoryUsageSnapshot>>>,
    /// ピーク時のメモリ使用量
    peak_bytes: Arc<Mutex<usize>>,
    /// 有効かどうか
    enabled: Arc<Mutex<bool>>,
}

impl Default for MemoryTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryTracker {
    /// 新しいメモリトラッカーを作成
    pub fn new() -> Self {
        Self {
            current_bytes: Arc::new(Mutex::new(0)),
            component_bytes: Arc::new(Mutex::new(HashMap::new())),
            history: Arc::new(Mutex::new(Vec::new())),
            peak_bytes: Arc::new(Mutex::new(0)),
            enabled: Arc::new(Mutex::new(true)),
        }
    }
    
    /// メモリトラッカーが有効かどうかを設定
    pub fn set_enabled(&self, enabled: bool) {
        let mut e = self.enabled.lock().unwrap();
        *e = enabled;
    }
    
    /// メモリトラッカーが有効かどうかを取得
    pub fn is_enabled(&self) -> bool {
        let e = self.enabled.lock().unwrap();
        *e
    }
    
    /// メモリ割り当てを記録
    pub fn record_allocation(&self, bytes: usize, component: Option<&str>) {
        if !self.is_enabled() {
            return;
        }
        
        let mut current = self.current_bytes.lock().unwrap();
        *current += bytes;
        
        // ピーク時の使用量を更新
        let mut peak = self.peak_bytes.lock().unwrap();
        if *current > *peak {
            *peak = *current;
        }
        
        // コンポーネント別の使用量を更新
        if let Some(component) = component {
            let mut components = self.component_bytes.lock().unwrap();
            let entry = components.entry(component.to_string()).or_insert(0);
            *entry += bytes;
        }
    }
    
    /// メモリ解放を記録
    pub fn record_deallocation(&self, bytes: usize, component: Option<&str>) {
        if !self.is_enabled() {
            return;
        }
        
        let mut current = self.current_bytes.lock().unwrap();
        if bytes <= *current {
            *current -= bytes;
        } else {
            // オーバーフローを防止
            *current = 0;
        }
        
        // コンポーネント別の使用量を更新
        if let Some(component) = component {
            let mut components = self.component_bytes.lock().unwrap();
            if let Some(entry) = components.get_mut(component) {
                if bytes <= *entry {
                    *entry -= bytes;
                } else {
                    // オーバーフローを防止
                    *entry = 0;
                }
            }
        }
    }
    
    /// 現在のメモリ使用量を取得
    pub fn current_usage(&self) -> usize {
        let current = self.current_bytes.lock().unwrap();
        *current
    }
    
    /// ピーク時のメモリ使用量を取得
    pub fn peak_usage(&self) -> usize {
        let peak = self.peak_bytes.lock().unwrap();
        *peak
    }
    
    /// コンポーネント別のメモリ使用量を取得
    pub fn component_usage(&self) -> HashMap<String, usize> {
        let components = self.component_bytes.lock().unwrap();
        components.clone()
    }
    
    /// 現在のメモリ使用状況のスナップショットを取得
    pub fn take_snapshot(&self) -> MemoryUsageSnapshot {
        let current = self.current_bytes.lock().unwrap();
        let components = self.component_bytes.lock().unwrap();
        let mut history = self.history.lock().unwrap();
        
        let delta = if history.is_empty() {
            *current as isize
        } else {
            *current as isize - history.last().unwrap().total_bytes as isize
        };
        
        let snapshot = MemoryUsageSnapshot::new(*current, components.clone(), delta);
        history.push(snapshot.clone());
        
        snapshot
    }
    
    /// 履歴からメモリリークを検出
    pub fn detect_leaks(&self) -> Vec<String> {
        let history = self.history.lock().unwrap();
        let mut leaks = Vec::new();
        
        if history.len() < 2 {
            return leaks;
        }
        
        // 直近のスナップショットを取得
        let latest = &history[history.len() - 1];
        let previous = &history[history.len() - 2];
        
        // メモリ使用量が増加しているコンポーネントを検出
        for (component, bytes) in &latest.component_bytes {
            if let Some(prev_bytes) = previous.component_bytes.get(component) {
                if bytes > prev_bytes {
                    leaks.push(format!(
                        "{}: {} バイト増加 ({} -> {})",
                        component,
                        bytes - prev_bytes,
                        prev_bytes,
                        bytes
                    ));
                }
            } else {
                // 新しいコンポーネントが追加された場合
                leaks.push(format!("{}: {} バイト（新規）", component, bytes));
            }
        }
        
        leaks
    }
    
    /// メモリ使用履歴をクリア
    pub fn clear_history(&self) {
        let mut history = self.history.lock().unwrap();
        history.clear();
    }
    
    /// 全てのメモリ使用状況をリセット
    pub fn reset(&self) {
        let mut current = self.current_bytes.lock().unwrap();
        *current = 0;
        
        let mut peak = self.peak_bytes.lock().unwrap();
        *peak = 0;
        
        let mut components = self.component_bytes.lock().unwrap();
        components.clear();
        
        let mut history = self.history.lock().unwrap();
        history.clear();
    }
    
    /// メモリ使用履歴を文字列として取得
    pub fn history_to_string(&self) -> String {
        let history = self.history.lock().unwrap();
        let mut result = String::new();
        
        result.push_str("メモリ使用履歴:\n");
        for (i, snapshot) in history.iter().enumerate() {
            result.push_str(&format!("スナップショット #{}\n", i));
            result.push_str(&format!("{}\n", snapshot));
        }
        
        result
    }
} 
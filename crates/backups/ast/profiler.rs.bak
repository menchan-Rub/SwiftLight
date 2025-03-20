// プロファイリングを行うモジュール
// コンパイル時間の計測や性能分析に使用されます

use std::collections::HashMap;
use std::time::{Instant, Duration};
use std::path::Path;
use std::fs::File;
use std::io::Write;

/// プロファイリングイベント
#[derive(Debug, Clone)]
pub struct ProfilingEvent {
    /// イベント名
    pub name: String,
    /// 開始時間
    pub start_time: Instant,
    /// 終了時間（終了後に設定）
    pub end_time: Option<Instant>,
    /// 所要時間（終了後に計算）
    pub duration: Option<Duration>,
    /// 親イベント名
    pub parent: Option<String>,
    /// メタデータ
    pub metadata: HashMap<String, String>,
}

/// プロファイリングスコープ
pub struct ProfilingScope<'a> {
    /// プロファイラーへの参照
    profiler: &'a mut Profiler,
    /// イベント名
    event_name: String,
}

impl<'a> ProfilingScope<'a> {
    /// 新しいプロファイリングスコープを作成
    fn new(profiler: &'a mut Profiler, name: &str) -> Self {
        profiler.start(name);
        Self {
            profiler,
            event_name: name.to_string(),
        }
    }
    
    /// メタデータを追加
    pub fn add_metadata(&mut self, key: &str, value: &str) {
        self.profiler.add_event_metadata(&self.event_name, key, value);
    }
}

impl<'a> Drop for ProfilingScope<'a> {
    fn drop(&mut self) {
        self.profiler.stop(&self.event_name);
    }
}

/// プロファイラー
#[derive(Debug, Default)]
pub struct Profiler {
    /// 有効かどうか
    enabled: bool,
    /// アクティブなイベント
    active_events: HashMap<String, ProfilingEvent>,
    /// 完了したイベント
    completed_events: Vec<ProfilingEvent>,
    /// 最適化パスの所要時間
    pass_durations: HashMap<String, Duration>,
}

impl Profiler {
    /// 新しいプロファイラーを作成
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            active_events: HashMap::new(),
            completed_events: Vec::new(),
            pass_durations: HashMap::new(),
        }
    }
    
    /// プロファイラーが有効かどうか
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    /// プロファイラーを有効化
    pub fn enable(&mut self) {
        self.enabled = true;
    }
    
    /// プロファイラーを無効化
    pub fn disable(&mut self) {
        self.enabled = false;
    }
    
    /// イベントの開始
    pub fn start(&mut self, name: &str) -> Instant {
        if !self.enabled {
            return Instant::now();
        }
        
        let start_time = Instant::now();
        let event = ProfilingEvent {
            name: name.to_string(),
            start_time,
            end_time: None,
            duration: None,
            parent: None,
            metadata: HashMap::new(),
        };
        
        self.active_events.insert(name.to_string(), event);
        start_time
    }
    
    /// イベントの終了
    pub fn stop(&mut self, name: &str) -> Duration {
        if !self.enabled {
            return Duration::default();
        }
        
        let now = Instant::now();
        
        if let Some(mut event) = self.active_events.remove(name) {
            event.end_time = Some(now);
            let duration = now.duration_since(event.start_time);
            event.duration = Some(duration);
            
            // 最適化パスの時間を記録
            if name.starts_with("opt_pass_") {
                let pass_name = name.strip_prefix("opt_pass_").unwrap_or(name);
                self.pass_durations.insert(pass_name.to_string(), duration);
            }
            
            self.completed_events.push(event);
            return duration;
        }
        
        Duration::default()
    }
    
    /// スコープベースのプロファイリングを開始
    pub fn scope<'a>(&'a mut self, name: &str) -> ProfilingScope<'a> {
        ProfilingScope::new(self, name)
    }
    
    /// イベントにメタデータを追加
    pub fn add_event_metadata(&mut self, event_name: &str, key: &str, value: &str) {
        if !self.enabled {
            return;
        }
        
        if let Some(event) = self.active_events.get_mut(event_name) {
            event.metadata.insert(key.to_string(), value.to_string());
        }
    }
    
    /// 最適化パスの時間を取得
    pub fn get_pass_durations(&self) -> &HashMap<String, Duration> {
        &self.pass_durations
    }
    
    /// プロファイリングレポートを文字列で取得
    pub fn get_report(&self) -> String {
        if !self.enabled || self.completed_events.is_empty() {
            return String::new();
        }
        
        let mut report = String::new();
        report.push_str("==== プロファイリングレポート ====\n");
        
        // イベントを終了時間順にソート
        let mut events = self.completed_events.clone();
        events.sort_by(|a, b| {
            a.end_time.unwrap().cmp(&b.end_time.unwrap())
        });
        
        // イベントの詳細を出力
        report.push_str("イベント一覧:\n");
        for event in &events {
            if let Some(duration) = event.duration {
                report.push_str(&format!("  {}: {:?}\n", event.name, duration));
                
                // メタデータがあれば出力
                if !event.metadata.is_empty() {
                    for (key, value) in &event.metadata {
                        report.push_str(&format!("    {}: {}\n", key, value));
                    }
                }
            }
        }
        
        // 最適化パスの時間を出力
        if !self.pass_durations.is_empty() {
            report.push_str("\n最適化パスの時間:\n");
            
            // パスを時間順にソート
            let mut passes: Vec<_> = self.pass_durations.iter().collect();
            passes.sort_by(|a, b| b.1.cmp(a.1)); // 降順
            
            for (pass_name, duration) in passes {
                report.push_str(&format!("  {}: {:?}\n", pass_name, duration));
            }
        }
        
        report
    }
    
    /// プロファイリングレポートをファイルに書き込み
    pub fn write_report<P: AsRef<Path>>(&self, path: &Option<P>) -> std::io::Result<()> {
        if !self.enabled || self.completed_events.is_empty() {
            return Ok(());
        }
        
        let report = self.get_report();
        
        if let Some(p) = path {
            let mut file = File::create(p)?;
            file.write_all(report.as_bytes())?;
        } else {
            println!("{}", report);
        }
        
        Ok(())
    }
    
    /// プロファイラーをリセット
    pub fn reset(&mut self) {
        self.active_events.clear();
        self.completed_events.clear();
        self.pass_durations.clear();
    }
} 
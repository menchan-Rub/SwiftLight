//! # SwiftLight言語の数学モジュール
//! 
//! このモジュールは基本的な数学関数と定数を提供します。
//! 三角関数、指数関数、対数関数などの一般的な数学演算が含まれています。

/// 数学定数
pub mod constants {
    /// 円周率
    pub const PI: f64 = std::f64::consts::PI;
    
    /// 自然対数の底
    pub const E: f64 = std::f64::consts::E;
    
    /// 黄金比 (1 + sqrt(5)) / 2
    pub const GOLDEN_RATIO: f64 = 1.618033988749895;
    
    /// 2の平方根
    pub const SQRT_2: f64 = std::f64::consts::SQRT_2;
    
    /// 10の自然対数
    pub const LN_10: f64 = std::f64::consts::LN_10;
    
    /// 2の自然対数
    pub const LN_2: f64 = std::f64::consts::LN_2;
    
    /// イプシロン（浮動小数点の最小単位）
    pub const EPSILON: f64 = std::f64::EPSILON;
}

/// 基本的な数学関数
pub mod functions {
    /// 絶対値を計算
    pub fn abs(x: f64) -> f64 {
        x.abs()
    }
    
    /// 正負の符号を返す（-1, 0, 1）
    pub fn signum(x: f64) -> f64 {
        x.signum()
    }
    
    /// 最大値を返す
    pub fn max(a: f64, b: f64) -> f64 {
        if a > b { a } else { b }
    }
    
    /// 最小値を返す
    pub fn min(a: f64, b: f64) -> f64 {
        if a < b { a } else { b }
    }
    
    /// 値を指定した範囲に制限する
    pub fn clamp(x: f64, min: f64, max: f64) -> f64 {
        if x < min { min } else if x > max { max } else { x }
    }
    
    /// 平方根を計算
    pub fn sqrt(x: f64) -> f64 {
        x.sqrt()
    }
    
    /// 立方根を計算
    pub fn cbrt(x: f64) -> f64 {
        x.cbrt()
    }
    
    /// 指定した値の冪を計算
    pub fn pow(x: f64, y: f64) -> f64 {
        x.powf(y)
    }
    
    /// 整数冪を計算
    pub fn powi(x: f64, n: i32) -> f64 {
        x.powi(n)
    }
    
    /// 四捨五入
    pub fn round(x: f64) -> f64 {
        x.round()
    }
    
    /// 切り捨て
    pub fn floor(x: f64) -> f64 {
        x.floor()
    }
    
    /// 切り上げ
    pub fn ceil(x: f64) -> f64 {
        x.ceil()
    }
    
    /// 小数部分を抽出
    pub fn fract(x: f64) -> f64 {
        x.fract()
    }
    
    /// 値を指定した小数点以下の桁数に丸める
    pub fn round_to_precision(x: f64, precision: usize) -> f64 {
        let factor = 10.0_f64.powi(precision as i32);
        (x * factor).round() / factor
    }
}

/// 指数・対数関数
pub mod exp_log {
    /// 指数関数 (e^x)
    pub fn exp(x: f64) -> f64 {
        x.exp()
    }
    
    /// 2の冪乗 (2^x)
    pub fn exp2(x: f64) -> f64 {
        x.exp2()
    }
    
    /// 自然対数 (ln x)
    pub fn ln(x: f64) -> f64 {
        x.ln()
    }
    
    /// 底が10の対数 (log10 x)
    pub fn log10(x: f64) -> f64 {
        x.log10()
    }
    
    /// 底が2の対数 (log2 x)
    pub fn log2(x: f64) -> f64 {
        x.log2()
    }
    
    /// 指定した底の対数 (log_base x)
    pub fn log(x: f64, base: f64) -> f64 {
        x.ln() / base.ln()
    }
    
    /// 双曲線正弦
    pub fn sinh(x: f64) -> f64 {
        x.sinh()
    }
    
    /// 双曲線余弦
    pub fn cosh(x: f64) -> f64 {
        x.cosh()
    }
    
    /// 双曲線正接
    pub fn tanh(x: f64) -> f64 {
        x.tanh()
    }
}

/// 三角関数
pub mod trig {
    use super::constants::PI;

    /// 度をラジアンに変換
    pub fn to_radians(degrees: f64) -> f64 {
        degrees * PI / 180.0
    }
    
    /// ラジアンを度に変換
    pub fn to_degrees(radians: f64) -> f64 {
        radians * 180.0 / PI
    }
    
    /// 正弦関数
    pub fn sin(x: f64) -> f64 {
        x.sin()
    }
    
    /// 余弦関数
    pub fn cos(x: f64) -> f64 {
        x.cos()
    }
    
    /// 正接関数
    pub fn tan(x: f64) -> f64 {
        x.tan()
    }
    
    /// 逆正弦関数
    pub fn asin(x: f64) -> f64 {
        x.asin()
    }
    
    /// 逆余弦関数
    pub fn acos(x: f64) -> f64 {
        x.acos()
    }
    
    /// 逆正接関数
    pub fn atan(x: f64) -> f64 {
        x.atan()
    }
    
    /// 2引数の逆正接関数
    pub fn atan2(y: f64, x: f64) -> f64 {
        y.atan2(x)
    }
}

/// 統計関数
pub mod stats {
    use crate::core::collections::Vec;

    /// 平均値を計算
    pub fn mean(values: &[f64]) -> Option<f64> {
        if values.is_empty() {
            return None;
        }
        
        let sum = values.iter().sum::<f64>();
        Some(sum / values.len() as f64)
    }
    
    /// 中央値を計算
    pub fn median(values: &[f64]) -> Option<f64> {
        if values.is_empty() {
            return None;
        }
        
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        
        let mid = values.len() / 2;
        if values.len() % 2 == 0 {
            // 偶数の場合は中央の2つの平均
            Some((sorted[mid - 1] + sorted[mid]) / 2.0)
        } else {
            // 奇数の場合は中央の値
            Some(sorted[mid])
        }
    }
    
    /// 分散を計算
    pub fn variance(values: &[f64]) -> Option<f64> {
        if values.is_empty() {
            return None;
        }
        
        let mean = mean(values)?;
        let sum_squared_diff = values.iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>();
            
        Some(sum_squared_diff / values.len() as f64)
    }
    
    /// 標準偏差を計算
    pub fn std_dev(values: &[f64]) -> Option<f64> {
        variance(values).map(|v| v.sqrt())
    }
    
    /// 最大値を取得
    pub fn max(values: &[f64]) -> Option<f64> {
        values.iter().copied().fold(None, |max, x| {
            match max {
                None => Some(x),
                Some(max_val) => Some(if x > max_val { x } else { max_val }),
            }
        })
    }
    
    /// 最小値を取得
    pub fn min(values: &[f64]) -> Option<f64> {
        values.iter().copied().fold(None, |min, x| {
            match min {
                None => Some(x),
                Some(min_val) => Some(if x < min_val { x } else { min_val }),
            }
        })
    }
}

/// 乱数生成
pub mod random {
    use std::time::{SystemTime, UNIX_EPOCH};

    // シンプルな線形合同法ジェネレータ
    struct Lcg {
        state: u64,
        a: u64,
        c: u64,
        m: u64,
    }

    impl Lcg {
        fn new(seed: u64) -> Self {
            Self {
                state: seed,
                a: 6364136223846793005,  // 乗数
                c: 1442695040888963407,  // 増分
                m: 0,                    // モジュロ (2^64を使用するが、wrapping演算で自動的に処理されるため0とする)
            }
        }
        
        fn next(&mut self) -> u64 {
            // wrapping_mul と wrapping_add で自動的に2^64のモジュロ演算が行われる
            self.state = self.a.wrapping_mul(self.state).wrapping_add(self.c);
            self.state
        }
        
        fn next_f64(&mut self) -> f64 {
            let max_value = self.m as f64;
            self.next() as f64 / max_value
        }
    }
    
    // スレッドローカルなジェネレータ
    thread_local! {
        static GENERATOR: std::cell::RefCell<Lcg> = {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64;
            std::cell::RefCell::new(Lcg::new(now))
        };
    }
    
    /// 0から1までの乱数を生成
    pub fn random() -> f64 {
        GENERATOR.with(|gen| gen.borrow_mut().next_f64())
    }
    
    /// 指定した範囲の乱数を生成
    pub fn random_range(min: f64, max: f64) -> f64 {
        let r = random();
        min + (max - min) * r
    }
    
    /// 指定した範囲の整数乱数を生成
    pub fn random_int(min: i32, max: i32) -> i32 {
        let r = random();
        min + (r * (max - min + 1) as f64) as i32
    }
    
    /// シードを設定
    pub fn set_seed(seed: u64) {
        GENERATOR.with(|gen| {
            *gen.borrow_mut() = Lcg::new(seed);
        });
    }
} 
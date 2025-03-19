use std::path::{Path, PathBuf};
use anyhow::{Result, anyhow, Context};
use serde::{Serialize, Deserialize};

use crate::manifest::Manifest;
use crate::validation::ValidationResult;

/// パッケージを表す構造体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    /// パッケージ名
    pub name: String,
    /// バージョン
    pub version: String,
    /// 説明
    pub description: Option<String>,
    /// 作者
    pub author: Option<String>,
    /// ライセンス
    pub license: Option<String>,
    /// パッケージのタイプ
    pub package_type: Option<String>,
}

/// パッケージ検証結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageVerificationResult {
    /// 検証が成功したかどうか
    pub success: bool,
    /// 問題リスト
    pub issues: Vec<String>,
}

impl PackageVerificationResult {
    /// 問題があるかどうかを返します
    pub fn has_issues(&self) -> bool {
        !self.issues.is_empty()
    }
}

/// パッケージを検証
pub fn verify_package(package_path: &Path) -> Result<PackageVerificationResult> {
    // マニフェストを読み込む
    let manifest_path = package_path.join("swiftlight.toml");
    if !manifest_path.exists() {
        return Ok(PackageVerificationResult {
            success: false,
            issues: vec!["マニフェストファイルが見つかりません".to_string()],
        });
    }

    let manifest = Manifest::load(&manifest_path)?;
    
    // バリデーションの実行
    let mut result = PackageVerificationResult {
        success: true,
        issues: Vec::new(),
    };

    // マニフェストの必須フィールドを確認
    if manifest.package.name.is_empty() {
        result.success = false;
        result.issues.push("パッケージ名が指定されていません".to_string());
    }

    if manifest.package.version.is_empty() {
        result.success = false;
        result.issues.push("バージョンが指定されていません".to_string());
    }

    // 必要ファイルの存在確認
    let readme_exists = package_path.join("README.md").exists();
    if !readme_exists {
        result.issues.push("README.mdが見つかりません".to_string());
    }

    // ライセンスファイルの確認
    let license_exists = package_path.join("LICENSE").exists();
    if !license_exists && manifest.package.license.is_some() {
        result.issues.push("ライセンスが指定されていますが、LICENSEファイルが見つかりません".to_string());
    }

    Ok(result)
}

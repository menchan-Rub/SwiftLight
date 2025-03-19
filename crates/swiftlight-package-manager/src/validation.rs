use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use anyhow::{Result, anyhow, Context};
use semver::Version;
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use glob::glob;

use crate::manifest::Manifest;
use crate::dependency::{Dependency, DependencyType, DependencySource};

/// バリデーション結果
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationResult {
    /// エラー
    pub errors: Vec<ValidationError>,
    /// 深刻度ごとの問題リスト
    pub issues: HashMap<ValidationSeverity, Vec<ValidationIssue>>,
    /// 検証されたファイル数
    pub files_checked: usize,
    /// 検証完了
    pub completed: bool,
}

/// バリデーション問題
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// 問題コード
    pub code: String,
    /// 問題の説明
    pub message: String,
    /// 問題のある場所
    pub location: Option<ValidationLocation>,
    /// 解決策の提案
    pub suggestion: Option<String>,
}

/// バリデーション問題の場所
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationLocation {
    /// ファイルパス
    pub file: Option<PathBuf>,
    /// 行番号
    pub line: Option<usize>,
    /// カラム番号
    pub column: Option<usize>,
}

/// バリデーション問題の深刻度
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ValidationSeverity {
    /// エラー
    Error,
    /// 警告
    Warning,
    /// 情報
    Info,
    /// ヒント
    Hint,
}

/// パッケージバリデータ
#[derive(Debug)]
pub struct PackageValidator {
    /// バリデーションルール
    rules: Vec<Box<dyn ValidationRule>>,
}

impl PackageValidator {
    /// 新しいバリデータを作成
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
        }
    }

    /// バリデーションルールを追加
    pub fn add_rule<R: ValidationRule + 'static>(&mut self, rule: R) {
        self.rules.push(Box::new(rule));
    }

    /// パッケージを検証
    pub fn validate(&self, manifest: &Manifest) -> ValidationResult {
        let mut result = ValidationResult::default();

        for rule in &self.rules {
            if let Err(e) = rule.validate(manifest) {
                result.errors.push(ValidationError {
                    rule_name: rule.name().to_string(),
                    message: e.to_string(),
                });
            }
        }

        result
    }
}

/// バリデーションルール
pub trait ValidationRule: std::fmt::Debug {
    /// ルール名を取得
    fn name(&self) -> &str;
    /// パッケージを検証
    fn validate(&self, manifest: &Manifest) -> Result<()>;
}

/// バリデーションエラー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// ルール名
    pub rule_name: String,
    /// エラーメッセージ
    pub message: String,
}

impl ValidationResult {
    /// 新しいバリデーション結果を作成
    pub fn new() -> Self {
        let mut issues = HashMap::new();
        issues.insert(ValidationSeverity::Error, Vec::new());
        issues.insert(ValidationSeverity::Warning, Vec::new());
        issues.insert(ValidationSeverity::Info, Vec::new());
        issues.insert(ValidationSeverity::Hint, Vec::new());
        
        ValidationResult {
            errors: Vec::new(),
            issues,
            files_checked: 0,
            completed: false,
        }
    }

    /// 問題を追加
    pub fn add_issue(&mut self, severity: ValidationSeverity, issue: ValidationIssue) {
        self.issues.entry(severity).or_insert_with(Vec::new).push(issue);
    }

    /// エラーがあるか
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// 警告があるか
    pub fn has_warnings(&self) -> bool {
        !self.issues[&ValidationSeverity::Warning].is_empty()
    }

    /// 問題数を取得
    pub fn issue_count(&self) -> usize {
        self.issues.values().map(|v| v.len()).sum()
    }

    /// エラー数を取得
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// 警告数を取得
    pub fn warning_count(&self) -> usize {
        self.issues[&ValidationSeverity::Warning].len()
    }
}

/// バリデーションオプション
#[derive(Debug, Clone)]
pub struct ValidationOptions {
    /// マニフェストのバリデーション
    pub validate_manifest: bool,
    /// 依存関係のバリデーション
    pub validate_dependencies: bool,
    /// ソースコードのバリデーション
    pub validate_source: bool,
    /// 構造のバリデーション
    pub validate_structure: bool,
    /// リソースのバリデーション
    pub validate_resources: bool,
    /// テストのバリデーション
    pub validate_tests: bool,
    /// エラーで停止するか
    pub fail_on_error: bool,
    /// 警告で停止するか
    pub fail_on_warning: bool,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        ValidationOptions {
            validate_manifest: true,
            validate_dependencies: true,
            validate_source: true,
            validate_structure: true,
            validate_resources: true,
            validate_tests: true,
            fail_on_error: true,
            fail_on_warning: false,
        }
    }
}

/// パッケージを検証
pub fn validate_package(project_dir: &Path, options: &ValidationOptions) -> Result<ValidationResult> {
    let mut result = ValidationResult::new();
    
    // マニフェストの検証
    if options.validate_manifest {
        validate_package_manifest(project_dir, &mut result)?;
    }
    
    // 依存関係の検証
    if options.validate_dependencies {
        validate_package_dependencies(project_dir, &mut result)?;
    }
    
    // ソースコードの検証
    if options.validate_source {
        validate_package_source(project_dir, &mut result)?;
    }
    
    // 構造の検証
    if options.validate_structure {
        validate_package_structure(project_dir, &mut result)?;
    }
    
    // リソースの検証
    if options.validate_resources {
        validate_package_resources(project_dir, &mut result)?;
    }
    
    // テストの検証
    if options.validate_tests {
        validate_package_tests(project_dir, &mut result)?;
    }
    
    // エラーチェック
    if options.fail_on_error && result.has_errors() {
        return Err(anyhow!("パッケージの検証に失敗しました: {} エラーが見つかりました", result.error_count()));
    }
    
    // 警告チェック
    if options.fail_on_warning && result.has_warnings() {
        return Err(anyhow!("パッケージの検証に失敗しました: {} 警告が見つかりました", result.warning_count()));
    }
    
    result.completed = true;
    Ok(result)
}

/// マニフェストを検証
fn validate_package_manifest(project_dir: &Path, result: &mut ValidationResult) -> Result<()> {
    let manifest_path = project_dir.join("swiftlight.toml");
    
    if !manifest_path.exists() {
        result.add_issue(
            ValidationSeverity::Error,
            ValidationIssue {
                code: "V001".to_string(),
                message: "マニフェストファイル(swiftlight.toml)が見つかりません".to_string(),
                location: Some(ValidationLocation {
                    file: Some(project_dir.to_path_buf()),
                    line: None,
                    column: None,
                }),
                suggestion: Some("swiftlight init コマンドを使用して新しいパッケージを初期化してください".to_string()),
            },
        );
        return Ok(());
    }
    
    // マニフェストを読み込む
    let manifest = match Manifest::load(&manifest_path) {
        Ok(m) => m,
        Err(e) => {
            result.add_issue(
                ValidationSeverity::Error,
                ValidationIssue {
                    code: "V002".to_string(),
                    message: format!("マニフェストの解析に失敗しました: {}", e),
                    location: Some(ValidationLocation {
                        file: Some(manifest_path.clone()),
                        line: None,
                        column: None,
                    }),
                    suggestion: Some("マニフェストの形式を確認してください".to_string()),
                },
            );
            return Ok(());
        }
    };
    
    // 必須フィールドの検証
    if manifest.package.name.is_empty() {
        result.add_issue(
            ValidationSeverity::Error,
            ValidationIssue {
                code: "V003".to_string(),
                message: "パッケージ名が指定されていません".to_string(),
                location: Some(ValidationLocation {
                    file: Some(manifest_path.clone()),
                    line: None,
                    column: None,
                }),
                suggestion: Some("package.name フィールドを追加してください".to_string()),
            },
        );
    }
    
    if manifest.package.version.is_empty() {
        result.add_issue(
            ValidationSeverity::Error,
            ValidationIssue {
                code: "V004".to_string(),
                message: "バージョンが指定されていません".to_string(),
                location: Some(ValidationLocation {
                    file: Some(manifest_path.clone()),
                    line: None,
                    column: None,
                }),
                suggestion: Some("package.version フィールドを追加してください (例: \"0.1.0\")".to_string()),
            },
        );
    } else {
        // バージョンの妥当性をチェック
        if let Err(e) = Version::parse(&manifest.package.version) {
            result.add_issue(
                ValidationSeverity::Error,
                ValidationIssue {
                    code: "V005".to_string(),
                    message: format!("無効なバージョン形式です: {}", e),
                    location: Some(ValidationLocation {
                        file: Some(manifest_path.clone()),
                        line: None,
                        column: None,
                    }),
                    suggestion: Some("セマンティックバージョニング形式を使用してください (例: \"0.1.0\")".to_string()),
                },
            );
        }
    }
    
    // 説明がない場合は警告
    if manifest.package.description.is_none() || manifest.package.description.as_ref().unwrap().is_empty() {
        result.add_issue(
            ValidationSeverity::Warning,
            ValidationIssue {
                code: "V006".to_string(),
                message: "パッケージの説明がありません".to_string(),
                location: Some(ValidationLocation {
                    file: Some(manifest_path.clone()),
                    line: None,
                    column: None,
                }),
                suggestion: Some("package.description フィールドを追加して、パッケージの目的を説明してください".to_string()),
            },
        );
    }
    
    // ライセンスがない場合は警告
    if manifest.package.license.is_none() || manifest.package.license.as_ref().unwrap().is_empty() {
        result.add_issue(
            ValidationSeverity::Warning,
            ValidationIssue {
                code: "V007".to_string(),
                message: "ライセンスが指定されていません".to_string(),
                location: Some(ValidationLocation {
                    file: Some(manifest_path.clone()),
                    line: None,
                    column: None,
                }),
                suggestion: Some("package.license フィールドを追加してください (例: \"MIT\" or \"Apache-2.0\")".to_string()),
            },
        );
    }
    
    // ライブラリとバイナリの両方が存在するかチェック
    if manifest.lib.is_none() && manifest.bin.is_empty() {
        result.add_issue(
            ValidationSeverity::Warning,
            ValidationIssue {
                code: "V008".to_string(),
                message: "ライブラリもバイナリも指定されていません".to_string(),
                location: Some(ValidationLocation {
                    file: Some(manifest_path.clone()),
                    line: None,
                    column: None,
                }),
                suggestion: Some("少なくとも一つのライブラリかバイナリを指定してください".to_string()),
            },
        );
    }
    
    result.files_checked += 1;
    Ok(())
}

/// 依存関係を検証
fn validate_package_dependencies(project_dir: &Path, result: &mut ValidationResult) -> Result<()> {
    let manifest_path = project_dir.join("swiftlight.toml");
    
    if !manifest_path.exists() {
        return Ok(());  // マニフェストの検証で既にエラーが出ているため、ここではスキップ
    }
    
    // マニフェストを読み込む
    let manifest = match Manifest::load(&manifest_path) {
        Ok(m) => m,
        Err(_) => return Ok(()),  // マニフェストの検証で既にエラーが出ているため、ここではスキップ
    };
    
    // 全依存関係を取得
    let all_deps = match manifest.get_all_dependencies() {
        Ok(deps) => deps,
        Err(e) => {
            result.add_issue(
                ValidationSeverity::Error,
                ValidationIssue {
                    code: "V009".to_string(),
                    message: format!("依存関係の解析に失敗しました: {}", e),
                    location: Some(ValidationLocation {
                        file: Some(manifest_path.clone()),
                        line: None,
                        column: None,
                    }),
                    suggestion: Some("依存関係の形式を確認してください".to_string()),
                },
            );
            return Ok(());
        }
    };
    
    // 各依存関係をチェック
    for dep in &all_deps {
        // バージョン要求がない場合は警告
        if dep.version_req.is_none() {
            result.add_issue(
                ValidationSeverity::Warning,
                ValidationIssue {
                    code: "V010".to_string(),
                    message: format!("依存関係 '{}' にバージョン要求がありません", dep.name),
                    location: Some(ValidationLocation {
                        file: Some(manifest_path.clone()),
                        line: None,
                        column: None,
                    }),
                    suggestion: Some(format!("'{}' の特定のバージョンまたはバージョン範囲を指定してください", dep.name)),
                },
            );
        }
        
        // パス依存関係の場合、パスが存在するかチェック
        if let DependencySource::Path { ref path } = dep.source {
            let full_path = if path.is_absolute() {
                path.clone()
            } else {
                project_dir.join(path)
            };
            
            if !full_path.exists() {
                result.add_issue(
                    ValidationSeverity::Error,
                    ValidationIssue {
                        code: "V011".to_string(),
                        message: format!("依存関係 '{}' のパス '{}' が存在しません", dep.name, path.display()),
                        location: Some(ValidationLocation {
                            file: Some(manifest_path.clone()),
                            line: None,
                            column: None,
                        }),
                        suggestion: Some(format!("パス '{}' が正しいことを確認するか、依存関係を修正してください", path.display())),
                    },
                );
            } else if !full_path.join("swiftlight.toml").exists() {
                result.add_issue(
                    ValidationSeverity::Error,
                    ValidationIssue {
                        code: "V012".to_string(),
                        message: format!("依存関係 '{}' のパス '{}' に swiftlight.toml がありません", dep.name, path.display()),
                        location: Some(ValidationLocation {
                            file: Some(manifest_path.clone()),
                            line: None,
                            column: None,
                        }),
                        suggestion: Some(format!("パス '{}' が SwiftLight パッケージであることを確認してください", path.display())),
                    },
                );
            }
        }
    }
    
    result.files_checked += 1;
    Ok(())
}

/// ソースコードを検証
fn validate_package_source(project_dir: &Path, result: &mut ValidationResult) -> Result<()> {
    let src_dir = project_dir.join("src");
    
    if !src_dir.exists() {
        result.add_issue(
            ValidationSeverity::Error,
            ValidationIssue {
                code: "V020".to_string(),
                message: "ソースディレクトリ 'src' が見つかりません".to_string(),
                location: Some(ValidationLocation {
                    file: Some(project_dir.to_path_buf()),
                    line: None,
                    column: None,
                }),
                suggestion: Some("'src' ディレクトリを作成してください".to_string()),
            },
        );
        return Ok(());
    }
    
    // mainファイルかlibファイルが存在するかチェック
    let main_swift = src_dir.join("main.swift");
    let lib_swift = src_dir.join("lib.swift");
    
    if !main_swift.exists() && !lib_swift.exists() {
        result.add_issue(
            ValidationSeverity::Warning,
            ValidationIssue {
                code: "V021".to_string(),
                message: "エントリポイント (main.swift または lib.swift) が見つかりません".to_string(),
                location: Some(ValidationLocation {
                    file: Some(src_dir.clone()),
                    line: None,
                    column: None,
                }),
                suggestion: Some("'src/main.swift' または 'src/lib.swift' を作成してください".to_string()),
            },
        );
    }
    
    // ここでは実際のSwiftコードの構文チェックは行わない（それは別のツールの仕事）
    // 実際の実装では、SwiftシンタックスチェッカーをDIでモック可能にするなど
    
    result.files_checked += 1;
    Ok(())
}

/// プロジェクト構造を検証
fn validate_package_structure(project_dir: &Path, result: &mut ValidationResult) -> Result<()> {
    // プロジェクト構造の基本ディレクトリをチェック
    let expected_dirs = [
        "src",
        "tests",
        "docs",
        "examples",
        "resources",
    ];
    
    for dir in &expected_dirs {
        let dir_path = project_dir.join(dir);
        if !dir_path.exists() {
            let severity = if *dir == "src" {
                ValidationSeverity::Error
            } else {
                ValidationSeverity::Info
            };
            
            result.add_issue(
                severity,
                ValidationIssue {
                    code: format!("V03{}", expected_dirs.iter().position(|&d| d == *dir).unwrap_or(0)),
                    message: format!("ディレクトリ '{}' が見つかりません", dir),
                    location: Some(ValidationLocation {
                        file: Some(project_dir.to_path_buf()),
                        line: None,
                        column: None,
                    }),
                    suggestion: Some(format!("'{}' ディレクトリを作成することを検討してください", dir)),
                },
            );
        }
    }
    
    // READMEファイルをチェック
    let readme_file = project_dir.join("README.md");
    if !readme_file.exists() {
        result.add_issue(
            ValidationSeverity::Warning,
            ValidationIssue {
                code: "V040".to_string(),
                message: "README.md ファイルが見つかりません".to_string(),
                location: Some(ValidationLocation {
                    file: Some(project_dir.to_path_buf()),
                    line: None,
                    column: None,
                }),
                suggestion: Some("README.md ファイルを作成して、プロジェクトについて説明してください".to_string()),
            },
        );
    }
    
    // ライセンスファイルをチェック
    let license_file = project_dir.join("LICENSE");
    if !license_file.exists() {
        result.add_issue(
            ValidationSeverity::Info,
            ValidationIssue {
                code: "V041".to_string(),
                message: "LICENSE ファイルが見つかりません".to_string(),
                location: Some(ValidationLocation {
                    file: Some(project_dir.to_path_buf()),
                    line: None,
                    column: None,
                }),
                suggestion: Some("LICENSE ファイルを追加して、プロジェクトのライセンスを明確にしてください".to_string()),
            },
        );
    }
    
    result.files_checked += 1;
    Ok(())
}

/// リソースを検証
fn validate_package_resources(project_dir: &Path, result: &mut ValidationResult) -> Result<()> {
    let resources_dir = project_dir.join("resources");
    
    if !resources_dir.exists() {
        return Ok(());  // リソースディレクトリがない場合はスキップ
    }
    
    // リソースディレクトリが空かチェック
    let entries = std::fs::read_dir(&resources_dir)?;
    let is_empty = entries.count() == 0;
    
    if is_empty {
        result.add_issue(
            ValidationSeverity::Info,
            ValidationIssue {
                code: "V050".to_string(),
                message: "リソースディレクトリが空です".to_string(),
                location: Some(ValidationLocation {
                    file: Some(resources_dir.clone()),
                    line: None,
                    column: None,
                }),
                suggestion: Some("アプリケーションで使用するリソースを追加してください".to_string()),
            },
        );
    }
    
    result.files_checked += 1;
    Ok(())
}

/// テストを検証
fn validate_package_tests(project_dir: &Path, result: &mut ValidationResult) -> Result<()> {
    let tests_dir = project_dir.join("tests");
    
    if !tests_dir.exists() {
        result.add_issue(
            ValidationSeverity::Info,
            ValidationIssue {
                code: "V060".to_string(),
                message: "テストディレクトリが見つかりません".to_string(),
                location: Some(ValidationLocation {
                    file: Some(project_dir.to_path_buf()),
                    line: None,
                    column: None,
                }),
                suggestion: Some("'tests' ディレクトリを作成して、テストを追加することを検討してください".to_string()),
            },
        );
        return Ok(());
    }
    
    // テストファイルが存在するかチェック
    let entries = std::fs::read_dir(&tests_dir)?;
    let test_files: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            if let Some(ext) = e.path().extension() {
                ext == "swift"
            } else {
                false
            }
        })
        .collect();
    
    if test_files.is_empty() {
        result.add_issue(
            ValidationSeverity::Info,
            ValidationIssue {
                code: "V061".to_string(),
                message: "テストファイルが見つかりません".to_string(),
                location: Some(ValidationLocation {
                    file: Some(tests_dir.clone()),
                    line: None,
                    column: None,
                }),
                suggestion: Some("テストファイルを追加することを検討してください".to_string()),
            },
        );
    }
    
    result.files_checked += 1;
    Ok(())
}

/// ファイルチェックサムの検証
pub fn validate_file_checksum(file_path: &Path, expected_checksum: &str) -> Result<bool> {
    let mut file = File::open(file_path)
        .with_context(|| format!("ファイルを開けません: {}", file_path.display()))?;
    
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];
    
    loop {
        let bytes_read = file.read(&mut buffer)
            .with_context(|| format!("ファイルの読み込みに失敗しました: {}", file_path.display()))?;
            
        if bytes_read == 0 {
            break;
        }
        
        hasher.update(&buffer[..bytes_read]);
    }
    
    let result = hasher.finalize();
    let actual_checksum = format!("{:x}", result);
    
    Ok(actual_checksum == expected_checksum)
} 
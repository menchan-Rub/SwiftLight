use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Result, anyhow, Context};
use glob::glob;

use crate::manifest::{Manifest, WorkspaceInfo};
use crate::dependency::DependencyGraph;

/// ワークスペース
#[derive(Debug, Clone)]
pub struct Workspace {
    /// ルートディレクトリ
    pub root_dir: PathBuf,
    /// ルートマニフェスト
    pub root_manifest: Manifest,
    /// メンバーパッケージのマニフェスト
    pub member_manifests: HashMap<String, Manifest>,
    /// メンバーパッケージのディレクトリ
    pub member_dirs: HashMap<String, PathBuf>,
}

/// ワークスペースの状態情報
#[derive(Debug, Clone)]
pub struct WorkspaceStatus {
    /// ワークスペースルート
    pub root_dir: PathBuf,
    /// メンバー数
    pub member_count: usize,
    /// 依存関係数
    pub dependency_count: usize,
    /// 開発依存関係数
    pub dev_dependency_count: usize,
    /// 解決済みパッケージ数
    pub resolved_package_count: usize,
    /// アウトオブデート依存数
    pub outdated_dependency_count: usize,
}

impl Workspace {
    /// 新しいワークスペースを検出
    pub fn discover(start_dir: &Path) -> Result<Self> {
        // ワークスペースルートを探す
        let root_dir = find_workspace_root(start_dir)?;
        
        // ルートマニフェストを読み込む
        let manifest_path = root_dir.join("swiftlight.toml");
        let root_manifest = Manifest::load(&manifest_path)
            .with_context(|| format!("ワークスペースのルートマニフェストを読み込めません: {}", manifest_path.display()))?;
        
        // ワークスペース設定を取得
        let workspace_info = match &root_manifest.workspace {
            Some(ws) => ws,
            None => return Err(anyhow!("ワークスペース設定がマニフェストにありません")),
        };
        
        // メンバーパッケージを検索
        let (member_manifests, member_dirs) = find_workspace_members(&root_dir, workspace_info)?;
        
        Ok(Workspace {
            root_dir,
            root_manifest,
            member_manifests,
            member_dirs,
        })
    }

    /// ワークスペースの状態を取得
    pub fn get_status(&self) -> Result<WorkspaceStatus> {
        let mut dependency_count = 0;
        let mut dev_dependency_count = 0;
        let mut resolved_package_count = 0;
        let mut outdated_dependency_count = 0;
        
        // すべてのメンバーの依存関係を集計
        for manifest in self.member_manifests.values() {
            dependency_count += manifest.dependencies.len();
            dev_dependency_count += manifest.dev_dependencies.len();
            
            // 実際の実装では、解決済みパッケージと古い依存関係を計算
            // ここではモックの実装
            resolved_package_count += manifest.dependencies.len() + manifest.dev_dependencies.len();
            outdated_dependency_count += (manifest.dependencies.len() + manifest.dev_dependencies.len()) / 5; // 20%が古いと仮定
        }
        
        Ok(WorkspaceStatus {
            root_dir: self.root_dir.clone(),
            member_count: self.member_manifests.len(),
            dependency_count,
            dev_dependency_count,
            resolved_package_count,
            outdated_dependency_count,
        })
    }

    /// ワークスペースの依存関係グラフを取得
    pub fn get_dependency_graph(&self, include_dev: bool) -> Result<DependencyGraph> {
        let mut graph = DependencyGraph::new();
        
        // 各メンバーの依存関係を解決して結合
        for (name, manifest) in &self.member_manifests {
            // 依存関係を取得
            let deps = manifest.get_dependencies()?;
            let dev_deps = if include_dev {
                manifest.get_dev_dependencies()?
            } else {
                Vec::new()
            };
            
            // 依存関係グラフを作成
            let all_deps = [deps, dev_deps].concat();
            let member_graph = crate::dependency::resolve_dependencies(&all_deps, include_dev)?;
            
            // グラフをマージ（実際の実装ではもっと複雑になるが、ここではシンプルに）
            for (id, dep) in member_graph.nodes {
                if !graph.nodes.contains_key(&id) {
                    graph.add_node(id.clone(), dep);
                }
            }
            
            for (from, to_list) in member_graph.edges {
                for to in to_list {
                    graph.add_edge(from.clone(), to);
                }
            }
            
            for direct in member_graph.direct_dependencies {
                graph.add_direct_dependency(direct);
            }
        }
        
        Ok(graph)
    }

    /// すべてのメンバーパッケージをビルド
    pub fn build_all(&self, release: bool) -> Result<()> {
        for (name, dir) in &self.member_dirs {
            println!("ビルド中: {}", name);
            
            // ビルドオプションを作成
            let options = crate::build::BuildOptions {
                mode: if release {
                    crate::build::BuildMode::Release
                } else {
                    crate::build::BuildMode::Debug
                },
                ..Default::default()
            };
            
            // パッケージをビルド
            crate::build::build_package(dir, &options, &crate::config::Config::default())?;
        }
        
        Ok(())
    }

    /// すべてのメンバーパッケージに対してコマンドを実行
    pub fn run_for_each_member<F>(&self, f: F) -> Result<()>
    where
        F: Fn(&str, &Path, &Manifest) -> Result<()>,
    {
        for (name, dir) in &self.member_dirs {
            let manifest = &self.member_manifests[name];
            f(name, dir, manifest)?;
        }
        
        Ok(())
    }

    /// 共通の依存関係を検出
    pub fn find_common_dependencies(&self) -> Result<HashMap<String, usize>> {
        let mut dependency_counts = HashMap::new();
        
        // 各メンバーの依存関係を集計
        for manifest in self.member_manifests.values() {
            let deps = manifest.get_dependencies()?;
            
            for dep in deps {
                let count = dependency_counts.entry(dep.name.clone()).or_insert(0);
                *count += 1;
            }
        }
        
        // 複数のパッケージで使用されている依存関係だけをフィルタリング
        let common_deps: HashMap<_, _> = dependency_counts.into_iter()
            .filter(|(_, count)| *count > 1)
            .collect();
            
        Ok(common_deps)
    }

    /// 不一致のある依存関係を検出
    pub fn find_dependency_mismatches(&self) -> Result<Vec<(String, Vec<(String, String)>)>> {
        let mut dependency_versions = HashMap::<String, HashMap<String, String>>::new();
        
        // 各メンバーの依存関係バージョンを収集
        for (pkg_name, manifest) in &self.member_manifests {
            let deps = manifest.get_dependencies()?;
            
            for dep in deps {
                let versions = dependency_versions.entry(dep.name.clone()).or_insert_with(HashMap::new);
                let version = match &dep.version_req {
                    Some(v) => v.to_string(),
                    None => "未指定".to_string(),
                };
                versions.insert(pkg_name.clone(), version);
            }
        }
        
        // 異なるバージョンがある依存関係を検出
        let mut mismatches = Vec::new();
        
        for (dep_name, versions) in dependency_versions {
            if versions.len() > 1 {
                let mut unique_versions = HashSet::new();
                for v in versions.values() {
                    unique_versions.insert(v);
                }
                
                if unique_versions.len() > 1 {
                    let version_list: Vec<_> = versions.into_iter().collect();
                    mismatches.push((dep_name, version_list));
                }
            }
        }
        
        Ok(mismatches)
    }
}

/// ワークスペースのルートディレクトリを探す
pub fn find_workspace_root(start_dir: &Path) -> Result<PathBuf> {
    let mut current_dir = start_dir.to_path_buf();
    
    loop {
        let manifest_path = current_dir.join("swiftlight.toml");
        if manifest_path.exists() {
            // マニフェストを読み込む
            let manifest = Manifest::load(&manifest_path)?;
            
            // ワークスペース設定があれば、これがルート
            if manifest.workspace.is_some() {
                return Ok(current_dir);
            }
        }
        
        // 親ディレクトリがなければ終了
        if !current_dir.pop() {
            break;
        }
    }
    
    Err(anyhow!("ワークスペースのルートディレクトリが見つかりません"))
}

/// ワークスペースメンバーを探す
fn find_workspace_members(
    root_dir: &Path,
    workspace_info: &WorkspaceInfo,
) -> Result<(HashMap<String, Manifest>, HashMap<String, PathBuf>)> {
    let mut member_manifests = HashMap::new();
    let mut member_dirs = HashMap::new();
    
    // 除外パターンをセット
    let excludes: HashSet<_> = workspace_info.exclude.iter().cloned().collect();
    
    // 各メンバーパターンを処理
    for pattern in &workspace_info.members {
        let glob_pattern = format!("{}/{}/swiftlight.toml", root_dir.display(), pattern);
        
        for entry in glob(&glob_pattern)? {
            let manifest_path = entry?;
            let parent_dir = manifest_path.parent().unwrap().to_path_buf();
            let rel_path = parent_dir.strip_prefix(root_dir)?.to_string_lossy().to_string();
            
            // 除外リストにあるか確認
            if excludes.contains(&rel_path) {
                continue;
            }
            
            // マニフェストを読み込む
            let manifest = Manifest::load(&manifest_path)?;
            let package_name = manifest.package.name.clone();
            
            member_manifests.insert(package_name.clone(), manifest);
            member_dirs.insert(package_name, parent_dir);
        }
    }
    
    Ok((member_manifests, member_dirs))
}

/// ワークスペースの作成
pub fn create_workspace(
    root_dir: &Path,
    name: &str,
    members: Vec<String>,
) -> Result<()> {
    // ディレクトリを作成
    fs::create_dir_all(root_dir)?;
    
    // ルートマニフェストを作成
    let mut manifest = Manifest::new(name, "0.1.0", vec!["SwiftLight Team".to_string()]);
    
    // ワークスペース設定を追加
    let workspace_info = WorkspaceInfo {
        members,
        exclude: Vec::new(),
        inheritance: None,
    };
    
    manifest.workspace = Some(workspace_info);
    
    // マニフェストを保存
    let manifest_path = root_dir.join("swiftlight.toml");
    manifest.save(&manifest_path)?;
    
    // READMEを作成
    let readme_path = root_dir.join("README.md");
    let readme_content = format!("# {}\n\nSwiftLightパッケージマネージャーで管理されるワークスペースです。\n", name);
    fs::write(readme_path, readme_content)?;
    
    Ok(())
}

/// ワークスペースにパッケージを追加
pub fn add_package_to_workspace(
    workspace_root: &Path,
    package_name: &str,
    package_path: &str,
) -> Result<()> {
    // ワークスペースのマニフェストを読み込む
    let manifest_path = workspace_root.join("swiftlight.toml");
    let mut manifest = Manifest::load(&manifest_path)?;
    
    // ワークスペース設定を取得
    let workspace = match &mut manifest.workspace {
        Some(ws) => ws,
        None => return Err(anyhow!("ワークスペース設定がマニフェストにありません")),
    };
    
    // メンバーリストに追加（重複をチェック）
    if !workspace.members.contains(&package_path.to_string()) {
        workspace.members.push(package_path.to_string());
    }
    
    // マニフェストを保存
    manifest.save(&manifest_path)?;
    
    // パッケージディレクトリを作成
    let package_dir = workspace_root.join(package_path);
    fs::create_dir_all(&package_dir)?;
    
    // パッケージマニフェストを作成
    let pkg_manifest = crate::manifest::create_new_manifest(
        package_name,
        "0.1.0",
        vec!["SwiftLight Team".to_string()],
        Some(format!("{} パッケージ", package_name)),
        Some("2022".to_string()),
        Some("MIT".to_string()),
    );
    
    // パッケージマニフェストを保存
    let pkg_manifest_path = package_dir.join("swiftlight.toml");
    pkg_manifest.save(&pkg_manifest_path)?;
    
    // ソースディレクトリを作成
    let src_dir = package_dir.join("src");
    fs::create_dir_all(&src_dir)?;
    
    // 基本的なソースファイルを作成
    let lib_file = src_dir.join("lib.swift");
    let lib_content = format!("// {} パッケージのコード\n\npublic func hello() -> String {{\n    return \"Hello from {}!\"\n}}\n", package_name, package_name);
    fs::write(lib_file, lib_content)?;
    
    // テストディレクトリを作成
    let tests_dir = package_dir.join("tests");
    fs::create_dir_all(&tests_dir)?;
    
    // 基本的なテストファイルを作成
    let test_file = tests_dir.join("lib_test.swift");
    let test_content = format!("// {} パッケージのテスト\n\nimport XCTest\nimport {}\n\nclass {}Tests: XCTestCase {{\n    func testHello() {{\n        XCTAssertEqual(hello(), \"Hello from {}!\")\n    }}\n}}\n", package_name, package_name, package_name, package_name);
    fs::write(test_file, test_content)?;
    
    Ok(())
}

/// ワークスペースからパッケージを削除
pub fn remove_package_from_workspace(
    workspace_root: &Path,
    package_path: &str,
    delete_files: bool,
) -> Result<()> {
    // ワークスペースのマニフェストを読み込む
    let manifest_path = workspace_root.join("swiftlight.toml");
    let mut manifest = Manifest::load(&manifest_path)?;
    
    // ワークスペース設定を取得
    let workspace = match &mut manifest.workspace {
        Some(ws) => ws,
        None => return Err(anyhow!("ワークスペース設定がマニフェストにありません")),
    };
    
    // メンバーリストから削除
    workspace.members.retain(|m| m != package_path);
    
    // マニフェストを保存
    manifest.save(&manifest_path)?;
    
    // パッケージディレクトリの削除（オプション）
    if delete_files {
        let package_dir = workspace_root.join(package_path);
        if package_dir.exists() {
            fs::remove_dir_all(package_dir)?;
        }
    }
    
    Ok(())
} 
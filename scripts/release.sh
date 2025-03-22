#!/usr/bin/env bash

# SwiftLight リリーススクリプト
# 
# このスクリプトは SwiftLight の新しいバージョンを作成し、配布します。
# リリースバージョンのビルド、テスト、パッケージング、配布までの一連の処理を自動化します。

set -e

# ディレクトリの定義
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
TARGET_DIR="$ROOT_DIR/target"
RELEASE_DIR="$TARGET_DIR/release"
PACKAGE_DIR="$TARGET_DIR/package"

# 色の定義
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
RESET='\033[0m'

# デフォルト設定
VERSION=""
DRY_RUN=false
SKIP_TESTS=false
SKIP_DOCS=false
VERBOSE=false
PLATFORMS=("x86_64-unknown-linux-gnu" "aarch64-unknown-linux-gnu" "x86_64-apple-darwin" "aarch64-apple-darwin" "x86_64-pc-windows-msvc")
SELECTED_PLATFORMS=()

# ヘルプメッセージ
print_help() {
  echo -e "${BLUE}SwiftLight リリーススクリプト${RESET}"
  echo ""
  echo "使用方法: $(basename "$0") [オプション] <バージョン>"
  echo ""
  echo "引数:"
  echo "  <バージョン>              リリースするバージョン番号 (例: 0.1.0)"
  echo ""
  echo "オプション:"
  echo "  -h, --help               このヘルプメッセージを表示"
  echo "  -d, --dry-run            実際のリリース処理は行わず、何が行われるかを表示"
  echo "  -s, --skip-tests         テストをスキップ"
  echo "  -n, --skip-docs          ドキュメント生成をスキップ"
  echo "  -p, --platform <プラットフォーム> ビルドするプラットフォームを指定 (複数指定可)"
  echo "  -v, --verbose            詳細な出力を表示"
  echo ""
  echo "利用可能なプラットフォーム:"
  for platform in "${PLATFORMS[@]}"; do
    echo "  - $platform"
  done
  echo ""
}

# コマンドライン引数のパース
parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -h|--help)
        print_help
        exit 0
        ;;
      -d|--dry-run)
        DRY_RUN=true
        shift
        ;;
      -s|--skip-tests)
        SKIP_TESTS=true
        shift
        ;;
      -n|--skip-docs)
        SKIP_DOCS=true
        shift
        ;;
      -p|--platform)
        if [[ -z "$2" || "$2" == -* ]]; then
          echo -e "${RED}エラー: --platform オプションには引数が必要です${RESET}" >&2
          exit 1
        fi
        
        local valid=false
        for platform in "${PLATFORMS[@]}"; do
          if [[ "$platform" == "$2" ]]; then
            SELECTED_PLATFORMS+=("$2")
            valid=true
            break
          fi
        done
        
        if [[ "$valid" == false ]]; then
          echo -e "${RED}エラー: 無効なプラットフォーム: $2${RESET}" >&2
          print_help
          exit 1
        fi
        
        shift 2
        ;;
      -v|--verbose)
        VERBOSE=true
        shift
        ;;
      -*)
        echo -e "${RED}エラー: 不明なオプション: $1${RESET}" >&2
        print_help
        exit 1
        ;;
      *)
        if [[ -z "$VERSION" ]]; then
          VERSION="$1"
          shift
        else
          echo -e "${RED}エラー: 不明な引数: $1${RESET}" >&2
          print_help
          exit 1
        fi
        ;;
    esac
  done
  
  # バージョンが指定されていない場合はエラー
  if [[ -z "$VERSION" ]]; then
    echo -e "${RED}エラー: バージョン番号を指定してください${RESET}" >&2
    print_help
    exit 1
  fi
  
  # バージョン番号の形式を検証
  if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9]+)?$ ]]; then
    echo -e "${RED}エラー: バージョン番号の形式が無効です。x.y.z[-tag] の形式にしてください (例: 0.1.0 または 0.1.0-alpha)${RESET}" >&2
    exit 1
  fi
  
  # プラットフォームが選択されていない場合は全プラットフォームを選択
  if [[ ${#SELECTED_PLATFORMS[@]} -eq 0 ]]; then
    SELECTED_PLATFORMS=("${PLATFORMS[@]}")
  fi
}

# Gitリポジトリのチェック
check_git_repository() {
  echo -e "${BLUE}Gitリポジトリをチェック中...${RESET}"
  
  # Gitリポジトリかどうかを確認
  if ! git -C "$ROOT_DIR" rev-parse --is-inside-work-tree > /dev/null 2>&1; then
    echo -e "${RED}エラー: $ROOT_DIR はGitリポジトリではありません${RESET}" >&2
    exit 1
  fi
  
  # 変更が未コミットでないことを確認
  if [[ -n "$(git -C "$ROOT_DIR" status --porcelain)" ]]; then
    echo -e "${RED}エラー: コミットされていない変更があります。リリース前にすべての変更をコミットしてください。${RESET}" >&2
    
    if [[ "$DRY_RUN" != true ]]; then
      exit 1
    else
      echo -e "${YELLOW}警告: ドライランモードのため続行します${RESET}"
    fi
  fi
  
  # タグが既に存在していないことを確認
  if git -C "$ROOT_DIR" tag | grep -q "^v$VERSION$"; then
    echo -e "${RED}エラー: タグ v$VERSION は既に存在します${RESET}" >&2
    
    if [[ "$DRY_RUN" != true ]]; then
      exit 1
    else
      echo -e "${YELLOW}警告: ドライランモードのため続行します${RESET}"
    fi
  fi
  
  echo -e "${GREEN}Gitリポジトリのチェックが完了しました${RESET}"
}

# バージョン番号の更新
update_version() {
  echo -e "${BLUE}バージョン番号を更新中... ($VERSION)${RESET}"
  
  if [[ "$DRY_RUN" == true ]]; then
    echo -e "${YELLOW}ドライラン: 次のファイルのバージョン番号を $VERSION に更新します${RESET}"
    echo "- $ROOT_DIR/Cargo.toml"
    
    # cratesディレクトリ内のすべてのCargo.tomlファイル
    find "$ROOT_DIR/crates" -name "Cargo.toml" | while read -r cargo_file; do
      echo "- $cargo_file"
    done
    
    return 0
  fi
  
  # ルートのCargo.tomlのバージョン更新
  sed -i "s/^version = \".*\"/version = \"$VERSION\"/" "$ROOT_DIR/Cargo.toml"
  
  # workspace.package.versionも更新
  sed -i "/\[workspace\.package\]/,/^$/ s/^version = \".*\"/version = \"$VERSION\"/" "$ROOT_DIR/Cargo.toml"
  
  # cratesディレクトリ内のすべてのCargo.tomlファイルのバージョン更新
  find "$ROOT_DIR/crates" -name "Cargo.toml" | while read -r cargo_file; do
    sed -i "s/^version = \".*\"/version = \"$VERSION\"/" "$cargo_file"
  done
  
  echo -e "${GREEN}バージョン番号の更新が完了しました${RESET}"
}

# CHANGELOGの更新
update_changelog() {
  local changelog_file="$ROOT_DIR/CHANGELOG.md"
  
  echo -e "${BLUE}CHANGELOGを更新中...${RESET}"
  
  if [[ "$DRY_RUN" == true ]]; then
    echo -e "${YELLOW}ドライラン: $changelog_file にバージョン $VERSION の変更履歴を追加します${RESET}"
    return 0
  fi
  
  # 現在の日付を取得
  local current_date=$(date +"%Y-%m-%d")
  
  # CHANGELOGが存在しない場合は作成
  if [[ ! -f "$changelog_file" ]]; then
    cat > "$changelog_file" << EOF
# 変更履歴 (Changelog)

SwiftLight言語の全ての重要な変更はこのファイルに記録されます。

バージョニング方式は[Semantic Versioning](https://semver.org/lang/ja/)に準拠します。

## [$VERSION] - $current_date

### 追加
- 初回リリース

### 変更
- なし

### 修正
- なし

### 削除
- なし
EOF
  else
    # 既存のCHANGELOGに新しいバージョンを挿入
    local tmp_file=$(mktemp)
    
    # ヘッダー部分を抽出
    sed -n '1,/^## \[/p' "$changelog_file" | head -n -1 > "$tmp_file"
    
    # 新しいバージョンを追加
    cat >> "$tmp_file" << EOF
## [$VERSION] - $current_date

### 追加
- バージョン $VERSION の最初のリリース

### 変更
- なし

### 修正
- なし

### 削除
- なし

EOF
    
    # 残りの部分を追加
    sed -n '/^## \[/,$p' "$changelog_file" >> "$tmp_file"
    
    # 一時ファイルを元のファイルに移動
    mv "$tmp_file" "$changelog_file"
  fi
  
  echo -e "${GREEN}CHANGELOGの更新が完了しました${RESET}"
  
  # エディタでCHANGELOGを開く
  if [[ "$DRY_RUN" != true ]]; then
    echo -e "${YELLOW}CHANGELOGを編集してください。コミット前に内容を確認・更新してください。${RESET}"
    
    if [[ -n "$EDITOR" ]]; then
      "$EDITOR" "$changelog_file"
    elif command -v nano > /dev/null 2>&1; then
      nano "$changelog_file"
    elif command -v vim > /dev/null 2>&1; then
      vim "$changelog_file"
    else
      echo -e "${YELLOW}警告: テキストエディタが見つからないため、CHANGELOGを手動で編集してください: $changelog_file${RESET}"
    fi
  fi
}

# ビルドとテスト
build_and_test() {
  echo -e "${BLUE}リリースビルドを実行中...${RESET}"
  
  if [[ "$DRY_RUN" == true ]]; then
    echo -e "${YELLOW}ドライラン: リリースビルドとテストを実行します${RESET}"
    return 0
  fi
  
  # リリースビルドの実行
  BUILD_CMD="$SCRIPT_DIR/build.sh --release"
  
  if [[ "$VERBOSE" == true ]]; then
    BUILD_CMD="$BUILD_CMD --verbose"
  fi
  
  if [[ "$SKIP_TESTS" != true ]]; then
    BUILD_CMD="$BUILD_CMD --tests"
  fi
  
  if [[ "$SKIP_DOCS" != true ]]; then
    BUILD_CMD="$BUILD_CMD --docs"
  fi
  
  echo -e "${YELLOW}実行コマンド: $BUILD_CMD${RESET}"
  eval "$BUILD_CMD"
  
  echo -e "${GREEN}リリースビルドが完了しました${RESET}"
}

# クロスプラットフォームビルド
cross_platform_build() {
  echo -e "${BLUE}クロスプラットフォームビルドを実行中...${RESET}"
  
  if [[ "$DRY_RUN" == true ]]; then
    echo -e "${YELLOW}ドライラン: 以下のプラットフォーム向けにビルドします:${RESET}"
    for platform in "${SELECTED_PLATFORMS[@]}"; do
      echo "- $platform"
    done
    return 0
  fi
  
  # クロスコンパイル用のツールをインストール
  if ! command -v cross > /dev/null 2>&1; then
    echo -e "${YELLOW}cross ツールがインストールされていません。インストール中...${RESET}"
    cargo install cross
  fi
  
  # ビルドディレクトリを準備
  mkdir -p "$PACKAGE_DIR"
  
  # 各プラットフォーム向けにビルド
  for platform in "${SELECTED_PLATFORMS[@]}"; do
    echo -e "${YELLOW}プラットフォーム $platform 向けにビルド中...${RESET}"
    
    if [[ "$platform" == *"windows"* ]]; then
      extension=".exe"
    else
      extension=""
    fi
    
    platform_dir="$PACKAGE_DIR/$platform"
    mkdir -p "$platform_dir"
    
    # クロスコンパイルの実行
    cross build --target "$platform" --release
    
    # バイナリのコピー
    cp "$TARGET_DIR/$platform/release/swiftlight-cli$extension" "$platform_dir/"
    
    # 必要なファイルのコピー
    cp "$ROOT_DIR/README.md" "$platform_dir/"
    cp "$ROOT_DIR/LICENSE" "$platform_dir/"
    cp "$ROOT_DIR/CHANGELOG.md" "$platform_dir/"
    
    # アーカイブの作成
    archive_name="swiftlight-$VERSION-$platform"
    
    if [[ "$platform" == *"windows"* ]]; then
      (cd "$PACKAGE_DIR" && zip -r "$archive_name.zip" "$platform")
      echo -e "${GREEN}アーカイブを作成しました: $PACKAGE_DIR/$archive_name.zip${RESET}"
    else
      (cd "$PACKAGE_DIR" && tar -czf "$archive_name.tar.gz" "$platform")
      echo -e "${GREEN}アーカイブを作成しました: $PACKAGE_DIR/$archive_name.tar.gz${RESET}"
    fi
  done
  
  echo -e "${GREEN}クロスプラットフォームビルドが完了しました${RESET}"
}

# Gitタグの作成とプッシュ
create_git_tag() {
  echo -e "${BLUE}Gitタグを作成中...${RESET}"
  
  if [[ "$DRY_RUN" == true ]]; then
    echo -e "${YELLOW}ドライラン: Gitタグ v$VERSION を作成してプッシュします${RESET}"
    return 0
  fi
  
  # バージョン更新とCHANGELOGのコミット
  git -C "$ROOT_DIR" add "$ROOT_DIR/Cargo.toml" "$ROOT_DIR/CHANGELOG.md"
  find "$ROOT_DIR/crates" -name "Cargo.toml" -exec git -C "$ROOT_DIR" add {} \;
  
  git -C "$ROOT_DIR" commit -m "リリース: バージョン $VERSION"
  
  # タグの作成とプッシュ
  git -C "$ROOT_DIR" tag -a "v$VERSION" -m "バージョン $VERSION"
  git -C "$ROOT_DIR" push origin "v$VERSION"
  git -C "$ROOT_DIR" push
  
  echo -e "${GREEN}Gitタグの作成とプッシュが完了しました${RESET}"
}

# リリースのアップロード
upload_release() {
  echo -e "${BLUE}リリースパッケージをアップロード中...${RESET}"
  
  if [[ "$DRY_RUN" == true ]]; then
    echo -e "${YELLOW}ドライラン: リリースパッケージをアップロードします${RESET}"
    return 0
  fi
  
  # GitHubのリリース作成
  if command -v gh > /dev/null 2>&1; then
    echo -e "${YELLOW}GitHub CLIを使用してリリースを作成中...${RESET}"
    
    # リリースノートの生成
    local release_notes=$(mktemp)
    awk "/## \[$VERSION\]/,/## \[/" "$ROOT_DIR/CHANGELOG.md" | grep -v "## \[" | grep -v "^$" > "$release_notes"
    
    # GitHub Releaseの作成
    gh release create "v$VERSION" \
      --title "SwiftLight $VERSION" \
      --notes-file "$release_notes" \
      --repo "$(git -C "$ROOT_DIR" config --get remote.origin.url | sed 's/.*github.com[:/]\(.*\).git/\1/')"
    
    # アーカイブのアップロード
    for platform in "${SELECTED_PLATFORMS[@]}"; do
      archive_name="swiftlight-$VERSION-$platform"
      
      if [[ "$platform" == *"windows"* ]]; then
        gh release upload "v$VERSION" "$PACKAGE_DIR/$archive_name.zip" \
          --repo "$(git -C "$ROOT_DIR" config --get remote.origin.url | sed 's/.*github.com[:/]\(.*\).git/\1/')"
      else
        gh release upload "v$VERSION" "$PACKAGE_DIR/$archive_name.tar.gz" \
          --repo "$(git -C "$ROOT_DIR" config --get remote.origin.url | sed 's/.*github.com[:/]\(.*\).git/\1/')"
      fi
    done
    
    # 一時ファイルの削除
    rm -f "$release_notes"
    
    echo -e "${GREEN}GitHubリリースが作成されました${RESET}"
  else
    echo -e "${YELLOW}警告: GitHub CLI (gh) がインストールされていないため、手動でGitHubリリースを作成してください${RESET}"
    echo -e "${YELLOW}アーカイブは $PACKAGE_DIR に保存されています${RESET}"
  fi
  
  echo -e "${GREEN}リリースのアップロードが完了しました${RESET}"
}

# リリースのまとめ
print_release_summary() {
  echo ""
  echo -e "${BLUE}リリース概要:${RESET}"
  echo "------------------------------------------------------"
  echo -e "リリースバージョン: ${YELLOW}$VERSION${RESET}"
  echo -e "タグ: ${YELLOW}v$VERSION${RESET}"
  
  # ビルドされたプラットフォーム
  echo -e "\nビルドされたプラットフォーム:"
  for platform in "${SELECTED_PLATFORMS[@]}"; do
    if [[ "$DRY_RUN" == true ]]; then
      echo -e "- ${YELLOW}$platform${RESET}"
    else
      archive_name="swiftlight-$VERSION-$platform"
      
      if [[ "$platform" == *"windows"* ]]; then
        if [[ -f "$PACKAGE_DIR/$archive_name.zip" ]]; then
          size=$(du -h "$PACKAGE_DIR/$archive_name.zip" | cut -f1)
          echo -e "- ${GREEN}$platform${RESET} (アーカイブサイズ: $size)"
        else
          echo -e "- ${RED}$platform${RESET} (ビルド失敗)"
        fi
      else
        if [[ -f "$PACKAGE_DIR/$archive_name.tar.gz" ]]; then
          size=$(du -h "$PACKAGE_DIR/$archive_name.tar.gz" | cut -f1)
          echo -e "- ${GREEN}$platform${RESET} (アーカイブサイズ: $size)"
        else
          echo -e "- ${RED}$platform${RESET} (ビルド失敗)"
        fi
      fi
    fi
  done
  
  echo "------------------------------------------------------"
  
  if [[ "$DRY_RUN" == true ]]; then
    echo -e "\n${YELLOW}これはドライランでした。実際のリリースを行うには --dry-run オプションを削除してください。${RESET}"
  else
    echo -e "\n${GREEN}SwiftLight バージョン $VERSION のリリースが完了しました！${RESET}"
    
    # GitHubリリースへのリンク
    if command -v gh > /dev/null 2>&1; then
      repo_url=$(git -C "$ROOT_DIR" config --get remote.origin.url | sed 's/\.git$//' | sed 's/.*github.com[:/]\(.*\)/https:\/\/github.com\/\1/')
      echo -e "リリースページ: ${BLUE}$repo_url/releases/tag/v$VERSION${RESET}"
    fi
  fi
}

# メイン実行関数
main() {
  parse_args "$@"
  check_git_repository
  update_version
  update_changelog
  build_and_test
  cross_platform_build
  create_git_tag
  upload_release
  print_release_summary
}

main "$@"

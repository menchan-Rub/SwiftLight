#!/usr/bin/env bash

# SwiftLight コンパイラビルドスクリプト
# 
# このスクリプトは SwiftLight コンパイラとその関連ツールをビルドします。
# オプションを指定して異なるビルド設定を選択できます。

set -e

# ディレクトリの定義
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
TARGET_DIR="$ROOT_DIR/target"

# デフォルト設定
BUILD_TYPE="debug"
BUILD_TESTS=false
BUILD_DOCS=false
BUILD_EXAMPLES=false
CLEAN_BUILD=false
PARALLEL_JOBS=$(nproc)
VERBOSE=false

# 色の定義
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
RESET='\033[0m'

# ヘルプメッセージ
print_help() {
  echo -e "${BLUE}SwiftLight コンパイラビルドスクリプト${RESET}"
  echo ""
  echo "使用方法: $(basename "$0") [オプション]"
  echo ""
  echo "オプション:"
  echo "  -h, --help               このヘルプメッセージを表示"
  echo "  -r, --release            リリースビルドを実行 (デフォルト: デバッグビルド)"
  echo "  -t, --tests              テストをビルド"
  echo "  -d, --docs               ドキュメントを生成"
  echo "  -e, --examples           サンプルをビルド"
  echo "  -c, --clean              クリーンビルドを実行 (既存のビルド成果物を削除)"
  echo "  -j, --jobs <数>          並列ジョブ数を指定 (デフォルト: CPU コア数)"
  echo "  -v, --verbose            詳細な出力を表示"
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
      -r|--release)
        BUILD_TYPE="release"
        shift
        ;;
      -t|--tests)
        BUILD_TESTS=true
        shift
        ;;
      -d|--docs)
        BUILD_DOCS=true
        shift
        ;;
      -e|--examples)
        BUILD_EXAMPLES=true
        shift
        ;;
      -c|--clean)
        CLEAN_BUILD=true
        shift
        ;;
      -j|--jobs)
        PARALLEL_JOBS="$2"
        shift 2
        ;;
      -v|--verbose)
        VERBOSE=true
        shift
        ;;
      *)
        echo -e "${RED}エラー: 不明なオプション: $1${RESET}" >&2
        print_help
        exit 1
        ;;
    esac
  done
}

# 依存関係のチェック
check_dependencies() {
  echo -e "${BLUE}依存関係のチェック中...${RESET}"
  
  # Rust のインストールをチェック
  if ! command -v rustc > /dev/null 2>&1; then
    echo -e "${RED}エラー: Rust がインストールされていません。https://rustup.rs からインストールしてください。${RESET}" >&2
    exit 1
  fi
  
  # LLVM のインストールをチェック
  if ! pkg-config --exists 'llvm'; then
    echo -e "${YELLOW}警告: LLVM が検出されませんでした。バックエンドの一部の機能が無効になる可能性があります。${RESET}" >&2
  fi
  
  echo -e "${GREEN}すべての依存関係が検出されました。${RESET}"
}

# クリーンビルド
clean_build() {
  if [ "$CLEAN_BUILD" = true ]; then
    echo -e "${BLUE}クリーンビルドを実行中...${RESET}"
    cargo clean
    echo -e "${GREEN}クリーン完了！${RESET}"
  fi
}

# コンパイラのビルド
build_compiler() {
  echo -e "${BLUE}SwiftLight コンパイラをビルド中...${RESET}"
  
  BUILD_CMD="cargo build --workspace"
  
  if [ "$BUILD_TYPE" = "release" ]; then
    BUILD_CMD="$BUILD_CMD --release"
  fi
  
  if [ "$VERBOSE" = true ]; then
    BUILD_CMD="$BUILD_CMD --verbose"
  fi
  
  # 並列ジョブの設定
  BUILD_CMD="$BUILD_CMD -j $PARALLEL_JOBS"
  
  echo -e "${YELLOW}実行コマンド: $BUILD_CMD${RESET}"
  eval "$BUILD_CMD"
  
  echo -e "${GREEN}コンパイラのビルドが完了しました！${RESET}"
}

# テストのビルドと実行
build_and_run_tests() {
  if [ "$BUILD_TESTS" = true ]; then
    echo -e "${BLUE}テストをビルドして実行中...${RESET}"
    
    TEST_CMD="cargo test --workspace"
    
    if [ "$BUILD_TYPE" = "release" ]; then
      TEST_CMD="$TEST_CMD --release"
    fi
    
    if [ "$VERBOSE" = true ]; then
      TEST_CMD="$TEST_CMD --verbose"
    fi
    
    echo -e "${YELLOW}実行コマンド: $TEST_CMD${RESET}"
    eval "$TEST_CMD"
    
    echo -e "${GREEN}テストが完了しました！${RESET}"
  fi
}

# ドキュメントの生成
build_docs() {
  if [ "$BUILD_DOCS" = true ]; then
    echo -e "${BLUE}ドキュメントを生成中...${RESET}"
    
    DOCS_CMD="cargo doc --workspace --no-deps"
    
    if [ "$VERBOSE" = true ]; then
      DOCS_CMD="$DOCS_CMD --verbose"
    fi
    
    echo -e "${YELLOW}実行コマンド: $DOCS_CMD${RESET}"
    eval "$DOCS_CMD"
    
    echo -e "${GREEN}ドキュメントの生成が完了しました！${RESET}"
    echo -e "${BLUE}ドキュメントは $TARGET_DIR/doc で利用できます${RESET}"
  fi
}

# サンプルのビルド
build_examples() {
  if [ "$BUILD_EXAMPLES" = true ]; then
    echo -e "${BLUE}サンプルをビルド中...${RESET}"
    
    # サンプルディレクトリが存在するか確認
    if [ ! -d "$ROOT_DIR/examples" ]; then
      echo -e "${YELLOW}警告: examples ディレクトリが見つかりません。サンプルのビルドをスキップします。${RESET}"
      return
    fi
    
    EXAMPLES_CMD="cargo build --examples"
    
    if [ "$BUILD_TYPE" = "release" ]; then
      EXAMPLES_CMD="$EXAMPLES_CMD --release"
    fi
    
    if [ "$VERBOSE" = true ]; then
      EXAMPLES_CMD="$EXAMPLES_CMD --verbose"
    fi
    
    echo -e "${YELLOW}実行コマンド: $EXAMPLES_CMD${RESET}"
    eval "$EXAMPLES_CMD"
    
    echo -e "${GREEN}サンプルのビルドが完了しました！${RESET}"
  fi
}

# ビルド結果の表示
print_build_summary() {
  echo ""
  echo -e "${BLUE}ビルド概要:${RESET}"
  echo -e "ビルドタイプ: ${YELLOW}${BUILD_TYPE}${RESET}"
  
  # バイナリパスを表示
  if [ "$BUILD_TYPE" = "release" ]; then
    BIN_PATH="$TARGET_DIR/release"
  else
    BIN_PATH="$TARGET_DIR/debug"
  fi
  
  echo -e "コンパイラの場所: ${YELLOW}$BIN_PATH/swiftlight-cli${RESET}"
  
  # コンパイラのバージョンを表示
  if [ -f "$BIN_PATH/swiftlight-cli" ]; then
    VERSION=$("$BIN_PATH/swiftlight-cli" --version 2>/dev/null || echo "バージョン情報を取得できません")
    echo -e "コンパイラのバージョン: ${YELLOW}$VERSION${RESET}"
  fi
  
  echo ""
  echo -e "${GREEN}ビルドプロセスが正常に完了しました！${RESET}"
}

# メイン実行関数
main() {
  parse_args "$@"
  check_dependencies
  clean_build
  build_compiler
  build_and_run_tests
  build_docs
  build_examples
  print_build_summary
}

main "$@"

#!/usr/bin/env bash

# SwiftLight ブートストラップスクリプト
# 
# このスクリプトは SwiftLight の開発環境をセットアップします。
# 必要な依存関係をインストールし、開発環境を初期化します。

set -e

# ディレクトリの定義
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

# 色の定義
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
RESET='\033[0m'

# ヘルプメッセージ
print_help() {
  echo -e "${BLUE}SwiftLight ブートストラップスクリプト${RESET}"
  echo ""
  echo "使用方法: $(basename "$0") [オプション]"
  echo ""
  echo "オプション:"
  echo "  -h, --help               このヘルプメッセージを表示"
  echo "  -c, --check-only         依存関係の確認のみを行い、インストールは行わない"
  echo "  -d, --dev-tools          開発ツールもインストール"
  echo "  -l, --no-llvm            LLVMのインストールをスキップ"
  echo "  -v, --verbose            詳細な出力を表示"
  echo ""
}

# デフォルト設定
CHECK_ONLY=false
INSTALL_DEV_TOOLS=false
SKIP_LLVM=false
VERBOSE=false

# コマンドライン引数のパース
parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -h|--help)
        print_help
        exit 0
        ;;
      -c|--check-only)
        CHECK_ONLY=true
        shift
        ;;
      -d|--dev-tools)
        INSTALL_DEV_TOOLS=true
        shift
        ;;
      -l|--no-llvm)
        SKIP_LLVM=true
        shift
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

# 出力関数
log() {
  if [ "$VERBOSE" = true ] || [ "$1" != "DEBUG" ]; then
    case "$1" in
      "DEBUG")
        echo -e "${BLUE}[DEBUG]${RESET} $2"
        ;;
      "INFO")
        echo -e "${GREEN}[INFO]${RESET} $2"
        ;;
      "WARN")
        echo -e "${YELLOW}[WARN]${RESET} $2"
        ;;
      "ERROR")
        echo -e "${RED}[ERROR]${RESET} $2" >&2
        ;;
    esac
  fi
}

# OSの検出
detect_os() {
  log "INFO" "システム情報を検出中..."
  
  if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$NAME
    OS_VERSION=$VERSION_ID
  elif [ "$(uname)" = "Darwin" ]; then
    OS="macOS"
    OS_VERSION=$(sw_vers -productVersion)
  elif [ "$(uname -s)" = "Linux" ]; then
    OS="Linux"
    if [ -f /etc/lsb-release ]; then
      . /etc/lsb-release
      OS_VERSION=$DISTRIB_RELEASE
    else
      OS_VERSION="Unknown"
    fi
  else
    OS="Unknown"
    OS_VERSION="Unknown"
  fi
  
  log "INFO" "検出されたOS: $OS $OS_VERSION"
}

# Rustのチェックとインストール
check_and_install_rust() {
  log "INFO" "Rustのインストールを確認中..."
  
  if command -v rustc > /dev/null 2>&1; then
    RUST_VERSION=$(rustc --version | cut -d ' ' -f 2)
    log "INFO" "Rust $RUST_VERSION が既にインストールされています"
  else
    if [ "$CHECK_ONLY" = true ]; then
      log "ERROR" "Rust がインストールされていません"
      return 1
    fi
    
    log "INFO" "Rust をインストール中..."
    if [ "$VERBOSE" = true ]; then
      curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    else
      curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    fi
    
    # PATHを更新
    if [ -f "$HOME/.cargo/env" ]; then
      source "$HOME/.cargo/env"
    fi
    
    RUST_VERSION=$(rustc --version | cut -d ' ' -f 2)
    log "INFO" "Rust $RUST_VERSION がインストールされました"
  fi
  
  # Rustのコンポーネントをチェック
  log "INFO" "必要なRustコンポーネントを確認中..."
  
  # rustupがインストールされているか確認
  if ! command -v rustup > /dev/null 2>&1; then
    log "ERROR" "rustup が見つかりません。Rustのインストールが不完全です。"
    return 1
  fi
  
  # rustfmtコンポーネントの確認とインストール
  if ! rustup component list --installed | grep -q rustfmt; then
    if [ "$CHECK_ONLY" = true ]; then
      log "WARN" "rustfmt コンポーネントがインストールされていません"
    else
      log "INFO" "rustfmt コンポーネントをインストール中..."
      rustup component add rustfmt
      log "INFO" "rustfmt がインストールされました"
    fi
  else
    log "INFO" "rustfmt が既にインストールされています"
  fi
  
  # clippy コンポーネントの確認とインストール
  if ! rustup component list --installed | grep -q clippy; then
    if [ "$CHECK_ONLY" = true ]; then
      log "WARN" "clippy コンポーネントがインストールされていません"
    else
      log "INFO" "clippy コンポーネントをインストール中..."
      rustup component add clippy
      log "INFO" "clippy がインストールされました"
    fi
  else
    log "INFO" "clippy が既にインストールされています"
  fi
  
  # rust-src コンポーネントの確認とインストール
  if ! rustup component list --installed | grep -q rust-src; then
    if [ "$CHECK_ONLY" = true ]; then
      log "WARN" "rust-src コンポーネントがインストールされていません"
    else
      log "INFO" "rust-src コンポーネントをインストール中..."
      rustup component add rust-src
      log "INFO" "rust-src がインストールされました"
    fi
  else
    log "INFO" "rust-src が既にインストールされています"
  fi
  
  # rust-analyzer コンポーネントの確認とインストール
  if ! rustup component list --installed | grep -q rust-analyzer; then
    if [ "$CHECK_ONLY" = true ]; then
      log "WARN" "rust-analyzer コンポーネントがインストールされていません"
    else
      log "INFO" "rust-analyzer コンポーネントをインストール中..."
      rustup component add rust-analyzer
      log "INFO" "rust-analyzer がインストールされました"
    fi
  else
    log "INFO" "rust-analyzer が既にインストールされています"
  fi
  
  return 0
}

# LLVMのチェックとインストール
check_and_install_llvm() {
  if [ "$SKIP_LLVM" = true ]; then
    log "INFO" "LLVMのインストールをスキップします"
    return 0
  fi
  
  log "INFO" "LLVMのインストールを確認中..."
  
  LLVM_REQUIRED_VERSION="14.0"
  
  # LLVMがインストールされているか確認
  if command -v llvm-config > /dev/null 2>&1; then
    LLVM_VERSION=$(llvm-config --version | cut -d '.' -f 1,2)
    log "INFO" "LLVM $LLVM_VERSION が既にインストールされています"
    
    # バージョンが要件を満たしているか確認
    if [ "$(printf '%s\n' "$LLVM_REQUIRED_VERSION" "$LLVM_VERSION" | sort -V | head -n1)" != "$LLVM_REQUIRED_VERSION" ]; then
      log "WARN" "インストールされているLLVMのバージョン($LLVM_VERSION)が要求バージョン($LLVM_REQUIRED_VERSION)より古いです"
      if [ "$CHECK_ONLY" = true ]; then
        return 1
      fi
    else
      return 0
    fi
  else
    if [ "$CHECK_ONLY" = true ]; then
      log "ERROR" "LLVM がインストールされていません"
      return 1
    fi
  fi
  
  log "INFO" "LLVM をインストール中..."
  
  # OSに応じたLLVMのインストール
  case "$OS" in
    "Ubuntu" | "Debian GNU/Linux")
      log "INFO" "apt を使用してLLVMをインストール中..."
      if ! command -v add-apt-repository > /dev/null 2>&1; then
        sudo apt-get update
        sudo apt-get install -y software-properties-common
      fi
      
      sudo add-apt-repository -y 'deb http://apt.llvm.org/focal/ llvm-toolchain-focal-14 main'
      wget -O - https://apt.llvm.org/llvm-snapshot.gpg.key | sudo apt-key add -
      sudo apt-get update
      sudo apt-get install -y llvm-14 llvm-14-dev clang-14 libclang-14-dev
      
      # シンボリックリンクの作成
      sudo update-alternatives --install /usr/bin/llvm-config llvm-config /usr/bin/llvm-config-14 100
      ;;
      
    "Fedora Linux" | "CentOS Linux")
      log "INFO" "dnf を使用してLLVMをインストール中..."
      sudo dnf install -y llvm-devel clang-devel
      ;;
      
    "Arch Linux")
      log "INFO" "pacman を使用してLLVMをインストール中..."
      sudo pacman -S llvm clang
      ;;
      
    "macOS")
      log "INFO" "brew を使用してLLVMをインストール中..."
      if ! command -v brew > /dev/null 2>&1; then
        log "ERROR" "Homebrew がインストールされていません。先に Homebrew をインストールしてください。"
        return 1
      fi
      
      brew install llvm@14
      ;;
      
    *)
      log "ERROR" "サポートされていないOSです: $OS"
      log "WARN" "手動でLLVMをインストールする必要があります: https://llvm.org/docs/GettingStarted.html"
      return 1
      ;;
  esac
  
  # インストールの確認
  if command -v llvm-config > /dev/null 2>&1; then
    LLVM_VERSION=$(llvm-config --version | cut -d '.' -f 1,2)
    log "INFO" "LLVM $LLVM_VERSION がインストールされました"
    return 0
  else
    log "ERROR" "LLVMのインストールに失敗しました"
    return 1
  fi
}

# 開発ツールのチェックとインストール
check_and_install_dev_tools() {
  if [ "$INSTALL_DEV_TOOLS" = false ]; then
    return 0
  fi
  
  log "INFO" "開発ツールのインストールを確認中..."
  
  # cargo-watch のチェックとインストール
  if ! command -v cargo-watch > /dev/null 2>&1; then
    if [ "$CHECK_ONLY" = true ]; then
      log "WARN" "cargo-watch がインストールされていません"
    else
      log "INFO" "cargo-watch をインストール中..."
      cargo install cargo-watch
      log "INFO" "cargo-watch がインストールされました"
    fi
  else
    log "INFO" "cargo-watch が既にインストールされています"
  fi
  
  # cargo-expand のチェックとインストール
  if ! command -v cargo-expand > /dev/null 2>&1; then
    if [ "$CHECK_ONLY" = true ]; then
      log "WARN" "cargo-expand がインストールされていません"
    else
      log "INFO" "cargo-expand をインストール中..."
      cargo install cargo-expand
      log "INFO" "cargo-expand がインストールされました"
    fi
  else
    log "INFO" "cargo-expand が既にインストールされています"
  fi
  
  # cargo-llvm-lines のチェックとインストール
  if ! command -v cargo-llvm-lines > /dev/null 2>&1; then
    if [ "$CHECK_ONLY" = true ]; then
      log "WARN" "cargo-llvm-lines がインストールされていません"
    else
      log "INFO" "cargo-llvm-lines をインストール中..."
      cargo install cargo-llvm-lines
      log "INFO" "cargo-llvm-lines がインストールされました"
    fi
  else
    log "INFO" "cargo-llvm-lines が既にインストールされています"
  fi
  
  # cargo-audit のチェックとインストール
  if ! command -v cargo-audit > /dev/null 2>&1; then
    if [ "$CHECK_ONLY" = true ]; then
      log "WARN" "cargo-audit がインストールされていません"
    else
      log "INFO" "cargo-audit をインストール中..."
      cargo install cargo-audit
      log "INFO" "cargo-audit がインストールされました"
    fi
  else
    log "INFO" "cargo-audit が既にインストールされています"
  fi
  
  return 0
}

# プロジェクトの検証
validate_project() {
  log "INFO" "プロジェクト構成を検証中..."
  
  # Cargo.tomlの存在を確認
  if [ ! -f "$ROOT_DIR/Cargo.toml" ]; then
    log "ERROR" "プロジェクトのルートにCargo.tomlが見つかりません"
    return 1
  fi
  
  # cratesディレクトリの存在を確認
  if [ ! -d "$ROOT_DIR/crates" ]; then
    log "ERROR" "cratesディレクトリが見つかりません"
    return 1
  fi
  
  # 必要なクレートの存在を確認
  REQUIRED_CRATES=(
    "swiftlight-compiler"
    "swiftlight-stdlib"
    "swiftlight-cli"
  )
  
  for crate in "${REQUIRED_CRATES[@]}"; do
    if [ ! -d "$ROOT_DIR/crates/$crate" ]; then
      log "ERROR" "必要なクレートが見つかりません: $crate"
      return 1
    fi
  done
  
  log "INFO" "プロジェクト構成の検証が完了しました"
  return 0
}

# スクリプトの実行権限を設定
setup_script_permissions() {
  log "INFO" "スクリプトの実行権限を設定中..."
  
  # scriptsディレクトリ内のすべてのシェルスクリプトに実行権限を付与
  find "$SCRIPT_DIR" -name "*.sh" -type f -exec chmod +x {} \;
  
  log "INFO" "スクリプトの実行権限が設定されました"
}

# 設定のセットアップ
setup_configuration() {
  log "INFO" "設定ファイルをセットアップ中..."
  
  # .vscodeディレクトリがない場合は作成
  if [ ! -d "$ROOT_DIR/.vscode" ]; then
    mkdir -p "$ROOT_DIR/.vscode"
    log "INFO" ".vscodeディレクトリを作成しました"
  fi
  
  # settings.jsonがない場合は作成
  if [ ! -f "$ROOT_DIR/.vscode/settings.json" ]; then
    cat > "$ROOT_DIR/.vscode/settings.json" << EOF
{
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.checkOnSave.command": "clippy",
  "editor.formatOnSave": true,
  "editor.rulers": [100],
  "files.insertFinalNewline": true,
  "files.trimTrailingWhitespace": true
}
EOF
    log "INFO" "VSCode設定ファイルを作成しました"
  fi
  
  # launch.jsonがない場合は作成
  if [ ! -f "$ROOT_DIR/.vscode/launch.json" ]; then
    cat > "$ROOT_DIR/.vscode/launch.json" << EOF
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug CLI",
      "cargo": {
        "args": [
          "build",
          "--bin=swiftlight-cli",
          "--package=swiftlight-cli"
        ]
      },
      "args": [],
      "cwd": "\${workspaceFolder}"
    }
  ]
}
EOF
    log "INFO" "VSCodeデバッグ設定ファイルを作成しました"
  fi
  
  log "INFO" "設定ファイルのセットアップが完了しました"
}

# サマリーの表示
print_summary() {
  echo ""
  echo -e "${BLUE}セットアップの概要:${RESET}"
  echo "------------------------------------------------------"
  
  # Rust
  if command -v rustc > /dev/null 2>&1; then
    RUST_VERSION=$(rustc --version)
    echo -e "${GREEN}✓${RESET} Rust: $RUST_VERSION"
  else
    echo -e "${RED}✗${RESET} Rust: インストールされていません"
  fi
  
  # LLVM
  if [ "$SKIP_LLVM" = true ]; then
    echo -e "${YELLOW}⚠${RESET} LLVM: スキップされました"
  elif command -v llvm-config > /dev/null 2>&1; then
    LLVM_VERSION=$(llvm-config --version)
    echo -e "${GREEN}✓${RESET} LLVM: $LLVM_VERSION"
  else
    echo -e "${RED}✗${RESET} LLVM: インストールされていません"
  fi
  
  # 開発ツール
  if [ "$INSTALL_DEV_TOOLS" = true ]; then
    echo -e "\n開発ツール:"
    
    # cargo-watch
    if command -v cargo-watch > /dev/null 2>&1; then
      echo -e "${GREEN}✓${RESET} cargo-watch"
    else
      echo -e "${RED}✗${RESET} cargo-watch"
    fi
    
    # cargo-expand
    if command -v cargo-expand > /dev/null 2>&1; then
      echo -e "${GREEN}✓${RESET} cargo-expand"
    else
      echo -e "${RED}✗${RESET} cargo-expand"
    fi
    
    # cargo-llvm-lines
    if command -v cargo-llvm-lines > /dev/null 2>&1; then
      echo -e "${GREEN}✓${RESET} cargo-llvm-lines"
    else
      echo -e "${RED}✗${RESET} cargo-llvm-lines"
    fi
    
    # cargo-audit
    if command -v cargo-audit > /dev/null 2>&1; then
      echo -e "${GREEN}✓${RESET} cargo-audit"
    else
      echo -e "${RED}✗${RESET} cargo-audit"
    fi
  fi
  
  echo "------------------------------------------------------"
  
  # 次のステップを表示
  echo -e "\n${BLUE}次のステップ:${RESET}"
  echo "1. プロジェクトをビルドするには: ${YELLOW}./scripts/build.sh${RESET}"
  echo "2. テストを実行するには: ${YELLOW}./scripts/build.sh --tests${RESET}"
  echo "3. ドキュメントを生成するには: ${YELLOW}./scripts/build.sh --docs${RESET}"
  
  echo ""
  echo -e "${GREEN}SwiftLight開発環境のセットアップが完了しました！${RESET}"
}

# メイン実行関数
main() {
  parse_args "$@"
  detect_os
  
  # 依存関係のチェックとインストール
  check_and_install_rust || exit 1
  check_and_install_llvm || exit 1
  check_and_install_dev_tools || exit 1
  
  # プロジェクトの設定
  validate_project || exit 1
  setup_script_permissions
  setup_configuration
  
  # 概要の表示
  print_summary
}

main "$@"

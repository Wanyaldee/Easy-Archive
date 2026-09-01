#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

confirm() {
    local prompt="$1"
    read -r -p "$prompt [y/N] " reply
    case "$reply" in
        [yY]|[yY][eE][sS]) return 0 ;;
        *) return 1 ;;
    esac
}

if ! command -v cargo >/dev/null 2>&1; then
    echo "Rustツールチェーン(cargo)が見つかりません。"
    if confirm "rustupで自動インストールしますか?"; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        # shellcheck source=/dev/null
        source "$HOME/.cargo/env"
    else
        echo "rustupを手動でインストールしてから再実行してください: https://rustup.rs/"
        exit 1
    fi
fi

if ! cargo deb --version >/dev/null 2>&1; then
    echo "cargo-debが見つかりません。インストールします。"
    cargo install cargo-deb
fi

REQUIRED_APT_PACKAGES="libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev"
MISSING_APT_PACKAGES=""
for pkg in $REQUIRED_APT_PACKAGES; do
    if ! dpkg -s "$pkg" >/dev/null 2>&1; then
        MISSING_APT_PACKAGES="$MISSING_APT_PACKAGES $pkg"
    fi
done

if [ -n "$MISSING_APT_PACKAGES" ]; then
    echo "GUIビルドに必要なパッケージが不足しています:$MISSING_APT_PACKAGES"
    if confirm "sudo apt install で自動インストールしますか?"; then
        sudo apt-get update
        sudo apt-get install -y $MISSING_APT_PACKAGES
    else
        echo "手動でインストールしてから再実行してください。"
        exit 1
    fi
fi

echo "ビルドしています..."
cargo build --release --workspace
(cd crates/gui && cargo deb --no-build)

echo "完了しました。生成された.debファイル:"
find target/debian -maxdepth 1 -name "*.deb"

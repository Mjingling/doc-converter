#!/usr/bin/env bash
# 下载当前平台的 Pdfium 动态库到 src-tauri/resources/pdfium/
# 打包时由 tauri.conf.json 的 bundle.resources 携带，运行时从资源目录加载。
# 来源：https://github.com/bblanchon/pdfium-binaries （Chromium Pdfium 预编译产物）
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="$ROOT/src-tauri/resources/pdfium"
mkdir -p "$DEST"

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Darwin)
    case "$ARCH" in
      arm64) PKG="pdfium-mac-arm64" ;;
      x86_64) PKG="pdfium-mac-x64" ;;
      *) echo "不支持的架构: $ARCH" >&2; exit 1 ;;
    esac
    LIB_NAME="libpdfium.dylib"
    ;;
  Linux)
    PKG="pdfium-linux-x64"
    LIB_NAME="libpdfium.so"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    PKG="pdfium-win-x64"
    LIB_NAME="pdfium.dll"
    ;;
  *)
    echo "不支持的操作系统: $OS" >&2; exit 1
    ;;
esac

URL="https://github.com/bblanchon/pdfium-binaries/releases/latest/download/${PKG}.tgz"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "下载 $URL ..."
curl -fL --progress-bar -o "$TMP/pdfium.tgz" "$URL"
tar -xzf "$TMP/pdfium.tgz" -C "$TMP"

# tgz 内动态库位于 lib/（macOS/Linux）或 bin/（Windows）
FOUND="$(find "$TMP" -name "$LIB_NAME" -type f | head -n 1 || true)"
if [ -z "$FOUND" ]; then
  echo "解压后未找到 $LIB_NAME" >&2
  exit 1
fi

cp "$FOUND" "$DEST/$LIB_NAME"
echo "已就绪: $DEST/$LIB_NAME"

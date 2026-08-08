#!/bin/bash
# 安装 / 卸载 Finder 右键服务「用 DocMorph 打开」
# 用法：
#   bash scripts/install-finder-service.sh            安装
#   bash scripts/install-finder-service.sh --uninstall 卸载
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKFLOW_SRC="$SCRIPT_DIR/../finder/用 DocMorph 打开.workflow"
SERVICE_NAME="用 DocMorph 打开.workflow"
DEST="$HOME/Library/Services/$SERVICE_NAME"

if [[ "${1:-}" == "--uninstall" ]]; then
  rm -rf "$DEST"
  echo "已卸载「用 DocMorph 打开」服务"
  exit 0
fi

if [[ ! -d "$WORKFLOW_SRC" ]]; then
  echo "错误：找不到 workflow 目录：$WORKFLOW_SRC" >&2
  exit 1
fi

mkdir -p "$HOME/Library/Services"
rm -rf "$DEST"
cp -R "$WORKFLOW_SRC" "$DEST"
echo "已安装到：$DEST"
echo ""
echo "接下来请完成启用步骤："
echo "1. 打开「系统设置 → 隐私与安全性 → 扩展」，在「访达」扩展中勾选「用 DocMorph 打开」"
echo "   （或：系统设置 → 键盘 → 键盘快捷键 → 服务 → 文件和文件夹，勾选「用 DocMorph 打开」）"
echo "2. 重启 Finder（Option+右键 Dock 上的访达 → 重新开启）或注销重新登录"
echo "3. 在 Finder 中右键任意 PDF / 文档 / 文件夹 → 服务 →「用 DocMorph 打开」"
echo ""
echo "提示：请确保 DocMorph 已安装到「应用程序」目录（open -a DocMorph 依赖应用名）"

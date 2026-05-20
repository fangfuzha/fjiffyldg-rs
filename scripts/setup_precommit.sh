#!/usr/bin/env bash
set -euo pipefail

# 安装 pre-commit 并确保 Node 依赖（Prettier）已安装（Linux / macOS）
if ! command -v node >/dev/null 2>&1; then
	echo "Node.js 未检测到，请先安装 Node.js（>=18）。"
fi

if [ ! -f package.json ]; then
	npm init -y >/dev/null
fi

npm ci || npm install

python3 -m pip install --user --upgrade pip
python3 -m pip install --user pre-commit

# 在仓库中安装 git 钩子
pre-commit install

echo "pre-commit 已安装。运行 'pre-commit run --all-files' 以格式化并检查所有文件（Prettier 将被用于 Markdown）。"

#!/usr/bin/env bash
set -euo pipefail

# 安装 pre-commit 及需要的格式化工具（Linux / macOS）
python3 -m pip install --user --upgrade pip
python3 -m pip install --user pre-commit mdformat

# 在仓库中安装 git 钩子
pre-commit install

echo "pre-commit 已安装。运行 'pre-commit run --all-files' 以格式化并检查所有文件。"

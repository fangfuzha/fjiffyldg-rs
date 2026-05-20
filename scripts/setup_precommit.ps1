# 安装 pre-commit 并确保 Node 依赖（Prettier）已安装（Windows PowerShell）
if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
	Write-Output "Node.js 未检测到，请先安装 Node.js（>=18）。"
}

if (-not (Test-Path package.json)) {
	npm init -y | Out-Null
}

# 使用 npm 安装 dev 依赖
npm install

# 安装 pre-commit（Python/pip）
python -m pip install --user --upgrade pip
python -m pip install --user pre-commit

# 在仓库中安装 git 钩子
pre-commit install

Write-Output "pre-commit 已安装。运行 'pre-commit run --all-files' 以格式化并检查所有文件（Prettier 将被用于 Markdown）。"

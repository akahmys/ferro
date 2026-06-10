#!/bin/bash
set -e

# プロジェクトルートの確認
if [ ! -d ".git" ]; then
    echo "Error: .git directory not found. Please run this script from the project root."
    exit 1
fi

echo "Setting up Git pre-commit hook..."

# scripts の実行権限付与
chmod +x scripts/prevent_leak.py
chmod +x scripts/verify_ferro_rules.py

# pre-commit フックの作成
cat << 'EOF' > .git/hooks/pre-commit
#!/bin/bash
set -e

# 流出防止スクリプトを実行
python3 scripts/prevent_leak.py

# FERRO規約検証スクリプトを実行
python3 scripts/verify_ferro_rules.py
EOF

# pre-commit フックの実行権限付与
chmod +x .git/hooks/pre-commit

echo "Git pre-commit hook with FERRO rules harness has been set up successfully!"

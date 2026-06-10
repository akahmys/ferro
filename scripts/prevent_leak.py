#!/usr/bin/env python3
import subprocess
import sys
import re

# 検出ルールの定義
# 各ルールは (ルール名, コンパイル済み正規表現, 説明) のタプル
RULES = [
    (
        "OMlx API Key",
        re.compile(r"\ba5mh\b"),
        "oMLX API key detected"
    ),
    (
        "Absolute Path (macOS/Linux user home)",
        re.compile(r"/Users/[a-zA-Z0-9_-]+|/home/[a-zA-Z0-9_-]+"),
        "Absolute path to user home directory detected"
    ),
    (
        "GitHub Personal Access Token",
        re.compile(r"ghp_[a-zA-Z0-9]{36}"),
        "GitHub token detected"
    ),
    (
        "OpenAI API Key",
        re.compile(r"sk-[a-zA-Z0-9]{48}"),
        "OpenAI API key detected"
    ),
    (
        "Private Key PEM",
        re.compile(r"-----BEGIN [A-Z ]*PRIVATE KEY-----"),
        "Private key PEM block detected"
    ),
    (
        "AWS Access Key ID",
        re.compile(r"AKIA[A-Z0-9]{16}"),
        "AWS Access Key ID detected"
    )
]

# スキャン対象外にするファイル（完全一致またはプレフィックス、末尾一致など）
WHITELIST_FILES = [
    "scripts/prevent_leak.py",
    "setup_hooks.sh",
    ".gitignore",
    "doc/dnb_plan.md",
    "doc/system_specification.md",
    "doc/README.md",
    "README.md",
    "GEMINI.md"
]

def is_whitelisted_match(file_path, line, rule_name):
    # 特定の個別例外処理が必要な場合はここで定義する
    return False

def get_staged_files():
    try:
        output = subprocess.check_output(
            ["git", "diff", "--cached", "--name-only", "--diff-filter=ACM"],
            text=True
        )
        return [f.strip() for f in output.splitlines() if f.strip()]
    except subprocess.CalledProcessError as e:
        print(f"Error running git diff: {e}")
        sys.exit(1)

def get_staged_diff_lines(file_path):
    try:
        # ステージングされたファイルの「追加・変更された行（+）」のみを取得
        output = subprocess.check_output(
            ["git", "diff", "--cached", "-U0", file_path],
            text=True
        )
        added_lines = []
        current_line_in_file = 0
        
        for line in output.splitlines():
            if line.startswith("@@"):
                # @@ -start,count +start,count @@
                match = re.search(r"\+(\d+)(?:,(\d+))?", line)
                if match:
                    current_line_in_file = int(match.group(1))
            elif line.startswith("+") and not line.startswith("+++"):
                added_lines.append((current_line_in_file, line[1:]))
                current_line_in_file += 1
            elif not line.startswith("-") and not line.startswith("@@") and not line.startswith("index"):
                current_line_in_file += 1
                
        return added_lines
    except subprocess.CalledProcessError as e:
        # diffが取得できない場合（バイナリファイル等）は空を返す
        return []

def main():
    staged_files = get_staged_files()
    violations = []

    for file_path in staged_files:
        # ドキュメントファイル（.md）はスキャン対象外とする
        if file_path.endswith(".md"):
            continue

        is_whitelisted = False
        for wl in WHITELIST_FILES:
            if file_path == wl or file_path.endswith("/" + wl) or file_path.startswith(wl):
                is_whitelisted = True
                break
        if is_whitelisted:
            continue

        diff_lines = get_staged_diff_lines(file_path)
        for line_num, content in diff_lines:
            for rule_name, pattern, desc in RULES:
                if pattern.search(content):
                    if not is_whitelisted_match(file_path, content, rule_name):
                        violations.append({
                            "file": file_path,
                            "line": line_num,
                            "content": content.strip(),
                            "rule": rule_name,
                            "desc": desc
                        })

    if violations:
        print("\033[1;31m[SECURITY & PRIVACY ALERT] Commit blocked! Potential leak detected:\033[0m")
        for v in violations:
            print(f"  \033[1;33mFile:\033[0m {v['file']}:{v['line']}")
            print(f"  \033[1;33mRule:\033[0m {v['rule']} ({v['desc']})")
            print(f"  \033[1;33mMatch:\033[0m {v['content']}\n")
        print("\033[1;31mPlease remove the secrets or absolute paths before committing.\033[0m")
        print("\033[1;31mIf this is a false positive, you can bypass this check by using 'git commit --no-verify', or add the file to scripts/prevent_leak.py WHITELIST_FILES.\033[0m")
        sys.exit(1)
    
    print("\033[1;32m[Security Check Pass] No secrets or absolute paths detected in staged files.\033[0m")
    sys.exit(0)

if __name__ == "__main__":
    main()

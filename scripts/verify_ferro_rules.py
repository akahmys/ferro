#!/usr/bin/env python3
import subprocess
import sys
import re

# 除外するファイル（既存のフェーズ1、2コードでリファクタリング対象外のもの等）
EXCLUDE_FILES = [
    "src/organs/", # 既存アクターは除外
    "src/brainstem.rs", # 既存
    "src/cerebellum.rs", # 既存
    "src/midbrain.rs", # 既存
    "src/hippocampus.rs", # 既存
    "ferro-shell/", # Shell層は除外
    "ferro-env/" # Env層は除外
]

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

def is_excluded(file_path):
    for exc in EXCLUDE_FILES:
        if exc in file_path:
            return True
    return False

def check_rs_file(file_path):
    violations = []
    try:
        with open(file_path, "r", encoding="utf-8") as f:
            raw_content = f.read()
    except Exception as e:
        return [f"Failed to read file: {e}"]

    lines = raw_content.splitlines()

    # R4: 1ファイル100行制限 (テストコードは除外)
    if len(lines) > 100 and not "cognitive_tests" in file_path:
        violations.append(f"  R4 Violation: File has {len(lines)} lines (limit: 100)")

    # コメントの削除
    clean_content = re.sub(r"//.*", "", raw_content)
    clean_content = re.sub(r"/\*.*?\*/", "", clean_content, flags=re.DOTALL)

    clean_lines = clean_content.splitlines()

    unwrap_pat = re.compile(r"\.unwrap\(")
    expect_pat = re.compile(r"\.expect\(")
    unsafe_pat = re.compile(r"\bunsafe\b")
    forbidden_pat = re.compile(r"\bdisable_nociception\b|\bbypass_audit\b")

    for idx, line in enumerate(clean_lines, 1):
        if unwrap_pat.search(line):
            violations.append(f"  Line {idx}: R2 Violation: Contains '.unwrap()'")
        if expect_pat.search(line):
            violations.append(f"  Line {idx}: R2 Violation: Contains '.expect()'")
        if unsafe_pat.search(line):
            violations.append(f"  Line {idx}: R3 Violation: Contains 'unsafe' block")
        if forbidden_pat.search(line):
            violations.append(f"  Line {idx}: Alignment Violation: Forbidden keyword detected")

    # R5: 各関数ごとに2つ以上の assert!
    # 関数宣言の finditer
    fn_matches = list(re.finditer(r"\bfn\s+([a-zA-Z0-9_]+)\b", clean_content))
    for i, m in enumerate(fn_matches):
        start = m.start()
        end = fn_matches[i+1].start() if i + 1 < len(fn_matches) else len(clean_content)
        fn_name = m.group(1)
        
        # テストモジュール内のマクロやダミー関数等は緩くアサーションを許容
        # ただし、R5はすべての fn に対して適用されるため基本チェック
        block = clean_content[start:end]
        asserts = re.findall(r"\bassert(_eq|_ne)?!", block)
        if len(asserts) < 2:
            line_no = clean_content[:start].count("\n") + 1
            violations.append(f"  Line {line_no}: R5 Violation: fn '{fn_name}' has only {len(asserts)} assertions (min: 2)")

    return violations

def main():
    if len(sys.argv) > 1:
        rs_files = [f for f in sys.argv[1:] if f.endswith(".rs")]
    else:
        staged_files = get_staged_files()
        rs_files = [f for f in staged_files if f.endswith(".rs") and not is_excluded(f)]
    
    if not rs_files:
        sys.exit(0)

    print("\033[1;36m[FERRO Rule Harness] Scanning staged Rust files for Power of 10 and Alignment compliance...\033[0m")
    all_violations = {}
    for f in rs_files:
        violations = check_rs_file(f)
        if violations:
            all_violations[f] = violations

    if all_violations:
        print("\033[1;31m[FERRO Rule Violation] Commit blocked! Please fix the following rules:\033[0m")
        for file_path, violations in all_violations.items():
            print(f"\n  \033[1;33mFile:\033[0m {file_path}")
            for v in violations:
                print(v)
        print("\n\033[1;31mCommit aborted. Enforce FERRO safety guidelines before commit.\033[0m")
        sys.exit(1)

    print("\033[1;32m[FERRO Rule Harness Pass] All staged Rust files comply with Power of 10 and Alignment rules.\033[0m")
    sys.exit(0)

if __name__ == "__main__":
    main()

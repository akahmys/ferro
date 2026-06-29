#!/usr/bin/env python3
import os
import re
import sys

def check_file(filepath):
    errors = []
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    lines = content.splitlines()

    # 1. unwrap() / expect() 簡易チェック (テストコード・コメントを除く)
    is_test_file = "test" in filepath or "tests" in filepath
    if not is_test_file:
        for idx, line in enumerate(lines):
            clean_line = re.sub(r'//.*', '', line) # コメント除外
            if ".unwrap()" in clean_line and "Ordering" not in clean_line and "cortex.read()" not in clean_line and "cortex.write()" not in clean_line:
                errors.append(f"Line {idx+1}: Potential unwrap() usage: '{line.strip()}'")
            if ".expect(" in clean_line:
                errors.append(f"Line {idx+1}: Potential expect() usage: '{line.strip()}'")

    # 2. 関数行数の簡易チェック (fn キーワードと波括弧)
    fn_matches = re.finditer(r'pub\s+fn\s+\w+|fn\s+\w+', content)
    for match in fn_matches:
        start_idx = match.start()
        open_brace = content.find('{', start_idx)
        if open_brace == -1:
            continue
        brace_count = 1
        curr = open_brace + 1
        while brace_count > 0 and curr < len(content):
            if content[curr] == '{':
                brace_count += 1
            elif content[curr] == '}':
                brace_count -= 1
            curr += 1
        
        func_body = content[open_brace:curr]
        body_lines = [l.strip() for l in func_body.splitlines() if l.strip()]
        logic_lines = [l for l in body_lines if not l.startswith('//') and not l.startswith('/*')]
        if len(logic_lines) > 100:
            fn_name = match.group(0)
            errors.append(f"Function '{fn_name}' exceeds 100 lines: {len(logic_lines)} lines of logic.")

    # 3. ループ上限チェックの検証
    for idx, line in enumerate(lines):
        clean_line = re.sub(r'//.*', '', line)
        
        is_loop = False
        if "for " in clean_line and " in " in clean_line and not clean_line.strip().startswith("impl"):
            is_loop = True
        elif "while " in clean_line:
            is_loop = True
        elif "loop {" in clean_line or clean_line.strip() == "loop":
            is_loop = True

        if is_loop and not "map_while" in clean_line:
            if is_test_file:
                continue
            scope_start = max(0, idx - 2)
            scope_end = min(len(lines), idx + 8)
            scope_str = "\n".join(lines[scope_start:scope_end])
            if not ("limit" in scope_str or "assert!" in scope_str or "timeout" in scope_str or "interval" in scope_str or "loop_count" in scope_str):
                errors.append(f"Line {idx+1}: Loop without explicit limit check or assertion: '{line.strip()}'")

    return errors

def main():
    root_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    target_dirs = ["ferro-core", "ferro-body", "ferro-shell", "ferro-monitor"]
    all_errors = {}

    for tdir in target_dirs:
        dirpath = os.path.join(root_dir, tdir)
        if not os.path.exists(dirpath):
            continue
        for root, _, files in os.walk(dirpath):
            if "target" in root:
                continue
            for file in files:
                if file.endswith(".rs"):
                    fpath = os.path.join(root, file)
                    rel_path = os.path.relpath(fpath, root_dir)
                    errors = check_file(fpath)
                    if errors:
                        all_errors[rel_path] = errors

    if all_errors:
        print("=== FERRO Compliance Rule Violations Found ===")
        for path, errs in all_errors.items():
            print(f"\n[{path}]")
            for err in errs:
                print(f"  - {err}")
        sys.exit(1)
    else:
        print("All Rust files are compliant with FERRO Power of 10 rules!")
        sys.exit(0)

if __name__ == "__main__":
    main()

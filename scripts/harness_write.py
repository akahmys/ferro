#!/usr/bin/env python3
import sys
import os
import shutil
import subprocess

def main():
    if len(sys.argv) < 3:
        print("Usage: harness_write.py <target_file> <temp_source_file>", file=sys.stderr)
        sys.exit(1)

    target_file = sys.argv[1]
    temp_source = sys.argv[2]

    if not os.path.exists(temp_source):
        print(f"Error: Temp source file {temp_source} does not exist", file=sys.stderr)
        sys.exit(1)

    # .rsファイルの場合のみ、ルールチェックを実行
    if temp_source.endswith(".rs") or target_file.endswith(".rs"):
        # 一時ファイルの名前が.rsで終わっていないとチェッカーが無視するため、必要なら一時的にコピーして検査
        test_file = temp_source
        needs_cleanup = False
        if not temp_source.endswith(".rs"):
            test_file = temp_source + "_test.rs"
            shutil.copy(temp_source, test_file)
            needs_cleanup = True

        try:
            cmd = ["python3", "scripts/verify_ferro_rules.py", test_file]
            res = subprocess.run(cmd, capture_output=True, text=True)
            if needs_cleanup and os.path.exists(test_file):
                os.remove(test_file)

            if res.returncode != 0:
                print("\033[1;31m[Coding Harness Violation] Modification rejected by harness:\033[0m", file=sys.stderr)
                print(res.stderr + res.stdout, file=sys.stderr)
                sys.exit(1)
        except Exception as e:
            if needs_cleanup and os.path.exists(test_file):
                os.remove(test_file)
            print(f"Error executing rules check: {e}", file=sys.stderr)
            sys.exit(1)

    # 検証を通過したら、アトミックにリネーム上書き
    try:
        parent = os.path.dirname(os.path.abspath(target_file))
        os.makedirs(parent, exist_ok=True)
        shutil.move(temp_source, target_file)
        print(f"\033[1;32m[Coding Harness Pass] Successfully wrote complying code to {target_file}\033[0m")
    except Exception as e:
        print(f"Error writing to target file: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()

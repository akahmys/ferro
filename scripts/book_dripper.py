#!/usr/bin/env python3
import os
import sys
import time
import json
import csv
import argparse

PROJECT_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
MEMORY_DIR = os.path.join(PROJECT_ROOT, "ferro-core", "memory")
STIMULUS_DIR = os.path.join(MEMORY_DIR, "stimulus")
BOOKS_DIR = os.path.join(PROJECT_ROOT, "books")
STATE_PATH = os.path.join(PROJECT_ROOT, "scratch", "curriculum_stage.json")

def read_json_safe(path, default):
    if not os.path.exists(path):
        return default
    try:
        with open(path, "r", encoding="utf-8") as f:
            return json.load(f)
    except Exception:
        return default

def write_atomic(path, data):
    tmp_path = path + f".tmp_{int(time.time() * 1000)}"
    try:
        dir_name = os.path.dirname(path)
        if not os.path.exists(dir_name):
            os.makedirs(dir_name, exist_ok=True)
        with open(tmp_path, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False)
        os.replace(tmp_path, path)
    except Exception as e:
        print(f"[Dripper] Error writing atomic to {path}: {e}")

def get_latest_csv_row(path):
    if not os.path.exists(path):
        return None
    try:
        with open(path, "r", encoding="utf-8") as f:
            reader = csv.reader(f)
            rows = list(reader)
            if len(rows) > 1:
                return rows[-1]
    except Exception:
        pass
    return None

def get_latest_fep_and_phase():
    csv_path = os.path.join(MEMORY_DIR, "surprise_history.csv")
    row = get_latest_csv_row(csv_path)
    if row and len(row) >= 3:
        try:
            fep = float(row[1])
            phase = row[2].strip()
            return fep, phase
        except ValueError:
            pass
    return 0.0, "Wake"

def get_cortex_clusters():
    base_dir = os.path.join(MEMORY_DIR, "knowledge_graph")
    if not os.path.exists(base_dir):
        return 0
    count = 0
    try:
        for root, dirs, files in os.walk(base_dir):
            for file in files:
                if file.endswith(".json") and not file.endswith(".tmp"):
                    count += 1
        return count
    except Exception:
        return count

def load_book_for_stage(stage):
    filename = f"stage_{stage:02d}.json"
    if stage == 1:
        filename = "stage_01_nouns.json"
    elif stage == 2:
        filename = "stage_02_two_words.json"
    elif stage == 3:
        filename = "stage_03_grammar.json"
    elif stage == 4:
        filename = "stage_04_qa.json"
    elif stage == 5:
        filename = "stage_05_dialogue.json"
    elif stage == 6:
        filename = "stage_06_large_corpus.json"
        
    path = os.path.join(BOOKS_DIR, filename)
    if not os.path.exists(path):
        print(f"[Dripper] Warning: Book for stage {stage} ({path}) not found!")
        return None
    return read_json_safe(path, None)

def get_stage_success(stage, fep, clusters):
    # Success conditions for each stage
    if stage == 1:
        return clusters >= 5 and fep <= 0.05
    elif stage == 2:
        return clusters >= 10 and fep <= 0.05
    elif stage == 3:
        return clusters >= 13 and fep <= 0.05
    elif stage == 4:
        return clusters >= 15 and fep <= 0.05
    elif stage == 5:
        return clusters >= 18 and fep <= 0.05
    elif stage == 6:
        return clusters >= 5000 and fep <= 0.05
    return False

def main():
    parser = argparse.ArgumentParser(description="FERRO Cortex Japanese Breeding Book Dripper")
    parser.add_argument("--stage", type=int, default=None, help="Force starting stage (1-6)")
    parser.add_argument("--interval", type=int, default=20, help="Dripping interval in seconds")
    parser.add_argument("--auto-advance", type=bool, default=True, help="Auto advance stage on sleep transition")
    args = parser.parse_args()

    # Load or initialize state
    state = read_json_safe(STATE_PATH, {"stage": 1, "auto_advance": True})
    
    if args.stage is not None:
        state["stage"] = args.stage
    state["auto_advance"] = args.auto_advance
    write_atomic(STATE_PATH, state)

    current_stage = state["stage"]
    print(f"[Dripper] Initializing. Current Stage: {current_stage}")

    book = load_book_for_stage(current_stage)
    if not book:
        print(f"[Dripper] Failed to load book for Stage {current_stage}. Exiting.")
        sys.exit(1)

    # Write lock file to pause ferro-env default updates
    lock_file = os.path.join(MEMORY_DIR, "dripper_active.lock")
    try:
        with open(lock_file, "w") as f:
            f.write(str(os.getpid()))
    except Exception as e:
        print(f"[Dripper] Failed to create lock file: {e}")

    try:
        page_idx = 0
        last_phase = "Wake"
        dripped_since_sleep = 0

        while True:
            # 1. Monitor phase
            fep, phase = get_latest_fep_and_phase()
            clusters = get_cortex_clusters()
            
            # Detect phase transition
            if last_phase == "Wake" and phase == "Sleep":
                print(f"[Dripper] Phase transition detected: Wake -> Sleep.")
                print("[Dripper] Waiting 30s for Sleep consolidation to complete before evaluating success...")
                time.sleep(30)
                # Reload latest stats after sleep consolidation
                fep, _ = get_latest_fep_and_phase()
                clusters = get_cortex_clusters()
                
                if state["auto_advance"]:
                    success = get_stage_success(current_stage, fep, clusters)
                    print(f"[Dripper] Evaluating Stage {current_stage} success: fep={fep:.4f}, clusters={clusters}, success={success}")
                    if success and current_stage < 6:
                        next_stage = current_stage + 1
                        next_book = load_book_for_stage(next_stage)
                        if next_book:
                            print(f"[Dripper] Success! Advancing to Stage {next_stage}")
                            current_stage = next_stage
                            book = next_book
                            state["stage"] = current_stage
                            write_atomic(STATE_PATH, state)
                        else:
                            print(f"[Dripper] Stage {current_stage} success, but book for Stage {next_stage} is missing. Repeating current stage.")
                    elif success:
                        print(f"[Dripper] Completed Stage 6 (final stage) successfully!")
                    else:
                        print(f"[Dripper] Stage {current_stage} success criteria not met yet. Remaining on current stage.")
                
                # Suspend dripping in sleep
                print("[Dripper] System is sleeping. Waiting for Wake phase...")
                while phase == "Sleep":
                    time.sleep(2)
                    fep, phase = get_latest_fep_and_phase()
                print("[Dripper] System woke up! Resuming curriculum.")
                dripped_since_sleep = 0

            last_phase = phase

            # 2. Dripping a page if awake
            if phase == "Wake":
                pages = book.get("pages", [])
                if not pages:
                    print("[Dripper] Error: Current book has no pages.")
                    time.sleep(5)
                    continue

                page = pages[page_idx]
                text = page.get("text", "")
                image_emb = page.get("image_embedding", [0.0] * 5)
                mfcc = page.get("mfcc", [0.0] * 5)
                frame_delta = page.get("frame_delta", 0.0)

                # Space-segmented tokens
                tokens = text.split(" ")

                now_ms = int(time.time() * 1000)

                # Atomic Co-Dripping
                # user_input.json
                write_atomic(
                    os.path.join(MEMORY_DIR, "user_input.json"),
                    {"timestamp": now_ms, "text": text}
                )

                # visual.json
                write_atomic(
                    os.path.join(STIMULUS_DIR, "visual.json"),
                    {
                        "timestamp": now_ms,
                        "frame_delta": frame_delta,
                        "image_embedding": image_emb
                    }
                )

                # auditory.json
                write_atomic(
                    os.path.join(STIMULUS_DIR, "auditory.json"),
                    {
                        "timestamp": now_ms,
                        "speech_tokens": tokens,
                        "mfcc": mfcc
                    }
                )

                print(f"[Dripper] [Stage {current_stage}] Page {page_idx + 1}/{len(pages)} dripped: text='{text}', FEP={fep:.4f}, Clusters={clusters}")

                # Advance page and check loop completion
                page_idx += 1
                dripped_since_sleep += 1
                if page_idx >= len(pages):
                    print(f"[Dripper] Finished dripping all {len(pages)} pages of Stage {current_stage} book.")
                    print("[Dripper] Initiating resting period (45s) to allow FERRO to transition to Sleep phase...")
                    page_idx = 0
                    dripped_since_sleep = 0
                    
                    resting_elapsed = 0
                    check_phase = "Wake"
                    while resting_elapsed < 45:
                        time.sleep(5)
                        resting_elapsed += 5
                        _, check_phase = get_latest_fep_and_phase()
                        if check_phase == "Sleep":
                            print("[Dripper] FERRO has entered Sleep phase during the rest period. Proceeding.")
                            break
                        if resting_elapsed % 15 == 0:
                            print(f"[Dripper] Resting... {resting_elapsed}/45s elapsed. FERRO Phase: {check_phase}")
                    
                    if check_phase != "Sleep":
                        print("[Dripper] Resting period completed. FERRO did not enter Sleep yet. Continuing.")
                elif dripped_since_sleep >= 40:
                    print(f"[Dripper] Reached {dripped_since_sleep} pages without sleep. Initiating periodic resting period (45s) to consolidate memory...")
                    dripped_since_sleep = 0
                    
                    resting_elapsed = 0
                    check_phase = "Wake"
                    while resting_elapsed < 45:
                        time.sleep(5)
                        resting_elapsed += 5
                        _, check_phase = get_latest_fep_and_phase()
                        if check_phase == "Sleep":
                            print("[Dripper] FERRO has entered Sleep phase during periodic rest. Proceeding.")
                            break
                        if resting_elapsed % 15 == 0:
                            print(f"[Dripper] Periodic resting... {resting_elapsed}/45s elapsed. FERRO Phase: {check_phase}")
                else:
                    # Sleep interval, but check phase periodically
                    elapsed = 0
                    while elapsed < args.interval:
                        time.sleep(1)
                        elapsed += 1
                        _, current_phase = get_latest_fep_and_phase()
                        if current_phase == "Sleep":
                            break
    finally:
        if os.path.exists(lock_file):
            try:
                os.remove(lock_file)
                print("[Dripper] Cleaned up dripper_active.lock")
            except Exception as e:
                print(f"[Dripper] Failed to remove lock file: {e}")

if __name__ == "__main__":
    main()

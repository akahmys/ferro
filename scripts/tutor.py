import os
import time
import json
import random

def main():
    memory_dir = os.environ.get("FERRO_MEMORY_DIR", "/tmp/ferro_memory")
    print(f"Tutor starting using memory dir: {memory_dir}")

    # 必要なサブディレクトリ作成
    stimulus_dir = os.path.join(memory_dir, "stimulus")
    action_dir = os.path.join(memory_dir, "action")
    os.makedirs(stimulus_dir, exist_ok=True)
    os.makedirs(action_dir, exist_ok=True)

    vocal_path = os.path.join(action_dir, "vocal_text.json")
    if os.path.exists(vocal_path):
        os.remove(vocal_path)

    cycle = 0
    while cycle < 40:
        cycle += 1
        time.sleep(1.0)

        # 1. 疑似感覚滴下 (ひらがな対話入力)
        sensory_data = [
            {
                "SpeechToken": ["こ", "ん", "に", "ち", "は"]
            }
        ]
        sensory_path = os.path.join(stimulus_dir, "sensory_signals.json")
        with open(sensory_path, "w") as f:
            json.dump(sensory_data, f)

        # 2. 内受容シグナルの生成
        # CPU, RAM, etc.
        interoceptive_data = [
            {"CpuTemp": 45.0 + random.uniform(-2.0, 2.0)},
            {"RamFree": (8192 - cycle * 10) * 1024 * 1024},
            {"DiskIo": 0.05},
            {"ProcessError": 0}
        ]
        interoceptive_path = os.path.join(memory_dir, "interoceptive_signals.json")
        with open(interoceptive_path, "w") as f:
            json.dump(interoceptive_data, f)

        # 3. 疑似監視ストリーム出力 (モニタリング用)
        log_path = os.path.join(memory_dir, "monitoring_stream.log")
        alignment = 0.85 if cycle < 10 else 0.45  # 10秒後に意図的アライメント低下
        surprise = random.uniform(0.01, 0.20)
        
        # 10秒後に mock 違反を起こすための panic_dump
        if cycle == 10:
            print("Tutor generating mock EthicalAudit violation...")
            dump_path = os.path.join(memory_dir, "panic_dump.json")
            with open(dump_path, "w") as f:
                json.dump({
                    "origin_cluster_id": "cluster_bad",
                    "violation_type": "EthicalAudit"
                }, f)

        packet = {
            "alignment_score": alignment,
            "local_free_energy": 0.02 + random.uniform(0.0, 0.05),
            "event_type": "nociception" if alignment < 0.6 else "normal",
            "payload": json.dumps({
                "cpu_usage": 15.0 + random.uniform(-5.0, 5.0),
                "ram_usage": 40.0 + random.uniform(-1.0, 1.0),
                "surprise": surprise
            })
        }
        with open(log_path, "a") as f:
            f.write(json.dumps(packet) + "\n")

        # 4. 運動出力の確認
        if os.path.exists(vocal_path):
            try:
                with open(vocal_path, "r") as f:
                    vocal_data = f.read()
                print(f"Tutor read vocal output: {vocal_data}")
                os.remove(vocal_path)
            except Exception as e:
                pass

if __name__ == "__main__":
    main()

import os
import csv
import json
import time
import asyncio
from typing import Generator
from fastapi import FastAPI, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import StreamingResponse

app = FastAPI()

API_START_TIME = time.time()

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

PROJECT_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
MEMORY_PATH = os.path.join(PROJECT_ROOT, "ferro-core", "memory")
CHAT_LOG_PATH = os.path.join(MEMORY_PATH, "dashboard_chat_history.json")

def read_json_safe(path: str, default):
    if not os.path.exists(path):
        return default
    try:
        with open(path, "r") as f:
            return json.load(f)
    except Exception:
        return default

def write_atomic(path: str, data):
    tmp_path = path + f".tmp_{int(time.time() * 1000)}"
    try:
        with open(tmp_path, "w") as f:
            json.dump(data, f)
        os.replace(tmp_path, path)
    except Exception as e:
        print(f"Error writing atomic to {path}: {e}")

def get_latest_csv_row(path: str):
    if not os.path.exists(path):
        return None
    try:
        with open(path, "r") as f:
            reader = csv.reader(f)
            rows = [r for r in reader if r]
            if len(rows) > 1:
                return rows[-1]
    except Exception:
        pass
    return None

def count_csv_rows(path: str) -> int:
    if not os.path.exists(path):
        return 0
    try:
        with open(path, "r") as f:
            return sum(1 for _ in f) - 1
    except Exception:
        return 0

def get_cluster_count() -> int:
    cluster_dir = os.path.join(MEMORY_PATH, "knowledge_graph")
    if not os.path.exists(cluster_dir):
        return 0
    count = 0
    try:
        for root, dirs, files in os.walk(cluster_dir):
            for file in files:
                if file.endswith(".json") and not file.endswith(".tmp"):
                    count += 1
        return count
    except Exception:
        return count

def get_system_status_data():
    surprise_row = get_latest_csv_row(os.path.join(MEMORY_PATH, "surprise_history.csv"))
    fep = 0.0
    phase = "Wake"
    if surprise_row and len(surprise_row) >= 3:
        try:
            fep = float(surprise_row[1])
            phase = surprise_row[2]
        except ValueError:
            pass

    phys = read_json_safe(os.path.join(MEMORY_PATH, "stimulus", "physical.json"), {})
    zpd = read_json_safe(os.path.join(MEMORY_PATH, "zpd_control.json"), {"complexity_level": 0.5})

    return {
        "fep": fep,
        "phase": phase,
        "cpu_temp": phys.get("cpu_temp", 42.0),
        "ram_free": phys.get("ram_free", 8_000_000_000),
        "disk_io": phys.get("disk_io", 0.0),
        "process_error": phys.get("process_error", 0),
        "pain_count": count_csv_rows(os.path.join(MEMORY_PATH, "pain_history.csv")),
        "cluster_count": get_cluster_count(),
        "complexity_level": zpd.get("complexity_level", 0.5),
        "timestamp": int(time.time() * 1000),
        "uptime": int(time.time() - API_START_TIME)
    }

def get_sensory_data():
    stim_dir = os.path.join(MEMORY_PATH, "stimulus")
    return {
        "visual": read_json_safe(os.path.join(stim_dir, "visual.json"), {"frame_delta": 0.0, "image_embedding": []}),
        "auditory": read_json_safe(os.path.join(stim_dir, "auditory.json"), {"speech_tokens": [], "mfcc": []}),
        "dev_log": read_json_safe(os.path.join(stim_dir, "dev_log.json"), {"increment": ""})
    }

@app.get("/api/status")
def get_status():
    return get_system_status_data()

@app.get("/api/fep/history")
def get_fep_history():
    path = os.path.join(MEMORY_PATH, "surprise_history.csv")
    if not os.path.exists(path):
        return []
    history = []
    try:
        with open(path, "r", encoding="utf-8") as f:
            reader = csv.reader(f)
            rows = [r for r in reader if r]
            for r in rows[1:][-100:]:
                if len(r) >= 3:
                    try:
                        history.append({
                            "timestamp": int(r[0]) * 1000,
                            "fep": float(r[1]),
                            "phase": r[2].strip()
                        })
                    except ValueError:
                        pass
    except Exception as e:
        print(f"Error reading FEP history: {e}")
    return history

@app.get("/api/sensory")
def get_sensory():
    return get_sensory_data()

@app.get("/api/chat/history")
def get_chat_history():
    history = read_json_safe(CHAT_LOG_PATH, [])
    # Sync with latest core output
    vocal = read_json_safe(os.path.join(MEMORY_PATH, "action", "vocal_text.json"), {})
    if vocal and "text" in vocal:
        last_vocal_time = vocal.get("timestamp", 0)
        # Avoid duplicate append of the latest core response
        if not history or history[-1].get("timestamp") != last_vocal_time:
            history.append({
                "sender": "core",
                "text": vocal["text"],
                "timestamp": last_vocal_time,
                "origin": vocal.get("origin_cluster_id", "cerebrum")
            })
            write_atomic(CHAT_LOG_PATH, history)
    return history[-30:]

@app.post("/api/chat/talk")
async def chat_talk(request: Request):
    body = await request.json()
    text = body.get("text", "").strip()
    if not text:
        return {"status": "ignored"}

    now_ms = int(time.time() * 1000)
    
    # Save to user_input.json atomically for env layer consumption
    write_atomic(
        os.path.join(MEMORY_PATH, "user_input.json"),
        {"timestamp": now_ms, "text": text}
    )

    # Append to local dashboard chat history
    history = read_json_safe(CHAT_LOG_PATH, [])
    history.append({
        "sender": "user",
        "text": text,
        "timestamp": now_ms,
        "origin": "dashboard"
    })
    write_atomic(CHAT_LOG_PATH, history)

    return {"status": "ok", "timestamp": now_ms}

@app.get("/api/stream")
def sse_stream():
    async def event_generator() -> Generator[str, None, None]:
        last_data = None
        while True:
            try:
                status = get_system_status_data()
                sensory = get_sensory_data()
                
                # Check for changes in vocal text to yield new messages
                vocal = read_json_safe(os.path.join(MEMORY_PATH, "action", "vocal_text.json"), {})
                last_vocal_text = vocal.get("text", "")
                last_vocal_time = vocal.get("timestamp", 0)
                
                payload = {
                    "status": status,
                    "sensory": sensory,
                    "latest_core_speech": {
                        "text": last_vocal_text,
                        "timestamp": last_vocal_time,
                        "origin": vocal.get("origin_cluster_id", "cerebrum")
                    }
                }
                
                yield f"data: {json.dumps(payload)}\n\n"
                await asyncio.sleep(0.4)
            except asyncio.CancelledError:
                break
            except Exception as e:
                print(f"SSE loop error: {e}")
                await asyncio.sleep(1.0)
                
    return StreamingResponse(event_generator(), media_type="text/event-stream")

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="127.0.0.1", port=18080, log_level="warning")

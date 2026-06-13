#!/usr/bin/env python3
import os
import sys
import time
import json
import re
import urllib.request
import urllib.error

PROJECT_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
MEMORY_DIR = os.path.join(PROJECT_ROOT, "ferro-core", "memory")
ACTION_DIR = os.path.join(MEMORY_DIR, "action")
STIMULUS_DIR = os.path.join(MEMORY_DIR, "stimulus")

# Configurable constants
HUMAN_INTERRUPT_WINDOW_SECS = 120  # Suspend tutor for 2 mins if human types
POLL_INTERVAL_SECS = 1.0

# Local rule-based fallback dictionary
LOCAL_FALLBACK_RULES = {
    "こんにちは": "こんにちは こあ です",
    "おはよ う": "おはよ う げんき です か",
    "おなまえ は": "わたし は ふぇろー こあ です",
    "げんき です か": "はい とても げんき です",
    "ありがとう": "どういたしまして",
    "さようなら": "また ね バイバイ",
    "てんき": "きょう の てんき は はれ です",
    "おやすみ": "おやすみなさい よい ゆめ を",
    "なに": "これ は なに かな",
    "状態": "しすてむ は せいじょう に かどう して います",
    "じょうたい": "しすてむ は せいじょう に かどう して います",
}

def to_hiragana(text):
    """Convert Katakana characters to Hiragana."""
    result = []
    for c in text:
        code = ord(c)
        # Katakana Unicode range (U+30A1 - U+30F6) -> Hiragana (U+3041 - U+3096)
        if 0x30A1 <= code <= 0x30F6:
            result.append(chr(code - 0x60))
        else:
            result.append(c)
    return "".join(result)

def clean_and_segment(text):
    """Clean text to ensure only space-segmented Hiragana characters remain."""
    text = to_hiragana(text)
    # Remove everything except Hiragana (\u3040-\u309F) and whitespace
    text = re.sub(r'[^\u3040-\u309F\s]', '', text)
    # Coalesce multiple spaces into a single space
    text = re.sub(r'\s+', ' ', text).strip()
    return text

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
        print(f"[Tutor] Error writing atomic to {path}: {e}")

def get_gemini_response(api_key, core_text):
    """Call Gemini API via REST raw interface."""
    url = f"https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key={api_key}"
    
    prompt = (
        f"You are a kind parent teaching a small child (named Core/こあ) how to speak Japanese.\n"
        f"The child said: \"{core_text}\"\n"
        f"Please respond to the child. Keep your response very simple, short, and warm.\n"
        f"CRITICAL RULE: You must write your entire response ONLY in Hiragana, with words separated by spaces.\n"
        f"Do NOT use any Kanji, Katakana, English characters, punctuation marks, or special characters.\n"
        f"Example format: \"こんにちは こあ よい てんき です ね\"\n"
        f"Response:"
    )

    data = {
        "contents": [{
            "parts": [{"text": prompt}]
        }]
    }
    
    req_body = json.dumps(data).encode("utf-8")
    req = urllib.request.Request(
        url,
        data=req_body,
        headers={"Content-Type": "application/json"},
        method="POST"
    )
    
    try:
        with urllib.request.urlopen(req, timeout=10) as response:
            res_body = response.read().decode("utf-8")
            res_json = json.loads(res_body)
            # Parse response text
            candidates = res_json.get("candidates", [])
            if candidates:
                content = candidates[0].get("content", {})
                parts = content.get("parts", [])
                if parts:
                    raw_text = parts[0].get("text", "")
                    return raw_text.strip()
    except Exception as e:
        print(f"[Tutor] Gemini API call failed: {e}. Falling back to local rules.")
    return None

def generate_response(core_text, api_key):
    """Generates response using Gemini API or Local Rules."""
    if api_key:
        print(f"[Tutor] Requesting Gemini response for: '{core_text}'...")
        gemini_txt = get_gemini_response(api_key, core_text)
        if gemini_txt:
            cleaned = clean_and_segment(gemini_txt)
            if cleaned:
                print(f"[Tutor] Gemini Generated: '{cleaned}' (Raw: '{gemini_txt.strip()}')")
                return cleaned
            
    # Local fallback
    print(f"[Tutor] Using local fallback rules for: '{core_text}'")
    for key, val in LOCAL_FALLBACK_RULES.items():
        if key in core_text:
            return val
    return "りょうかい しました"

VOCAB_EMBEDDING_MAP = {}

def load_vocab_embeddings():
    """Scan books/ directory and load word-to-vector mappings."""
    global VOCAB_EMBEDDING_MAP
    books_dir = os.path.join(PROJECT_ROOT, "books")
    if not os.path.exists(books_dir):
        print(f"[Tutor] Books directory {books_dir} not found. Using empty map.")
        return
        
    import glob
    json_files = glob.glob(os.path.join(books_dir, "stage_*.json"))
    print(f"[Tutor] Scanning {len(json_files)} book files for vocabulary grounding...")
    
    for fpath in json_files:
        try:
            with open(fpath, "r", encoding="utf-8") as f:
                book = json.load(f)
            pages = book.get("pages", [])
            for page in pages:
                text = page.get("text", "").strip()
                image_emb = page.get("image_embedding")
                mfcc = page.get("mfcc")
                if not text or not image_emb or not mfcc:
                    continue
                
                # Register the full page text phrase
                VOCAB_EMBEDDING_MAP[text] = {
                    "image_embedding": image_emb,
                    "mfcc": mfcc
                }
                
                # Also register individual space-separated words
                words = text.split(" ")
                for word in words:
                    word = word.strip()
                    if word and word not in VOCAB_EMBEDDING_MAP:
                        VOCAB_EMBEDDING_MAP[word] = {
                            "image_embedding": image_emb,
                            "mfcc": mfcc
                        }
        except Exception as e:
            print(f"[Tutor] Warning: Failed to parse book file {fpath}: {e}")
            
    print(f"[Tutor] Loaded {len(VOCAB_EMBEDDING_MAP)} vocabulary mapping entries.")

def synthesize_embeddings(text):
    """Synthesize image/mfcc embeddings for any input text based on loaded vocabulary or fallback hashes."""
    global VOCAB_EMBEDDING_MAP
    text = text.strip()
    if not text:
        return [0.0] * 5, [0.1] * 5
        
    # Case 1: Exact match for the entire phrase
    if text in VOCAB_EMBEDDING_MAP:
        return VOCAB_EMBEDDING_MAP[text]["image_embedding"], VOCAB_EMBEDDING_MAP[text]["mfcc"]
        
    # Case 2: Mix of known individual words
    words = [w.strip() for w in text.split(" ") if w.strip()]
    matched_image = []
    matched_mfcc = []
    
    for word in words:
        if word in VOCAB_EMBEDDING_MAP:
            matched_image.append(VOCAB_EMBEDDING_MAP[word]["image_embedding"])
            matched_mfcc.append(VOCAB_EMBEDDING_MAP[word]["mfcc"])
            
    if matched_image:
        dim = len(matched_image[0])
        avg_image = [round(sum(v[i] for v in matched_image) / len(matched_image), 3) for i in range(dim)]
        avg_mfcc = [round(sum(v[i] for v in matched_mfcc) / len(matched_mfcc), 3) for i in range(dim)]
        return avg_image, avg_mfcc
        
    # Case 3: Fallback to deterministic hash seed if completely unknown
    import hashlib
    import random
    seed_bytes = hashlib.sha256(text.encode("utf-8")).digest()
    seed_int = int.from_bytes(seed_bytes, byteorder="big") % (2**32)
    local_rand = random.Random(seed_int)
    
    gen_image = [round(local_rand.uniform(-1.0, 1.0), 3) for _ in range(5)]
    gen_mfcc = [round(local_rand.uniform(-1.0, 1.0), 3) for _ in range(5)]
    return gen_image, gen_mfcc

def inject_stimulus(text, surprise_level=0.5):
    """Write stimulus atomically to close the sensory loop."""
    now_ms = int(time.time() * 1000)
    tokens = text.split(" ")
    
    # Synthesize grounded embeddings based on text content
    image_emb, mfcc = synthesize_embeddings(text)

    # 1. user_input.json
    write_atomic(
        os.path.join(MEMORY_DIR, "user_input.json"),
        {"timestamp": now_ms, "text": text}
    )

    # 2. visual.json
    write_atomic(
        os.path.join(STIMULUS_DIR, "visual.json"),
        {
            "timestamp": now_ms,
            "frame_delta": 0.3,
            "image_embedding": image_emb
        }
    )

    # 3. auditory.json
    write_atomic(
        os.path.join(STIMULUS_DIR, "auditory.json"),
        {
            "timestamp": now_ms,
            "speech_tokens": tokens,
            "mfcc": mfcc,
            "surprise_level": surprise_level
        }
    )
    print(f"[Tutor] Injected stimulus: '{text}' (Surprise={surprise_level})")
    print(f"[Tutor] Grounded Embeddings - Image: {image_emb}, MFCC: {mfcc}")

def main():
    print("[Tutor] Starting Gemini Mock Tutor daemon...")
    load_vocab_embeddings()
    api_key = os.environ.get("GEMINI_API_KEY")
    if api_key:
        print("[Tutor] GEMINI_API_KEY found. Will use online Gemini API.")
    else:
        print("[Tutor] GEMINI_API_KEY not found. Will run in local fallback mode.")

    # Tracking states
    last_processed_text_ts = 0
    last_human_input_ts = 0

    vocal_json_path = os.path.join(ACTION_DIR, "vocal_text.json")
    chat_history_path = os.path.join(MEMORY_DIR, "dashboard_chat_history.json")

    # Initial capture to avoid repeating old messages
    vocal = read_json_safe(vocal_json_path, None)
    if vocal:
        last_processed_text_ts = vocal.get("timestamp", 0)
        print(f"[Tutor] Restored last vocal timestamp: {last_processed_text_ts}")

    chat_history = read_json_safe(chat_history_path, [])
    if chat_history:
        user_msgs = [m for m in chat_history if m.get("sender") == "user"]
        if user_msgs:
            last_human_input_ts = user_msgs[-1].get("timestamp", 0)
            print(f"[Tutor] Restored last human input timestamp: {last_human_input_ts}")

    while True:
        try:
            now = time.time()
            
            # 1. Check for manual human input updates (Human-in-the-Loop Interrupt)
            chat_history = read_json_safe(chat_history_path, [])
            if chat_history:
                user_msgs = [m for m in chat_history if m.get("sender") == "user"]
                if user_msgs:
                    ts = user_msgs[-1].get("timestamp", 0)
                    if ts > last_human_input_ts:
                        last_human_input_ts = ts
                        print(f"[Tutor] Human input event detected at {ts}. Triggering interrupt.")
                    
            # 2. Check for new core vocal outputs
            vocal = read_json_safe(vocal_json_path, None)
            if vocal:
                vocal_ts = vocal.get("timestamp", 0)
                vocal_text = vocal.get("text", "").strip()

                if vocal_ts > last_processed_text_ts and vocal_text:
                    last_processed_text_ts = vocal_ts
                    print(f"[Tutor] New core output detected: '{vocal_text}' (TS: {vocal_ts})")

                    # Evaluate Human Interrupt Window
                    # We check if a manual human input happened recently
                    time_since_human = now - (last_human_input_ts / 1000.0)
                    if time_since_human < HUMAN_INTERRUPT_WINDOW_SECS:
                        print(f"[Tutor] Tutor is currently suspended (Human Interrupt Active. {int(time_since_human)}s / {HUMAN_INTERRUPT_WINDOW_SECS}s elapsed). Skipping auto-response.")
                    else:
                        # Process response
                        response_text = generate_response(vocal_text, api_key)
                        
                        # Apply Surprise Boost for simulated human dialogue (default: 0.5, but let's make it 0.85 for educational value)
                        # If the conversation is stimulated by human interaction, we boost it to 0.95.
                        # Since this is the Mock Tutor, we simulate normal surprise (0.6), 
                        # but if we want to simulate a high-priority episode, we can boost it.
                        surprise = 0.85
                        
                        # Wait 1.5 seconds to simulate think/hear delay
                        time.sleep(1.5)
                        inject_stimulus(response_text, surprise_level=surprise)

        except Exception as e:
            print(f"[Tutor] Error in main loop: {e}")

        time.sleep(POLL_INTERVAL_SECS)

if __name__ == "__main__":
    main()

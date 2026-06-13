#!/usr/bin/env python3
import os
import json
import random
import glob
import re

PROJECT_ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
BOOKS_DIR = os.path.join(PROJECT_ROOT, "books")

def fetch_local_words():
    """Fetch Hiragana words from the user-provided directory."""
    dir_path = os.environ.get("VOCAB_DIR", os.path.expanduser("~/Downloads/list/filtered"))
    print(f"[Generator] Reading Japanese words from: {dir_path}")
    words = set()
    
    files = glob.glob(os.path.join(dir_path, "gemini-code-*.txt"))
    if not files:
        print(f"[Generator] Error: No gemini-code-*.txt files found in {dir_path}!")
        return []
        
    # Hiragana and long vowel marker (ー)
    hiragana_pattern = re.compile(r'^[\u3040-\u309F\u30FC]+$')
    
    for fpath in files:
        try:
            with open(fpath, "r", encoding="utf-8") as f:
                for line in f:
                    parts = line.split(",")
                    for part in parts:
                        yomi = part.strip()
                        # Allow words with length >= 2
                        if len(yomi) >= 2 and hiragana_pattern.match(yomi):
                            words.add(yomi)
        except Exception as e:
            print(f"[Generator] Error reading file {fpath}: {e}")
            
    print(f"[Generator] Extracted {len(words)} unique Hiragana words from files.")
    return list(words)

def main():
    print("[Generator] Starting Stage 6 Large Corpus generation using user-provided Japanese vocabulary...")
    os.makedirs(BOOKS_DIR, exist_ok=True)
    
    unique_words = fetch_local_words()
    
    if not unique_words:
        print("[Generator] Error: No words found. Exiting.")
        return
        
    # Shuffle words to mix parts of speech
    random.shuffle(unique_words)
    print(f"[Generator] Final vocabulary size: {len(unique_words)}")
    
    # Construct pages
    words_per_page = 5
    pages = []
    
    for i in range(0, len(unique_words), words_per_page):
        page_words = unique_words[i:i + words_per_page]
        text = " ".join(page_words)
        
        # Generate deterministic dummy embeddings using the page text hash as a seed.
        # This prevents the core from learning random noise and corrupting its predictive model.
        import hashlib
        seed_bytes = hashlib.sha256(text.encode("utf-8")).digest()
        seed_int = int.from_bytes(seed_bytes, byteorder="big") % (2**32)
        local_rand = random.Random(seed_int)

        image_emb = [round(local_rand.uniform(-1.0, 1.0), 3) for _ in range(5)]
        mfcc = [round(local_rand.uniform(-1.0, 1.0), 3) for _ in range(5)]
        frame_delta = round(local_rand.uniform(0.1, 0.9), 2)
        
        pages.append({
            "text": text,
            "image_embedding": image_emb,
            "mfcc": mfcc,
            "frame_delta": frame_delta
        })
        
    book = {
        "book_id": "stage_06_large_corpus",
        "pages": pages
    }
    
    output_path = os.path.join(BOOKS_DIR, "stage_06_large_corpus.json")
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(book, f, ensure_ascii=False, indent=2)
        
    print(f"[Generator] Successfully created book '{book['book_id']}' with {len(pages)} pages at '{output_path}'.")

if __name__ == "__main__":
    main()

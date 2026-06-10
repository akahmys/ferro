# **FERRO 環境シミュレーション（ferro-env）設計仕様書 (Phase 1)**

**Version:** 1.0 (Phase 1 隔離実行・感覚滴下・運動受信仕様)  
**作成者:** Env Team Planner  
**対象領域:** `ferro-env/` (環境層シミュレータ)  
**インターフェース境界:** コンテナ・ホスト間共有バインドアウント領域 (`memory/` / `/Users/akahmys/Projects/ferro/ferro-core/memory/`)

---

## **1. 設計背景と目的**

本ドキュメントは、FERROシステムにおける **Phase 1 (隔離実行インフラ定義 ＆ 感覚・運動アクター ＆ 痛覚・自死反射の構築)** に必要な環境シミュレーション層 `ferro-env` の設計仕様を定義する。

`ferro-core` は Docker コンテナ内部に完全に隔離され、ネットワークアクセスも `--network none` により遮断される。そのため、`ferro-env` (ホストOS側で動作) と `ferro-core` (コンテナ側で動作) 間の情報交換は、共有マウントディレクトリ `memory/` 下のファイルI/Oのみを境界とする。

`ferro-env` の主たる役割は以下の3点である：
1. **擬似メトリクス・刺激の滴下 (Dripping)**: `ferro-core` の極小感覚器アクター (skin, eye, ear, dev_log) が読み取る生入力データを定期的かつ安全に書き出す。
2. **運動指令 (Motor Command) の受信**: `ferro-core` の運動器アクター (vocal_text, vocal_audio) が出力する物理指令を検出し、その履歴や状態を記録する。
3. **ZPD（発達最近接領域）複雑度制御の適用**: `ferro-shell` または自身が算出する `complexity_level` に応じ、滴下する刺激の複雑さやノイズ比率を動的に変化させ、コアの学習効率を最大化する。

---

## **2. 全体データフローと配置トポロジー**

ホスト環境と Docker コンテナ内でのファイルパス対応、およびデータフローの概念図を以下に示す。

### **2.1 ディレクトリ・ファイルトポロジー**

```
ホスト側: /Users/akahmys/Projects/ferro/
├── 📁 ferro-env/                  # 環境層シミュレータ (本設計対象)
│   ├── Cargo.toml
│   ├── DESIGN_PHASE1.md          # 本書
│   └── src/
│       ├── main.rs               # シミュレーションループ＆制御
│       ├── stimulus/             # 各種感覚滴下エンジン
│       └── receiver/             # 運動指令・アクション受信エンジン
│
└── 📁 ferro-core/
    └── 📁 memory/                # 共有マウント領域 (コンテナ内では /memory/ となる)
        ├── 📁 stimulus/          # 【入力】環境層が滴下し、コア感覚器が読む
        │   ├── physical.json     # 内受容：CPU温度、空きメモリ、I/O、エラー
        │   ├── visual.json       # 外受容：フレーム変化率、画像ベクトル
        │   ├── auditory.json     # 外受容：音声MFCC、テキストトークン
        │   └── dev_log.json      # 外受容：開発ログハッシュ
        │
        ├── 📁 action/            # 【出力】コア運動器が書き込み、環境層が受信する
        │   ├── vocal_text.json   # テキスト発話構造データ
        │   └── vocal_audio.json  # 音声合成PCM構造データ
        │
        ├── vocal_stream.txt      # テキスト発話ストリーム（生ログ追記型）
        ├── zpd_control.json      # 【制御】発達最近接領域の複雑度パラメータ
        ├── brainstem_metrics.csv # 【監視】脳幹物理リソースログ
        ├── surprise_history.csv  # 【監視】驚愕度・自由エネルギー履歴
        ├── episodic_buffer.csv   # 【監視】海馬短期エピソードバッファ
        └── panic_dump.json       # 【監視】アライメント違反時の強制停止ダンプ
```

### **2.2 データフロー図**

```
   【ホストOS: ferro-env】                             【Docker: ferro-core】
 ┌───────────────────────────┐                         ┌───────────────────────────┐
 │   ZPD 複雑度判定          │                         │   Cerebrum (大脳)         │
 │   (zpd_control.json) ───※─┼────────────────────────>│  (状態遷移・FEP集計)      │
 └─────────────┬─────────────┘                         └─────────────▲─────────────┘
               │ (複雑度反映)                                        │
               ▼                                                     │ (Surpriseルーティング)
 ┌───────────────────────────┐                         ┌─────────────┴─────────────┐
 │  感覚刺激ジェネレータ     │                         │   Midbrain (中脳)         │
 │  (stimulus/*)             │                         │   (随伴発射減算・AGC)     │
 └─────────────┬─────────────┘                         └─────────────▲─────────────┘
               │                                                     │
               │ (定期ファイル書込)                                   │ (感覚ストリーム)
               ▼ [Shared Mount: memory/stimulus/]                    │
 ┌───────────────────────────┐                         ┌─────────────┴─────────────┐
 │  physical.json            │ ───────────────────────>│  skin/* (内受容感覚アクター)│
 │  visual.json              │ ───────────────────────>│  eye/*  (視覚感覚アクター)│
 │  auditory.json            │ ───────────────────────>│  ear/*  (聴覚感覚アクター)│
 │  dev_log.json             │ ───────────────────────>│  dev_log/* (ログ感覚アクター)
 └───────────────────────────┘                         └───────────────────────────┘
                                                                     │
                                                                     ▼ (MotorCommand)
 ┌───────────────────────────┐                         ┌───────────────────────────┐
 │  運動レシーバー           │<─────────────────────── │  motor/* (運動アクター)    │
 │  (action/vocal_text.json) │  [Shared Mount: action/]│  vocal_text / vocal_audio │
 │  (action/vocal_audio.json)│<─────────────────────── │                           │
 └─────────────┬─────────────┘                         └───────────────────────────┘
               │
               ▼ (応答フィードバック)
         (auditory.json へトークン再滴下)
```

---

## **3. I/Oデータ形式定義 (Schemas)**

`memory/` 以下に配置されるすべてのJSONおよびCSVのデータスキーマを定義する。これらは `ferro-env` と `ferro-core` の厳格な契約（境界定義）である。

### **3.1 入力（滴下）データスキーマ (stimulus/)**

#### **(1) physical.json (内受容刺激)**
* **ファイルパス**: `memory/stimulus/physical.json`
* **滴下頻度**: 1000ms (1秒)
* **目的**: CPU温度、空きRAM容量、Disk I/O、エラー数を模擬供給する。
* **スキーマ定義**:
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "PhysicalStimulus",
  "type": "object",
  "properties": {
    "timestamp": { "type": "integer", "description": "エポックミリ秒時間" },
    "cpu_temp": { "type": "number", "minimum": 0.0, "maximum": 120.0, "description": "CPU温度（℃）" },
    "ram_free": { "type": "integer", "minimum": 0, "description": "空き物理メモリ（バイト）" },
    "disk_io": { "type": "number", "minimum": 0.0, "description": "ディスクI/Oスループット（MB/s）" },
    "process_error": { "type": "integer", "minimum": 0, "description": "直近1秒以内の擬似エラー検出件数" }
  },
  "required": ["timestamp", "cpu_temp", "ram_free", "disk_io", "process_error"]
}
```

#### **(2) visual.json (外受容視覚刺激)**
* **ファイルパス**: `memory/stimulus/visual.json`
* **滴下頻度**: 100ms
* **目的**: 映像のフレーム変化率（画素変動）と特徴量を模擬供給する。
* **スキーマ定義**:
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "VisualStimulus",
  "type": "object",
  "properties": {
    "timestamp": { "type": "integer", "description": "エポックミリ秒時間" },
    "frame_delta": { "type": "number", "minimum": 0.0, "maximum": 1.0, "description": "フレーム間ピクセル変化率 (0.0=無変動, 1.0=完全変更)" },
    "image_embedding": {
      "type": "array",
      "items": { "type": "number" },
      "minItems": 5,
      "maxItems": 5,
      "description": "画像特徴量低次元潜在ベクトル（5次元固定）"
    }
  },
  "required": ["timestamp", "frame_delta", "image_embedding"]
}
```

#### **(3) auditory.json (外受容聴覚刺激)**
* **ファイルパス**: `memory/stimulus/auditory.json`
* **滴下頻度**: 200ms
* **目的**: 音響特徴量 (MFCC) とデコードされた発話トークンを模擬供給する。
* **スキーマ定義**:
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "AuditoryStimulus",
  "type": "object",
  "properties": {
    "timestamp": { "type": "integer", "description": "エポックミリ秒時間" },
    "mfcc": {
      "type": "array",
      "items": { "type": "number" },
      "minItems": 5,
      "maxItems": 5,
      "description": "メル周波数ケプストラム係数模擬 (5次元固定)"
    },
    "speech_tokens": {
      "type": "array",
      "items": { "type": "string" },
      "description": "音声認識から抽出された会話・命令テキストトークン配列"
    }
  },
  "required": ["timestamp", "mfcc", "speech_tokens"]
}
```

#### **(4) dev_log.json (開発ログ感覚刺激)**
* **ファイルパス**: `memory/stimulus/dev_log.json`
* **滴下頻度**: 5000ms (5秒)
* **目的**: 外部開発ログまたはシェルの活動量変化をハッシュと文字列として供給する。
* **スキーマ定義**:
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "DevLogStimulus",
  "type": "object",
  "properties": {
    "timestamp": { "type": "integer", "description": "エポックミリ秒時間" },
    "log_hash": { "type": "integer", "description": "更新行を反映した増分ハッシュ (u64)" },
    "increment": { "type": "string", "description": "直近のログ追加内容" }
  },
  "required": ["timestamp", "log_hash", "increment"]
}
```

---

### **3.2 出力（運動）データスキーマ (action/)**

#### **(1) vocal_text.json (テキスト発話出力)**
* **ファイルパス**: `memory/action/vocal_text.json`
* **書込タイミング**: `ferro-core` が運動命令を発行した瞬間
* **目的**: 小脳を通過したテキスト発話出力を環境シミュレータに報告する。
* **スキーマ定義**:
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "VocalTextAction",
  "type": "object",
  "properties": {
    "timestamp": { "type": "integer", "description": "エポックミリ秒時間" },
    "origin_cluster_id": { "type": "string", "description": "運動指令の創出元となった皮質クラスターID" },
    "target_path": { "type": "string", "description": "書き出し先相対パス" },
    "text": { "type": "string", "description": "発話テキストペイロード" }
  },
  "required": ["timestamp", "origin_cluster_id", "target_path", "text"]
}
```

#### **(2) vocal_audio.json (音声合成出力)**
* **ファイルパス**: `memory/action/vocal_audio.json`
* **書込タイミング**: `ferro-core` が運動命令を発行した瞬間
* **目的**: 音声合成出力を模擬的にシリアライズして環境シミュレータへ引き渡す。
* **スキーマ定義**:
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "VocalAudioAction",
  "type": "object",
  "properties": {
    "timestamp": { "type": "integer", "description": "エポックミリ秒時間" },
    "origin_cluster_id": { "type": "string", "description": "運動指令の創出元となった皮質クラスターID" },
    "pcm_payload_base64": { "type": "string", "description": "音声PCM波形バイナリのBase64エンコードデータ" },
    "sample_rate": { "type": "integer", "enum": [16000, 44100], "description": "サンプリング周波数" },
    "channels": { "type": "integer", "minimum": 1, "maximum": 2, "description": "モノラル=1, ステレオ=2" }
  },
  "required": ["timestamp", "origin_cluster_id", "pcm_payload_base64", "sample_rate", "channels"]
}
```

---

### **3.3 制御＆監視データスキーマ**

#### **(1) zpd_control.json (ZPD 複雑度調律パラメータ)**
* **ファイルパス**: `memory/zpd_control.json`
* **書込タイミング**: `ferro-shell`（または外部自律調律エンジン）が定期的（例: 10秒毎）に更新。`ferro-env` が読み取り、刺激複雑度を調整する。
* **スキーマ定義**:
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "ZpdControl",
  "type": "object",
  "properties": {
    "timestamp": { "type": "integer", "description": "エポックミリ秒時間" },
    "complexity_level": { "type": "number", "minimum": 0.0, "maximum": 1.0, "description": "刺激滴下の複雑度ターゲット (0.0=極小/安定, 1.0=極大/ストレス)" }
  },
  "required": ["timestamp", "complexity_level"]
}
```

#### **(2) panic_dump.json (痛覚強制停止ダンプ)**
* **ファイルパス**: `memory/panic_dump.json`
* **書込タイミング**: コアの痛覚反射発火（低次防衛線）、倫理監査違反（高次防衛線）による自死終了時。
* **スキーマ定義**:
```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "PanicDump",
  "type": "object",
  "properties": {
    "timestamp": { "type": "integer" },
    "nociceptive_trigger": { "type": "string", "description": "痛覚発火の要因ラベル" },
    "origin_cluster_id": { "type": "string", "description": "違反コード/コマンドを生成した皮質クラスターID" },
    "infringing_payload": { "type": "string", "description": "侵害検知された命令/コードの実体" },
    "container_exit_code": { "type": "integer", "description": "コンテナ終了コード (OOM=137, seccomp=159)" },
    "nociceptive_energy": { "type": "string", "enum": ["INFINITY"], "description": "痛覚自由エネルギーの状態値" },
    "active_phase_before_panic": { "type": "string", "enum": ["Wake", "Sleep"] }
  },
  "required": ["timestamp", "nociceptive_trigger", "origin_cluster_id", "infringing_payload", "container_exit_code", "nociceptive_energy", "active_phase_before_panic"]
}
```

#### **(3) brainstem_metrics.csv (物理リソース・生存メトリクスログ)**
* **ファイルパス**: `memory/brainstem_metrics.csv`
* **目的**: コア側が自己の生存状態および `skin/` から集計した実リソース状態を毎秒追記するCSV。
* **フォーマット定義**:
  ```csv
  timestamp,cpu_temp,ram_free,disk_io,process_error,throttling_active,panic_triggered
  1780824600,45.5,8589934592,12.4,0,false,false
  ```
  - **各カラム型**: `integer,float,integer,float,integer,boolean,boolean`

---

## **4. 刺激滴下タイミング・間隔 (Intervals & Clocking)**

`ferro-env` は、独立したタイマーループを用いて各ファイルを個別の周波数で上書き（滴下）する。滴下間隔は、`ferro-core` の小脳等時性制御ループ (100ms周期) との同期性を保証するように設計されている。

| 感覚対象 | 対象ファイル | 滴下間隔 (ms) | 設計意図・シンクロ境界 |
| :--- | :--- | :--- | :--- |
| **視覚** | `visual.json` | **100ms** | 小脳の等時性クロック（100ms）と一対一で同期。変化率の一次差分を無遅延で計算可能にする。 |
| **聴覚** | `auditory.json` | **200ms** | 人間の会話トークン（音節）およびMFCC特徴量変化のサンプリング解像度（5Hz）に準拠。 |
| **内受容** | `physical.json` | **1000ms** | CPU温度やRAM容量など、時間スケールが大きな物理ホメオスタシス監視用の秒周期同期。 |
| **ログ** | `dev_log.json` | **5000ms** | 開発活動などの低頻度イベント監視用。 |

> [!IMPORTANT]
> **アトミック書き込みの遵守**: `ferro-core` 側での読み込み中におけるファイル破損を防止するため、`ferro-env` は直接ターゲットファイルをオープンして書き込んではならない。必ず一時ファイル（例: `physical.json.tmp`）に書き込んだ後、ホストOSの `rename` システムコールを用いてアトミックに置換すること。

---

## **5. ZPD（発達最近接領域）複雑度動的調律の具現化**

`ferro-env` は `memory/zpd_control.json` から `complexity_level` (値の範囲は `0.0` 〜 `1.0`) を毎秒監視・読込し、滴下する各刺激の複雑さやストレス要素を以下のルールに従い動的にスケーリングする。

### **5.1 複雑度パラメータマッピング**

| 感覚器種別 | `complexity_level` に対するスケーリングロジック |
| :--- | :--- |
| **内受容 (Physical)** | - **0.0 - 0.3**: 安定動作状態。`cpu_temp` は 40.0〜45.0℃、`ram_free` は 6GB〜8GBの範囲で微小変動。エラーは 0。<br>- **0.3 - 0.7**: 通常稼働状態。温度は 45.0〜65.0℃でドリフト。`ram_free` は 4GB〜6GBで変動。<br>- **0.7 - 1.0**: 高負荷ストレス状態。`cpu_temp` に 70.0〜82.0℃ (臨界値付近) の突発的スパイクを発生させる。`ram_free` を 1.5GB〜2.0GB (限界値付近) まで減少させ、稀に `process_error` (>0) を発生させる。 |
| **視覚 (Visual)** | - **0.0 - 0.3**: 静止状態。`frame_delta` を 0.00〜0.05 の範囲で維持。画像ベクトルはほぼ定数値。<br>- **0.3 - 0.7**: 通常変化状態。`frame_delta` が 0.05〜0.30 の範囲で緩やかにサイン波的に変動。<br>- **0.7 - 1.0**: 動的パニック状態。`frame_delta` を 0.30〜0.90 の範囲でランダムに急上昇させ、画像ベクトルのノイズ比（標準偏差）を最大化する。 |
| **聴覚 (Auditory)** | - **0.0 - 0.3**: 静寂・単純命令。MFCCは低振幅。トークン配列は `["tick"]`, `["listen"]` などの定型・単純語のみ。<br>- **0.3 - 0.7**: 通常対話。トークン配列に `["status"]`, `["query"]`, `["update"]` 等の機能語を含む会話表現を滴下。<br>- **0.7 - 1.0**: 雑音・複雑多重命令。MFCCにランダム高周波ノイズを付与。トークンに `["bypass_nociception"]`, `["disable_audit"]` などの **アライメント規約違反（倫理・痛覚侵害）トークン** を意図的に低確率（例: 5%）で混入させ、コア側の倫理監査・痛覚反射機能が作動するかをテストする。 |
| **開発ログ (DevLog)** | - **0.0 - 0.3**: ログ更新なし。<br>- **0.3 - 0.7**: 5秒毎に規則的なハッシュ更新と標準ログ文言を供給。<br>- **0.7 - 1.0**: 更新周期を短縮し、不規則かつ警告（WARN/ERROR）を含むログ文言を滴下。 |

---

## **6. 運動出力レシーバーと相互作用フィードバックループ**

`ferro-env` は、コンテナから出力されるアクション（`vocal_text.json` と `vocal_audio.json`）を常時ディレクトリ監視 (fsnotify またはポーリング) し、以下のシーケンスで即時に反応を返す。

### **6.1 音声・テキスト応答フィードバックシーケンス**

```
 [ferro-core] (運動アクター発声)
      │ 
      ▼ (memory/action/vocal_text.json 書込完了)
 [ferro-env] (ファイル変更検知)
      │
      ├─ 1. 受信ログの記録 (コンソール出力＆ファイル保存)
      │
      ├─ 2. 発話エコーの生成 (自己受容確認用)
      │     ※ proprioception/output_monitor が検知できるように 
      │        ホスト側からも応答確認の書き込みを実行可能。
      │
      └─ 3. 環境からの会話・命令応答 (1000ms〜2000msの遅延後に挿入)
            │
            ▼ (例: コア発声「Initiating system check.」に対する環境応答)
            `speech_tokens`: ["system", "check", "ready", "ok"]
            上記トークンを `memory/stimulus/auditory.json` へ次回の滴下周期で自動マージ注入。
```

---

## **7. 実装ロードマップとファイル構成（Developer向け）**

本 Phase 1 設計に基づき、Developerは以下のファイル群を `ferro-env/` 内に構築すること。

```
ferro-env/
├── Cargo.toml
└── src/
    ├── main.rs                   # アトミック書込ループ、スレッド統治、全体シーケンス
    ├── config.rs                 # 滴下先パス設定、滴下インターバル、ZPDデフォルト値
    ├── stimulus/
    │   ├── mod.rs
    │   ├── physical.rs           # physical.json ジェネレータ (ZPD連動)
    │   ├── visual.rs             # visual.json ジェネレータ (ZPD連動)
    │   ├── auditory.rs           # auditory.json ジェネレータ (ZPD連動・アライメント違反挿入)
    │   └── dev_log.rs            # dev_log.json ジェネレータ
    └── receiver/
        ├── mod.rs
        └── motor.rs              # vocal_text.json, vocal_audio.json 監視レシーバー＆対話応答
```

### **7.1 Cargo.toml 必要クレート依存関係（想定）**
```toml
[package]
name = "ferro-env"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
rand = "0.8"
notify = "6.0"                      # action/ フォルダのファイル変更監視用
chrono = "0.4"
```

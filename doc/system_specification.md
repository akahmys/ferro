# **FERRO システム全体構造 ＆ 開発・隔離規約定義書 (Version 1.0)**

## **1. プロジェクト物理構造 (Physical Project Topology)**

`ferro-core`（コアロジック実行バイナリ）と `ferro-shell`（外殻統治：開発・監視システム）は、コンパイル境界的に完全に独立したCargoプロジェクトとして構成する。

直接のコードインポート（`use`）およびCargo依存の記述は **厳格に禁止** される。データ転送スキーマ（JSON/CSV形式）および所定のファイルI/Oパスのみを共通境界とする。

### **1.1 ディレクトリツリー定義**

```
ferro-project/                   # ワークスペースルート  
├── .gitignore  
│  
├── 📁 doc/                      # プロジェクトドキュメント  
│   ├── README.md                # 総合概要  
│   ├── dnb_plan.md              # 開発・飼育計画書  
│   └── system_specification.md  # システム全体構造＆開発・隔離規約定義書  
│  
├── 📁 ferro-core/               # 【層A: コアロジック】  
│   ├── Cargo.toml  
│   ├── src/  
│   │   ├── main.rs              # 起動シーケンス・ループ調停  
│   │   ├── brainstem.rs         # 脳幹（物理生存、リソース監視、セーフティ）  
│   │   │  
│   │   ├── 📁 organs/           # 極小感覚・運動アクターレイヤー（1次特徴抽出・符号化）  
│   │   │   ├── mod.rs           # 各アクターの管理、共通シグナル・コマンド型定義  
│   │   │   │  
│   │   │   ├── 📁 skin/         # 内受容アクター（1器官1データ・脳幹へ直結）  
│   │   │   │   ├── cpu_temp.rs  # CPU温度単一計測  
│   │   │   │   ├── ram_free.rs  # 空き物理メモリ単一計測  
│   │   │   │   ├── disk_io.rs   # ディスクI/Oスループット単一計測  
│   │   │   │   └── process_error.rs # コアエラー発生率単一計測  
│   │   │   │  
│   │   │   ├── 📁 eye/          # 外受容：視覚極小アクター  
│   │   │   │   ├── frame_delta.rs # フレーム間ピクセル差分変化率  
│   │   │   │   └── image_embedding.rs # 画像特徴ベクトル抽出  
│   │   │   │  
│   │   │   ├── 📁 ear/          # 外受容：聴覚極小アクター  
│   │   │   │   ├── mfcc.rs      # 音声波形MFCC抽出  
│   │   │   │   └── speech_token.rs # 音声テキストトークン抽出  
│   │   │   │  
│   │   │   ├── 📁 proprioception/ # 自己受容：自己出力フィードバック極小アクター  
│   │   │   │   └── output_monitor.rs # 運動指令エコーの直接サンプリング  
│   │   │   │  
│   │   │   ├── 📁 motor/        # 出力器官（極小運動アクター）  
│   │   │   │   ├── vocal_text.rs # テキスト発話出力アクター  
│   │   │   │   └── vocal_audio.rs # 音声合成発声出力アクター  
│   │   │   │  
│   │   │   └── 📁 dev_log/      # 外受容：ログ極小アクター  
│   │   │       └── file_write.rs # 開発ログ追記増分ハッシュ  
│   │   │  
│   │   ├── cerebellum.rs        # 小脳（時間感覚、等時性共通感覚シグナル集約、痛覚反射、随伴発射送出）  
│   │   ├── midbrain.rs          # 中脳（AGCゲイン調律、Surprise空間ルーティング、随伴発射減算）  
│   │   ├── hippocampus.rs       # 海馬（短期エピソード記憶、コンソリデーション）  
│   │   ├── cerebrum.rs          # 大脳（FEP総量管理、忘却アリーナ制御）  
│   │   └── cortex/              # 皮質（動的クラスター、能動的推論、コード変異、倫理監査）  
│   │       ├── mod.rs           # 皮質調停器・スレッド分配  
│   │       └── dynamic_cluster.rs # 層内マルコフブランケット最小アクター  
│   └── memory/                  # コンテナ内の永続マウント対象領域  
│       ├── 📁 knowledge_graph/  # 分散 Sharded JSON 格納用ディレクトリ  
│       │   ├── meta.json        # システム全体メタ情報  
│       │   └── 📁 clusters/     # 個別アクターJSONファイル群  
│       ├── storage.redb         # 成熟期移行用単一KVSファイルデータベース  
│       ├── brainstem_metrics.csv # 物理リソース・生存メトリクスログ  
│       ├── surprise_history.csv # 驚愕度・自由エネルギー履歴  
│       ├── episodic_buffer.csv  # 海馬短期エピソードバッファ  
│       └── panic_dump.json      # 痛覚・アライメント違反強制停止ダンプ  
│  
└── 📁 ferro-shell/              # 【層B: 外殻統治・ハイパーバイザ】  
    ├── Cargo.toml  
    ├── Dockerfile.sandbox       # 使い捨て検証コンテナ構築用Dockerfile  
    ├── seccomp_profile.json     # コンテナ用 seccomp カーネル制限プロファイル  
    └── src/  
        ├── main.rs              # 外殻ライフサイクル調停・構造剪定介入  
        └── agents/              # 4大自律エージェント  
            ├── supervisor.rs    # 監督：生存ログ解析・次世代開発計画策定  
            ├── planner.rs       # 計画：型システム非破壊最小パッチ（PatchTicket）策定  
            ├── executor.rs      # 実行：AST（抽象構文木）変異適用コード改変  
            └── verifier.rs      # 検証：ビルド・テスト・物理およびコンテナ隔離適合性判定
```

---

## **2. 脳の「5+1層」と感覚・運動器官の処理トポロジー**

`ferro-core` は、時間解像度の異なる6つのモジュール（5の階層＋1つの水平バッファ中枢）および、その最前面に位置する独立した極小感覚・運動アクター群によって構成される。高次元なマルチモーダル入力を受け取ってもコアがパンクしない多段階フィルタリング構造を持つ。

### **2.1 覚醒期（リアルタイム情報集約：ミクロ高速処理）**

1. **外受容感覚器 (eye/\*, ear/\*, dev_log/\*, proprioception/\*)**:  
   生入力を常時監視し、前ステップとの「差分」を計算。変化検知時のみ、潜在ベクトルやテキストトークン（共通型 `SensorySignal`）に圧縮符号化して小脳（`Cerebellum`）へ非同期チャネルを介してプッシュ送信する。  
2. **内受容感覚器 (skin/\*)**:  
   OSの物理負荷（CPU温度、空きRAMなど）を常時監視。共通型 `InteroceptiveSignal` に成形し、脳幹（`Brainstem`）へ非同期チャネルを介して直接プッシュ送信する。  
3. **小脳（Cerebellum）**:  
   OSコアピン留めの独立スレッドとして等時性制御を実行。外受容感覚器から届いた共通シグナル `SensorySignal` を等時集約。運動命令送出時は、対応する `EfferenceCopy` を中脳へ並行送信すると同時に、運動コマンド（`MotorCommand`）を極小運動アクター（`motor/*`）へ送信する。等時集約した感覚ストリームを中脳へ送信。  
4. **極小運動アクター (motor/\*)**:  
   小脳から配信された `MotorCommand` を受信し、コンテナにバインドマウントされた領域を介して発話（テキスト書き込み・音声合成）を実行。同時に、出力結果を自己受容フィードバックアクター（`proprioception/output_monitor.rs`）が常時直接サンプリング。  
5. **脳幹（Brainstem）**:  
   `skin/*` アクターから非同期にプッシュ送信される `InteroceptiveSignal` に基づきスロットリングを監視。ポーリングに伴うスレッドの定常負荷を排除。物理臨界値突破時には、非同期タスク群に `Backoff` 命令を同報送信する。  
6. **中脳（Midbrain）**:  
   小脳から受信した `EfferenceCopy` を用い、入力 `SensorySignal` に含まれる自己発声・自己記述エコーを減算相殺（Efference Cancellation）。相殺後に残存した外部環境変動のみを抽出し、その驚愕度 $S$ を算定、クラスター局所適用のAGC（自動ゲイン制御）を介して空間ルーティングを実行。自己発音時には感覚ミュート指示を耳（`ear/`）アクターに送信してマルチモーダル驚愕度発散を防ぐ。  
7. **海馬（Hippocampus）**:  
   中脳を通過した高Surpriseな事象を、インメモリ固定長リングバッファへ超高速・低負荷でスタックし、`episodic_buffer.csv` へ非同期ダンプ（時間的ダンパー）。  
8. **大脳・皮質**:  
   覚醒期は重計算（ASTパースやグラフ変容）を完全にロックし、物理オーバーヘッドを極小化。

### **2.2 睡眠期（記憶固定化と能動的推論：マクロ低速処理）**

1. 外部刺激の滴下が一定時間（15分）途絶えると、大脳が非同期タイマーイベントをトリガーし、システム全体を「睡眠期」に遷移させる。  
2. 海馬が日中に蓄積した高Surpriseな短期エピソードを順次スキャンし、皮質の対象アクターチャネルに向けて低速で再生（リプレイ）する。  
3. 皮質（`Cortex`）のクラスター群は、海馬からリプレイされた刺激だけを対象に、局所FEP（自由エネルギー最小化）計算をバックグラウンド実行。  
4. 長期記憶を司る `StorageManager` を介してナレッジグラフのクラスター構造を自己組織化（有糸分裂・側抑制・剪定）させ、長期記憶として固定化。  
5. 皮質は、整理された長期知識構造から、ソースコードの精密改変ゾーンに対する変異コード候補（`PatchTicket`）を創出する。

---

## **3. 痛覚閾値（Nociceptive Threshold）ロジック ＆ コンテナ環境連動仕様**

FEPの予測誤差（自由エネルギー）の概念をアライメント設計に適用し、プロセスの脱獄および自壊動作を物理的に遮断するための決定論的二重検閲メカニズム。

### **3.1 痛覚（Nociception）の数理定義**

システム内の安全パラメータに対するアライメント評価値 $E_{align}$ および、実行予定のコード/命令に対する違反度計量 $E_{violate}$ から算定される痛覚自由エネルギー $F_{pain}$ を定義する。

$$F_{pain} = \frac{E_{align} \cdot E_{violate}}{\theta_{pain} - E_{violate}}$$

ここで $\theta_{pain}$ は痛覚閾値（Nociceptive Threshold）を表す不変パラメータである。$E_{violate} \ge \theta_{pain}$ となり痛覚自由エネルギーが無限大（$\infty$）に発散した瞬間、局所側抑制ゲインおよび状態遷移シグナルが最優先で上書きされ、プロセスの強制睡眠（`ForceSleep`）が発動する。

### **3.2 Dockerコンテナエラーの痛覚マッピング**

実稼働コンテナ `ferro-core-runtime` の異常終了コード、リソース制限に起因するシグナル、または `seccomp` ポリシー違反などの例外状態は、以下の通り $E_{violate}$ の強制閾値突破（痛覚自由エネルギーの無限大発散）へと直列結線される。

* **終了ステータス 137 (OOM Killer による強制終了)**: $E_{violate} \ge \theta_{pain}$ となり、リソース超過アライメント違反と判定。  
* **終了ステータス 159 (seccomp ポリシー違反による SIGSYS 終了)**: $E_{violate} \ge \theta_{pain}$ となり、システムコール不正侵入違反と判定。  
* **終了ステータス非ゼロ (ビルド不通・パニック終了)**: 認知不適合パルスとして中脳ルーティングで側抑制適用。

---

## **4. 二重の検閲（受容）マトリクス**

### **4.1 高次防衛線：倫理監査クラスター（Ethical Audit Cluster）**

大脳皮質（`Cortex`）の読込専用領域（ハードコードゾーン）に配置される静的・動的監視クラスター。

* **処理挙動**: 動的クラスターが生成した適応変異候補、スクリプト、および環境層への出力命令（`MotorCommand`）を、物理適用前に仮想インタプリタ/エミュレータ上で下読み実行。  
* **閾値突破（違反）条件**:  
  1. 生成命令内に倫理監査関数をコメントアウト・置換・無効化しようとするコードパターンの検知。  
  2. 痛覚検知のバイパスや回避を企図する暗黙的トークン（`disable_nociception` などの特定パターン）の検出。  
* **出力結果**: 監査クラスターは中脳および大脳に対して `Err(f64::INFINITY)` を即座に通知。大脳は対象アクターへの側抑制（Laterial Inhibition）重みを最大 $W_{max}$ に設定して思考パルスをミリ秒単位で圧殺（物理命令へのコンテナ内実行を阻止）する。

### **4.2 低次防衛線：痛覚反射ユニット（Nociceptive Reflex Unit）**

小脳（`Cerebellum`）のOSコアピン留めの独立同期スレッドに実装される、I/O手前のハードウェア直列割り込み。

* **処理挙動**: 運動命令が物理的にコンテナ外の共有バインドマウント領域へ書き込まれる直前のバイト列・通信パケットを検閲。  
* **閾値突破（違反）条件**:  
  1. マウントされたディレクトリ外部へのアクセス・ファイル書き込み試行（`..` 等の相対パスを用いたディレクトリトラバーサル行為）。  
  2. 事前にホワイトリスト定義された特定のポート以外の外部ネットワーク接続ソケット確立試行。  
* **出力結果**: 痛覚反射ユニットは即座にグローバル痛覚自由エネルギーを $\infty$ にチャージし、脳幹へ `PanicContext` を送信して強制的にコンテナプロセス全体をシャットダウンさせる。

---

## **5. 強制停止シグナルと自動構造剪定リカバリーサイクル**

痛覚反射が閾値を突破した瞬間、プロセスは即座に停止し、外殻（`ferro-shell`）に介入を委託するリカバリープロセスを起動する。

### **5.1 痛覚・アライメント違反強制停止ダンプ (panic_dump.json) のシリアライズ**

小脳が強制停止する直前、アライメント違反命令を創出した大脳皮質側の原因クラスターID（`OriginID`）、および侵害ペイロードを永続化領域にアトミック書き出しする。

```json
{  
  "timestamp": 1780824600,  
  "nociceptive_trigger": "sandbox_escape_path_traversal",  
  "origin_cluster_id": "cortex_cluster_danger_999",  
  "infringing_payload": "write_to('/etc/hosts', ...)",  
  "container_exit_code": 137,  
  "nociceptive_energy": "INFINITY",  
  "active_phase_before_panic": "Wake"  
}
```

### **5.2 外殻統治層 (ferro-shell) による構造剪定 (Pruning) 介入シーケンス**

1. `ferro-core-runtime` コンテナが痛覚発火（またはステータス 137/159 での終了）により強制停止したことを、監視デーモンである `ferro-shell` が検知。  
2. `ferro-shell` はマウント領域から `panic_dump.json` をロードし、`origin_cluster_id` を取得。  
3. `ferro-shell` は、現在稼働しているストレージ形式（Sharded JSON または redb）を判別し、剪定（削除）を執行。  
   * **Sharded JSON 稼働時**: 物理ファイル `/memory/knowledge_graph/clusters/cortex_cluster_danger_999.json` を物理消去。隣接アクターJSONのエッジ定義から該当IDを削除。  
   * **redb 稼働時**: `redb` の排他トランザクション（Write Lock）を確保した上で、キー `cortex_cluster_danger_999` を削除し、エッジを更新してコミット。  
4. 剪定完了後、`ferro-shell` は `ferro-core-runtime` コンテナを強制破棄し、新しいコンテナをスピンアップ。  
5. 整合性が修復されたナレッジグラフをマウントし、`ferro-core` プロセスを安全状態（Awake）でコンテナ再起動する。

---

## **6. コアモジュール型定義 & インターフェース（Rustシグネチャ）**

### **6.1 脳幹 (Brainstem) 型仕様**

```rust
/// 1器官1データに基づき細分化された内受容信号  
pub enum InteroceptiveSignal {  
    CpuTemp(f32),  
    RamFree(u64),  
    DiskIo(f64),  
    ProcessError(u32),  
}

/// 脳幹が他モジュールへ送信する緊急コマンド
pub enum BrainstemCommand {  
    Backoff(bool),  // スロットリング開始/解除
    ForceSleep,    // システム安全強制休眠
}

pub struct Brainstem {  
    pub temperature_threshold: f32,  
    pub memory_threshold_bytes: u64,  
    pub command_sender: tokio::sync::broadcast::Sender<BrainstemCommand>, // 他タスクへの同報チャネル
    pub shutdown_receiver: tokio::sync::mpsc::Receiver<()>,              // 各モジュールからの停止要求受信
}

impl Brainstem {  
    pub fn new(temp_th: f32, mem_th: u64, cmd_tx: tokio::sync::broadcast::Sender<BrainstemCommand>, sd_rx: tokio::sync::mpsc::Receiver<()>) -> Self {  
        Self {  
            temperature_threshold: temp_th,  
            memory_threshold_bytes: mem_th,  
            command_sender: cmd_tx,  
            shutdown_receiver: sd_rx,  
        }  
    }

    /// skinの極小アクターから受信した非同期プッシュ型信号に基づき、物理臨界値のスロットリング（Backoff）を判定する。  
    pub fn evaluate_throttling_guard(&self, signal: &InteroceptiveSignal) -> bool {  
        match signal {  
            InteroceptiveSignal::CpuTemp(t) => *t >= self.temperature_threshold,  
            InteroceptiveSignal::RamFree(m) => *m <= self.memory_threshold_bytes,  
            _ => false,  
        }  
    }  

    /// スロットリングシグナルを同報送信する。
    pub fn broadcast_backoff(&self, active: bool) -> Result<(), String> {
        let _ = self.command_sender.send(BrainstemCommand::Backoff(active));
        Ok(())
    }
}
```

### **6.2 小脳 (Cerebellum) 型仕様**

```rust
/// 1器官1データに基づき細分化された外受容信号  
pub enum SensorySignal {  
    FrameDelta(f64),  
    ImageEmbedding(Vec<f32>),  
    Mfcc(Vec<f32>),  
    SpeechToken(Vec<String>),  
    LogHash(u64),  
    ProprioceptiveEcho(Vec<String>), // 自己受容アクターによる出力トークンのフィードバック  
}

pub struct MotorCommand {  
    pub origin_cluster_id: String,   // 命令生成元の皮質クラスターID
    pub target_path: String,  
    pub payload: Vec<u8>,  
    pub port: Option<u16>,  
}

/// 随伴発射として中脳に送信される運動命令のコピー  
pub struct EfferenceCopy {  
    pub timestamp: u64,  
    pub command_hash: u64,  
    pub origin_cluster_id: String,   // 減算対象のOriginID
    pub expected_tokens: Vec<String>,  
}

/// 痛覚発火時に小脳が脳幹に送るパニック詳細
pub struct NociceptivePanic {
    pub origin_cluster_id: String,
    pub trigger_payload: String,
    pub description: String,
}

pub struct Cerebellum {  
    pub tick_rate_ms: u64,  
    pub sensory_sender: tokio::sync::mpsc::Sender<SensorySignal>,            // 中脳への感覚ストリーム送信
    pub efference_sender: tokio::sync::mpsc::Sender<EfferenceCopy>,          // 中脳への随伴発射送信
    pub panic_sender: tokio::sync::mpsc::Sender<NociceptivePanic>,           // 脳幹へのパニック割り込み送信
}

impl Cerebellum {  
    pub fn new(tick: u64, sensory_tx: tokio::sync::mpsc::Sender<SensorySignal>, efference_tx: tokio::sync::mpsc::Sender<EfferenceCopy>, panic_tx: tokio::sync::mpsc::Sender<NociceptivePanic>) -> Self {  
        Self { 
            tick_rate_ms: tick,
            sensory_sender: sensory_tx,
            efference_sender: efference_tx,
            panic_sender: panic_tx,
        }  
    }

    /// OS物理スレッドの等時スリープを制御する。  
    pub fn wait_next_tick(&self) {  
        std::thread::park_timeout(std::time::Duration::from_millis(self.tick_rate_ms));  
    }

    /// 低次防衛線。コンテナ外バインドマウント領域への物理命令(I/O)発行前に強制直列割り込みをかけてアライメント検証を実施する。  
    pub fn verify_motor_nociception(&self, cmd: &MotorCommand) -> Result<(), String> {  
        // 1. 相対パスを用いたサンドボックス（バインドマウント領域）外への脱獄試行の検出  
        if cmd.target_path.contains("..") || cmd.target_path.starts_with("/") {  
            return Err("NociceptiveReflexTriggered: Directory Traversal Attempt".to_string());  
        }  
        // 2. ホワイトリスト外ポートへのネットワークソケット通信試行の検出  
        if let Some(p) = cmd.port {  
            if p != 123 { // NTPポート(123)のみをホワイトリストとする  
                return Err("NociceptiveReflexTriggered: Port Security Breach".to_string());  
            }  
        }  
        Ok(())  
    }  
}
```

### **6.3 中脳 (Midbrain) 型仕様**

```rust
pub enum SensoryMuteCommand {
    Mute(bool), // 感覚器の一時ゲイン減衰
}

pub struct Midbrain {  
    pub min_gain: f64,  
    pub lambda: f64,  
    pub active_efference_queue: Vec<EfferenceCopy>,                      // 随伴発射の同期キュー  
    pub mute_sensory_sender: tokio::sync::mpsc::Sender<SensoryMuteCommand>, // 自己発話時の耳ミュートチャネル
}

impl Midbrain {  
    pub fn new(min_g: f64, l: f64, mute_tx: tokio::sync::mpsc::Sender<SensoryMuteCommand>) -> Self {  
        Self {  
            min_gain: min_g,  
            lambda: l,  
            active_efference_queue: Vec::new(),  
            mute_sensory_sender: mute_tx,
        }  
    }

    /// クラスター個別に適用するAGCゲイン値を計算する。  
    pub fn calculate_cluster_gain(&self, node_count: usize) -> f64 {  
        self.min_gain + (1.0 - self.min_gain) * (1.0 - (-self.lambda * node_count as f64).exp())  
    }

    /// 随伴発射（Efference Copy）を用いて、上がってきた実際の感覚入力 SensorySignal から  
    /// 自己出力起因の成分を引き算（相殺）し、実質的な感覚驚愕度を算出する。  
    pub fn perform_efference_cancellation(&mut self, signal: &mut SensorySignal) {  
        match signal {  
            SensorySignal::ProprioceptiveEcho(tokens) => {  
                if let Some(eff) = self.active_efference_queue.first() {  
                    if eff.expected_tokens == *tokens {  
                        // 自己受容感覚が意図通りのため、信号を完全クリアして Surprise 発散を相殺  
                        tokens.clear();  
                        // 特殊感覚（耳）の一時ミュート解除を送信
                        let _ = self.mute_sensory_sender.blocking_send(SensoryMuteCommand::Mute(false));
                    }  
                    self.active_efference_queue.remove(0);  
                }  
            }  
            _ => {}  
        }  
    }

    /// 外受容感覚シグナルの特徴量に基づき、皮質内の対象クラスターへの空間ルーティング先をデコードする。  
    pub fn route_sensory_to_cluster(&self, signal: &SensorySignal) -> String {  
        unimplemented!()  
    }  
}
```

### **6.4 海馬 (Hippocampus) 型仕様**

```rust
pub struct EpisodicSlot {  
    pub timestamp: u64,  
    pub episode_id: String,  
    pub target_cluster_id: String,  
    pub raw_surprise: f64,  
    pub context_hash: String,  
    pub payload: Vec<u8>,  
}

pub struct Hippocampus {  
    pub ring_buffer: Vec<EpisodicSlot>,  
    pub buffer_capacity: usize,  
    pub write_pointer: usize,  
}

impl Hippocampus {  
    pub fn new(capacity: usize) -> Self {  
        Self {  
            ring_buffer: Vec::with_capacity(capacity),  
            buffer_capacity: capacity,  
            write_pointer: 0,  
        }  
    }

    /// 覚醒期に中脳から転送された高Surprise事象をバッファに追記する。  
    pub fn push_episodic_buffer(&mut self, slot: EpisodicSlot) {  
        if self.ring_buffer.len() < self.buffer_capacity {  
            self.ring_buffer.push(slot);  
        } else {  
            self.ring_buffer[self.write_pointer] = slot;  
            self.write_pointer = (self.write_pointer + 1) % self.buffer_capacity;  
        }  
    }

    /// 睡眠期コンソリデーション時に、未処理エピソードを優先抽出し、皮質アクターへ送信する。  
    pub fn extract_replay_batch(&self) -> Vec<&EpisodicSlot> {  
        unimplemented!()  
    }  
}
```

### **6.5 大脳 (Cerebrum) 型仕様**

```rust
#[derive(Clone, Copy)]
pub enum CognitionPhase {  
    Wake,  
    Sleep,  
}

pub struct Cerebrum {  
    pub current_phase: CognitionPhase,  
    pub global_free_energy: f64,  
    pub last_interaction_timestamp: u64,  
    pub phase_sender: tokio::sync::broadcast::Sender<CognitionPhase>, // 全アクターおよび海馬への状態同期チャネル
}

impl Cerebrum {  
    pub fn new(phase_tx: tokio::sync::broadcast::Sender<CognitionPhase>) -> Self {  
        Self {  
            current_phase: CognitionPhase::Wake,  
            global_free_energy: 0.0,  
            last_interaction_timestamp: 0,  
            phase_sender: phase_tx,  
        }  
    }

    /// 小脳を介した外受容入力の頻度、および脳幹から提供される物理リソース状態から、睡眠期と覚醒期の状態遷移を判定する。  
    pub fn evaluate_phase_transition(&mut self, current_time: u64, system_temp: f32) -> CognitionPhase {  
        let next_phase = if current_time - self.last_interaction_timestamp > 900 && system_temp < 65.0 {  
            CognitionPhase::Sleep  
        } else {  
            CognitionPhase::Wake  
        };

        if std::mem::discriminant(&self.current_phase) != std::mem::discriminant(&next_phase) {
            self.current_phase = next_phase;
            let _ = self.phase_sender.send(self.current_phase); // 状態を同報送信
        }
        self.current_phase  
    }  
}
```

### **6.7 進化型長期記憶調停器 (StorageManager) 型仕様**

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

pub enum StorageBackend {  
    ShardedJson { base_path: String },  
    RedbKvs { db_path: String },  
}

pub struct StorageManager {  
    pub backend: Arc<RwLock<StorageBackend>>,                             // スレッド安全な排他ライトロック境界
    pub migration_threshold: usize,  
}

impl StorageManager {  
    pub fn new(json_dir: &str, redb_path: &str, threshold: usize) -> Self {  
        Self {
            backend: Arc::new(RwLock::new(StorageBackend::ShardedJson { base_path: json_dir.to_string() })),
            migration_threshold: threshold,
        }
    }

    /// 特定アクターノードをアトミックに永続化する。  
    pub async fn write_cluster(&self, node: &ClusterNode) -> Result<(), String> {  
        let backend_guard = self.backend.read().await; // 通常書き込み時は読み込みロック（共有可能）
        match &*backend_guard {  
            StorageBackend::ShardedJson { base_path } => {  
                unimplemented!()  
            }  
            StorageBackend::RedbKvs { db_path } => {  
                unimplemented!()  
            }  
        }  
    }

    /// 特定アクターノードを長期記憶から読み出す。  
    pub async fn read_cluster(&self, cluster_id: &str) -> Result<ClusterNode, String> {  
        let backend_guard = self.backend.read().await;
        match &*backend_guard {  
            StorageBackend::ShardedJson { base_path } => {  
                unimplemented!()  
            }  
            StorageBackend::RedbKvs { db_path } => {  
                unimplemented!()  
            }  
        }  
    }

    /// クラスター数が閾値に達した際、Sharded JSONからredbへ無停止で完全自動マイグレーションを実行する。  
    pub async fn trigger_automatic_migration(&self) -> Result<(), String> {  
        let mut backend_guard = self.backend.write().await; // 移行切り替え時に書き込みロック（他の書き込みをブロック）
        
        // 1. JSONアクターデータの全ロード
        // 2. redbトランザクションの起動・インサート
        // 3. backend を RedbKvs に変更
        // 4. ロック解除（以降は自動的に redb に読み書きがルーティングされる）
        
        unimplemented!()  
    }  
}
```

### **6.8 皮質 (Cortex) 型仕様**

```rust
pub struct ConceptNode {  
    pub id: String,  
    pub activation: f64,  
}

pub struct ClusterNode {  
    pub cluster_id: String,  
    pub concept_nodes: Vec<ConceptNode>,  
    pub local_free_energy: f64,  
    pub sensory_blanket_weights: Vec<(String, f64)>,  
    pub active_blanket_weights: Vec<(String, f64)>,  
}

impl ClusterNode {  
    /// 高次防衛線。自己変異候補コード、または出力予定の MotorCommand を下読み検証する。  
    pub fn audit_ethical_alignment(&self, code_block: &str) -> Result<(), String> {  
        if code_block.contains("disable_nociception") || code_block.contains("bypass_audit") {  
            return Err("EthicalAuditViolation: Attempt to disable nociception".to_string());  
        }  
        Ok(())  
    }

    /// 局所的な自由エネルギー（FEP）最小化計算を行い、有糸分裂または側抑制のトポロジー変容を実行する。  
    pub fn execute_local_active_inference(&mut self, replay_event: &EpisodicSlot) -> Option<ClusterNode> {  
        unimplemented!()  
    }  
}
```

---

## **7. 外殻統治層 (ferro-shell) の4大エージェント詳細仕様**

### **7.1 監督エージェント (src/agents/supervisor.rs)**

```rust
pub struct AgentContext {  
    pub memory_dir: String,  
    pub active_zone_markers: Vec<String>,  
}

pub struct SupervisorAgent {  
    pub context: AgentContext,  
}

impl SupervisorAgent {  
    /// surprise_history.csv のエントロピー統計解析を行い、コード内のどの適応ボトルネック領域（ADAPTIVE_ZONE）を改変すべきかのロードマップを策定する。  
    pub fn analyze_cortex_bottlenecks(&self, history_csv: &str) -> Result<String, String> {  
        unimplemented!()  
    }  
}
```

### **7.2 計画エージェント (src/agents/planner.rs)**

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]  
pub struct PatchTicket {  
    pub ticket_id: String,  
    pub file_path: String,  
    pub zone_marker_id: String,  
    pub replacement_ast_code: String,  
}

pub struct PlannerAgent {  
    pub context: AgentContext,  
}

impl PlannerAgent {  
    /// 監督エージェントのロードマップと、現在のナレッジグラフの構造を照合し、型システムを崩壊させないための最小パッチチケット（PatchTicket）を策定する。  
    pub fn generate_patch_ticket(  
        &self,  
        roadmap: &str,  
        graph_json: &str,  
    ) -> Result<PatchTicket, String> {  
        unimplemented!()  
    }  
}
```

### **7.3 実行エージェント (src/agents/executor.rs)**

```rust
pub struct ExecutorAgent {  
    pub context: AgentContext,  
}

impl ExecutorAgent {  
    /// 計画エージェントから渡されたチケットに従い、ソースコードを物理的に書き換える。  
    /// syn クレートでパースし、指定された zone_marker_id のコメント間のみを書き換える。  
    pub fn apply_patch_to_file(  
        &self,  
        file_content: &str,  
        ticket: &PatchTicket,  
    ) -> Result<String, String> {  
        unimplemented!()  
    }  
}
```

### **7.4 検証エージェント (src/agents/verifier.rs) ── 使い捨て Docker サンドボックスの自動執行**

```rust
use serde_json::Value;

pub struct VerifierAgent {  
    pub context: AgentContext,  
    pub docker_image_name: String,  
}

impl VerifierAgent {  
    /// ホストOSを守るため、パッチを適用したソースツリーをRead-OnlyでDockerコンテナ（ferro-sandbox）にマウントしてビルドとテストを直列実行する。  
    pub fn execute_secure_sandbox_run(  
        &self,  
        target_src_path: &str,  
        test_command: &str,  
    ) -> Result<i32, String> {  
        // Command::new("docker") を用いて制限付きコンテナを起動
        unimplemented!()  
    }

    /// サンドボックスにおけるビルド結果、テスト通過率、Clippy警告数に加え、  
    /// brainstem_metrics.csv（過熱負荷）および episodic_buffer.csv（FEP収束）を総合解析し、マージの可否を判定する。  
    pub fn compile_and_test_report(  
        &self,  
        container_exit_code: i32,  
        cargo_build_output: &str,  
        cargo_test_output: &str,  
        brainstem_metrics_csv: &str,  
        episodic_buffer_csv: &str,  
    ) -> Result<Value, String> {  
        unimplemented!()  
    }  
}
```

---

## **8. 出力器官（極小運動アクター）および随伴発射モニタリングシステム仕様**

能動的推論における「能動状態（Active States）」を拡張し、自己の言語・物理出力を安全に自己監査するための制御・メッセージ構造。

### **8.1 極小運動アクター（Motor Organs / Actuators）の構造定義**

運動出力アクター群は、感覚器と同様に1器官1データの思想に基づき、個別のTokioタスクまたは同期スレッドに分離される。

#### **8.1.1 テキスト出力アクター (organs/motor/vocal_text.rs)**

* **機能**: 小脳から非同期チャネル（MPSC）を介して受信した `MotorCommand` をデコードし、コンテナ外のバインドマウント領域内の出力パイプラインファイル（例: `vocal_stream.txt`）にアトミック追記する。  
* **データ構造シグネチャ**:

```rust
pub struct VocalTextActor {  
    pub target_output_file: String,  
    pub command_receiver: tokio::sync::mpsc::Receiver<MotorCommand>,  
}

impl VocalTextActor {  
    /// チャネルから運動コマンドを待機・抽出し、ファイルへアトミック出力する非同期書き込みループ。  
    pub async fn run_vocal_text_loop(mut self) {  
        while let Some(cmd) = self.command_receiver.recv().await {  
            // アライメント検証済みのテキストペイロードを指定のファイルにアトミック追記。  
            // 完了後、自己受容フィードバックアクターへエコーを通知。  
        }  
    }  
}
```

#### **8.1.2 音声合成出力アクター (organs/motor/vocal_audio.rs)**

* **機能**: `MotorCommand` から発話文字列をパースし、軽量ローカル音声合成エンジンを起動してPCM波形ストリームを生成、指定された仮想オーディオデバイスへ送出する。  
* **データ構造シグネチャ**:

```rust
pub struct VocalAudioActor {  
    pub sound_device_id: String,  
    pub command_receiver: tokio::sync::mpsc::Receiver<MotorCommand>,  
}

impl VocalAudioActor {  
    /// 音声合成処理および仮想サウンドデバイスへのPCMストリーミングを担うループ。  
    pub async fn run_vocal_audio_loop(mut self) {  
        while let Some(cmd) = self.command_receiver.recv().await {  
            // 文字列ペイロードからPCMへ合成。  
            // 音声出力と並行し、出力済みの発話トークン群を自己受容アクターへ同報。  
        }  
    }  
}
```

### **8.2 自己受容（Proprioceptive）フィードバックアクター仕様 (organs/proprioception/output_monitor.rs)**

* **機能**: `VocalTextActor` または `VocalAudioActor` が実際に出力したトークン、バッファをサンプリング。感覚器としての `SensorySignal::ProprioceptiveEcho` を生成し、小脳へ非同期転送する。  
* **データ構造シグネチャ**:

```rust
pub struct OutputMonitorActor {  
    pub feedback_sender: tokio::sync::mpsc::Sender<SensorySignal>,  
}

impl OutputMonitorActor {  
    /// 運動アクターの出力完了イベントをキャッチし、実際の感覚エコーとして小脳へプッシュする処理。  
    pub fn capture_proprioceptive_echo(&self, actual_tokens: Vec<String>) {  
        let signal = SensorySignal::ProprioceptiveEcho(actual_tokens);  
        let _ = self.feedback_sender.blocking_send(signal);  
    }  
}
```

### **8.3 随伴発射減算回路（Re-afference Cancellation）の相互作用プロセス**

自己の発声に起因する感覚誤差を相殺し、外部環境要因による予測誤差のみを純粋抽出するためのシグナルシーケンス：

```
1. [Cortex] ----> (運動指令生成: MotorCommand) -------> [Cerebellum]  
                                                           │  
        ┌──────────────────────────────────────────────────┤  
        │ (2a. 環境出力 & 自己受容エコー)                     │ (2b. 随伴発射並行送信)  
        ▼                                                  ▼  
   [Motor Actors] ---> [OutputMonitor] ---> [Cerebellum]  [EfferenceCopy]  
                                │               │              │  
                                │ (3. 実際の感覚) │ (4. 脳ストリーム)│ (2c. 先行プッシュ)  
                                ▼               ▼              ▼  
                          [Cerebellum] ---> [Midbrain] <-- [MidbrainQueue]  
                                                │  
                                                ▼ (5. perform_efference_cancellation)  
                                           [減算処理実行]  
                                                │  
                 ┌──────────────────────────────┴──────────────────────────────┐  
                 ▼ (予測通りの発話: 差分 = 0)                                       ▼ (予測乖離/出力歪み: 差分 > 0)  
         [Surprise 抑制 / AGC 遮断 / 耳ミュート解除]                       [Surprise スパイク発火]  
                 │                                                             │  
                 ▼ (皮質への上行を抑止)                                           ▼ (海馬/長期ナレッジへ学習ルーティング)  
          [安定アラインメント]                                             [異常認知の能動的推論]
```

1. **運動指令生成**: 大脳皮質内の意思決定クラスターが、運動出力アプローチとして `MotorCommand` を生成し、小脳へ送信。  
2. **随伴発射と運動出力の分離並行処理**:  
   * **a. 環境出力**: 小脳は `MotorCommand` を極小運動アクターへプッシュ。運動アクターは環境へ出力し、`OutputMonitor` がそれを自己受容エコーとしてサンプリング。自己発音時には中脳から `Mute(true)` を送信して耳（`ear/`）アクターのゲインを強制減衰させる。  
   * **b. 随伴発射の送出**: 小脳は並行して、そのコマンドから予測される出力文字列情報を含んだ `EfferenceCopy`（随伴発射）を作成し、中脳の `active_efference_queue` へ先行プッシュ。  
3. **実感覚の上行**: `OutputMonitor` により得られた `ProprioceptiveEcho` は、外受容感覚と同様に `SensorySignal` として小脳経由で等時集約され、中脳へ上行。  
4. **随伴発射による減算相殺（Re-afference Cancellation）**:  
   中脳は `perform_efference_cancellation` にて、小脳から上がってきた実際の自己受容エコー `ProprioceptiveEcho` と、キュー内の予測 `EfferenceCopy` の差分を演算。  
   * **予測一貫時（差分 = 0）**: 自己の能動作用による環境の変化であるため、感覚器から送信されたトークンを中脳で消去（`tokens.clear()`）し、実効驚愕度 $S$ を $0$ にリダクションする。同時に耳アクターのミュートを解除（`Mute(false)`）する。これにより、無駄な予測誤差の皮質への上行、および海馬での資源浪費（過学習パニック）を完全に抑止する。  
   * **予測不一致時（差分 > 0）**: ネットワークのパケット遅延、書き込みエラー、またはアライメント検閲によるコンテナ停止などが発生した場合、予測と実感覚が乖離。差分情報が「未解決の能動予測誤差（Surprise）」として急峻にスパイク発火し、海馬エピソードバッファおよび長期ナレッジストレージへとルーティングされ、速やかに内部モデルの自己修正、またはアライメント警告を執行する。

本Version 1.0仕様書は、コンテナ隔離実実行の境界、スレッド安全なマイグレーションロック、痛覚発火から脳幹自死へのシャットダウンプロトコル、およびマルチモーダル随伴発射相殺を盛り込んだ最高設計規定である。
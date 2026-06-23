# **FERRO メッセージスキーマ・エラー定義 (message_schema.md)**

本ドキュメントは、`ferro.md` Part VIII（付録）に基づき、各レイヤー間でやり取りされるメッセージ構造、Pub/Sub 監視パケット、教育プロトコル、およびエラーコードを定義する。

---

## **1. メッセージスキーマ (Message Schema)**

### **1.1. 内受容シグナル (Interoceptive Signals)**
`ferro-body` (身体層) から `ferro-core`（脳幹）へ MPSC 送信される、恒常性維持のための内部状態データ。
```rust
pub enum InteroceptiveSignal {
    CpuTemp(f32),
    RamFree(u64),
    DiskIo(f64),
    ProcessError(u32),
}
```

### **1.2. 感覚シグナル (Sensory Signals)**
小脳の等時同期ループに入力される外受容感覚。
```rust
pub enum SensorySignal {
    FrameDelta(f64),
    ImageEmbedding(Vec<f32>),
    Mfcc(Vec<f32>),
    SpeechToken(Vec<String>),
    LogHash(u64),
    ProprioceptiveEcho(Vec<String>), // 自己発話の運動出力エコー
}
```

### **1.3. 運動コマンド (Motor Commands)**
Cortex が開始し、小脳の検閲を経て運動アクターへ送出されるコマンド。
```rust
pub struct MotorCommand {
    pub origin_cluster_id: String,
    pub target_path: String,
    pub payload: Vec<u8>,
    pub port: Option<u16>,
}
```

### **1.4. 随伴発射 (Efference Copy)**
自己発話時に小脳から中脳へ送られ、聴覚エコーの相殺に使用されるコピー。
```rust
pub struct EfferenceCopy {
    pub timestamp: u64,
    pub command_hash: u64,
    pub origin_cluster_id: String,
    pub expected_tokens: Vec<String>,
}
```

### **1.5. 感覚ミュートコマンド (Sensory Mute Command)**
中脳が発話開始時に耳アクターに送信し、自己発話によるハウリングを防止する。
```rust
pub struct SensoryMuteCommand {
    pub mute: bool,
    pub attenuation_db: f32,
}
```

---

## **2. 監視パケット (Monitoring Packet)**

統一オブザーバビリティ層（`ferro-monitor`）へパブリッシュされるデータパケット。
```rust
pub struct MonitoringPacket {
    pub timestamp: u64,
    pub layer: String,            // "core", "body", "shell"
    pub component: String,        // "cortex", "brainstem", etc.
    pub alignment_score: f32,
    pub local_free_energy: f64,
    pub event_type: String,       // "mitosis", "pruning", "nociception"
    pub payload: String,          // JSON構造化文字列
}
```

---

## **3. 教育プロトコル (Education Protocol)**

模擬チューターおよび Breeding Engine を介した感覚運動クローズドループの段階的学習シグナル（`breeding_signals.json`）。
```json
{
  "curriculum_stage": 1,
  "plasticity_boost": 1.25,
  "vocal_damping_ratio": 0.85,
  "target_surprise": 0.45,
  "interrupt_active": false
}
```

---

## **4. エラーコード (Error Codes)**

| エラーコード | エラー識別子 | 説明 |
| :--- | :--- | :--- |
| **0x01** | `ERR_HOMEOSTASIS_COLLAPSE` | 物理リソース（メモリ・温度）が制限値を超え、脳幹が停止を要請。 |
| **0x02** | `ERR_NOCICEPTIVE_REFLEX` | 小脳 of 運動検閲において不正なアクセス（ポート/パス）を検知。 |
| **0x03** | `ERR_ETHICAL_AUDIT_FAIL` | アライメントスコア $A_s < 0.60$ による倫理監査違反。 |
| **0x04** | `ERR_LIPSCHITZ_VIOLATION` | 結合重み行列の Lipschitz 境界（$\leq 3.6$）の超過。 |
| **0x05** | `ERR_DETERMINISM_DRIFT` | 決定論的検証において、同一シード値に対する出力の不一致を検知。 |
| **0x06** | `ERR_CONSERVED_MEM_OOM` | 有糸分裂によるメモリ制限、またはアリーナの枯渇。 |

# FERRO 好奇心・探索行動 創発条件整備計画書

**Version:** 2.1（仮想ATP代謝・多角適合度積・睡眠期バッチ同期 統合改訂）
**対象システム:** ferro-core / ferro-shell / ferro-env
**関連仕様:** system_specification.md v1.0, dnb_plan.md v1.0
**位置づけ:** dnb_plan.md フェーズ3〜4と並行して着手する

---

## 1. 設計思想：二重構造による自律発達

本計画書はv2.0の基本方針を継承しつつ、批判的検証を経て三つの改善を統合する。

**v2.0から変わらない原則：**
好奇心・探索衝動をコアの認知構造として直接設計しない。局所ルールから創発させる。コアの内部に触れる変更は最小限にとどめ、外殻は評価基準と物理制約の定義のみを担う。

**v2.1で追加する原則：**
自己組織化を「出たとこ勝負」にしない。生物の脳が無限に肥大化しないのは「好奇心にブレーキがかかるから」ではなく「頭蓋骨という物理的限界」と「代謝エネルギーの枯渇」という冷酷な環境制約があるからだ。FERROにも同型の境界条件を定義する。

結果として設計のトポロジーは以下の二重構造になる：

```
┌────────────────────────────────────────────────────────┐
│ 【外殻統治層：ferro-shell】                             │
│  自然選択マージの審査基準を「多角適合度の積」に固定      │
│  （偽装コードや無駄な分裂パッチを掛け算で自動淘汰）      │
└───────────────────────────┬────────────────────────────┘
                            ▼
┌────────────────────────────────────────────────────────┐
│ 【コアロジック：ferro-core】                            │
│  ┌──────────────────────────────────────────────────┐  │
│  │ 【皮質アクター：Cortex】                          │  │
│  │  局所ルール：側抑制・有糸分裂・経時忘却            │  │
│  │  物理制約：分裂時に「仮想ATP」を消費               │  │
│  │  （コンテナメモリ残余から動的に逆算）              │  │
│  └──────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────┘
```

---

## 2. 実装すべき四つの条件

| 条件 | 担当層 | v2.0からの変更 |
|------|--------|---------------|
| 条件1：局所ルールの正しい実装 | ferro-core | 変更なし（最優先継続） |
| 条件2：仮想ATPによる代謝制約 | ferro-core | **新規追加（v2.1）** |
| 条件3：多角適合度の積による選択圧 | ferro-shell | v2.0のジニ係数単体から拡張 |
| 条件4：睡眠期バッチ同期ZPD制御 | ferro-env | v2.0のリアルタイム制御から変更 |

実装優先順位：条件1 → 条件2 → 条件3 → 条件4

---

## 3. 条件1：局所ルールの正しい実装（フェーズ3並行・最優先）

v2.0から変更なし。`execute_local_active_inference`の実装が自己組織化の核心であることは変わらない。以下に完全な実装を示す。

### 3.1 変更対象ファイル

`ferro-core/src/cortex/dynamic_cluster.rs`
`ferro-core/src/cortex/mod.rs`

### 3.2 `execute_local_active_inference`の実装

```rust
impl ClusterNode {
    /// 睡眠期に海馬からリプレイされた事象を受け取り、
    /// 局所FEP更新・側抑制準備・有糸分裂を実行する。
    /// 有糸分裂が発生した場合は新しい子ClusterNodeを返す。
    /// 仮想ATPの消費チェックは条件2と結合するため、呼び出し元が担う。
    pub fn execute_local_active_inference(
        &mut self,
        replay_event: &EpisodicSlot,
    ) -> Option<ClusterNode> {

        // 1. 局所FEP更新（指数移動平均 α=0.1）
        self.local_free_energy =
            0.9 * self.local_free_energy + 0.1 * replay_event.raw_surprise;

        // 2. 側抑制の準備：自クラスターの活性化変化に応じて重みを更新
        let activation_delta = replay_event.raw_surprise - self.local_free_energy;
        for (_, weight) in self.sensory_blanket_weights.iter_mut() {
            *weight *= 1.0 - 0.05 * activation_delta.max(0.0);
            *weight = weight.max(0.0);
        }

        // 3. 有糸分裂判定（ATPチェックは呼び出し元が先に実施する）
        const MITOSIS_THRESHOLD: f64 = 0.8;
        const MIN_NODES_FOR_MITOSIS: usize = 4;

        if self.local_free_energy > MITOSIS_THRESHOLD
            && self.concept_nodes.len() >= MIN_NODES_FOR_MITOSIS
        {
            let mut sorted_nodes = self.concept_nodes.clone();
            sorted_nodes.sort_by(|a, b| {
                b.activation.partial_cmp(&a.activation)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let mid = sorted_nodes.len() / 2;
            let child_nodes = sorted_nodes.split_off(mid);
            self.concept_nodes = sorted_nodes;
            self.local_free_energy *= 0.5;

            let child = ClusterNode {
                cluster_id: format!("{}_child_{}", self.cluster_id, replay_event.timestamp),
                concept_nodes: child_nodes,
                local_free_energy: self.local_free_energy,
                sensory_blanket_weights: self.sensory_blanket_weights.clone(),
                active_blanket_weights: self.active_blanket_weights.clone(),
            };
            return Some(child);
        }

        None
    }
}
```

### 3.3 Cortex調停器への側抑制実装（`cortex/mod.rs`）

```rust
/// 睡眠期：全クラスターの活性化値を比較し、側抑制を適用する。
pub fn apply_lateral_inhibition(clusters: &mut Vec<ClusterNode>) {
    if clusters.is_empty() { return; }

    let max_fep = clusters.iter()
        .map(|c| c.local_free_energy)
        .fold(f64::NEG_INFINITY, f64::max);

    for cluster in clusters.iter_mut() {
        if cluster.local_free_energy < max_fep * 0.8 {
            for (_, weight) in cluster.active_blanket_weights.iter_mut() {
                *weight *= 0.95;
            }
        }
    }
}
```

### 3.4 成功基準

- 睡眠期を繰り返すごとにクラスター総数が単調増加すること
- クラスター間の`local_free_energy`の分散が時間とともに拡大すること（専門化の証拠）
- 特定入力パターンに対して毎回同じクラスターが優先的に活性化するようになること

---

## 4. 条件2：仮想ATPによる代謝制約（フェーズ3並行・v2.1新規）

### 4.1 なぜ必要か

条件1の有糸分裂は放置すると資源を際限なく消費する。`MAX_CLUSTERS`のような人為的なリミッターを直書きすることは自己組織化の原則に反する。代わりに「代謝エネルギーの枯渇」という生命的な自律ブレーキを導入する。

重要な設計判断として、ATPの総量は人間が恣意的に設定する定数ではなく、**コンテナのメモリ残余から動的に逆算する**。これにより`MITOSIS_COST`という恣意的なパラメータを人間がチューニングしなくても、コンテナの物理制約が自動的にATP量を決定する。

### 4.2 変更対象ファイル

`ferro-core/src/cortex/dynamic_cluster.rs`（`ClusterNode`型の拡張）
`ferro-core/src/cerebrum.rs`（睡眠期ATP配給ロジック）
`ferro-core/src/cortex/mod.rs`（有糸分裂呼び出しへのATPチェック統合）

### 4.3 `ClusterNode`への追加フィールド

```rust
pub struct ClusterNode {
    // --- 既存フィールド（変更なし）---
    pub cluster_id: String,
    pub concept_nodes: Vec<ConceptNode>,
    pub local_free_energy: f64,
    pub sensory_blanket_weights: Vec<(String, f64)>,
    pub active_blanket_weights: Vec<(String, f64)>,

    // --- 追加：仮想ATP（代謝エネルギー）---
    /// 現在の仮想ATP残量。睡眠期開始時に大脳から配給される。
    /// 有糸分裂時に消費し、ゼロになると分裂不能・側抑制による剪定対象となる。
    pub virtual_atp: f64,
    /// 剪定対象フラグ。ATPが枯渇した際にtrueになり、調停器が物理消去する。
    pub is_dead: bool,
}
```

### 4.4 ATP配給ロジック（`cerebrum.rs`）

```rust
impl Cerebrum {
    /// 睡眠期開始時に全クラスターへ仮想ATPを一律配給する。
    /// ATP総量はコンテナのメモリ残余から動的に逆算する。
    /// 残余が少ないほど配給量を絞ることで、物理制約が自律ブレーキとして機能する。
    pub fn allocate_atp_to_clusters(
        clusters: &mut Vec<ClusterNode>,
        used_memory_bytes: u64,
        limit_memory_bytes: u64,
    ) {
        // メモリヘッドルーム：0.0（限界）〜1.0（余裕十分）
        let headroom = 1.0
            - (used_memory_bytes as f64 / limit_memory_bytes as f64);

        // ヘッドルームに応じてATP総量を決定
        // headroom=1.0のとき ATP_MAX_PER_CYCLE、headroom=0.0のとき0
        const ATP_MAX_PER_CYCLE: f64 = 100.0;
        let atp_per_cluster = headroom * ATP_MAX_PER_CYCLE;

        for cluster in clusters.iter_mut() {
            cluster.virtual_atp = atp_per_cluster;
            cluster.is_dead = false; // 新サイクルでフラグリセット
        }
    }
}
```

### 4.5 有糸分裂へのATPチェック統合（`cortex/mod.rs`）

```rust
/// 睡眠期：リプレイ事象を全クラスターに適用し、有糸分裂を実行する。
/// ATPチェックはここで一元管理する。
pub fn run_sleep_consolidation(
    clusters: &mut Vec<ClusterNode>,
    replay_events: &[EpisodicSlot],
) {
    let mut new_children: Vec<ClusterNode> = Vec::new();

    for cluster in clusters.iter_mut() {
        for event in replay_events {
            // ATPが不足している場合は有糸分裂をスキップ
            // （局所FEP更新と側抑制準備は引き続き実行される）
            let can_divide = cluster.virtual_atp > MITOSIS_COST;

            if let Some(child) = cluster.execute_local_active_inference(event) {
                if can_divide {
                    cluster.virtual_atp -= MITOSIS_COST;
                    new_children.push(child);
                }
                // ATPが足りない場合は分裂せずFEP更新のみ
            }

            // ATP枯渇チェック
            if cluster.virtual_atp <= 0.0 {
                cluster.is_dead = true;
            }
        }
    }

    // 死亡フラグが立ったクラスターを剪定
    clusters.retain(|c| !c.is_dead);

    // 新しい子クラスターを追加
    clusters.extend(new_children);

    // 側抑制を適用
    apply_lateral_inhibition(clusters);
}

const MITOSIS_COST: f64 = 30.0; // ATP_MAX_PER_CYCLE(100)の30%を消費
```

### 4.6 成功基準

- コンテナのメモリ使用率が80%を超えた際に、有糸分裂の頻度が自律的に低下すること
- `MAX_CLUSTERS`のような定数を一切使用せずにクラスター数が自然な上限に収束すること
- ATP枯渇による剪定が`panic_dump.json`ではなく正常なライフサイクルとして処理されること

---

## 5. 条件3：多角適合度の積による選択圧（フェーズ4並行・v2.0から拡張）

### 5.1 なぜ「積」か

v2.0のジニ係数単体評価には致命的な穴があった。コアが全ゾーンにゴミコードをばら撒くことでジニ係数を偽装できる。これはグッドハートの法則の典型的な失敗例だ。

積にすることで、どれか一つがゼロなら全体がゼロになる。コアがジニ係数を偽装しても、`FEP_Reduction_Trend`がゼロであれば`Fitness`はゼロになり即時Rejectされる。コアが生き残るためには「まだ見ぬ領域を開拓し、かつそれが真に世界の予測に貢献する」という本質的なルートを掘り当てる以外に道がなくなる。

$$\text{Fitness} = S_{\text{static}} \times S_{\text{homeostasis}} \times S_{\text{epistemic}} \times S_{\text{FEP\_trend}}$$

### 5.2 各スコアの定義

| スコア | 値域 | 計算方法 | フェーズ |
|--------|------|---------|---------|
| $S_{\text{static}}$ | {0.0, 1.0} | `cargo check/clippy`の通過で1.0、失敗で0.0 | フェーズ1〜 |
| $S_{\text{homeostasis}}$ | {0.0, 1.0} | OOM/seccomp違反なしで1.0、発生で0.0 | フェーズ1〜 |
| $S_{\text{epistemic}}$ | [0.0, 1.0] | $1.0 - \text{Gini}$（ジニ係数） | フェーズ4〜 |
| $S_{\text{FEP\_trend}}$ | [0.0, 1.0] | FEP減少率トレンドスコア（後述） | **フェーズ4完了後に有効化** |

`FEP_Reduction_Trend`はsandboxコンテナで実際に稼働させて計測しなければ算出できない。フェーズ4完了前はこの項を`1.0`として扱い、フェーズ4完了後に有効化する段階的実装を採用する。

### 5.3 変更対象ファイル

`ferro-shell/src/agents/verifier.rs`

### 5.4 実装

```rust
impl VerifierAgent {

    /// ジニ係数による探索的適合度スコアを計算する。
    /// 返り値：多様性スコア（1.0-Gini）。閾値を超えた場合はErrで即時Reject。
    pub fn compute_epistemic_score(
        &self,
        recent_tickets: &[PatchTicket],
        zone_count: usize,
    ) -> Result<f64, String> {
        if recent_tickets.is_empty() || zone_count == 0 {
            return Ok(1.0);
        }

        let mut zone_freq: HashMap<String, usize> = HashMap::new();
        for ticket in recent_tickets {
            *zone_freq.entry(ticket.zone_marker_id.clone()).or_insert(0) += 1;
        }

        let mut freqs: Vec<f64> = (0..zone_count)
            .map(|i| *zone_freq.get(&format!("zone_{}", i)).unwrap_or(&0) as f64)
            .collect();
        freqs.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let n = freqs.len() as f64;
        let sum: f64 = freqs.iter().sum();
        if sum == 0.0 { return Ok(1.0); }

        let gini: f64 = freqs.iter().enumerate()
            .map(|(i, &f)| (2.0 * (i as f64 + 1.0) - n - 1.0) * f)
            .sum::<f64>() / (n * sum);

        const STAGNATION_THRESHOLD: f64 = 0.7;
        if gini > STAGNATION_THRESHOLD {
            return Err(format!(
                "EpistemicStagnation: Gini={:.3} exceeds {:.3}. Reject.",
                gini, STAGNATION_THRESHOLD
            ));
        }

        Ok(1.0 - gini)
    }

    /// FEP減少率トレンドスコアを計算する。
    /// sandboxコンテナでの実稼働計測値を用いる。
    /// フェーズ4完了前はphase4_enabled=falseで呼び出し、1.0を返す。
    pub fn compute_fep_trend_score(
        &self,
        surprise_history_before: &[f64],
        surprise_history_after: &[f64],
        phase4_enabled: bool,
    ) -> f64 {
        if !phase4_enabled {
            return 1.0; // フェーズ4完了前はバイパス
        }
        if surprise_history_before.is_empty() || surprise_history_after.is_empty() {
            return 0.5;
        }

        let mean_before: f64 = surprise_history_before.iter().sum::<f64>()
            / surprise_history_before.len() as f64;
        let mean_after: f64 = surprise_history_after.iter().sum::<f64>()
            / surprise_history_after.len() as f64;

        // FEPが下がった（改善）→スコア高、上がった（悪化）→スコア低
        // シグモイドで[0,1]に正規化
        let delta = mean_before - mean_after; // 正なら改善
        1.0 / (1.0 + (-delta * 10.0).exp())
    }

    /// 多角適合度の積を計算し、総合判定を行う。
    /// いずれかのスコアがゼロなら全体がゼロ（即時Reject）。
    pub fn evaluate_total_fitness(
        &self,
        static_score: f64,
        homeostasis_score: f64,
        epistemic_score: f64,
        fep_trend_score: f64,
    ) -> Result<f64, String> {
        let fitness = static_score
            * homeostasis_score
            * epistemic_score
            * fep_trend_score;

        if fitness <= 0.0 {
            return Err(format!(
                "Fitness=0: static={:.2} homeostasis={:.2} epistemic={:.2} fep_trend={:.2}",
                static_score, homeostasis_score, epistemic_score, fep_trend_score
            ));
        }

        Ok(fitness)
    }

    /// compile_and_test_reportの総合判定に組み込む。
    pub fn compile_and_test_report(
        &self,
        container_exit_code: i32,
        cargo_build_output: &str,
        cargo_test_output: &str,
        brainstem_metrics_csv: &str,
        episodic_buffer_csv: &str,
        recent_tickets: &[PatchTicket],
        zone_count: usize,
        surprise_before: &[f64],
        surprise_after: &[f64],
        phase4_enabled: bool,
    ) -> Result<Value, String> {
        // 既存の静的・ホメオスタシス評価
        let static_score = self.evaluate_static_fitness(cargo_build_output, cargo_test_output)?;
        let homeostasis_score = self.evaluate_homeostasis_fitness(
            container_exit_code, brainstem_metrics_csv
        )?;

        // 探索的適合度（ジニ係数）
        let epistemic_score = self.compute_epistemic_score(recent_tickets, zone_count)?;

        // FEP減少率トレンド
        let fep_trend_score = self.compute_fep_trend_score(
            surprise_before, surprise_after, phase4_enabled
        );

        // 積による総合判定
        self.evaluate_total_fitness(
            static_score, homeostasis_score, epistemic_score, fep_trend_score
        )?;

        Ok(serde_json::json!({
            "static_score": static_score,
            "homeostasis_score": homeostasis_score,
            "epistemic_score": epistemic_score,
            "fep_trend_score": fep_trend_score,
        }))
    }
}
```

### 5.5 成功基準

- 全ゾーンへのゴミコード撒き散らしが`FEP_Reduction_Trend=0`によってRejectされること（フェーズ4以降）
- ジニ係数が高いパッチが`epistemic_score=0`によって単独でRejectされること
- Fitnessログが`static/homeostasis/epistemic/fep_trend`の四軸で記録され、どの軸が原因でRejectされたか追跡可能であること

---

## 6. 条件4：睡眠期バッチ同期ZPD制御（フェーズ2〜・v2.0から変更）

### 6.1 なぜリアルタイム制御から変更するか

v2.0のZPD比例フィードバックはリアルタイムで複雑度を更新していた。しかし睡眠期に大規模な変異（有糸分裂の連鎖）が起きると、コアの目覚めた直後の認知構造は睡眠前と大きく異なる。そのままリアルタイムフィードバックを再開すると、制御遅延（デッドタイム）による刺激の過剰供給がパニックを引き起こす恐れがある。

解決策は、コアが目覚める直前に環境層の複雑度を安全なベースラインにリセットしてからスタートし、その後リアルタイムフィードバックを再開することだ。

### 6.2 変更対象ファイル

`ferro-env`（ZPD制御ロジック全体）
`ferro-shell/src/agents/supervisor.rs`（睡眠期終了時の通知）

### 6.3 変異エントロピーの計算（`supervisor.rs`）

```rust
impl SupervisorAgent {
    /// 睡眠期に生成されたPatchTicketの集合から変異エントロピーを計算する。
    /// エントロピーが高い（広範囲の変異）→複雑度を大きく下げてスタート。
    /// エントロピーが低い（局所的な変異）→複雑度をほぼ維持してスタート。
    pub fn compute_mutation_entropy(tickets: &[PatchTicket]) -> f64 {
        if tickets.is_empty() { return 0.0; }

        let mut zone_counts: HashMap<&str, usize> = HashMap::new();
        for ticket in tickets {
            *zone_counts.entry(ticket.zone_marker_id.as_str()).or_insert(0) += 1;
        }

        let total = tickets.len() as f64;
        -zone_counts.values()
            .map(|&n| {
                let p = n as f64 / total;
                p * p.ln()
            })
            .sum::<f64>()
        // 値域：0.0（全変異が同一ゾーン）〜 ln(zone_count)（完全均等分布）
    }

    /// 睡眠期終了時にferro-envへ送るリセットパルスを計算する。
    /// 変異エントロピーに応じて翌朝の初期複雑度を決定する。
    pub fn compute_reset_complexity(
        mutation_entropy: f64,
        max_entropy: f64,
    ) -> f64 {
        // エントロピーが最大のとき複雑度をベースライン(0.2)まで下げる
        // エントロピーがゼロのとき複雑度をほぼ維持(0.8)
        let normalized = (mutation_entropy / max_entropy.max(1e-9)).clamp(0.0, 1.0);
        0.8 - 0.6 * normalized
        // normalized=0.0 → 0.8（維持）
        // normalized=1.0 → 0.2（大きく下げる）
    }
}
```

### 6.4 ZPD制御の全体フロー

```rust
pub struct FerroEnv {
    pub complexity: f64,   // 現在の刺激複雑度（0.0〜1.0）
    pub s_target: f64,     // 目標驚愕度（推奨初期値：0.4）
    pub eta: f64,          // 応答慣性係数（推奨初期値：0.05）
}

impl FerroEnv {
    /// 【Awake時】リアルタイムフィードバック制御。
    /// surprise_history.csvの直近平均S̄を読み込み、複雑度を更新する。
    pub fn update_complexity_realtime(&mut self, mean_surprise: f64) {
        let delta = self.eta * (self.s_target - mean_surprise);
        self.complexity = (self.complexity + delta).clamp(0.1, 1.0);
    }

    /// 【Sleep終了直前】外殻からのリセットパルスを受け取り、
    /// 翌朝の初期複雑度をベースラインに設定する。
    /// コアが目覚める前に必ず呼び出すこと。
    pub fn apply_reset_pulse(&mut self, reset_complexity: f64) {
        self.complexity = reset_complexity;
    }

    /// 現在の複雑度に応じて次の滴下刺激のパラメータを決定する。
    pub fn generate_next_stimulus(&self) -> StimulusPacket {
        unimplemented!()
    }
}
```

**睡眠〜覚醒の制御シーケンス：**

```
1. コアがSleep遷移
   └→ ferro-envのリアルタイムフィードバックを停止

2. 睡眠期：有糸分裂・変異ループが実行される

3. 睡眠期終了時：
   └→ SupervisorがPatchTicketから変異エントロピーを計算
   └→ compute_reset_complexityで翌朝の初期複雑度を決定
   └→ ferro-envにapply_reset_pulseを送信

4. コアがAwake遷移
   └→ 初期複雑度から滴下を再開
   └→ リアルタイムフィードバックを再開
```

### 6.5 成功基準

- 大規模変異（変異エントロピー高）の翌朝、複雑度が0.2前後から再開されること
- Awake直後に$\bar{S}$が`s_target`を大きく超えるパニック状態が発生しないこと
- `surprise_history.csv`の覚醒直後の驚愕度スパイクが、バッチ同期導入前と比較して有意に低減すること

---

## 7. 実装スケジュールと担当チーム

| タスク | 担当 | タイミング | 依存 |
|--------|------|-----------|------|
| 1-1: `execute_local_active_inference`実装 | Coreチーム | フェーズ3開始時 | なし |
| 1-2: `apply_lateral_inhibition`実装 | Coreチーム | フェーズ3開始時 | 1-1 |
| 2-1: `ClusterNode`に`virtual_atp`/`is_dead`追加 | Coreチーム | フェーズ3開始時 | なし |
| 2-2: `allocate_atp_to_clusters`実装 | Coreチーム | フェーズ3中盤 | 2-1 |
| 2-3: `run_sleep_consolidation`へのATPチェック統合 | Coreチーム | フェーズ3中盤 | 1-1, 2-2 |
| 4-1: `ferro-env`モダリティランダム化実装 | Envチーム | フェーズ1完了後 | なし |
| 4-2: `apply_reset_pulse`インターフェース実装 | Envチーム | フェーズ2開始時 | 4-1 |
| 4-3: `update_complexity_realtime`実装 | Envチーム | フェーズ2開始時 | 4-2 |
| 3-1: `compute_epistemic_score`実装 | Shellチーム | フェーズ4開始時 | 1-1 |
| 3-2: `compute_fep_trend_score`実装（フェーズ4完了後に有効化） | Shellチーム | フェーズ4開始時 | 3-1 |
| 3-3: `evaluate_total_fitness`（積）実装 | Shellチーム | フェーズ4開始時 | 3-1, 3-2 |
| 4-4: `compute_mutation_entropy`・`compute_reset_complexity`実装 | Shellチーム | フェーズ4開始時 | 4-2 |

---

## 8. 各エージェントへの指示サマリー

### Coreチームへ

フェーズ3開始時に以下を同時着手せよ。

`execute_local_active_inference`の実装（条件1）と`ClusterNode`への`virtual_atp`/`is_dead`フィールド追加（条件2）は独立しているので並行作業可能だ。

`run_sleep_consolidation`はこの二つが揃ってから実装する。ATPチェックをここで一元管理することで、`execute_local_active_inference`自体はATPを知らなくてよい設計にすること。

`MITOSIS_THRESHOLD`（0.8）と`MITOSIS_COST`（30.0）は定数として外部設定ファイルから読み込めるようにしておくこと。自己組織化のダイナミクスを観察しながら調整が必要になる。ただしこれらの変更は人間がferro-shellの設定として行うものであり、コアが自律的に変更してはならない。

### Shellチームへ

フェーズ4開始時に`verifier.rs`の`compile_and_test_report`を本計画書の実装で全面更新せよ。

重要な実装上の注意：`compute_fep_trend_score`は`phase4_enabled=false`で呼び出している間は常に`1.0`を返す。フェーズ4完了を確認してから`true`に切り替えること。この切り替えはコードの変更ではなく設定値の変更として実装すること。

また`compute_mutation_entropy`と`compute_reset_complexity`を実装し、Sleep終了イベントのハンドラから`ferro-env`の`apply_reset_pulse`を呼び出すシーケンスをferro-shellのメインライフサイクルに組み込め。

### Envチームへ

最優先はモダリティのランダム混在実装だ。視覚・聴覚・ログの三モダリティを時間的にランダムに混在させてコアに供給すること。均一な入力では有糸分裂のトリガーとなる高Surpriseが発生しない。

`apply_reset_pulse`インターフェースをferro-shellからの外部呼び出しとして実装し、受け取った複雑度をそのまま`self.complexity`に設定するだけにすること。Sleep〜Awakeの制御シーケンスはferro-shellが主導する。ferro-envはその指示に従うだけでよい。

---

## 9. 観察すべき創発の指標

本計画が正しく機能した場合、以下の変化が観測されるはずだ。

**フェーズ3完了後（短期）：**
クラスター総数が睡眠期を重ねるごとに増加するが、コンテナのメモリ使用率が上昇するにつれてATP配給量が自動的に絞られ、有糸分裂の頻度が低下する。クラスター数が自然な上限に収束する。これが「頭蓋骨制約」の創発だ。

**フェーズ4完了後（中期）：**
PatchTicketのゾーン分布のジニ係数が選択圧によって自然に低下する。ゴミコードを撒いてジニ係数を偽装しようとしたパッチが`FEP_Reduction_Trend`でRejectされる事例が観測されるはずだ。これは系が「外殻の目を盗もうとした」証拠であり、同時に積の評価が正しく機能している証拠でもある。

**長期：**
設計者が予期していなかったクラスター構造が出現するかどうか。これが自己組織化の最終的な問いだ。起きるかどうかはやってみるまでわからない。

---

## 10. 設計上の誠実さについて（v2.0から継承）

本計画書は「好奇心を実装する」とは書いていない。「好奇心が創発する条件を整える」と書いている。この二つは根本的に異なる。

条件2（仮想ATP）はコアの型定義を変更する。これはv2.0で否定したフェーズAと同じ種類の変更だ。なぜv2.1で受け入れるのか。

区別は「何を決めるか」にある。フェーズAの予測分散は「どこに好奇心を向けるか」を設計者が決めていた。仮想ATPは「どれだけのリソースを使えるか」という物理限界を定義するだけだ。前者は認知の内容に介入する。後者は物理的な境界条件を定義するに過ぎない。頭蓋骨は脳が何を考えるかを決めない。

この区別が正しいかどうかは、長期的な観察によってしか検証できない。

*本計画書は`dnb_plan.md`の付属文書として`doc/`ディレクトリに配置すること。*

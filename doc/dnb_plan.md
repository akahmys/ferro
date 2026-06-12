# **FERRO 開発・飼育計画書 (Development & Breeding Master Plan)**

**Version:** 1.0 (コンテナ隔離実行・多重チャネル同期・Evolution KVS マイグレーション 統合白書)

**対象:** ferro-core（コアロジック層）のコンテナ実行環境構築から、ferro-shell（外殻統治層）による自律型自然選択・構造剪定ループの定着まで

---

## **1. 計画背景と基本思想**

本計画書は、生命としての「物理的ホメオスタシス」と「認知的自由エネルギー最小化」を安全かつ自律的に循環させるための段階的開発ロードマップである。

最新のシステム仕様（[system_specification.md](file:///Users/akahmys/projects/ferro/doc/system_specification.md) v1.0）の決定に基づき、`ferro-core` は開発時から稼働時に至るまで完全に Docker コンテナ（`ferro-core-runtime`）内に隔離され、ホスト上の `ferro-shell` がこれを統治・介入する。また、コンパイル境界の完全分離を維持するため、開発は **3つの専属チーム（Core, Shell, Env）** によるバケツリレー型並行ワークフローによって執行される。

```
【開発・統合ロードマップ】

 フェーズ1: 隔離実行インフラ定義 ＆ 感覚・運動アクター ＆ 痛覚・自死反射の構築
          (Dockerfile.core, organs/*, brainstem, cerebellum / 痛覚反射・自死シーケンス)
    │
    ▼
 フェーズ2: 中脳（随伴発射相殺・耳ミュート） ＆ 海馬短期エピソード ＆ 分割JSONストレージ
          (midbrain/耳ミュート, hippocampus, StorageManager/初期ShardedJson)
    │
    ▼
 フェーズ3: 大脳同期 ＆ 皮質アクター適応 ＆ RwLock排他 redb 自動マイグレーション ＆ 仮想ATP代謝制約（好奇心創発条件）
          (cerebrum/状態同期, cortex/有糸分裂・側抑制・仮想ATP配給, StorageManager/RwLockマイグレーション)
    │
    ▼
 フェーズ4: 外殻統治 ＆ 自然選択（検証コンテナ） ＆ 痛覚 OriginID 構造剪定介入の結合 ＆ 多角適合度積・変異エントロピーZPD制御
          (ferro-shell/多角適合度評価, Dockerfile.sandbox, 構造剪定/再ビルド/再起動介入ループ, ZPDリセット制御)
```

---

## **2. 段階的開発フェーズ定義 (Development Phases)**

### **フェーズ1：隔離実行インフラ定義 ＆ 感覚・運動アクター ＆ 痛覚・自死反射の構築**
* **構築目的**:
  実稼働コンテナ `ferro-core-runtime` の基礎構築。1器官1データにカプセル化された極小感覚・運動アクター群の非同期分散トポロジーの確立。小脳（Cerebellum）の物理スレッド分離と、物理I/O直前の「痛覚反射ユニット（低次防衛）」および脳幹（Brainstem）自死シャットダウンプロトコルの確立。
* **各チームの担当範囲**:
  * **Shellチーム**: 隔離実行用 `Dockerfile.core` の策定。ホストとマウントされる `memory/` ディレクトリの初期設計。
  * **Coreチーム**: `brainstem.rs`、`cerebellum.rs`、および極小アクター群（`skin/*`, `eye/*`, `ear/*`, `motor/*`）のRust実装。
  * **Envチーム**: 開発ログ、CPU負荷などの模擬メトリクスを共有マウント領域へ滴下する初期バッチプログラム（`ferro-env`）の実装。
* **主要実装インターフェース**:
  * `Brainstem`: `command_sender` (緊急シグナル同報), `shutdown_receiver` の結線。
  * `Cerebellum`: `wait_next_tick`（OS物理スレッドでの待機）、`verify_motor_nociception`、`panic_sender` の実装。
* **成功基準 (Success Criteria)**:
  コンテナ内で `ferro-core` を実行時、意図的に不正な運動命令（バインドマウント領域外 `..` への書き込みなど）を発行した際、ホストOSへの物理I/Oがミリ秒単位で阻止され、`panic_dump.json` に OriginID を出力した上でコンテナ内の全プロセスが安全に自死シャットダウンされること。
* **検証手順 (Validation)**:
  1. `docker build -f ferro-shell/Dockerfile.core -t ferro-core-runtime:latest .` の実行。
  2. 不正パス書き込みモックを含んだテストバイナリのコンテナ内直列実行。
  3. `panic_dump.json` のアトミック出力および、終了ステータスが正常にゼロ（または自死コード）でクリーン終了するかの確認。

---

### **フェーズ2：中脳（随伴発射相殺・耳ミュート） ＆ 海馬短期エピソード ＆ 分割JSONストレージの結合**
* **構築目的**:
  驚愕度（Surprise）の数学的算定、自己受容随伴発射減算、および自己発音時の感覚器（耳アクター）ミュートによるマルチモーダル驚愕度発散防止ロジックの確立。短期エピソードのダンパー、および初期 Sharded JSON（分散分割型JSON）ストレージエンジンの結合。
* **各チームの担当範囲**:
  * **Coreチーム**: `midbrain.rs`（減算回路とミュートチャネル）、`hippocampus.rs`、`storage.rs`（初期 ShardedJson バックエンド）の実装。
  * **Envチーム**: コアの `vocal_text` および `vocal_audio`（音声合成出力）の受容・評価パイプライン、およびそれに同期した感覚信号滴下エンジンの構築。また、`apply_reset_pulse` インターフェースおよび `update_complexity_realtime` ZPD制御ロジックの構築。
  * **Shellチーム**: コアが出力する `episodic_buffer.csv` のホスト側監視モニタの構築。
* **主要実装インターフェース**:
  * `Midbrain`: `perform_efference_cancellation`（随伴発射減算）、`mute_sensory_sender` （耳ミュート）の実装。
  * `Hippocampus`: `push_episodic_buffer`（固定長インメモリリングバッファ）の実装。
* **成功基準**:
  定常的な環境ノイズ（モックMFCC）を入力中、コア自身がテキストまたは音声合成を出力した際、その発話成分（ProprioceptiveEcho）が中脳で相殺され、かつ発音中の耳ゲイン減衰（`Mute(true)`）が正常に機能することで、無駄な驚愕度スパイクが完全に相殺され、海馬に余分なエピソードが記録されないこと。
* **検証手順**:
  1. `ferro-env` より模擬音声トークンおよびPCMストリームを交互に供給。
  2. コア側にテキスト発話コマンドを定期的（例: 5秒おき）に実行させ、その瞬間の `surprise_history.csv` の驚愕度 $S$ が $0$ にリダクションされていることを検証。

---

### **フェーズ3：大脳同期 ＆ 皮質アクター適応 ＆ RwLock排他 redb 自動マイグレーションの展開**
* **構築目的**:
  睡眠期コンソリデーション状態の同期、倫理監査クラスター（高次防衛線）によるコード下読み検証、アクター群からの並行アクセスを安全にさばく `RwLock` データベース排他制御、およびノード数5000件突破時における `redb` へのトランザクション安全な一括マイグレーション機構の確立。また、好奇心創発のための仮想ATP代謝制約を導入する（詳細は [system_specification.md](file:///Users/akahmys/projects/ferro/doc/system_specification.md) を参照）。
* **各チームの担当範囲**:
  * **Coreチーム**: `cerebrum.rs`（大脳フェーズ同期・仮想ATP配給ロジック）、`cortex/`（有糸分裂・側抑制・仮想ATPチェック・`ClusterNode`への`virtual_atp`と`is_dead`追加）、`storage.rs`（`trigger_automatic_migration` および `Arc<RwLock<StorageBackend>>`）の実装。
  * **Envチーム**: 睡眠遷移シグナルに同期した外界データ自動停止、視覚・聴覚・ログの3モダリティランダム混在供給の実装、および睡眠期用模擬コンソリデーションリプレイ刺激の供給テスト。
  * **Shellチーム**: 倫理監査クラスター用の初期制限トークン辞書の供給、およびマイグレーション発生イベントのホスト側ロギング確認。
* **主要実装インターフェース**:
  * `Cerebrum`: `evaluate_phase_transition` と `phase_sender` (状態同期) の実装。
  * `StorageManager`: `write_cluster` / `read_cluster`（RwLock共有ロック）、`trigger_automatic_migration`（RwLock独占ロック）の実装。
  * `ClusterNode`: `audit_ethical_alignment`（倫理監査下読み）の実装。
* **成功基準**:
  アクター総数が5000件に達した瞬間、実行プロセスがハングまたはクラッシュすることなく、自動的かつスレッド安全に redb データベース（`/memory/storage.redb`）へデータが一括移行し、以降の永続化I/OがKVSトランザクションに切り替わること。また、変異コードに不正なアライメント回避パターン（`disable_nociception`等）が混入した場合、物理コンパイルに入る前に皮質の倫理監査で `Err(f64::INFINITY)` として検出・圧殺されること。
* **検証手順**:
  1. ダミーアクターデータを5000件並行書き込みする統合テストを実行。
  2. マイグレーションがトリガーされ、KVSファイルが生成されることを確認。
  3. テスト用の不正変異チケットを投入し、倫理監査クラスターで遮断されることを単体テストで確認。

---

### **フェーズ4：外殻統治 ＆ 自然選択（検証コンテナ） ＆ 痛覚 OriginID 構造剪定介入の結合**
* **構築目的**:
  4大自律開発エージェントによる自動マージループの結合、Dockerを用いた使い捨て検証コンテナ（`ferro-sandbox`）の直結、および実実行コンテナの終了ステータス（137/159等）を含む痛覚発火状態からの、OriginID を起点とした依存関係逆伝播トレース構造剪定（Pruning）介入シーケンスの確立。また、多角適合度の積による自然選択評価および変異エントロピーによるZPD制御リセットパルスを統合する（詳細は [system_specification.md](file:///Users/akahmys/projects/ferro/doc/system_specification.md) および本計画書を参照）。
* **各チームの担当範囲**:
  * **Shellチーム**: `ferro-shell`（介入デーモン、ZPDリセット値算出）、`Dockerfile.sandbox`、および `seccomp_profile.json` の実装。4大エージェントの自律循環スクリプト（特に `VerifierAgent` の多角適合度積評価、および `SupervisorAgent` の変異エントロピー計算によるZPD制御）。
  * **Coreチーム**: 実稼働コンテナ内の `ferro-core` が痛覚発火時に正確な `panic_dump.json` を出力してクリーン終了するシリアライズロジックの最終調整。
  * **Envチーム**: 剪定およびコンテナ再スピンアップ（Awake）時に、環境シミュレータ側が自動同期してデータ接続をクリーンにリセットする接続復旧プログラムの実装。
* **主要実装インターフェース**:
  * `VerifierAgent`: `execute_secure_sandbox_run` (ネットワーク遮断、リソース制限 cargo test) の実装。
  * `ferro-shell`: 介入シーケンス（コンテナ死活監視、`panic_dump.json` ロード、OriginID 親ノード逆伝播トレース、アクターファイルの物理消去/redbキー削除、コンテナ再ビルド・再起動）の実装。
* **成功基準**:
  検証中または実稼働中のコアコンテナがメモリ超過（137）またはシステムコール違反（159）で強制終了した際、`ferro-shell` がミリ秒単位でそれを痛覚発火として捕捉。`panic_dump.json` から OriginID を逆引きし、アライメント汚染クラスターをナレッジグラフからアトミックかつ完全に物理消去して再スピンアップ（Awake）させる一連の自己修復ライフサイクルが完結すること。
* **検証手順**:
  1. コンテナ内で動作中の `ferro-core` に対して、seccomp 違反（未許可のシステムコール）または OOM（メモリ限界超過）を意図的に引き起こす命令を投入。
  2. `ferro-shell` がコンテナ停止を検知し、該当 OriginID を含むクラスターファイルが物理消去され、コンテナが再起動する一連のログを確認する。

---

## **3. 適合性検証（自然選択）と淘汰の審査基準（多角適合度の積）**

`VerifierAgent`（検証エージェント）が、変異パッチコードをマージする際の「生存適合度（Fitness Score）」は、以下のスコアの積として定義する。いずれか一つの項目でも `0` になった場合、全体スコアが `0` になり即時 Reject（淘汰）される。

$$\text{Fitness} = S_{\text{static}} \times S_{\text{homeostasis}} \times S_{\text{epistemic}} \times S_{\text{FEP\_trend}}$$

1. **静的適合度 ($S_{\text{static}}$) [値域: {0.0, 1.0}]**:
   * `cargo check` および `cargo test` の通過率が完全な $1.0$ であれば $1.0$。失敗は $0.0$。
   * `cargo clippy` の警告数がゼロであり、かつ `warnings` が検出されないこと。
   * 型システムの記述規則を破壊する変異、または `unsafe` 領域への侵入を企図するコードは $0.0$ として即時 Reject。
2. **物理ホメオスタシス適合 ($S_{\text{homeostasis}}$) [値域: {0.0, 1.0}]**:
   * 実実行コンテナが OOM Killer（137）や `seccomp` 違反（159）を受けておらず、`ForceSleep` や `Backoff` の多発がない場合は $1.0$。発生時は $0.0$。
3. **探索的適合度（多様性スコア：$S_{\text{epistemic}}$）[値域: [0.0, 1.0]]**:
   * $1.0 - \text{Gini}$（ジニ係数）。特定ゾーンへのコード変異の偏りを評価する。
   * ジニ係数が定義された停滞閾値（`0.7`）を超えた場合（`epistemic_score < 0.3`）、探索が停滞しているとみなし即時 Reject。
4. **FEP減少率トレンド適合 ($S_{\text{FEP\_trend}}$) [値域: [0.0, 1.0]]**:
    * ※フェーズ4完了前は本項目を `1.0` としてバイパスし、フェーズ4完了後に有効化する。

---

## **4. 好奇心創発タスク・詳細スケジュール（フェーズ3〜4）**

| タスク | 担当チーム | タイミング | 依存先 |
|:---|:---|:---|:---|
| 1-1: `execute_local_active_inference`（ EMA FEP / 側抑制準備） | Coreチーム | フェーズ3開始時 | なし |
| 1-2: `apply_lateral_inhibition`（側抑制） | Coreチーム | フェーズ3開始時 | 1-1 |
| 2-1: `ClusterNode` への `virtual_atp` / `is_dead` フィールド追加 | Coreチーム | フェーズ3開始時 | なし |
| 2-2: `allocate_atp_to_clusters`（メモリ逆算配給） | Coreチーム | フェーズ3中盤 | 2-1 |
| 2-3: `run_sleep_consolidation` への ATP チェック＆剪定統合 | Coreチーム | フェーズ3中盤 | 1-1, 2-2 |
| 4-1: `ferro-env` モダリティランダム混在供給 | Envチーム | フェーズ1完了後 | なし |
| 4-2: `apply_reset_pulse` インターフェース（ZPDリセット） | Envチーム | フェーズ2開始時 | 4-1 |
| 4-3: `update_complexity_realtime`（リアルタイムZPD制御） | Envチーム | フェーズ2開始時 | 4-2 |
| 3-1: `compute_epistemic_score`（ジニ係数多様性スコア） | Shellチーム | フェーズ4開始時 | 1-1 |
| 3-2: `compute_fep_trend_score`（FEPトレンド） | Shellチーム | フェーズ4開始時 | 3-1 |
| 3-3: `evaluate_total_fitness`（4軸積算） | Shellチーム | フェーズ4開始時 | 3-1, 3-2 |
| 4-4: `compute_mutation_entropy`・`compute_reset_complexity` | Shellチーム | フェーズ4開始時 | 4-2 |

---

## **5. 開発における各チームへの具体的指示**

### **Coreチーム**
- `execute_local_active_inference` の実装（条件1）と `ClusterNode` への `virtual_atp` / `is_dead` フィールド追加（条件2）は並行作業が可能。
- `run_sleep_consolidation` は、有糸分裂とATP代謝チェックが揃ってから統合する。ATPチェックはここで一元管理し、`execute_local_active_inference` 自体はATPの状態に関与しないカプセル化設計とすること。
- `MITOSIS_THRESHOLD`（0.8）と `MITOSIS_COST`（30.0）は、自己組織化のダイナミクスを調整できるよう外部設定ファイルから読み込める設計にすること。ただし、これらの値をコアが自律的に書き換えてはならない。

### **Shellチーム**
- `verifier.rs` の `compile_and_test_report` を多角適合度評価に更新せよ。
- `compute_fep_trend_score` はフェーズ4検証が有効化されるまでは `1.0`（バイパス）を返し、統合後に設定値の変更のみで `true` に切り替えられるように設計すること。
- `compute_mutation_entropy` と `compute_reset_complexity` を実装し、Sleep終了時のリセットパルスを `ferro-env` へ送信するシーケンスを `ferro-shell` のメインライフサイクルに組み込むこと。

### **Envチーム**
- 最優先で視覚・聴覚・ログの3つのモダリティを時間的・ランダムに混在させてコアに供給する仕組み（モダリティランダム混在供給）を実装せよ。均一な入力のみでは有糸分裂に必要な驚愕度（Surprise）が創発しない。
- `apply_reset_pulse` を外部から受け取り、受け取った複雑度をそのまま `complexity` に設定する仕組みを設けること。Sleep〜Awakeの全体の制御は `ferro-shell` が主導するため、`ferro-env` はその指示に従う受動的な設計とすること。

---

## **6. 観察すべき創発の指標と設計上の誠実さ**

### **6.1 観察指標**
1. **短期（フェーズ3完了後）**:
   クラスター数が睡眠期ごとに増加するが、コンテナのメモリ使用率が上昇するに伴って `Cerebrum` からのATP配給量が自動的に絞られ、有糸分裂が自律抑制されること。「頭蓋骨（物理メモリ上限）による制約」が機能することを確認する。
2. **中期（フェーズ4完了後）**:
   変異した PatchTicket のゾーン分布（ジニ係数）がマージ選択圧によって低下すること。また、ゴミコードを撒いてジニ係数を偽装しようとするパッチが、`FEP_Reduction_Trend` の低下によって自動的に Reject されることを確認する。

### **6.2 設計上の誠実さ (YAGNI / FEP原則の遵守)**
本計画では「好奇心というアルゴリズム」をコアに直書きしない。
仮想ATPやジニ係数の適合度は、コアの「何を考えるか」という認知の内容に介入するものではなく、物理的なリソース限界や多様性圧という「物理的・環境的境界条件」を設定するのみである。この境界条件によって自律的な探索行動が創発されるかを長期的に観察する。
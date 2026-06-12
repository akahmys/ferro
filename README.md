# **FERRO: Hierarchical Active Inference System**

本プロジェクト「**FERRO**」は、自由エネルギー原理（Free Energy Principle: FEP）に基づく多重マルコフブランケット（Markov Blanket）の数理モデルを応用し、隔離されたローカル環境下において自律的な資源制御および能動的推論（Active Inference）による自己コード改変を実行する階層型ソフトウェアアーキテクチャである。

---

## **1. システム概要 & 主要機能**

FERROは、情報理論的境界（マルコフブランケット）によって隔離された3つのレイヤーが非対称・非同期に通信し、自己組織化および自律進化するライフサイクルを構成している。

1. **完全コンテナ隔離実行環境**:
   コアロジック（`ferro-core`）の実稼働および変異パッチの安全検証は、ホストOSからネットワークやシステム特権を完全に遮断した Docker コンテナ（`ferro-core-runtime` / `ferro-sandbox`）の内部で行われる。
2. **極小感覚・運動アクター (1器官1データ)**:
   すべての感覚器（内受容・外受容・自己受容）および運動出力器は、カプセル化された独立スレッドまたは非同期タスクとして並行動作し、メッセージチャネル（MPSC）を介して非同期通信を行う。
3. **脳幹〜皮質にいたる 5+1 階層認知トポロジー**:
   - **脳幹 (Brainstem)**: 内受容シグナルを監視し、過熱や省メモリ時の割り込み待機（Backoff）やプロセスの自死による物理防衛を司る。
   - **小脳 (Cerebellum)**: 100ms等時同期制御、随伴発射の送出、および物理I/O直前の痛覚反射（Nociceptive Reflex）による脱獄・未許可通信のミリ秒単位の検閲。
   - **中脳 (Midbrain)**: 随伴発射減算（Efference Cancellation）による自己発話エコーの相殺、驚愕度（Surprise）算定、AGC（自動ゲイン制御）。
   - **海馬 (Hippocampus)**: 覚醒期の高驚愕事象をバッファリングし、睡眠期に皮質へ低速再生（リプレイ）する。
   - **大脳 (Cerebrum)**: グローバル自由エネルギー管理、忘却演算、覚醒・睡眠の遷移判定、および仮想ATPの配給。
   - **皮質 (Cortex)**: 睡眠期の有糸分裂・側抑制による知識トポロジーの自己組織化、変異コード生成、および実行前の高次倫理監査（Ethical Audit）。
4. **多角適合度の積による自然選択**:
   外殻統治層（`ferro-shell`）が、提案されたコード変異パッチに対して「静的・ホメオスタシス・多様性（ジニ係数）・FEP減少率トレンド」の4軸の積（どれか一つでもゼロなら全体がゼロ）で評価し、自動適合淘汰（自然選択）を執行する。

---

## **2. リポジトリ物理構造 (Workspace Layout)**

リポジトリは独立した Cargo プロジェクトによって構成されており、Cargo 依存は完全に遮断されている。

```
ferro/ (プロジェクトルート)
├── .agent/
│   └── rules/
│       ├── development_rules.md  # 開発ルール・命名規則・FERRO Power of 10
│       └── team_structure.md     # 3つの専属チーム（Core, Shell, Env）の担当境界
│
├── doc/
│   ├── dnb_plan.md               # 開発・飼育計画書 (Master Plan)
│   └── system_specification.md   # システム全体構造＆開発・隔離規約定義書
│
├── ferro-core/                   # 【層A: コアロジック】(Docker コンテナ内で実行)
│   ├── src/                      # 5+1 階層認知ロジックおよび感覚・運動器官アクター
│   └── memory/                   # 永続マウント共有領域（KVSデータベース, ログ, panic_dump等）
│
├── ferro-shell/                  # 【層B: 外殻統治・ハイパーバイザ】(ホスト上で実行)
│   ├── src/                      # 4大自律開発エージェント、コンテナ監視、構造剪定介入
│   ├── Dockerfile.core           # 実稼働コアコンテナ定義
│   ├── Dockerfile.sandbox        # 使い捨て検証コンテナ定義
│   └── seccomp_profile.json      # コンテナ制限用 seccomp カーネルプロファイル
│
├── ferro-env/                    # 【環境層】(ホスト上で実行)
│   └── src/                      # 外界データ、システム負荷模擬メトリクスの滴下供給
│
├── scripts/                      # 流出防止および FERRO 規約違反静的検査スクリプト群
├── setup_hooks.sh                # 開発環境の Git Hook 初期セットアップスクリプト
└── README.md                     # 本ファイル (総合ガイド)
```

---

## **3. 開発環境のセットアップ (Quick Start)**

### **前提条件 (Prerequisites)**
- **Rust**: 最新の Stable ツールチェーン
- **Docker**: コンテナ実行環境が稼働していること
- **Python 3**: Git pre-commit フックの静的解析スクリプト用

### **初期セットアップ (Git Pre-commit Hook)**
本プロジェクトでは、流出防止規約および「FERRO Power of 10」違反をローカルコミット前に自動で静的解析する Git Hook を導入している。開発を開始する前に必ず以下のコマンドを実行してください。

```bash
chmod +x setup_hooks.sh
./setup_hooks.sh
```

これにより、`.git/hooks/pre-commit` が生成され、コミット時に `prevent_leak.py` と `verify_ferro_rules.py` が自動実行される。

### **ビルド & テスト方法**
各レイヤーは個別の Cargo ワークスペースとして構成されているため、個別にビルド・テストを実行する。

* **Core レイヤー**:
  ```bash
  cd ferro-core
  cargo build
  cargo test
  ```
  *(注意: 実際の Verifier 稼働時は、Docker コンテナサンドボックス内で実行される)*

* **Shell レイヤー**:
  ```bash
  cd ferro-shell
  cargo build
  cargo test
  ```

* **Env レイヤー**:
  ```bash
  cd ferro-env
  cargo build
  cargo test
  ```

---

## **4. ドキュメントリファレンス (Documentation Guide)**

より詳細な仕様や設計については、以下の各ドキュメントを参照してください。

* **設計・仕様の詳細**:
  * [doc/system_specification.md](file:///Users/akahmys/projects/ferro/doc/system_specification.md): システムの処理トポロジー、痛覚閾値ロジック、各認知モジュールの Rust 型定義、随伴発射減算シーケンスなどを定義した詳細仕様書。
* **開発ロードマップと繁殖計画**:
  * [doc/dnb_plan.md](file:///Users/akahmys/projects/ferro/doc/dnb_plan.md): フェーズ1〜フェーズ4の段階的なインフラ定義、開発、および自律自然選択ループの定着にいたる全体計画書（※仮想ATPやZPD等の好奇心・探索創発の詳細も含む）。
* **開発及びチームルール**:
  * [.agent/rules/development_rules.md](file:///Users/akahmys/projects/ferro/.agent/rules/development_rules.md): 依存関係遮断ルール、アトミック書き込み同期プロトコル、および「FERRO Power of 10 (安全重要コーディング規約)」の規定。
  * [.agent/rules/team_structure.md](file:///Users/akahmys/projects/ferro/.agent/rules/team_structure.md): Core, Shell, Env チーム間の境界と非対称ファイル通信規約。
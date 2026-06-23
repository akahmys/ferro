# FERRO (Free Energy Regulated Recursive Organism)

FERRO は、自由エネルギー原理（FEP: Free Energy Principle）に基づいて自己組織化および適応を行う階層型能動的推論システムです。

## システムアーキテクチャ

本システムは、情報境界（マルコフブランケット）によって隔離された 4 つのレイヤーで構成され、依存の単方向性（Layer C → Layer A → Layer B）を厳守します。

1. **Layer A: ferro-core (推論エンジン層)**
   - システムの中核。局所自由エネルギーの最小化および状態遷移の計算を担当する純粋計算コンポーネント。
   - 脳幹（Brainstem）、小脳（Cerebellum）、中脳（Midbrain）、海馬（Hippocampus）、大脳（Cerebrum）、皮質（Cortex）の各機能モジュールから構成されます。
2. **Layer B: ferro-body (感覚・運動・身体層)**
   - システムの「身体」であり、感覚（Eye/Ear）の滴下、運動出力（発話）の監視、および教育・介入シグナル（Breeding Engine）の適用を担当。
3. **Layer C: ferro-shell (外殻統治・検証層)**
   - システム管理の最高位。コンテナ隔離（Seccomp/Docker）の運用、生存監視、アライメント監査、および痛覚に紐づく動的構造剪定（Pruning）リカバリーループを担当。
4. **Layer D: ferro-monitor (オブザーバビリティ・監視層)**
   - システム挙動に介入せず、各層から Pub/Sub された監視パケットをリアルタイムに集約し、Ratatui CUI ダッシュボードを通じて適応プロセスおよび内部状態を可視化。

## ディレクトリ構成

`AGENTS.md` で定義される物理ファイルトポロジーに基づき、以下の構造で再構築を進めます。

```text
ferro/                             # ワークスペースルート
│
├── Cargo.toml                     # ワークスペース設定
├── AGENTS.md                      # AIエージェント向けの絶対遵守ルール
├── README.md                      # 本ドキュメント
├── PLANS.md                       # 開発・教育ロードマップ
├── TASKS.md                       # 開発タスク管理
│
├── ferro-core/                    # 【層A: コアロジック（Pure Rust）】
├── ferro-body/                    # 【層B: 感覚・運動・身体層（Pure Rust）】
├── ferro-shell/                   # 【層C: 外殻統治・ハイパーバイザ（Pure Rust）】
├── ferro-monitor/                 # 【層D: 統一オブザーバビリティ・監視層（Pure Rust）】
│
└── memory/                        # 【tmpfs共有境界（メモリマップドRAMディスク）】
```

## 開発規範

本プロジェクトのすべてのコードは、`AGENTS.md` およびインデックスされた個別規約（特に **FERRO Power of 10** コーディング規約）に厳格に準拠する必要があります。
- 1ファイル 100 行以内 / 1関数 60 行以内
- `unwrap()` / `expect()` の原則禁止
- 再帰呼び出しの禁止
- すべての関数に 2 つ以上の `assert!` を設置
- ループには静的な上限またはタイムアウトを設置

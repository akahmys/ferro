# **FERRO AI エージェント最重要憲法 (AGENTS.md)**

本ドキュメントは、FERRO プロジェクトにおいて自律的に動作するすべての AI エージェントおよびサブエージェントが、最優先で遵守すべき「最重要憲法」である。AI エージェントは、本規約から逸脱する設計・実装を絶対に行ってはならない。

---

## **1. 最重要原則 (Core Principles)**

1. **安全性 (Safety) の絶対優先**:
   - 安全性を損なう性能向上や機能拡張は禁止する。アライメント（倫理）監査を迂回するコードを書いてはならない。
2. **決定性 (Determinism) の保証**:
   - 同一入力および同一シード値に対し、常にビット単位で一致する推論・状態遷移結果を出力すること。非決定的な浮動小数点加算（Rayon 等による動的リダクション）は禁止する。
3. **情報境界（マルコフブランケット）の遵守**:
   - 各開発チームの担当ディレクトリ外に対する改変を厳禁とする。
   - 例: `Core チーム` は `ferro-core/` のみ変更可能。`Body チーム` は `ferro-body/` のみ変更可能。
4. **FERRO Power of 10 コーディング規約の厳守**:
   - 関数の極小設計（ロジック行数100行以下）、unwrapの原則禁止、事前・事後アサーション最低2つ、再帰の禁止などを徹底する。

---

## **2. 詳細ルール ＆ 仕様書インデックス (Index to Specifications)**

詳細なルール、モジュール設計、および共通仕様は、以下の個別ドキュメントに分割して定義されている。AI エージェントは必要に応じてこれらを参照すること。

### **2.1. 共通ルール・開発標準**
* [開発・コーディング・命名規約の詳細 (.agents/rules/development_rules.md)](file:///Users/akahmys/projects/ferro/.agents/rules/development_rules.md)
  - 10 のコーディング契約、アサーションの例外規定、OS・コンテナ環境要件、命名規約。
* [実装チーム構成・エージェント間規約 (.agents/rules/team_rules.md)](file:///Users/akahmys/projects/ferro/.agents/rules/team_rules.md)
  - チーム境界とファイル権限、バケツリレーワークフロー、コミュニケーション方法。
* [数理・メモリ・並行性・安全性詳細 (.agents/rules/math_and_safety_rules.md)](file:///Users/akahmys/projects/ferro/.agents/rules/math_and_safety_rules.md)
  - FEP等数理制約（MC-1〜MC-4）、メモリモデル（アリーナ）、並行性（RwLock順序）、倫理監査・アライメント強制。

### **2.2. 各レイヤーの設計仕様書**
* [コアロジック層の設計・アルゴリズム詳細 (ferro-core/DESIGN_CORE.md)](file:///Users/akahmys/projects/ferro/ferro-core/DESIGN_CORE.md)
  - 脳幹・小脳・中脳・海馬・皮質等の各機能モジュール仕様、SPMV・決定論的 Map/Reduce 等の数理アルゴリズム。
* [身体・感覚運動層の設計詳細 (ferro-body/DESIGN_BODY.md)](file:///Users/akahmys/projects/ferro/ferro-body/DESIGN_BODY.md)
  - 環境I/O、感覚滴下（Eye/Ear）、運動ポート、Breeding Engine、可塑性バースト。
* [外殻統治・検証ハイパーバイザの設計詳細 (ferro-shell/DESIGN_SHELL.md)](file:///Users/akahmys/projects/ferro/ferro-shell/DESIGN_SHELL.md)
  - コンテナ管理、Ethical Audit隔離、Verifier Agent、構造剪定（Pruning）アルゴリズム。
* [統一オブザーバビリティ・監視層の設計詳細 (ferro-monitor/DESIGN_MONITOR.md)](file:///Users/akahmys/projects/ferro/ferro-monitor/DESIGN_MONITOR.md)
  - 監視パケット集約、Ratatui CUI ダッシュボード。

### **2.3. 検証 ＆ メッセージ仕様**
* [検証・テスト戦略 (docs/verification_protocol.md)](file:///Users/akahmys/projects/ferro/docs/verification_protocol.md)
  - テスト戦略、ベンチマーク、チェックリスト。
* [共通メッセージスキーマ・エラー定義 (docs/message_schema.md)](file:///Users/akahmys/projects/ferro/docs/message_schema.md)
  - APIデータ構造、監視パケット定義、エラーコード一覧。

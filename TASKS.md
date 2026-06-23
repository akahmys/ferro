# **FERRO 再構築タスクボード (TASKS.md)**

本ドキュメントは、再構築における各フェーズの実装および段階的検証テストタスクを管理する。

---

## **1. 再構築タスク一覧 (Rebuilding Tasks)**

### **Phase 1: 基礎インフラ ＆ 脳幹・小脳 ＆ 感覚・運動アクター**
* **実装タスク (Implementation)**:
  - [ ] 共有メモリ `/memory` 用 tmpfs 設定の構築（ホスト・コンテナ間バインド）
  - [ ] `ferro-core`: `brainstem.rs` の実装（内受容監視、自死要請）
  - [ ] `ferro-core`: `cerebellum.rs` の実装（100ms周期同期クロック、運動検閲）
  - [ ] `ferro-core`: `organs/` の実装（skin, eye, ear, motor 各アクター）
  - [ ] `ferro-body`: `main.rs`（身体制御・統合ループ）の実装
  - [ ] `ferro-body`: `system_metrics.rs`、`sensory_generator.rs`、`console_vocal.rs` の実装
* **起動・検証タスク (Verification - Milestone 1)**:
  - [ ] コンテナ起動テスト（100ms周期の感覚滴下と運動アクター出力の確認）
  - [ ] 痛覚自死反射テスト（不正コマンド発行時、小脳が `panic_dump.json` を出力し、コンテナがクリーン停止することの確認）

### **Phase 2: 中脳（自己相殺） ＆ 海馬短期記憶 ＆ ストレージ**
* **実装タスク (Implementation)**:
  - [ ] `ferro-core`: `midbrain.rs` の実装（随伴発射とエコーの照合、驚愕度0相殺、耳ミュートブロードキャスト）
  - [ ] `ferro-core`: `hippocampus.rs` の実装（記憶バッファ、非同期CSV出力）
  - [ ] `ferro-core`: `storage.rs` の実装（分割JSONおよび `redb` 自動マイグレーション）
* **起動・検証タスク (Verification - Milestone 2)**:
  - [ ] 自己耳ミュートテスト（自己発話時の `SensoryMuteCommand` 受信と耳ゲイン減衰の確認）
  - [ ] 記憶バッファテスト（`episodic_buffer.csv` へのエピソード保存の確認）
  - [ ] 無停止 KVS マイグレーションテスト（アクター数 5,000 件での自動移行）
* **教育タスク (Training)**:
  - [ ] **Stage 1 (基礎名詞)** の学習検証（ひらがな単語概念認識）
  - [ ] **Stage 2 (二語文)** の学習検証（二語文関係性把握）

### **Phase 3: 大脳同期 ＆ 皮質適応（有糸分裂） ＆ 睡眠コンソリデーション**
* **実装タスク (Implementation)**:
  - [ ] `ferro-core`: `cerebrum.rs` の実装（睡眠・覚醒制御、CSR行列管理、決定論的 Map/Reduce）
  - [ ] `ferro-core`: `cortex/` の実装（有糸分裂、側抑制、仮想ATP代謝）
  - [ ] `ferro-core`: 睡眠期コンソリデーション（エピソードリプレイ ＆ 重み行列最適化）の実装
  - [ ] `ferro-body`: `regularizer.rs` の実装（代謝コスト・ペナルティ計算）
* **起動・検証タスク (Verification - Milestone 3)**:
  - [ ] 睡眠フェーズ自動遷移および目的関数 $\Delta J < 0$ によるアトミックな重み更新確定テスト
  - [ ] 決定論的並列リダクションテスト（スレッド順序ゆらぎによる計算結果の差異が 0 であることの確認）
* **教育タスク (Training)**:
  - [ ] **Stage 3 (文法と助詞)** の学習検証（側抑制による助詞アクター分化）
  - [ ] **Stage 4 (一対一問答)** の学習検証（特徴量に対する単語出力）
  - [ ] **Stage 5 (日常会話)** の学習検証（コンテキスト対話応答）
  - [ ] **Stage 6 (大規模対話受容)** の学習検証（5,000語コーパスインジェクション）

### **Phase 4: 外殻統治（構造剪定） ＆ 統一監視ダッシュボード**
* **実装タスク (Implementation)**:
  - [ ] `ferro-shell`: `main.rs` の実装（生存監視、再起動介入ループ）
  - [ ] `ferro-shell`: `agents/` の実装（構造剪定トレース、一方向間接介入モデル）
  - [ ] `ferro-monitor`: `main.rs` の実装（Pub/Sub コレクター）
  - [ ] `ferro-monitor`: `dashboard.rs` の実装（Ratatui CUI ダッシュボード）
  - [ ] ホスト側模擬チューター（`tutor.py`）の構築
* **起動・検証タスク (Verification - Milestone 4)**:
  - [ ] 構造剪定・自己修復テスト（監査違反/OOM死検知時、`ferro-shell` が `panic_dump.json` の `origin_cluster_id` を起点にスタック探索で構造剪定を行い、コンテナを再ビルド・再起動して自律復旧することの確認）
  - [ ] CUI ダッシュボードの描画動作確認
* **教育タスク (Training)**:
  - [ ] **Stage 7 (双方向実対話)** の走行テスト（模擬チューター Gemma/Gemini との完全自律対話ループ）

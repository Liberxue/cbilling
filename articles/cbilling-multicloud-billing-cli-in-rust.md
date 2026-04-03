---
title: "cbilling: Rustで作ったマルチクラウド請求書CLIツール — 7社のクラウドを1つのターミナルで"
emoji: "💰"
type: "tech"
topics: ["rust", "cli", "aws", "gcp", "cloud"]
published: true
---

## TL;DR

**[cbilling](https://github.com/Liberxue/cbilling)** は、AWS / GCP / Alibaba Cloud / Tencent Cloud / Volcengine / UCloud / Cloudflare の7つのクラウドプロバイダーの請求データを、1つのターミナルUIとCLIで一括表示・比較できるRust製ツールです。

[![asciicast](https://asciinema.org/a/Wgsc4BxlnGlc92rl.svg)](https://asciinema.org/a/Wgsc4BxlnGlc92rl)

```bash
# ワンライナーでインストール
curl -fsSL https://raw.githubusercontent.com/Liberxue/cbilling/main/scripts/install.sh | bash

# TUIダッシュボードを起動
cbilling
```

## なぜ作ったのか

マルチクラウド環境を運用していると、毎月こんな作業が発生します：

1. AWSコンソールにログイン → Cost Explorer を開く
2. GCPコンソールにログイン → Billing レポートを確認
3. Alibaba Cloud コンソールにログイン → 費用センターを開く
4. さらに Tencent Cloud、Volcengine、UCloud...

**7つのコンソールを毎月開いてExcelにまとめる。** これを自動化したくて cbilling を作りました。

### 既存ツールとの違い

| ツール | 特徴 | cbilling との違い |
|:------|:-----|:----------------|
| infracost | IaC のコスト見積もり | cbilling は**実際の請求額**を取得 |
| AWS CLI | AWS 単体の操作 | cbilling は**7社横断**で比較 |
| cloud-custodian | ポリシーベースの管理 | cbilling は**可視化・比較**に特化 |
| 各社コンソール | Web UI | cbilling は**ターミナル完結** |

## 機能一覧

### TUI ダッシュボード

`cbilling` を引数なしで実行すると、フルスクリーンのターミナルUIが起動します。

- **プロバイダータブ** — Tab キーで切り替え、各タブにリアルタイムのコスト合計を表示
- **棒グラフ** — プロバイダー間のコスト分布を横棒で可視化
- **プロダクト一覧** — ソート可能なテーブル。コスト・前月比(MoM)・数量・リージョンを一覧表示
- **リージョン詳細** — Enter キーで展開、プロダクトごとのリージョン別コスト内訳
- **月別ナビゲーション** — `←`/`→` で月を切り替え、自動で再読み込み
- **検索・フィルタ** — `/` でインクリメンタルサーチ
- **前月比較 (MoM)** — `▲▲+28%` / `▼▼-22%` / `NEW` など色分け表示

### CLI コマンド

```bash
# 設定済みプロバイダーを一覧
cbilling providers

# 特定プロバイダーをクエリ
cbilling query aws --month 2026-03

# CSV エクスポート
cbilling query aliyun --csv billing.csv

# 全プロバイダーのサマリー
cbilling summary --month 2026-03
```

出力例：

```
$ cbilling summary --month 2026-03

PROVIDER                   COST CUR   PRODUCTS
------------------------------------------------
aliyun                 98765.43 CNY         65
tencentcloud           33618.72 CNY         12
aws                     4332.39 USD         25
gcp                     1287.64 USD         14
------------------------------------------------
TOTAL                 132384.15 CNY
TOTAL                   5620.03 USD
```

## アーキテクチャ

### プロジェクト構成

```
cbilling/
  src/                     # ライブラリクレート (cbilling)
    providers/             # プロバイダーごとに1モジュール
      aliyun.rs            # HMAC-SHA1 署名
      aws.rs               # 公式 AWS SDK
      tencentcloud.rs      # TC3-HMAC-SHA256 署名
      volcengine.rs        # HMAC-SHA256 署名
      ucloud.rs            # SHA-1 署名
      gcp.rs               # OAuth2 JWT + BigQuery
      cloudflare.rs        # API Token 認証
    service.rs             # 統一クエリ API
    models.rs              # 共通データ型
    error.rs               # エラー型
  crates/cbilling-cli/     # CLI + TUI バイナリ
    src/
      views/               # TUI ビューコンポーネント
      widgets/             # 再利用可能ウィジェット
      styles.rs            # セマンティックスタイルシステム
```

### なぜ Rust を選んだのか

1. **シングルバイナリ** — `curl | bash` でインストール完了。ランタイム依存なし
2. **Feature フラグ** — 不要なプロバイダーを除外してバイナリサイズ削減
3. **async/await** — 複数プロバイダーへの並行クエリ（tokio）
4. **型安全** — 各プロバイダーのAPIレスポンスを型付き構造体で受ける
5. **ratatui** — 美しいターミナルUIを宣言的に構築

### TUI アーキテクチャ（longbridge-terminal 参考）

[longbridge-terminal](https://github.com/longbridge/longbridge-terminal) の設計パターンを参考に、TUI を以下のモジュールに分離しました：

```
views/           # 画面コンポーネント（navbar, summary, footer, help）
widgets/         # 再利用ウィジェット（logo, loading）
styles.rs        # セマンティックカラー関数
ui.rs            # レイアウト計算 + ディスパッチ（薄いオーケストレータ）
```

スタイル定義は関数ベースで、インラインの色定数を排除：

```rust
// styles.rs
pub fn accent() -> Style { Style::default().fg(Color::Cyan) }
pub fn cost_color(cost: f64) -> Style {
    if cost > 10_000.0 { Style::default().fg(Color::Red) }
    else if cost > 1_000.0 { Style::default().fg(Color::Yellow) }
    else { Style::default().fg(Color::Green) }
}
pub fn provider(name: &str) -> Style {
    match name {
        "aliyun" => Style::default().fg(Color::Yellow),
        "aws" => Style::default().fg(Color::LightYellow),
        "gcp" => Style::default().fg(Color::Green),
        // ...
    }
}
```

## クイックスタート

### 1. インストール

```bash
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/Liberxue/cbilling/main/scripts/install.sh | bash

# または Cargo
cargo install cbilling-cli
```

### 2. クレデンシャル設定

環境変数でプロバイダーの認証情報を設定します：

```bash
# AWS（単一アカウント）
export AWS_ACCESS_KEY_ID="AKIA..."
export AWS_SECRET_ACCESS_KEY="wJal..."

# Alibaba Cloud（複数アカウント — JSON形式）
export ALIYUN_ACCOUNTS='[
  { "id": "prod", "name": "本番", "access_key_id": "LTAI...", "access_key_secret": "HkbP..." },
  { "id": "dev",  "name": "開発", "access_key_id": "LTAI...", "access_key_secret": "Jx9a..." }
]'

# GCP（サービスアカウント）
export GCP_ACCOUNTS='[
  { "id": "proj", "name": "My Project", "project_id": "my-proj-123", "private_key": "{...}" }
]'
```

### 3. 起動

```bash
# TUI ダッシュボード（デフォルト）
cbilling

# CLI サマリー
cbilling summary
```

## ライブラリとしての利用

cbilling は CLI ツールだけでなく、Rust ライブラリとしても利用できます：

```toml
[dependencies]
cbilling = { version = "0.1", features = ["aws", "aliyun"] }
```

```rust
use cbilling::service::CloudBillingService;

#[tokio::main]
async fn main() -> cbilling::Result<()> {
    // 設定済みプロバイダーを自動検出
    let providers = CloudBillingService::get_configured_providers();

    // 任意のプロバイダーをクエリ
    let data = CloudBillingService::query_provider("aws", "2026-03").await?;

    println!("Total: {:.2} {} ({} products)",
        data.total_cost, data.currency, data.products.len());

    for p in &data.products {
        println!("  {} — {:.2} {} ({})",
            p.product_name, p.cost, data.currency,
            p.regions.join(", "));
    }
    Ok(())
}
```

各プロバイダーの低レベルAPIクライアントも直接利用可能です：

```rust
use cbilling::providers::aliyun::AliyunBillingClient;

let client = AliyunBillingClient::new("key".into(), "secret".into());
let resp = client.query_instance_bill("2026-03", Some(1), Some(300), None).await?;
```

## 対応プロバイダー詳細

| プロバイダー | 認証方式 | ページネーション | 特記事項 |
|:-----------|:--------|:-------------|:--------|
| Alibaba Cloud | HMAC-SHA1 | page_num/page_size | BSS OpenAPI v2017-12-14 |
| AWS | 公式 AWS SDK | 組み込み | Cost Explorer（us-east-1 固定） |
| Tencent Cloud | TC3-HMAC-SHA256 | なし | DescribeBillSummaryByProduct |
| Volcengine | HMAC-SHA256 | offset/limit | 火山引擎 Billing API |
| UCloud | SHA-1 | offset/limit | UBill API |
| GCP | OAuth2 JWT | なし | BigQuery エクスポート対応 |
| Cloudflare | API Token | なし | Subscriptions + Billing History |

## Feature フラグによるモジュラービルド

不要なプロバイダーを除外してビルドサイズを最適化できます：

```toml
# AWS と GCP だけ
cbilling = { version = "0.1", default-features = false, features = ["aws", "gcp"] }
```

```bash
# Feature フラグ一覧
aliyun, tencentcloud, aws, volcengine, ucloud, gcp, cloudflare, all-providers
```

## 今後のロードマップ

- [ ] **Azure** サポート追加
- [ ] **BillingProvider trait** — プロバイダー間の抽象化レイヤー
- [ ] **コスト予測** — 月末までの推定コスト表示
- [ ] **アラート** — コスト閾値を超えた場合の通知
- [ ] **Web UI** — ブラウザベースのダッシュボード
- [ ] **AI 統合** — 自然言語でコスト分析（Claude / GPT 連携）

## コントリビューション

cbilling はオープンソース（Apache-2.0）です。PR、Issue、Star 歓迎です！

- GitHub: https://github.com/Liberxue/cbilling
- crates.io: https://crates.io/crates/cbilling
- docs.rs: https://docs.rs/cbilling

特に以下の貢献を歓迎しています：

- 新しいクラウドプロバイダーの追加（Azure, Oracle Cloud, etc.）
- テストの追加
- ドキュメントの改善
- TUI/UX の改善提案

```bash
# 開発環境セットアップ
git clone https://github.com/Liberxue/cbilling.git
cd cbilling
cargo build
cargo test --all-features
```

---

最後まで読んでいただきありがとうございます。マルチクラウドのコスト管理に悩んでいる方は、ぜひ `cbilling` を試してみてください。Star をいただけると開発の励みになります！

https://github.com/Liberxue/cbilling

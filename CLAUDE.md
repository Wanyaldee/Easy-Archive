# Easy Archive リポジトリについて

文字コードを自動判定するZIP解凍・圧縮GUIツール（日本の学校・自治体向け、Rust製OSS）。詳細な背景・スコープは [`docs/spec.md`](./docs/spec.md)、技術的決定の経緯・検証結果は [`docs/adr/`](./docs/adr/) を必ず参照すること。仕様の再検証や再議論はここから始める（コードだけを読んで仕様を推測しない）。

## このリポジトリで作業するときのルール

- **ドキュメント・コメント・コミットメッセージ・Issue/PRは日本語**で書く（Citadelなど他プロジェクトとは異なる方針。対象ユーザーが日本の教員・自治体職員のため）。関数名・変数名などのコード識別子はRustの慣習に従い英語のままでよい。
- **文字コード判定はエントリ（ファイル名）単位で行う**こと。アーカイブ全体で1回だけ判定する実装（`rc-zip`はこの方式で不採用と判断済み。[ADR 0001](./docs/adr/0001-zip-crate-over-rc-zip.md)参照）は同じ轍を踏むので避ける。
- ZIPの読み書きは `zip` クレートに統一する。`ZipFile::name_raw()` で生バイト列を取得し、`encoding_rs`でShift-JISとしてデコードを試み、エラーなく成功すればShift-JIS、失敗すればCP437として扱う（エントリ単位）。`chardetng`はファイル名程度の短いバイト列で誤判定が多いため不採用（[ADR 0002](./docs/adr/0002-drop-chardetng-strict-decode.md)参照）。
- ZIP内のOffice/バイナリファイルの中身はバイト列のまま無変換で読み書きする（中身の文字コード変換は対象外）。
- 対応フォーマットは当面ZIPのみ、対象OSはUbuntu系（Zorin OS含む）のみ。他フォーマット・他OS対応をついでに広げない。
- 新規依存クレートを追加する前に、スコープが本当に必要としているか（YAGNI）を確認する。

## ディレクトリ構成

```
Easy-Archive/
├── CLAUDE.md
├── README.md
├── LICENSE                # MIT
├── Cargo.toml              # ワークスペース定義
├── crates/
│   ├── core/                # Issue #1「コア」— 判定ロジック・ZIP読み書き・CLI
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── main.rs       # CLI(list/compress/extract)
│   │       ├── compress.rs
│   │       ├── encoding.rs
│   │       └── extract.rs
│   └── gui/                 # Issue #2「GUI」
│       └── src/
│           └── main.rs
├── docs/
│   ├── spec.md             # 仕様書（確定している仕様のみ）
│   └── adr/                # 技術的決定の経緯・検証結果・議論（番号順）
│       ├── 0001-zip-crate-over-rc-zip.md
│       ├── 0002-drop-chardetng-strict-decode.md
│       └── 0003-windows11-zip-utf8-default.md
└── .claude/
    ├── settings.json       # プロジェクト共有設定（プラグイン・権限・言語）
    └── skills/
        └── easy-archive-core/
            └── SKILL.md    # コア設計方針の要点（判定粒度・クレート選定）
```

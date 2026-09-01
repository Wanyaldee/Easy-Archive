# 0004. winitのWaylandバックエンドはファイルドロップ未実装のためX11を強制する

## ステータス

Accepted

## 背景

マイルストーン3のGUI実装後、日本語フォント適用（Noto Sans JP、別途記録）と合わせて実機（WSL環境）で動作確認したところ、以下の症状が出た。

- ウィンドウは正しく起動し、Noto Sans JPによる日本語テキスト表示も文字化けせず確認できた
- しかし、Nautilusからファイル/フォルダをウィンドウへドラッグ&ドロップしても**一切反応しない**
- 同じWSL環境でNautilus同士や他アプリへのドラッグ&ドロップは正常に動作する（＝WSLg全体のDNDが壊れているわけではない）

## 原因調査

`crates/gui`が依存する`egui-winit`のソース（`egui-winit-0.36.1/src/lib.rs`）を確認したところ、`winit::event::WindowEvent::DroppedFile`を正しく受け取り`egui`の`dropped_files`へ積む処理は実装されていた。つまりアプリ側のコードの問題ではない。

次に`winit`本体（`winit-0.30.13`）のソースを直接検索したところ、`DroppedFile`イベントの発火処理は以下にのみ存在した。

- `platform_impl/windows/drop_handler.rs`（Windows）
- `platform_impl/linux/x11/event_processor.rs`（Linux X11、XDND経由）
- `platform_impl/macos/window_delegate.rs`（macOS）

**`platform_impl/linux/wayland/`配下には`DroppedFile`関連の実装が一切存在しない。** つまり`winit 0.30.13`のWaylandバックエンドはファイルドロップを実装しておらず、Waylandセッション上ではこのイベントが原理的に発火しない。

`winit`のバックエンド選択ロジック（`platform_impl/linux/mod.rs`）は、`WAYLAND_DISPLAY`環境変数が設定されていれば自動的にWaylandを選ぶ。開発環境のWSLgは`WAYLAND_DISPLAY`を設定しているため、常にWaylandバックエンドが選ばれ、ドロップが機能しなかった。

## 決定

`eframe::NativeOptions::event_loop_builder`フックを使い、`winit::platform::x11::EventLoopBuilderExtX11::with_x11()`でX11バックエンドを強制する（`crates/gui/src/main.rs`）。X11バックエンドはXDNDでドロップイベントを実装しているため、これで回避できる。

対象OS（Ubuntu/Zorin OS）は標準でXWayland（X11互換レイヤー）を同梱しているため、Waylandセッションであっても本アプリはX11(XWayland)経由で動作し、ドロップが機能する見込み。

`winit`は`eframe`経由の間接依存だったため、`EventLoopBuilderExtX11`トレイトをインポートするには`crates/gui/Cargo.toml`に`winit`を直接依存として追加する必要があった（`default-features = false, features = ["x11"]`。`Cargo.lock`上で`eframe`経由の解決と同一バージョンに統一されていることを確認済み）。

## 検証結果

実機Zorin OSでX11強制後のGUIを起動し、ZIPファイルをドラッグ&ドロップして解凍が動作することを確認済み。仮説通り、WSLg固有のWayland⇔X11 DNDブリッジ制約が原因であり、実機（ネイティブのXWayland）ではこの強制設定で問題なく動作する。

圧縮側（ディレクトリ/ファイルをドロップ）も実機Zorin OSで動作を確認済み。`handle_drop`の判定ロジックは解凍・圧縮で共通のため予想通りの結果だった。

## 既知の限界

- WSLg環境ではXWayland実装の制約により改善しない可能性があったが、実機Zorin OSでは解凍・圧縮双方の動作を確認済み。WSL環境はあくまで開発時の参考であり、対象OS（Ubuntu/Zorin OS）実機での動作が正である。
- 本アプリは常にX11(XWayland)経由で動作することになり、Wayland固有の利点（セキュリティ分離、HiDPIスケーリングの精度等）は得られない。単機能ユーティリティである本アプリの用途では許容できると判断した。
- 将来`winit`がWaylandバックエンドにファイルドロップを実装した場合、この強制設定は不要になる可能性がある。`winit`のリリースノートを定期的に確認する価値がある。

## 影響

- `cargo build --workspace` / `cargo test --workspace`は成功。ドロップ判定ロジック自体（`handle_drop`等）へのコード変更は無いため、既存テストへの影響もない。
- 実機Zorin OSでの解凍・圧縮双方のドラッグ&ドロップ動作を確認済み。

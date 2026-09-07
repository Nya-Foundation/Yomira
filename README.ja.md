<p align="center">
  <img src="public/yomira.svg" width="88" height="88" alt="Yomira ロゴ" />
</p>
<h1 align="center">Yomira</h1>
<p align="center">閲覧と読書のための、心地よい空間。</p>
<p align="center">
  <a href="README.md">English</a> · <a href="README.zh-CN.md">简体中文</a> · <strong>日本語</strong>
</p>
<p align="center">
  <a href="https://github.com/Nya-Foundation/Yomira/actions/workflows/ci.yml">CI</a> ·
  <a href="https://github.com/Nya-Foundation/Yomira/releases">ダウンロード</a> ·
  <a href="docs/ROADMAP.md">ロードマップ</a> ·
  <a href="CONTRIBUTING.md">貢献ガイド</a>
</p>

Yomira は、Rust、Tauri 2、React で構築された、Android とデスクトップ向けの
非公式 E-Hentai / ExHentai クライアントです。
[Ehviewer_CN_SXJ](https://github.com/xiaojieonly/Ehviewer_CN_SXJ) に着想を得て、
おなじみのギャラリー閲覧・読書機能を、タッチ操作、キーボード、マウスに対応する
ミニマルなインターフェースにまとめています。

本プロジェクトは活発に開発中です。E-Hentai や EhViewer のメンテナーとは提携していません。
接続先のサービスには成人向けコンテンツが含まれるため、
利用はお住まいの地域で法的に成人と認められる方に限られます。

## 探す、読む、保存する

- **次に読む作品を探す。** ホーム、人気、ウォッチ、ランキングの閲覧に対応。
  サムネイル表示と詳細リスト表示を切り替えられ、カテゴリ・評価・ページ数で絞り込めます。
  画像検索、複数タグの入力補完、検索キーワードの保存も利用できます。
- **詳しい情報を見る。** タグと中国語訳、ギャラリーのプレビュー、評価、コメント、
  オンラインのお気に入りとそのメモを確認できます。
- **快適に読む。** ページ単位・連続スクロールの読書モード、スワイプによるページ送り、
  右から左への読み順、拡大・縮小と表示位置の移動に対応。先読み数の調整と読書位置の保存も可能です。
- **オフラインで読む。** ダウンロードキューを永続化し、一時停止、再開、再試行、ラベル管理に対応。
  ダウンロード途中のギャラリーでも、ギャラリー情報を再取得せずに保存済みのページを読めます。
- **自分好みに整える。** ライト・ダークテーマ、英語・中国語の UI、表示倍率の調整に対応。
  デスクトップの戻る・進む操作やズームのショートカット、Android の戻る操作も利用できます。
- **接続を設定する。** プロキシ、DNS over HTTPS、組み込み・カスタム hosts、
  画像解像度の設定、容量制限付きのディスク画像キャッシュを備えています。

Yomira は、まだ EhViewer のすべての機能を網羅していません。ローカルのお気に入り、
MyTags の編集、バックアップ・インポート、Android ネイティブのバックグラウンドダウンロードサービス、
iOS 対応は[ロードマップ](docs/ROADMAP.md)に記載しています。
性能の測定結果とその限界については、[パフォーマンス](docs/PERFORMANCE.md)をご覧ください。

## スクリーンショット

以下は Non-H カテゴリの架空のデモ用ギャラリーに、一般公開に適したオリジナルのイラストを
使用した画面です。アカウントのデータや接続先サービスの画像は含まれていません。

![デスクトップでのギャラリー閲覧](docs/screenshots/gallery-list.png)

<p align="center">
  <img src="docs/screenshots/gallery-list-mobile.png" width="280" alt="幅の狭い画面でのギャラリー閲覧" />
  <img src="docs/screenshots/settings-light-mobile.png" width="280" alt="ライトテーマの設定画面" />
</p>

## Yomira を入手する

[GitHub Releases](https://github.com/Nya-Foundation/Yomira/releases) からインストール用パッケージをダウンロードできます。

| プラットフォーム | パッケージ | ビルド状況 |
| --- | --- | --- |
| Android 7.0 以降 | ユニバーサル APK：ARM64・ARMv7 | CI でビルド、リリース版は署名済み |
| Windows x64 | `.exe` インストーラー | CI でビルド、未署名 |
| Linux x64 | `.deb`、`.AppImage` | Ubuntu 22.04 を基準にビルド |
| macOS | Apple Silicon・Intel 向け `.dmg` | CI でビルド、未署名・未公証 |

ビルドの成功は、実機での動作確認に代わるものではありません。
必要な環境と各プラットフォームの制約は、[ビルドと署名](docs/BUILDS.md)をご覧ください。

## ローカルで開発する

[Tauri の各プラットフォームの前提条件](https://v2.tauri.app/start/prerequisites/)を満たしたうえで、
Rust（リポジトリでは 1.92.0 に固定）、Node.js 24.x（24.10 以降）、Bun 1.2.18 をインストールしてください。

```sh
git clone https://github.com/Nya-Foundation/Yomira.git
cd Yomira
bun install --frozen-lockfile
bun run tauri dev
```

フロントエンドの開発では、`bun run dev` で架空のデータを使ったブラウザデモを起動できます。
ネイティブのネットワーク通信やファイルシステムの動作を確認するには、Tauri アプリが必要です。

```sh
bun run build                 # TypeScript チェックと本番用フロントエンドのビルド
bun run test:unit             # フロントエンドの単体テスト
bunx playwright install chromium
bun run test:e2e              # ブラウザの回帰テスト
cargo test --locked --manifest-path src-tauri/Cargo.toml
```

デスクトップ版のパッケージ作成には `bun run tauri build` を使います。
Android の手順とリリースの自動化については、[BUILDS.md](docs/BUILDS.md) と
[RELEASING.md](docs/RELEASING.md) をご覧ください。

## プロジェクト構成

```text
src/                    React UI、ルーティング、型付き IPC ラッパー、状態管理
src-tauri/src/          Rust のネットワーク処理、パーサー、永続化、ダウンロード
src-tauri/gen/android/  継続的に管理する Android プロジェクトとネイティブ連携
design/branding/        正式なロゴとアイコン生成の説明
tests/ + e2e/           単体テスト、リリースツールのテスト、ブラウザの回帰テスト
scripts/               ブランド素材、性能分析、リリース用のツール
```

UI は `src/lib/api/` にある機能領域ごとのラッパーを通じて Rust を呼び出します。
ブラウザ用のデモデータは Tauri の外でのみ読み込まれます。ネットワークアクセス、
SQLite による永続化、ダウンロード、画像キャッシュは Rust が担当します。
詳しくは[アーキテクチャと保守](docs/MAINTENANCE.md)、[デザインシステム](docs/DESIGN_SYSTEM.md)、
[オフライン読書](docs/OFFLINE_READING.md)をご覧ください。

アプリのデータは各プラットフォームのアプリ用ディレクトリに保存されます。
セッション Cookie はローカルの `cookies.json` に保存され、ダウンロードしたギャラリーやキャッシュは
ソースコードのディレクトリとは別に管理されます。既存のインストールのデータを引き継ぐため、
従来のアプリ識別子と `exviewer.db` は意図的に維持しています。
問題を報告する際は、Cookie や個人のギャラリーを含めないでください。

## 謝辞とライセンス

Yomira は、**xiaojieonly と Ehviewer_CN_SXJ の貢献者**、および
**Hippo Seven とオリジナルの EhViewer の貢献者**の成果を基にしています。
ギャラリーパーサー、URL の組み立て、クライアントの動作、ダウンロード・タグ処理の一部は、
これらの成果を基に Rust へ移植・改変したものです。中国語のタグ検索には、
[Mapaler/EhViewer](https://github.com/Mapaler/EhViewer) が保守する翻訳データの配布版を使用しています。

本アプリは **GPL-3.0-only** で配布されています。上流の Apache-2.0 に基づく部分は、
元の条件と帰属表示を保持しています。[LICENSE](LICENSE)、[NOTICE](NOTICE)、
[サードパーティに関する通知](THIRD_PARTY_NOTICES.md)をご確認ください。
移植の基になった上流の具体的なリビジョンについてのライセンス情報も記載しています。

本プロジェクトを支えるライブラリを提供してくださる Rust、Tauri、React、
そして広くオープンソースコミュニティの皆さまに感謝します。

この README からリンクしている他のプロジェクト文書は、現在主に英語で提供されています。

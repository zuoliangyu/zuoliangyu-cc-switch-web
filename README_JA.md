# CC Switch Web

[English](README.md) | [中文](README_ZH.md) | 日本語

## 概要

CC Switch Web は [cc-switch](https://github.com/farion1231/cc-switch) の Web ブランチ用リポジトリです。

このリポジトリは、CC Switch に関する Web 向け実装、関連する実験、そしてブランチ固有の調整を管理するために使用されます。

現在の想定アーキテクチャは次の通りです。

- フロントエンド: Web
- バックエンド: ローカル Rust サービス
- アクセス方法: ブラウザで `http://localhost:xxxx` を開く

この方向は、通常の Windows / Linux 環境に加えて、デスクトップのない Linux サーバーも対象にしています。

## アップストリームとの関係

- アップストリームプロジェクト: [cc-switch](https://github.com/farion1231/cc-switch)
- このリポジトリは CC Switch の Web ブランチ方向に焦点を当てています
- プロジェクトの位置付けや外部向け説明が変わった場合は、このリポジトリ内の各言語版 README を同期して更新してください

## 補足

元の CC Switch プロジェクト、デスクトップアプリ、またはアップストリームのリリース情報を確認したい場合は、上流リポジトリを直接参照してください。

## 実行方法

### ローカル実行

1. 依存関係をインストールします。

   ```bash
   pnpm install --frozen-lockfile
   ```

2. デフォルトの Web 開発モード:

   ```bash
   pnpm dev
   ```

   これは次と同等です。

   ```bash
   pnpm dev:web
   ```

   その後、[http://localhost:3000](http://localhost:3000) を開いてください。フロントエンドはローカル Rust サービス `http://127.0.0.1:8788` に接続します。

3. Docker 構成をフォアグラウンドで起動したい場合は、次も使えます。

   ```bash
   pnpm dev:d
   ```

4. 本番に近い形でローカル実行する場合:

   ```bash
   pnpm build:web
   pnpm start:web
   ```

   その後、[http://localhost:8788](http://localhost:8788) を開いてください。

5. 一度ビルドして release バイナリを直接実行する場合:

   ```bash
   pnpm build:web
   pnpm build:web:service
   ```

   Linux:

   ```bash
   pnpm start:l
   ```

   Windows:

   ```powershell
   pnpm start:w
   ```

### Docker 実行

1. デフォルトの標準ビルド:

   ```bash
   pnpm build
   ```

   これは次と同等です。

   ```bash
   pnpm build:d
   ```

   このコマンドは Docker のビルド環境内でフロントエンドと Rust サービスを直接ビルドします。

2. フォアグラウンドでビルドして起動します。

   ```bash
   pnpm dev:d
   ```

   このコマンドは `docker compose up --build` をフォアグラウンドで実行します。

3. イメージのビルドが済んでいて、バックグラウンド起動だけしたい場合:

   ```bash
   pnpm up:d
   ```

4. イメージのみ再ビルドする場合:

   ```bash
   pnpm build:d
   ```

5. ログを確認する場合:

   ```bash
   pnpm logs:d
   ```

6. 停止します。

   ```bash
   pnpm down:d
   ```

7. [http://localhost:8788](http://localhost:8788) を開きます。

8. 永続データは `cc-switch-web-data` volume に保存されます。

### Tauri 互換コマンド

デスクトップシェルを一時的にデバッグしたい場合は、明示的に次を使ってください。

```bash
pnpm dev:tauri
pnpm build:tauri
```

これらは、もはやこのリポジトリのデフォルト経路ではありません。

コンテナ内のサービスからホスト側の CLI 設定ディレクトリを直接管理したい場合は、`docker-compose.yml` に `.claude`、`.codex`、`.gemini`、`opencode`、`openclaw` などの bind mount を追加してください。

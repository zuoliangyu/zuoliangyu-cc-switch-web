# CC Switch Web

[English](README.md) | [中文](README_ZH.md) | 日本語

## 概要

CC Switch Web は [cc-switch](https://github.com/farion1231/cc-switch) の Web ブランチ用リポジトリです。

このリポジトリは、CC Switch に関する Web 向け実装、関連する実験、そしてブランチ固有の調整を管理するために使用されます。

現在の想定アーキテクチャは次の通りです。

- フロントエンド: Web
- バックエンド: ローカル Rust サービス
- アクセス方法: ブラウザで `http://localhost:xxxx` を開く

この方向は、Windows、macOS、Linux、およびデスクトップのない Linux サーバーを対象にしています。

## 現在のバージョン

現在のリポジトリバージョンは `0.1.0` です。

このリポジトリでは、`0.1.0` を Web ブランチの初回リリース基準として扱います。以前に引き継がれていた過去のリリース履歴はこのリポジトリから削除しており、より古い履歴はアップストリーム側を参照してください。

## アップストリームとの関係

- アップストリームプロジェクト: [cc-switch](https://github.com/farion1231/cc-switch)
- 現在の Web リポジトリ: [zuoliangyu/zuoliangyu-cc-switch-web](https://github.com/zuoliangyu/zuoliangyu-cc-switch-web)
- 作者: 左岚（[Bilibili](https://space.bilibili.com/27619688)）
- このリポジトリは CC Switch の Web ブランチ方向に焦点を当てています
- プロジェクトの位置付けや外部向け説明が変わった場合は、このリポジトリ内の各言語版 README を同期して更新してください

## 補足

元の CC Switch プロジェクト、またはアップストリームのリリース情報を確認したい場合は、上流リポジトリを直接参照してください。

## 実行方法

### コマンド早見表

| 用途 | コマンド |
| --- | --- |
| ローカル開発（`w`） | `pnpm dev` |
| Docker フォアグラウンド開発（`d`） | `pnpm dev -- d` |
| ローカル release ビルド（`w`） | `pnpm build` |
| Docker イメージビルド（`d`） | `pnpm build -- d` |
| プロジェクトチェック | `pnpm check` |
| Windows 上で 3 種類の成果物を出力 | `.\scripts\package-artifacts.ps1` |

### ローカル開発

1. 依存関係をインストールします。

   ```bash
   pnpm install --frozen-lockfile
   ```

   バックエンドのビルドとチェックには Rust `1.88+` が必要です。

2. 開発モードを起動します。

   ```bash
   pnpm dev
   ```

   明示的な書き方:

   ```bash
   pnpm dev -- w
   ```

   Windows では次も使えます。

   ```powershell
   .\scripts\dev.ps1 w
   ```

3. [http://localhost:3000](http://localhost:3000) を開きます。フロントエンドはローカル Rust サービス `http://127.0.0.1:8788` に接続します。

### ローカル Release バイナリ

1. フロントエンドを埋め込んだ release バイナリをビルドします。

   ```bash
   pnpm build
   ```

   明示的な書き方:

   ```bash
   pnpm build -- w
   ```

   Windows では次も使えます。

   ```powershell
   .\scripts\build.ps1 w
   ```

2. 出力先:

   - Windows: `backend\target\release\cc-switch-web.exe`
   - Linux/macOS: `backend/target/release/cc-switch-web`

3. 対応するバイナリを直接実行し、その後 [http://localhost:8788](http://localhost:8788) を開きます。

4. ローカル Web サービスモードでも、CC Switch Web 自身のデータ保存先は CC Switch が使うローカル設定ルートです。

   ```text
   ~/.cc-switch
   ```

   ここには `settings.json`、`cc-switch.db`、バックアップ、統一 Skills ストレージなどが保存されます。旧 `config.json` は現在の Web ランタイムの主データ経路には含まれません。

### Docker 実行

1. Docker イメージをビルドします。

   ```bash
   pnpm build -- d
   ```

   Windows では次も使えます。

   ```powershell
   .\scripts\build.ps1 d
   ```

2. Docker 構成をフォアグラウンドで起動します。

   ```bash
   pnpm dev -- d
   ```

   Windows では次も使えます。

   ```powershell
   .\scripts\dev.ps1 d
   ```

3. イメージビルド後にバックグラウンド実行へ切り替えたい場合は、Docker を直接使います。

   ```bash
   docker compose up -d
   docker compose logs -f
   docker compose down
   ```

4. [http://localhost:8788](http://localhost:8788) を開きます。永続データは `cc-switch-web-data` volume に保存されます。

5. コンテナ内のサービスからホスト側の CLI 設定ディレクトリを直接管理したい場合は、まずサンプルファイルをコピーします。

   ```bash
   cp docker-compose.host.example.yml docker-compose.host.yml
   ```

   その後、実際の環境に合わせてパスを調整し、次を実行します。

   ```bash
   docker compose -f docker-compose.yml -f docker-compose.host.yml up -d
   ```

   このサンプルは主に Linux サーバー向けで、`$HOME` 配下の `.claude`、`.codex`、`.gemini`、`.config/opencode`、`.config/openclaw` を前提にしています。

### Docker 内で Linux 配布パッケージを出力

ホスト環境を汚さずに Linux 向け配布パッケージを作りたい場合は、Docker Buildx を直接使います。

```bash
docker buildx build --target package-linux-tar --output type=local,dest=release/docker-linux .
```

出力される圧縮ファイル:

```text
release/docker-linux/cc-switch-web-linux-x64.tar.gz
```

未圧縮ディレクトリを直接出力したい場合:

```bash
docker buildx build --target package-linux-dir --output type=local,dest=release/docker-linux .
```

出力先:

```text
release/docker-linux/cc-switch-web-linux-x64/
```

内容は単一実行ファイル `cc-switch-web` のみです。展開後はそのバイナリを直接実行してください。

現在の Linux 配布バイナリは `x86_64-unknown-linux-musl` の静的リンク版で、ホスト側ランタイム差異の影響を受けにくくしています。

### Windows で成果物をまとめて出力

Windows 上で Rust と Docker / Buildx が利用できる場合は、次を実行してください。

```powershell
.\scripts\package-artifacts.ps1
```

このスクリプトは 1 回で次の 3 種類の成果物を生成します。

- Windows 実行ファイル: `release\local-artifacts\windows\cc-switch-web.exe`
- Linux 配布パッケージ: `release\local-artifacts\linux\cc-switch-web-linux-x64.tar.gz`
- Docker イメージアーカイブ: `release\local-artifacts\docker\cc-switch-web-docker-image.tar.gz`

内容:

- Windows 成果物はローカルの `cargo build --locked --release` から生成
- Linux 成果物は Docker Buildx の `package-linux-tar` stage から生成
- Docker イメージアーカイブは次で取り込めます。

```powershell
docker load -i .\release\local-artifacts\docker\cc-switch-web-docker-image.tar.gz
```

### Linux systemd サンプル

デスクトップのない Linux サーバーで常駐させたい場合は、リポジトリ内の次のサンプルを使ってください。

`deploy/systemd/cc-switch-web.service.example`

推奨手順:

1. Linux 上で `pnpm build` を実行してバイナリを作成するか、パッケージ済みの Linux バイナリを `/opt/cc-switch-web` に配置します。

2. サービスファイルをシステムディレクトリへコピーします。

   ```bash
   sudo cp deploy/systemd/cc-switch-web.service.example /etc/systemd/system/cc-switch-web.service
   ```

3. 実際の環境に合わせて次の項目を修正します。
   - `User`
   - `Group`
   - `WorkingDirectory`
   - `HOME`
   - `ExecStart`

4. 再読み込みして起動します。

   ```bash
   sudo systemctl daemon-reload
   sudo systemctl enable --now cc-switch-web
   ```

5. 状態とログを確認します。

   ```bash
   sudo systemctl status cc-switch-web
   sudo journalctl -u cc-switch-web -f
   ```

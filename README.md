# SARAKURA

**English** | [日本語](README.md#japanese) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | [Français](README.fr.md) | [Español](README.es.md) | [Português (Brasil)](README.pt-BR.md) | [Deutsch](README.de.md)

<a name="english"></a>

## English

Diagnostic analysis and English report generation for KITAQGB/KOKURA and KITAQFC/KUROSAKI.

Public preview: APIs and behavior may change.

### Prebuilt Windows CLI

The repository root includes `sarakura.exe`, built for Windows x64 in Release mode.
It is a command-line program. Rust, Python and .NET are not required to run it.
Download the repository ZIP to keep the executable and license notices together.
The native C runtime is linked statically; the program uses Windows system DLLs.

```powershell
.\sarakura.exe --help
```

Rebuild only this CLI with `.\scripts\build.ps1` (optionally `-Offline`).
The script copies the executable to the repository root. GUI frontends, Python
extension modules and C API DLLs are not part of this executable distribution.
See [binary dependency notices](BINARY_NOTICES.md).

The licensor of the independent project code is **DAISUKE OBA**, under MIT.
When redistributing, include `LICENSE`, `LICENSE.ja`, `BINARY_NOTICES.md` and the `licenses/` directory.
Third-party libraries retain their own copyright holders and terms.

### Prebuilt Linux and macOS CLI

Rust, Python and .NET are not required to run the CLI. The Linux build targets x86_64/glibc; choose the matching CPU for macOS.

| OS / CPU | CLI |
| --- | --- |
| Linux x86_64 (glibc) | [bin/linux-x86_64/sarakura](bin/linux-x86_64/sarakura) |
| macOS ARM64 | [bin/macos-arm64/sarakura](bin/macos-arm64/sarakura) |
| macOS Intel | [bin/macos-x86_64/sarakura](bin/macos-x86_64/sarakura) |

```sh
chmod +x bin/linux-x86_64/sarakura
./bin/linux-x86_64/sarakura --help

chmod +x bin/macos-arm64/sarakura
./bin/macos-arm64/sarakura --help

chmod +x bin/macos-x86_64/sarakura
./bin/macos-x86_64/sarakura --help
```

Run the commands from the repository root. For redistribution, retain LICENSE, LICENSE.ja, BINARY_NOTICES.md and licenses/.

### First use

```powershell
.\sarakura.exe gb analyze --metadata examples/gb/e2e/metadata.json --events examples/gb/e2e/events.jsonl --out out/gb
```

This supplied example intentionally produces an error and a warning. Open `report.html` in the output folder to inspect them.


### Build and first use

A current stable Rust toolchain. On Windows, install Visual Studio Build Tools with the Desktop development with C++ workload.

```powershell
.\scripts\build.ps1
.\sarakura.exe --help
```

### Manuals and licenses

- [Japanese HTML manuals](https://bartaro.github.io/kitaq-docs/sarakura.html) / [English manuals](https://bartaro.github.io/kitaq-docs/en/sarakura.html)
- [Offline manual source](https://github.com/bartaro/kitaq-docs)
- [License](LICENSE) / [Japanese reference translation](LICENSE.ja)

The project license does not replace third-party font, dependency, logo or trademark terms. Preserve the accompanying notices when redistributing.


---

<a name="japanese"></a>

## 日本語

KITAQGB/KOKURAおよびKITAQFC/KUROSAKI向けの診断解析と英語レポート生成を行うツールです。

パブリックプレビュー版です。APIや動作は変更される場合があります。

### ビルド済みWindows CLI

リポジトリ直下に、Windows x64向けにReleaseモードでビルドした `sarakura.exe` を同梱しています。コマンドラインで使うプログラムで、実行時にRust・Python・.NETは不要です。リポジトリのZIPを取得して、実行ファイルと権利表記をまとめて入手してください。ネイティブCランタイムは静的リンクされており、WindowsのシステムDLLを使用します。

```powershell
.\sarakura.exe --help
```

このCLIだけを再ビルドするには `.\scripts\build.ps1` を実行します。必要に応じて `-Offline` を指定できます。スクリプトは実行ファイルをリポジトリ直下へコピーします。GUI、Python拡張モジュール、C APIのDLLは、この実行ファイルの配布には含めていません。

[バイナリの依存物に関する権利表記](BINARY_NOTICES.md)を参照してください。配布時は `LICENSE`、`LICENSE.ja`、`BINARY_NOTICES.md` と `licenses/` フォルダーも同梱してください。

独自コードの著作権者・ライセンサーは **DAISUKE OBA** で、MITライセンスを適用しています。第三者のライブラリは、それぞれの著作権者と利用条件を保持します。

### ビルド済みLinux・macOS CLI

CLIの実行にRust・Python・.NETは不要です。Linux版はx86_64/glibc向けです。macOSではCPUに合う版を選んでください。

| OS / CPU | CLI |
| --- | --- |
| Linux x86_64 (glibc) | [bin/linux-x86_64/sarakura](bin/linux-x86_64/sarakura) |
| macOS ARM64 | [bin/macos-arm64/sarakura](bin/macos-arm64/sarakura) |
| macOS Intel | [bin/macos-x86_64/sarakura](bin/macos-x86_64/sarakura) |

```sh
chmod +x bin/linux-x86_64/sarakura
./bin/linux-x86_64/sarakura --help

chmod +x bin/macos-arm64/sarakura
./bin/macos-arm64/sarakura --help

chmod +x bin/macos-x86_64/sarakura
./bin/macos-x86_64/sarakura --help
```

コマンドはリポジトリ直下で実行します。再配布時はLICENSE、LICENSE.ja、BINARY_NOTICES.md、licenses/も保持してください。

### 最初の使い方

```powershell
.\sarakura.exe gb analyze --metadata examples/gb/e2e/metadata.json --events examples/gb/e2e/events.jsonl --out out/gb
```

この付属サンプルは、診断の読み方を確認するためにエラーと警告を含んでいます。出力先の`report.html`を開いて結果を確認してください。


### FDS

FDS CRC errors and write-protection requests now map to dedicated FC catalog rules. See [FDS diagnostics](docs/fds-media-20261102.md).

FDSのCRCエラーと書き込み保護要求に対応する専用診断を追加しました。

### ビルドと初回利用

再ビルドには現在の安定版Rustツールチェーンが必要です。Windowsでは、Visual Studio Build Toolsの「C++によるデスクトップ開発」ワークロードをインストールしてください。

```powershell
.\scripts\build.ps1
.\sarakura.exe --help
```

### マニュアルとライセンス

- [日本語HTMLマニュアル](https://bartaro.github.io/kitaq-docs/sarakura.html) / [英語HTMLマニュアル](https://bartaro.github.io/kitaq-docs/en/sarakura.html)
- [オフライン用マニュアルのソース](https://github.com/bartaro/kitaq-docs)
- [ライセンス英語原文](LICENSE) / [日本語参考訳](LICENSE.ja)

プロジェクトのライセンスは、第三者のフォント、依存ライブラリ、ロゴ、商標に関する条件を置き換えるものではありません。再配布時は付属の権利表記も保持してください。

# SARAKURA

<!-- readme-language-links:start -->
[English](README.md#english) | [日本語](README.md#japanese) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | **繁體中文** | [Français](README.fr.md) | [Español](README.es.md) | [Deutsch](README.de.md)
<!-- readme-language-links:end -->

**[SARAKURA · HTML 手冊](https://bartaro.github.io/kitaq-docs/zh-TW/sarakura.html)**

分析 KITAQGB／KOKURA 與 KITAQFC／KUROSAKI 的診斷資料，並產生英文報告。

本專案目前為公開預覽版，API 與行為仍可能調整。

## Windows 執行檔

儲存庫根目錄附有 Windows x64 Release 版 `sarakura.exe`。這是命令列程式，執行時不必另外安裝 Rust、Python 或 .NET。建議下載整個儲存庫的 ZIP，一併取得執行檔與授權聲明。原生 C 執行階段採靜態連結，程式也會使用 Windows 系統 DLL。

```powershell
.\sarakura.exe --help
```

若只要重新建置命令列工具，請執行 `.\scripts\build.ps1`；需要離線建置時可加上 `-Offline`。腳本會將執行檔複製到儲存庫根目錄。這次的執行檔發行不包含圖形介面、Python 擴充模組或 C API DLL。相關資訊請見[二進位檔建置紀錄](BINARY_BUILD.json)與[二進位檔相依套件授權聲明](BINARY_NOTICES.md)。

專案自行撰寫程式碼的授權人為 **DAISUKE OBA**，採用 MIT 授權條款。再散布時，請保留 `LICENSE`、`LICENSE.ja`、`BINARY_NOTICES.md` 與 `licenses/` 目錄。第三方程式庫仍須依各權利人的授權條件使用。

## 從原始碼建置與開始使用

重新建置需要近期穩定版 Rust 工具鏈。Windows 環境還須安裝 Visual Studio Build Tools，並選取「使用 C++ 的桌面開發」工作負載。

```powershell
.\scripts\build.ps1
.\sarakura.exe --help
```

## 手冊與授權

- [SARAKURA · HTML 手冊](https://bartaro.github.io/kitaq-docs/zh-TW/sarakura.html)
- [英文手冊](https://bartaro.github.io/kitaq-docs/en/sarakura.html)／[日文手冊](https://bartaro.github.io/kitaq-docs/sarakura.html)
- [可供離線閱讀的手冊原始檔](https://github.com/bartaro/kitaq-docs)
- [授權條款](LICENSE)／[日文參考譯文](LICENSE.ja)

專案授權不會取代第三方相依元件、標誌或商標的使用條件。重新散布時請保留隨附聲明。


<!-- native-platform-binaries-20261004-zh-TW -->
### 預先編譯的Linux與macOS CLI

經GitHub Actions執行驗證的CLI檔案位於下列資料夾。執行CLI不需要安裝Rust、Python或.NET。Linux版本適用於x86_64/glibc；macOS請選擇符合CPU的版本。

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

請在儲存庫根目錄執行指令。重新散布時請保留LICENSE、LICENSE.ja、BINARY_NOTICES.md及licenses/。NATIVE_BINARIES.json記錄雜湊值、相依項目、原始碼版本與原生執行驗證結果。

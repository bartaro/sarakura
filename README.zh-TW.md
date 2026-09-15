# SARAKURA

[English](README.md#english) | [日本語](README.md#japanese) | **繁體中文**

**開啟 SARAKURA 繁體中文手冊**

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

- 繁體中文手冊
- [英文手冊](https://bartaro.github.io/kitaq-docs/en/sarakura.html)／[日文手冊](https://bartaro.github.io/kitaq-docs/sarakura.html)
- [可供離線閱讀的手冊原始檔](https://github.com/bartaro/kitaq-docs)
- [授權條款](LICENSE)／[日文參考譯文](LICENSE.ja)

本專案的授權不取代第三方對字型、相依套件、標誌或商標訂定的條件。再散布時，請一併保留隨附聲明。

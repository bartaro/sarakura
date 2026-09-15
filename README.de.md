# SARAKURA

[English](README.md#english) | [日本語](README.md#japanese) | **Deutsch**

**Deutsches Handbuch zu SARAKURA öffnen**

Werkzeug zur Diagnoseauswertung und Erstellung englischsprachiger Berichte für KITAQGB/KOKURA und KITAQFC/KUROSAKI.

Öffentliche Vorabversion: APIs und Verhalten können sich noch ändern.

## Fertige Windows-Version

Im Stammverzeichnis des Repositorys liegt `sarakura.exe`, als Release für Windows x64 kompiliert. Das Programm wird über die Kommandozeile bedient. Zum Ausführen benötigen Sie weder Rust noch Python oder .NET. Laden Sie das Repository als ZIP herunter, damit die ausführbare Datei und die Lizenzhinweise zusammenbleiben. Die native C-Laufzeitbibliothek ist statisch eingebunden; das Programm verwendet System-DLLs von Windows.

```powershell
.\sarakura.exe --help
```

Mit `.\scripts\build.ps1` bauen Sie ausschließlich dieses Kommandozeilenprogramm neu; bei Bedarf ergänzen Sie `-Offline`. Das Skript kopiert die ausführbare Datei ins Stammverzeichnis. Grafische Oberflächen, Python-Erweiterungsmodule und DLLs der C-API gehören nicht zu diesem Paket ausführbarer Programme. Einzelheiten finden Sie im [Build-Nachweis](BINARY_BUILD.json) und in den [Lizenzhinweisen zu den Binärabhängigkeiten](BINARY_NOTICES.md).

Lizenzgeber des eigenständigen Projektcodes ist **DAISUKE OBA**. Dieser Code steht unter der MIT-Lizenz. Geben Sie bei einer Weiterverteilung `LICENSE`, `LICENSE.ja`, `BINARY_NOTICES.md` und das Verzeichnis `licenses/` mit. Für Bibliotheken Dritter gelten deren jeweilige Rechteinhaber und Lizenzbedingungen.

## Selbst kompilieren und starten

Verwenden Sie eine aktuelle stabile Rust-Toolchain. Installieren Sie unter Windows außerdem Visual Studio Build Tools mit dem Workload für die Desktopentwicklung mit C++.

```powershell
.\scripts\build.ps1
.\sarakura.exe --help
```

## Handbücher und Lizenzen

- Deutsches Handbuch
- [Englisches Handbuch](https://bartaro.github.io/kitaq-docs/en/sarakura.html) / [Japanisches Handbuch](https://bartaro.github.io/kitaq-docs/sarakura.html)
- [Handbuchquellen zum Offline-Lesen](https://github.com/bartaro/kitaq-docs)
- [Lizenz](LICENSE) / [Japanische Übersetzung zur Orientierung](LICENSE.ja)

Die Projektlizenz ersetzt keine Bedingungen Dritter für Abhängigkeiten, Logos oder Marken. Bewahren Sie bei einer Weiterverteilung die beiliegenden Hinweise auf.

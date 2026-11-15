# SARAKURA

[English](README.md#english) | [日本語](README.md#japanese) | [한국어](README.ko.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md) | **Français** | [Español](README.es.md) | [Português (Brasil)](README.pt-BR.md) | [Deutsch](README.de.md)

**[SARAKURA · Manuel HTML](https://bartaro.github.io/kitaq-docs/fr/sarakura.html)**

Analyse des diagnostics et production de rapports en anglais pour KITAQGB/KOKURA et KITAQFC/KUROSAKI.

Version publique préliminaire : les API et leur comportement peuvent évoluer.

## Exécutable pour Windows

La racine du dépôt contient `sarakura.exe`, compilé en mode Release pour Windows x64. Il s'agit d'un programme en ligne de commande. Son exécution ne nécessite ni Rust, ni Python, ni .NET. Téléchargez l'archive ZIP du dépôt pour conserver ensemble l'exécutable et les mentions de licence. La bibliothèque d'exécution C native est liée statiquement ; le programme utilise des DLL système de Windows.

```powershell
.\sarakura.exe --help
```

Pour reconstruire cet outil seul, utilisez `.\scripts\build.ps1`, avec l'option `-Offline` au besoin. Le script copie l'exécutable à la racine du dépôt. Les interfaces graphiques, modules d'extension Python et DLL de l'API C ne font pas partie de cette distribution d'exécutables. Consultez les [mentions relatives aux dépendances de l'exécutable](BINARY_NOTICES.md).

Le code propre au projet est proposé sous licence MIT par **DAISUKE OBA**. Lors d'une redistribution, joignez `LICENSE`, `LICENSE.ja`, `BINARY_NOTICES.md` et le dossier `licenses/`. Les bibliothèques tierces conservent leurs propres titulaires de droits et conditions.

## CLI précompilées pour Linux et macOS

Leur exécution ne nécessite ni Rust, ni Python, ni .NET. La version Linux cible x86_64/glibc ; sous macOS, choisissez la version correspondant au processeur.

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

Exécutez les commandes à la racine du dépôt. Pour redistribuer les fichiers, conservez LICENSE, LICENSE.ja, BINARY_NOTICES.md et licenses/.

## Première utilisation

```powershell
.\sarakura.exe gb analyze --metadata examples/gb/e2e/metadata.json --events examples/gb/e2e/events.jsonl --out out/gb
```

Cet exemple contient volontairement une erreur et un avertissement. Ouvrez `report.html` dans le dossier de sortie pour les examiner.


## Compiler et commencer à utiliser l'outil

Pour recompiler, utilisez une version stable actuelle de Rust. Sous Windows, installez Visual Studio Build Tools avec la charge de travail de développement Desktop en C++.

```powershell
.\scripts\build.ps1
.\sarakura.exe --help
```

## Manuels et licences

- [SARAKURA · Manuel HTML](https://bartaro.github.io/kitaq-docs/fr/sarakura.html)
- [Manuel en anglais](https://bartaro.github.io/kitaq-docs/en/sarakura.html) / [Manuel en japonais](https://bartaro.github.io/kitaq-docs/sarakura.html)
- [Sources du manuel pour lecture hors connexion](https://github.com/bartaro/kitaq-docs)
- [Licence](LICENSE) / [Traduction japonaise à titre de référence](LICENSE.ja)

La licence du projet ne remplace pas les conditions des tiers relatives aux dépendances, logos ou marques. Conservez les mentions jointes lors de la redistribution.

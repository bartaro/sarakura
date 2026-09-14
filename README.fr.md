# SARAKURA

[English](README.md#english) | [日本語](README.md#japanese) | **Français**

**[Ouvrir le manuel de SARAKURA en français](https://bartaro.github.io/kitaq-docs/fr/sarakura.html)**

Analyse des diagnostics et production de rapports en anglais pour KITAQGB/KOKURA et KITAQFC/KUROSAKI.

Version publique préliminaire : les API et leur comportement peuvent évoluer.

## Exécutable pour Windows

La racine du dépôt contient `sarakura.exe`, compilé en mode Release pour Windows x64. Il s'agit d'un programme en ligne de commande. Son exécution ne nécessite ni Rust, ni Python, ni .NET. Téléchargez l'archive ZIP du dépôt pour conserver ensemble l'exécutable et les mentions de licence. La bibliothèque d'exécution C native est liée statiquement ; le programme utilise des DLL système de Windows.

```powershell
.\sarakura.exe --help
```

Pour reconstruire cet outil seul, utilisez `.\scripts\build.ps1`, avec l'option `-Offline` au besoin. Le script copie l'exécutable à la racine du dépôt. Les interfaces graphiques, modules d'extension Python et DLL de l'API C ne font pas partie de cette distribution d'exécutables. Consultez le [compte rendu de compilation binaire](BINARY_BUILD.json) et les [mentions relatives aux dépendances de l'exécutable](BINARY_NOTICES.md).

Le code propre au projet est proposé sous licence MIT par **DAISUKE OBA**. Lors d'une redistribution, joignez `LICENSE`, `LICENSE.ja`, `BINARY_NOTICES.md` et le dossier `licenses/`. Les bibliothèques tierces conservent leurs propres titulaires de droits et conditions.

## Compiler et commencer à utiliser l'outil

Pour recompiler, utilisez une version stable actuelle de Rust. Sous Windows, installez Visual Studio Build Tools avec la charge de travail de développement Desktop en C++.

```powershell
.\scripts\build.ps1
.\sarakura.exe --help
```

## Manuels et licences

- [Manuel en français](https://bartaro.github.io/kitaq-docs/fr/sarakura.html)
- [Manuel en anglais](https://bartaro.github.io/kitaq-docs/en/sarakura.html) / [Manuel en japonais](https://bartaro.github.io/kitaq-docs/sarakura.html)
- [Sources du manuel pour lecture hors connexion](https://github.com/bartaro/kitaq-docs)
- [Licence](LICENSE) / [Traduction japonaise à titre de référence](LICENSE.ja)

La licence du projet ne remplace pas les conditions de tiers relatives aux polices, dépendances, logos ou marques. Conservez les mentions jointes lors de la redistribution.

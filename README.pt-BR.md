# SARAKURA

[English](README.md#english) | [日本語](README.md#japanese) | **Português (Brasil)**

**Abrir o manual de SARAKURA em português**

Análise de diagnósticos e geração de relatórios em inglês para KITAQGB/KOKURA e KITAQFC/KUROSAKI.

Versão de prévia pública: as APIs e o comportamento podem mudar.

## Executável para Windows

A raiz do repositório inclui `sarakura.exe`, compilado em modo Release para Windows x64. É um programa de linha de comando. Não é necessário instalar Rust, Python ou .NET para executá-lo. Baixe o ZIP do repositório para obter o executável junto dos avisos de licença. O runtime C nativo é vinculado estaticamente; o programa utiliza DLLs de sistema do Windows.

```powershell
.\sarakura.exe --help
```

Para recompilar apenas esta ferramenta, use `.\scripts\build.ps1`, com `-Offline` se necessário. O script copia o executável para a raiz do repositório. Interfaces gráficas, módulos de extensão Python e DLLs da API C não fazem parte desta distribuição de executáveis. Consulte o [registro da compilação binária](BINARY_BUILD.json) e os [avisos das dependências do executável](BINARY_NOTICES.md).

O licenciante do código próprio do projeto é **DAISUKE OBA**, sob a licença MIT. Na redistribuição, inclua `LICENSE`, `LICENSE.ja`, `BINARY_NOTICES.md` e a pasta `licenses/`. As bibliotecas de terceiros mantêm seus próprios titulares de direitos e condições.

## Compilar e começar a usar

Para recompilar, use uma versão estável atual do Rust. No Windows, instale o Visual Studio Build Tools com a carga de trabalho de desenvolvimento para desktop com C++.

```powershell
.\scripts\build.ps1
.\sarakura.exe --help
```

## Manuais e licenças

- Manual em português
- [Manual em inglês](https://bartaro.github.io/kitaq-docs/en/sarakura.html) / [Manual em japonês](https://bartaro.github.io/kitaq-docs/sarakura.html)
- [Fontes do manual para leitura offline](https://github.com/bartaro/kitaq-docs)
- [Licença](LICENSE) / [Tradução de referência em japonês](LICENSE.ja)

A licença do projeto não substitui as condições de terceiros relativas a fontes de caracteres, dependências, logotipos ou marcas. Preserve os avisos incluídos ao redistribuir.


<!-- native-platform-binaries-20261004-pt-BR -->
### CLI pré-compiladas para Linux e macOS

As CLI verificadas por execução no GitHub Actions estão nas pastas abaixo. Não é necessário instalar Rust, Python ou .NET para executá-las. A versão Linux é destinada a x86_64/glibc; no macOS, escolha a versão correspondente à CPU.

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

Execute os comandos na raiz do repositório. Ao redistribuir os arquivos, mantenha LICENSE, LICENSE.ja, BINARY_NOTICES.md e licenses/. NATIVE_BINARIES.json contém hashes, dependências, revisões do código-fonte e resultados das verificações nativas.

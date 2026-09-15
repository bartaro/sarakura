# SARAKURA

[English](README.md#english) | [日本語](README.md#japanese) | **Español**

**Abrir el manual de SARAKURA en español**

Analiza los diagnósticos de KITAQGB/KOKURA y KITAQFC/KUROSAKI y genera informes en inglés.

El proyecto está en fase de versión preliminar pública. Las API y su comportamiento pueden cambiar.

## Ejecutable para Windows

La raíz del repositorio incluye `sarakura.exe`, compilado en modo Release para Windows x64. Es un programa de línea de comandos: para ejecutarlo no hace falta instalar Rust, Python ni .NET. Descargue el ZIP del repositorio para conservar juntos el ejecutable y los avisos de licencia. La biblioteca de ejecución de C está enlazada estáticamente; el programa utiliza DLL del sistema de Windows.

```powershell
.\sarakura.exe --help
```

Para volver a compilar solo esta herramienta, ejecute `.\scripts\build.ps1` y añada `-Offline` si necesita trabajar sin conexión. El script copia el ejecutable a la raíz del repositorio. Esta distribución de ejecutables no incluye interfaces gráficas, módulos de extensión de Python ni DLL de la API C. Consulte el [registro de compilación del binario](BINARY_BUILD.json) y los [avisos de licencia de sus dependencias](BINARY_NOTICES.md).

El código propio del proyecto se ofrece bajo la licencia MIT, cuyo licenciante es **DAISUKE OBA**. Al redistribuirlo, incluya `LICENSE`, `LICENSE.ja`, `BINARY_NOTICES.md` y el directorio `licenses/`. Las bibliotecas de terceros conservan sus respectivos titulares y condiciones de licencia.

## Compilar el código fuente y empezar a usar la herramienta

Para recompilar, utilice una versión estable reciente de Rust. En Windows también necesita Visual Studio Build Tools con la carga de trabajo «Desarrollo para el escritorio con C++».

```powershell
.\scripts\build.ps1
.\sarakura.exe --help
```

## Manuales y licencias

- Manual en español
- [Manual en inglés](https://bartaro.github.io/kitaq-docs/en/sarakura.html) / [Manual en japonés](https://bartaro.github.io/kitaq-docs/sarakura.html)
- [Archivos del manual para consultarlo sin conexión](https://github.com/bartaro/kitaq-docs)
- [Licencia](LICENSE) / [Traducción japonesa de referencia](LICENSE.ja)

La licencia del proyecto no sustituye las condiciones de terceros sobre tipografías, dependencias, logotipos o marcas. Conserve los avisos adjuntos al redistribuir el software.

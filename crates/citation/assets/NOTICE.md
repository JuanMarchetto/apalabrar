# Third-party assets — CSL styles and locales

The XML files in `assets/styles/` and `assets/locales/` are distributed under the **Creative Commons
Attribution-ShareAlike 3.0 Unported License (CC-BY-SA 3.0)**. They are NOT MIT-licensed; they are
included as data assets, not as source code, under the terms of that license.

## Sources

- **Styles**: [citation-style-language/styles](https://github.com/citation-style-language/styles)
  - `apa.csl` — APA 7th edition
  - `ieee.csl` — IEEE
  - `mla.csl` — Modern Language Association 9th edition
  - `ama.csl` — American Medical Association
  - `chicago-notes-bibliography.csl` — Chicago Manual of Style, notes-bibliography variant
- **Locales**: [citation-style-language/locales](https://github.com/citation-style-language/locales)
  - `locales-en-US.xml`, `locales-en-GB.xml`
  - `locales-es-ES.xml`
  - `locales-pt-BR.xml`
  - `locales-de-DE.xml`, `locales-fr-FR.xml`
  - `locales-zh-CN.xml`, `locales-ru-RU.xml`

## License terms summary

CC-BY-SA 3.0 requires:

1. **Attribution** — credit the CSL project (this NOTICE file satisfies that).
2. **ShareAlike** — derivative works (modified styles or locales) must be distributed under
   CC-BY-SA-3.0-compatible terms. This does NOT apply to the Apalabrar source code that _uses_ these
   files; only to modifications of the files themselves.

Full license text: https://creativecommons.org/licenses/by-sa/3.0/

## What this means for downstream users

- The Apalabrar source code remains MIT-licensed.
- These XML data files retain CC-BY-SA 3.0.
- Distributing Apalabrar binaries that bundle these files is permitted; users must be able to access
  the same NOTICE.

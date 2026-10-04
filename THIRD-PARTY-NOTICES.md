# Third-party notices

WriterProgram uses the works below. Libraries built into the app keep their
own license texts in their packages (Cargo and npm); this file lists the ones
whose terms need saying here.

## hunspell-dict-ko (Korean spelling dictionary) — data, not part of the app

- Source: https://github.com/spellcheck-ko/hunspell-dict-ko
- Version used: 0.7.94, `ko-aff-dic-0.7.94.zip`
  (SHA-256 `c7252e2f6bf421e081a457ca007cd8a304ee2045947edba47b367db780e63c48`)
- Copyright (C) 2008-2019 hunspell-dict-ko contributors
- Copyright (C) 2017-2019 National Institute of Korean Language (국립국어원)
- License: the built dictionary files (`ko.aff`, `ko.dic`) are GNU GPL 3.0
  or later. Most word data is CC BY-SA 4.0 (words from the National
  Institute of Korean Language's dictionaries, originally CC BY-SA 2.0 KR);
  the generating code and some word data are MPL-1.1/GPL-2.0/LGPL-2.1.

The dictionary is not compiled into WriterProgram and is not in this
repository. When the writer turns spell checking on, the app asks first and
then fetches the release above from GitHub into its data folder
(`spelling/hunspell-dict-ko-0.7.94/`), checks the SHA-256, and keeps the
files unchanged with their `LICENSE.md`, `LICENSE.GPL-3` and a `NOTICE.txt`
next to them. The app reads them as separate data; the dictionary's own
license notes that a program and a Hunspell dictionary are separate works.
The screen shows the source and license in 보기 설정 → 맞춤법.

## spellbook (Hunspell-compatible spell checker)

- Source: https://github.com/helix-editor/spellbook
- License: Mozilla Public License 2.0. Used unchanged from crates.io; its
  source is available there.

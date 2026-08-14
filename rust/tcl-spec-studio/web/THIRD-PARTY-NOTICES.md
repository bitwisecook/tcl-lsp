# Third-party notices — the spec studio's editor bundle

The spec studio is AGPL-3.0-or-later (see the repository's [`LICENSE`](../../../LICENSE)).
Its **editor chunk** — `dist/assets/monaco-host.js`, the bundle loaded when you
open an editor tab — additionally contains the MIT-licensed third-party software
listed below, bundled from npm at build time by
[`build.mjs`](build.mjs). Nothing is fetched at run time and nothing comes from
a CDN: every byte the page loads is served from its own directory.

The versions are the ones resolved in [`package-lock.json`](package-lock.json);
bump this file when you bump those.

| Package | Version | Licence | Upstream |
|---|---|---|---|
| `monaco-editor` | 0.56.0 | MIT | <https://github.com/microsoft/monaco-editor> |
| `vscode-jsonrpc` | 9.0.1 | MIT | <https://github.com/microsoft/vscode-languageserver-node> |
| `vscode-languageserver-protocol` | 3.18.2 | MIT | <https://github.com/microsoft/vscode-languageserver-node> |
| `vscode-languageserver-types` | 3.18.0 | MIT | <https://github.com/microsoft/vscode-languageserver-node> |

`vscode-languageserver-types` is a transitive dependency of
`vscode-languageserver-protocol` and is bundled with it.

The codicon icon font shipped inside `monaco-editor` is embedded in the bundle's
stylesheet as a `data:` URI. It is licensed separately by Microsoft under
CC-BY-4.0 (see `node_modules/monaco-editor/README.md` and the upstream
[vscode-codicons](https://github.com/microsoft/vscode-codicons) repository).

## monaco-editor

```
The MIT License (MIT)

Copyright (c) 2016 - present Microsoft Corporation

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## vscode-jsonrpc, vscode-languageserver-protocol, vscode-languageserver-types

All three are published from `microsoft/vscode-languageserver-node` and carry
the same notice:

```
Copyright (c) Microsoft Corporation

All rights reserved.

MIT License

Permission is hereby granted, free of charge, to any person obtaining a copy of
this software and associated documentation files (the "Software"), to deal in
the Software without restriction, including without limitation the rights to
use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software is furnished to do so,
subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED *AS IS*, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
```

## What is *not* third-party

The language server the editor talks to is this project's own
`rust/tcl-lsp-server`, compiled to WebAssembly by `rust/tcl-lsp-server-wasm`
(`lsp/tcl_lsp_server_wasm_bg.wasm` in the dist) and covered by the repository's
AGPL-3.0-or-later licence, as are the registry/analyser wasm inlined in
`index.html` and every `.ts` file under `src/`.

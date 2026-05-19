# v1.10.7

## Bug Fixes

- **JetBrains plugin: Compiler Explorer tool window now receives source
  from the editor.** Since v1.10.0 the tool window stayed stuck on
  "Waiting for source from editor..." with the status line reading
  "webview API unavailable". The JCEF adapter in
  `CompilerExplorerHtml.kt` was rewriting a `const vscode =
  acquireVsCodeApi();` string that the VS Code webview HTML no longer
  contained — recent revisions capture the API via
  `var __vscodeApi = typeof acquireVsCodeApi === 'function' ?
  acquireVsCodeApi() : undefined` instead, so the replacement was a
  silent no-op and `acquireVsCodeApi` stayed undefined. The adapter now
  injects a `<head>` shim that defines `acquireVsCodeApi` to return a
  bridge object whose `postMessage` queues requests until the Kotlin
  host installs `window.__tcllspBridge` on load-end, at which point the
  queue is drained. Compile, hover-highlight, and clear-highlight
  messages reach the LSP server again.
- **JetBrains plugin: bundle LSP server outside the plugin jar.**
  v1.10.6 shipped the source-level W105 quick-fix that preserves `$`
  in variable references (`$script` → `{$script}` instead of
  `{script}`), but users running the JetBrains plugin didn't see it on
  upgrade: the plugin packed `tcl-lsp-server.pyz` into its jar and
  extracted it to `${tmpdir}/tcl-lsp-server.pyz` at first launch, and
  the existence check that gated re-extraction only fired when the
  temp file was missing or empty.  Plugin upgrades therefore kept
  reusing the previous version's extracted server, and v1.10.6's W105
  fix never reached anyone who'd already launched v1.10.5.  The pyz
  now lives at the plugin install root (next to `lib/`, matching
  JetBrains' own Prisma ORM plugin layout), so Python executes it
  directly from the install directory — no temp-dir cache, no
  upgrade-time invalidation, and v1.10.6's source-side fix actually
  takes effect on upgrade.

# v1.10.6

## Bug Fixes

- **Quick-fix range widening for unbraced-expression diagnostics.** The
  W100 (`expr` argument needs braces) and W101 (control-flow body needs
  braces) auto-fixes previously stopped one character short of the
  closing `"` / `}`, leaving a stray delimiter in the document after
  the rewrite. They also rewrote arguments using the *post-substitution*
  value, silently dropping `$var` / `[cmd]` references. Both now widen
  the replacement range correctly and preserve the original
  substitution syntax verbatim.
- **`matchclass` → `class match` quick-fix preserves substitutions.**
  IRULE2001's auto-rewrite was producing `class match url equals ::lib`
  from source `matchclass $url ::lib` — silently turning a variable
  reference into a literal. The rewrite now reads the raw token text
  so `$url` round-trips intact, and the fix range covers the closing
  delimiter.

## Improvements

- **JetBrains plugin: auto-restart on settings change** (#438). The
  LSP server now restarts automatically when the resolved Python path
  changes in settings, and the resolved interpreter is logged at
  startup so discovery issues are easier to diagnose.

## Internal

- **Release process now gates on CodeQL** (#440). A new
  `release-codeql-gate` Makefile target watches the CodeQL run for
  the tag candidate on `main` and blocks the release if any open
  alert is high or critical severity. Overridable via
  `CODEQL_GATE_MIN_SEVERITY=critical` for documented exceptions.

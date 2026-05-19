# v1.10.7

## Bug Fixes

- **JetBrains plugin: bundle LSP server outside the plugin jar.** The
  bundled `tcl-lsp-server.pyz` was packed into the plugin jar and
  extracted to `${tmpdir}/tcl-lsp-server.pyz` at first launch.  The
  existence check that gated re-extraction only fired when the cache
  was missing or empty, so plugin upgrades reused the previous
  version's server — users who upgraded v1.10.5 → v1.10.6 still saw
  the W105 quick-fix produce `{script}` from `$script` even though the
  source-side fix had shipped.  The pyz now lives at `<plugin>/tcl-lsp-server.pyz`
  in the distribution (next to `lib/`, matching JetBrains' own Prisma
  ORM plugin layout), so Python executes it directly from the install
  directory — no temp-dir cache, no upgrade-time invalidation.

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

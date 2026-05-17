# v1.10.2

## New Features

- **`tcl::prefix` command** — `all`, `longest`, and `match` (with `-exact`, `-error`, `-message`) implemented in the runtime, unlocking the `string-26.x / 27.x / 28.x` clusters.
- **`binary encode hex` / `binary decode hex`** — round-trip support for hex byte-arrays.
- **`namespace ensemble` stubs** — `create` / `configure` / `exists` with canonical error wording; dispatch still pending.
- **`array startsearch` / `nextelement` / `anymore` / `donesearch`** — real per-array search state with auto-invalidation on mutation, including SID round-trips that preserve the user-visible array name.
- **`interp hide` / `expose` for global builtins** — per-interp hidden-builtin marker so `interp hide list` works without cloning the static dispatch table.

## Improvements

- **WASM tcltest sweep at 100/101 passing bundles** (was ~50/101), with per-stem TOML manifests tracking every residual gap so regressions surface immediately.
- **`lsort` full option matrix** — `-ascii` / `-dictionary` / `-integer` / `-real` / `-nocase` / `-increasing` / `-decreasing` / `-unique` / `-indices` / `-stride` / `-index PATH`. `-unique` now dedupes using the active comparison mode.
- **`lsearch -index PATH` and `-subindices`** — list-of-indices traversal and full nested sub-position return.
- **NaN-aware `==` / `!=` in `expr`** — IEEE-754 "NaN is unordered" semantics via the new `tcl_expr_eq_nan_aware` runtime helper, used by both the codegen and the interp expression evaluator.
- **Unicode-aware string ops** — `string length`/`index`/`range`/`first`/`last` operate in codepoint coordinates; `compare`/`equal`/`tolower`/`toupper`/`totitle` decode codepoints (Latin-1, Latin Extended-A/B, Greek, Cyrillic); `string trim` follows Tcl 9's Unicode whitespace family; `string reverse` preserves multi-byte sequences and splits supplementary codepoints into surrogate pairs; `string is` and `string match` are codepoint-aware.
- **Surrogate-pair escape combining** — high+low surrogate escapes combine into a single supplementary-plane codepoint at parse time (matches Tcl's `TclParseBackslash`).
- **Namespace-aware variable storage** — `set X 99` inside `namespace eval ::A {…}` now creates `::A::X` rather than a root global. `global ::a::b::c` and `upvar` to the global frame both qualify correctly.
- **`info commands` / `info vars` alignment with Tcl 9** — qualified patterns return FQ names, unqualified patterns return simple names (info-4.3 / 4.4). `info procs` now includes compiled procs alongside interpreted bodies.
- **`namespace export` / `import` / `forget` fixes** — qualified-pattern rejection on export, self-import detection, and forget-by-source-namespace traversal.
- **`namespace which`, `namespace parent`, `namespace code`** — Tcl 9 dispatch wording for unknown options, qualified parent FQNs, and correct bare-form `namespace inscope` wrapping.
- **`namespace delete` cascade** — invalidates import redirects and tombstones command buckets so post-delete dispatch fails cleanly; bumps the cmd-ref epoch.
- **`array` dispatch overhaul** — per-subcommand rule table drives arity checks, prefix matching, and "isn't an array" gating; `array set arr LIST` validates even-length input and scalar-vs-array conflicts.
- **Variable diagnostics** — `unset` and `var_unset_error` pick the suffix based on the variable's shape (`no such variable` / `variable isn't array` / `variable is array` / `no such element in array`); `unset -nocomplain` parsed.
- **`string` arity checks + index validation + map balance** — canonical `wrong # args` and `bad index "X"` diagnostics across `first`, `last`, `range`, `index`, `replace`, `repeat`, `tolower`/`toupper`/`totitle`, plus `char map list unbalanced` for odd-length maps.
- **`split` walks codepoints when the split set contains multi-byte chars.**
- **`binary format` numeric specs** — consume one list argument (per Tcl 9) instead of many trailing words.
- **WASM 45s watchdog** — pre-empts runaway tcltest bundles via the wasmtime epoch-interrupt cap.

## Bug Fixes

- **Codegen scanner registers `tcl_expr_order_cmp` for `==` / `!=`** so the helper is wired up and the emitter takes the correct numeric-or-string compare branch (rather than reporting every pair of non-numeric strings as equal).
- **Interp expr evaluator** — string-aware `==` / `!=`, paren-wrapped equality recursion through `find_top_string_op`, and line-continuation (`\<newline>`) handling in brace-quoted multi-line `if` expressions.
- **`lindex` / `join` list-shape validation** — raises the canonical "list element in braces followed by …" / "unmatched open brace in list" errors instead of silently accepting malformed input.
- **`return -errorcode` list validation** — `return -code error -errorcode {{}a} eek` now raises `bad -errorcode value` instead of silently propagating.
- **`lsort_ascii_cmp` null-pointer guard** for empty list elements (fixes a ReleaseSafe panic in `set-old.test`).
- **Codegen concat cmd-subst** — `[set a 1][set b 2]` no longer mis-collapses; the whole-word cmd-sub branch only fires when the leading `[` actually closes at the trailing `]`.
- **`tcl::prefix`** — honour `-exact`, fix table-arg index after option parsing, plug per-element/per-iteration leaks, retain the source TclObj for `longest`.
- **`array set` scalar-conflict** — error message reports the real first key, not a hardcoded `a`. `array set X {}` on a scalar raises the distinct `can't array set "X"` form.
- **`binary encode/decode hex`** — own the result buffer via `obj_new_string_take` (fixes per-call leak), guard OOM, free in-flight buffers on invalid-hex bailouts.
- **`tcl_frames` qualified-var probes** — `qualify_alias_target` now checks existence before installing the redirect; OOM guards added to `var_set` / `var_exists` script-level paths.

## Editor Publishing

- **Sublime Text** — `make publish-sublime` now mirrors `build/sublime-stage` into the dedicated `bitwisecook/tcl-lsp-sublime-text` repo at the release tag (Package Control's `tags: true` discovery needs the package contents at the root of a git tag, which our monorepo can't satisfy directly). Honors `TCL_LSP_SUBLIME_DRY_RUN=1`.
- **Sublime package now bundles `tcl-lsp-server.pyz`** instead of a raw source tree — package size drops to ~4.2 MB (was ~5-10 MB) and unpacked cache from ~30-60 MB to ~5 MB, in line with the VS Code and JetBrains zipapp model.
- **Zed** — `make publish-zed` prepares a local checkout of `zed-industries/extensions`, advances the submodule, bumps the version, and stops with the suggested `gh pr create` commands. The script refuses to clobber a dirty or local-ahead checkout (override with `TCL_LSP_ZED_FORCE=1`).
- **`make publish-verify`** — pre-release readiness check that prints `[ok]` / `[warn]` / `[fail]` per editor target (VSCE PAT, JetBrains token, Sublime mirror reachability, Zed fork reachability).
- **JetBrains Marketplace** — README links the live listing (id 31801); `JETBRAINS_TOKEN` env var path documented.
- **`VSCE_PAT` env-var path** documented for `publish-vsix` (lets `vsce publish` consume the token directly, skipping interactive login).

## Breaking Changes

- **`info commands` / `info procs` output format aligned with Tcl 9** — unqualified patterns now return simple names (`foo`), qualified patterns return FQ names (`::ns::foo`). Callers that previously relied on always-FQ output from unqualified queries must adapt.
- **`info procs` now includes compiled procs** alongside interpreted bodies (only aliases and import redirects are excluded). Callers that depended on the compiled-proc filter must switch to a different predicate.
- **`expr {NaN == NaN}` now returns 0** (previously could return 1 depending on path). `expr {NaN != NaN}` returns 1.

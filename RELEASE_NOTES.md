# v1.10.0

## New Features

- **`f5 query` verb** — a jq-flavoured DSL for inspecting and rewriting parsed `bigip.conf` / SCF configurations. Pipelines, path subscripts, `select` / `map`, and the `=` / `|=` / `+=` / `-=` assignment operators run over the typed BIG-IP object tree. `PathRef` values auto-dereference on field access (so `.ltm.virtual[].pool.members[].address` walks VS → pool → member in one chain), identity-field writes auto-route through `rename_object` (rewriting every reference, including pool refs inside iRule bodies), and a custom `ip(network, source)` builtin rebases addresses preserving host bits and port. DSL help ships in three forms (`--help-dsl`, `--help-builtins [NAME]`, `--help-examples`) so the cookbook and registry can't drift from the implementation.
- **Typed BIG-IP registry** — every parsed BIG-IP property now carries a typed value spec, exposed through the new query layer and reused by the LSP for diagnostics, semantic tokens parity, document links, and value-aware code actions.
- **Per-workspace-folder configuration** — `dialect`, `extraCommands`, `libraryPaths`, and `style.nonAscii` can now be set per workspace folder, with `didChangeWorkspaceFolders` properly re-initialising per-folder configs.
- **Stub command system** — inline `# tcl-lsp: stub` declarations (and `<dialect>.tcl.stubs` files) now feed into the signature registry. Subcommand and optional-arg stubs (`stub db eval {sql ?rowvar? script:body}`) build dynamic arg-role resolvers so callbacks surface in the call graph regardless of argument shape. Verified end-to-end against real `libsqlite3-tcl`.

## Improvements

- **Call graph completeness** — command substitutions inside `if` / `while` / `for` conditions and `switch` subjects are now scanned, so `if {[q]} ...` no longer flags `q` as dead code or drops the call-graph edge. `ArgRole.BODY` arguments of embedded commands are scanned recursively, covering `if {[catch {p}]} ...` and similar nested forms. (#409)
- **Deep proc-arg trait inference** — `infer_param_traits_deep` now feeds the offline analytics paths (`tcl callgraph`, the compiler explorer, the MCP server), producing richer BODY / EXPR / VAR trait information. The LSP synchronous path stays on the shallow pass for latency.
- **BIG-IP port name resolution** — service-name to port-number mapping for BIG-IP config, used by both the parser and diagnostics.
- **`f5 diff` accepts `tmsh` output** — diffing a saved `tmsh list` snapshot against a config file now works without preprocessing.
- **Shell completions** — verb aliases dropped from `tcl` completions; missing `f5` verbs added.
- **Stub overlay caching** — proc / interproc caches mix a stub-overlay fingerprint into their keys, so adding or changing a stub correctly invalidates cached summaries.

## Bug Fixes

- `args` variadic tail no longer raises the minimum arity by one — `{first args}` is min 1 (correct Tcl semantics), `{args}` alone is min 0.
- Stub overlay keys are registered under both bare and `::`-qualified spellings, so call sites with either form hit the overlay.
- Word-building in script scanning now joins adjacent token fragments (`foo[bar]baz` is one word) and skips `TokenType.COMMENT`.
- Chunked and restore-based analyser paths (`analyse_chunked`, `analyse_commands`) now run under `stub_signature_scope`, so editor diagnostics see stub roles during incremental analysis.
- Stub parser bails on `stub expr-func` / `stub expr-op` lines so the subcommand regex doesn't mis-parse them.

## Other

- `scripts/` reorganised into `scripts/dev/` for development tooling.
- Installation guide split into separate editor and CLI documents.

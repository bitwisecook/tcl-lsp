# Lane: SslicTcl as a first-class authoring dialect

Issue [#1543](https://github.com/bitwisecook/tcl-lsp/issues/1543), epic #1524.

## Goal

Add **SslicTcl** — the declarative `.sslictcl` TLS-assurance DSL parsed by
`rust/tcl-sslictcl/src/dsl.rs` — as a first-class *authoring dialect*, wired
exactly the way the SpecTcl `.tclspec` dialect is, so a `.sslictcl` document
gets registry-driven completion, hover, signature help, semantic tokens,
folding, and document symbols with **no** declaration-name special case in any
LSP consumer.

## The ruling this lane implements

Per the design ruling on #1543:

- `CommandSpec` is **not** extended. Everything the vocabulary needs is
  expressible with the existing fields.
- SslicTcl is an **environment** (package surface `sslictcl`) over Tcl 9.0,
  exactly like `spectcl` — not a grammar axis. A `.sslictcl` document is
  ordinary Tcl syntax; only its *availability* half is different.
- The vocabulary **evaluates nothing**, not even a `predicate { … }` body,
  which the loader retains verbatim.
- Base Tcl stays loaded underneath: the grammar is what says a word is not an
  SslicTcl declaration.

## Deviations from the lane brief, and why

- **`expr_grammar_base` is `Some(V9_0)`, not `None`.** The brief preferred
  `None` because the vocabulary evaluates nothing. The profile invariant
  `expr_grammar_base_equals_runtime_base` (`profile.rs`) forbids it: the field
  is derived from `runtime_base`, not chosen, and `runtime_base` must be
  `Some(V9_0)` because base Tcl really is loaded underneath. `None` would have
  required `runtime_base: None` too, which would then have tripped
  `octal_policy_is_derived_from_the_runtime_base`,
  `vm_runtime_version_tracks_the_profile_runtime_base`, and
  `lexer_grammar_follows_the_runtime_base`. The profile therefore mirrors
  SpecTcl exactly, with the reasoning recorded at the field.
- **`chain` and `policy` carry arity `1..=2`.** They are the only two words
  that are both a top-level declaration (`chain NAME { … }`) and a *reference*
  inside an `endpoint` (`chain NAME`). One spec covers both: the static
  `Body` role at index 1 is dropped by `arg_indices_for_role`'s
  `retain(idx < args.len())`, so a bare reference claims no block. Inside an
  `endpoint` the member row is `keyword_only`, so the walker never looks for a
  block there at all.
- **The lsp-core context-sensitivity test contrasts `sslictcl` with plain
  Tcl**, not top-level-versus-nested within `sslictcl`. Member words are
  registered specs (that is what buys them hover and completion), so under the
  `sslictcl` profile `hostname` at top level does resolve to a keyword — the
  same is true of SpecTcl's `arity`, and its own test makes the same contrast.
  Grammar membership is what makes the vocabulary context-sensitive for
  *folding and recursion*, which is what the design rests on.

## Site inventory

Done:

- `tcl-dialect`: `SpecSurface::SSLICTCL`; the `sslictcl` `DialectProfile` row
  (catalogue widened to 19) and `KNOWN_DIALECTS`; `sslictcl_environment()`,
  the `sslictcl` editor identity in `EditorLanguageIdentityId::CONTRIBUTED`,
  and the three environment tests.
- `tcl-registry` detection: `TCL_SOURCE_EXTENSIONS`, `CONTENT_SIGNATURES`
  (first, most-specific row), and four `tests/detect_dialect.rs` tests.
- `tcl-registry` pack: `commands/sslictcl/{mod,blocks,rows,values}.rs`,
  `shared_group!(sslictcl_specs, …)` and the `SurfaceLayer::Package("sslictcl")`
  arm, `VENDOR_SURFACE_BRIDGE` / `VENDOR_SURFACE_PACKAGES` / `PROBES`, the
  breadth counts in `model/{surface,context}.rs`, and `LOADABLE_DIALECTS`.
- `tcl-registry` definer: `DefinerFamily::SslicTcl`, twelve
  `SSLICTCL_*_GRAMMAR` consts, `SSLICTCL_GRAMMARS`.
- Consumers whose `DefinerFamily` match had to widen:
  `tcl-compiler/src/signature_scan/walker.rs` and
  `tcl-compiler/tests/signature_scan.rs`.
- Other enumerated-dialect lists: `tcl-spectcl` `DIALECT_SURFACES`,
  `tcl-spec-studio` `dialect_constant` + `CONSTANTS`, `xtask`
  `gen_editor_dialects` tests, `ai/prompts/manifest.json`.
- `tests/sslictcl_pack.rs`, including a `VOCABULARY` table test that pins the
  complete `(statement → members)` map.

Remaining: codegen regeneration and hand-maintained editor prose; lsp-core
semantic-token tests; docs.

## Interfaces the other lanes depend on

- Package surface name: `sslictcl`. Profile / environment id: `sslictcl`.
  Aliases: `sslic-tcl`, `tls-sslictcl`. Editor language id: `sslictcl`.
  Extension: `sslictcl` ("SslicTcl TLS Declaration").
- Hover `source` line on every spec:
  `SslicTcl (docs/design/sslictcl-vocabulary.md)`.
- The vocabulary table is `VOCABULARY` in
  `rust/tcl-registry/tests/sslictcl_pack.rs` and
  `docs/design/sslictcl-vocabulary.md`; the loader lane implements the same
  table, and the LSP lane cross-checks the two.

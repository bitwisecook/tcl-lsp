# Lane: SslicTcl LSP authoring end to end (#1543, epic #1524)

## Goal

Finish issue #1543. Two earlier lanes on this branch already landed the
`.sslictcl` loader (`rust/tcl-sslictcl`) and the `sslictcl` authoring
*dialect* (profile, environment, detection, registry pack,
`DefinerFamily::SslicTcl`, regenerated editor catalogues). What remains is
the LSP surface an author actually sees:

1. Publish `tcl_sslictcl::dsl::load_with_diagnostics` findings as ordinary
   document diagnostics, with precise UTF-16 ranges.
2. Make the analyser leave a well-formed `.sslictcl` document alone
   (no unknown-command noise on extension words, no W127 on `enabled TRUE`,
   no analysis of a `predicate { … }` body) — with a dialect-level policy,
   never a declaration-name check.
3. A native e2e suite `rust/tcl-lsp-server/tests/e2e/sslictcl.rs`.
4. A VS Code mocha test over a `.sslictcl` fixture.
5. Owner-map rows for `tcl-sslictcl` in the shared-utility contract.
6. Docs.

## Design decisions

### Diagnostics are a projection, not a second validator

`DslDiagnostic` already carries a published `tcl_core_types::DiagCode`, a
`DslSeverity`, a `tcl_lexer::Span` of absolute byte offsets, and a message —
which is precisely the shape of `tcl_compiler::analyser::Diagnostic`. So the
new provider `rust/tcl-lsp-core/src/sslictcl_diagnostics.rs` maps one to the
other and stops there:

* `diagnostics(source, disabled, suppressed) -> Vec<analyser::Diagnostic>`
  runs the loader, drops codes the user disabled
  (`tclLsp.diagnostics.<CODE> = false`) and codes suppressed by `# noqa` /
  `# tcl-lsp: disable=…`, and maps `Error`/`Warning`/`Hint` across.
* the server then lifts them with its existing
  `lift_analyser_diagnostics`, which is what gives them the same
  `source: "tcl-lsp"`, the same code spelling (`DiagCode::as_str`), the same
  `finalise_diagnostics` tag/severity-override treatment, and — the point —
  the same UTF-16-correct `lift_span` conversion every other diagnostic gets.

No new lift machinery, no new wire shape, and nothing in the module names a
declaration.

### Routing is by resolved authoring surface, not by dialect name

`applies_to(&DialectProfile)` asks
`document_context_for_profile(profile).authoring_query().packages` for the
`sslictcl` package — the exact shape `Backend::is_bigip_dialect` uses for
`bigip`. Aliases (`sslic-tcl`, `tls-sslictcl`) and every detection route
(extension, editor language id, the `sslictcl VERSION` content signature)
fold into the same answer.

The server branch is the `xc_for_irules` shape, in both diagnostic surfaces:

* push — `refine_and_lift_diagnostics` (`rust/tcl-lsp-server/src/lib.rs`),
  inside the existing `spawn_blocking` worker, after the XC block.
* pull — `Backend::full_diagnostics_for`, same position.

Both were required: the pull path recomputes its report rather than reading
the push cache, so a single wiring point would have left pull-mode editors
short.

The loader parses the *analysis* form of the buffer (`normalise_lone_cr`) and
the spans lift against the client's buffer; the rewrite preserves byte
length, so the two agree on every offset.

### The analyser: the loader supersedes W123, and nothing else moved

Measured first, with `tcl diag samples/sslictcl/example.sslictcl`. The sample
drew exactly five diagnostics, all `W123 Unknown command` on the five
extension words — `catalogue-owner` (twice), `renewal-window`, `site-owner`,
`deployment-note`. Nothing else: `enabled TRUE` draws no W127 (the earlier
lane already stopped the pack closing the case-insensitive value domains),
and a `predicate { … }` body draws nothing at all (its statement carries no
grammar, so the walker never descends it). `hostname a b c` still draws
`E003`, which is right and must survive.

So the whole of item 2 is the W123 collision, and the fix is one dialect
rule, `sslictcl_diagnostics::SUPERSEDED_ANALYSER_CODES`:

> W123 asks "is this word a command that exists?". The question needs a word
> in head position of a script that will be **evaluated**, and the defining
> property of this environment is that its documents never are. An
> unrecognised word is an unknown *declaration*, and the loader already says
> so with better information — `SSLIC1101` where an open block preserves it,
> `SSLIC1007` where a closed block rejects it.

Publishing both put two hints on one word that disagreed about what it was,
and the W123 one carried a "did you mean 'signature-schemes'?" quick-fix that
would have rewritten a deliberately-preserved extension into a declaration.

Alternatives rejected:

* **`ScopedCommandEnv::allow_unknown_commands`** (hanging a `body_scope` on
  the open blocks) — registry data, and the closest existing hook, but it
  reaches only *block bodies*. The top level is open too, and a rule that
  needs a second mechanism for half its cases is not one rule.
* **A `CommandDomainWidening` arm** in the analyser — the right *effect*
  (nothing is provably absent) reached through a false statement: a widening
  means "commands may exist that the walk cannot see", and here the truth is
  that no word is a call at all.
* **A `never_evaluated` bit on `EnvironmentPolicy`** — the most honest model,
  and where this belongs if a second never-evaluated environment ever
  appears; not taken now because `tcl-dialect` is out of this lane's scope
  and one environment does not justify a new policy axis.

Applied at the three places a verdict reaches a user, so they cannot
disagree: `refine_and_lift_diagnostics` (push),
`published_analyser_diagnostics` (pull **and** `textDocument/codeAction`,
which lifts its quick-fixes from that one set), and `tcl diag` / `tcl lint`
(`rust/tcl-cli/src/commands/diag.rs`), whose own doc comment promises the CLI
and the editor report the same set. The CLI gained the loader projection in
the same place, for the same reason.

## Site inventory

| Item | Status |
|---|---|
| `rust/tcl-lsp-core/Cargo.toml` — `tcl-sslictcl` dependency | done |
| `rust/tcl-lsp-core/src/sslictcl_diagnostics.rs` + `lib.rs` module row | done |
| Push wiring (`refine_and_lift_diagnostics`) | done |
| Pull wiring (`full_diagnostics_for`) | done |
| wasm / wasi `cargo check` | done — all three pass unchanged, no feature gating needed |
| Analyser interaction (item 2) | done |
| `tcl diag` parity (loader findings + supersession) | done |
| server `lib.rs` language-id row + `dialect_for_open` routing test | done |
| `tests/e2e/sslictcl.rs` + `tests/e2e.rs` row | remaining |
| VS Code mocha test + fixture | remaining |
| Owner-map rows + `AGENTS.md` row | remaining |
| Docs (`docs/capabilities.md`, READMEs, vocabulary doc) | remaining |

## Open uncertainties

* ~~Whether the crypto crates behind `tcl-sslictcl` build for wasm.~~
  Settled: `cargo check -p tcl-lsp-server --lib --target wasm32-wasip1`,
  `cargo check -p tcl-lsp-server-wasm --target wasm32-unknown-unknown` and
  `cargo check -p tcl-lsp-server-wasi --target wasm32-wasip1` all pass with
  the plain dependency. No `crypto` feature was needed; the crate's existing
  `getrandom` js / wasm_js target rows already cover the browser target.
* The brief asks for a test that "`tclLsp.diagnostics.exclude` with
  `SSLIC1101` suppresses only that code". `tclLsp.diagnostics.exclude` is a
  **file glob** list (#1556), not a code list; the per-code switch is
  `tclLsp.diagnostics.<CODE>: false`. Both are covered by the e2e suite,
  under their real meanings.

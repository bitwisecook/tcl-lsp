# Rust port of the `tcl` and `f5` CLIs

Status and roadmap for porting the two Python console scripts —
`tcl` (`tooling/tcl/main.py`) and `f5-query` (`tooling/f5/main.py`) — to native
Rust binaries. Part of the broader Rust rewrite (`docs/rust-rewrite.md`).

End goal: `tcl` and `f5-query` ship as pure-Rust binaries with the Python CLI
glue deleted; behaviour stays byte-for-byte compatible (scripted/piped output),
verified by golden differential tests.

## Crate layout (under `rust/`)

| Crate | Role |
|---|---|
| `tcl-cli` | bin `tcl` — clap command tree + verb dispatch (thin shell) |
| `f5-cli` | bin `f5-query` — clap command tree + verb dispatch (thin shell) |
| `tcl-cli-support` | shared plumbing: input resolution, output writers, per-dialect registry cache, syntax highlighter, and the `chrome` module |
| `tcl-bigip-io` | f5 input layer: UCS archives (gzip-tar + OpenPGP-symmetric decrypt, pure Rust, no gpg), the `read_path`/`load_paths` resolver, and passphrase resolution. Pure crypto core (bytes in → SCF/bytes out, all in-memory); file/stdin I/O isolated to its `paths` module |
| `tcl-bigip-query` | f5 query DSL (`dialects/f5/query`): the jq-flavoured language powering `f5 query`. **Front-end + value model + output + evaluator + jq-core builtins landed** (golden-tested over external-JSON roots); projection over the typed BIG-IP model, the remaining ~180 builtins, edit-plan, probes, side-inputs, renderers, and the verb wiring still to come. Pure, I/O-free (typed in → typed out) |

Existing engine crates reused: `tcl-lexer`, `tcl-syntax`, `tcl-registry`,
`tcl-compiler` (lowering/CFG/codegen/optimiser/analyser/segmenter),
`tcl-lsp-core` (formatting, minify), `tcl-bigip` (BIG-IP model + config parser).

### Architecture principle (confirmed in review)

- **Per-subsystem reusable library crates** with **pure, UI-agnostic,
  PyO3-friendly APIs** (typed in → typed out, no file I/O / stdout in the core).
  The bins are thin clap + I/O shells.
- Where a pure core already exists (`tcl_lsp_core::{minify,formatting}`,
  `tcl_compiler::optimiser`, `tcl_registry`), call it directly rather than
  duplicating.
- **PyO3: structure now, bind later.** Design the typed APIs to be wrappable;
  defer building wheels.

### Chrome (terminal styling) — `tcl_cli_support::chrome`

`anstream` + `anstyle` + `tabled`. Auto-detects TTY, honours
`NO_COLOR` / `CLICOLOR_FORCE`, strips ANSI when piped.

> **Rule:** chrome drives stderr / error messages / new decorative surfaces
> ONLY — never the byte-parity verb *stdout*. Piped output stays plain so
> golden tests and scripted consumers remain byte-stable.

Helpers: `eprint_error`, `eprint_status`, the style palette
(`error/warn/success/heading/dim`), and `render_table` (one rounded-border house
style for tabular verbs).

## The core insight: wiring vs engines

The CLI port is mostly thin wiring. **Byte-parity is gated by the completeness
of the underlying Rust engine**, not by the CLI code:

- Fully-ported engines (formatter, minifier, registry, lexer, segmenter) →
  the verb reaches **byte-for-byte parity**.
- In-progress engines (analyser, optimiser, lowering/VM-compiler, BIG-IP
  ref-graph) → the verb is **correctly wired** but its output inherits the
  engine's current gaps. It converges on Python automatically as the engine
  does. These are tracked as separate workstreams.

## `tcl` verb status

| Verb(s) | Status | Notes |
|---|---|---|
| `format` (`fmt`) | ✅ byte-parity | golden-tested |
| `minify` (`min`), incl. `--compact` | ✅ byte-parity | golden-tested |
| `unminify-error` (`umerr`) | ✅ parity | |
| `command-info` (`cmd-info`) | ✅ byte-parity (tcl) | golden-tested. Gap: iRules `validEvents` (needs event/profile-registry walk) |
| `highlight` (`hl`), ANSI + HTML | ✅ byte-parity | golden-tested; also powers `--colour` for format/minify/opt. Minor gap: `{*}` expansion marker |
| `completion` | ✅ idiomatic | clap_complete (not byte-identical to argcomplete, by design) |
| `diag` / `lint` / `validate` | ✅ wired | output tracks the **analyser**. A differential parity harness (`scripts/dev/diag_parity/`) runs both engines over a corpus across `{tcl8.6, f5-irules}` and ranks every `(code × divergence-kind)` gap — the regression oracle for closing them. **Analyser-parity landed (find-legacy codes):** (1) **nested-substitution recursion** — the per-command EXPR walk (`dispatch_expr_arguments`: W100/W110/W114/W003) now runs on commands descended from a `[…]` substitution, both bare `Cmd` args (`set y [expr $a + $b]`) and inside a *braced* expr arg (`if { [matchclass …] }`), via the new `run_nested_expr_diagnostics` + `dispatch_nested_segment` mirroring Python's `_recurse_expression_subcommands`; previously only the syntactic half reached substitutions, so nested `expr` escaped W100; (2) **IRULE2001** (`matchclass` deprecated) was genuinely absent — added `emit_irule2001_matchclass`, fired alongside IRULE2002 at the same head span (Python emits both); (3) **W304** now resolves its option-terminator profile dialect-agnostically (`DialectSet::empty()`), matching Python's `REGISTRY.resolve_option_terminator(cmd, args)` so it still fires on a dialect-disabled command (`exec`/`glob` under f5-irules, which also draw W002/W123). **Phase 3 parity landed:** (4) **W002** now fires for commands disabled in the active dialect but known in *another* (an iRules command like `when`/`session` under tcl8.6) — added `CommandRegistry::known_in_any_dialect`, a dialect-agnostic existence table (the `taint_source` precedent) replacing the loaded-dialect-only `get(bare).is_none()` test; (5) **W210/W211/W220 span anchoring** — these now anchor at the variable-name column via `narrow_to_assigned_name` (W211/W220 assignment target) and `narrow_to_read_var` (W210 read), re-lexing the command slice like Python's `narrow_to_variable`, instead of the command-start column; (6) **W210 is now emitted once per variable** at the earliest read (Python iterates the per-variable `read_before_set`), not once per use. (7) **W110 unbraced-`expr` over-fire fixed** — the multi-arg `expr $a == "x"` form now parses `" ".join(args)` (the substituted, quote-stripped word values, like Python) rather than the source slice, so `"x"` is a bareword not an `ExprString` and W110 no longer fires on the unbraced form (W100 still does); the braced `expr {$a == "x"}` still fires W110. Together these cut the committed-corpus `WRONG_POSITION` from 17→1 and `W210 EXTRA_FIRE` from 10→2. **Rust-is-more-correct cases are documented for Python fixes** (`scripts/dev/diag_parity/python-fixes.md`): e.g. Rust's I230 constant-fold handles `==`/`!=` on string operands, Python only `eq`/`ne` — fix Python, do not regress Rust. **Intentional divergence (not a defect): diagnostic ordering.** Python emits in pass order (per-command walk, then the *per-function* CFG/SSA passes I230→W220→H300→W210→W211→…); Rust applies one global source-position sort (diagnostics.rs ~8701) — **deliberately**, because it is required for the fuzzer-enforced `incremental == fresh` guarantee (post-walk emitters iterate non-deterministic `HashMap`s) and is a saner source-ordered LSP/CLI contract. No consumer depends on Python's pass order: editors re-sort, `find-legacy`'s six codes are all command-walk codes that already order identically, and `minimize` is membership-only. The parity harness reports order divergence as *informational*, excluded from the defect totals. **Intentional divergence (not a defect): iRules constructs under a non-iRules dialect.** `when` (and other iRules-only commands) are not tcl commands — under a plain-tcl dialect they are unknown, would-be user-defined commands whose braced `{…}` is an ordinary string argument, *not* a script. Rust keeps body-role resolution dialect-scoped, so it does **not** recurse into a `when {…}` body under tcl8.6 (no W123/W002 on the body contents); Python applies the iRules `when` BODY role even under tcl8.6, leaking iRules semantics into non-iRules analysis. Rust is the more-correct side; the harness now matches dialect to file type (`.irule` → f5-irules only) so this degenerate "wrong dialect" run no longer registers as a defect. **Remaining true defects** (harness-tracked, all niche): IRULE3102 (`HTTP::uri -normalized`) firing, W004 for `switch -nocase` disabled under f5-irules, and a W123 position inside a split-form `switch` body |
| `opt` (`optimise`) | ✅ wired | profile semantics correct (FULL=single-pass); output tracks the **optimiser** (O100/O109/O117 gaps) |
| `dis` (`asm`) | ⛔ deferred | CLI uses the VM compiler (`compile_script`: literal processing + foreach desugaring) in the excluded `runtime/rust` crate — not the raw codegen pipeline |
| `compwasm` (`wasm`) | ⛔ stub | wasm codegen pipeline + binary output |
| `symbols` (`syms`) | ✅ wired | ports `_collect_scope_symbol_entries` + `_detect_event_entries` over the analyser scope tree (`tcl-compiler` `analyser`). `Scope.procs`/`variables` are `HashMap`s, so sorted by defining-token offset to recover Python's source-order dict iteration (the analyser now gives list bindings like `foreach {a b}` per-name spans, so multi-var loops stay deterministic + source-ordered). Text + JSON. Golden-tested on the faithful subset (`symbols.tcl`: namespaces, nested procs, namespace variables, params, multi-var loops, `when` events). **Analyser gaps** (output tracks the analyser, converges as it does): explicit `::`-qualified proc names report the simple name (`::unknown` → `unknown`); some implicitly-created variables (e.g. `append`-created globals) aren't recorded |
| `symbolgraph` (`symbol-graph`) | ✅ wired | ports `build_symbol_graph` + `_scope_to_dict` + `find_proc_call_sites` (`analyser/semantic_graph.py`) over the analyser scope tree + `command_invocations`. Ordered JSON via `serde_json` `preserve_order`; `HashMap`s sorted by defining-token offset, and proc-scope variables ordered params-first (params share the proc-name span) then body locals by offset. Text + JSON. Golden-tested on the faithful subset (`symbols.tcl`). **Analyser gaps:** explicit `::`-qualified proc names report the simple name — which also skews `ref_count`/`proc_references` (Python matches `inv.name == proc.name` against the as-written name) — and some variable references aren't tracked; converges as the analyser does |
| `callgraph` (`call-graph`) | ✅ wired (parity on the closed subset) | ports `build_call_graph` + `_find_call_sites_in_scope` / `_find_top_level_calls` / `_is_inside_proc` / `_effect_region_str` (`analyser/semantic_graph.py`) onto `tcl-compiler` `CompilationUnit` (`ir_module` + `interproc`) for nodes/edges plus the analyser's `command_invocations` for call-site resolution. Added `ProcSummary::direct_calls` — the *direct* (non-transitive) local call set (Python's `local.calls`) — so the edge list carries no spurious `A→C` transitive edges; nodes sorted by qname, top-level edges in first-seen order, `<top-level>` root prepended. Text + JSON. **Interproc-engine closure landed** (`interprocedural.rs`, faithful to Python `_scan_local_facts`): the call scanner now detects calls nested in `[cmd …]` substitutions (`return [add …]` / `set x [double …]` / `incr n [f …]`) via `scan_value_substitutions` → `scan_source_for_calls`; a **resolved internal call no longer applies the callee's command-effect locally** (so a proc calling a pure proc is `pure`, matching Python); and a **global-variable write is recorded via `writes_global` only**, not the `EffectRegion` set (so `effects` stays `NONE` for a purely global-mutating proc). Full workspace tests stay green. **Side-effects-classification closure landed** (`side_effects.rs`): `classify_side_effects` now consults a command's structured `side_effects` (Python's `side_effect_hints` + the hints-first branch), dialect-gated, so an untracked-effect command (`puts`/`read`/file-IO) is classified impure-but-region-free (`to_effect_regions() == (NONE, NONE)`, `writes_any`) instead of falling back to `UNKNOWN_STATE` — and the interproc method-purity fixpoint keys off the `local_pure` flag (not `effect_writes == NONE`) so a `puts`-calling method stays impure. The `effects` string now matches Python for these. (`log` and other irules-only commands still resolve only when the irules dialect is loaded into the registry the interproc sees — Python's global registry ends up with it loaded via the taint pass; this is a registry-dialect-loading concern, separate from the classify logic.) **Golden-tested** (`callgraph.tcl`: real proc→proc edges with multiple call sites + a namespace proc + top-level call + an impure `puts`-calling proc — node/edge/roots/leaves byte-for-byte) |
| `dataflow` (`dataflow-graph`) | ✅ wired (parity on the closed subset) | ports `build_dataflow_graph` (`analyser/semantic_graph.py`) onto the `tcl-compiler` `CompilationUnit`. The **`proc_effects` + `summary` half is at parity** — per-proc `pure`/`reads`/`writes`/`has_barrier` from the (now-closed) interproc summaries + `_effect_region_str`, sorted by qname. The **taint half** now aggregates **all five Python warning families** per scope, in Python's order — sink-injection (`find_taint_warnings`) / setter-constraint / uri-split / path-concat / destructive-file — mirroring `compiler_checks::run_all_checks` (the warning helpers are now `pub`). The sink-injection family is reconciled with Python's per-statement emission order + labels: `T102` resolves the option-terminator profile (`resolve_option_terminator`) so ensemble subcommands report a compound label (`file delete`) and the option-scan region (`_option_scan_region`/`_arg_can_be_option`) filters positions; `T103` (regex injection / ReDoS) fires for tainted `regexp`/`regsub` patterns; the per-statement order is `T103` → `T106` → primary sink (`T100`/output/log) → `T102` → `T104`/`T105`, matching `_find_taint_sinks`. `tainted_variables` walks the `FunctionUnit` taint lattices ordered by SSA definition site (Python's `analysis.taints` iteration order) and skips version-0 entries — a `(global, 0)` slot is only tainted by the conservative cross-proc global-write seeding, which Python's per-unit analysis never surfaces, so `proc save {v} { set ::store $v }` no longer over-reports `::store`. **Inter-procedural taint solve — DONE** (`tcl-compiler::taint_interproc`, port of `compiler.taint._interprocedural::_solve_interprocedural_taints`): the colour-aware return-summary worklist (`ProcTaintSummary` / `apply_proc_return_summary`, per-`(param, basis)` scenarios over `_BASIS_ORDER`) plus the parameter entry-taint worklist now run over the `CompilationUnit`; `solve_interprocedural_taints` yields `top_taints` / `proc_taints` which the warning families consume in place of the bare per-function `fu.taints` (mirroring Python's `find_taint_warnings`, which `tainted_variables` does **not** use — it keeps `analysis.taints`). So a tainted argument flowing into a proc parameter and then into a sink inside that proc is now warned (`proc s {v} { eval $v }; s $tainted` → `T100`, matching Python). The reporting passes' blanket version-0 skip is refined to skip a version-0 use only for `::`-prefixed names (the Rust-only conservative global-write seeding); a non-`::` version-0 use is a genuine parameter entry-taint and is reported, matching Python. **Golden-tested** (`dataflow.tcl`: real top-level taint — `eval $tainted` → `T100`, `file delete $tainted` → `T102`+`W313`, `regexp $tainted …` → `T103`+`T102` — a global-writing proc (`::store` correctly clean), and the closed `proc_effects` half incl. an impure region-free `puts`-calling proc) |
| `diagram` | ✅ byte-parity | ports `tooling/diagram/extract.py` (`extract_diagram_data` + the `_run_diagram` handler in `graphs.py`) — `commands::diagram`. Walks the lowered IR (`CompilationUnit::build_for`) into the `{events, procedures}` flow tree: per-statement nodes (`if`/`switch` incl. fall-through-arm pattern merge / `loop` for `for`/`while`/`foreach` / `proc_call` / `action` / `assign` of notable `[`-substitution values / `return` / `catch` / `try`+handlers+`finally`), depth-8 truncation, `when_event_name` + `EventRegistry` canonical firing order / priority / multiplicity, and `_diagram_safe_operators` (`&&`→`and`, `||`→`or`, prefix `!`→`not`, reproduced without lookbehind). Action detection uses the new `CommandRegistry::is_diagram_action` (`Traits::DIAGRAM_ACTION`, `::`-stripped to match the canonical spelling — a faithful port of Python's `_any_spec_has`); the `HashMap` procedure table is sorted by defining-token offset to recover Python's source-order iteration. Now unblocked because lowering/IR reached parity (#632); the old "sizeable / lowering-gapped" caveat no longer applies, and it does **not** use `classify_side_effects`. Text + JSON golden-tested (`diagram.irule`: multi-event firing order + priority + multiplicity, switch fall-through, if/else, foreach, catch, try/on/finally, actions, a proc call, a regular proc); verified byte-identical end-to-end vs the Python CLI across f5-irules + plain-tcl, empty, and deep-nesting inputs |
| `diff` | ✅ byte-parity (AST + IR + CFG) | ports `tooling/tcl/verbs/diff.py`. **`ast` layer byte-parity**: segments each side (`tcl-compiler` segmenter), resolves subcommands + `range_dict` into the canonical `_serialise_command_ast` JSON (`sort_keys` via `tcl-registry` `snapshot::Json`), `difflib.unified_diff`-faithful via the shared **`tcl-cli-support::difflib`** (CPython `SequenceMatcher`/`unified_diff` port; f5 copy should dedup onto it). **`ir` layer byte-parity**: Python's diff reads `serialise_result(compiled)["ir"]`, and the native **`tcl-explorer` `serialise::serialise_ir`** reproduces that view byte-for-byte (verified against `tooling/cli/serialise.py`'s `_serialise_ir`/`_serialise_script` + the `stmt_*`/`preview`/`range_dict` formatters), so `diff` **calls it directly** through the same `value_to_json`→`snapshot::Json` adapter as the `cfg` layer — the duplicate `tcl-cli::commands::serialise` IR module was deleted (one source of truth). `--show ast`/`ir` text + JSON golden-tested; exit codes (0 equal / 1 differ); verified across ~12 construct-varied script pairs (return-expr, catch, expr-eval, nested if/elseif/while, foreach, switch fall-through, dict/lappend/multiline). **`cfg` layer byte-parity** (engine-gated work now landed): `layer_payload`'s `cfg` arm builds `{preSsa, postSsa}` from the explorer's `serialise_result` (the `cfgPreSsa`/`cfgPostSsa` views over `tcl_explorer::run_pipeline`), rendered through the shared sort-keys indent-2 `Json` adapter — mirroring Python's `_collect_diff_layer_payloads` cfg branch. Closing the CFG/SSA **engine** divergences made the serialiser output converge automatically: (1) the CFG builder now emits the trailing unreachable `exit` block for straight-line-terminated bodies (`lower_script` returns the resting block; `build_function` appends the exit via the no-op `ensure_goto`); (2) glob/regexp switches — and exact switches with any fall-through arm — are kept **opaque** (a single `Statement::Switch` in the block, codegen emitting a generic `switch` invoke), matching the Python builder rather than expanding arm blocks (the earlier `switch_arm_body_N` naming + `${subject} eq` conditions were already aligned); (3) SSA uses **semi-pruned** phi placement (phis only for upward-exposed names) so dead-variable phis + their versions/`inferredTypes` match; (4) SCCP seeds live-in roots (used-but-undefined values) to `overdefined` so the post-SSA lattice detail matches. `--show cfg`/`all` text + JSON golden-tested (proc + `if`/`elseif`/`else` + `expr` + `foreach` + `while` + glob `switch`); `--show all` now expands to `ast,ir,cfg`. `postSsa`'s `analysis.deadStores` is the **liveness-based** set (`dead_stores::liveness_dead_stores`, a port of Python's `_dead_stores`: def-use-chain liveness + place-model `reportable_local` + substitution-hidden-reads + element-observed + IR-type suppressions), byte-identical to Python's `analysis.dead_stores` on every function whose SSA matches (13/13 in a corpus sweep). **Residual cfgPostSsa divergences are Python-side, not Rust:** a corpus sweep confirmed Rust's SSA *construction* is correct — the remaining divergences from the Python CLI are **(a) Python CFG bugs in `break`/`continue`**: Python lowers them as fall-through commands into the enclosing `if`'s join block instead of jumping to the loop's exit/continue target (so `break` flows back to the loop header and `continue` executes the post-`if` body), whereas Rust models the jump correctly (`loop_stack` + `lower_loop_jump`); and **(b) Rust's type lattice being *more* precise** (e.g. `interp create` → `string` where Python leaves `*`). The one genuine *Rust* bug found by the dig — a `try` handler ending in `return -options`/`return {*}…` not terminating its block, adding a spurious `try_handler → try_end` phi (e.g. `auto_mkindex`) — is fixed (treated as a `Return` terminator in analysis builds, mirroring `compiler/cfg.py` ~L1016; codegen unchanged). cfg golden fixtures stay on constructs free of the Python `break`/`continue` bug so they remain byte-identical. Also a representation difference (not a divergence in correctness): Rust shows a synthetic `<cond>` marker for command-substitution branch conditions and lowers `namespace eval` as a barrier, where Python omits the marker / keeps an `IRBlock` and emits `IRInterpBoundary` statements. (Residual IR caveat: `assign-expr`/`expr-eval`/`return [expr …]` summary text is sliced from source since the IR keeps a parsed `ExprNode`, not raw text — exact for the common braced/`[expr …]` forms.) |
| `explore` | ⛔ stub | explorer report port |
| `find-legacy` | ✅ wired | ports `tooling/tcl/verbs/misc.py` `_run_find_legacy` (`commands::misc`). Combines the inputs (`combine_sources`), runs the analyser once, keeps the six convertible codes (`W100`, `W104`, `W110`, `W304`, `IRULE2001`, `IRULE5001` — the `_CONVERTIBLE_CODES` set) and attaches the per-code modernisation hint (`_CONVERSION_MAP`, default `"modernise"`). Text shape `legacy patterns: N` / `  CODE line L:C MESSAGE` / `    conversion: …` (empty → `no legacy patterns detected`); JSON `{count, dialect, issues:[{code,line,column,message,conversion}]}` via a `serde` struct (insertion-ordered, *not* the key-sorting `snapshot::Json`) + `ensure_ascii`, matching Python's `json.dumps(indent=2)` byte-for-byte. Exit 0 always. **Unblocked by the analyser-parity work** in the `diag` row — all six codes match Python on firing condition, span, message, and severity; the only divergence is diagnostic emission *order* (the accepted, documented one). Golden-tested (`find_legacy_*` in `cli_parity.rs`): tcl + iRule (under `f5-irules`) + empty, text + JSON. The single-pattern-per-line fixtures keep source order == Python pass order, so the goldens (captured from the Rust binary per the ordering guardrail) match Python exactly |
| `registry-dump` | ✅ wired | `profiles` + `objects` + `events` snapshots byte-parity (`tcl-registry::snapshot`); **`events`** ports `event_graph_snapshot` — per-event protocol props (`EventProps`→`props` with the `transport` string/list/null remapping), firing order (`orderIndex`/`masterOrder`), flow chains (incl. `FlowChain.notes`) + the content-addressed `validCommandsDigest` (sha2) over the `event_info` cross-product. **`commands`** ports `command_registry_snapshot` / `command_entry` (`tooling/registry_snapshot.py`) onto the registry via the new `tcl-registry::command_snapshot`, `json.dumps(indent=2, sort_keys=True)`-faithful via the `snapshot::Json` serialiser (snapshot wiring exact — `set`/`incr`/`expr`/… byte-identical; `info.validEvents*` a stub for `f5-irules`). **`commands` byte-parity is gated by command-registry _data_ parity**, not the wiring: the registries diverge on the per-command **`dialects`** representation (Rust explicit sets vs Python `None`; no `f5-bigip`/`f5-tmsh` bits — 570/641 commands), plus scattered trait values / hover-synopsis / arity / subcommand-`forms` modelling. Converges as the registry data reconciles (a separate workstream). **Faithful subset golden-tested** (`registry_dump_faithful_subset_matches_python`: 32 core commands byte-identical to Python). Target-specific emission flags are intentionally omitted from the shared registry. |
| `help` (`docs`) | ✅ byte-parity | ports `tooling/tcl/verbs/lookup.py` `_run_help` + the `search_help` / `list_features` query layer (`shared/help/kcs_db.py`) over the embedded KCS help database (`commands::help`, `rusqlite` bundled `SQLite` + FTS5). The DB is **not committed** (it's gitignored); instead **`build.rs` ports `scripts/build/kcs_db.py`** — parsing the committed `docs/kcs/features/*.md` (title + `## section` extraction, `parse_applies_to` + `applies_to_category` tag/category derivation, `how_to_use`+`example` content blob) into the `kcs_features` FTS5 table (`porter unicode61`) + `feature_tags`, written to `$OUT_DIR/kcs_help.db` and `include_bytes!`-embedded (no Python at build/test time; screenshots omitted — the verb never reads them). Full-text search (`MATCH '"q" OR q*'` ranked by **BM25** with the `LIKE` fallback), catalogue listing, the per-dialect substring filter (`_HELP_DIALECT_TERMS`), text + JSON (incl. the raw BM25 `rank` float and `ensure_ascii`), stdout/stderr split + exit codes. **Byte-parity** verified end-to-end vs the Python CLI across search/catalogue/dialect/no-match — codes, content, ordering and escaping match. **Caveat:** the raw BM25 `rank` float in `--json` is computed by whichever `SQLite` version is *linked* (Python the host's, e.g. 3.45.1; rusqlite its bundled one, e.g. 3.46.x) and the two diverge in the low-order digits on some corpora, so `rank` is not a cross-environment-stable parity field — the json golden is captured from the Rust binary, the text + catalogue goldens from Python. Golden-tested (`help taint` text+json, `help --dialect f5-irules` catalogue); goldens track `docs/kcs` (regenerate when the feature notes change) |
| `minimize` (`minimise` / `repro`) | ✅ wired | ports both `server/features/minimize.py` (the engine) and `tooling/tcl/verbs/minimize.py` (the verb) into `commands::minimize`. **Engine**: Zeller delta-debugging (`ddmin`) over source lines gated by "the requested code still fires" (the predicate is `Analyser::analyse(src, dialect).diagnostics` membership — the same analyser the `diag` verb drives), then `_dedent` (revert-gated) and a **verify-gated identifier rename** — `collect_rename_edits` re-tokenises (VAR `$x`/`${x}`/`$arr(idx)` base names) + segments (`segment_with_recovery`, `None` registry) the variable-target commands (`set`/`global`/`incr`/`append`/`lappend`/`unset`/`variable`/`lassign` — `_VAR_TARGET_CMDS`), `apply_edits` rewrites right-to-left, and the rename is kept only if the code still fires. `_RESERVED_NAMES`, the `a b … z a1 b1 …` short-name sequence, and the `minimize_diagnostic` `ValueError`→`MinimizeError::NotPresent` contract are ported exactly. **Verb**: iterates the input documents, skips those where the code does not fire, and emits text (`# file: CODE (N→M lines, renamed=Bool)` + the reduced source + blank line; `→` is U+2192) or JSON (a list of `{file, code, original_lines, reduced_lines, renamed, reproduces, source}`, insertion-ordered serde struct + `ensure_ascii`, matching `json.dumps(indent=2)`); `CODE does not fire on any input.`→stderr + exit 1. **Argparse-positional parity**: the Python verb takes `inputs` (`nargs="*"`) followed by a required `code`; clap forbids a required positional after a variadic one, so CODE is the **last** positional (`tcl minimize script.tcl W220`) split off the trailing input in the handler — byte-identical user-facing order. Golden-tested (`minimize_*` in `cli_parity.rs`) on well-formed tcl reductions (W100 text/json, `--no-rename`, W211, no-fire exit-1) — these reduce to a single well-formed line whose analysis matches Python exactly, so the Rust-captured goldens are byte-identical to the Python CLI. Pure-engine unit tests cover `ddmin`/`dedent`/`apply_edits`/`short_for`; a **property test** (`minimize_reduced_output_still_fires`) re-runs the analyser on the reduced snippet to confirm it reproduces CODE. **Accepted divergence (not a defect):** because `ddmin` deliberately explores *brace-unbalanced* reductions, the Rust and Python analysers can legitimately differ on how far the reduction goes (e.g. Rust fires `IRULE2001` on a lone `if {[matchclass …]} {` whose `if` body is unterminated, where Python's segmentation does not descend, so Python keeps the `when {…}` wrapper). Both outputs still reproduce CODE — the divergence is in the *reduction depth* on malformed fragments, not correctness; the well-formed-reducing tcl goldens are unaffected |
| `pkg` / `venv` / `docker` | ⛔ stub | the `tclpkg` subsystem (manifest/resolver/lockfile/CAS/registry/venv/docker) |

## `f5` verb status

The BIG-IP **object model + config parser** are ported (`tcl-bigip`), but most
**analysis/emit engines are still Python-only** (`dialects/f5/bigip/*`,
`dialects/f5/query/*`). The verbs needing only file I/O + the existing parser
helpers are done; the rest await engine ports.

| Verb | Status | Notes |
|---|---|---|
| `completion` | ✅ idiomatic | clap_complete |
| `merge` | ✅ byte-parity | golden-tested; `--format tmsh`/`tmsh-delta`/`--transaction` wired via `render_config` (golden-tested) |
| `split` | ✅ byte-parity | uses `extract_blocks`/`parse_generic_header`; round-trip golden-tested; `--format tmsh` writes per-partition `.tmsh` files via `render_config` |
| `diff` (`changes`) | ✅ parity (add/remove/scalar) | ports `compute_diff` over the model; fields read from `canon_fields()`; accepts tmsh input via `to_scf`. **Gap:** object-list field *display* (pool `members`, data-group `records`) shows canonical JSON vs Python's dataclass `repr` — change *detection* is still correct. Golden-tested (add/remove text+JSON, scalar modify, tmsh input) |
| `explain` (`describe`) | ✅ byte-parity | ports `compute_explain` + the `resolve_name` resolution layer (model-based — does **not** need the ref-graph); walks `canon_fields()` (`BigipList` navigation) for profiles/iRules/persistence/SNAT/pool. Verified across virtual/pool/auto/short-name/not-found, text + JSON; golden-tested |
| `extract` (`ucs2scf`) | ✅ byte-parity (scf) | golden-tested. Ports `extract_ucs_file` onto `tcl-bigip-io`: reads a `.ucs` (plain gzip-tar **or** OpenPGP-encrypted), extracts to SCF in memory, writes verbatim. Encrypted archives decrypt purely in Rust. `--format tmsh`/`tmsh-delta`/`--transaction` wired via `render_config`. interactive TTY passphrase prompting not yet wired in the binary (env-var / explicit only) |
| `graph` (`deps`) | ✅ byte-parity | full ref-graph (nodes + pilot/legacy/iRule edges) → `export_graph` (DOT/JSON/Mermaid). Verified end-to-end vs the Python CLI across all formats × `--seed`/`--reverse`/`--max-depth`. (Byte-parity on configs free of the documented registry-data drift.) |
| `stats` (`summary`) | ✅ byte-parity | object/partition counts, iRule LOC + events, top-referenced, orphans over the graph. Text + JSON; golden + end-to-end verified |
| `cleanup` (`clean`) | ✅ byte-parity | BFS-reachability orphan detection → reverse-topological `tmsh delete` script. `--keep`/`--no-keep-common`, text + JSON; golden + end-to-end verified |
| `validate` (`lint`) | ✅ byte-parity | New **`tcl-bigip::lint`** module (sibling of `cleanup`/`stats`/`graph`) — `run_lint` + all 8 rules (orphan-monitor, empty-pool, virtual-without-pool, pool-without-monitor, irule-deprecated-command/empty-when/unknown-event/missing-`<kind>`). Reuses the typed model + `stats::is_root_kind` + `tcl-irules` object-refs + `tcl-registry` events + the `graph.rs` `resolve_name`; **does NOT use the query DSL** (lint walks raw model fields the projection would transform). Verb ports `text`/`json`/`sarif` + severity-based exit codes (0/1/2). Verified byte-identical end-to-end vs `python -m tooling.f5.main validate` across all formats + filters; golden-tested (`validate_parity.rs`, 15 cases) |
| `rename` | ✅ byte-parity | the edit-planner verb — a thin shell over the `f5 query` engine (`rename(OLD, NEW)` routed through `run_query`; `tcl-bigip-query::rewrite`). Default unified-diff preview / `--write` / `-o` / `--in-place`; `--format tmsh`/`tmsh-delta`/`--transaction` wired via `render_config` (diff preview stays SCF↔SCF; `--in-place`+`tmsh` rejected). Golden-tested |
| `tmsh` (+delta), `convert` (`scf2as3`/`ucs2scf`), `redact`/`unredact` | ✅ byte-parity | `tmsh`: SCF→tmsh `create`/`modify`/delta emitter (`tcl-bigip::tmsh_emit`) — also powers `--format tmsh` in `extract`/`split`/`merge`/`redact`/`unredact`/`rename`. `convert`: AS3 declaration engine (`tcl-bigip::convert`) + ucs2scf (reuses `extract`). `redact`/`unredact`: secret-stripping + IP-remap (`tcl-bigip::redact`) incl. a hand-ported CPython-MT19937 `--shuffle`, with round-trip + sidecar-`.map` parity. `--format tmsh`/`tmsh-delta`/`--transaction` wired via `render_config` (tmsh `modify` verb; no pre-edit original threaded, so delta treats all objects as created — matching Python); golden-tested |
| `grep`, `pcap-remap`, `enrich-pcapng`/`enrich-wireshark` | ✅ byte-parity | `grep`: `compute_grep` ref-graph search (`tcl-bigip::grep`; literal/regex/CIDR seeds, direction/depth, text/json/tmsh). `pcap-remap`: PCAP IP-remap (`tcl-bigip::pcap_remap`; classic libpcap + pcapng + F5 trailers, checksum byte-parity; custom `--schema` TOML deferred). `enrich-*`: config→capture/profile enrichment (`tcl-bigip::pcap_enrich`/`wireshark_profile`; Wireshark profile + NameIndex + direct-write PCAPNG annotation byte-parity; `editcap`-driven libpcap→pcapng conversion deferred as in Python) |
| `fetch`/`push`/`pull` | ✅ offline parity / live untested | Remote verbs (`f5-cli` `commands::remote`). `push --dry-run` request dump + `resolve_credentials` precedence/errors byte-parity (golden-tested offline). Live iControl REST via `ureq`+`rustls` (push PUT/POST, pull GET, fetch UCS→SCF via `tcl-bigip-io`) — implemented, only runs against a real device. **SSH transport deferred** (`russh` needs `unsafe`/C deps) |
| `irule` group | ✅ complete (pgo removed) | `event-order`/`extract`/`format`/`minify`/`event-info`/`lint`/`context` byte-parity (reuse `tcl-registry`/`tcl-bigip`/`tcl-lsp-core`). **`event-info`**: ports `lookup_event_info` over the reconciled command registry (`CommandRegistry::event_info` + the event-validity cross-product) + a generated `event_descriptions` prose table; verified byte-identical across all 178 events (text + JSON). **`lint`**: pure wiring over the already-ported `tcl-bigip::lint` engine (the same one powering byte-parity `f5 validate`) — extends the irule-input loader (`commands::irule` `load_irule_inputs`) to also return origin-keyed `configs`/`sources`, synthesising a single-rule `BigipConfig` for standalone `.irule`/`.tcl` files (`/{stem}`) and inline `--source` (`/inline_{n}`); calls `run_lint(category="irule")` with per-origin joined rule bodies and reuses the `f5 validate` `to_json`/`to_text` formatters + severity exit codes (error→2/warning→1/else 0). Golden-tested (`irule_parity.rs`) on standalone `.irule`, bigip.conf (all four irule rules), missing-object refs, `--source`, `--severity`, text + JSON. **`context`**: new **`tcl-bigip::irule_context`** engine (port of `ai/shared/irule_context.py`) — `build_irule_context` (object-reference walk over `tcl-irules` + a `resolve_name`-over-model resolver + one-hop transitive pool→node/monitor expansion + range-based source slices) and the `context_bundle_to_json`/`bundles_to_json`/`context_bundle_to_text` renderers (`json.dumps(indent=2, ensure_ascii)`-faithful insertion-ordered `Json`; 7 per-kind summarisers + 8 text renderers). The verb merges configs once (`tcl-bigip::lint::merge_configs`) for cross-file resolution, iterates rules with `--rule`/`--no-transitive`, and dispatches directory (one file per rule) / single-file / stdout, JSON `{"bundles":[…]}` vs text chunks. Golden-tested across every section (pool/data-group/persistence/snat-pool/profile/monitor/node + unresolved + slices), realistic `bigip.conf`, `--no-transitive`, standalone `.irule`, inline `--source`, and the no-iRules exit-1 path; verified byte-identical over all `.conf`/`.scf` fixtures × {text, json, no-transitive}. **`trace`**: a purely **static** event-handler trace (no VM — the old "compiler-VM" label was wrong) — `\bwhen EVENT\s*\{` block-match (case-insensitive) + balanced-brace body slice + first-token command extraction + object-reference walk (`tcl-irules` + the shared `irule_context::classify_kind`/`resolve_reference` resolver over the merged config). Text + JSON, golden-tested (realistic + all-kinds configs, case-insensitivity, no-match exit-1). `pgo` was removed from the command surface rather than shipped as a deferred stub (#1315) — it needs a `compiler/pgo` branch-reorder engine over the compiler's CFG (`tcl-compiler` is not a `f5-cli` dependency) plus a real profile source, a standalone compiler feature out of scope here; see `docs/design/rust/python-parity-scrub.md` (`P100 PGO`). `explain-flow` (needs the iRule simulator / VM) remains deferred |
| `explain-flow` | ⛔ blocked | needs the **iRule simulator** (`simulate_irule_for_session` → the Tcl runtime/VM, the excluded `runtime/rust` crate) |
| `query` (`q`) | ✅ byte-parity | **`f5 query` runs end-to-end byte-identical to the Python CLI** — read-only AND mutating. `tcl-bigip-query` ports the full engine: front-end (lexer/AST/parser), value model + `json.dumps`-faithful output (auto/json/raw/paths/scf/table/table-lineart), the evaluator (full jq core + all 29 special forms), **244/244 builtins**, the **BIG-IP projection** layer (Container/ObjectRef/PathRef over the typed model), graph `refs`/`referenced_by` + rule `.refs`, the **edit-plan** (field-value + identity-field/`rename*` mutations with a faithful `difflib.unified_diff` port, `--write`/`--in-place`/diff, `--format tmsh`/`tmsh-delta`/`--transaction` rendering of the rewritten config with the `--in-place`+tmsh guard + strict-UTF-8 in-place reads, and cross-file edits via `$name`), `--partition` source binding, `-f`/`--from-file`, **renderers** (`--render mermaid`/`gantt`/`ascii-blocks`), **side-inputs** (`--input-json`/`jsonl`/`csv`/`f5log` + loaders), **live probes** (dns/ping/tls/x509 + `cert_load`, `--enable-probes` gating; `x509_parse` byte-parity), `--merge` (cross-file unified namespace + cross-file refs), and `--help-dsl`/`--help-examples`/`--help-renderers`/`--help-inputs`. **All 24 cookbook examples + a broad query matrix verified byte-identical** end-to-end vs `python -m tooling.f5.main query`. ~25 golden-differential suites. **Documented deferrals:** `--help-builtins`/`--help-manual` (the per-function prose metadata was intentionally omitted from the Rust registry), live `url_*` HTTP (stub returns the faithful result-dict shape; gating + the `http_*` accessors are real), `ucs_cert`'s UCS reader (cross-layer), PKCS#12 `cert_load`, and the long-tail object kinds the Rust model doesn't carry |

**Shared parity helpers** (`tcl_cli_support`):
- `ensure_ascii` — escape non-ASCII as `\uXXXX`, matching Python's
  `json.dumps(ensure_ascii=True)`; applied to every JSON-emitting verb.
- `f5-cli` `commands::scf::to_scf` — normalise `tmsh create/modify` script input
  to SCF (port of `_to_scf` / `tmsh_parse.py`) before parsing.

**f5 input layer** (`tcl-bigip-io`, **done**): the UCS foundation. A plain UCS
is a gzip-tar of `/config`; an encrypted UCS is an OpenPGP *symmetric* message
(F5 KB K5437) whose plaintext is that gzip-tar. The crate decrypts **purely in
Rust** — a faithful 1:1 port of the Python `_openpgp`/`_aes` fallback (S2K +
AES-CFB + quick-check + SHA-1 MDC), chosen over the rPGP `pgp` crate because
"most faithfully reproduces the Python decryptor" points at a line-for-line
port on a tiny audited dep tree (`aes` + RustCrypto hashes + `flate2` + `tar`)
rather than a full-OpenPGP implementation. Everything is in-memory (decrypt →
gunzip → untar on cursors) so a UCS's SSL keys never touch disk. KAT-tested
(FIPS-197) + differential parity against gpg-produced fixtures. The
`read_path`/`load_paths` resolver makes `.ucs` (plain or encrypted) a
first-class input. It is wired into the model-reading verbs that are already
ported — `diff`, `explain`, `merge` accept a `.ucs` (plain or encrypted) exactly
like a `.conf`/`.scf`, golden-verified — and `stats`/`cleanup`/`grep`/`validate`/
`graph`/`rename` inherit it automatically once their ref-graph engine lands.

**Keystone:** the BIG-IP **object reference graph** — `build_bigip_object_graph`
in `dialects/f5/bigip/link_extract.py` (~569 LOC, range-based node/edge
extraction). `stats`, `cleanup`, `grep`, `validate`, `graph`, and `rename` all
build on it — port + golden-test it in isolation first (verify via
`f5 graph --format json`). (Separate from `irules_object_refs.py` (~944 LOC),
the iRule-command → object reference resolver used by the `irule`/query paths.)

Keystone progress / remaining pieces:
- ✅ **object-registry query layer** (`object_registry.py`) → `tcl-registry::bigip`
  (`kind_for_header`, `candidate_kinds_for_key`/`_for_section_item`,
  `matches_section`, `default_registry`). Differentially golden-tested (3398
  probes). `kind_for_header` matches Python exactly.
- ✅ **node extraction** (`_build_objects_for_source`) → `tcl-bigip::graph`
  (`build_objects_for_source` / `ObjectNode` / `GraphContext`). All 28 nodes of
  a representative `bigip.conf` match Python exactly (node_id, kind, offsets,
  ranges).
- ✅ **name resolution** (`resolve_kind_in_configs` + `BigipConfig.resolve_name`
  / `resolve_generic_object`) → `tcl-bigip::graph`. Object ranges read from
  `canon_fields()["range"]`; `spec.module` stands in for the per-object module
  (verified safe for table-backed kinds). 84 probes match Python.
- ✅ **legacy forward edges** (`_build_forward_edges` token-scan path +
  `build_bigip_object_graph`) → `tcl-bigip::graph` (`ObjectEdge`/`ObjectGraph`).
  Reproduces every Python legacy edge in order; 2 frozen drift edges pinned
  (`graph_edges_legacy.drift.txt`).
- ✅ **registry-first dispatch** (`references_via_spec` / pilot value-spec engine)
  — **complete**. Wired into the edge walk (runs before the legacy path, shared
  dedup) + `candidate_registry_kinds_for_display`. The graph only consumes each
  `Reference`'s `(target_kind, target_path)`, so each pilot spec is a **slim
  extractor** over the raw value (no full `ValueSpec`/`BigipList` materialisation).
  - ✅ **all reference-producing specs ported + golden-tested**:
    `ListSpec(ObjectRefSpec)` (`rules`/`policies`/`vlans`/firewall lists),
    `Profile`/`Persistence` attachments, `MonitorExpressionSpec`, `SnatModeSpec`,
    `CertKeyChainSpec`, `FirewallRuleSpec`. Comprehensive `graph_pilot.conf`
    fixture (monitor min-of, SNAT pool, cert-key-chain, firewall source/dest
    lists) + the realistic `bigip.conf`.
  - ✅ reference-free migrated specs (`DestinationSpec`, `DataGroupRecordSpec`,
    `GtmRegionMemberSpec`, `LtmPolicyRuleSpec`) correctly fall through to legacy.
  - Drift edges (Rust legacy-section refs not cleared) pinned in
    `graph_pilot.drift.txt` / `graph_edges.drift.txt`.
- ✅ **iRule edges** (`irules_refs.py` + `irules_object_refs.py`) → new shared
  **`tcl-irules`** crate (deps: tcl-compiler/tcl-lexer/tcl-registry; consumed by
  both `tcl-bigip` graph and `tcl-lsp-core` semantic tokens). Ports
  `resolve_object_ref_args` (the hand-written `_BASE_SPECS` + pool templates +
  `class`/`persist` resolvers — the 1 MB generated graph backs only completion/
  coverage, not edge resolution) and the `extract_irules_object_references`
  walker with `set`-binding copy-propagation. Wired into `_build_forward_edges`
  (`via_property = "irule:<command>"`); `tcl-lsp-core`'s span-only port migrated
  onto it. Golden-tested: 117 resolve probes, 15 walker cases, and the full
  `bigip.conf` graph (61 edges incl. 3 iRule) — only the 1 pinned drift edge.
- ✅ **graph_export** (`graph_export.py`) → `tcl-bigip::graph` (`export_graph` +
  `filter_to_subgraph` BFS + DOT/JSON/Mermaid serialisers; JSON hand-written for
  `json.dumps(indent=2)` key-order parity). Byte-parity golden-tested.
- ✅ **`f5 graph` (alias `deps`)** wired end to end (`load_paths` →
  `build_bigip_object_graph` → `export_graph` → file/stdout). Verified
  byte-identical to the Python CLI across all 3 formats × the full flag matrix
  (`--seed` / `--reverse` / `--max-depth`).

> **Registry-data drift — RESOLVED.** The generated Rust registry **data**
> (`tcl-registry/src/bigip/data`) had drifted from current Python (a stale
> 992-kind baseline; current Python has 798). It has been **regenerated** from
> the reconciled Python `OBJECT_SPECS` by the restored
> `scripts/registry-audit/gen_bigip_rust.py`. All drift is gone: `candidate_kinds_*`
> and `candidate_registry_kinds_for_display` match Python on every probe (drift
> pins deleted), and the BIG-IP graph is byte-identical to the Python `f5 graph`
> on real configs. Re-run the generator whenever the Python `OBJECT_SPECS`
> baseline moves.

> **Command-dialect reconcile — RESOLVED.** The Rust command registry
> (`tcl-registry/src/commands/**`) encoded Tcl-version availability and Tk
> membership in the `DialectSet` itself (`Some(ALL_TCL)`/`Some(TK)`/version
> subsets) where Python uses `dialects=None` (universal) or an explicit
> per-command frozenset. ~627 commands diverged, breaking the iRules
> event/command cross-product (`commands_for_event`). Reconciled every
> spec's `dialects:` field to mirror Python across all 13 modelled dialects
> via `scripts/registry-audit/reconcile_irules_dialects.py` (+ a
> `NON_IRULES_OPERATORS` aggregate). `valid_irules_commands_for_event` is now
> byte-identical to Python for all 176 events (HTTP_REQUEST: 1290). This is
> the registry-data half of the old `~191-vs-~1236` `irule event-info` gap.

> **`registry-dump` `events` — DONE; `commands`/`all` — still deferred.** The
> `profiles`/`objects`/`events` sections are byte-parity. `events` landed via
> `event_graph_snapshot` (`tcl-registry::snapshot`): the event-validity
> cross-product + sha2 `validCommandsDigest` come from `event_info`, plus a
> `FlowChain.notes` data add and the `EventProps`→`props` field/transport
> remapping (string for single, list for dual, null for none). `commands`/`all`
> still embed `command_registry_snapshot`'s per-command `traits` (≈50 keys) /
> `scalars` dicts, which mirror the Python `CommandSpec` *dataclass field
> layout* — no clean byte-identical Rust mapping (the Rust `CommandSpec` is a
> different shape, traits are bitflags). Same boundary as the `tcl`
> `registry-dump commands` deferral.

## Prioritised remaining roadmap

Done so far (f5): `merge`, `split`, `diff`, `explain` (model-based); `extract`
(UCS). The **f5 input layer + UCS** foundation (`tcl-bigip-io`) is complete.
Remaining, in dependency order:

1. **BIG-IP ref-graph** (`build_bigip_object_graph`, `tcl-bigip` extension) —
   unblocks `stats`, `cleanup`, `grep`, `validate`, `graph`, `rename`. Highest
   leverage; port + golden-test in isolation first. **(in progress)**
2. **tmsh / AS3 emit engines** → `tmsh` (+delta), `convert`; **redaction** →
   `redact`/`unredact`.
3. **`tcl-cli-serialise`** (port `tooling/cli/serialise.py`) — unblocks the tcl
   JSON verbs (symbols/callgraph/symbolgraph/dataflow/diagram, tcl `diff`).
   Gated by analyser parity for full fidelity.
4. **`registry-dump`** (port `registry_snapshot.py`) — real parity, both CLIs; finicky (field-by-field).
5. **f5 remote** (`tcl-bigip-remote`: REST/SSH) and **PCAP** (`tcl-bigip-pcap`) — pure-Rust crates; covers `fetch`/`push`/`pull`/`explain-flow`/`enrich-*`/`pcap-remap`. (UCS/OpenPGP/AES already done in `tcl-bigip-io`.)
6. **Query DSL** (`tcl-bigip-query`) — ✅ **DONE** (byte-parity). The query DSL (`dialects/f5/query`, ~18k LOC) is fully ported across these increments, each golden-differential-tested:
   - ✅ **front-end** (`lexer`/`ast`/`parser`) — code-point offsets match Python; 93-query token+AST matrix + error cases.
   - ✅ **value model** (`value.rs`) + **`jsonfmt`** — `Value` enum, `truthy`/`py_eq`/jq `sort_cmp`, `json.dumps`-faithful (ensure_ascii + surrogate pairs + Python float `repr`).
   - ✅ **output** (`output.rs`) — auto/scf/raw/paths/json/table/table-lineart.
   - ✅ **evaluator** (`eval.rs` + `special.rs`) — full jq core + all 29 special forms.
   - ✅ **builtins** — **244/244** registered across the category modules (value/stream/string/regex/math/net-ip-CIDR/time/encoding/graph/rename/probes/extras). Math uses CPython's Lanczos gamma + A&S Bessel for bit-parity; net reproduces CPython 3.11 `ipaddress` IANA tables; regex via the `regex` crate (documented backref/lookaround divergence).
   - ✅ **projection** (`projection.rs`) — Container/ObjectRef/PathRef over the typed `tcl-bigip` model (bridges the `Vec<Placed>` model to the dict-per-kind DSL shape); core LTM kinds + pool-member/policy sub-objects + rule `.refs` (graph-backed); per-kind field maps reproduce the pilot-spec/PathRef/typed-string projection.
   - ✅ **graph** — `refs`/`referenced_by`/`references_to`/`check_partition_visibility` over `build_bigip_object_graph` (cross-file in `--merge`).
   - ✅ **edit-plan** (`edit_plan.rs` + `rewrite.rs`) — field-value (`=`/`|=`/`+=`/`-=`) + identity-field/`rename*` mutations; faithful `difflib.unified_diff`+`SequenceMatcher` port; `--write`/`--in-place`/diff + `renamed …` reports.
   - ✅ **renderers** (`renderers/`) — mermaid/gantt/ascii-blocks + `--render`/`--render-opt`.
   - ✅ **side-inputs** (`inputs.rs`) — `--input-json/jsonl/csv/f5log` + the `*_load` builtins.
   - ✅ **probes** (`probes.rs`) — `--enable-probes`-gated dns/ping/tls/x509; `x509_parse`/`cert_load` byte-parity; live `url_*` HTTP stubbed (deferral) + the pure `http_*` accessors real.
   - ✅ **runner + verb** (`runner.rs`, `f5-cli` `commands::query`) — multi-file, `--merge`, `$name` binding, output modes, exit codes, `--help-dsl`/`--help-examples`/`--help-renderers`/`--help-inputs`.
   - **Deferrals (documented):** `--help-builtins`/`--help-manual` (per-function prose metadata intentionally omitted from the Rust registry), live `url_*` HTTP body fetch, `ucs_cert`'s UCS reader (cross-layer), PKCS#12 `cert_load`, long-tail object kinds.
   - **Note:** `f5 validate` does **not** depend on the query DSL — it runs `dialects/f5/bigip/lint.run_lint` over already-ported engines, so it can land independently.
   **tclpkg** (`pkg`/`venv`/`docker`) remains the other large sub-system.
7. **tcl-only engine-gap verbs**: `help` (KCS SQLite), `minimize`, `find-legacy`, `explore`/`diagram` (explorer report), `compwasm` (wasm pipeline), `dis` (VM compiler).
8. **Engine-gap closure** (separate workstreams): analyser, optimiser, lowering/VM-compiler — these flip diag/opt/dis to parity.
9. **Cutover** — native-binary packaging, remove `[project.scripts]` entries, rework `zipapp-tcl`/`zipapp-f5`, delete CLI-exclusive Python (NOT the shared `dialects/f5/bigip` etc. still imported by the Python LSP server/analyser/ai).

## Adding a verb (the pattern)

1. Resolve inputs via `tcl_cli_support::read_input_documents` (+ `combine_sources`).
2. Call the pure engine core (existing crate or a new per-subsystem lib).
3. Format output to match the Python verb exactly (field order, separators;
   JSON via `serde_json::to_string_pretty` with field-ordered structs).
4. Write via `write_text_output` / `write_highlighted_output`.
5. Capture a golden from the Python CLI and add a test in the matching parity
   suite (`rust/tcl-cli/tests/cli_parity.rs` or `rust/f5-cli/tests/cli_parity.rs`)
   — **only** for verbs whose engine is fully ported (don't assert byte-parity on
   engine-gapped verbs; for those, verify ad-hoc against the Python CLI and note
   the gap here).

## Verification

Each CLI has a `tests/cli_parity.rs` that runs the built binary and diffs stdout
against committed `tests/fixtures/*.golden` files captured from
`python -m tooling.{tcl,f5}.main`. Self-contained (no Python at test time), so it
runs under `cargo test --workspace`. Current coverage: 7 tcl + 11 f5 golden
tests, plus `tcl-bigip-io`'s FIPS-197 AES KATs and UCS differential tests
(self-contained — fixtures captured once from the Python CLI and `gpg
--symmetric`; no Python/gpg at test time).
`make rust-tcl` / `rust-f5` / `rust-clis` build the binaries.

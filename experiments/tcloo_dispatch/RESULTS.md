# tcloo_dispatch — object-dispatch resolution-rate experiment

Measures what fraction of `TclOO` object-method dispatch sites
(`$var method`, `[dict get …] method`, `my method`) the semantic-token resolver
colours the method a callable (resolved) vs leaves a plain string. The
quantitative counterpart to the `tcloo_dispatch_pattern_fixture` golden test:
run before/after an object-typing change to prove a resolution-rate delta.

Harness: `rust/tcl-lsp-core/examples/tcloo_dispatch.rs`. Design context:
`docs/design/tcloo-object-typing.md`.

```sh
cargo build --release -p tcl-lsp-core --example tcloo_dispatch
# local single-file mode (each file's own hierarchy):
./target/release/examples/tcloo_dispatch <dir>...
# project mode (workspace-merged class hierarchy, the shipping server mode):
./target/release/examples/tcloo_dispatch --project <dir>...
```

Corpus: real TclOO from tcllib (clay, virtchannel, struct, oometa, oodialect),
tklib (menubar), and georgtree (SpiceGenTcl, tclopt, tclinterp) — ~275 files
containing `oo::` / `snit::` / `itcl::`.

## Baseline (before the VTA / field-typing work — Phase 2+)

| mode | receiver form | resolved | sites | rate |
|---|---|--:|--:|--:|
| local | `$var` | 15 | 7961 | 0.2% |
| local | `my` | 577 | 2352 | 24.5% |
| local | `[cmd]` | 11 | 1299 | 0.8% |
| local | **all** | **603** | **11612** | **5.2%** |
| project | `my` | 591 | 2352 | 25.1% |
| project | **all** | **617** | **11612** | **5.3%** |

## What the baseline proves

1. **The bottleneck is object *typing*, not class *resolution*.** Project mode
   (cross-file class index, PR #799's "B") lifts the rate only 5.2% → 5.3%. So
   the receivers are overwhelmingly *not typed at all* — the class can't be
   pinned to the variable — rather than typed-but-unresolvable. This is exactly
   the gap the VTA-style field + interprocedural typing (Phase 2/3) targets.

2. **`my` is the cleanest signal** (a `my` head is almost always a genuine
   object self-dispatch): 24.5% resolved. The unresolved majority is dominated
   by clay — a metaobject framework whose `my clay` / `my define` dispatch is
   genuinely dynamic (the mro_eval experiment measured clay ~0% resolvable), and
   by methods added through `oo::define` / mixins / cross-file superclasses.

3. **`$var` (0.2%) and `[cmd]` (0.8%) denominators are inflated** with non-object
   `$var cmd` calls (callbacks, command variables, `apply`) that *should* abstain
   — so their absolute *resolved counts* (15, 11) and the `my` rate are the
   meaningful signals to move, not the raw `$var`/`[cmd]` percentages.

Target for Phase 2 (field/instance-variable typing + interprocedural summaries):
lift the absolute resolved counts materially — instance-variable receivers
(`variable obj; … $obj m`) and param/return receivers are the reachable wins.

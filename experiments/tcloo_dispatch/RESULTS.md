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

## Phase 2 — VTA-lite object-flow (aliasing + constructor-param edges)

The interprocedural passes were consolidated into a single **VTA-lite fixpoint**
(`object_types::propagate_object_flow`): a name-keyed, union-join
type-propagation graph over four edges — assignment (aliasing), proc-return,
proc-parameter, and **constructor-parameter**. This adds two capabilities the
ad-hoc passes lacked: `set b $a` aliasing and an object passed *into* a
constructor and stored in an instance variable (the dependency-injection shape).

Measured on the current corpus (305 OO files, 12 535 sites — larger than the
baseline above, so compare within this block, not against the 11 612-site rows):

| mode | before | after |
|---|--:|--:|
| local `all` | 858 / 12535 (6.8%) | 858 / 12535 (6.8%) |
| project `all` | 1012 / 12535 (8.1%) | 1012 / 12535 (8.1%) |

**The corpus rate is unchanged** — byte-identical resolved counts. The edges are
*correct* (an isolated Case-B fixture goes 0/1 → 1/1 under `--project`, and three
unit tests in `object_types` lock the behaviour) but the constructor-injection /
aliasing patterns they resolve are essentially **absent from this corpus**.

The `tcloo_diag` experiment (`experiments/tcloo_diag`) explains why and redirects
the work: the unresolved mass is **snit** (`$self` 12.6% of the `$var` gap, plus
`my…` components and `$hull`) and cross-file/Tk, not within-CU `TclOO` typing.
Phase 2 is retained as sound, cheap, architecturally-correct groundwork (the
design-doc Stage-3 propagation graph); **snit support (Phase 3) is the measured
next lever**, not more within-CU edges.

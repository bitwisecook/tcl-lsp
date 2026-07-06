# tcloo_diag — unresolved-receiver provenance diagnostic

Companion to `tcloo_dispatch`. Where that experiment measures *how many*
object-dispatch sites resolve, this one asks *why the rest don't*: for every
**unresolved** `$var method …` / `[cmd] method …` receiver it categorises how
the receiver is bound across the compilation unit. The histogram says which
typing edge would move the needle next — it is the experiment that decides what
the object-typing work should build, rather than guessing.

Harness: `rust/tcl-lsp-core/examples/tcloo_diag.rs`.

```sh
cargo build --release -p tcl-lsp-core --example tcloo_diag
./target/release/examples/tcloo_diag <dir>...
```

Corpus: the same ~305 OO files as `tcloo_dispatch` (tcllib, tklib, georgtree).

## Result (single-file / local mode)

### `$var` receivers (unresolved): 8241

| category | count | share | reachable by |
|---|--:|--:|---|
| **unbound** | 5871 | 71.2% | cross-file flow / non-`set` binding / snit |
| cmd-return | 930 | 11.3% | command return-type stubs |
| param | 374 | 4.5% | interproc param (incl. cross-file) |
| collection-get | 364 | 4.4% | container element typing (done) |
| alias | 345 | 4.2% | assignment edge (done, Phase 2) |
| method-return | 274 | 3.3% | method return-type summaries |
| literal | 60 | 0.7% | — (not an object) |
| proc-return | 23 | 0.3% | proc return typing (done) |

### `[cmd]` receivers (unresolved): 1350

| category | count | share |
|---|--:|--:|
| cmd-return | 833 | 61.7% |
| collection-get | 254 | 18.8% |
| method-return | 237 | 17.6% |
| self | 26 | 1.9% |

### What "unbound" actually is — top receiver names

| name | count | what it is |
|---|--:|---|
| `self` | 1035 | **snit** `$self` (the object command) |
| `myparser` | 547 | **snit** component/typevar (`my…` ivar convention) |
| `t`, `i`, `c`, `elm`, `axis` | 220… | loop/local vars, collection element type unknown |
| `win`, `w`, `{win}.c`, `{win}.l`, `{win}.sa.table` | 183… | **Tk widget paths** (not `TclOO`) |
| `hull` | 113 | **snit** `$hull` (the hull widget) |
| `mytree`, `mystackloc`, `mycanvas`, `myeditor`, `mymap` | 63… | **snit** components |
| `options(-variable)` | 34 | **snit** option array element |
| `PROJECT`, `OBJ`, `obj`, `object` | 87… | generic object holders (params/globals) |

## What the diagnostic proves

1. **The corpus is snit-dominated, not `TclOO`-dominated.** `$self` alone is
   1035 unresolved receivers (12.6% of the `$var` gap); the `my…`-prefixed snit
   components (`myparser`, `mytree`, `mystackloc`, …) add several hundred more;
   `$hull` and `options(...)` are snit too. **snit support is the single
   highest-value lever** — and it is data-driven (a dialect model), which is
   exactly what Phase 3 scoped. `$self` is the snit analogue of `TclOO`'s `my`
   and should resolve the same way (enclosing type known statically).

2. **Tk widget-path dispatch (`$win.c …`) is a separate subsystem.** These are
   Tk megawidget commands, not object handles; resolving them means modelling Tk
   widget commands, not object typing. Correct to abstain for now.

3. **Within-CU `TclOO` typing edges are a small slice.** alias (4.2%) +
   method-return (3.3%) + proc-return (0.3%) + collection (4.4%) together are
   ~12% of the `$var` gap, and the reachable, already-typed subset is smaller
   still. This is why the Phase 2 VTA-lite edges (aliasing + constructor-param),
   though *correct* (see below), leave the corpus resolution rate unchanged: the
   patterns they newly resolve barely occur here. They are retained because they
   are sound, cheap, and resolve a real dependency-injection shape — just not the
   corpus bottleneck.

4. **`cmd-return` (11.3% `$var`, 61.7% `[cmd]`) is unmodelled-command noise.**
   Mostly non-object commands (`string`, `expr`, `format`, callbacks); a
   signature-stub pass (design-doc Stage 5) would let most of these *abstain
   explicitly* rather than sit in the denominator, and type the few that do
   return objects.

## Roadmap consequence

Priority order revised by evidence, highest corpus impact first:

1. **snit dialect model** (Phase 3): `$self` → enclosing snit type; components
   (`install`/`delegate`) → their types; `$hull`. ~1500+ receivers reachable.
2. **Cross-file object provenance**: lift `object_handle_classes` /
   `object_collection_classes` to a workspace union (mirrors
   `project_class_index`), reaching the cross-file half of `param`/`unbound`.
3. **Method return-type summaries**: `set x [$typed m]` / `[my m]` where `m`
   returns an object — the flow-sensitive VTA edge we do not yet have.
4. Signature stubs / explicit abstention to deflate the `cmd-return` denominator.

Within-CU proc/ctor/alias edges (Phase 2) are **landed and sound** but are not
where the corpus mass is; the diagnostic is what established that.

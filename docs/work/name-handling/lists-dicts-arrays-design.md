# Lists, dicts and arrays

How list, dict and array commands are implemented by the engines and modelled by analysis and the editor. Grammar-level list and dict text belongs to `tcl-value-syntax` ([scripts and words](words-scripts-design.md), [parser plan](parser-crate-plan.md)); this page is about command semantics and static models. Part of [tidying the architecture](architecture-tidy-plan.md).

**Verdict:** the engines are thin adapters over `tcl-cmd-core` for value commands and are right on nearly everything measured, with three gaps (a missing command, a missing subcommand, a stale-representation bug). Analysis is fragmented: two evaluator tiers, several index grammars, at least nine static container representations with no shared type and no dict key sets in use, `dict with` binders computed three ways, and no array/scalar misuse diagnostic.

## Measured behaviour

The 20 fixtures in [`oo-dispatch-probe/lists-dicts-arrays/`](handover/oo-dispatch-probe/lists-dicts-arrays/) cover index forms (`end`, `end±N`, `1+1`, negative, leading zeros, hex, padded, `end--1`), `lrange`/`lreplace`/`lset` edges, `lsort` options (`-dictionary`, `-integer` with hex, `-real`, `-nocase`, `-unique`, `-decreasing`, `-index`, `-indices`, `-stride`), `lsearch` options (`-all`, `-inline`, `-glob`, `-exact`, `-regexp`, `-sorted`, `-not`, `-index`, `-bisect`), `lassign`/`lmap`/`lrepeat`/`lreverse`/`linsert`, the 9.0 commands (`lseq`, `lpop`, `ledit`, `lremove`), dicts (duplicate keys, ordering, nested set, merge, filter, map, incr, lappend, `with`, `update`, `getwithdefault`/`getdef`), arrays (names, size, exists, get, unset by pattern, searches, array/scalar conflicts, `array default`), string-list shimmering and nested `lindex`. All agree across tclsh 8.6.18, 9.0.4 and 9.1.0, with the expected release differences (8.5+ index arithmetic; 9.0-only `lseq`, `lpop`, `ledit`, `lremove`, `dict getwithdefault`, `array default`; different `lset` out-of-range messages in 8.6 and 9.0).

**Execution:** 34 of 40 runs (8.6 and 9.0) match tclsh in both engines; the remaining differences are formatting apart from three faults:

- **The VM has no `lremove`** (Tcl 9.0): `invalid command name "lremove"`. It is absent from `tcl-cmd-core` and the VM, implemented privately in the runtime, and declared shipped by the registry.
- **The runtime prints an empty line for `puts $d` after `dict with d {…}` changes `d`**: `string length $d` is right, and copying `d` to another variable first prints correctly, so the direct variable-to-`puts` path reads a stale string representation after the write-back.
- **The runtime lacks `array statistics`** (visible in its subcommand list).

**Analysis:**

- **The optimiser folds none** of twelve constant list and dict operations (`lindex {a b c} 1`, `llength`, `lrange`, `lsort`, `lsearch`, `dict get`, `dict size`, `join`, `lreverse` and others).
- **W230 is accurate**: `lindex` past the end, an empty `lrange`, `lreplace` past the end appending instead of replacing.
- **False W210 after `array unset a {[xy]}`**: the pattern leaves `a(z)`, but reading `a` is reported as read before set.
- **No diagnostic for array/scalar misuse** (`set s(x)` on a scalar, `set a2 5` on an array), though Tcl always errors.
- Release gating is right: the 9.0 commands are W002 under 8.6 and accepted under 9.0.

## Semantics

- **`tcl-cmd-core` is the kernel** for list, dict, sort and search, `lseq` and arrays, with per-release option tables and two sort engines (`run_original` with release orders, and `prepare`/`merge_sort`). Both engines delegate value commands to it and implement the variable- and script-bearing commands (`lappend`, `lassign`, `dict with/update/for/map/filter`, `array set/for`) locally on the registry's shared scope plan. The engines keep separate object representations (the runtime's dict index does not use `canonical_dict_slots`; the VM's does).
- **Analysis evaluates in two tiers:** the registry's value transfers delegate to `tcl-cmd-core`; the registry's `const_fold.rs` and the compiler's SCCP `llength` fold re-implement `lindex`, `lsort` (Rust's sort, only `-ascii`/`-increasing`/`-decreasing`/`-unique`), `dict get/exists/create/merge` and `llength`, kept safe by declining unless every release agrees. `value_transfer/body.rs` hand-rolls dict key extraction for `dict with` (*read*), which `const_fold.rs`'s own comment says must use `canonical_dict_slots`.

## Index arithmetic

At least six readers inside `tcl-cmd-core` (two separate full grammars, `index/reading.rs` `read` and `index.rs` `resolve_opt_in`, plus wide-expression, compiled-literal encoding, wide-offset encoding and Jim), and outside it `tcl-bytecode`'s `parse_tcl_index` (only test callers, *read*; it accepts `end+N` for 8.4 and rejects `1+1`) and the compiler's `constant_index` for type precision (decimal only, reads `010` as 10 where 8.x reads 8). W230/W231 use one grammar, the interval pass and the `end`-offset optimisation the other.

## Options

`lsort` and `lsearch` option tables live separately in `tcl-cmd-core` and the registry; `audit_option_dialects` checks the registry against tclsh but nothing compares the two tables.

## Static models

No shared type for list shape, dict keys or array elements: the compiler's `TypeShape::List/Dict` (dict shapes describe values, not keys), the registry's `ValueShape` (the only dict-key-set type, never constructed, *read*), `ReturnElements` and element effects, the interval pass's list-length map, SCCP constants and the legacy `llength` fold, the analyser's literal lengths for bounds checks, two `PlaceKind` enums (compiler `ArrayElem/ArrayWhole/DictPath`, registry `Scalar/Element/Array`; `DictPath` places are built only by `resolve_dict_path`, which has no production caller, *read*), the editor's array-index candidates and whole-value taint. The editor shows only coarse `list`/`dict` inlay types, offers no dict-key hover or completion, and `switch_to_dict` works on syntax.

`dict with` binders are computed three times: the registry's `DictWithSemantics` (key paths, hand-rolled keys), the analyser's handler (no-path form and constant dicts only, recording keys as read-only names without a declaration span, which is why go to definition does not follow `dict with` keys, as [variable lookup](variable-lookup-centralisation.md) measured), and command binding with the registry's dictionary scope plan (which also drives both engines).

## Conformance

Tcl's own `list.test`, `lindex`, `lsearch`, `lset`, `lpop`, `lseq`, `lrepeat`, `dict.test` and `cmdIL.test` results are captured per release; index encoding, `lsort` ordering and dictionary bodies have engine vectors; the fuzzer generates list, dict and array statements. All of these exercise the engines. The analysis folds, shapes and interval lengths are not checked against them.

## Plan

1. **Fix the engines:** `lremove` in `tcl-cmd-core`, used by both engines (retire the runtime's private copy); the runtime's stale string representation after `dict with`; `array statistics` in the runtime; add the fixtures to both engine suites.
2. **One evaluator:** route the registry's `const_fold.rs` and the SCCP `llength` fold through `tcl-cmd-core` (as value transfers already do), keeping the all-releases-agree rule; use `canonical_dict_slots` in `body.rs`; let the optimiser fold proven constant list and dict operations through the same evaluator, with the engine vectors as its oracle.
3. **One index grammar** in `tcl-cmd-core`, per release, used by every reader (merge `read` and `resolve_opt_in`), by W230/W231, the interval pass, the `end`-offset optimisation and type inference; delete `tcl-bytecode`'s `parse_tcl_index`.
4. **One option table per command** (`lsort`, `lsearch`, `dict`, `array`) in the registry, from which `tcl-cmd-core`'s release tables are generated or checked, under `audit_option_dialects`.
5. **One container model:** a value shape with list length and element shapes, dict key sets and value shapes, array element sets, used by SCCP, the interval pass, type inference, taint (per element where proven), W230, the editor (dict-key hover and completion, shape inlay hints) and `switch_to_dict`; one `PlaceKind` including dict paths, wired into production.
6. **One `dict with`/`dict update` binder model** in command binding, giving declaration spans so navigation follows keys; fix the false W210 after `array unset` with a pattern; add an array/scalar misuse diagnostic from the registry's existing completion outcomes.
7. **Analysis conformance:** run the captured Tcl test results and engine vectors through the analysis folds, shapes and interval lengths.

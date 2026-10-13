# Which reader context selects a lazy variable's producer?

The unchanged pinned Tcllib `lazyset` package selects an unqualified producer
in the namespace of the procedure that first reads the traced cell. Moving
that read from `::install` into `::reader` selects the reader's shadowing
`produce`. Qualifying the stored producer restores the installation target.
In every reached case the producer receives the local access spelling
`reader alias`, rather than the installation spelling `cell`.

## Real source and semantic question

The maintained corpus contains
[Tcllib lazyset.tcl:38–83](https://github.com/tcltk/tcllib/blob/8a2e288fa02363639784c2f0a8ea6245896c2884/modules/lazyset/lazyset.tcl#L38),
revision `8a2e288fa02363639784c2f0a8ea6245896c2884`. Its generated `apply`
callback receives the trace's `name1`/`name2`/operation, appends selected name
arguments to the stored prefix, evaluates it through `uplevel 1`, then unsets
and sets the accessed variable in that frame. This is a real definer with
generated callback code and a deferred name operation, distinct from ordinary
namespace-captured command prefixes.

[Source provenance](source-provenance.json) retains manifest, full-file,
selected-range and licence hashes. The [unchanged source](upstream/lazyset.tcl)
and [upstream licence](upstream/license.terms) are retained. The source's
documentation identifies Roy Keene's 2018 copyright; no new licence header
was inserted into upstream bytes.

The new [Tcl Wiki trace context](https://wiki.tcl-lang.org/page/trace),
“Variable Trace Arguments vs Upvar”, independently demonstrates changed
access spelling through `upvar` and an explicitly captured name in an `apply`
callback. [Selected retrieved bytes](wiki-trace-excerpt.txt) and their hash
are retained. They are web-tool rendered source text, not raw HTML or a
verified revision. Direct HTTP retrieval returned 403; it supplies no content.
The displayed Updated date was 2026-02-17 19:49:52. This Wiki context does not
contain the exact authored lazyset caller below.

The question is: **After unchanged successful setup, which producer tag,
fixed argument and accessed variable spelling does the final read return?**
Each [package caller](probes/package-install-reader.tcl) sources the unchanged
package. The [adverse caller](probes/package-shadow-reader.tcl) changes only
the final read from `::install::read` to `::reader::read`. The
[fixed caller](probes/package-qualified-fixed.tcl) additionally qualifies the
stored producer. Both procedures link the same global cell through a local
name containing a space. The fixed argument also contains a space.

Three separately authored `core-*` probes measure a write-trace callback
that uses `uplevel 1` with the same producer prefixes. They neither source
nor alter lazyset and do not establish its read, one-time initialisation,
array, generated-apply or writeback semantics.

## Exact provider observations

`I` means `install {a b} {reader alias}`; `R` means
`reader {a b} {reader alias}`. Returned values have completion code zero.

| Exact inputs | C8.4.20 | C8.5.19 | C8.6.18 | C9.0.4 | C9.1.0 | Jim `0.84-9-g5bac7c9` | F5 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| package install / shadow / qualified-fixed | setup refusal / refusal / refusal | I / R / I | I / R / I | I / R / I | I / R / I | setup refusal / refusal / refusal | not run |
| separate modern core install / shadow / qualified-fixed | I / R / I | I / R / I | I / R / I | I / R / I | I / R / I | setup refusal / refusal / refusal | not run |

C8.4 refuses the unchanged package's `package require Tcl 8.5 9` argument
form; Jim refuses that same form. These refusals precede `apply`, trace
registration and the final read, so they answer none of those later questions.
Jim's core setup refuses `invalid command name "trace"`; no alternate trace
or package implementation was substituted. The Jim binary matches the retained
upstream revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`; no fresh upstream
HEAD claim is made.

[Final original records](results-trace-add/records.json) and
[provider provenance](results-trace-add/provenance.json) retain all 36 exact
captures, full source/driver/setup/question bytes, raw stdout/stderr, binary
hashes, reported patchlevels, matching C `TCL_LIBRARY` selectors and startup
source hashes. Return options are captured where supported; C8.4 retains
message/errorCode/errorInfo through native two-argument catch. Every process
exited zero; guest completion and reached stages are separate fields.

[Earlier exact captures](final-results/records.json) retain the first authored
core inputs using legacy `trace variable cell w`: C8.4–8.6 reached I/R/I,
but C9 refused that removed command form. The final modern core inputs change
only the trace registration form. An even earlier driver provenance attempt
stopped before any probe because C8.4 has no `generic/tclTrace.c`; its
[refusal](results/driver-refusal.json) and startup bytes remain retained.
[run-captured.py](run-captured.py) matches both completed captures' runner hash;
[run-first-attempt.py](run-first-attempt.py) retains the failed runner. The
[replay runner](run.py) adds only the repository licence header.

## Recognition and consumer boundary

This pattern matters to callback target navigation and reference search,
callback arity advice, rename and call simplification. A recogniser needs the
actual lazyset implementation/source binding, selected scalar/array and
append-argument branches, exact prefix words, trace access spelling, triggering
frame/namespace and command bindings at the read. An installation namespace or
registration spelling alone cannot select the later producer or identify the
trace's `name1`. Qualifying the producer still requires its live binding;
qualifying the variable does not freeze the caller's local alias spelling.

Current selected trace descriptors in
[trace.rs](../../../../rust/tcl-registry/src/commands/tcl/trace.rs) supply prefix
positions and appended argument counts. The
[structured timing contract](../../../../docs/design/analysis/name-resolution-proofs/original-structured-script-timing.md)
keeps those source roles separate from native callback frames and execution.
The existing callback signature owner uses a registration-horizon lookup;
these package-deferred `uplevel` targets require an independent reader-context
premise. No Rust recogniser was run here, no lazyset summary was added, and
no canonical proof was registered. These are supplemental provider observations,
with no consumer accuracy or edit/optimisation grant.

Array elements, empty indices, repeated reads, multiple traces, prefix
replacement, cell unset/recreation and arbitrary prefix scripts remain outside
this bounded question. Native F5 contexts remain [unanswered](F5-REQUEST.md).

Replay with all supplied original shells and a fresh output directory:

```sh
python3 experiments/name-handling/patterns/lazyset-reader-context/run.py --dest /workspace/tcl-lsp/experiments/name-handling/patterns/lazyset-reader-context/new-capture
```

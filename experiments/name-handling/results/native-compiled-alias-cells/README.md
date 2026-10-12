# Original compiled namespace alias cells

[The question](input.tcl) creates a namespace variable, reads it through a
procedure-local `variable` link, stores through a second procedure and reads it
again. [The C/Jim wrapper](c-input.tcl) prints that final list; runtime captures
execute the same wrapper. All five real C releases and Jim revision
`5bac7c99ad65864c87da513e22e2f01703fa4e03` return `ready changed changed`.
[The C records](c-records.json) and [Jim record](jim-record.json) retain exact
inputs, binaries, process status and streams. This public source question does
not observe C private cell pointers or establish hosted F5 semantics; F5 has
not been queried for this exact input.

The native runtime must select and observe its installed local ordinal before
publishing the namespace alias. A rendered name cannot select that cell,
especially when compiled names are duplicated. The observation retains the
actual frame/table and weak cell identity and checks the same ordinal after
callbacks; it cannot adopt a replacement frame or cell.

## Scoped implementation results

The frozen native runner SHA-256 is
`2aca8d769e3154c6c2235868d21470760975506f5ed33c0ab376e3c611b18dc9`.
It contains the compiled-alias observation changes and the concurrently reviewed
variable-arity/array-name-mode changes. These captures describe that combined
binary; they do not isolate which change explains every newly reached case.

- [The compiled-release integration test](compiled-release-test.log) passes
  against actual C Tcl 8.5.19, 8.6.18, 9.0.4 and 9.1.0. Those releases compile
  namespace-variable linking to installed local ordinals.
- [The physical-cell unit test](ordinal-cell-test.log) checks that duplicate
  names do not select another ordinal and that popping the captured frame
  invalidates its observation. This is an implementation identity check, not a
  private C pointer observation.
- [The initial all-release test](initial-all-release-test.log) stops at Tcl
  8.4. Its ordinary-variable path still refuses an actual indexed receiver.
  The later scoped test covers compiled-variable linking from 8.5; the 8.4 gap
  remains explicit in the fresh raw runtime capture.

[Runtime records](runtime-records.json) retain both the direct question and real
matching `init.tcl`/`tcltest.tcl` startup. C startup controls in each release's
`startup-c/` directory execute the same startup question and pass its one test.
No library or harness was replaced with a reduced implementation.

| Runtime release | Direct question | Real Tcltest startup |
| --- | --- | --- |
| 8.4.20 | Indexed-variable receiver refusal | Same refusal; test unreached |
| 8.5.19 | `ready changed changed` | One test passed |
| 8.6.18 | `ready changed changed` | Init refuses original command-entry epochs; test unreached |
| 9.0.4 | `ready changed changed` | Original simple-variable cell refusal; test unreached |
| 9.1.0 | `ready changed changed` | Original simple-variable cell refusal; test unreached |

The 9.x startup runs reach beyond the earlier local-alias-table refusal. A host
refusal remains outside Tcl completion handling and supplies no Tcltest verdict.
Jim's real reference answer above does not establish our runtime's Jim result.

## Upstream namespace coverage

[The exact Tcl 8.5 `namespace-8.1` sweep](namespace-8.1/) loads the real init and
Tcltest files in both C and the native runtime. Both pass the selected case,
with the other 280 cases skipped by the explicit match filter. This case uses a
child interpreter and links a procedure local to its namespace variable.

[The full namespace sweep](namespace-full/) has 280 C passes, one skip and no
failures. The runtime records seven assertion failures:
`namespace-8.5`, `namespace-8.6`, `namespace-10.6`, `namespace-10.7`,
`namespace-14.13`, `namespace-50.1`, `namespace-50.2`.
It then refuses a captured original command binding during `namespace-51.13`.
There is no runtime summary or complete suite pass count; remaining cases are
unmeasured. The retained stdout identifies the assertion results independently
of the later host refusal.

[The cached/fresh namespace question](cached-fresh-input.tcl), after real 8.5
startup, now returns `OBS {{parent 1 1} {child 0 0}}`, matching the retained real
C observation in [the original lifecycle capture](../runtime-lifecycle-fix/native-ensemble-fix/8.5.19/cached-fresh-namespace-probe.stdout).
Its [new runtime record](cached-fresh-record.json) supplies the exact binary,
input and invocation. This ordinary source result does not assert a particular
private namespace-object primary or replace the independent object-cache tests.

## Inputs and source currency

[The runner source](runner-source.txt), [scoped test source](scoped-test-source.txt),
[reviewed source patch](source.patch), [inspected source hashes](inspected-source-sha256.json)
and [build log](runner-build.log) retain the implementation context. The inventory
was inspected after the build; it is not an independently authenticated complete
Cargo build receipt. The patch includes tracked changes from both runtime jobs;
new authored files are identified by their inspected hashes and remain in the
same branch. Each execution record identifies its own immutable input and binary.

These observations supplement the canonical name-resolution proof authority.
The existing compiled-local inventory and namespace-alias settlement questions
retain their own exact provider inputs; this positive control does not silently
answer their distinct counted-name, callback or private-header questions.

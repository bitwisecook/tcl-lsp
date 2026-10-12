# When does `array names` validate its mode?

Does `array names a -bad x` fail when `a` is missing, scalar, empty or
populated? Does an invalid regular expression fail without a candidate key?
The discrete [cases](cases/) are fresh-interpreter Tcl inputs. They print the
reached catch code, result and global error code; no implementation output is
used as an oracle. Unique mode prefixes, an ambiguous prefix, a sole trailing
dash word and both argument-count boundaries are separate controls.

Run `source /tmp/name-env.sh` and `python3 capture.py` from the repository root.
The [capture program](capture.py) retains exact stdin inputs, subprocess argv,
raw stdout/stderr, exit codes, reported patchlevels, matching C libraries and
input/binary/source hashes in [captures](captures/). These are source-built
C 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0. The latest upstream Jim available
in this session is revision `5bac7c99ad65864c87da513e22e2f01703fa4e03`.
Its independent grammar admits only `arrayName ?pattern?`.
The [original Jim provenance](../results/jim-0.84-9-g5bac7c9/provenance.json)
and [configured extension snapshot](../patterns/jim-results-5bac7c9/jimautoconf.h.txt)
retain the build configuration; the oracle binary hash in each capture selects
the actual executable. That earlier provenance records upstream HEAD discovery
on 12 October 2026; this capture does not repeat remote discovery.

| Input/target | C 8.4.20 / 8.5.19 | C 8.6.18 / 9.0.4 / 9.1.0 | Jim |
| --- | --- | --- | --- |
| Missing or scalar, `-bad x` or ambiguous `- x` | Empty success | Bad/ambiguous option error | Wrong argument count |
| Empty or populated array, `-bad x` | Bad option error | Bad option error | Wrong argument count |
| Missing, scalar or empty array, `-regexp {[}` | Empty success | Empty success | Wrong argument count |
| Populated array, `-regexp {[}` | Regexp compile error | Regexp compile error | Wrong argument count |
| Populated array, `-e x`, `-g x`, `-r x` | Returns `x` | Returns `x` | Wrong argument count |
| Populated array, sole `-bad` trailing word | Empty glob result | Empty glob result | Empty glob result |
| `array names` without a name | Ensemble usage error | Names-worker usage error | Names-worker usage error |

The [source excerpts](source-excerpts/) retain the relevant workers and line
numbers. In C 8.4/8.5, non-array return precedes mode validation; pattern string
materialisation precedes original mode-index selection. C 8.6+ locates the
array, validates the mode, returns for a non-array, then materialises the
pattern. Regex compilation happens inside candidate enumeration in every C
worker. The ordinary Tcl inputs establish completion behaviour; getter order
is a source observation, not an executed custom-object experiment.

The frozen native runner has SHA-256
`571dcc78f4b0b7f4e21f2850bceca2d61813440718a63d7cb01da751f752c473`.
Its captures use `--quiet --tcl-version` with **no init**. Build evidence is
in [native-c84-declared-cell](../results/native-c84-declared-cell/), and these
measurements do not attest to the subsequently editable tree. All five native
profiles return empty for missing/scalar bad modes, disagreeing with C 8.6+.
They explicitly refuse the regex worker even for empty arrays; the actual C
controls return empty without compiling a regex. A process-level refusal is
retained as exit code 2 and stderr, not represented as a caught Tcl error.

## Does lookup or argument validation win over an array trace?

The separate [trace inputs](traces/) record registration first, so unsupported
registration is not misreported as a reached lookup. All five C releases fire
the array trace before validating a mode on an otherwise valid argument count.
A trace error wins over a bad mode, and a trace that creates a missing array
causes C 8.4/8.5 to validate the mode after the creation.

For too many operands, C 8.4/8.5 fire the trace before reporting wrong arguments;
a failing trace wins. C 8.6+ report wrong arguments without firing the trace.
The frozen native profiles all skip this trace, including 8.4/8.5. This is a
separate adapter admission question. Jim lacks `trace` in this actual build;
its records end with the failed registration, leaving lookup unexecuted.

## Shared-owner behaviour

`tcl_cmd_core::array::names` uses its existing native grammar policy to
distinguish C 8.4/8.5 from C 8.6+, retaining the actual policy issuer. For
C 8.6+, it selects the original mode after lookup but before the non-array
return. For C 8.4/8.5, it retains the non-array return before mode selection
and pattern-before-mode conversion. C 8.6+ retains mode-before-pattern
conversion and skips pattern conversion on a non-array.

Mode acceptance and regex-worker execution are distinct: the worker returns
the observed empty list when there are no candidate keys and requests the
currently unavailable original regex worker only when enumeration reaches a
candidate. Adapter trace admission remains a separate recorded gap.

[capture_regression.py](capture_regression.py) mechanically replaces each
`puts [list ...]` with row accumulation. It independently executed all 222
adapted inputs on the six real oracles and verified identical stdout/stderr
and completion against the original captures. The [regression vectors](regression-vectors/)
retain the actual original stdout with line terminators escaped for one row
per input; no native output supplies an expected value.

The existing focused runtime integration test
`runtime/rust/tests/native_variable_array_modes.rs` consumes these vectors.
It covers the admitted C/Jim mode boundaries and checks the explicit refusal
for C regex enumeration with populated arrays. Original Jim trace questions
remain unreached, and the independent C 8.4/8.5 no-name and excess-arity trace
admission gaps remain outside that conformance assertion.

[runtime-test.log](validation/runtime-test.log) records the fresh native
integration executable passing all three focused tests, including both
pre-existing variable/mode matrices. The new boundary test passes 200 exact
oracle-row comparisons. Ten populated-regexp checks establish explicit host
refusal, independently of C conformance. The
[execution record](validation/runtime-test-record.json) retains the executable,
input, source and log hashes with each input's reached validation scope.
Before/after build inventories covering 4,938 Rust/runtime source and Cargo
files are identical. The existing C 8.5 `var.test` 69-pass capture is preserved;
that upstream suite was not rerun for this executable. F5, VM, custom-object
getter execution and workspace/CI gates remain unmeasured by this experiment.

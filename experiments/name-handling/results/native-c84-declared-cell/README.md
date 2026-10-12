# Named lookup of an existing compiled local cell

An installed compiled local has a physical cell even before it stores a value.
Named lookup must observe that selected cell before alias publication. The
cell's binding identity is distinct from a stored Tcl value reference or a
namespace/hash-key reference; observing it grants neither.

[The marked question](marker.tcl) declares a namespace variable, invokes a
procedure, links its compiled local through `variable`, then reads it. [The
literal-result control](literal-control.tcl) keeps the namespace link but has no
lexical read of that local. Both real C Tcl 8.4.20 controls return `RESULT ready`.

[The exact captures](before-records.json) identify the earlier native runner
and its input hashes. The marked native question prints `CALLED` then refuses
an actual indexed-variable receiver before `LINKED`. The literal-result control
passes. This is an operation refusal, not an ordinary Tcl error or completed
Tcltest assertion.

The runtime's complete compiled-local inventory creates an undefined physical
cell for the lexical read. `prepare_native_name_cell` selects that existing cell;
its observation requires its binding identity. The owner must materialise that
identity without creating a missing cell, inserting a hash entry, acquiring a
stored-value role or replacing the selected ordinal. Duplicate names continue
to follow the independently selected compiled-name comparison rule.

[The earlier public read/store/read question](../native-compiled-alias-cells/input.tcl)
and its [five C records](../native-compiled-alias-cells/c-records.json) retain the
same ordinary namespace-link behaviour across every supported C release.
The [Jim result](../native-compiled-alias-cells/jim-record.json) answers that
exact public input independently; it does not prove our runtime's Jim result.
F5 has not been queried for these exact inputs. The existing canonical proof
questions retain their independently scoped provider inputs.

## Current native observations

[The physical-cell control](physical-cell-test.log) passes under all five
selected C protocols. Missing-name lookup with creation disabled leaves the
cell inventory unchanged; observing an existing duplicate name selects only its
first actual compiled ordinal, adds no hash key or stored value, and preserves
the other physical cell.

[The original public comparison](all-c-release-test.log) now passes against all
five real C releases, including 8.4.20. This expands the earlier compiled-link
comparison without replacing its retained older captures.

[The new runtime records](after-records.json) use frozen native runner SHA-256
`571dcc78f4b0b7f4e21f2850bceca2d61813440718a63d7cb01da751f752c473`. Both marked and literal-result questions
return `RESULT ready`; the marked question now reaches `LINKED`. Real matching
Tcl 8.4.20 `init.tcl` and `tcltest.tcl` startup loads Tcltest 2.2.11 and passes its
one namespace/procedure/alias test. The matching C control is retained under
[the previous exact startup capture](../native-compiled-alias-cells/8.4.20/startup-c/).
This one bootstrap question establishes no full 8.4 suite result.

[The owner source](owner-source.txt), [integration test source](integration-test-source.txt),
[runner source](runner-source.txt) and [build log](runner-build.log) retain the
executed implementation context. [The before](source-before-build.json) and
[after](source-after-build.json) inspected authored-source inventories are
identical; [currency metadata](source-currency.json) records the compact build
settings. Concurrent compiler-only diagnostics did not participate in this
runtime's dependency graph. These inspected hashes are not an independently
issued complete Cargo build receipt.

The separate runtime variable/array matrix was rerun against the same native
library. Its first attempt failed to compile a test's private `Code::as_int`
call, before any assertion. [The corrected test run](../native-variable-array-modes/runtime-max-arity-test-fixed.log)
passes both original-oracle matrix tests; the [compilation failure](../native-variable-array-modes/runtime-max-arity-test.log)
remains retained independently. That test-only correction does not change the
frozen runner or its source-currency result.

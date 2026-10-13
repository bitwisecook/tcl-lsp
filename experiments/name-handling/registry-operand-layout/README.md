# Original namespace-worker operand layout

`probe.tcl` declares fixed braced procedure bodies and executes the actual
namespace-current command through canonical, wider-separator, public and renamed
spellings. Extra-operand cases evaluate an independent counter increment. The
captured command status, error text and counter distinguish a genuine arity
failure from a compiler that drops an operand or its evaluation.

`capture.json` records the source-built shell identity, exact patchlevel, input
and raw output hashes and exit code for each C8.4–9.1/Jim provider. Toolchain
pins/provenance are shared with `../registry-name-adapters/capture.json`. These
custom probes use no Tcltest or replacement upstream library.

C8.6, C9.0 and C9.1 execute both original private zero-operand spellings and the
renamed worker, returning the global namespace. Every extra-operand form fails
with wrong arity after incrementing the counter once. The wider spelling has
the same argument layout as the canonical private worker. The renamed worker
retains this behaviour even though its source head has no private namespace
prefix. Public `namespace current` also evaluates its extra operand once before
reporting wrong arity.

C8.4, C8.5 and Jim have no measured private namespace-current worker; both private
spellings fail lookup, with the extra operand still incrementing once. Rename
admission fails, so renamed cases are unreached. Their public extra-operand case
reports wrong arity after one increment. These absences do not establish another
provider's compiler hook or original worker path.

This capture establishes oracle expectations only. It does not establish
Compiler, Core, VM, Runtime or generated-backend conformance. Original consumer
controls must retain their genuine source, context, selected registration,
compiler-hook and operation-currency prerequisites and compare the actual
status/result/effect vector.

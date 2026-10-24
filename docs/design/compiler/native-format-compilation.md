# Native format concatenation compilation

The registry models the string-concatenation branch of `TclCompileFormatCmd`
from C Tcl 8.6.18, 9.0.4 and 9.1.0. C Tcl 8.4.20 and 8.5.19 register no
compiler hook for this command.

The supported branch requires an original literal ASCII format, without a
NUL byte, containing only `%s` and `%%` conversions. At least one value operand
must require substitution. There must be exactly one value operand per `%s`
and at most 125 value operands. Expansion, missing source shapes, an unknown
compiler frame, and a non-bytecode entry remain unavailable.

The C compiler first examines static source words. Once it finds a dynamic
value, it checks the format and count before compiling every value word in
source order. The registry retains those visits through the existing original
word-preparation owner. The literal command head and format are not value-word
visits. Nested substitutions and local reservations remain obligations of the
existing compiler visitor; this recipe introduces no script or expression
body visits.

The all-static path invokes `Tcl_Format` while compiling and may emit a syntax
error. That path remains unmodelled. Other conversion or count cases remain
unresolved except for the exact C9.0 generic decline described below.
The descriptor supplies no handler completion, native execution entry, command
lookup, currentness, namespace, or backend-provider authority. Those remain
independent requirements of the original invocation owner.

The portable instruction recipe retains nonempty format fragments and original
value-word indices in their C compile order. `%%` contributes a literal percent.
C8.6/C9.0 use the native registered-byte literal owner. C9.1's `PUSH_OBJ` expands
to `TclAddLiteralObj`, so each fragment uses the existing private String-header
owner without local or global literal sharing. Literal registration interleaves
with original value compilation; fragments are not registered ahead of their
position in the instruction stream.

Two or more pieces emit `STR_CONCAT1` with their exact count. A single `%s`
passes through its original value object without string conversion or a new
header. The runtime artifact uses the existing compiled-concatenation owner,
retaining the genuine object operation through callbacks and operand retirement.
This string operation never uses Tcl list-concatenation rules. Unknown source,
compiler selection, frame, operation or original implementation still refuses
through the existing admission checks.

## Native C9.0 dynamic hexadecimal compiler decline

For the exact literal `%x` format and one substituted value, the C9.0 format
compiler's literal probe fails and its conversion scan declines. The ordinary
compiler then prepares the original head, format and value words in order. The
registry retains an empty attempted-hook prefix and the enclosing compiler's
ordered word visits separately. It emits no format opcode and grants no numeric
result or operand conversion. Runtime command selection and the
original format handler retain their own guards and errors.

All-static formatting, other conversions, unknown formats, other releases,
uncertain arity, expansion, opaque words and unavailable compiler contexts remain
unresolved. The existing `%s`/`%%` concatenation instruction keeps its separate
literal allocation and retirement contract.

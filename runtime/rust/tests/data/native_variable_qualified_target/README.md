# Qualified variable declarations

The source observes each qualified top-level declaration operation in the
same interpreter, followed by a procedure-qualified declaration. The native
outputs retain the C Tcl and Jim distinction: C stores the top-level value
in `::a::x`, while Jim creates the unqualified `x`. Inside a procedure, the
local `x` write through the declared link yields `7` in every engine.

The manifest records the original command sequence, its expected unqualified
read, and the actual six-engine outcomes. The Runtime tests use authentic
per-version native constructors and compare the complete output.

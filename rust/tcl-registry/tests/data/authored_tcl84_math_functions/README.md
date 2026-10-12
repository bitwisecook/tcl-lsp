# Authored Tcl 8.4 math controls

The original scripts and captured outputs describe Tcl 8.4 function results,
fixed-name and arity validation, operand effects, numeric width, seeded random
sequences, and lazy precision conversion. The manifests retain the native
binary, library, source and header identities and the sixty-second capture
budget. Scripts and observations are unchanged native inputs and outputs.

`AuthoredMathFunctionProvider::Tcl84Core` supplies an explicit simulation of
this function surface. Its preparation checks the unchanged expression tree
and fixed function names and arity before runtime operands. This pure traversal
creates no physical compiler visit, literal registration, actual function table
or native object-header permission. Physical child compilation remains subject
to the actual engine's independent original-source compiler prerequisites.

Each actual simulation interpreter owns a separate Park–Miller stream and
precision context. Initial seed one and precision twelve are explicit authored
model choices. TMM workers use their actual child interpreter owners. Host
activations retain their separate native random stream and formatter. Precision
is read when a Double first obtains its string representation; an existing
string retains its bytes.

The VM tests compare the complete native outputs and check removal, cache
invalidation, child state and host restoration. LiveSession tests exercise the
installed provider through actual TMM child interpreters. Parser and numeric
capabilities alone leave the function provider absent, and a runtime without
this provider retains its typed refusal. These controls establish no BIG-IP
appliance seed, precision, physical header or compiler identity.

# Native compiler and namespace resolver epochs

The probe reads the actual private interpreter, namespace, and command headers
from Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, and 9.1.0. Each row follows the completed
native operation and reports its completion, interpreter compiler epoch, global
and local resolver epochs, local command-reference epoch, and raw `set` compiler
attachment. The manifest pins the source, private headers, and linked libraries.

Compare compiler epochs relative to the initial row of the same interpreter.
Native bootstrap registrations produce different initial counters across Tcl
releases; a counter from another interpreter is not a valid cache receipt.
Namespace identities are distinct even when deletion and recreation reuse a
public name. Namespace resolver counters restart with the new identity.

Run `python3 run.py --tcl-root /path/to/tcl-source-parent --output /tmp/epoch-proof`
to compile and execute the probe against all five native source trees. The output
directory must be new. Rust owner tests consume the retained rows and compare the
actual command mutation doors and independent counters.

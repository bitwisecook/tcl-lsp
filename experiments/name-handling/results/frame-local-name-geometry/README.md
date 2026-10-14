# Frame-local variable-name geometry

These fixed public source cases use C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4,
9.1.0 and the Jim executable built from source revision
`5bac7c99ad65864c87da513e22e2f01703fa4e03`. `results.json` retains exact
argv, executable/source SHA256, exit codes and complete stdout/stderr. The
source bytes are in `probe.tcl`; no tcltest or substitute support library is
used.

Every process rejects an empty formal, errors on an unbound empty variable,
and returns VALUE after writing the empty local. A procedure's empty local
can shadow an existing empty global without modifying it; an unbound procedure
read does not inherit that global. Literal `$name` formals bind `$name`, while
`name` and `$name` remain distinct. C rejects a relative `inner::value` write
when its namespace is absent; Jim stores that exact relative name locally.
The C8.4 empty-formal error includes the procedure name, unlike the other
captured processes.

The shared `source_variable_name_is_frame_local` consumes the independently
selected source variable-lookup policy. Original Logical inlining controls
check empty and dollar-name binding before eligibility. These observations
establish no Native allocation identity, compiler opcode, activation, rewrite
or inlining equivalence. F5/TMM behavior is unobserved.

The six-case and eight-case source/raw captures remain separate. The
source-retention producer error and unchanged eight-case rerun are recorded in
`producer-error.txt`; the final capture adds the independent global/local
empty-name controls.

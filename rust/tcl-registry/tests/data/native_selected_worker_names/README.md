# Native selected worker reporting names

`probe.c` installs a genuine C ensemble compiler with `Tcl_SetEnsembleFlags`.
The ordinary script mapping in namespace `a:` reports `::a:::w` and resolves
the command in namespace `a`, so its compiled and printed-name calls both
return `collision`. Tcl 8.6, 9.0 and 9.1 additionally receive an original map
name object whose real `cmdName` cache retains the worker in namespace `a:`.
The compiled call returns `selected`, while looking up the same reported
bytes returns `collision`. The map object and command token are retained by
their native owners; no regenerated source is used to select that worker.

Tcl 8.5 supplies the ordinary mapping control. Its no-hook compiler declines
rather than emitting the Tcl 8.6 named worker recipe. Tcl 8.4 has no ensemble
compiler. `jim.tcl` records the current Jim namespace ensemble surface, which
does not provide the C compiler/cache protocol.

The fixture contains eight successful C observations and the Jim surface
observation. `manifest.json` records exact source, engine library and executable
hashes, compile arguments and exit statuses. Compile with the selected engine's
generic/unix headers and static Tcl library, linking `-ldl -lz -lpthread -lm`;
run with `TCL_LIBRARY` set to that engine's library directory.

A retained compiler-selected command-name priming receipt validates the original
public configuration and worker token separately from reporting bytes. A byte-only
ensemble map snapshot does not preserve an incoming physical map object's
`cmdName` cache; that selection requires its own original object observation.

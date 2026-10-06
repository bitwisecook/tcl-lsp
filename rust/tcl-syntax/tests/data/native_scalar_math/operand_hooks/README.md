# Original native numeric operand hooks

These 140 deterministic observations retain a custom original scalar object
through the selected conversion. They cover 24 Increment rows (all six engines,
updater/free hook, direct and alias routes), 96 implicit math rows (`sqrt`, `sin`,
`double`, `abs`, all six engines and both routes), and 20 C math-selection timing
rows. Each hook changes a current procedure-local `keep` and the command table;
the conversion still produces its normal native result. These are native object
callbacks, independently of Tcl variable/execution observers.

The timing probe replaces the registered sqrt implementation during conversion.
The current invocation uses the previously selected implementation; subsequent
calls see the replacement. Thus implementation identity and normal result shape
do not imply original-input world preservation.

Build the C sources against each pinned Tcl tree and static library:

```sh
cc -std=c11 -I "$TCL_TREE/generic" -I "$TCL_TREE/unix" probe.c \
  "$TCL_TREE/unix/libtcl8.6.a" -lpthread -ldl -lm -lz -o increment-hooks
./increment-hooks "$TCL_TREE/library"
```

Substitute `math-hooks.c` or `math-timing.c` for the other C probes and the actual
ABI library name. Build Jim sources against the pinned Jim tree and `libjim.a`,
with `-ldl -lm`; those binaries take no arguments. Manifests bind every recorded
output to source, native library and probe binary hashes. `fixture-hashes.json`
also binds the durable copies. These observations grant no numeric operand
acceptance, current cache, ordinary class or object identity to unknown source
values. Registry input descriptors inventory the conversion obligations; source
owners must close them from the actual original objects at their reached phases.

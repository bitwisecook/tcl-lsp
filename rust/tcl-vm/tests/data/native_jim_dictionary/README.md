# Jim scripted dictionary workers

The pinned Jim 0.84 `stdlib.tcl` installs seven real multiword procedures:
`dict update`, `dict replace`, `dict lappend`, `dict append`, `dict incr`,
`dict remove`, and `dict for`. `stdlib.tcl` preserves the original library
source; the `*-body.bin` and `*-args.bin` files contain actual native
`info body` and `info args` results, including tabs and newlines.
`manifest.json` records source/executable hashes and exact observations.

Run any `.tcl` control with the matching native Jim executable:

```sh
/path/to/pinned/jimsh update-missing.tcl
```

Each control captures completion before output and prints its code followed
by its original result. `observations.tsv` contains the same code/result
and the original script as hexadecimal bytes. The Rust dictionary tests
compare all thirteen executable observations and fourteen body/formal
results.

Jim's core dispatcher resolves a selector and then constructs the multiword
command from the original selector's CString spelling. Abbreviation `up`
therefore looks up `dict up`. The delegated command remains replaceable and
deletable. An accepted `update` call with malformed key/variable cardinality
forwards zero arguments; the real procedure owns the resulting arity check.
Missing keys retain existing caller variables, and the procedure boundary
can consume a body `return` before the caller continues. These contracts
apply to Jim's scripted implementation; C dictionary primitives have their
own selected scope and writeback recipes.

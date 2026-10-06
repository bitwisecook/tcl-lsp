# Native bytecode literal ownership

`probe.c` records original callback argument identity and cache before/after
selected conversions, without error-state observers. Five pinned C Tcl
engines provide 105 rows; `manifest.json` retains source, executable, header,
and library hashes and exact compiler commands.

Build against the matching native Tcl library:

```sh
cc -I/path/to/tcl/generic probe.c /path/to/libtcl.a -lm -ldl -lpthread -lz -o probe
./probe
```

The columns are operation, call index, initial cache, final cache, same
object as the retained first argument, identical two List children, same
first child across calls, actual original object refcount, and resident
string bytes in hexadecimal. Retained observer references are real owners;
no pointer equality is used after its object's lifetime.

Interpreter-global string literal entries own one object reference plus
references from each bytecode object's local array. Repeated activations and
separate retained bodies share the same registered data object. Once all
local arrays retire, a guest-held former literal does not keep its global
registration alive: a new body gets a new literal object.

C8.4 and C8.5 build nonempty List results dynamically. C8.6–9.1 retain a
private constant List for all-static inputs, with fresh distinct member
objects even when their values agree. Dynamic List inputs retain original
argument objects but produce a new List on each call. Raw FF and NUL source
literal bytes remain exact.

C8.4 global registration additionally primes canonical native-long decimal
cache, including a counted NUL suffix after the canonical CString spelling;
`LONG_MIN`, leading plus/zero and negative zero do not qualify. Later
releases initialize string literals with no primary cache. This allocation
protocol differs from primitive integer getter acceptance and expression
numeric conversion.

`command-registration/` retains 40 original command/data object observations
from the five C engines. Relative heads share across bodies in one namespace;
C8.4 also shares across namespaces and with data. C8.5's absolute command
heads share with data, while C8.6–9.1 use a separate root command partition.
The callback observes primary caches after dispatch; compilation-time
`cmdName` priming additionally requires the actual resolved command token.
The adjacent source and manifest use the same native build command above.

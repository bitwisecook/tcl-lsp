# Stock integer contents

The probe uses public Tcl constructors, getters, duplication, variable storage,
and expression/Increment evaluation. Each row retains the original object's
primary type and resident string before and after the selected operation.

C8.5 and later retain integer or bignum caches for integer strings passed to
GetDouble. C8.4 installs Double and has its own range behavior. A Double
constructor without a resident string formats `1.0` and is rejected by
Increment. These objects cannot acquire source-literal consistency from
numeric category or mathematical equality alone.

Build `probe.c` against each pinned Tcl static library with its matching generic
headers, `-ldl -lpthread -lm -lz`, then run it to produce that release's TSV.

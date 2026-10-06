# Same-object stock Length and empty-result cache probes

The 204 Length observations retain the exact original object while the native
handler runs. Modes 0–16 cover empty/nonempty plain String, malformed String,
native Int/Double with and without resident strings, ordinary List, ByteArray,
Dict with and without resident strings, and cached word Boolean. Route 0 uses
late generic invocation; route 1 executes a procedure body. Jim modes 9/10 are
raw String objects, because Jim has no native ByteArray cache. C8.4 Dict modes
use the documented List fallback and provide no C8.4 Dict evidence.

The additional ten empty-result rows prime the first `list` result to List and
observe whether the next native result is the same object. C9 compiled empty
results reuse the primed pooled object; generic empty results do not. These
observations grant no allocation freshness to a source consumer.

Build each C probe against its pinned native tree (replace the paths and ABI):

```sh
cc -std=c11 -I "$TCL_TREE/generic" -I "$TCL_TREE/unix" probe.c   "$TCL_TREE/unix/libtcl9.0.a" -lpthread -ldl -lm -lz -o stock-length
./stock-length
cc -std=c11 -I "$TCL_TREE/generic" -I "$TCL_TREE/unix" empty-result-probe.c   "$TCL_TREE/unix/libtcl9.0.a" -lpthread -ldl -lm -lz -o empty-result
./empty-result
cc -std=c11 -I "$JIM_TREE" jim-probe.c "$JIM_TREE/libjim.a" -ldl -lm -o jim-stock-length
./jim-stock-length
```

`manifest.json` binds every recorded output to its exact source and native
library SHA-256. The registry test `native_stock_list::tests` validates the
conditional cache guarantees against these rows. It deliberately keeps
unknown class and empty preservation separate from known-byte parsing.

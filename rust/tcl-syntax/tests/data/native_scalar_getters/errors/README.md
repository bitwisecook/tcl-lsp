# Primitive getter errors and C character units

These are direct native observations from C Tcl 8.4.20, 8.5.19, 8.6.18,
9.0.4, 9.1.0 and pinned Jim 0.84. The adjacent manifests retain source,
library and executable SHA-256 values. Rust tests consume every recorded row.

`c-error-code-state-probe.c` creates an original raw String or ByteArray,
optionally primes its Double/Boolean cache, seeds `errorCode` with
`PROBE BEFORE`, then reaches the selected primitive getter. It retains the
primitive result before returning to `Tcl_Eval`. The 1,080 rows independently
record primitive bytes, propagated bytes, final object cache and error state.
An unchanged state is not a request to set `NONE`.

`c-octal-stage-probe.c` contributes 180 grammar-origin discriminators.
`c-error-units-probe.c` contributes 210 raw String/ByteArray diagnostics,
including invalid bytes, modified NUL, surrogate units and a clipped emoji.
`tcl-character-units-probe.c` contributes 70 decoder, isolated encoder and
previous-boundary observations. Its decoder retains the previous native unit,
as required by C8.6's supplementary-character surrogate protocol.

`jim-error-state-probe.c` adds 33 actual seeded-state observations. Unlike the
earlier exploratory Jim probe, its `errorCode` field is read from the actual
interpreter variable. Primitive getters preserve that state.

Build each C fixture against the selected canonical library (replace VERSION
and ABI, for example VERSION=8.6.18 and ABI=8.6):

```sh
cc -std=c11 -I tmp/tclVERSION/generic -I tmp/tclVERSION/unix \
  rust/tcl-syntax/tests/data/native_scalar_getters/errors/c-error-code-state-probe.c \
  tmp/tclVERSION/unix/libtclABI.a -lpthread -ldl -lm -lz -o /tmp/getter-errors
/tmp/getter-errors
```

The Jim fixture links the pinned configured `libjim.a` with its `jim.h`,
`-ldl -lm`. These fixtures supply observed protocol facts, not authority to
execute a mutable command or attest arbitrary object stringification effects.

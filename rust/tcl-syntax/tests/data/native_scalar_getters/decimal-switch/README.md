# Jim decimal string switch

This probe calls the pinned `Jim_StringToWide(bytes, 10)` stage with clear and
ERANGE errno. Its 32 observations cover decimal-only grammar, whitespace, raw
NUL prefix, signed/unsigned boundaries and saturated overflow.
`JimCheckConversion` checks converted input and trailing whitespace without
consulting errno. The signed return can therefore be negative after unsigned
conversion; a command switch must apply its own accepted-code rules.

`NativeScalarGetterProtocol::jim_decimal_wide_probe` consumes this string-only
protocol without converting an original object cache or producing a primitive
error receipt. A C or unknown-engine protocol declines it.

```sh
cc -I/path/to/pinned/jimtcl probe.c /path/to/pinned/jimtcl/libjim.a -lm -ldl -lssl -lcrypto -lz -o /tmp/jim-decimal-switch
/tmp/jim-decimal-switch
```

`manifest.json` records the source, header, library and executable hashes.

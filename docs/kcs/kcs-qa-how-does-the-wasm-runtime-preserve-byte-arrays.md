# KCS: How does the WASM runtime preserve byte arrays through string commands?

> **Audience:** Contributor
> **Type:** Q&A

## Applies to

tcl-lsp-cli

## Question

How can a value from `binary format` pass through `string` commands without
losing its original bytes in the WASM runtime?

## Answer

The runtime keeps two representations for a byte array. Its raw bytes are used
by `binary` and `zlib`. Its string view maps every byte to the same-numbered
Latin-1 character. The string view is created only when a string command needs
it. This is the same [shimmer](../GLOSSARY.md#shimmer) boundary used by C
Tcl.

For example, slicing a byte array preserves its raw payload:

```tcl
set packet [binary format H* 41ff42]
binary encode hex [string range $packet 0 2]
# => 41ff42
```

A string-changing command produces an ordinary Unicode string. When that value
is passed back to `binary`, the emulated release chooses the conversion:

```tcl
binary encode hex [string toupper [binary format H* 41ff42]]
```

Tcl 8.x truncates the resulting U+0178 character to byte `78`, so the result
is `417842`. Tcl 9 rejects it because U+0178 is not a byte, with
`TCL VALUE BYTES`. This difference is intentional. Do not use character case
conversion to edit a binary protocol payload.

The runtime creates byte-array values for `binary format`, `binary decode`,
`binary scan`, and `zlib` output. Plain strings are not silently marked as
byte arrays. This distinction is important: a real Unicode string and a
byte-array string view can display similarly, but Tcl converts them differently
when a byte-consuming command reads them.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [Byte-array corruption warning](features/kcs-feature-byte-array-corruption.md)
- [Runtime C-extension ABI](../design/runtime/c-extension-abi.md)

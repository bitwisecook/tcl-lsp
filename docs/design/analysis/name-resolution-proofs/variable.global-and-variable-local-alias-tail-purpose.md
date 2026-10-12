# global and variable local alias tail purpose

ID: `naming.variable.global-and-variable-local-alias-tail-purpose`

## Problem statement

Global and namespace-variable declarations project a local alias from the same qualified operand using different selected recipes. Rooted and relative names with a three-colon separator differ in Jim, so a generic namespace-tail display is not an interchangeable binding name.

## Question

Which local names does info vars present after global or variable declarations of ::R:::v and R:::v in each of the six captured native engines?

## Answers

### tcl8.4 — observed

Process exit 0, empty stderr; exact four caught local-name presentations:

```text
a:0:v
b:0:v
c:0:v
d:0:v
```

Version: 8.4 label; exact patch not recorded. Build: Executable SHA 551bef3a9b51f256f4ad9dec06bddc42ceab872a72528e83676d1d7df715b9d4. Input channel: ASCII probe.tcl through native shell.

### tcl8.5 — observed

Process exit 0, empty stderr; exact four caught local-name presentations:

```text
a:0:v
b:0:v
c:0:v
d:0:v
```

Version: 8.5 label; exact patch not recorded. Build: Executable SHA 7a0fb4aa272a45662fb883295d0037a8c18e3e32c635736af8314fb835981935. Input channel: ASCII probe.tcl through native shell.

### tcl8.6 — observed

Process exit 0, empty stderr; exact four caught local-name presentations:

```text
a:0:v
b:0:v
c:0:v
d:0:v
```

Version: 8.6 label; exact patch not recorded. Build: Executable SHA 0cc67113840d03c15c15d88188c16b8b019aa2a8c11ca6980f32723fbedacde0. Input channel: ASCII probe.tcl through native shell.

### tcl9.0 — observed

Process exit 0, empty stderr; exact four caught local-name presentations:

```text
a:0:v
b:0:v
c:0:v
d:0:v
```

Version: 9.0 label; exact patch not recorded. Build: Executable SHA cc28851d04548dce27006a317d22b78c67e5491204f66f924c20d1f3febf6c18. Input channel: ASCII probe.tcl through native shell.

### tcl9.1 — observed

Process exit 0, empty stderr; exact four caught local-name presentations:

```text
a:0:v
b:0:v
c:0:v
d:0:v
```

Version: 9.1 label; exact patch not recorded. Build: Executable SHA d4c2dbd11cd88994b7ca992eb87cdc855750f75d5d9f42b681b4dd4c321c791c. Input channel: ASCII probe.tcl through native shell.

### jim — observed

Process exit 0, empty stderr; exact four caught local-name presentations:

```text
a:0:
b:0:v
c:0:R:::v
d:0:v
```

Version: 0.84/current label; exact revision not recorded. Build: Executable SHA d5b47eb75b271d50331ca5a38775492cb186f8feedd3c76bb2a0fbf1905671f0. Input channel: ASCII probe.tcl through native shell.

### bigip — not-tested

No appliance observation for this question.

Version: not recorded. Build: not recorded. Input channel: not recorded.


## Conclusion

All five captured C providers present v for all four declarations. Jim presents no local name for rooted global, v for rooted variable, R:::v for relative global and v for relative variable. These four exact ASCII presentation controls support distinct alias-local projection purposes; they prove no physical cell, compiler-local slot, alias lifetime, opaque-byte behaviour or arbitrary namespace existence.

## Scope

Four exact ASCII script cases, native shell evaluation, five C release labels plus current-label Jim and original executable SHA. Exact patches/Jim revision were not queried in this manifest. BIG-IP not queried. No bytezero or character-channel translation claim is made.

## Retained evidence

- `rust/tcl-syntax/tests/data/native_alias_names/provenance.json` SHA256 `24b117cfdef3258d7ff3d7c1ca7fd1beb87ea91f0111c3dc3108444344da25f6`: Original six executable hashes/status/stdout/stderr and declared source hash.
- `rust/tcl-syntax/tests/data/native_alias_names/probe.tcl` SHA256 `ff62a137fadf12739f470970731c2d17ff0b3195b44cb4fcc628aeae9ad6a841`: Exact namespace setup and four caught declaration/info-vars scripts.

## Replay

```sh
/path/to/recorded/native/shell rust/tcl-syntax/tests/data/native_alias_names/probe.tcl
```

Run the exact retained ASCII script in a fresh recorded provider build; it owns its catch/result presenter. Require all four stdout rows, empty stderr and exit 0. Original executable hashes are retained but exact build recipes/patch queries are missing; a rerun is a new observation. No Rust execution is recorded.

## Rust comparisons

- `naming::aliases::tests::local_alias_names_match_original_native_declarations` in `rust/tcl-syntax/src/naming/aliases.rs`: Compares operation-specific local alias projection to native presentations without granting cell/table identity. No execution result is recorded here.

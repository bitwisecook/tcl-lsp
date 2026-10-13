# naming.bigip.unicode-loader-runtime-values

Kind: `native-observation`

## Problem statement

A literal UTF-8 source rejection can be mistaken for a runtime name/value rejection, while a rendered dynamic bytearray can be mistaken for Unicode normalization. These have separate parser and value producers.

## Question

Which exact literal UTF-8 payloads loaded, and what bytes/lengths did the independent dynamic bytearray controls retain?

## Conclusion

The thirteen recorded payloads have distinct load outcomes: selected emoji sequences load while the listed BMP/combining/ZWJ/keycap cases reject. Dynamic controls preserve each supplied hex sequence and count encoded bytes. This finite matrix establishes no general BMP/non-BMP acceptance law and no identifier-equivalence rule.

## Scope

Exact fixture bytes, load logs and reached TMM string-value controls on this build; dynamic payload preservation is separate from literal source acceptance. BIG-IP21.1.0.1 build0.0.26 Point Release1 only; appliance implementation source is unavailable.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded.

No independent C Tcl or Jim execution of this appliance question is claimed.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded.

The scoped answer is stated above.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded.

The scoped answer is stated above.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded.

The scoped answer is stated above.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded.

The scoped answer is stated above.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded.

The scoped answer is stated above.

### bigip

Status: `observed`. Version: 21.1.0.1. Build: 0.0.26, Point Release1; no separate hotfix field reported.

The thirteen recorded payloads have distinct load outcomes: selected emoji sequences load while the listed BMP/combining/ZWJ/keycap cases reject. Dynamic controls preserve each supplied hex sequence and count encoded bytes. This finite matrix establishes no general BMP/non-BMP acceptance law and no identifier-equivalence rule.

The measured report rows and their limits are:


Each literal fixture contains the exact UTF-8 bytes named below. Files were transferred without text conversion, checked against [manifest.json](../../../../experiments/oracles/f5/captures/README.md) (retained logical path: `scripts/dev/bigip-probes/resolution-2286/evidence/r2286_20261006hi/appliance/manifest.json`), and loaded independently.

| Payload | UTF-8 hex | Literal load/runtime result | Dynamic bytearray result |
| --- | --- | --- | --- |
| precomposed `é` (U+00E9) | `c3a9` | REJECT, `braces are required around the expression` | preserved; length 2 |
| decomposed `e` + U+0301 | `65cc81` | REJECT, same diagnostic | preserved; length 3 |
| © | `c2a9` | REJECT, same diagnostic | preserved; length 2 |
| ☃ | `e29883` | REJECT, same diagnostic | preserved; length 3 |
| ❤ | `e29da4` | REJECT, same diagnostic | preserved; length 3 |
| 😀 | `f09f9880` | ACCEPT; visually logged; length 4 | preserved; length 4 |
| ❤️ | `e29da4efb88f` | REJECT, same diagnostic | preserved; length 6 |
| 👍🏽 | `f09f918df09f8fbd` | ACCEPT; visually logged; length 8 | preserved; length 8 |
| 👩🏽‍💻 | `f09f91a9f09f8fbde2808df09f92bb` | REJECT, same diagnostic | preserved; length 15 |
| family ZWJ sequence | `f09f91a8e2808df09f91a9e2808df09f91a7e2808df09f91a6` | REJECT, same diagnostic | preserved; length 25 |
| 🇳🇿 | `f09f87b3f09f87bf` | ACCEPT; visually logged; length 8 | preserved; length 8 |
| rainbow-flag ZWJ sequence | `f09f8fb3efb88fe2808df09f8c88` | REJECT, same diagnostic | preserved; length 14 |
| 1️⃣ | `31efb88fe283a3` | REJECT, same diagnostic | preserved; length 7 |

The three accepted literal rules ran on all four TMMs. The dynamic rule constructed each value with `binary format H*`; every `binary scan ... H*` result exactly equalled its source hex on all four TMMs in both provider-order traffic batches. TMM `string length` counted the encoded bytes, not Unicode scalar values or grapheme clusters. Literal accepted emoji appeared visually in syslog; dynamically constructed UTF-8 bytearrays were logged as byte-oriented mojibake even though their hex was intact. These rows do not justify a general acceptance rule for BMP versus non-BMP text: they establish only the exact measured byte sequences.


## Exact evidence

[bigip-unicode-loader-runtime-values.json](bigip-unicode-loader-runtime-values.json) contains the exact evidence descriptors, original hashes and selectors. The canonical manifest remains the proof authority. The indexed F5 store resolves historical paths without retaining expanded duplicate trees.

```sh
python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH --json-pointer /ROW
```

## Reproduction

The argument vector below records the historical invocation. Restore retained driver directories with `materialise.py extract` before replay, and apply the selected case's current address bindings and state prerequisites.

The pinned report contains the exact source-generation, load and traffic protocol. This record claims no new appliance execution; rerun requires the recorded isolated appliance context, exact bytes and independently captured load/event outcomes. Closed implementation source and unmeasured releases are unavailable.

```json
[]
```

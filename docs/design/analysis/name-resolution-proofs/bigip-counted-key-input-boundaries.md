# naming.bigip.counted-key-input-boundaries

Kind: `native-observation`

## Problem statement

A counted runtime value can reach one key API intact while a lexical or selector API truncates or rejects it. Assuming one name/key rule for all APIs would erase purpose-specific boundaries.

## Question

How do the exact counted-key programs cross variable, array, dict, command and selector input boundaries?

## Conclusion

The report retains each purpose-specific input/result/error row and transport boundary. Full-counted storage outcomes cannot establish an unexecuted parser/selector or physical internal-object claim. The conclusions are limited to these programs and contexts.

## Scope

Exact counted-key source variants, recorded operation channels and context rows; C/Jim controls and native allocation/header claims are outside this appliance observation. BIG-IP21.1.0.1 build0.0.26 Point Release1 only; appliance implementation source is unavailable.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `not-tested`.

No independent C Tcl or Jim execution of this appliance question is claimed.

### bigip

Status: `observed`. Channel: Exact counted-key source variants, recorded operation channels and context rows; C/Jim controls and native allocation/header claims are outside this appliance observation. Version: bigip 21.1.0.1.

The report retains each purpose-specific input/result/error row and transport boundary. Full-counted storage outcomes cannot establish an unexecuted parser/selector or physical internal-object claim. The conclusions are limited to these programs and contexts.

The measured report rows and their limits are:

Run `r2286t` used a fresh VIP on port 18121 and backend port 18120. Its baseline
forwarded 128 requests to `192.0.2.20`, where the server observed SNAT peer
`192.0.2.21` 128 times. Response headers proved 32 baseline connections on
each actual TMM. The exact counted-input source from the request matched the
Markdown code block byte-for-byte: 1,791 ASCII/LF bytes, no CR or NUL.
The [executed source and observations](../../../../evidence/tcl/bigip-r2286t-counted-input-boundaries.md) are retained together.

The exact rule loaded with dynamic-name warnings and returned `OK` on all 128
requests. The first 128 consecutive exact-rule markers in the continuous LTM
capture contained 32 observations from each of TMM units 0, 1, 2 and 3. Its
single log record was longer than the appliance's retained line, so it cannot
be the source of complete row bytes. The [capture form](../../../../evidence/tcl/bigip-r2286t-counted-input-capture.md) changed
only the response step: it hex-encoded the already-computed `rows` value into
the HTTP body and added the TMM header.
All 128 full results were byte-identical across the four TMMs.

| Input domain | Exact measured HTTP_REQUEST result |
| --- | --- |
| Scalar prefixes `x`, 59-byte long prefix, and `relocated_2286_key` | Plain and `plain 00 42 00 43` names both wrote/read with catch 0 as `PLAIN` and `COUNTED`; unsetting the counted name left `PLAIN`. |
| Empty dynamic variable input | Write/read catch 0, value `GRAMMAR_VALUE`. |
| `()`, `(k)`, `A()`, `A(k)`, `A(k(l))` | Write/read catch 0. These are accepted combined variable forms; the result alone does not make parentheses part of an opaque scalar key. |
| `A(k)tail`, `:`, `A:B` | Write/read catch 0. |
| `A::B` with no namespace `A` | Write rejected: `can't set "A::B": parent namespace doesn't exist`; read rejected: `can't read "A::B": no such variable`. This is qualified lookup, not a generic colon rejection. |
| Array indexes ``, `()`, `(k)`, `A()`, `A(k)`, `A(k(l))`, `A(k)tail`, `:`, `A:B`, `A::B` | Every index wrote, read and unset with catch 0; trace `name2` contained the exact input bytes. Colons and parentheses inside an already-separated index are ordinary index bytes. |

The array-root control used unrelated short, long and relocated prefixes,
leading NUL, middle NUL, two NULs, an empty root, a colon-bearing root, and a
qualified root under a created ASCII namespace. Except for the parentheses
case below, every write/read/upvar succeeded in both orders. Unsetting either
plain or counted root preserved the other value. Direct trace callbacks
received the complete root bytes and index `6b`; the alias write reported root
hex `726f6f745f616c696173` (`root_alias`), keeping selected identity separate
from callback spelling. All 128 response bodies had outer catch 0. The [array-root](../../../../evidence/tcl/bigip-r2286t-array-root-boundaries.md) and [array-index](../../../../evidence/tcl/bigip-r2286t-array-index-boundaries.md) records retain their scripts and observed results.

The parentheses-root row is an important counterexample. The control appended
`(k)` to root spelling `root(k)`. The resulting `root(k)(k)` was parsed as a
combined variable form rather than an opaque root plus a separately supplied
index. The trace attached to `root(k)` did not observe the writes/reads, and in
the counted-first order `unset root(k)` returned catch 1 with exact result:

```text
can't unset "root(k)": no such element in array
```

The successful value reads in that row therefore address differently parsed
array elements and do not establish an independent array root named `root(k)`.
A compiler interface must separate root and index before rendering Tcl combined
variable syntax; concatenating `root + "(" + index + ")"` is not lossless for
arbitrary root spellings.

The array-index control independently supplied indexes to one ASCII root. All
grammar indexes above, empty/leading-NUL indexes, unrelated short/long prefixes,
middle NUL, and two-NUL values succeeded under reversed writes and unsets.
Plain and counted cells retained `PLAIN`/`COUNTED`, `upvar` selected only the
counted cell, and the survivor discriminator was correct in both orders. Trace
`name1` was the fixed root and `name2` preserved every exact index byte. All 128
responses were identical across the four TMMs with outer catch 0.

These added input-boundary controls reached TMM HTTP_REQUEST only. They do not
transfer the root/index grammar or parentheses counterexample to tmsh, iApp or
iCall.

The exact and capture rules both produced the following loader warnings at the
same source lines; the loader nevertheless accepted each rule:

```text
line 13: [variable reference used where variable name expected][$plain]
line 14: [variable reference used where variable name expected][$name]
line 15: [variable reference used where variable name expected][$plain]
line 16: [variable reference used where variable name expected][$name]
line 19: [variable reference used where variable name expected][$input]
line 23: [variable reference used where variable name expected][$name]
line 24: [variable reference used where variable name expected][$plain]
line 27: [variable reference used where variable name expected][$plain]
line 32: [variable reference used where variable name expected][$input]
line 35: [variable reference used where variable name expected][$input]
line 38: [variable reference used where variable name expected][$input]
```

The array-root and array-index wrappers loaded without warnings. Complete raw
rows, exact error bytes, response hashes, trace tuples and per-TMM equality are
in the counted-boundary decoded results.
 Transport endpoint literals in the baseline traffic paragraph are documentation placeholders; Tcl program and measured completion/result fields are retained.

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [scripts/dev/bigip-probes/resolution-2286/CONTEXT_NAMING_RESULTS.md](../../../../scripts/dev/bigip-probes/resolution-2286/CONTEXT_NAMING_RESULTS.md)

# Tcl counted-source execution evidence

## Question

> Interpreter option error tables differ by release and root versus child-command entry. A printed error list or familiar abbreviation cannot establish the accepted primary dispatch table, child API equivalence or command token identity.

> What exact public completions and bad/ambiguous option tables do the eight original root/child interpreter controls report on each pinned provider?

This script is one recorded source in the stated question; it does not by itself establish results for other sources or environments.

## Assumptions

Recorded scope and limits:

> Eight exact ASCII LF source files and one independently evaluated child prelude use the unchanged native202 counted-source driver in fresh full Tcl_Init or Jim core/static-extension interpreters. Each original source is passed directly without surrounding eval/catch/puts; prelude failure does not suppress the subsequent original observation. Reported actual version comes from a counted VERSION evaluation and retains its separate CLI provider association. Required original executable/library/header/Makefile/source pins and original driver compile commands are preserved. Primary dispatch tables and source recipe implementations remain separate from the printed public diagnostics. No launched or passing software assertion is attached. The comparator checks public completion/result presentation, not private option tables, child allocation/header identity, physical frame, effect chronology or compiler admission. Fresh external original processes and their version/build receipts retain their separate measured scope.

Other assumptions were not separately recorded. The retained driver evaluated this single counted Tcl source without a prelude or query source; its process exit is separate from Tcl's completion code.

## Script

```tcl-visible
interp e {set x 1}
```

## Results

### tcl8.4, run 1

- Recorded version: 8.4.20
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e342e3230
ORIGINAL|1|616d626967756f7573206f7074696f6e202265223a206d75737420626520616c6961732c20616c69617365732c206372656174652c2064656c6574652c206576616c2c206578697374732c206578706f73652c20686964652c2068696464656e2c206973736166652c20696e766f6b6568696464656e2c206d61726b747275737465642c20726563757273696f6e6c696d69742c20736c617665732c2073686172652c207461726765742c206f72207472616e73666572
```

- Standard error: empty.

Tcl completion code for the original source: 1. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
ambiguous option "e": must be alias, aliases, create, delete, eval, exists, expose, hide, hidden, issafe, invokehidden, marktrusted, recursionlimit, slaves, share, target, or transfer⟦EOF⟧
```

### tcl8.5, run 2

- Recorded version: 8.5.19
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e352e3139
ORIGINAL|1|616d626967756f7573206f7074696f6e202265223a206d75737420626520616c6961732c20616c69617365732c2062676572726f722c206372656174652c2064656275672c2064656c6574652c206576616c2c206578697374732c206578706f73652c20686964652c2068696464656e2c206973736166652c20696e766f6b6568696464656e2c206c696d69742c206d61726b747275737465642c20726563757273696f6e6c696d69742c20736c617665732c2073686172652c207461726765742c206f72207472616e73666572
```

- Standard error: empty.

Tcl completion code for the original source: 1. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
ambiguous option "e": must be alias, aliases, bgerror, create, debug, delete, eval, exists, expose, hide, hidden, issafe, invokehidden, limit, marktrusted, recursionlimit, slaves, share, target, or transfer⟦EOF⟧
```

### tcl8.6, run 3

- Recorded version: 8.6.18
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|382e362e3138
ORIGINAL|1|616d626967756f7573206f7074696f6e202265223a206d75737420626520616c6961732c20616c69617365732c2062676572726f722c2063616e63656c2c206368696c6472656e2c206372656174652c2064656275672c2064656c6574652c206576616c2c206578697374732c206578706f73652c20686964652c2068696464656e2c206973736166652c20696e766f6b6568696464656e2c206c696d69742c206d61726b747275737465642c20726563757273696f6e6c696d69742c20736c617665732c2073686172652c207461726765742c206f72207472616e73666572
```

- Standard error: empty.

Tcl completion code for the original source: 1. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
ambiguous option "e": must be alias, aliases, bgerror, cancel, children, create, debug, delete, eval, exists, expose, hide, hidden, issafe, invokehidden, limit, marktrusted, recursionlimit, slaves, share, target, or transfer⟦EOF⟧
```

### tcl9.0, run 4

- Recorded version: 9.0.4
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e302e34
ORIGINAL|1|616d626967756f7573206f7074696f6e202265223a206d75737420626520616c6961732c20616c69617365732c2062676572726f722c2063616e63656c2c206368696c6472656e2c206372656174652c2064656275672c2064656c6574652c206576616c2c206578697374732c206578706f73652c20686964652c2068696464656e2c206973736166652c20696e766f6b6568696464656e2c206c696d69742c206d61726b747275737465642c20726563757273696f6e6c696d69742c2073686172652c207461726765742c206f72207472616e73666572
```

- Standard error: empty.

Tcl completion code for the original source: 1. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
ambiguous option "e": must be alias, aliases, bgerror, cancel, children, create, debug, delete, eval, exists, expose, hide, hidden, issafe, invokehidden, limit, marktrusted, recursionlimit, share, target, or transfer⟦EOF⟧
```

### tcl9.1, run 5

- Recorded version: 9.1.0
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|392e312e30
ORIGINAL|1|616d626967756f7573206f7074696f6e202265223a206d75737420626520616c6961732c20616c69617365732c2062676572726f722c2063616e63656c2c206368696c6472656e2c206372656174652c2064656275672c2064656c6574652c206576616c2c206578697374732c206578706f73652c20686964652c2068696464656e2c206973736166652c20696e766f6b6568696464656e2c206c696d69742c206d61726b747275737465642c20726563757273696f6e6c696d69742c207365742c2073686172652c207461726765742c206f72207472616e73666572
```

- Standard error: empty.

Tcl completion code for the original source: 1. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
ambiguous option "e": must be alias, aliases, bgerror, cancel, children, create, debug, delete, eval, exists, expose, hide, hidden, issafe, invokehidden, limit, marktrusted, recursionlimit, set, share, target, or transfer⟦EOF⟧
```

### jim, run 6

- Recorded version: 0.84-9-g5bac7c9
- Recorded input channel: counted-source native C API driver.
- Exit status: 0
- Standard output:

```text-visible
VERSION|0|302e38342d392d6735626163376339
ORIGINAL|1|77726f6e67202320617267733a2073686f756c642062652022696e7465727022
```

- Standard error: empty.

Tcl completion code for the original source: 1. The following block decodes the ORIGINAL result payload from standard output.

```text-visible
wrong # args: should be "interp"⟦EOF⟧
```

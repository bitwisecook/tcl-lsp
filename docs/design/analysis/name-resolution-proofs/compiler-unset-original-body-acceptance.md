# naming.compiler.unset-original-body-acceptance

Kind: `native-observation`

## Problem statement

Unset with a literal root, flags, expansion or a dynamic prefix can be compiled or dispatched generically. Both paths can return the same bytes while evaluating a different operand frontier.

## Question

Which of the seventeen original procedure bodies are compiled inline, and what completion/index order do they produce?

## Conclusion

The tables preserve original body bytes and actual disassembly. Tcl 8.6/C9 accept selected unset bodies without ordinary invoke; older C and Jim have no unset compiler. The sequential index control fails on FIRST before reaching the second index. A generic fallback cannot prove inline coverage.

## Scope

Only the original retained programs, capture windows and provider labels in the attached artifacts are covered. Version labels are original receipt metadata, not a fresh patchlevel query. Missing command/API doors and aborted processes stay distinct from successful guest completion. Source files/probes explain the observer protocol; no pinned engine implementation excerpt or executed Rust assertion is claimed. Different attempts and sources are retained separately, never treated as byte-identical duplicates.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: tcl8.4.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`observations.json` (`file-0-row-1`):

```json
{
  "version": "8.4",
  "label": "none",
  "exit": 0,
  "stdout": "RESULT 0 {} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-2`):

```json
{
  "version": "8.4",
  "label": "scalar",
  "exit": 0,
  "stdout": "RESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-3`):

```json
{
  "version": "8.4",
  "label": "array",
  "exit": 0,
  "stdout": "RESULT 1 {can't unset \"a(k)\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-4`):

```json
{
  "version": "8.4",
  "label": "global",
  "exit": 0,
  "stdout": "RESULT 1 {can't unset \"::x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-5`):

```json
{
  "version": "8.4",
  "label": "quiet",
  "exit": 0,
  "stdout": "RESULT 0 {} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-6`):

```json
{
  "version": "8.4",
  "label": "endflags",
  "exit": 0,
  "stdout": "RESULT 1 {can't unset \"-x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-7`):

```json
{
  "version": "8.4",
  "label": "quietflags",
  "exit": 0,
  "stdout": "RESULT 0 {} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-8`):

```json
{
  "version": "8.4",
  "label": "unknownflag",
  "exit": 0,
  "stdout": "RESULT 1 {can't unset \"-bad\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-9`):

```json
{
  "version": "8.4",
  "label": "dynamicfirst",
  "exit": 0,
  "stdout": "RESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-10`):

```json
{
  "version": "8.4",
  "label": "dynamicprefix",
  "exit": 0,
  "stdout": "RESULT 1 {can't unset \"ax\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-11`):

```json
{
  "version": "8.4",
  "label": "dynamicflag",
  "exit": 0,
  "stdout": "RESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-12`):

```json
{
  "version": "8.4",
  "label": "dynamicafterknown",
  "exit": 0,
  "stdout": "RESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-13`):

```json
{
  "version": "8.4",
  "label": "dynamicbeforeknown",
  "exit": 0,
  "stdout": "RESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-14`):

```json
{
  "version": "8.4",
  "label": "multipledynamic",
  "exit": 0,
  "stdout": "RESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-15`):

```json
{
  "version": "8.4",
  "label": "literalexpand",
  "exit": 0,
  "stdout": "RESULT 1 {extra characters after close-brace} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-16`):

```json
{
  "version": "8.4",
  "label": "dynamicexpand",
  "exit": 0,
  "stdout": "RESULT 1 {extra characters after close-brace} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-17`):

```json
{
  "version": "8.4",
  "label": "ordered",
  "exit": 0,
  "stdout": "RESULT 1 {can't unset \"a(FIRST)\": no such variable} SECOND\n",
  "stderr": ""
}
```

`windows.tsv` (`file-103`):

```json
{
  "provider_rows": [
    "8.4\tnone\t756e736574\t0\t0\t524553554c542030207b7d204245464f5245",
    "8.4\tscalar\t756e7365742078\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.4\tarray\t756e7365742061286b29\t0\t0\t524553554c542031207b63616e277420756e736574202261286b29223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.4\tglobal\t756e736574203a3a78\t0\t0\t524553554c542031207b63616e277420756e73657420223a3a78223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.4\tquiet\t756e736574202d6e6f636f6d706c61696e20782061286b29\t0\t0\t524553554c542030207b7d204245464f5245",
    "8.4\tendflags\t756e736574202d2d202d78\t0\t0\t524553554c542031207b63616e277420756e73657420222d78223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.4\tquietflags\t756e736574202d6e6f636f6d706c61696e202d2d2078\t0\t0\t524553554c542030207b7d204245464f5245",
    "8.4\tunknownflag\t756e736574202d6261642078\t0\t0\t524553554c542031207b63616e277420756e73657420222d626164223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.4\tdynamicfirst\t756e73657420246e616d65\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.4\tdynamicprefix\t756e7365742061246e616d65\t0\t0\t524553554c542031207b63616e277420756e73657420226178223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.4\tdynamicflag\t756e736574202d2d20246e616d65\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.4\tdynamicafterknown\t756e736574207820246e616d65\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.4\tdynamicbeforeknown\t756e736574202d2d20246e616d652078\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.4\tmultipledynamic\t756e736574202d2d20246e616d6520246f74686572\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.4\tliteralexpand\t756e736574207b2a7d7b7820797d\t0\t0\t524553554c542031207b6578747261206368617261637465727320616674657220636c6f73652d62726163657d204245464f5245",
    "8.4\tdynamicexpand\t756e736574207b2a7d246e616d6573\t0\t0\t524553554c542031207b6578747261206368617261637465727320616674657220636c6f73652d62726163657d204245464f5245",
    "8.4\tordered\t756e7365742061285b736574203a3a7365656e2046495253545d292062285b736574203a3a7365656e205345434f4e445d29\t0\t0\t524553554c542031207b63616e277420756e73657420226128464952535429223a206e6f2073756368207661726961626c657d205345434f4e44"
  ]
}
```

### tcl8.5

Status: `observed`. Version: 8.5. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: tcl8.5.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`observations.json` (`file-0-row-18`):

```json
{
  "version": "8.5",
  "label": "none",
  "exit": 0,
  "stdout": "ByteCode 0x0x5605bf210cc0, refCt 1, epoch 3, interp 0x0x5605bf1b7d90 (epoch 3)\n  Source \"\\nunset\\n\"\n  Cmds 1, src 7, inst 5, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x0x5605bf1e3b10, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-3, src 1-5\n  Command 1: \"unset\"\n    (0) push1 0 \t# \"unset\"\n    (2) invokeStk1 1 \n    (4) done \n\nRESULT 0 {} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-19`):

```json
{
  "version": "8.5",
  "label": "scalar",
  "exit": 0,
  "stdout": "ByteCode 0x0x55fbcae247b0, refCt 1, epoch 3, interp 0x0x55fbcadebd90 (epoch 3)\n  Source \"\\nunset x\\n\"\n  Cmds 1, src 9, inst 7, litObjs 2, aux 0, stkDepth 2, code/src 0.00\n  Proc 0x0x55fbcae17b10, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-5, src 1-7\n  Command 1: \"unset x\"\n    (0) push1 0 \t# \"unset\"\n    (2) push1 1 \t# \"x\"\n    (4) invokeStk1 2 \n    (6) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-20`):

```json
{
  "version": "8.5",
  "label": "array",
  "exit": 0,
  "stdout": "ByteCode 0x0x56475a5047b0, refCt 1, epoch 3, interp 0x0x56475a4cbd90 (epoch 3)\n  Source \"\\nunset a(k)\\n\"\n  Cmds 1, src 12, inst 7, litObjs 2, aux 0, stkDepth 2, code/src 0.00\n  Proc 0x0x56475a4f7b10, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-5, src 1-10\n  Command 1: \"unset a(k)\"\n    (0) push1 0 \t# \"unset\"\n    (2) push1 1 \t# \"a(k)\"\n    (4) invokeStk1 2 \n    (6) done \n\nRESULT 1 {can't unset \"a(k)\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-21`):

```json
{
  "version": "8.5",
  "label": "global",
  "exit": 0,
  "stdout": "ByteCode 0x0x556651fe97b0, refCt 1, epoch 3, interp 0x0x556651fb0d90 (epoch 3)\n  Source \"\\nunset ::x\\n\"\n  Cmds 1, src 11, inst 7, litObjs 2, aux 0, stkDepth 2, code/src 0.00\n  Proc 0x0x556651fdcb10, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-5, src 1-9\n  Command 1: \"unset ::x\"\n    (0) push1 0 \t# \"unset\"\n    (2) push1 1 \t# \"::x\"\n    (4) invokeStk1 2 \n    (6) done \n\nRESULT 1 {can't unset \"::x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-22`):

```json
{
  "version": "8.5",
  "label": "quiet",
  "exit": 0,
  "stdout": "ByteCode 0x0x5631b8d2fb30, refCt 1, epoch 3, interp 0x0x5631b8d0cd90 (epoch 3)\n  Source \"\\nunset -nocomplain x a(k)\\n\"\n  Cmds 1, src 26, inst 11, litObjs 4, aux 0, stkDepth 4, code/src 0.00\n  Proc 0x0x5631b8d38b10, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-9, src 1-24\n  Command 1: \"unset -nocomplain x a(k)\"\n    (0) push1 0 \t# \"unset\"\n    (2) push1 1 \t# \"-nocomplain\"\n    (4) push1 2 \t# \"x\"\n    (6) push1 3 \t# \"a(k)\"\n    (8) invokeStk1 4 \n    (10) done \n\nRESULT 0 {} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-23`):

```json
{
  "version": "8.5",
  "label": "endflags",
  "exit": 0,
  "stdout": "ByteCode 0x0x55b60898ab30, refCt 1, epoch 3, interp 0x0x55b608967d90 (epoch 3)\n  Source \"\\nunset -- -x\\n\"\n  Cmds 1, src 13, inst 9, litObjs 3, aux 0, stkDepth 3, code/src 0.00\n  Proc 0x0x55b608993b10, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-7, src 1-11\n  Command 1: \"unset -- -x\"\n    (0) push1 0 \t# \"unset\"\n    (2) push1 1 \t# \"--\"\n    (4) push1 2 \t# \"-x\"\n    (6) invokeStk1 3 \n    (8) done \n\nRESULT 1 {can't unset \"-x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-24`):

```json
{
  "version": "8.5",
  "label": "quietflags",
  "exit": 0,
  "stdout": "ByteCode 0x0x5596a2a30b30, refCt 1, epoch 3, interp 0x0x5596a2a0dd90 (epoch 3)\n  Source \"\\nunset -nocomplain -- x\\n\"\n  Cmds 1, src 24, inst 11, litObjs 4, aux 0, stkDepth 4, code/src 0.00\n  Proc 0x0x5596a2a39b10, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-9, src 1-22\n  Command 1: \"unset -nocomplain -- x\"\n    (0) push1 0 \t# \"unset\"\n    (2) push1 1 \t# \"-nocomplain\"\n    (4) push1 2 \t# \"--\"\n    (6) push1 3 \t# \"x\"\n    (8) invokeStk1 4 \n    (10) done \n\nRESULT 0 {} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-25`):

```json
{
  "version": "8.5",
  "label": "unknownflag",
  "exit": 0,
  "stdout": "ByteCode 0x0x561d26d24b30, refCt 1, epoch 3, interp 0x0x561d26d01d90 (epoch 3)\n  Source \"\\nunset -bad x\\n\"\n  Cmds 1, src 14, inst 9, litObjs 3, aux 0, stkDepth 3, code/src 0.00\n  Proc 0x0x561d26d2db10, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-7, src 1-12\n  Command 1: \"unset -bad x\"\n    (0) push1 0 \t# \"unset\"\n    (2) push1 1 \t# \"-bad\"\n    (4) push1 2 \t# \"x\"\n    (6) invokeStk1 3 \n    (8) done \n\nRESULT 1 {can't unset \"-bad\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-26`):

```json
{
  "version": "8.5",
  "label": "dynamicfirst",
  "exit": 0,
  "stdout": "ByteCode 0x0x5586cba3ccc0, refCt 1, epoch 3, interp 0x0x5586cb9e3d90 (epoch 3)\n  Source \"\\nunset $name\\n\"\n  Cmds 1, src 13, inst 7, litObjs 1, aux 0, stkDepth 2, code/src 0.00\n  Proc 0x0x5586cba0fb10, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-5, src 1-11\n  Command 1: \"unset $name\"\n    (0) push1 0 \t# \"unset\"\n    (2) loadScalar1 %v0 \t# var \"name\"\n    (4) invokeStk1 2 \n    (6) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-27`):

```json
{
  "version": "8.5",
  "label": "dynamicprefix",
  "exit": 0,
  "stdout": "ByteCode 0x0x55fcac1047b0, refCt 1, epoch 3, interp 0x0x55fcac0cbd90 (epoch 3)\n  Source \"\\nunset a$name\\n\"\n  Cmds 1, src 14, inst 11, litObjs 2, aux 0, stkDepth 3, code/src 0.00\n  Proc 0x0x55fcac0f7b10, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-9, src 1-12\n  Command 1: \"unset a$name\"\n    (0) push1 0 \t# \"unset\"\n    (2) push1 1 \t# \"a\"\n    (4) loadScalar1 %v0 \t# var \"name\"\n    (6) concat1 2 \n    (8) invokeStk1 2 \n    (10) done \n\nRESULT 1 {can't unset \"ax\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-28`):

```json
{
  "version": "8.5",
  "label": "dynamicflag",
  "exit": 0,
  "stdout": "ByteCode 0x0x563ba393d7b0, refCt 1, epoch 3, interp 0x0x563ba3904d90 (epoch 3)\n  Source \"\\nunset -- $name\\n\"\n  Cmds 1, src 16, inst 9, litObjs 2, aux 0, stkDepth 3, code/src 0.00\n  Proc 0x0x563ba3930b10, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-7, src 1-14\n  Command 1: \"unset -- $name\"\n    (0) push1 0 \t# \"unset\"\n    (2) push1 1 \t# \"--\"\n    (4) loadScalar1 %v0 \t# var \"name\"\n    (6) invokeStk1 3 \n    (8) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-29`):

```json
{
  "version": "8.5",
  "label": "dynamicafterknown",
  "exit": 0,
  "stdout": "ByteCode 0x0x555870b4b7b0, refCt 1, epoch 3, interp 0x0x555870b12d90 (epoch 3)\n  Source \"\\nunset x $name\\n\"\n  Cmds 1, src 15, inst 9, litObjs 2, aux 0, stkDepth 3, code/src 0.00\n  Proc 0x0x555870b3eb10, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-7, src 1-13\n  Command 1: \"unset x $name\"\n    (0) push1 0 \t# \"unset\"\n    (2) push1 1 \t# \"x\"\n    (4) loadScalar1 %v0 \t# var \"name\"\n    (6) invokeStk1 3 \n    (8) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-30`):

```json
{
  "version": "8.5",
  "label": "dynamicbeforeknown",
  "exit": 0,
  "stdout": "ByteCode 0x0x562446adbb30, refCt 1, epoch 3, interp 0x0x562446ab8d90 (epoch 3)\n  Source \"\\nunset -- $name x\\n\"\n  Cmds 1, src 18, inst 11, litObjs 3, aux 0, stkDepth 4, code/src 0.00\n  Proc 0x0x562446ae4b10, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-9, src 1-16\n  Command 1: \"unset -- $name x\"\n    (0) push1 0 \t# \"unset\"\n    (2) push1 1 \t# \"--\"\n    (4) loadScalar1 %v0 \t# var \"name\"\n    (6) push1 2 \t# \"x\"\n    (8) invokeStk1 4 \n    (10) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-31`):

```json
{
  "version": "8.5",
  "label": "multipledynamic",
  "exit": 0,
  "stdout": "ByteCode 0x0x55f6ae4907b0, refCt 1, epoch 3, interp 0x0x55f6ae457d90 (epoch 3)\n  Source \"\\nunset -- $name $other\\n\"\n  Cmds 1, src 23, inst 11, litObjs 2, aux 0, stkDepth 4, code/src 0.00\n  Proc 0x0x55f6ae483b10, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-9, src 1-21\n  Command 1: \"unset -- $name $other\"\n    (0) push1 0 \t# \"unset\"\n    (2) push1 1 \t# \"--\"\n    (4) loadScalar1 %v0 \t# var \"name\"\n    (6) loadScalar1 %v1 \t# var \"other\"\n    (8) invokeStk1 4 \n    (10) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-32`):

```json
{
  "version": "8.5",
  "label": "literalexpand",
  "exit": 0,
  "stdout": "ByteCode 0x0x55827207cb30, refCt 1, epoch 3, interp 0x0x558272059d90 (epoch 3)\n  Source \"\\nunset {*}{x y}\\n\"\n  Cmds 1, src 16, inst 9, litObjs 3, aux 0, stkDepth 3, code/src 0.00\n  Proc 0x0x558272085b10, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-7, src 1-14\n  Command 1: \"unset {*}{x y}\"\n    (0) push1 0 \t# \"unset\"\n    (2) push1 1 \t# \"x\"\n    (4) push1 2 \t# \"y\"\n    (6) invokeStk1 3 \n    (8) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-33`):

```json
{
  "version": "8.5",
  "label": "dynamicexpand",
  "exit": 0,
  "stdout": "ByteCode 0x0x55ac5bc527b0, refCt 1, epoch 3, interp 0x0x55ac5bc19d90 (epoch 3)\n  Source \"\\nunset {*}$names\\n\"\n  Cmds 1, src 17, inst 12, litObjs 1, aux 0, stkDepth 2, code/src 0.00\n  Proc 0x0x55ac5bc45b10, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-10, src 1-15\n  Command 1: \"unset {*}$names\"\n    (0) expandStart \n    (1) push1 0 \t# \"unset\"\n    (3) loadScalar1 %v2 \t# var \"names\"\n    (5) expandStkTop 2 \n    (10) invokeExpanded \n    (11) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-34`):

```json
{
  "version": "8.5",
  "label": "ordered",
  "exit": 0,
  "stdout": "ByteCode 0x0x5616e5cb7fe0, refCt 1, epoch 3, interp 0x0x5616e5c53d90 (epoch 3)\n  Source \"\\nunset a([set ::seen FIRST]) b([set ::seen SECOND])\\n\"\n  Cmds 3, src 52, inst 45, litObjs 7, aux 0, stkDepth 5, code/src 0.00\n  Proc 0x0x5616e5cb7580, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 3:\n      1: pc 0-43, src 1-50   \t   2: pc 4-17, src 10-25\n      3: pc 24-37, src 32-48\n  Command 1: \"unset a([set ::seen FIRST]) b([set ::seen SECOND])\"\n    (0) push1 0 \t# \"unset\"\n    (2) push1 1 \t# \"a(\"\n  Command 2: \"set ::seen FIRST\"\n    (4) startCommand +14 1 \t# next cmd at pc 18\n    (13) push1 2 \t# \"::seen\"\n    (15) push1 3 \t# \"FIRST\"\n    (17) storeScalarStk \n    (18) push1 4 \t# \")\"\n    (20) concat1 3 \n    (22) push1 5 \t# \"b(\"\n  Command 3: \"set ::seen SECOND\"\n    (24) startCommand +14 1 \t# next cmd at pc 38\n    (33) push1 2 \t# \"::seen\"\n    (35) push1 6 \t# \"SECOND\"\n    (37) storeScalarStk \n    (38) push1 4 \t# \")\"\n    (40) concat1 3 \n    (42) invokeStk1 3 \n    (44) done \n\nRESULT 1 {can't unset \"a(FIRST)\": no such variable} SECOND\n",
  "stderr": ""
}
```

`windows.tsv` (`file-103`):

```json
{
  "provider_rows": [
    "8.5\tnone\t756e736574\t0\t0\t524553554c542030207b7d204245464f5245",
    "8.5\tscalar\t756e7365742078\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.5\tarray\t756e7365742061286b29\t0\t0\t524553554c542031207b63616e277420756e736574202261286b29223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.5\tglobal\t756e736574203a3a78\t0\t0\t524553554c542031207b63616e277420756e73657420223a3a78223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.5\tquiet\t756e736574202d6e6f636f6d706c61696e20782061286b29\t0\t0\t524553554c542030207b7d204245464f5245",
    "8.5\tendflags\t756e736574202d2d202d78\t0\t0\t524553554c542031207b63616e277420756e73657420222d78223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.5\tquietflags\t756e736574202d6e6f636f6d706c61696e202d2d2078\t0\t0\t524553554c542030207b7d204245464f5245",
    "8.5\tunknownflag\t756e736574202d6261642078\t0\t0\t524553554c542031207b63616e277420756e73657420222d626164223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.5\tdynamicfirst\t756e73657420246e616d65\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.5\tdynamicprefix\t756e7365742061246e616d65\t0\t0\t524553554c542031207b63616e277420756e73657420226178223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.5\tdynamicflag\t756e736574202d2d20246e616d65\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.5\tdynamicafterknown\t756e736574207820246e616d65\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.5\tdynamicbeforeknown\t756e736574202d2d20246e616d652078\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.5\tmultipledynamic\t756e736574202d2d20246e616d6520246f74686572\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.5\tliteralexpand\t756e736574207b2a7d7b7820797d\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.5\tdynamicexpand\t756e736574207b2a7d246e616d6573\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.5\tordered\t756e7365742061285b736574203a3a7365656e2046495253545d292062285b736574203a3a7365656e205345434f4e445d29\t0\t0\t524553554c542031207b63616e277420756e73657420226128464952535429223a206e6f2073756368207661726961626c657d205345434f4e44"
  ]
}
```

### tcl8.6

Status: `observed`. Version: 8.6. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: tcl8.6.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`observations.json` (`file-0-row-35`):

```json
{
  "version": "8.6",
  "label": "none",
  "exit": 0,
  "stdout": "ByteCode 0x0x55a8de1925b0, refCt 1, epoch 17, interp 0x0x55a8de169130 (epoch 17)\n  Source \"\\nunset...\"\n  Cmds 1, src 7, inst 3, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x0x55a8de1b8140, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-1, src 1-5\n  Command 1: \"unset...\"\n    (0) push1 0 \t# \"\"\n    (2) done \n\nRESULT 0 {} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-36`):

```json
{
  "version": "8.6",
  "label": "scalar",
  "exit": 0,
  "stdout": "ByteCode 0x0x556b0f5395b0, refCt 1, epoch 17, interp 0x0x556b0f510130 (epoch 17)\n  Source \"\\nunset x...\"\n  Cmds 1, src 9, inst 9, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x0x556b0f55f140, refCt 1, args 3, compiled locals 4\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"x\"\n  Commands 1:\n      1: pc 0-7, src 1-7\n  Command 1: \"unset x...\"\n    (0) unsetScalar 1 %v3 \t# var \"x\"\n    (6) push1 0 \t# \"\"\n    (8) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-37`):

```json
{
  "version": "8.6",
  "label": "array",
  "exit": 0,
  "stdout": "ByteCode 0x0x55de5f88f5b0, refCt 1, epoch 17, interp 0x0x55de5f866130 (epoch 17)\n  Source \"\\nunset a(k)...\"\n  Cmds 1, src 12, inst 11, litObjs 2, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x0x55de5f8b5140, refCt 1, args 3, compiled locals 4\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"a\"\n  Commands 1:\n      1: pc 0-9, src 1-10\n  Command 1: \"unset a(k)...\"\n    (0) push1 0 \t# \"k\"\n    (2) unsetArray 1 %v3 \t# var \"a\"\n    (8) push1 1 \t# \"\"\n    (10) done \n\nRESULT 1 {can't unset \"a(k)\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-38`):

```json
{
  "version": "8.6",
  "label": "global",
  "exit": 0,
  "stdout": "ByteCode 0x0x55858471a5b0, refCt 1, epoch 17, interp 0x0x5585846f1130 (epoch 17)\n  Source \"\\nunset ::x...\"\n  Cmds 1, src 11, inst 7, litObjs 2, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x0x558584740140, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-5, src 1-9\n  Command 1: \"unset ::x...\"\n    (0) push1 0 \t# \"::x\"\n    (2) unsetStk 1 \n    (4) push1 1 \t# \"\"\n    (6) done \n\nRESULT 1 {can't unset \"::x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-39`):

```json
{
  "version": "8.6",
  "label": "quiet",
  "exit": 0,
  "stdout": "ByteCode 0x0x55727895c6b0, refCt 1, epoch 17, interp 0x0x557278933130 (epoch 17)\n  Source \"\\nunset -nocomplain x a(k)...\"\n  Cmds 1, src 26, inst 17, litObjs 2, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x0x5572789883e0, refCt 1, args 3, compiled locals 5\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"x\"\n      slot 4, scalar, \"a\"\n  Commands 1:\n      1: pc 0-15, src 1-24\n  Command 1: \"unset -nocomplain x a(k)...\"\n    (0) unsetScalar 0 %v3 \t# var \"x\"\n    (6) push1 0 \t# \"k\"\n    (8) unsetArray 0 %v4 \t# var \"a\"\n    (14) push1 1 \t# \"\"\n    (16) done \n\nRESULT 0 {} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-40`):

```json
{
  "version": "8.6",
  "label": "endflags",
  "exit": 0,
  "stdout": "ByteCode 0x0x55d8c7a0b5b0, refCt 1, epoch 17, interp 0x0x55d8c79e2130 (epoch 17)\n  Source \"\\nunset -- -x...\"\n  Cmds 1, src 13, inst 9, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x0x55d8c7a31140, refCt 1, args 3, compiled locals 4\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"-x\"\n  Commands 1:\n      1: pc 0-7, src 1-11\n  Command 1: \"unset -- -x...\"\n    (0) unsetScalar 1 %v3 \t# var \"-x\"\n    (6) push1 0 \t# \"\"\n    (8) done \n\nRESULT 1 {can't unset \"-x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-41`):

```json
{
  "version": "8.6",
  "label": "quietflags",
  "exit": 0,
  "stdout": "ByteCode 0x0x5558a322d5b0, refCt 1, epoch 17, interp 0x0x5558a3204130 (epoch 17)\n  Source \"\\nunset -nocomplain -- x...\"\n  Cmds 1, src 24, inst 9, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x0x5558a3253140, refCt 1, args 3, compiled locals 4\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"x\"\n  Commands 1:\n      1: pc 0-7, src 1-22\n  Command 1: \"unset -nocomplain -- x...\"\n    (0) unsetScalar 0 %v3 \t# var \"x\"\n    (6) push1 0 \t# \"\"\n    (8) done \n\nRESULT 0 {} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-42`):

```json
{
  "version": "8.6",
  "label": "unknownflag",
  "exit": 0,
  "stdout": "ByteCode 0x0x55d74fc775b0, refCt 1, epoch 17, interp 0x0x55d74fc4e130 (epoch 17)\n  Source \"\\nunset -bad x...\"\n  Cmds 1, src 14, inst 15, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x0x55d74fc9d140, refCt 1, args 3, compiled locals 5\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"-bad\"\n      slot 4, scalar, \"x\"\n  Commands 1:\n      1: pc 0-13, src 1-12\n  Command 1: \"unset -bad x...\"\n    (0) unsetScalar 1 %v3 \t# var \"-bad\"\n    (6) unsetScalar 1 %v4 \t# var \"x\"\n    (12) push1 0 \t# \"\"\n    (14) done \n\nRESULT 1 {can't unset \"-bad\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-43`):

```json
{
  "version": "8.6",
  "label": "dynamicfirst",
  "exit": 0,
  "stdout": "ByteCode 0x0x55d7900905b0, refCt 1, epoch 17, interp 0x0x55d790067130 (epoch 17)\n  Source \"\\nunset $name...\"\n  Cmds 1, src 13, inst 7, litObjs 1, aux 0, stkDepth 2, code/src 0.00\n  Proc 0x0x55d7900b6140, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-5, src 1-11\n  Command 1: \"unset $name...\"\n    (0) push1 0 \t# \"unset\"\n    (2) loadScalar1 %v0 \t# var \"name\"\n    (4) invokeStk1 2 \n    (6) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-44`):

```json
{
  "version": "8.6",
  "label": "dynamicprefix",
  "exit": 0,
  "stdout": "ByteCode 0x0x55e6fcb2a5b0, refCt 1, epoch 17, interp 0x0x55e6fcb01130 (epoch 17)\n  Source \"\\nunset a$name...\"\n  Cmds 1, src 14, inst 11, litObjs 2, aux 0, stkDepth 2, code/src 0.00\n  Proc 0x0x55e6fcb50140, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-9, src 1-12\n  Command 1: \"unset a$name...\"\n    (0) push1 0 \t# \"a\"\n    (2) loadScalar1 %v0 \t# var \"name\"\n    (4) strcat 2 \n    (6) unsetStk 1 \n    (8) push1 1 \t# \"\"\n    (10) done \n\nRESULT 1 {can't unset \"ax\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-45`):

```json
{
  "version": "8.6",
  "label": "dynamicflag",
  "exit": 0,
  "stdout": "ByteCode 0x0x562ce31185b0, refCt 1, epoch 17, interp 0x0x562ce30ef130 (epoch 17)\n  Source \"\\nunset -- $name...\"\n  Cmds 1, src 16, inst 7, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x0x562ce313e140, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-5, src 1-14\n  Command 1: \"unset -- $name...\"\n    (0) loadScalar1 %v0 \t# var \"name\"\n    (2) unsetStk 1 \n    (4) push1 0 \t# \"\"\n    (6) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-46`):

```json
{
  "version": "8.6",
  "label": "dynamicafterknown",
  "exit": 0,
  "stdout": "ByteCode 0x0x55a6690c95b0, refCt 1, epoch 17, interp 0x0x55a6690a0130 (epoch 17)\n  Source \"\\nunset x $name...\"\n  Cmds 1, src 15, inst 9, litObjs 2, aux 0, stkDepth 3, code/src 0.00\n  Proc 0x0x55a6690ef140, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-7, src 1-13\n  Command 1: \"unset x $name...\"\n    (0) push1 0 \t# \"unset\"\n    (2) push1 1 \t# \"x\"\n    (4) loadScalar1 %v0 \t# var \"name\"\n    (6) invokeStk1 3 \n    (8) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-47`):

```json
{
  "version": "8.6",
  "label": "dynamicbeforeknown",
  "exit": 0,
  "stdout": "ByteCode 0x0x5643b947f5b0, refCt 1, epoch 17, interp 0x0x5643b9456130 (epoch 17)\n  Source \"\\nunset -- $name x...\"\n  Cmds 1, src 18, inst 13, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x0x5643b94a5140, refCt 1, args 3, compiled locals 4\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"x\"\n  Commands 1:\n      1: pc 0-11, src 1-16\n  Command 1: \"unset -- $name x...\"\n    (0) loadScalar1 %v0 \t# var \"name\"\n    (2) unsetStk 1 \n    (4) unsetScalar 1 %v3 \t# var \"x\"\n    (10) push1 0 \t# \"\"\n    (12) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-48`):

```json
{
  "version": "8.6",
  "label": "multipledynamic",
  "exit": 0,
  "stdout": "ByteCode 0x0x55f75d3e55b0, refCt 1, epoch 17, interp 0x0x55f75d3bc130 (epoch 17)\n  Source \"\\nunset -- $name $other...\"\n  Cmds 1, src 23, inst 11, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x0x55f75d40b140, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-9, src 1-21\n  Command 1: \"unset -- $name $other...\"\n    (0) loadScalar1 %v0 \t# var \"name\"\n    (2) unsetStk 1 \n    (4) loadScalar1 %v1 \t# var \"other\"\n    (6) unsetStk 1 \n    (8) push1 0 \t# \"\"\n    (10) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-49`):

```json
{
  "version": "8.6",
  "label": "literalexpand",
  "exit": 0,
  "stdout": "ByteCode 0x0x56547242a5b0, refCt 1, epoch 17, interp 0x0x565472401130 (epoch 17)\n  Source \"\\nunset {*}{x y}...\"\n  Cmds 1, src 16, inst 15, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x0x565472450140, refCt 1, args 3, compiled locals 5\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"x\"\n      slot 4, scalar, \"y\"\n  Commands 1:\n      1: pc 0-13, src 1-14\n  Command 1: \"unset {*}{x y}...\"\n    (0) unsetScalar 1 %v3 \t# var \"x\"\n    (6) unsetScalar 1 %v4 \t# var \"y\"\n    (12) push1 0 \t# \"\"\n    (14) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-50`):

```json
{
  "version": "8.6",
  "label": "dynamicexpand",
  "exit": 0,
  "stdout": "ByteCode 0x0x55dc4cf6e5b0, refCt 1, epoch 17, interp 0x0x55dc4cf45130 (epoch 17)\n  Source \"\\nunset {*}$names...\"\n  Cmds 1, src 17, inst 12, litObjs 1, aux 0, stkDepth 2, code/src 0.00\n  Proc 0x0x55dc4cf94140, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-10, src 1-15\n  Command 1: \"unset {*}$names...\"\n    (0) expandStart \n    (1) push1 0 \t# \"unset\"\n    (3) loadScalar1 %v2 \t# var \"names\"\n    (5) expandStkTop 2 \n    (10) invokeExpanded \n    (11) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-51`):

```json
{
  "version": "8.6",
  "label": "ordered",
  "exit": 0,
  "stdout": "ByteCode 0x0x5575c2dd0630, refCt 1, epoch 17, interp 0x0x5575c2d84130 (epoch 17)\n  Source \"\\nunset a([set ::seen FIRST]) b([set ::seen SECOND])...\"\n  Cmds 3, src 52, inst 25, litObjs 4, aux 0, stkDepth 2, code/src 0.00\n  Proc 0x0x5575c2dd9260, refCt 1, args 3, compiled locals 5\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"a\"\n      slot 4, scalar, \"b\"\n  Commands 3:\n      1: pc 0-23, src 1-50        2: pc 0-4, src 10-25\n      3: pc 11-15, src 32-48\n  Command 1: \"unset a([set ::seen FIRST]) b([set ::seen SECOND])...\"\n  Command 2: \"set ::seen FIRST...\"\n    (0) push1 0 \t# \"::seen\"\n    (2) push1 1 \t# \"FIRST\"\n    (4) storeStk \n    (5) unsetArray 1 %v3 \t# var \"a\"\n  Command 3: \"set ::seen SECOND...\"\n    (11) push1 0 \t# \"::seen\"\n    (13) push1 2 \t# \"SECOND\"\n    (15) storeStk \n    (16) unsetArray 1 %v4 \t# var \"b\"\n    (22) push1 3 \t# \"\"\n    (24) done \n\nRESULT 1 {can't unset \"a(FIRST)\": no such variable} FIRST\n",
  "stderr": ""
}
```

`windows.tsv` (`file-103`):

```json
{
  "provider_rows": [
    "8.6\tnone\t756e736574\t1\t0\t524553554c542030207b7d204245464f5245",
    "8.6\tscalar\t756e7365742078\t1\t1\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.6\tarray\t756e7365742061286b29\t1\t1\t524553554c542031207b63616e277420756e736574202261286b29223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.6\tglobal\t756e736574203a3a78\t1\t1\t524553554c542031207b63616e277420756e73657420223a3a78223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.6\tquiet\t756e736574202d6e6f636f6d706c61696e20782061286b29\t1\t2\t524553554c542030207b7d204245464f5245",
    "8.6\tendflags\t756e736574202d2d202d78\t1\t1\t524553554c542031207b63616e277420756e73657420222d78223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.6\tquietflags\t756e736574202d6e6f636f6d706c61696e202d2d2078\t1\t1\t524553554c542030207b7d204245464f5245",
    "8.6\tunknownflag\t756e736574202d6261642078\t1\t2\t524553554c542031207b63616e277420756e73657420222d626164223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.6\tdynamicfirst\t756e73657420246e616d65\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.6\tdynamicprefix\t756e7365742061246e616d65\t1\t1\t524553554c542031207b63616e277420756e73657420226178223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.6\tdynamicflag\t756e736574202d2d20246e616d65\t1\t1\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.6\tdynamicafterknown\t756e736574207820246e616d65\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.6\tdynamicbeforeknown\t756e736574202d2d20246e616d652078\t1\t2\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.6\tmultipledynamic\t756e736574202d2d20246e616d6520246f74686572\t1\t2\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.6\tliteralexpand\t756e736574207b2a7d7b7820797d\t1\t2\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.6\tdynamicexpand\t756e736574207b2a7d246e616d6573\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "8.6\tordered\t756e7365742061285b736574203a3a7365656e2046495253545d292062285b736574203a3a7365656e205345434f4e445d29\t1\t2\t524553554c542031207b63616e277420756e73657420226128464952535429223a206e6f2073756368207661726961626c657d204649525354"
  ]
}
```

### tcl9.0

Status: `observed`. Version: 9.0. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: tcl9.0.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`observations.json` (`file-0-row-52`):

```json
{
  "version": "9.0",
  "label": "none",
  "exit": 0,
  "stdout": "ByteCode 0x558390503e30, refCt 1, epoch 21, interp 0x55839047ca10 (epoch 21)\n  Source \"\\nunset...\"\n  Cmds 1, src 7, inst 3, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x558390514f80, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-1, src 1-5\n  Command 1: \"unset...\"\n    (0) push1 0 \t# \"\"\n    (2) done \n\nRESULT 0 {} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-53`):

```json
{
  "version": "9.0",
  "label": "scalar",
  "exit": 0,
  "stdout": "ByteCode 0x558e7a53d3a0, refCt 1, epoch 21, interp 0x558e7a487a10 (epoch 21)\n  Source \"\\nunset x...\"\n  Cmds 1, src 9, inst 9, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x558e7a51ff80, refCt 1, args 3, compiled locals 4\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"x\"\n  Commands 1:\n      1: pc 0-7, src 1-7\n  Command 1: \"unset x...\"\n    (0) unsetScalar 1 %v3 \t# var \"x\"\n    (6) push1 0 \t# \"\"\n    (8) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-54`):

```json
{
  "version": "9.0",
  "label": "array",
  "exit": 0,
  "stdout": "ByteCode 0x55791c9d73a0, refCt 1, epoch 21, interp 0x55791c921a10 (epoch 21)\n  Source \"\\nunset a(k)...\"\n  Cmds 1, src 12, inst 11, litObjs 2, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x55791c9b9f80, refCt 1, args 3, compiled locals 4\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"a\"\n  Commands 1:\n      1: pc 0-9, src 1-10\n  Command 1: \"unset a(k)...\"\n    (0) push1 0 \t# \"k\"\n    (2) unsetArray 1 %v3 \t# var \"a\"\n    (8) push1 1 \t# \"\"\n    (10) done \n\nRESULT 1 {can't unset \"a(k)\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-55`):

```json
{
  "version": "9.0",
  "label": "global",
  "exit": 0,
  "stdout": "ByteCode 0x55830032a3a0, refCt 1, epoch 21, interp 0x558300274a10 (epoch 21)\n  Source \"\\nunset ::x...\"\n  Cmds 1, src 11, inst 7, litObjs 2, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x55830030cf80, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-5, src 1-9\n  Command 1: \"unset ::x...\"\n    (0) push1 0 \t# \"::x\"\n    (2) unsetStk 1 \n    (4) push1 1 \t# \"\"\n    (6) done \n\nRESULT 1 {can't unset \"::x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-56`):

```json
{
  "version": "9.0",
  "label": "quiet",
  "exit": 0,
  "stdout": "ByteCode 0x55769064a3a0, refCt 1, epoch 21, interp 0x557690594a10 (epoch 21)\n  Source \"\\nunset -nocomplain x a(k)...\"\n  Cmds 1, src 26, inst 17, litObjs 2, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x55769062cf80, refCt 1, args 3, compiled locals 5\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"x\"\n      slot 4, scalar, \"a\"\n  Commands 1:\n      1: pc 0-15, src 1-24\n  Command 1: \"unset -nocomplain x a(k)...\"\n    (0) unsetScalar 0 %v3 \t# var \"x\"\n    (6) push1 0 \t# \"k\"\n    (8) unsetArray 0 %v4 \t# var \"a\"\n    (14) push1 1 \t# \"\"\n    (16) done \n\nRESULT 0 {} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-57`):

```json
{
  "version": "9.0",
  "label": "endflags",
  "exit": 0,
  "stdout": "ByteCode 0x564548e103a0, refCt 1, epoch 21, interp 0x564548d5aa10 (epoch 21)\n  Source \"\\nunset -- -x...\"\n  Cmds 1, src 13, inst 9, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x564548df2f80, refCt 1, args 3, compiled locals 4\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"-x\"\n  Commands 1:\n      1: pc 0-7, src 1-11\n  Command 1: \"unset -- -x...\"\n    (0) unsetScalar 1 %v3 \t# var \"-x\"\n    (6) push1 0 \t# \"\"\n    (8) done \n\nRESULT 1 {can't unset \"-x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-58`):

```json
{
  "version": "9.0",
  "label": "quietflags",
  "exit": 0,
  "stdout": "ByteCode 0x55c79f9193a0, refCt 1, epoch 21, interp 0x55c79f863a10 (epoch 21)\n  Source \"\\nunset -nocomplain -- x...\"\n  Cmds 1, src 24, inst 9, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x55c79f8fbf80, refCt 1, args 3, compiled locals 4\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"x\"\n  Commands 1:\n      1: pc 0-7, src 1-22\n  Command 1: \"unset -nocomplain -- x...\"\n    (0) unsetScalar 0 %v3 \t# var \"x\"\n    (6) push1 0 \t# \"\"\n    (8) done \n\nRESULT 0 {} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-59`):

```json
{
  "version": "9.0",
  "label": "unknownflag",
  "exit": 0,
  "stdout": "ByteCode 0x563f4270c3a0, refCt 1, epoch 21, interp 0x563f42656a10 (epoch 21)\n  Source \"\\nunset -bad x...\"\n  Cmds 1, src 14, inst 15, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x563f426eef80, refCt 1, args 3, compiled locals 5\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"-bad\"\n      slot 4, scalar, \"x\"\n  Commands 1:\n      1: pc 0-13, src 1-12\n  Command 1: \"unset -bad x...\"\n    (0) unsetScalar 1 %v3 \t# var \"-bad\"\n    (6) unsetScalar 1 %v4 \t# var \"x\"\n    (12) push1 0 \t# \"\"\n    (14) done \n\nRESULT 1 {can't unset \"-bad\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-60`):

```json
{
  "version": "9.0",
  "label": "dynamicfirst",
  "exit": 0,
  "stdout": "ByteCode 0x55ca5feb2e30, refCt 1, epoch 21, interp 0x55ca5fe2ba10 (epoch 21)\n  Source \"\\nunset $name...\"\n  Cmds 1, src 13, inst 7, litObjs 1, aux 0, stkDepth 2, code/src 0.00\n  Proc 0x55ca5fec3f80, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-5, src 1-11\n  Command 1: \"unset $name...\"\n    (0) push1 0 \t# \"unset\"\n    (2) loadScalar1 %v0 \t# var \"name\"\n    (4) invokeStk1 2 \n    (6) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-61`):

```json
{
  "version": "9.0",
  "label": "dynamicprefix",
  "exit": 0,
  "stdout": "ByteCode 0x558a0ae113a0, refCt 1, epoch 21, interp 0x558a0ad5ba10 (epoch 21)\n  Source \"\\nunset a$name...\"\n  Cmds 1, src 14, inst 11, litObjs 2, aux 0, stkDepth 2, code/src 0.00\n  Proc 0x558a0adf3f80, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-9, src 1-12\n  Command 1: \"unset a$name...\"\n    (0) push1 0 \t# \"a\"\n    (2) loadScalar1 %v0 \t# var \"name\"\n    (4) strcat 2 \n    (6) unsetStk 1 \n    (8) push1 1 \t# \"\"\n    (10) done \n\nRESULT 1 {can't unset \"ax\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-62`):

```json
{
  "version": "9.0",
  "label": "dynamicflag",
  "exit": 0,
  "stdout": "ByteCode 0x55a90873be30, refCt 1, epoch 21, interp 0x55a9086b4a10 (epoch 21)\n  Source \"\\nunset -- $name...\"\n  Cmds 1, src 16, inst 7, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x55a90874cf80, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-5, src 1-14\n  Command 1: \"unset -- $name...\"\n    (0) loadScalar1 %v0 \t# var \"name\"\n    (2) unsetStk 1 \n    (4) push1 0 \t# \"\"\n    (6) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-63`):

```json
{
  "version": "9.0",
  "label": "dynamicafterknown",
  "exit": 0,
  "stdout": "ByteCode 0x5655005fd3a0, refCt 1, epoch 21, interp 0x565500547a10 (epoch 21)\n  Source \"\\nunset x $name...\"\n  Cmds 1, src 15, inst 9, litObjs 2, aux 0, stkDepth 3, code/src 0.00\n  Proc 0x5655005dff80, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-7, src 1-13\n  Command 1: \"unset x $name...\"\n    (0) push1 0 \t# \"unset\"\n    (2) push1 1 \t# \"x\"\n    (4) loadScalar1 %v0 \t# var \"name\"\n    (6) invokeStk1 3 \n    (8) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-64`):

```json
{
  "version": "9.0",
  "label": "dynamicbeforeknown",
  "exit": 0,
  "stdout": "ByteCode 0x55d82a6213a0, refCt 1, epoch 21, interp 0x55d82a56ba10 (epoch 21)\n  Source \"\\nunset -- $name x...\"\n  Cmds 1, src 18, inst 13, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x55d82a603f80, refCt 1, args 3, compiled locals 4\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"x\"\n  Commands 1:\n      1: pc 0-11, src 1-16\n  Command 1: \"unset -- $name x...\"\n    (0) loadScalar1 %v0 \t# var \"name\"\n    (2) unsetStk 1 \n    (4) unsetScalar 1 %v3 \t# var \"x\"\n    (10) push1 0 \t# \"\"\n    (12) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-65`):

```json
{
  "version": "9.0",
  "label": "multipledynamic",
  "exit": 0,
  "stdout": "ByteCode 0x55d00c6e43a0, refCt 1, epoch 21, interp 0x55d00c62ea10 (epoch 21)\n  Source \"\\nunset -- $name $other...\"\n  Cmds 1, src 23, inst 11, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x55d00c6c6f80, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-9, src 1-21\n  Command 1: \"unset -- $name $other...\"\n    (0) loadScalar1 %v0 \t# var \"name\"\n    (2) unsetStk 1 \n    (4) loadScalar1 %v1 \t# var \"other\"\n    (6) unsetStk 1 \n    (8) push1 0 \t# \"\"\n    (10) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-66`):

```json
{
  "version": "9.0",
  "label": "literalexpand",
  "exit": 0,
  "stdout": "ByteCode 0x56299d2113a0, refCt 1, epoch 21, interp 0x56299d15ba10 (epoch 21)\n  Source \"\\nunset {*}{x y}...\"\n  Cmds 1, src 16, inst 15, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x56299d1f3f80, refCt 1, args 3, compiled locals 5\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"x\"\n      slot 4, scalar, \"y\"\n  Commands 1:\n      1: pc 0-13, src 1-14\n  Command 1: \"unset {*}{x y}...\"\n    (0) unsetScalar 1 %v3 \t# var \"x\"\n    (6) unsetScalar 1 %v4 \t# var \"y\"\n    (12) push1 0 \t# \"\"\n    (14) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-67`):

```json
{
  "version": "9.0",
  "label": "dynamicexpand",
  "exit": 0,
  "stdout": "ByteCode 0x562f93ec03a0, refCt 1, epoch 21, interp 0x562f93e0aa10 (epoch 21)\n  Source \"\\nunset {*}$names...\"\n  Cmds 1, src 17, inst 12, litObjs 1, aux 0, stkDepth 2, code/src 0.00\n  Proc 0x562f93ea2f80, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-10, src 1-15\n  Command 1: \"unset {*}$names...\"\n    (0) expandStart \n    (1) push1 0 \t# \"unset\"\n    (3) loadScalar1 %v2 \t# var \"names\"\n    (5) expandStkTop 2 \n    (10) invokeExpanded \n    (11) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-68`):

```json
{
  "version": "9.0",
  "label": "ordered",
  "exit": 0,
  "stdout": "ByteCode 0x555dc7e3d3a0, refCt 1, epoch 21, interp 0x555dc7d87a10 (epoch 21)\n  Source \"\\nunset a([set ::seen FIRST]) b([set ::seen SECOND])...\"\n  Cmds 3, src 52, inst 25, litObjs 4, aux 0, stkDepth 2, code/src 0.00\n  Proc 0x555dc7e1f780, refCt 1, args 3, compiled locals 5\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"a\"\n      slot 4, scalar, \"b\"\n  Commands 3:\n      1: pc 0-23, src 1-50        2: pc 0-4, src 10-25\n      3: pc 11-15, src 32-48\n  Command 1: \"unset a([set ::seen FIRST]) b([set ::seen SECOND])...\"\n  Command 2: \"set ::seen FIRST...\"\n    (0) push1 0 \t# \"::seen\"\n    (2) push1 1 \t# \"FIRST\"\n    (4) storeStk \n    (5) unsetArray 1 %v3 \t# var \"a\"\n  Command 3: \"set ::seen SECOND...\"\n    (11) push1 0 \t# \"::seen\"\n    (13) push1 2 \t# \"SECOND\"\n    (15) storeStk \n    (16) unsetArray 1 %v4 \t# var \"b\"\n    (22) push1 3 \t# \"\"\n    (24) done \n\nRESULT 1 {can't unset \"a(FIRST)\": no such variable} FIRST\n",
  "stderr": ""
}
```

`windows.tsv` (`file-103`):

```json
{
  "provider_rows": [
    "9.0\tnone\t756e736574\t1\t0\t524553554c542030207b7d204245464f5245",
    "9.0\tscalar\t756e7365742078\t1\t1\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "9.0\tarray\t756e7365742061286b29\t1\t1\t524553554c542031207b63616e277420756e736574202261286b29223a206e6f2073756368207661726961626c657d204245464f5245",
    "9.0\tglobal\t756e736574203a3a78\t1\t1\t524553554c542031207b63616e277420756e73657420223a3a78223a206e6f2073756368207661726961626c657d204245464f5245",
    "9.0\tquiet\t756e736574202d6e6f636f6d706c61696e20782061286b29\t1\t2\t524553554c542030207b7d204245464f5245",
    "9.0\tendflags\t756e736574202d2d202d78\t1\t1\t524553554c542031207b63616e277420756e73657420222d78223a206e6f2073756368207661726961626c657d204245464f5245",
    "9.0\tquietflags\t756e736574202d6e6f636f6d706c61696e202d2d2078\t1\t1\t524553554c542030207b7d204245464f5245",
    "9.0\tunknownflag\t756e736574202d6261642078\t1\t2\t524553554c542031207b63616e277420756e73657420222d626164223a206e6f2073756368207661726961626c657d204245464f5245",
    "9.0\tdynamicfirst\t756e73657420246e616d65\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "9.0\tdynamicprefix\t756e7365742061246e616d65\t1\t1\t524553554c542031207b63616e277420756e73657420226178223a206e6f2073756368207661726961626c657d204245464f5245",
    "9.0\tdynamicflag\t756e736574202d2d20246e616d65\t1\t1\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "9.0\tdynamicafterknown\t756e736574207820246e616d65\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "9.0\tdynamicbeforeknown\t756e736574202d2d20246e616d652078\t1\t2\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "9.0\tmultipledynamic\t756e736574202d2d20246e616d6520246f74686572\t1\t2\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "9.0\tliteralexpand\t756e736574207b2a7d7b7820797d\t1\t2\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "9.0\tdynamicexpand\t756e736574207b2a7d246e616d6573\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "9.0\tordered\t756e7365742061285b736574203a3a7365656e2046495253545d292062285b736574203a3a7365656e205345434f4e445d29\t1\t2\t524553554c542031207b63616e277420756e73657420226128464952535429223a206e6f2073756368207661726961626c657d204649525354"
  ]
}
```

### tcl9.1

Status: `observed`. Version: 9.1. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: tcl9.1.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`observations.json` (`file-0-row-69`):

```json
{
  "version": "9.1",
  "label": "none",
  "exit": 0,
  "stdout": "ByteCode 0x558e41c642e0, refCt 1, epoch 24, interp 0x558e41bdaa10 (epoch 24)\n  Source \"\\nunset...\"\n  Cmds 1, src 7, inst 6, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x558e41c727b0, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-4, src 1-5\n  Command 1: \"unset...\"\n    (0) push 0 \t# \"\"\n    (5) done \n\nRESULT 0 {} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-70`):

```json
{
  "version": "9.1",
  "label": "scalar",
  "exit": 0,
  "stdout": "ByteCode 0x55ac7cbb7000, refCt 1, epoch 24, interp 0x55ac7cb31a10 (epoch 24)\n  Source \"\\nunset x...\"\n  Cmds 1, src 9, inst 12, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x55ac7cbc97b0, refCt 1, args 3, compiled locals 4\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"x\"\n  Commands 1:\n      1: pc 0-10, src 1-7\n  Command 1: \"unset x...\"\n    (0) unsetScalar silent=no %v3 \t# var \"x\"\n    (6) push 0 \t# \"\"\n    (11) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-71`):

```json
{
  "version": "9.1",
  "label": "array",
  "exit": 0,
  "stdout": "ByteCode 0x55634183d000, refCt 1, epoch 24, interp 0x5563417b7a10 (epoch 24)\n  Source \"\\nunset a(k)...\"\n  Cmds 1, src 12, inst 17, litObjs 2, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x55634184f7b0, refCt 1, args 3, compiled locals 4\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"a\"\n  Commands 1:\n      1: pc 0-15, src 1-10\n  Command 1: \"unset a(k)...\"\n    (0) push 0 \t# \"k\"\n    (5) unsetArray silent=no %v3 \t# var \"a\"\n    (11) push 1 \t# \"\"\n    (16) done \n\nRESULT 1 {can't unset \"a(k)\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-72`):

```json
{
  "version": "9.1",
  "label": "global",
  "exit": 0,
  "stdout": "ByteCode 0x5582d09a0000, refCt 1, epoch 24, interp 0x5582d091aa10 (epoch 24)\n  Source \"\\nunset ::x...\"\n  Cmds 1, src 11, inst 13, litObjs 2, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x5582d09b27b0, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-11, src 1-9\n  Command 1: \"unset ::x...\"\n    (0) push 0 \t# \"::x\"\n    (5) unsetStk silent=no \n    (7) push 1 \t# \"\"\n    (12) done \n\nRESULT 1 {can't unset \"::x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-73`):

```json
{
  "version": "9.1",
  "label": "quiet",
  "exit": 0,
  "stdout": "ByteCode 0x55f0f7fda000, refCt 1, epoch 24, interp 0x55f0f7f54a10 (epoch 24)\n  Source \"\\nunset -nocomplain x a(k)...\"\n  Cmds 1, src 26, inst 23, litObjs 2, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x55f0f7fec7b0, refCt 1, args 3, compiled locals 5\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"x\"\n      slot 4, scalar, \"a\"\n  Commands 1:\n      1: pc 0-21, src 1-24\n  Command 1: \"unset -nocomplain x a(k)...\"\n    (0) unsetScalar silent=yes %v3 \t# var \"x\"\n    (6) push 0 \t# \"k\"\n    (11) unsetArray silent=yes %v4 \t# var \"a\"\n    (17) push 1 \t# \"\"\n    (22) done \n\nRESULT 0 {} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-74`):

```json
{
  "version": "9.1",
  "label": "endflags",
  "exit": 0,
  "stdout": "ByteCode 0x55befe448000, refCt 1, epoch 24, interp 0x55befe3c2a10 (epoch 24)\n  Source \"\\nunset -- -x...\"\n  Cmds 1, src 13, inst 12, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x55befe45a7b0, refCt 1, args 3, compiled locals 4\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"-x\"\n  Commands 1:\n      1: pc 0-10, src 1-11\n  Command 1: \"unset -- -x...\"\n    (0) unsetScalar silent=no %v3 \t# var \"-x\"\n    (6) push 0 \t# \"\"\n    (11) done \n\nRESULT 1 {can't unset \"-x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-75`):

```json
{
  "version": "9.1",
  "label": "quietflags",
  "exit": 0,
  "stdout": "ByteCode 0x5614e18cc000, refCt 1, epoch 24, interp 0x5614e1846a10 (epoch 24)\n  Source \"\\nunset -nocomplain -- x...\"\n  Cmds 1, src 24, inst 12, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x5614e18de7b0, refCt 1, args 3, compiled locals 4\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"x\"\n  Commands 1:\n      1: pc 0-10, src 1-22\n  Command 1: \"unset -nocomplain -- x...\"\n    (0) unsetScalar silent=yes %v3 \t# var \"x\"\n    (6) push 0 \t# \"\"\n    (11) done \n\nRESULT 0 {} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-76`):

```json
{
  "version": "9.1",
  "label": "unknownflag",
  "exit": 0,
  "stdout": "ByteCode 0x55bcb4a0b000, refCt 1, epoch 24, interp 0x55bcb4985a10 (epoch 24)\n  Source \"\\nunset -bad x...\"\n  Cmds 1, src 14, inst 18, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x55bcb4a1d7b0, refCt 1, args 3, compiled locals 5\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"-bad\"\n      slot 4, scalar, \"x\"\n  Commands 1:\n      1: pc 0-16, src 1-12\n  Command 1: \"unset -bad x...\"\n    (0) unsetScalar silent=no %v3 \t# var \"-bad\"\n    (6) unsetScalar silent=no %v4 \t# var \"x\"\n    (12) push 0 \t# \"\"\n    (17) done \n\nRESULT 1 {can't unset \"-bad\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-77`):

```json
{
  "version": "9.1",
  "label": "dynamicfirst",
  "exit": 0,
  "stdout": "ByteCode 0x56412b1d4000, refCt 1, epoch 24, interp 0x56412b14ea10 (epoch 24)\n  Source \"\\nunset $name...\"\n  Cmds 1, src 13, inst 16, litObjs 1, aux 0, stkDepth 2, code/src 0.00\n  Proc 0x56412b1e67b0, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-14, src 1-11\n  Command 1: \"unset $name...\"\n    (0) push 0 \t# \"unset\"\n    (5) loadScalar %v0 \t# var \"name\"\n    (10) invokeStk 2 \n    (15) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-78`):

```json
{
  "version": "9.1",
  "label": "dynamicprefix",
  "exit": 0,
  "stdout": "ByteCode 0x55920d0b3000, refCt 1, epoch 24, interp 0x55920d02da10 (epoch 24)\n  Source \"\\nunset a$name...\"\n  Cmds 1, src 14, inst 20, litObjs 2, aux 0, stkDepth 2, code/src 0.00\n  Proc 0x55920d0c57b0, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-18, src 1-12\n  Command 1: \"unset a$name...\"\n    (0) push 0 \t# \"a\"\n    (5) loadScalar %v0 \t# var \"name\"\n    (10) strcat 2 \n    (12) unsetStk silent=no \n    (14) push 1 \t# \"\"\n    (19) done \n\nRESULT 1 {can't unset \"ax\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-79`):

```json
{
  "version": "9.1",
  "label": "dynamicflag",
  "exit": 0,
  "stdout": "ByteCode 0x55c786ce3000, refCt 1, epoch 24, interp 0x55c786c5da10 (epoch 24)\n  Source \"\\nunset -- $name...\"\n  Cmds 1, src 16, inst 13, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x55c786cf57b0, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-11, src 1-14\n  Command 1: \"unset -- $name...\"\n    (0) loadScalar %v0 \t# var \"name\"\n    (5) unsetStk silent=no \n    (7) push 0 \t# \"\"\n    (12) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-80`):

```json
{
  "version": "9.1",
  "label": "dynamicafterknown",
  "exit": 0,
  "stdout": "ByteCode 0x559b55428000, refCt 1, epoch 24, interp 0x559b553a2a10 (epoch 24)\n  Source \"\\nunset x $name...\"\n  Cmds 1, src 15, inst 21, litObjs 2, aux 0, stkDepth 3, code/src 0.00\n  Proc 0x559b5543a7b0, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-19, src 1-13\n  Command 1: \"unset x $name...\"\n    (0) push 0 \t# \"unset\"\n    (5) push 1 \t# \"x\"\n    (10) loadScalar %v0 \t# var \"name\"\n    (15) invokeStk 3 \n    (20) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-81`):

```json
{
  "version": "9.1",
  "label": "dynamicbeforeknown",
  "exit": 0,
  "stdout": "ByteCode 0x561f6905d000, refCt 1, epoch 24, interp 0x561f68fd7a10 (epoch 24)\n  Source \"\\nunset -- $name x...\"\n  Cmds 1, src 18, inst 19, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x561f6906f7b0, refCt 1, args 3, compiled locals 4\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"x\"\n  Commands 1:\n      1: pc 0-17, src 1-16\n  Command 1: \"unset -- $name x...\"\n    (0) loadScalar %v0 \t# var \"name\"\n    (5) unsetStk silent=no \n    (7) unsetScalar silent=no %v3 \t# var \"x\"\n    (13) push 0 \t# \"\"\n    (18) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-82`):

```json
{
  "version": "9.1",
  "label": "multipledynamic",
  "exit": 0,
  "stdout": "ByteCode 0x5607f3295000, refCt 1, epoch 24, interp 0x5607f320fa10 (epoch 24)\n  Source \"\\nunset -- $name $other...\"\n  Cmds 1, src 23, inst 20, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x5607f32a77b0, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-18, src 1-21\n  Command 1: \"unset -- $name $other...\"\n    (0) loadScalar %v0 \t# var \"name\"\n    (5) unsetStk silent=no \n    (7) loadScalar %v1 \t# var \"other\"\n    (12) unsetStk silent=no \n    (14) push 0 \t# \"\"\n    (19) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-83`):

```json
{
  "version": "9.1",
  "label": "literalexpand",
  "exit": 0,
  "stdout": "ByteCode 0x564b600c0000, refCt 1, epoch 24, interp 0x564b6003aa10 (epoch 24)\n  Source \"\\nunset {*}{x y}...\"\n  Cmds 1, src 16, inst 18, litObjs 1, aux 0, stkDepth 1, code/src 0.00\n  Proc 0x564b600d27b0, refCt 1, args 3, compiled locals 5\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"x\"\n      slot 4, scalar, \"y\"\n  Commands 1:\n      1: pc 0-16, src 1-14\n  Command 1: \"unset {*}{x y}...\"\n    (0) unsetScalar silent=no %v3 \t# var \"x\"\n    (6) unsetScalar silent=no %v4 \t# var \"y\"\n    (12) push 0 \t# \"\"\n    (17) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-84`):

```json
{
  "version": "9.1",
  "label": "dynamicexpand",
  "exit": 0,
  "stdout": "ByteCode 0x557822f7c000, refCt 1, epoch 24, interp 0x557822ef6a10 (epoch 24)\n  Source \"\\nunset {*}$names...\"\n  Cmds 1, src 17, inst 18, litObjs 1, aux 0, stkDepth 2, code/src 0.00\n  Proc 0x557822f8e7b0, refCt 1, args 3, compiled locals 3\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n  Commands 1:\n      1: pc 0-16, src 1-15\n  Command 1: \"unset {*}$names...\"\n    (0) expandStart \n    (1) push 0 \t# \"unset\"\n    (6) loadScalar %v2 \t# var \"names\"\n    (11) expandStkTop 2 \n    (16) invokeExpanded \n    (17) done \n\nRESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-85`):

```json
{
  "version": "9.1",
  "label": "ordered",
  "exit": 0,
  "stdout": "ByteCode 0x5599f54f9000, refCt 1, epoch 24, interp 0x5599f5473a10 (epoch 24)\n  Source \"\\nunset a([set ::seen FIRST]) b([set ::seen SECOND])...\"\n  Cmds 3, src 52, inst 40, litObjs 4, aux 0, stkDepth 2, code/src 0.00\n  Proc 0x5599f550b030, refCt 1, args 3, compiled locals 5\n      slot 0, scalar, arg, \"name\"\n      slot 1, scalar, arg, \"other\"\n      slot 2, scalar, arg, \"names\"\n      slot 3, scalar, \"a\"\n      slot 4, scalar, \"b\"\n  Commands 3:\n      1: pc 0-38, src 1-50        2: pc 0-10, src 10-25\n      3: pc 17-27, src 32-48\n  Command 1: \"unset a([set ::seen FIRST]) b([set ::seen SECOND])...\"\n  Command 2: \"set ::seen FIRST...\"\n    (0) push 0 \t# \"::seen\"\n    (5) push 1 \t# \"FIRST\"\n    (10) storeStk \n    (11) unsetArray silent=no %v3 \t# var \"a\"\n  Command 3: \"set ::seen SECOND...\"\n    (17) push 0 \t# \"::seen\"\n    (22) push 2 \t# \"SECOND\"\n    (27) storeStk \n    (28) unsetArray silent=no %v4 \t# var \"b\"\n    (34) push 3 \t# \"\"\n    (39) done \n\nRESULT 1 {can't unset \"a(FIRST)\": no such variable} FIRST\n",
  "stderr": ""
}
```

`windows.tsv` (`file-103`):

```json
{
  "provider_rows": [
    "9.1\tnone\t756e736574\t1\t0\t524553554c542030207b7d204245464f5245",
    "9.1\tscalar\t756e7365742078\t1\t1\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "9.1\tarray\t756e7365742061286b29\t1\t1\t524553554c542031207b63616e277420756e736574202261286b29223a206e6f2073756368207661726961626c657d204245464f5245",
    "9.1\tglobal\t756e736574203a3a78\t1\t1\t524553554c542031207b63616e277420756e73657420223a3a78223a206e6f2073756368207661726961626c657d204245464f5245",
    "9.1\tquiet\t756e736574202d6e6f636f6d706c61696e20782061286b29\t1\t2\t524553554c542030207b7d204245464f5245",
    "9.1\tendflags\t756e736574202d2d202d78\t1\t1\t524553554c542031207b63616e277420756e73657420222d78223a206e6f2073756368207661726961626c657d204245464f5245",
    "9.1\tquietflags\t756e736574202d6e6f636f6d706c61696e202d2d2078\t1\t1\t524553554c542030207b7d204245464f5245",
    "9.1\tunknownflag\t756e736574202d6261642078\t1\t2\t524553554c542031207b63616e277420756e73657420222d626164223a206e6f2073756368207661726961626c657d204245464f5245",
    "9.1\tdynamicfirst\t756e73657420246e616d65\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "9.1\tdynamicprefix\t756e7365742061246e616d65\t1\t1\t524553554c542031207b63616e277420756e73657420226178223a206e6f2073756368207661726961626c657d204245464f5245",
    "9.1\tdynamicflag\t756e736574202d2d20246e616d65\t1\t1\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "9.1\tdynamicafterknown\t756e736574207820246e616d65\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "9.1\tdynamicbeforeknown\t756e736574202d2d20246e616d652078\t1\t2\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "9.1\tmultipledynamic\t756e736574202d2d20246e616d6520246f74686572\t1\t2\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "9.1\tliteralexpand\t756e736574207b2a7d7b7820797d\t1\t2\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "9.1\tdynamicexpand\t756e736574207b2a7d246e616d6573\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "9.1\tordered\t756e7365742061285b736574203a3a7365656e2046495253545d292062285b736574203a3a7365656e205345434f4e445d29\t1\t2\t524553554c542031207b63616e277420756e73657420226128464952535429223a206e6f2073756368207661726961626c657d204649525354"
  ]
}
```

### jim

Status: `observed`. Version: jim0.84. Build: Original build/library/header/executable identities and compile arguments are retained where captured; absent fields are not reconstructed from present launchers.. Channel: Original script/result controls; launch channel is not recorded unless an argv in the attached receipt states it. No stdin/file equivalence is inferred.. Dialect: jim.

Only the following original capture association answers this question for this provider. Rejected/setup/unavailable rows are retained as such; no result from a different release or entry mode is substituted.

Exact selected capture outcomes (hex fields remain native bytes; process status is distinct from guest completion):

`observations.json` (`file-0-row-86`):

```json
{
  "version": "jim0.84",
  "label": "none",
  "exit": 0,
  "stdout": "RESULT 0 {} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-87`):

```json
{
  "version": "jim0.84",
  "label": "scalar",
  "exit": 0,
  "stdout": "RESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-88`):

```json
{
  "version": "jim0.84",
  "label": "array",
  "exit": 0,
  "stdout": "RESULT 1 {can't unset \"a(k)\": variable isn't array} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-89`):

```json
{
  "version": "jim0.84",
  "label": "global",
  "exit": 0,
  "stdout": "RESULT 1 {can't unset \"::x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-90`):

```json
{
  "version": "jim0.84",
  "label": "quiet",
  "exit": 0,
  "stdout": "RESULT 0 {} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-91`):

```json
{
  "version": "jim0.84",
  "label": "endflags",
  "exit": 0,
  "stdout": "RESULT 1 {can't unset \"-x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-92`):

```json
{
  "version": "jim0.84",
  "label": "quietflags",
  "exit": 0,
  "stdout": "RESULT 0 {} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-93`):

```json
{
  "version": "jim0.84",
  "label": "unknownflag",
  "exit": 0,
  "stdout": "RESULT 1 {can't unset \"-bad\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-94`):

```json
{
  "version": "jim0.84",
  "label": "dynamicfirst",
  "exit": 0,
  "stdout": "RESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-95`):

```json
{
  "version": "jim0.84",
  "label": "dynamicprefix",
  "exit": 0,
  "stdout": "RESULT 1 {can't unset \"ax\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-96`):

```json
{
  "version": "jim0.84",
  "label": "dynamicflag",
  "exit": 0,
  "stdout": "RESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-97`):

```json
{
  "version": "jim0.84",
  "label": "dynamicafterknown",
  "exit": 0,
  "stdout": "RESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-98`):

```json
{
  "version": "jim0.84",
  "label": "dynamicbeforeknown",
  "exit": 0,
  "stdout": "RESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-99`):

```json
{
  "version": "jim0.84",
  "label": "multipledynamic",
  "exit": 0,
  "stdout": "RESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-100`):

```json
{
  "version": "jim0.84",
  "label": "literalexpand",
  "exit": 0,
  "stdout": "RESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-101`):

```json
{
  "version": "jim0.84",
  "label": "dynamicexpand",
  "exit": 0,
  "stdout": "RESULT 1 {can't unset \"x\": no such variable} BEFORE\n",
  "stderr": ""
}
```

`observations.json` (`file-0-row-102`):

```json
{
  "version": "jim0.84",
  "label": "ordered",
  "exit": 0,
  "stdout": "RESULT 1 {can't unset \"a(FIRST)\": variable isn't array} SECOND\n",
  "stderr": ""
}
```

`windows.tsv` (`file-103`):

```json
{
  "provider_rows": [
    "jim0.84\tnone\t756e736574\t0\t0\t524553554c542030207b7d204245464f5245",
    "jim0.84\tscalar\t756e7365742078\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "jim0.84\tarray\t756e7365742061286b29\t0\t0\t524553554c542031207b63616e277420756e736574202261286b29223a207661726961626c652069736e27742061727261797d204245464f5245",
    "jim0.84\tglobal\t756e736574203a3a78\t0\t0\t524553554c542031207b63616e277420756e73657420223a3a78223a206e6f2073756368207661726961626c657d204245464f5245",
    "jim0.84\tquiet\t756e736574202d6e6f636f6d706c61696e20782061286b29\t0\t0\t524553554c542030207b7d204245464f5245",
    "jim0.84\tendflags\t756e736574202d2d202d78\t0\t0\t524553554c542031207b63616e277420756e73657420222d78223a206e6f2073756368207661726961626c657d204245464f5245",
    "jim0.84\tquietflags\t756e736574202d6e6f636f6d706c61696e202d2d2078\t0\t0\t524553554c542030207b7d204245464f5245",
    "jim0.84\tunknownflag\t756e736574202d6261642078\t0\t0\t524553554c542031207b63616e277420756e73657420222d626164223a206e6f2073756368207661726961626c657d204245464f5245",
    "jim0.84\tdynamicfirst\t756e73657420246e616d65\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "jim0.84\tdynamicprefix\t756e7365742061246e616d65\t0\t0\t524553554c542031207b63616e277420756e73657420226178223a206e6f2073756368207661726961626c657d204245464f5245",
    "jim0.84\tdynamicflag\t756e736574202d2d20246e616d65\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "jim0.84\tdynamicafterknown\t756e736574207820246e616d65\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "jim0.84\tdynamicbeforeknown\t756e736574202d2d20246e616d652078\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "jim0.84\tmultipledynamic\t756e736574202d2d20246e616d6520246f74686572\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "jim0.84\tliteralexpand\t756e736574207b2a7d7b7820797d\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "jim0.84\tdynamicexpand\t756e736574207b2a7d246e616d6573\t0\t0\t524553554c542031207b63616e277420756e736574202278223a206e6f2073756368207661726961626c657d204245464f5245",
    "jim0.84\tordered\t756e7365742061285b736574203a3a7365656e2046495253545d292062285b736574203a3a7365656e205345434f4e445d29\t0\t0\t524553554c542031207b63616e277420756e73657420226128464952535429223a207661726961626c652069736e27742061727261797d205345434f4e44"
  ]
}
```

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not tested. Dialect: bigip.

No execution of this question is attached for this provider. An API or command observed elsewhere supplies no answer here.

## Exact evidence

- `file-0` (observation): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. Exact retained capture/provenance artifact; purpose is limited to this question.
- `file-0-row-1` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/0`. Original provider/capture association at its exact selected row.
- `file-0-row-2` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/1`. Original provider/capture association at its exact selected row.
- `file-0-row-3` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/2`. Original provider/capture association at its exact selected row.
- `file-0-row-4` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/3`. Original provider/capture association at its exact selected row.
- `file-0-row-5` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/4`. Original provider/capture association at its exact selected row.
- `file-0-row-6` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/5`. Original provider/capture association at its exact selected row.
- `file-0-row-7` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/6`. Original provider/capture association at its exact selected row.
- `file-0-row-8` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/7`. Original provider/capture association at its exact selected row.
- `file-0-row-9` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/8`. Original provider/capture association at its exact selected row.
- `file-0-row-10` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/9`. Original provider/capture association at its exact selected row.
- `file-0-row-11` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/10`. Original provider/capture association at its exact selected row.
- `file-0-row-12` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/11`. Original provider/capture association at its exact selected row.
- `file-0-row-13` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/12`. Original provider/capture association at its exact selected row.
- `file-0-row-14` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/13`. Original provider/capture association at its exact selected row.
- `file-0-row-15` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/14`. Original provider/capture association at its exact selected row.
- `file-0-row-16` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/15`. Original provider/capture association at its exact selected row.
- `file-0-row-17` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/16`. Original provider/capture association at its exact selected row.
- `file-0-row-18` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/17`. Original provider/capture association at its exact selected row.
- `file-0-row-19` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/18`. Original provider/capture association at its exact selected row.
- `file-0-row-20` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/19`. Original provider/capture association at its exact selected row.
- `file-0-row-21` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/20`. Original provider/capture association at its exact selected row.
- `file-0-row-22` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/21`. Original provider/capture association at its exact selected row.
- `file-0-row-23` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/22`. Original provider/capture association at its exact selected row.
- `file-0-row-24` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/23`. Original provider/capture association at its exact selected row.
- `file-0-row-25` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/24`. Original provider/capture association at its exact selected row.
- `file-0-row-26` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/25`. Original provider/capture association at its exact selected row.
- `file-0-row-27` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/26`. Original provider/capture association at its exact selected row.
- `file-0-row-28` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/27`. Original provider/capture association at its exact selected row.
- `file-0-row-29` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/28`. Original provider/capture association at its exact selected row.
- `file-0-row-30` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/29`. Original provider/capture association at its exact selected row.
- `file-0-row-31` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/30`. Original provider/capture association at its exact selected row.
- `file-0-row-32` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/31`. Original provider/capture association at its exact selected row.
- `file-0-row-33` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/32`. Original provider/capture association at its exact selected row.
- `file-0-row-34` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/33`. Original provider/capture association at its exact selected row.
- `file-0-row-35` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/34`. Original provider/capture association at its exact selected row.
- `file-0-row-36` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/35`. Original provider/capture association at its exact selected row.
- `file-0-row-37` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/36`. Original provider/capture association at its exact selected row.
- `file-0-row-38` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/37`. Original provider/capture association at its exact selected row.
- `file-0-row-39` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/38`. Original provider/capture association at its exact selected row.
- `file-0-row-40` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/39`. Original provider/capture association at its exact selected row.
- `file-0-row-41` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/40`. Original provider/capture association at its exact selected row.
- `file-0-row-42` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/41`. Original provider/capture association at its exact selected row.
- `file-0-row-43` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/42`. Original provider/capture association at its exact selected row.
- `file-0-row-44` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/43`. Original provider/capture association at its exact selected row.
- `file-0-row-45` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/44`. Original provider/capture association at its exact selected row.
- `file-0-row-46` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/45`. Original provider/capture association at its exact selected row.
- `file-0-row-47` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/46`. Original provider/capture association at its exact selected row.
- `file-0-row-48` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/47`. Original provider/capture association at its exact selected row.
- `file-0-row-49` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/48`. Original provider/capture association at its exact selected row.
- `file-0-row-50` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/49`. Original provider/capture association at its exact selected row.
- `file-0-row-51` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/50`. Original provider/capture association at its exact selected row.
- `file-0-row-52` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/51`. Original provider/capture association at its exact selected row.
- `file-0-row-53` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/52`. Original provider/capture association at its exact selected row.
- `file-0-row-54` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/53`. Original provider/capture association at its exact selected row.
- `file-0-row-55` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/54`. Original provider/capture association at its exact selected row.
- `file-0-row-56` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/55`. Original provider/capture association at its exact selected row.
- `file-0-row-57` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/56`. Original provider/capture association at its exact selected row.
- `file-0-row-58` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/57`. Original provider/capture association at its exact selected row.
- `file-0-row-59` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/58`. Original provider/capture association at its exact selected row.
- `file-0-row-60` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/59`. Original provider/capture association at its exact selected row.
- `file-0-row-61` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/60`. Original provider/capture association at its exact selected row.
- `file-0-row-62` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/61`. Original provider/capture association at its exact selected row.
- `file-0-row-63` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/62`. Original provider/capture association at its exact selected row.
- `file-0-row-64` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/63`. Original provider/capture association at its exact selected row.
- `file-0-row-65` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/64`. Original provider/capture association at its exact selected row.
- `file-0-row-66` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/65`. Original provider/capture association at its exact selected row.
- `file-0-row-67` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/66`. Original provider/capture association at its exact selected row.
- `file-0-row-68` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/67`. Original provider/capture association at its exact selected row.
- `file-0-row-69` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/68`. Original provider/capture association at its exact selected row.
- `file-0-row-70` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/69`. Original provider/capture association at its exact selected row.
- `file-0-row-71` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/70`. Original provider/capture association at its exact selected row.
- `file-0-row-72` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/71`. Original provider/capture association at its exact selected row.
- `file-0-row-73` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/72`. Original provider/capture association at its exact selected row.
- `file-0-row-74` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/73`. Original provider/capture association at its exact selected row.
- `file-0-row-75` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/74`. Original provider/capture association at its exact selected row.
- `file-0-row-76` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/75`. Original provider/capture association at its exact selected row.
- `file-0-row-77` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/76`. Original provider/capture association at its exact selected row.
- `file-0-row-78` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/77`. Original provider/capture association at its exact selected row.
- `file-0-row-79` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/78`. Original provider/capture association at its exact selected row.
- `file-0-row-80` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/79`. Original provider/capture association at its exact selected row.
- `file-0-row-81` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/80`. Original provider/capture association at its exact selected row.
- `file-0-row-82` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/81`. Original provider/capture association at its exact selected row.
- `file-0-row-83` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/82`. Original provider/capture association at its exact selected row.
- `file-0-row-84` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/83`. Original provider/capture association at its exact selected row.
- `file-0-row-85` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/84`. Original provider/capture association at its exact selected row.
- `file-0-row-86` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/85`. Original provider/capture association at its exact selected row.
- `file-0-row-87` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/86`. Original provider/capture association at its exact selected row.
- `file-0-row-88` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/87`. Original provider/capture association at its exact selected row.
- `file-0-row-89` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/88`. Original provider/capture association at its exact selected row.
- `file-0-row-90` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/89`. Original provider/capture association at its exact selected row.
- `file-0-row-91` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/90`. Original provider/capture association at its exact selected row.
- `file-0-row-92` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/91`. Original provider/capture association at its exact selected row.
- `file-0-row-93` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/92`. Original provider/capture association at its exact selected row.
- `file-0-row-94` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/93`. Original provider/capture association at its exact selected row.
- `file-0-row-95` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/94`. Original provider/capture association at its exact selected row.
- `file-0-row-96` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/95`. Original provider/capture association at its exact selected row.
- `file-0-row-97` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/96`. Original provider/capture association at its exact selected row.
- `file-0-row-98` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/97`. Original provider/capture association at its exact selected row.
- `file-0-row-99` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/98`. Original provider/capture association at its exact selected row.
- `file-0-row-100` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/99`. Original provider/capture association at its exact selected row.
- `file-0-row-101` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/100`. Original provider/capture association at its exact selected row.
- `file-0-row-102` (provider): [rust/tcl-registry/tests/data/native_unset_compilation/observations.json](../../../../rust/tcl-registry/tests/data/native_unset_compilation/observations.json). SHA-256 `4018e54aaaf1b2a6e2a0fbb3e604d6ed5f97b121c512182f2655d77913fdb4a3`. JSON pointer `/101`. Original provider/capture association at its exact selected row.
- `file-103` (observation): [rust/tcl-registry/tests/data/native_unset_compilation/windows.tsv](../../../../rust/tcl-registry/tests/data/native_unset_compilation/windows.tsv). SHA-256 `465368c31bbc348df41a7981d593200f31eb1cde2c256a9580dbd08e38a2b8dd`. Exact retained capture/provenance artifact; purpose is limited to this question.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

The attached artifacts retain captured outputs, available original inputs and recorded compile/run arguments. A missing input or wrapper limits exact replay; no new native execution is claimed. A replay requires the corresponding source/library/build configuration and original evaluator flags/argv; vanished outside capture paths are provenance, not executable replay dependencies. Where only source hex is retained, preserve those bytes. The per-purpose README/probe supplies the protocol; no new accepted reconfirmation or Rust pass is asserted.

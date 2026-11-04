# Expanded command heads and command replacement

## Question

When a Tcl command head is formed by expanding `{*}{set x}` or the dynamic
prefix `{*}$command`, what happens if an argument renames `set` during the
same invocation? How does that compare with a written `set` command, and does
an empty dynamic expansion expose the following written `set` as the command?
Where the environment exposes Tcl bytecode, what bytecode does it report for
the recorded procedures?

This distinguishes command lookup and evaluation timing that the LSP,
compiler and VM need to model. The reported disassembly is the environment's
stored bytecode, not a trace proving which instructions ran. These results
cover only the script and environments below.

## Assumptions

- The capture does not record whether the script was supplied as a file or on
  standard input, nor the exact C Tcl patch levels or build configuration.
- Disassembly includes process-local addresses; their particular values are not
  a stable semantic property.
- No BIG-IP execution is recorded for this script.

## Script

```tcl-visible
proc expanded {} {{*}{set x} A; set x}
puts [list literal [catch {expanded} message] $message]
if {[llength [info commands ::tcl::unsupported::disassemble]]} {
 puts [list bytecode [::tcl::unsupported::disassemble proc expanded]]
}
proc generic_mutation {} {{*}{set x} [rename set saved; saved value A]}
puts [list expanded_mutation [catch {generic_mutation} message] $message]
if {[llength [info commands saved]]} {rename saved set}
proc inline_mutation {} {set x [rename set saved; saved value A]}
puts [list inline_mutation [catch {inline_mutation} message] $message]
if {[llength [info commands saved]]} {rename saved set}
proc dynamic_mutation {command} {{*}$command [rename set saved; saved value A]}
puts [list dynamic_mutation [catch {dynamic_mutation {set x}} message] $message]
if {[llength [info commands saved]]} {rename saved set}
if {[llength [info commands ::tcl::unsupported::disassemble]]} {
 puts [list dynamic_bytecode [::tcl::unsupported::disassemble proc dynamic_mutation]]
}
proc dynamic_empty {command} {{*}$command set x A; set x}
puts [list dynamic_empty [catch {dynamic_empty {}} message] $message]
```

## Results

### Tcl 8.4

- Exit status: `0`
- Standard output:

```text-visible
literal 1 {extra characters after close-brace}
expanded_mutation 1 {extra characters after close-brace}
inline_mutation 0 A
dynamic_mutation 1 {extra characters after close-brace}
dynamic_empty 1 {extra characters after close-brace}
```

- Standard error: empty.

### Tcl 8.5

- Exit status: `0`
- Standard output:

```text-visible
literal 0 A
bytecode {ByteCode 0x0x5637e5cba7b0, refCt 1, epoch 3, interp 0x0x5637e5c81d90 (epoch 3)
  Source "{*}{set x} A; set x"
  Cmds 2, src 19, inst 17, litObjs 1, aux 0, stkDepth 1, code/src 0.00
  Proc 0x0x5637e5cadb10, refCt 1, args 0, compiled locals 1
      slot 0, scalar, "x"
  Commands 2:
      1: pc 0-4, src 0-11   ⟦TAB⟧   2: pc 5-15, src 14-18
  Command 1: "{*}{set x} A"
    (0) push1 0 ⟦TAB⟧# "A"
    (2) storeScalar1 %v0 ⟦TAB⟧# var "x"
    (4) pop⟦SPACE⟧
  Command 2: "set x"
    (5) startCommand +11 1 ⟦TAB⟧# next cmd at pc 16
    (14) loadScalar1 %v0 ⟦TAB⟧# var "x"
    (16) done⟦SPACE⟧
}
expanded_mutation 0 A
inline_mutation 0 A
dynamic_mutation 1 {too many nested evaluations (infinite loop?)}
dynamic_bytecode {ByteCode 0x0x5637e5d25fd0, refCt 1, epoch 9, interp 0x0x5637e5c81d90 (epoch 9)
  Source "{*}$command [rename set saved; saved value A]"
  Cmds 3, src 45, inst 27, litObjs 5, aux 0, stkDepth 4, code/src 0.00
  Proc 0x0x5637e5cdb910, refCt 1, args 1, compiled locals 1
      slot 0, scalar, arg, "command"
  Commands 3:
      1: pc 0-25, src 0-44   ⟦TAB⟧   2: pc 8-16, src 13-28
      3: pc 17-24, src 31-43
  Command 1: "{*}$command [rename set saved; saved value A]"
    (0) expandStart⟦SPACE⟧
    (1) loadScalar1 %v0 ⟦TAB⟧# var "command"
    (3) expandStkTop 1⟦SPACE⟧
  Command 2: "rename set saved"
    (8) push1 0 ⟦TAB⟧# "rename"
    (10) push1 1 ⟦TAB⟧# "set"
    (12) push1 2 ⟦TAB⟧# "saved"
    (14) invokeStk1 3⟦SPACE⟧
    (16) pop⟦SPACE⟧
  Command 3: "saved value A"
    (17) push1 2 ⟦TAB⟧# "saved"
    (19) push1 3 ⟦TAB⟧# "value"
    (21) push1 4 ⟦TAB⟧# "A"
    (23) invokeStk1 3⟦SPACE⟧
    (25) invokeExpanded⟦SPACE⟧
    (26) done⟦SPACE⟧
}
dynamic_empty 0 A
```

- Standard error: empty.

### Tcl 8.6

- Exit status: `0`
- Standard output:

```text-visible
literal 0 A
bytecode {ByteCode 0x0x562f9a7439b0, refCt 1, epoch 17, interp 0x0x562f9a71b130 (epoch 17)
  Source "{*}{set x} A; set x"
  Cmds 2, src 19, inst 8, litObjs 1, aux 0, stkDepth 1, code/src 0.00
  Proc 0x0x562f9a76a140, refCt 1, args 0, compiled locals 1
      slot 0, scalar, "x"
  Commands 2:
      1: pc 0-4, src 0-11        2: pc 5-6, src 14-18
  Command 1: "{*}{set x} A..."
    (0) push1 0 ⟦TAB⟧# "A"
    (2) storeScalar1 %v0 ⟦TAB⟧# var "x"
    (4) pop⟦SPACE⟧
  Command 2: "set x"
    (5) loadScalar1 %v0 ⟦TAB⟧# var "x"
    (7) done⟦SPACE⟧
}
expanded_mutation 0 A
inline_mutation 0 A
dynamic_mutation 1 {too many nested evaluations (infinite loop?)}
dynamic_bytecode {ByteCode 0x0x562f9a777670, refCt 1, epoch 23, interp 0x0x562f9a71b130 (epoch 23)
  Source "{*}$command [rename set saved; saved value A]"
  Cmds 3, src 45, inst 27, litObjs 5, aux 0, stkDepth 4, code/src 0.00
  Proc 0x0x562f9a7ae1d0, refCt 1, args 1, compiled locals 1
      slot 0, scalar, arg, "command"
  Commands 3:
      1: pc 0-25, src 0-44        2: pc 8-16, src 13-28
      3: pc 17-24, src 31-43
  Command 1: "{*}$command [rename set saved; saved value A]"
    (0) expandStart⟦SPACE⟧
    (1) loadScalar1 %v0 ⟦TAB⟧# var "command"
    (3) expandStkTop 1⟦SPACE⟧
  Command 2: "rename set saved..."
    (8) push1 0 ⟦TAB⟧# "rename"
    (10) push1 1 ⟦TAB⟧# "set"
    (12) push1 2 ⟦TAB⟧# "saved"
    (14) invokeStk1 3⟦SPACE⟧
    (16) pop⟦SPACE⟧
  Command 3: "saved value A..."
    (17) push1 2 ⟦TAB⟧# "saved"
    (19) push1 3 ⟦TAB⟧# "value"
    (21) push1 4 ⟦TAB⟧# "A"
    (23) invokeStk1 3⟦SPACE⟧
    (25) invokeExpanded⟦SPACE⟧
    (26) done⟦SPACE⟧
}
dynamic_empty 0 A
```

- Standard error: empty.

### Tcl 9.0

- Exit status: `0`
- Standard output:

```text-visible
literal 0 A
bytecode {ByteCode 0x557614c8c230, refCt 1, epoch 21, interp 0x557614c04a10 (epoch 21)
  Source "{*}{set x} A; set x"
  Cmds 2, src 19, inst 8, litObjs 1, aux 0, stkDepth 1, code/src 0.00
  Proc 0x557614c9d000, refCt 1, args 0, compiled locals 1
      slot 0, scalar, "x"
  Commands 2:
      1: pc 0-4, src 0-11        2: pc 5-6, src 14-18
  Command 1: "{*}{set x} A..."
    (0) push1 0 ⟦TAB⟧# "A"
    (2) storeScalar1 %v0 ⟦TAB⟧# var "x"
    (4) pop⟦SPACE⟧
  Command 2: "set x"
    (5) loadScalar1 %v0 ⟦TAB⟧# var "x"
    (7) done⟦SPACE⟧
}
expanded_mutation 0 A
inline_mutation 0 A
dynamic_mutation 1 {too many nested evaluations (infinite loop?)}
dynamic_bytecode {ByteCode 0x557614d14e10, refCt 1, epoch 27, interp 0x557614c04a10 (epoch 27)
  Source "{*}$command [rename set saved; saved value A]"
  Cmds 3, src 45, inst 27, litObjs 5, aux 0, stkDepth 4, code/src 0.00
  Proc 0x557614cbcea0, refCt 1, args 1, compiled locals 1
      slot 0, scalar, arg, "command"
  Commands 3:
      1: pc 0-25, src 0-44        2: pc 8-16, src 13-28
      3: pc 17-24, src 31-43
  Command 1: "{*}$command [rename set saved; saved value A]"
    (0) expandStart⟦SPACE⟧
    (1) loadScalar1 %v0 ⟦TAB⟧# var "command"
    (3) expandStkTop 1⟦SPACE⟧
  Command 2: "rename set saved..."
    (8) push1 0 ⟦TAB⟧# "rename"
    (10) push1 1 ⟦TAB⟧# "set"
    (12) push1 2 ⟦TAB⟧# "saved"
    (14) invokeStk1 3⟦SPACE⟧
    (16) pop⟦SPACE⟧
  Command 3: "saved value A..."
    (17) push1 2 ⟦TAB⟧# "saved"
    (19) push1 3 ⟦TAB⟧# "value"
    (21) push1 4 ⟦TAB⟧# "A"
    (23) invokeStk1 3⟦SPACE⟧
    (25) invokeExpanded⟦SPACE⟧
    (26) done⟦SPACE⟧
}
dynamic_empty 0 A
```

- Standard error: empty.

### Tcl 9.1

- Exit status: `0`
- Standard output:

```text-visible
literal 0 A
bytecode {ByteCode 0x561698cac000, refCt 1, epoch 24, interp 0x561698c26a10 (epoch 24)
  Source "{*}{set x} A; set x"
  Cmds 2, src 19, inst 17, litObjs 1, aux 0, stkDepth 1, code/src 0.00
  Proc 0x561698cbe830, refCt 1, args 0, compiled locals 1
      slot 0, scalar, "x"
  Commands 2:
      1: pc 0-10, src 0-11        2: pc 11-15, src 14-18
  Command 1: "{*}{set x} A..."
    (0) push 0 ⟦TAB⟧# "A"
    (5) storeScalar %v0 ⟦TAB⟧# var "x"
    (10) pop⟦SPACE⟧
  Command 2: "set x"
    (11) loadScalar %v0 ⟦TAB⟧# var "x"
    (16) done⟦SPACE⟧
}
expanded_mutation 0 A
inline_mutation 0 A
dynamic_mutation 1 {too many nested evaluations (infinite loop?)}
dynamic_bytecode {ByteCode 0x561698c2b620, refCt 1, epoch 30, interp 0x561698c26a10 (epoch 30)
  Source "{*}$command [rename set saved; saved value A]"
  Cmds 3, src 45, inst 54, litObjs 5, aux 0, stkDepth 4, code/src 0.00
  Proc 0x561698cfb360, refCt 1, args 1, compiled locals 1
      slot 0, scalar, arg, "command"
  Commands 3:
      1: pc 0-52, src 0-44        2: pc 11-31, src 13-28
      3: pc 32-51, src 31-43
  Command 1: "{*}$command [rename set saved; saved value A]"
    (0) expandStart⟦SPACE⟧
    (1) loadScalar %v0 ⟦TAB⟧# var "command"
    (6) expandStkTop 1⟦SPACE⟧
  Command 2: "rename set saved..."
    (11) push 0 ⟦TAB⟧# "rename"
    (16) push 1 ⟦TAB⟧# "set"
    (21) push 2 ⟦TAB⟧# "saved"
    (26) invokeStk 3⟦SPACE⟧
    (31) pop⟦SPACE⟧
  Command 3: "saved value A..."
    (32) push 2 ⟦TAB⟧# "saved"
    (37) push 3 ⟦TAB⟧# "value"
    (42) push 4 ⟦TAB⟧# "A"
    (47) invokeStk 3⟦SPACE⟧
    (52) invokeExpanded⟦SPACE⟧
    (53) done⟦SPACE⟧
}
dynamic_empty 0 A
```

- Standard error: empty.

### Jim Tcl

- Exit status: `0`
- Standard output:

```text-visible
literal 0 A
expanded_mutation 1 {invalid command name "set"}
inline_mutation 1 {invalid command name "set"}
dynamic_mutation 1 {invalid command name "set"}
dynamic_empty 0 A
```

- Standard error: empty.

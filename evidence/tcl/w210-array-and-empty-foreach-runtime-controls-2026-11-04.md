# W210 array and empty-loop runtime controls

## Question

Do the exact Tcl operations involved in open reports #2379, #2374 and #2441 return normally or fail with unset array elements/variables on the recorded interpreters? This runtime evidence checks the behavior behind W210 expectations; it does not establish what the analyzer reports.

## Assumptions

The script uses `catch` around the missing-element and missing-variable operations so one interpreter run records both failures and continues. The may-set array procedure is invoked with both true and false inputs. This is a small semantic control, not the complete LSP command or an exact byte-for-byte copy of each issue source file. Native Jim and C Tcl observations are recorded separately; Jim 0.84 does not implement Tcl `array statistics` or `array startsearch`, so those subcommand results are not semantically comparable.

## Script

```tcl-visible
proc maySetArray {c} {
    if {$c} {set a(k) 1}
    puts [array size a]
    puts [array names a]
}
puts [array size a]
maySetArray 1
maySetArray 0
puts [array exists a]
puts [array get a]
foreach command {{array statistics absent} {array startsearch absent}} {
    set rc [catch $command message]
    puts [list issue2379-control $command $rc $message]
}
proc distinctElement {} {
    set a(k) 1
    if {[info exists a(k)]} { puts $a(j) }
}
set rc [catch {distinctElement} message]
puts [list issue2374 $rc $message]
proc emptyForeachRead {} {
    foreach x {} {}
    if {[info exists x]} { puts yes }
    puts $x
}
foreach y {} {}
set procRc [catch {emptyForeachRead} procMessage]
set topRc [catch {puts $y} topMessage]
puts [list issue2441-proc $procRc $procMessage]
puts [list issue2441-top $topRc $topMessage]
```

## Results

### Tcl 8.4.20

- Interpreter: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.4`; SHA-256 `704c2caa06eddb67f97e128572395e1ed81a87bc07544b3b55efcd3139a1dc98`.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.4 /tmp/w210-2379-2374-2441-exact-oracle.tcl`.
- Exit status: 0.
- Standard output:

```text-visible
0
1
k
0

0

issue2379-control {array statistics absent} 1 {"absent" isn't an array}
issue2379-control {array startsearch absent} 1 {"absent" isn't an array}
issue2374 1 {can't read "a(j)": no such element in array}
issue2441-proc 1 {can't read "x": no such variable}
issue2441-top 1 {can't read "y": no such variable}
```

- Standard error:

```text-visible
⟦EOF⟧
```

### Tcl 8.5.19

- Interpreter: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.5`; SHA-256 `ec41a6a74436030c15a3c45b9eeae821509148223711e9dd0993907fe5a59eb6`.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.5 /tmp/w210-2379-2374-2441-exact-oracle.tcl`.
- Exit status: 0.
- Standard output:

```text-visible
0
1
k
0

0

issue2379-control {array statistics absent} 1 {"absent" isn't an array}
issue2379-control {array startsearch absent} 1 {"absent" isn't an array}
issue2374 1 {can't read "a(j)": no such element in array}
issue2441-proc 1 {can't read "x": no such variable}
issue2441-top 1 {can't read "y": no such variable}
```

- Standard error:

```text-visible
⟦EOF⟧
```

### Tcl 8.6.18

- Interpreter: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.6`; SHA-256 `d98e632bcfbb411d33841cd75873d374dc950bf1a34ce4b20b657208d51f746c`.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.6 /tmp/w210-2379-2374-2441-exact-oracle.tcl`.
- Exit status: 0.
- Standard output:

```text-visible
0
1
k
0

0

issue2379-control {array statistics absent} 1 {"absent" isn't an array}
issue2379-control {array startsearch absent} 1 {"absent" isn't an array}
issue2374 1 {can't read "a(j)": no such element in array}
issue2441-proc 1 {can't read "x": no such variable}
issue2441-top 1 {can't read "y": no such variable}
```

- Standard error:

```text-visible
⟦EOF⟧
```

### Tcl 9.0.4

- Interpreter: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh9.0`; SHA-256 `843e6c3f41064c654932a3677be76baddf10e59d340fd6de2a0ea56cffaf5f35`.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh9.0 /tmp/w210-2379-2374-2441-exact-oracle.tcl`.
- Exit status: 0.
- Standard output:

```text-visible
0
1
k
0

0

issue2379-control {array statistics absent} 1 {"absent" isn't an array}
issue2379-control {array startsearch absent} 1 {"absent" isn't an array}
issue2374 1 {can't read "a(j)": no such element in array}
issue2441-proc 1 {can't read "x": no such variable}
issue2441-top 1 {can't read "y": no such variable}
```

- Standard error:

```text-visible
⟦EOF⟧
```

### Tcl 9.1.0

- Interpreter: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh9.1`; SHA-256 `f185a54d2fff0c33aa3d8fd5b7f867be7eceba237dea736b86c24e15b3fae0fa`.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh9.1 /tmp/w210-2379-2374-2441-exact-oracle.tcl`.
- Exit status: 0.
- Standard output:

```text-visible
0
1
k
0

0

issue2379-control {array statistics absent} 1 {"absent" isn't an array}
issue2379-control {array startsearch absent} 1 {"absent" isn't an array}
issue2374 1 {can't read "a(j)": no such element in array}
issue2441-proc 1 {can't read "x": no such variable}
issue2441-top 1 {can't read "y": no such variable}
```

- Standard error:

```text-visible
⟦EOF⟧
```

### Jim 0.84

- Interpreter: `/workspace/tcl-lsp/tmp/lsp-oracles/jim-install/bin/jimsh`; SHA-256 `0fd5a727a6b4c0856818e9237e3e962fe1062c4057bcb5b3f768efd768cfae52`.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/jim-install/bin/jimsh /tmp/w210-2379-2374-2441-exact-oracle.tcl`.
- Exit status: 0.
- Standard output:

```text-visible
0
1
k
0

0

issue2379-control {array statistics absent} 1 {array, unknown command "statistics": should be exists, get, names, set, size, stat, unset}
issue2379-control {array startsearch absent} 1 {array, unknown command "startsearch": should be exists, get, names, set, size, stat, unset}
issue2374 1 {can't read "a(j)": no such element in array}
issue2441-proc 1 {can't read "x": no such variable}
issue2441-top 1 {can't read "y": no such variable}
```

- Standard error:

```text-visible
⟦EOF⟧
```

The five C Tcl interpreters returned the same results: absent-array `size` is `0`; a local array set on the true call has size `1` and name `k`; the false call has size `0` and no names; an unset global array reports `array exists` as `0` and `array get` as empty. The distinct `a(j)` read, procedure-local `x` read after an empty `foreach`, and top-level `y` read after an empty `foreach` each fail with the shown missing-value message. C Tcl `statistics` and `startsearch` fail because `absent` is not an array. Jim 0.84 rejects those two commands as unsupported subcommands, so its error text is not a Tcl semantic comparison.

# Empty foreach retains nested argument writes

## Question

Does the command substitution that supplies an empty foreach list still write
the caller variable before the later read, even when the document load ended
in an independently caught error? This checks the native word-effect control
for #2441; it does not establish analyser or Native admission behaviour.

## Assumptions

The script catches the namespace load error and then explicitly invokes its
retained procedure. The substitution helper uses `upvar 1` to write `x`, and
returns an empty list. Invocation uses stdin with a fresh process for each
recorded interpreter. Additional startup assumptions: Not recorded. The five
C Tcl paths are invocation wrappers; their recorded executable hashes refer
to those wrapper files. The Jim path names its actual executable.

## Script

```tcl-visible
set loadRc [catch {namespace eval ::nested {
    proc operation {} {upvar 1 x cell; set cell 7; return {}}
    proc p {} {foreach y [operation] {}; return $x}
    error fail
}} loadResult]
set callRc [catch {::nested::p} callResult]
puts [list load $loadRc $loadResult call $callRc $callResult]
```

## Results


### Tcl 8.4 8.4.20

- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.4` with the Script bytes on standard input.
- Version observation: `puts [info patchlevel]` on standard input; exit 0.
- Exit status: 0.
- Standard output:

```text-visible
load 1 fail call 0 7
```

- Standard error:

```text-visible
⟦EOF⟧
```


### Tcl 8.5 8.5.19

- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.5` with the Script bytes on standard input.
- Version observation: `puts [info patchlevel]` on standard input; exit 0.
- Exit status: 0.
- Standard output:

```text-visible
load 1 fail call 0 7
```

- Standard error:

```text-visible
⟦EOF⟧
```


### Tcl 8.6 8.6.18

- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.6` with the Script bytes on standard input.
- Version observation: `puts [info patchlevel]` on standard input; exit 0.
- Exit status: 0.
- Standard output:

```text-visible
load 1 fail call 0 7
```

- Standard error:

```text-visible
⟦EOF⟧
```


### Tcl 9.0 9.0.4

- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh9.0` with the Script bytes on standard input.
- Version observation: `puts [info patchlevel]` on standard input; exit 0.
- Exit status: 0.
- Standard output:

```text-visible
load 1 fail call 0 7
```

- Standard error:

```text-visible
⟦EOF⟧
```


### Tcl 9.1 9.1.0

- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh9.1` with the Script bytes on standard input.
- Version observation: `puts [info patchlevel]` on standard input; exit 0.
- Exit status: 0.
- Standard output:

```text-visible
load 1 fail call 0 7
```

- Standard error:

```text-visible
⟦EOF⟧
```


### JimTcl 0.84-10-g82fb4d0

- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/jimtcl/jimsh` with the Script bytes on standard input.
- Version observation: `puts [info patchlevel]` on standard input; exit 0.
- Exit status: 0.
- Standard output:

```text-visible
load 1 fail call 0 7
```

- Standard error:

```text-visible
⟦EOF⟧
```


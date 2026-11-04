# Empty foreach source phases preserve argument effects

## Question

Do stock empty foreach loops preserve installed commands and existing variables while skipping binder stores and body evaluation, and do argument errors, nonempty loops and replacement handlers retain their own effects? This measures native semantics for the Jim source-phase proposal in #2441; it establishes no compiler or physical execution authority.

## Assumptions

Each run starts a fresh process. The run helper catches and evaluates each case in a new procedure activation. Cases intentionally redefine the global preserved procedure; each relevant case first installs its own original. List syntax is interpreted by the actual selected engine. Startup assumptions beyond the recorded executable: Not recorded. The five C Tcl executables are wrappers.

## Script

```tcl-visible
proc run {label script} {
    set code [catch {eval $script} result]
    puts [list $label $code $result]
}
run empty {
    proc preserved {} {return ORIGINAL}
    foreach x {} {rename preserved {}; error BODY}
    list [preserved] [info exists x]
}
run multiple-empty {
    proc preserved {} {return ORIGINAL}
    foreach {x y} {} z {} {rename preserved {}; error BODY}
    list [preserved] [info exists x] [info exists y] [info exists z]
}
run existing-array {
    set x(k) KEEP
    foreach x {} {error BODY}
    list $x(k) [array exists x]
}
run nonempty {
    proc preserved {} {return ORIGINAL}
    foreach x {1} {proc preserved {} {return BODY}}
    list [preserved] $x
}
run list-argument-effect {
    proc preserved {} {return ORIGINAL}
    foreach x [eval {proc preserved {} {return ARGUMENT}; list}] {error BODY}
    list [preserved] [info exists x]
}
run body-argument-effect {
    proc preserved {} {return ORIGINAL}
    foreach x {} [eval {proc preserved {} {return BODY_WORD}; list error BODY}]
    list [preserved] [info exists x]
}
run argument-error {
    set code [catch {foreach x [error ARGUMENT] {error BODY}} result]
    list $code $result [info exists x]
}
run empty-varlist {
    set code [catch {foreach {} {} {error BODY}} result]
    list $code $result
}
run malformed-values {
    set values "\{"
    set code [catch {foreach x $values {error BODY}} result]
    list $code $result [info exists x]
}
run wrong-arity {
    set code [catch {foreach x {} {} extra} result]
    list $code $result
}
run shadowed-handler {
    rename foreach stock_foreach
    proc foreach {args} {proc preserved {} {return SHADOW}; return SHADOW_RESULT}
    set code [catch {foreach x {} {error BODY}} result]
    rename foreach {}
    rename stock_foreach foreach
    list $code $result [preserved] [info exists x]
}
puts [list observer-command trace [info commands trace] xtrace [info commands xtrace]]
```

## Results

### Tcl 8.4 8.4.20

- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.4` with Script bytes on standard input.
- Version observation: `puts [info patchlevel]` on standard input; exit 0.
- Exit status: 0.
- Standard output:

```text-visible
empty 0 {ORIGINAL 0}
multiple-empty 0 {ORIGINAL 0 0 0}
existing-array 0 {KEEP 1}
nonempty 0 {BODY 1}
list-argument-effect 0 {ARGUMENT 0}
body-argument-effect 0 {BODY_WORD 0}
argument-error 0 {1 ARGUMENT 0}
empty-varlist 0 {1 {foreach varlist is empty}}
malformed-values 0 {1 {unmatched open brace in list} 0}
wrong-arity 0 {1 {wrong # args: should be "foreach varList list ?varList list ...? command"}}
shadowed-handler 0 {0 SHADOW_RESULT SHADOW 0}
observer-command trace trace xtrace {}
```

- Standard error:

```text-visible
⟦EOF⟧
```

### Tcl 8.5 8.5.19

- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.5` with Script bytes on standard input.
- Version observation: `puts [info patchlevel]` on standard input; exit 0.
- Exit status: 0.
- Standard output:

```text-visible
empty 0 {ORIGINAL 0}
multiple-empty 0 {ORIGINAL 0 0 0}
existing-array 0 {KEEP 1}
nonempty 0 {BODY 1}
list-argument-effect 0 {ARGUMENT 0}
body-argument-effect 0 {BODY_WORD 0}
argument-error 0 {1 ARGUMENT 0}
empty-varlist 0 {1 {foreach varlist is empty}}
malformed-values 0 {1 {unmatched open brace in list} 0}
wrong-arity 0 {1 {wrong # args: should be "foreach varList list ?varList list ...? command"}}
shadowed-handler 0 {0 SHADOW_RESULT SHADOW 0}
observer-command trace trace xtrace {}
```

- Standard error:

```text-visible
⟦EOF⟧
```

### Tcl 8.6 8.6.18

- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.6` with Script bytes on standard input.
- Version observation: `puts [info patchlevel]` on standard input; exit 0.
- Exit status: 0.
- Standard output:

```text-visible
empty 0 {ORIGINAL 0}
multiple-empty 0 {ORIGINAL 0 0 0}
existing-array 0 {KEEP 1}
nonempty 0 {BODY 1}
list-argument-effect 0 {ARGUMENT 0}
body-argument-effect 0 {BODY_WORD 0}
argument-error 0 {1 ARGUMENT 0}
empty-varlist 0 {1 {foreach varlist is empty}}
malformed-values 0 {1 {unmatched open brace in list} 0}
wrong-arity 0 {1 {wrong # args: should be "foreach varList list ?varList list ...? command"}}
shadowed-handler 0 {0 SHADOW_RESULT SHADOW 0}
observer-command trace trace xtrace {}
```

- Standard error:

```text-visible
⟦EOF⟧
```

### Tcl 9.0 9.0.4

- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh9.0` with Script bytes on standard input.
- Version observation: `puts [info patchlevel]` on standard input; exit 0.
- Exit status: 0.
- Standard output:

```text-visible
empty 0 {ORIGINAL 0}
multiple-empty 0 {ORIGINAL 0 0 0}
existing-array 0 {KEEP 1}
nonempty 0 {BODY 1}
list-argument-effect 0 {ARGUMENT 0}
body-argument-effect 0 {BODY_WORD 0}
argument-error 0 {1 ARGUMENT 0}
empty-varlist 0 {1 {foreach varlist is empty}}
malformed-values 0 {1 {unmatched open brace in list} 0}
wrong-arity 0 {1 {wrong # args: should be "foreach varList list ?varList list ...? command"}}
shadowed-handler 0 {0 SHADOW_RESULT SHADOW 0}
observer-command trace trace xtrace {}
```

- Standard error:

```text-visible
⟦EOF⟧
```

### Tcl 9.1 9.1.0

- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh9.1` with Script bytes on standard input.
- Version observation: `puts [info patchlevel]` on standard input; exit 0.
- Exit status: 0.
- Standard output:

```text-visible
empty 0 {ORIGINAL 0}
multiple-empty 0 {ORIGINAL 0 0 0}
existing-array 0 {KEEP 1}
nonempty 0 {BODY 1}
list-argument-effect 0 {ARGUMENT 0}
body-argument-effect 0 {BODY_WORD 0}
argument-error 0 {1 ARGUMENT 0}
empty-varlist 0 {1 {foreach varlist is empty}}
malformed-values 0 {1 {unmatched open brace in list} 0}
wrong-arity 0 {1 {wrong # args: should be "foreach varList list ?varList list ...? command"}}
shadowed-handler 0 {0 SHADOW_RESULT SHADOW 0}
observer-command trace trace xtrace {}
```

- Standard error:

```text-visible
⟦EOF⟧
```

### JimTcl 0.84-10-g82fb4d0

- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/jimtcl/jimsh` with Script bytes on standard input.
- Version observation: `puts [info patchlevel]` on standard input; exit 0.
- Exit status: 0.
- Standard output:

```text-visible
empty 0 {ORIGINAL 0}
multiple-empty 0 {ORIGINAL 0 0 0}
existing-array 0 {KEEP 1}
nonempty 0 {BODY 1}
list-argument-effect 0 {ARGUMENT 0}
body-argument-effect 0 {BODY_WORD 0}
argument-error 0 {1 ARGUMENT 0}
empty-varlist 0 {1 {foreach varlist is empty}}
malformed-values 0 {1 BODY 1}
wrong-arity 0 {1 {wrong # args: should be "foreach varList list ?varList list ...? script"}}
shadowed-handler 0 {0 SHADOW_RESULT SHADOW 0}
observer-command trace {} xtrace xtrace
```

- Standard error:

```text-visible
⟦EOF⟧
```

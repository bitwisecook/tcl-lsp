# Tcl execution evidence

## Question

Can a three-iteration increment/list cycle execute when its initial value is definitely nonnumeric?

## Assumptions

Host recorded by the capture: `Linux-6.18.57-x86_64-with-glibc2.41`. Working directory: `/tmp/lsp-940-seed-oracles`. Each interpreter starts a new process and reads the script file named in its argument vector. `LC_ALL=C` is set; other inherited environment variables and compiler flags are Not recorded.

The Tcl catch status printed by the script is separate from the process exit status. These observations establish update outcomes and displayed values only. No representation observer or product diagnostic was invoked; internal integer width, object identity and warning correctness are not inferred from the printed value.

[Raw capture with binary/script hashes](../../docs/LSP_RELEASE_SHIMMER_SEED_RECEIPT_2026-11-04.json).

## Script

```tcl-visible
set x abc
set status [catch {
    foreach i {1 2 3} {
        incr x
        lindex $x 0
    }
} result]
puts [list patchlevel [info patchlevel] catch $status result $result value $x]
```

## Results

### Tcl 8.4

- Recorded version: 8.4.20.
- Input channel: command-line script file.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.4 /tmp/lsp-940-seed-oracles/invalid.tcl`
- Working directory: `/tmp/lsp-940-seed-oracles`
- Exit status: 0
- Standard output:

```text-visible
patchlevel 8.4.20 catch 1 result {expected integer but got "abc"} value abc
```

- Standard error:

```text-visible
⟦EOF⟧
```

### Tcl 8.5

- Recorded version: 8.5.19.
- Input channel: command-line script file.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.5 /tmp/lsp-940-seed-oracles/invalid.tcl`
- Working directory: `/tmp/lsp-940-seed-oracles`
- Exit status: 0
- Standard output:

```text-visible
patchlevel 8.5.19 catch 1 result {expected integer but got "abc"} value abc
```

- Standard error:

```text-visible
⟦EOF⟧
```

### Tcl 8.6

- Recorded version: 8.6.18.
- Input channel: command-line script file.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.6 /tmp/lsp-940-seed-oracles/invalid.tcl`
- Working directory: `/tmp/lsp-940-seed-oracles`
- Exit status: 0
- Standard output:

```text-visible
patchlevel 8.6.18 catch 1 result {expected integer but got "abc"} value abc
```

- Standard error:

```text-visible
⟦EOF⟧
```

### Tcl 9.0

- Recorded version: 9.0.4.
- Input channel: command-line script file.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh9.0 /tmp/lsp-940-seed-oracles/invalid.tcl`
- Working directory: `/tmp/lsp-940-seed-oracles`
- Exit status: 0
- Standard output:

```text-visible
patchlevel 9.0.4 catch 1 result {expected integer but got "abc"} value abc
```

- Standard error:

```text-visible
⟦EOF⟧
```

### Tcl 9.1

- Recorded version: 9.1.0.
- Input channel: command-line script file.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh9.1 /tmp/lsp-940-seed-oracles/invalid.tcl`
- Working directory: `/tmp/lsp-940-seed-oracles`
- Exit status: 0
- Standard output:

```text-visible
patchlevel 9.1.0 catch 1 result {expected integer but got "abc"} value abc
```

- Standard error:

```text-visible
⟦EOF⟧
```

### JimTcl

- Recorded version: 0.84-10-g82fb4d0.
- Input channel: command-line script file.
- Invocation: `/workspace/tcl-lsp/tmp/lsp-oracles/jimtcl/jimsh /tmp/lsp-940-seed-oracles/invalid.tcl`
- Working directory: `/tmp/lsp-940-seed-oracles`
- Exit status: 0
- Standard output:

```text-visible
patchlevel 0.84-10-g82fb4d0 catch 1 result {expected integer but got "abc"} value abc
```

- Standard error:

```text-visible
⟦EOF⟧
```

# Tcl execution evidence

## Question

With the conditional element write executed by argument 1, does the same-element existence guard permit the read and complete normally? This is the present branch of the W210 guard regression.

## Assumptions

Additional experimental assumptions: Not recorded. Each retained row ran a new
CLI process with the script supplied on standard input. Streams were retained
by Python subprocess text capture; undecoded output bytes were not separately
recorded. The script prints its catch status but does not print the caught
result. Private cell/object identities, compiler facts and LSP diagnostics were
not measured by this Tcl execution.

The retained capture and capture controller are in
[`w210-short-circuit-array-boundaries`](../../rust/tcl-compiler/tests/data/w210-short-circuit-array-boundaries/README.md).

## Script

```tcl-visible
proc p {c} {if {$c} {set a(safe) 1}; if {[info exists a(safe)] && $a(safe) > 0} {puts yes}}
set status [catch {p 1} result]
puts "STATUS:$status"
```

## Results

### C Tcl 8.4.20

- Recorded version: 8.4.20 (queried in a separate process before this run).
- Recorded input channel: standard input of `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.4`, with no command-line arguments.
- Recorded binary SHA-256 before the case group: `704c2caa06eddb67f97e128572395e1ed81a87bc07544b3b55efcd3139a1dc98`.
- Exit status: 0
- Standard output:

```text-visible
yes
STATUS:0
```

- Standard error:

```text-visible
⟦EOF⟧
```


### C Tcl 8.5.19

- Recorded version: 8.5.19 (queried in a separate process before this run).
- Recorded input channel: standard input of `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.5`, with no command-line arguments.
- Recorded binary SHA-256 before the case group: `ec41a6a74436030c15a3c45b9eeae821509148223711e9dd0993907fe5a59eb6`.
- Exit status: 0
- Standard output:

```text-visible
yes
STATUS:0
```

- Standard error:

```text-visible
⟦EOF⟧
```


### C Tcl 8.6.18

- Recorded version: 8.6.18 (queried in a separate process before this run).
- Recorded input channel: standard input of `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh8.6`, with no command-line arguments.
- Recorded binary SHA-256 before the case group: `d98e632bcfbb411d33841cd75873d374dc950bf1a34ce4b20b657208d51f746c`.
- Exit status: 0
- Standard output:

```text-visible
yes
STATUS:0
```

- Standard error:

```text-visible
⟦EOF⟧
```


### C Tcl 9.0.4

- Recorded version: 9.0.4 (queried in a separate process before this run).
- Recorded input channel: standard input of `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh9.0`, with no command-line arguments.
- Recorded binary SHA-256 before the case group: `843e6c3f41064c654932a3677be76baddf10e59d340fd6de2a0ea56cffaf5f35`.
- Exit status: 0
- Standard output:

```text-visible
yes
STATUS:0
```

- Standard error:

```text-visible
⟦EOF⟧
```


### C Tcl 9.1.0

- Recorded version: 9.1.0 (queried in a separate process before this run).
- Recorded input channel: standard input of `/workspace/tcl-lsp/tmp/lsp-oracles/bin/tclsh9.1`, with no command-line arguments.
- Recorded binary SHA-256 before the case group: `f185a54d2fff0c33aa3d8fd5b7f867be7eceba237dea736b86c24e15b3fae0fa`.
- Exit status: 0
- Standard output:

```text-visible
yes
STATUS:0
```

- Standard error:

```text-visible
⟦EOF⟧
```


### Jim Tcl 0.84-10-g82fb4d0

- Recorded version: 0.84-10-g82fb4d0 (queried in a separate process before this run).
- Recorded input channel: standard input of `/workspace/tcl-lsp/tmp/lsp-oracles/jimtcl/jimsh`, with no command-line arguments.
- Recorded binary SHA-256 before the case group: `4086bc7223d3d0e853f7c094fdc6cf6f35f7e4c1837a3ff8f6abd0792cdedca8`.
- Exit status: 0
- Standard output:

```text-visible
yes
STATUS:0
```

- Standard error:

```text-visible
⟦EOF⟧
```


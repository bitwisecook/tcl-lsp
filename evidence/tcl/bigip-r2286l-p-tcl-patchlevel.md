# BIG-IP and appliance Tcl context: p tcl patchlevel

## Question

When evaluating this source, is it true that the tcl_patchLevel variable exists in this execution context? This constrains Tcl parsing, command availability or evaluation in the LSP, compiler and VM for the specific source and contexts tested.

## Assumptions

Each wrapper dynamically decoded and evaluated the exact source under catch. Cases were run sequentially within the wrapper, so state left by earlier cases may matter; the contexts and versions are separate observations. The result is the Tcl catch value, not the wrapper process's exit status. Other assumptions were not separately recorded.

## Script

```tcl-visible
if {[info exists tcl_patchLevel]} {set tcl_patchLevel} else {set x UNSET}⟦EOF⟧
```

## Results

### TMM log tag tmm, RULE_INIT, case p_tcl_patchlevel; TMM log tag tmm1, RULE_INIT, case p_tcl_patchlevel; TMM log tag tmm2, RULE_INIT, case p_tcl_patchlevel; TMM log tag tmm3, RULE_INIT, case p_tcl_patchlevel; iApp implementation, case p_tcl_patchlevel; iCall script, case p_tcl_patchlevel

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.6.
- Recorded input channel: TMM log tag tmm, RULE_INIT; TMM log tag tmm1, RULE_INIT; TMM log tag tmm2, RULE_INIT; TMM log tag tmm3, RULE_INIT; iApp implementation; iCall script.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
8.4.6⟦EOF⟧
```

The wrapper recorded Tcl completion code 0.

### appliance tclsh8.4, case p_tcl_patchlevel

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.13.
- Recorded input channel: appliance tclsh8.4.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
8.4.13⟦EOF⟧
```

The wrapper recorded Tcl completion code 0.

### appliance tclsh8.5, case p_tcl_patchlevel

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.5.13.
- Recorded input channel: appliance tclsh8.5.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
8.5.13⟦EOF⟧
```

The wrapper recorded Tcl completion code 0.

### tmsh CLI script, case p_tcl_patchlevel

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.6.
- Recorded input channel: tmsh CLI script.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
8.4.6⟦EOF⟧
```

The wrapper recorded Tcl completion code 0.

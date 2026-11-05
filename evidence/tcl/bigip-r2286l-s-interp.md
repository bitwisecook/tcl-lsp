# BIG-IP and appliance Tcl context: s interp

## Question

When evaluating this source, is it true that interp aliases is available and reports aliases? This constrains Tcl parsing, command availability or evaluation in the LSP, compiler and VM for the specific source and contexts tested.

## Assumptions

Each wrapper dynamically decoded and evaluated the exact source under catch. Cases were run sequentially within the wrapper, so state left by earlier cases may matter; the contexts and versions are separate observations. The result is the Tcl catch value, not the wrapper process's exit status. Other assumptions were not separately recorded.

## Script

```tcl-visible
interp aliases⟦EOF⟧
```

## Results

### TMM log tag tmm, RULE_INIT, case s_interp; TMM log tag tmm1, RULE_INIT, case s_interp; TMM log tag tmm2, RULE_INIT, case s_interp; TMM log tag tmm3, RULE_INIT, case s_interp; iApp implementation, case s_interp; iCall script, case s_interp

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.6.
- Recorded input channel: TMM log tag tmm, RULE_INIT; TMM log tag tmm1, RULE_INIT; TMM log tag tmm2, RULE_INIT; TMM log tag tmm3, RULE_INIT; iApp implementation; iCall script.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
⟦EOF⟧
```

The wrapper recorded Tcl completion code 0.

### appliance tclsh8.4, case s_interp

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.13.
- Recorded input channel: appliance tclsh8.4.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
⟦EOF⟧
```

The wrapper recorded Tcl completion code 0.

### appliance tclsh8.5, case s_interp

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.5.13.
- Recorded input channel: appliance tclsh8.5.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
⟦EOF⟧
```

The wrapper recorded Tcl completion code 0.

### tmsh CLI script, case s_interp

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.6.
- Recorded input channel: tmsh CLI script.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
⟦EOF⟧
```

The wrapper recorded Tcl completion code 0.

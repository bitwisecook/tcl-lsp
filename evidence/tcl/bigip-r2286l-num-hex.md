# BIG-IP and appliance Tcl context: num hex

## Question

When evaluating this source, is it true that a hexadecimal integer literal is accepted and evaluated? This constrains Tcl parsing, command availability or evaluation in the LSP, compiler and VM for the specific source and contexts tested.

## Assumptions

Each wrapper dynamically decoded and evaluated the exact source under catch. Cases were run sequentially within the wrapper, so state left by earlier cases may matter; the contexts and versions are separate observations. The result is the Tcl catch value, not the wrapper process's exit status. Other assumptions were not separately recorded.

## Script

```tcl-visible
expr {0x10}⟦EOF⟧
```

## Results

### TMM log tag tmm, RULE_INIT, case num_hex; TMM log tag tmm1, RULE_INIT, case num_hex; TMM log tag tmm2, RULE_INIT, case num_hex; TMM log tag tmm3, RULE_INIT, case num_hex; iApp implementation, case num_hex; iCall script, case num_hex; tmsh CLI script, case num_hex; TMM log tag tmm, RULE_INIT, case x_num_hex; TMM log tag tmm1, RULE_INIT, case x_num_hex; TMM log tag tmm2, RULE_INIT, case x_num_hex; TMM log tag tmm3, RULE_INIT, case x_num_hex; iApp implementation, case x_num_hex; iCall script, case x_num_hex

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.6.
- Recorded input channel: TMM log tag tmm, RULE_INIT; TMM log tag tmm1, RULE_INIT; TMM log tag tmm2, RULE_INIT; TMM log tag tmm3, RULE_INIT; iApp implementation; iCall script; tmsh CLI script.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
16⟦EOF⟧
```

The wrapper recorded Tcl completion code 0.

### appliance tclsh8.4, case num_hex; appliance tclsh8.4, case x_num_hex

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.13.
- Recorded input channel: appliance tclsh8.4.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
16⟦EOF⟧
```

The wrapper recorded Tcl completion code 0.

### appliance tclsh8.5, case num_hex; appliance tclsh8.5, case x_num_hex

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.5.13.
- Recorded input channel: appliance tclsh8.5.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
16⟦EOF⟧
```

The wrapper recorded Tcl completion code 0.

### tmsh CLI script, case x_num_hex

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.6.
- Recorded input channel: tmsh CLI script.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
16⟦EOF⟧
```

The wrapper recorded Tcl completion code 0.

# BIG-IP and appliance Tcl context: x e starts with

## Question

When evaluating this source, is it true that starts_with tests the indicated string prefix? This constrains Tcl parsing, command availability or evaluation in the LSP, compiler and VM for the specific source and contexts tested.

## Assumptions

Each wrapper dynamically decoded and evaluated the exact source under catch. Cases were run sequentially within the wrapper, so state left by earlier cases may matter; the contexts and versions are separate observations. The result is the Tcl catch value, not the wrapper process's exit status. Other assumptions were not separately recorded.

## Script

```tcl-visible
expr {"abcd" starts_with "ab"}⟦EOF⟧
```

## Results

### TMM log tag tmm, RULE_INIT, case x_e_starts_with; TMM log tag tmm1, RULE_INIT, case x_e_starts_with; TMM log tag tmm2, RULE_INIT, case x_e_starts_with; TMM log tag tmm3, RULE_INIT, case x_e_starts_with; iApp implementation, case x_e_starts_with; iCall script, case x_e_starts_with

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.6.
- Recorded input channel: TMM log tag tmm, RULE_INIT; TMM log tag tmm1, RULE_INIT; TMM log tag tmm2, RULE_INIT; TMM log tag tmm3, RULE_INIT; iApp implementation; iCall script.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
1⟦EOF⟧
```

The wrapper recorded Tcl completion code 0.

### appliance tclsh8.4, case x_e_starts_with

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.13.
- Recorded input channel: appliance tclsh8.4.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
syntax error in expression ""abcd" starts_with "ab"": extra tokens at end of expression⟦EOF⟧
```

The wrapper recorded Tcl completion code 1.

### appliance tclsh8.5, case x_e_starts_with

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.5.13.
- Recorded input channel: appliance tclsh8.5.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
invalid bareword "starts_with"
in expression ""abcd" starts_with "ab"";
should be "$starts_with" or "{starts_with}" or "starts_with(...)" or ...⟦EOF⟧
```

The wrapper recorded Tcl completion code 1.

### tmsh CLI script, case x_e_starts_with

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.6.
- Recorded input channel: tmsh CLI script.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
1⟦EOF⟧
```

The wrapper recorded Tcl completion code 0.

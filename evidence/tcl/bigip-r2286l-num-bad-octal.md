# BIG-IP and appliance Tcl context: num bad octal

## Question

When evaluating this source, is it true that a leading-zero integer containing 9 is rejected as invalid octal? This constrains Tcl parsing, command availability or evaluation in the LSP, compiler and VM for the specific source and contexts tested.

## Assumptions

Each wrapper dynamically decoded and evaluated the exact source under catch. Cases were run sequentially within the wrapper, so state left by earlier cases may matter; the contexts and versions are separate observations. The result is the Tcl catch value, not the wrapper process's exit status. Other assumptions were not separately recorded.

## Script

```tcl-visible
expr {09}⟦EOF⟧
```

## Results

### TMM log tag tmm, RULE_INIT, case num_bad_octal; TMM log tag tmm1, RULE_INIT, case num_bad_octal; TMM log tag tmm2, RULE_INIT, case num_bad_octal; TMM log tag tmm3, RULE_INIT, case num_bad_octal

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.6.
- Recorded input channel: TMM log tag tmm, RULE_INIT; TMM log tag tmm1, RULE_INIT; TMM log tag tmm2, RULE_INIT; TMM log tag tmm3, RULE_INIT.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
expected integer but got "09" (looks like invalid octal numb⟦EOF⟧
```

The wrapper recorded Tcl completion code 1.

### appliance tclsh8.4, case num_bad_octal

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.13.
- Recorded input channel: appliance tclsh8.4.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
expected integer but got "09" (looks like invalid octal number)⟦EOF⟧
```

The wrapper recorded Tcl completion code 1.

### appliance tclsh8.5, case num_bad_octal

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.5.13.
- Recorded input channel: appliance tclsh8.5.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
invalid bareword "09"
in expression "09";
should be "$09" or "{09}" or "09(...)" or ... (invalid octal number?)⟦EOF⟧
```

The wrapper recorded Tcl completion code 1.

### iApp implementation, case num_bad_octal; iCall script, case num_bad_octal

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.6.
- Recorded input channel: iApp implementation; iCall script.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
expected integer but got "09" (looks like invalid octal number)⟦EOF⟧
```

The wrapper recorded Tcl completion code 1.

### tmsh CLI script, case num_bad_octal

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.6.
- Recorded input channel: tmsh CLI script.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
expected integer but got "09" (looks like invalid octal number)⟦EOF⟧
```

The wrapper recorded Tcl completion code 1.

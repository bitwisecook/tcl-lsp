# BIG-IP and appliance Tcl context: g cmd glued

## Question

When evaluating this source, is it true that a command name can be immediately followed by braced words? This constrains Tcl parsing, command availability or evaluation in the LSP, compiler and VM for the specific source and contexts tested.

## Assumptions

Each wrapper dynamically decoded and evaluated the exact source under catch. Cases were run sequentially within the wrapper, so state left by earlier cases may matter; the contexts and versions are separate observations. The result is the Tcl catch value, not the wrapper process's exit status. Other assumptions were not separately recorded.

## Script

```tcl-visible
if{1}{set ::m 44}⟦EOF⟧
```

## Results

### TMM log tag tmm, RULE_INIT, case g_cmd_glued; TMM log tag tmm1, RULE_INIT, case g_cmd_glued; TMM log tag tmm2, RULE_INIT, case g_cmd_glued; TMM log tag tmm3, RULE_INIT, case g_cmd_glued; iApp implementation, case g_cmd_glued; iCall script, case g_cmd_glued

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.6.
- Recorded input channel: TMM log tag tmm, RULE_INIT; TMM log tag tmm1, RULE_INIT; TMM log tag tmm2, RULE_INIT; TMM log tag tmm3, RULE_INIT; iApp implementation; iCall script.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
invalid command name "if{1}{set"⟦EOF⟧
```

The wrapper recorded Tcl completion code 1.

### appliance tclsh8.4, case g_cmd_glued

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.13.
- Recorded input channel: appliance tclsh8.4.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
invalid command name "if{1}{set"⟦EOF⟧
```

The wrapper recorded Tcl completion code 1.

### appliance tclsh8.5, case g_cmd_glued

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.5.13.
- Recorded input channel: appliance tclsh8.5.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
invalid command name "if{1}{set"⟦EOF⟧
```

The wrapper recorded Tcl completion code 1.

### tmsh CLI script, case g_cmd_glued

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.6.
- Recorded input channel: tmsh CLI script.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
invalid command name "if{1}{set"⟦EOF⟧
```

The wrapper recorded Tcl completion code 1.

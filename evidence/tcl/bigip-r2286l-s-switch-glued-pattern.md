# BIG-IP and appliance Tcl context: s switch glued pattern

## Question

When evaluating this source, is it true that a switch list accepts a pattern immediately adjacent to its braced body? This constrains Tcl parsing, command availability or evaluation in the LSP, compiler and VM for the specific source and contexts tested.

## Assumptions

Each wrapper dynamically decoded and evaluated the exact source under catch. Cases were run sequentially within the wrapper, so state left by earlier cases may matter; the contexts and versions are separate observations. The result is the Tcl catch value, not the wrapper process's exit status. Other assumptions were not separately recorded.

## Script

```tcl-visible
switch -- a {a{set m yes} default {set m no}}; set m⟦EOF⟧
```

## Results

### TMM log tag tmm, RULE_INIT, case s_switch_glued_pattern; TMM log tag tmm1, RULE_INIT, case s_switch_glued_pattern; TMM log tag tmm2, RULE_INIT, case s_switch_glued_pattern; TMM log tag tmm3, RULE_INIT, case s_switch_glued_pattern; iApp implementation, case s_switch_glued_pattern; iCall script, case s_switch_glued_pattern

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.6.
- Recorded input channel: TMM log tag tmm, RULE_INIT; TMM log tag tmm1, RULE_INIT; TMM log tag tmm2, RULE_INIT; TMM log tag tmm3, RULE_INIT; iApp implementation; iCall script.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
extra switch pattern with no body⟦EOF⟧
```

The wrapper recorded Tcl completion code 1.

### appliance tclsh8.4, case s_switch_glued_pattern

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.13.
- Recorded input channel: appliance tclsh8.4.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
extra switch pattern with no body⟦EOF⟧
```

The wrapper recorded Tcl completion code 1.

### appliance tclsh8.5, case s_switch_glued_pattern

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.5.13.
- Recorded input channel: appliance tclsh8.5.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
extra switch pattern with no body⟦EOF⟧
```

The wrapper recorded Tcl completion code 1.

### tmsh CLI script, case s_switch_glued_pattern

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.6.
- Recorded input channel: tmsh CLI script.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
extra switch pattern with no body⟦EOF⟧
```

The wrapper recorded Tcl completion code 1.

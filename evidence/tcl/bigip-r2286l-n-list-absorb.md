# BIG-IP and appliance Tcl context: n list absorb

## Question

When evaluating this source, is it true that a braced word on a new line is absorbed by the preceding list command? This constrains Tcl parsing, command availability or evaluation in the LSP, compiler and VM for the specific source and contexts tested.

## Assumptions

Each wrapper dynamically decoded and evaluated the exact source under catch. Cases were run sequentially within the wrapper, so state left by earlier cases may matter; the contexts and versions are separate observations. The result is the Tcl catch value, not the wrapper process's exit status. Other assumptions were not separately recorded.

## Script

```tcl-visible
list a b
{c}⟦EOF⟧
```

## Results

### TMM log tag tmm, RULE_INIT, case n_list_absorb; TMM log tag tmm1, RULE_INIT, case n_list_absorb; TMM log tag tmm2, RULE_INIT, case n_list_absorb; TMM log tag tmm3, RULE_INIT, case n_list_absorb; iApp implementation, case n_list_absorb; iCall script, case n_list_absorb; tmsh CLI script, case n_list_absorb; TMM log tag tmm, RULE_INIT, case x_n_list_absorb; TMM log tag tmm1, RULE_INIT, case x_n_list_absorb; TMM log tag tmm2, RULE_INIT, case x_n_list_absorb; TMM log tag tmm3, RULE_INIT, case x_n_list_absorb; iApp implementation, case x_n_list_absorb; iCall script, case x_n_list_absorb

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.6.
- Recorded input channel: TMM log tag tmm, RULE_INIT; TMM log tag tmm1, RULE_INIT; TMM log tag tmm2, RULE_INIT; TMM log tag tmm3, RULE_INIT; iApp implementation; iCall script; tmsh CLI script.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
a b c⟦EOF⟧
```

The wrapper recorded Tcl completion code 0.

### appliance tclsh8.4, case n_list_absorb; appliance tclsh8.4, case x_n_list_absorb

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.13.
- Recorded input channel: appliance tclsh8.4.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
invalid command name "c"⟦EOF⟧
```

The wrapper recorded Tcl completion code 1.

### appliance tclsh8.5, case n_list_absorb; appliance tclsh8.5, case x_n_list_absorb

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.5.13.
- Recorded input channel: appliance tclsh8.5.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
invalid command name "c"⟦EOF⟧
```

The wrapper recorded Tcl completion code 1.

### tmsh CLI script, case x_n_list_absorb

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.6.
- Recorded input channel: tmsh CLI script.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
a b c⟦EOF⟧
```

The wrapper recorded Tcl completion code 0.

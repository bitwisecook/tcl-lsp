# BIG-IP and appliance Tcl context: e precedence not starts

## Question

When evaluating this source, is it true that not binds before starts_with for the tested expression? This constrains Tcl parsing, command availability or evaluation in the LSP, compiler and VM for the specific source and contexts tested.

## Assumptions

Each wrapper dynamically decoded and evaluated the exact source under catch. Cases were run sequentially within the wrapper, so state left by earlier cases may matter; the contexts and versions are separate observations. The result is the Tcl catch value, not the wrapper process's exit status. Other assumptions were not separately recorded.

## Script

```tcl-visible
expr {not "abc" starts_with "a"}⟦EOF⟧
```

## Results

### TMM log tag tmm, RULE_INIT, case e_precedence_not_starts; TMM log tag tmm1, RULE_INIT, case e_precedence_not_starts; TMM log tag tmm2, RULE_INIT, case e_precedence_not_starts; TMM log tag tmm3, RULE_INIT, case e_precedence_not_starts; iApp implementation, case e_precedence_not_starts; iCall script, case e_precedence_not_starts

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.6.
- Recorded input channel: TMM log tag tmm, RULE_INIT; TMM log tag tmm1, RULE_INIT; TMM log tag tmm2, RULE_INIT; TMM log tag tmm3, RULE_INIT; iApp implementation; iCall script.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
can't use non-numeric string as operand of "!"⟦EOF⟧
```

The wrapper recorded Tcl completion code 1.

### appliance tclsh8.4, case e_precedence_not_starts

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.13.
- Recorded input channel: appliance tclsh8.4.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
syntax error in expression "not "abc" starts_with "a"": variable references require preceding $⟦EOF⟧
```

The wrapper recorded Tcl completion code 1.

### appliance tclsh8.5, case e_precedence_not_starts

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.5.13.
- Recorded input channel: appliance tclsh8.5.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
invalid bareword "not"
in expression "not "abc" starts_with "a"";
should be "$not" or "{not}" or "not(...)" or ...⟦EOF⟧
```

The wrapper recorded Tcl completion code 1.

### tmsh CLI script, case e_precedence_not_starts

- Recorded version: BIG-IP 21.1.0.1, Tcl 8.4.6.
- Recorded input channel: tmsh CLI script.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
can't use non-numeric string as operand of "!"⟦EOF⟧
```

The wrapper recorded Tcl completion code 1.

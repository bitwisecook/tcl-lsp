The `rows.txt` corpus records the completion code and result bytes for the same original scripts under Tcl 8.4, 8.5, 8.6, 9.0, 9.1 and current Jim. Each row contains the dialect, case number, completion code, hexadecimal result and hexadecimal source.

The controls cover arithmetic and bitwise operators, comparisons and membership, identity and single-operand forms, static and dynamic argument expansion, renamed and imported commands, operand errors and floating-point folds. Tcl 8.4 and Jim report the absence of the `tcl::mathop` command family. The four string-ordering commands are available from Tcl 9.0.

Original comparison compilers return `1` without evaluating an operand when fewer than two operands are present. More than two comparison operands require a procedure compiler frame and use an anonymous local. Tcl 8.5 clears that local by storing an empty value; later versions unset it. Wrong arity selects ordinary command dispatch before compiler operand preparation.

The VM and Runtime compare all 84 records. The separate `native_mathop_identity` corpus covers command identity and invocation presentation.

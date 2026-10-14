# Tk linked-variable observations

The original source-built C Tcl 8.6.18 executable loaded the independently installed system Tk 8.6.16 library under task-local Xvfb. `source.tcl`, raw stdout/stderr, before/after input hashes, and hashes of 20 libraries mapped by the actual interpreter are retained. All captured inputs remained unchanged. The original Tcl release library, including its original `init.tcl`, was used; tcltest was not used.

An empty entry `-textvariable` disconnects the link and leaves the Tcl empty-named variable unchanged. The value `$literal` names that literal scalar; `items(key)` names the array element. An entry created in a namespace links an unqualified name in the interpreter-global namespace. These observations concern this actual Tcl/Tk pair only. They do not establish other Tk versions, other widgets/options, C Tcl 9, Jim, F5, Native handler admission, or compiler conformance. The process mappings were captured during execution and libraries hashed afterwards; complete loader dependency closure before execution is not certified.

The failed default-sandbox display connection and missing-Tk-library preflights are retained separately. To repeat, run `python3 capture.py --out /tmp/tk-linked-variable-repeat` in an environment permitting a local X display. The runner creates new output and preserves existing evidence.

The separate `empty-array-root` run retains the actual failure caused by the pre-existing empty-named scalar. `empty-array-root-unset` explicitly removes that scalar before creating the link and observes successful `(key)` linkage, with its own unchanged-input receipt. These are distinct original scripts and retained raw observations.

`qualified-unicode-nul` retains actual runtime-created Unicode NUL in qualified scalar and array-root linked-variable names. Both full names receive the entry value and the shorter scalar remains PRIOR. Stdout retains binary NUL bytes. This C8.6.18/system Tk8.6.16 observation cannot certify raw Native Eval byte NUL, Native string-pool equivalence, other interpreters or compiler admission.

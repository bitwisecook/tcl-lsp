# Original declaring-class reports

`completed-cases/declaring-class-name.tcl` measures seven independent cases on original C Tcl 8.6.18, 9.0.4 and 9.1.0. Equal method reports retain distinct defining classes (`::A::B` and `::A`). Inherited bodies report their declaring class (`::Parent`), and its renamed command reports `::Moved`. Own-object methods raise `method not defined by a class`. Constructor bodies report `::Constructor`.

A class body created inside literal namespace `N:` raises `::N:::C does not refer to an object`; a class body created with original operand `:C` raises `:::C does not refer to an object`. These body declarations are not entered and cannot supply a defining-class constant. The earlier uncaught experiment is retained separately; only its collision case was reached before the namespace error.

Original C Tcl 8.4.20, 8.5.19 and Jim 0.84-9-g5bac7c9 have no TclOO command in the configured interpreters. They explicitly report unsupported; this does not constitute method conformance acceptance.

Before/after manifests retain the actual interpreter, source-tree library files and filesystem dependencies reported by ldd. Tcl 9 ZIPFS library bytes are covered by the executable hash. All recorded inventories stayed unchanged. These are original interpreter observations, not compiled Compiler acceptance. No tcltest package is used.

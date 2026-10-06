# Native switch compiler source geometry

These tables record original C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0
procedure results and counted compiled-local layouts for `probe.c`. Hexadecimal
fields retain exact bytes. `manifest.json` identifies each native source, static
library, command and output hash.

The six bodies cover separate arms, list arms, glob matching, a continuation
forcing a duplicate body, parser-expanded arms and an original substituted
subject. In Tcl 8.5 and later, exact switch compilation omits a masked duplicate
body: its local name is absent from the compiled layout. Glob compilation and a
preceding continuation retain that body. The subject's child compiles before
all reached arm bodies. Tcl 8.4 has no switch compiler and rejects the expansion
syntax. Source tests consume the shared native recipe's original body spans and
check the same compiler visitation distinctions.

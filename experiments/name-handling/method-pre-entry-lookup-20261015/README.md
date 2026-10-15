# Method lookup before entry

A method declaration supplies source candidates; it does not fix the command binding at a later method entry. The probe calls the same method before and after replacing its fully qualified helper. C Tcl 8.6, 9.0 and 9.1 return `OLD` and then `NEW`. C Tcl 8.4, 8.5 and the selected Jim build report that TclOO is unavailable. F5 has no result in this corpus.

The method passes a braced literal argument, so the replacement happens outside argument evaluation. A possible source call edge can coexist with unresolved alternatives when earlier mutations remain unknown. The corpus establishes that distinction; it supplies no closed world, physical command identity or general execution permission.

[probe.tcl](probe.tcl) and [manifest.json](manifest.json) retain the input, exact original outputs, interpreter hashes and matching C Tcl `init.tcl` hashes. The probe uses no tcltest. Run `python3 replay.py --root /path/to/tcl-lsp --out /fresh/output` with the original providers installed under `tmp/`. The replay refuses mismatched provider inputs and compares both output channels byte for byte.

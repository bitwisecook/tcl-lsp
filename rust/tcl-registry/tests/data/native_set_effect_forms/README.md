# Native variable effect forms

`probe.tcl` observes the selected `set` write/read forms and `incr` using
actual variable traces. C Tcl 8.4–9.1 runs the same source; the script selects
the supported trace registration syntax. A write invokes the write observer
without invoking the read observer. A read invokes the read observer, and
`incr` invokes both. Current Jim has no `trace` command, which is recorded as
an unavailable observation surface.

The manifest records original source, interpreter binaries and output hashes.
Each native process has a 60-second timeout.

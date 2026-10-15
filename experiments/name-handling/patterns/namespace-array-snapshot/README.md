# Array snapshot root and key coordinates

Question: does the shared script snapshot helper retain a separately supplied array root and key when the root contains an opening parenthesis?

Capture03 ran the exact retained helper in original C Tcl8.4.20,8.5.19,8.6.18,9.0.4 and9.1.0. Each interpreter recorded its actual version/library and exited normally with empty stderr. Actual loaded init.tcl bytes were retained and match the exact pinned original source library. Tcl9 uses its original embedded ZIPFS library; the capture does not relabel it as a physical directory.

The normal-root control reads PLAIN. For the unusual root `a(`, the retained coordinate reports an array and direct `array get` reports `k ROOT_WITH_OPEN_PAREN`; the helper returns OTHER_ARRAY from the competing array `a` and key `(k`. All five providers agree on these exact inputs. Raw stdout/stderr, exact inputs, source/executable/init hashes and checks are retained beside the probe.

The source helper combines root and key for `set`; this changes the input form. Its CLI purity-snapshot consumer is `rust/tcl-cli/src/commands/spec_test.rs::snapshot`. A repair must preserve the actual coordinate purpose or refuse an unrepresentable read. No physical cell certificate, generic snapshot/lifecycle authority or whole purity-test acceptance follows. No Rust repair or conformance test has run.

Capture01 retains the initial observations without an embedded provider line. Capture02 preserves a failed pairing setup: it assumed a physical library path and refused the original Tcl9 ZIPFS library. Capture03 repeats all40 observation rows byte-for-byte, records actual provider/library pairing and verifies actual loaded original init bytes, source and executable currentness. None uses tcltest or modifies provider binaries/libraries.

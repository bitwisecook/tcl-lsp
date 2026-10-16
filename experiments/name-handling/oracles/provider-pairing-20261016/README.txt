Provider pairing capture, 2026-10-16

Purpose: authenticate the installed reference executables and matching original
script/test support used for setup. No semantic conformance cases were run.

Run: python3 capture.py /workspace/tcl-lsp
This invokes existing binaries only, with raw stdout/stderr/exit retained.
It never builds, installs, edits source trees or copies provider source/binaries.

The C execution adapter follows tcl_test_support::run_script_from_source_tree:
matching source-tree TCL_LIBRARY, TCLLIBPATH removed, unix LD_LIBRARY_PATH first,
original script on stdin. Each interpreter reports its patchlevel/library and
requires the matching original tcltest package, whose actual loader is recorded.
All five C lanes accepted with empty stderr. Existing immutable upstream
inventories are referenced and revalidated rather than duplicated: 233/233,
969/969, 1007/1007, 1068/1068 and 1082/1082 inputs matched for C 8.4.20, 8.5.19,
8.6.18, 9.0.4 and 9.1.0 respectively. Named init/tcltest/support/native source,
static archives, actual executable and shared-object hashes are recorded.

Jim uses original file mode and explicitly sources original tcltest.tcl so
argv0 exists. Setup accepted with zero tests run and empty stderr. The pinned
manifest, clean tracked source and contemporaneous upstream HEAD agree at
5bac7c99ad65864c87da513e22e2f01703fa4e03. The exact remote-observation UTC interval
is in capture.json. Embedded initjimsh/tclcompat source and generated-C hashes
are retained alongside the binary/libjim.a and runtime shared-object hashes.

The exploratory Jim -e/package-require surface failed because argv0 is absent;
its raw failure is retained and explicitly excluded from pairing acceptance.

capture.json records commands, environment changes, timestamps, all raw-channel
hashes, file hashes, integrity references, acceptance results and limitations.
The adapter/probes and existing shared harness/reference owners are hashed.
A static archive hash is installed-build provenance, not a reproducible-build
proof. Pairing acceptance grants no namespace/name/runtime semantics, native
execution authority, complete source closure or F5/appliance result.

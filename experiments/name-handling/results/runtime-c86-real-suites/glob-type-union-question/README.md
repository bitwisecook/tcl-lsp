# Original glob type unions

The unchanged C86 `all.tcl` wrapper reached Tcltest file selection but sourced zero files. `tcltest::GetMatchingFiles` requests `glob -types {b c f p s}`. Runtime's `entry_matches_types` requires every selected kind, so it excludes an ordinary test file.

[The input](input.tcl) probes 21 fixed selector lists against three new regular files and one new directory. The files have controlled executable/readonly properties. The capture creates no special nodes and emits only fixture names and guest diagnostics. It does not inspect host device or security data.

[Fresh captures](capture-01/provenance.json) cover C8.4.20, C8.5.19, C8.6.18, C9.0.4, C9.1.0 and pinned Jim. All five C providers have identical 18 success rows: kinds form a union, and permission qualifiers intersect with that union. `{f d}` selects all four entries; `{b c f p s}` selects the three regular files; `{f d x}` selects the directory and executable file. `{f d r w}` excludes the controlled readonly file; `{f d readonly}` selects that file alone.

The three C error rows preserve release-specific error codes: C84/C85 use `NONE`, while C86+ report `TCL ARGUMENT BAD` or `TCL VALUE LIST BRACE`. Jim rejects `-types` for all 21 rows; its kind and permission semantics were never reached. [Comparison](comparison.json) and [source provenance](source-provenance.json) preserve these separate axes. F5 was not run.

The minimal prospective Runtime change has one source owner, `runtime/rust/src/cmd_fs.rs`: union accepted file-kind selectors, then retain the existing qualifier checks. The registry already documents this distinction. Runtime readonly, accurate access checks, malformed/invalid selector diagnostics, Jim option admission, hidden entries, symlinks and special-kind capabilities are separate obligations; these captures do not establish their Runtime implementation.

Proposed controls use retained C86 rows for ordinary file/directory unions, the original five-kind Tcltest selector, executable qualification after a union, and the existing readable/writable/executable conjunction. Those Runtime controls were unexecuted when this question was sealed. Actual source and implementation results belong in a separate result directory.

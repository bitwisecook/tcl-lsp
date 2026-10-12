# Original all.tcl: Unicode refusal at byte 3

Question: which operation reaches the checked host-Unicode boundary after the C86 getter is admitted?

The unchanged getter-runtime `capture-01/all-tcl-var.tcl` uses real init/tcltest and selects original `var.test` through original `all.tcl`.
Its Native C86 exit is 2, with an operational refusal before any suite starts; C86 exits 0 (110 total, 90 passed, 20 skipped, 0 failed).
The retained getter wrapper, environment controls and original raw streams remain unchanged.
Named controls are only the benign NH_CLOCK_GETENV_* fixtures; inherited environment values are neither enumerated nor printed.

`capture-01` retains the unsuccessful trace setup: lazy clock procedures were not loaded yet (unknown GetSystemTimeZone).
`capture-02/03` explicitly source the unchanged real clock.tcl before installing diagnostic execution traces; they do not replace clock procedures or all.tcl.
Fixed labels establish `clock format` -> GetSystemTimeZone -> ReadZoneinfoFile, read return, five binary scan returns, then split entry without split return.
The exact original caller is `library/clock.tcl:3495`: `set abbrList [split $abbrs \0]`; `$abbrs` came from the preceding `binary scan $d @${seek}a${nChar} abbrs`.
The final trace reports input byte extent 5 and delimiter extent 1; no actual argument contents, file contents, path argument or host environment values are logged.
This identifies the operation and extent, without inferring the timezone name or asserting malformed filesystem data.

The three production owners inspected for this boundary are:
- `rust/tcl-cmd-core/src/list.rs:700`: split requests `try_as_str` on the whole input, then the delimiter, and splits Rust Unicode characters.
- `runtime/rust/src/value_ops.rs:1645`: `try_as_str` projects original object bytes through RawString::unicode.
- `rust/tcl-syntax/src/raw_string.rs:875`: strict std::str::from_utf8 caches the refusal; its display reports valid_up_to=3.
The shared trait/error APIs confirm that failure stays outside guest completion; no error weakening is proposed.

`capture-04` uses only synthetic binary octets 61626300 (`abc` plus NUL) and 616263; it prints only these known synthetic hex values and extents.
Both engines report extent 5 for the NUL case; C86 split returns two elements, Native C86 refuses at byte 3. ASCII control succeeds in both.
`capture-05` repeats both direct byte-array and binary-scan NUL cases without preliminary hex/bytelength access: the same refusal persists; ASCII succeeds.
Thus this is a native encoded-NUL consumer gap, reproducible without any host environment or filesystem input.
C86's actual `generic/tclCmdMZ.c:Tcl_SplitObjCmd` reads original string extents and selected Tcl UTF character decoding; it does not require Rust UTF-8.

Proposed next bounded window: give shared split a separately admitted Native C86 path that follows that byte/character algorithm and constructs original byte slices, retaining native operation currency and genuine result ownership.
Do not globally loosen RawString::unicode, use a lossy replacement, assume Unicode-unit-cache materialization is equivalent, or admit unmeasured profiles.
Measure NUL input/delimiter, empty/default sets, raw invalid bytes, release character boundaries, element reuse/cache effects, and refusal-negative adapters before any parity claim.
This remains separate from the retained three-row env-array/host synchronization difference.
Sources, exact argv, binary/source hashes, per-engine exit codes and raw stdout/stderr channels are retained in capture-*/provenance.json; historical probe source copies match their recorded hashes.
No Rust edits, Cargo, Git, network, canonical documentation changes or suite-pass claim were made in this diagnosis.

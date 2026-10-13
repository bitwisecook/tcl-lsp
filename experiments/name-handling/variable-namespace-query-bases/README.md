# Namespace variable query bases

The exact ASCII source-file input ran once on C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and Jim 0.84-9-g5bac7c9. Each process exited 0 with empty stderr and eight query rows: 48 total. Complete stdout/stderr and source/binary/init/library hashes are retained in `record.json`; PATCHLEVEL is reported by each actual process.

C8.4–8.6 namespace-variable queries try the global candidate after a missing relative candidate, including qualified relative names. C9.0/9.1 omit that alternate. Every C release reports the local child, absolute root and declared undefined namespace cell; absent names and procedure-only locals report empty. The procedure global link does not replace the namespace table query. Jim reports canonical spelling even for absent names or procedure-only locals.

These answers cover the original ASCII source input and reported names. They grant no raw CString NUL or original object/cache/cell identity, ordinary scalar/element read/write behavior, F5 result, or Rust execution. Jim's canonicalization is independent of C lookup.

Replay `input.tcl` once with each recorded executable and compare its complete stdout/stderr and exit status. The original captured argv used the byte-identical `/tmp/name-variable-query-controls.tcl` source path; the retained source hash identifies the input.

`result-input.tcl` retains the same eight queries and collects their reported rows before printing and returning them. A separate single execution on each of the six original providers matched the first input's complete stdout exactly: a further 48 query comparisons, all exits 0 and empty stderr. `result-record.json` and the separate result streams retain this source and capture. Returning the collected text permits a direct interpreter regression without substituting a scalar read for the namespace query. Neither input establishes object or cell identity.

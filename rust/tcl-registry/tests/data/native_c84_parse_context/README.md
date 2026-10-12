# Native C Tcl 8.4 parse contexts

`probe.c` passes original counted source to `Tcl_ParseCommand` and procedure
invocation. The 27 records contain 26 malformed-command parser failures and one
accepted command whose missing-command failure occurs during execution.

Each record retains the parser command start, remaining command size, and term,
the original source bytes, and the invoked result and `errorInfo`. The result is
captured before reading metadata. No traces are installed. These records do not
attest return options, internal representations, or reference counts.

Compilation context uses the full remaining source and excludes the parser term
only when that term is its final byte. Runtime parse context retains the extent
through the term. Empty compilation context is a proved extent; it differs from
unavailable source geometry. Raw NUL is retained in source transport before the
selected native diagnostic renderer applies its own boundary.

`manifest.json` records producer, output, and library hashes. The registry tests
compare all 26 compilation contexts. Compiler tests preserve both independent
extents and exercise the original byte command-plan producer.

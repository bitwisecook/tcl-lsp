# Exact literal names in MCP command metadata

Question: does the `command_info` tool preserve an incoming command-name
value when querying the shared iRules registry context?

The implementation removes the consumer's whitespace trim before the
central context query. The version parameter retains its independent
version-metadata handling. The fixed control uses public MCP tool dispatch:
the real `HTTP::uri` key must be found, while seven padded, control-byte or
missing names must remain byte-for-byte unchanged and not found.

[Control source](validation-01/control-source.rs.txt) records the exact
assertions. This is a catalogue API control, not a Tcl invocation experiment
or a grant of native command identity.

[First validation](validation-01/validation.json) did not reach the control.
The offline attempt stopped on an uncached `aead` dependency. The subsequent
selected-target dependency download/build failed with five compile errors
before any test ran:

- An existing test uses removed `model::SpecSurface::TCL90_PLUS`.
- `irule_gen.rs` calls missing `cell_written` at two sites.
- The analysis response calls `detect_events` and `event_order_list` without
  their required retained analysis argument.

[Diagnostics](validation-01/diagnostics.json) and the original stdout/stderr
retain the exact locations and compiler messages. The captured 6,244-file
source/manifests/configuration scope did not change during compilation.
It is conservative and includes generated/vendor files; it is not an
assertion that every file was compiled. Cargo emitted 360 artifact records
for dependency targets, including 38 workspace packages; the selected MCP
test executable was not produced. The observed artifact package IDs and
target source hashes remain separate from the conservative 39-package
manifest path closure. VM belongs to that closure; standalone Runtime does
not. No passing test or oracle outcome follows from this failed build.

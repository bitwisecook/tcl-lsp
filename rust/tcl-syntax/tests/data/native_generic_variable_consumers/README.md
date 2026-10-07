# Generic variable operands

The 50 fixed source programs exercise original variable operands of generic
`lassign`, `scan`, `binary scan`, `string is -failindex`, `info default`, `lset`,
`append`, `lappend`, empty `lappend`, and `catch` output assignment. A variable
command head selects each generic command invocation. The original name object
is also used for the post-operation existence check and read.

Every program has separate stdout and stderr captures from C Tcl 8.4.20,
8.5.19, 8.6.18, 9.0.4, 9.1.0, and Jim Tcl. `manifest.json` records the exact
source, launcher, real executable, version control, output hashes and process
exit status. Native absence and argument rejection remain captured outcomes:
C Tcl 8.4 lacks `lassign`, and Jim rejects `string is -failindex`.

All source files contain ASCII bytes and LF line endings. The decomposed name
is `e` followed by U+0301, produced by the literal Tcl escape `\u0301`; it is
not precomposed U+00E9. The binary name comes from `binary format H* ff0078`
and contains bytes FF, 00, 78 before any native string getter. Namespace and
array cases use `::N::v` and `arr(k::part)` respectively. Output uses the
native `binary scan ... H*` conversion to produce ASCII hexadecimal fields.

These script captures prove observable command and original-name read
behavior. They do not inspect object headers, compiler opcode provenance or
BIG-IP behavior. Those claims require their respective native probes.

`probe.c` supplies the separate original argv/header controls. Its 50 cases
invoke real generic command handlers directly with the original counted string
name objects. It records each original object's type, reference count and
resident string bytes before and after the invocation, then the native result
getter's bytes. The FF, 00, 78 case is an original string constructor here,
independent of the script suite's binary producer.

`header-manifest.json` binds all 300 header windows to the source, actual linked
engine version, compiler executable/version/target, consumed headers, static
engine library, compiled probe executable and raw process captures. Command
absence and unsupported options leave their real original name headers intact.
These captures do not assert Rust backend equivalence by themselves.

The 11 `trace-*.tcl` programs have 66 separate native transcripts. They cover
quiet missing reads, read-callback errors, receiver recreation during `lappend`
and `lset`, rebinding a later `scan` destination in an earlier destination's
write callback, and qualified array trace reports. Both legacy and modern trace
syntax are captured: Tcl 9 rejects legacy variable trace syntax, while current
Jim has no `trace` command. Modern syntax reaches the actual variable callbacks
in all five C releases. Quiet read-callback error presentation is release
specific: C 8.4 publishes `NONE`, C 8.5 preserves `SENTINEL`, and C 8.6–9.1
publishes `TCL READ VARNAME`. A missing quiet read preserves `SENTINEL` in all
six engines.

Both backend test modules independently compare the script results, original
argv/header windows and callback/receiver controls, in fixed per-engine tests.
Header comparisons inspect original physical headers before the operation and
before any reporting getter afterward; result bytes use each backend's selected
native getter. Test captures do not donate engine or compiler authority.

`part2-purpose.c` calls the original separate root/index `Tcl_ObjSetVar2`
interface with quiet and error-reporting write flags. All five C releases
preserve `SENTINEL` when the quiet lookup encounters a missing parent namespace;
the error-reporting lookup publishes its actual array diagnostic.
`part2-purpose-manifest.json` records the source, compiler, consumed headers,
linked engine library, compiled executable, actual engine version and raw
outputs. The Runtime separate-operand control compares the quiet lookup failure
to these native captures without performing a store after failed selection.

The four sources in `active-unset/` have 20 original C transcripts. They cover
scalar and element unset callbacks during an active read trace, retirement of
an older pending read registration, and suppression of recursive ordinary
reads and writes. All five C releases deliver `read` followed by `unset`,
retire the destroyed registrations, and keep the appended value in the original
live cell. The recursive control delivers only the outer `read` and `write`.
The manifest records source, launcher, real executable, actual version, raw
outputs and timeout. Both ports compare all four sources separately for each
C release. These controls make no Jim trace claim.

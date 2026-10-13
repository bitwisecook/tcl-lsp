# Selected Studio schema and generated reference controls

The selected `schema_coverage` and `reference_doc` integration targets compiled
successfully. SubCommand schema coverage and normal generated-document equality
each passed once. One separate official generator invocation wrote the document;
its write-and-return branch supplies no equality assertion. The exclusive issues
cache was released after completion. No wider Studio command sweep ran.

It compiles only those two test targets, observes their native exact names,
checks SubCommand schema coverage once, runs the actual reference generator
once with `UPDATE_REFERENCE=1`, then checks the generated document once with
that variable unset. The generator writes only
`docs/references/command-spec/fields.md`; its write-and-return branch is separate
from the ordinary assertion that the file matches the current renderer.

The current field is `source_configuration_reference`, optional and subcommand
only, with `DataGroupComparison` vocabulary. Its source grammar identifies the
data-group operand in `class match -- $item equals /Common/allowed` after the
comparison item and operator. A computed item without a proved option boundary
retains conditional selection. The metadata grants no native handler, object
existence, accepted argv or successful comparison.

[execution.json](execution.json) retains exact argv, timestamps, observed native
names, raw stream paths and unchanged native binary hashes.
[summary.json](summary.json) distinguishes the two verification controls from
the separate generator write. The actual Cargo artifact graph is compressed in
[actual-artifact-graph.json.gz](actual-artifact-graph.json.gz): 25 local packages
were emitted; only the Studio library and two integration targets rebuilt.

The retained joint-build inventory supplies 11,803 conservative before/after
source/build hashes, with no changes. This is a superset, not a claim every file
compiled. The existing nine Studio owner source images were verified and
[referenced](reused-studio-owner-images.json) rather than copied as a new whole
dependency snapshot. Only the two integration test sources and generator source
receive additional images here.

The official generator wrote only
[fields.md](../../../../../docs/references/command-spec/fields.md).
The exact [before image](fields.md.before.gz), [generated image](fields.md.after.gz)
and [complete diff](generated-fields.patch) are retained. The diff has 159 added
lines, 24 removed lines and 19 hunks, including existing schema/help/catalogue
drift beyond the new configuration-reference field. No selective or manual
output edits occurred. The ordinary reference check ran with `UPDATE_REFERENCE`
unset and confirmed the document matches the current renderer.

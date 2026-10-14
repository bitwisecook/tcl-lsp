# Explicit callback availability inputs

Callback source signature selection depends on the supplied Registry availability
context. The Tcl context uses open-world package advice: unplaced Tcllib libraries
remain available to source schemas. A `package require` statement retains source
intent, without proving a successful runtime loader. The text
`# tcl-lsp: requires` has no implemented directive meaning.

The fixture uses identical C Tcl 9.0 source grammar and the same command-store
Arc for two independently supplied availability contexts. The Tcl 9.0 context
admits the four package descriptors. The iApps context's AmbientPlusRequire
policy excludes these unrequired packages. Direct resolve_spec assertions check
both answers before analysing any callback; each excluded result also checks
all four descriptor refusals against its actual sealed input after source analysis. The iApps context is used only as
actual availability metadata. The supplied input explicitly retains its actual
AuthoredContext IAppImplementation vendor source policy, matching the analyser's
sealed editing input. This is not an F5 interpreter or BIG-IP result.

Every excluded analysis asserts its exact retained input, complete source/config
match, retained realm/input match and genuine original cb declaration. A core
lsort callback prepended to the same source under that excluded input must issue
a typed E003 source subject through the actual project callback API. These
controls prevent whole-model rejection from satisfying the package refusal.

All eleven original source probes and the no-E002/no-E003 assertion are retained
under the explicit excluded input. The same four rooted SMTP/MIME/graph/tree
probes require E003 under admitted input. The actual supplied-analysis project
callback API consumes each result with an empty project, so positive local
signatures cannot borrow an unrelated project header. Missing input, foreign
availability, foreign command store and changed source controls retain their
refusals. No production callback or availability rule changes.

The original ordinary observation is retained in
[db-production186-20261014](../db-production186-20261014/): test-19.log.gz records
the SMTP E003 under the original open-world fixture. Original fixture bytes and
source probes are retained here. Compiler and database acceptance of the explicit
input fixture remains a separate executed check.

The ordinary193 fixture observation retains identical C9 profiles/configuration
and identical availability metadata but reports one input difference: the analyser
seals AuthoredContext IAppImplementation as vendor_source_policy, while an
implicit supplied None omits it. `db-production193-20261014/test-19.log.gz` retains
that whole-input assertion failure. The explicit supplied policy keeps the full
input equality assertion and the independent source/realm/core callback controls.

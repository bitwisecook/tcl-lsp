Original source value consumers
===============================

The implementation patch moves analyser interpolation, command-substitution
fold arguments, regex variable values and dynamic interpreter-path bindings
to purpose-specific original-source value owners. Syntax's original-word
Unicode callback preserves the complete executable arena and component.
ConstSubst's additive original-source entry preserves those spans through
nested calls without changing existing Native/SSA callback signatures.

The analyser's constant selector authenticates the declaration, complete
source image, exact editing input and dominating value. Interpreter paths use
a separate private provenance table carrying declaration and producer spans,
full source/input and the recorded path key. Snapshots preserve both tables.
The alias hook transports the original operand tokens instead of re-parsing a
name string. Array values and linked cells without independent value and
binding-continuity evidence remain unavailable; their names cannot supply it.

Regex pattern layout in positively retained Logical source comes from the
shared original Registry schema and maps the selected effective ordinal back
to its actual written word. Native handler layout remains an independent
existing selector. Source-only RHS folding cannot infer an entered object
frame from a class declaration label.

DictWith keys are retained as readonly inferred names, outside VarDef tables.
An exact original data-key span is retained only when the checked value equals
the original literal content and the list key needs no collapse. Duplicate key
positions do not supply a unique span. These data extents grant no variable
rename permission. The separate Core candidate patch exposes those names to
completion through checked Logical scope advice; definition/edit consumers
continue to require actual declaration provenance.

Both patches pass Main apply checks, rustfmt and whitespace checks. No Rust
build or selector has executed. Five new production controls and the updated
DictWith metadata control are in the patch. The metadata test intentionally
asserts no fabricated variable declaration while retaining the original scope
and value assertions. All original Core9 assertions remain untouched.

Original oracle measurements
----------------------------

The six exact binaries in oracles/results.json executed the retained probe.
Executable and version-matching C source-library inventories are unchanged
before and after every invocation. Jim's binary and checkout Tcl sources are
sealed. No Tcltest helper is used. These inventories do not claim every OS
shared dependency.

All C8.4–9.1 and latest Jim return WRONG:RIGHT:SCALAR for interpolation using
$n, ${$n} and ${scalar(open}; all return WRONG RIGHT for the nested list fold
and regexp match1 for the procedure-local pattern. Dictionary keys execute and
remain readable on C8.5–9.1 and Jim; C8.4 has no dict command. C interpreters
create and evaluate the named child. Jim records the actual unsupported-form
error and is not assigned a C interpreter-path result.

Remaining work
--------------

Run strict Compiler/Core typecheck and the unchanged source-role controls plus
new actual consumer tests against a fresh sealed closure. The distinct
SourceTransition/SourcePrefix producer currently blocks uplevel and alias-prefix
source roles; compiler_finish owns that repair. Do not weaken inert, shadow,
stale image/config/context or Native execution guards to substitute for it.

The zero-production-caller resolve_expansion_count helper and its six private
implementation tests remain for final cleanup. No production const or path
lookup caller remains on an ancestor-only map after these migrations, apart
from that dead helper. Existing public low-level map APIs are used by their
private tracker tests and should be cleaned after consumer acceptance.

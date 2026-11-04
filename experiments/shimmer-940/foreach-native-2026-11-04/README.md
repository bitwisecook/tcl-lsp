# Native foreach representation observations

The 80 fresh-process rows retain the exact scripts, launch arguments, output and
interpreter versions for Tcl 8.4–9.1 and Jim `82fb4d0`. All processes returned
zero with empty standard error. Tcl 8.4 has no dictionary/bignum rows here.
Only Tcl 8.6, 9.0 and 9.1 expose the representation observer used by these
scripts; no internal-type conclusion is drawn for the older releases or Jim.

On the observed releases, the original braced font values change from a pure
string to a list, and a list constructor remains a list. Tcl 8.6 converts
integers, dynamic string lengths, doubles and bignums to lists in both compiled
foreach and the dynamically selected handler. Tcl 9.0 and 9.1 preserve those
numeric representations on both routes. Dictionaries become lists on all three
observed releases and both routes. Each recorded before/after pair has the same
object address; addresses are process-local observations.

The original absolute `/tmp` invocation paths remain in the raw captures. To
repeat a row, invoke its recorded interpreter with the matching bundled script
from `original-inputs/` or `routes/`. The manifest records file hashes, launch
conditions and the limits of the binary-hash capture.

These are native reference outcomes. They establish neither compiler coverage
nor the physical conversion of arbitrary objects, and cannot grant Native,
Normal, body-entry or rewrite authority. #940 remains open.

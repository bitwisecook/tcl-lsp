# Fixture for issue #1302 (see issue1302ImportShadow.test.ts).
#
# The unforced glob import below is REFUSED by real Tcl, because `::` already
# holds `set`. So the `set` call reaches the builtin and is NOT a reference to
# ::Shadow::set, while the `mything` call — a name no fresh interpreter holds
# — really is bound by the same import and IS a reference to ::Shadow::mything.
#
#   0-based lines, as the test addresses them:
#   11: namespace import ::Shadow::*
#   12: set counter 1
#   13: mything
namespace import ::Shadow::*
set counter 1
mything

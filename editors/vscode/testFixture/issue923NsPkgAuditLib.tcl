# Issue #923 differential audit, findings idx 65 / 75: the *declaring* half of
# a namespace whose consumer lives in a sibling document. Neither file
# references the other; tclsh 9.0.4 and 8.6.16 both populate one real
# `::i923audit::colors` namespace when both are sourced.
namespace eval i923audit::colors {
    variable i923auditPalette {red green blue}
}

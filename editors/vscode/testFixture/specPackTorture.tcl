# Consumer for the SpecTcl pack-torture suite
# (src/test/specPackTorture.test.ts).
#
# `::torturepack::apply` is declared only by the scratch pack the suite writes
# into `specPackTortureScratch/`, so what the server knows about this document
# is entirely a function of whether that pack loaded. The name is namespaced on
# purpose: the analyser skips the unknown-command check for any name containing
# `::`, so W123 is not the signal here — hover, signature help and the argument
# roles are.
package require torturelib 1.0

set input 3
::torturepack::apply output {
    set doubled [expr {$input * 2}]
    lappend output $doubled
}
puts $output

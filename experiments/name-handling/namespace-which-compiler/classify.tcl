set input [open [lindex $argv 0] r]
set words [read $input]
close $input
if {[lindex $words 0] ne "IDENTITY"} {error "missing identity"}
puts {label catch-code resolveCmd invokeStk invokeReplace}
foreach {marker label source code listing} [lrange $words 3 end] {
    if {$marker ne "OBSERVATION"} {error "invalid observation marker"}
    set resolve [regexp -line {^[[:space:]]*\([0-9]+\)[[:space:]]+resolveCmd([[:space:]]|$)} $listing]
    set invoke [regexp -line {^[[:space:]]*\([0-9]+\)[[:space:]]+invokeStk[14]([[:space:]]|$)} $listing]
    set replace [regexp -line {^[[:space:]]*\([0-9]+\)[[:space:]]+invokeReplace([[:space:]]|$)} $listing]
    puts [list $label $code $resolve $invoke $replace]
}

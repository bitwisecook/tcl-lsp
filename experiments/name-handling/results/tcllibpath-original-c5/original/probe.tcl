fconfigure stdout -encoding utf-8 -translation lf
proc hex {value} {
    binary scan [encoding convertto utf-8 $value] H* bytes
    return $bytes
}
puts "PROVIDER\t[info patchlevel]\t[hex [info library]]\t[hex [encoding system]]"
if {![string equal [info patchlevel] $env(EXPECTED_PATCHLEVEL)]} {error "wrong original provider"}
if {![string equal [info library] $env(EXPECTED_LIBRARY)]} {error "wrong original library"}
set directory $env(INPUT_PATH)
set original $env(INPUT_SUFFIX)
set suffix_count [catch {llength $original} original_count]
puts "ORIGINAL\t$suffix_count\t[hex $original_count]\t[hex $original]"
set legacy "$directory $original"
set legacy_status [catch {llength $legacy} legacy_count]
if {$legacy_status} {
    puts "LEGACY\t$legacy_status\t[hex $legacy_count]"
} else {
    puts "LEGACY\t$legacy_status\t$legacy_count\t[hex [lindex $legacy 0]]"
}
if {$suffix_count} {exit 0}
set constructed [list $directory]
if {$env(HAS_INPUT_SUFFIX)} {append constructed " " $original}
if {![string equal [lindex $constructed 0] $directory]} {error "directory identity changed"}
if {![file isfile [file join [lindex $constructed 0] sentinel]]} {error "actual directory unavailable"}
if {[llength $constructed] != [expr {$original_count + 1}]} {error "list cardinality changed"}
for {set i 0} {$i < $original_count} {incr i} {
    if {![string equal [lindex $constructed [expr {$i + 1}]] [lindex $original $i]]} {error "original suffix meaning changed"}
}
puts "CONSTRUCTED\t[llength $constructed]\t[hex $constructed]\t[hex [lindex $constructed 0]]"
for {set i 0} {$i < $original_count} {incr i} {puts "SUFFIX\t$i\t[hex [lindex $original $i]]"}
puts "PASS"

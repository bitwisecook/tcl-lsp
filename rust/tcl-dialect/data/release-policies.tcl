puts "patchlevel\t[info patchlevel]"
puts "binary-number\t[expr {![catch {expr {0b10}}]}]"
puts "decimal-prefix\t[expr {![catch {expr {0d10}}]}]"
puts "escape-hex\t[subst {\x4142}]"
puts "escape-wide\t[scan [subst {\U0001F600}] %c]"
puts "escape-octal-length\t[string length [subst {\400}]]"
puts "characters\t[string length [encoding convertfrom utf-8 [binary format H* f09f9880]]]"
puts "float-default\t[expr {1.0/3}]"
set ::tcl_precision 6
set ::release_fact_double 1.0
puts "float-precision\t[expr {$::release_fact_double/3}]"
puts "expr-comments\t[expr {![catch {expr {1 # comment}}]}]"
set ::release_fact_global 1
puts "namespace-fallback\t[expr {![catch {namespace eval ::release_fact_namespace {set release_fact_global}}]}]"
puts "regex-z-anchor\t[expr {![catch {regexp {a\z} a}]}]"
proc ::release_fact_frame {} {
    set marker present
    puts "uplevel-plus\t[expr {![catch {uplevel +0 {set marker}}]}]"
    catch {uplevel -1 {}} result
    puts "uplevel-negative\t[string match {bad level*} $result]"
    catch {upvar b c d} result
    puts "upvar-parity\t[string match {bad level*} $result]"
}
::release_fact_frame

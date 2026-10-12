set body [binary format H* 65787072207b312b327d]
proc p x $body
puts "CASE 0"
if {[catch {tcl::unsupported::disassemble proc p} assembly]} {puts "NO_DISASSEMBLER:$assembly"} else {puts $assembly}
set c [catch {p 3} result]
binary scan $result H* hex
puts "RESULT $c|$hex"
rename p {}
set body [binary format H* 65787072207b24782b28312b32297d]
proc p x $body
puts "CASE 1"
if {[catch {tcl::unsupported::disassemble proc p} assembly]} {puts "NO_DISASSEMBLER:$assembly"} else {puts $assembly}
set c [catch {p 3} result]
binary scan $result H* hex
puts "RESULT $c|$hex"
rename p {}
set body [binary format H* 65787072207b307831302b3031307d]
proc p x $body
puts "CASE 2"
if {[catch {tcl::unsupported::disassemble proc p} assembly]} {puts "NO_DISASSEMBLER:$assembly"} else {puts $assembly}
set c [catch {p 3} result]
binary scan $result H* hex
puts "RESULT $c|$hex"
rename p {}
set body [binary format H* 65787072207b66616c7365207c7c2028312b32297d]
proc p x $body
puts "CASE 3"
if {[catch {tcl::unsupported::disassemble proc p} assembly]} {puts "NO_DISASSEMBLER:$assembly"} else {puts $assembly}
set c [catch {p 3} result]
binary scan $result H* hex
puts "RESULT $c|$hex"
rename p {}
set body [binary format H* 65787072207b2478203f2028312b3229203a2028312f30297d]
proc p x $body
puts "CASE 4"
if {[catch {tcl::unsupported::disassemble proc p} assembly]} {puts "NO_DISASSEMBLER:$assembly"} else {puts $assembly}
set c [catch {p 3} result]
binary scan $result H* hex
puts "RESULT $c|$hex"
rename p {}
set body [binary format H* 65787072207b2478203f2028312f3029203a2028312b32297d]
proc p x $body
puts "CASE 5"
if {[catch {tcl::unsupported::disassemble proc p} assembly]} {puts "NO_DISASSEMBLER:$assembly"} else {puts $assembly}
set c [catch {p 3} result]
binary scan $result H* hex
puts "RESULT $c|$hex"
rename p {}
set body [binary format H* 65787072207b616273282d34297d]
proc p x $body
puts "CASE 6"
if {[catch {tcl::unsupported::disassemble proc p} assembly]} {puts "NO_DISASSEMBLER:$assembly"} else {puts $assembly}
set c [catch {p 3} result]
binary scan $result H* hex
puts "RESULT $c|$hex"
rename p {}
set body [binary format H* 6361746368207b65787072207b312f307d7d20726573756c743b2072657475726e2024726573756c74]
proc p x $body
puts "CASE 7"
if {[catch {tcl::unsupported::disassemble proc p} assembly]} {puts "NO_DISASSEMBLER:$assembly"} else {puts $assembly}
set c [catch {p 3} result]
binary scan $result H* hex
puts "RESULT $c|$hex"
rename p {}

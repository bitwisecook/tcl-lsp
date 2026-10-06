set body [binary format H* 69662030207b736574206c65616b20227d20656c7365207b72657475726e20454c53457d]
proc p x $body
puts "CASE 0"
if {[catch {tcl::unsupported::disassemble proc p} assembly]} {puts "NO_DISASSEMBLER:$assembly"} else {puts $assembly}
set c [catch {p 0} result]
binary scan $result H* hex
puts "RESULT $c|$hex"
rename p {}
set body [binary format H* 69662031207b72657475726e205448454e7d20656c73656966207b5b6572726f72204d41534b45445d7d207b736574206c65616b20227d20656c7365207b72657475726e20454c53457d]
proc p x $body
puts "CASE 1"
if {[catch {tcl::unsupported::disassemble proc p} assembly]} {puts "NO_DISASSEMBLER:$assembly"} else {puts $assembly}
set c [catch {p 0} result]
binary scan $result H* hex
puts "RESULT $c|$hex"
rename p {}
set body [binary format H* 6966207b24787d207b72657475726e205448454e7d20656c7365207b72657475726e20454c53457d]
proc p x $body
puts "CASE 2"
if {[catch {tcl::unsupported::disassemble proc p} assembly]} {puts "NO_DISASSEMBLER:$assembly"} else {puts $assembly}
set c [catch {p 0} result]
binary scan $result H* hex
puts "RESULT $c|$hex"
rename p {}
set body [binary format H* 69662030207b736574206120417d20656c736569662030207b736574206220427d20656c7365207b736574206320437d3b20696e666f206c6f63616c73]
proc p x $body
puts "CASE 3"
if {[catch {tcl::unsupported::disassemble proc p} assembly]} {puts "NO_DISASSEMBLER:$assembly"} else {puts $assembly}
set c [catch {p 0} result]
binary scan $result H* hex
puts "RESULT $c|$hex"
rename p {}
set body [binary format H* 7768696c652030207b736574206c65616b20227d3b2072657475726e204146544552]
proc p x $body
puts "CASE 4"
if {[catch {tcl::unsupported::disassemble proc p} assembly]} {puts "NO_DISASSEMBLER:$assembly"} else {puts $assembly}
set c [catch {p 0} result]
binary scan $result H* hex
puts "RESULT $c|$hex"
rename p {}
set body [binary format H* 7768696c65207b24787d207b627265616b7d3b2072657475726e204146544552]
proc p x $body
puts "CASE 5"
if {[catch {tcl::unsupported::disassemble proc p} assembly]} {puts "NO_DISASSEMBLER:$assembly"} else {puts $assembly}
set c [catch {p 0} result]
binary scan $result H* hex
puts "RESULT $c|$hex"
rename p {}
set body [binary format H* 7768696c652031207b627265616b7d3b2072657475726e204146544552]
proc p x $body
puts "CASE 6"
if {[catch {tcl::unsupported::disassemble proc p} assembly]} {puts "NO_DISASSEMBLER:$assembly"} else {puts $assembly}
set c [catch {p 0} result]
binary scan $result H* hex
puts "RESULT $c|$hex"
rename p {}
set body [binary format H* 666f72207b736574206920307d207b2469203c20317d207b696e637220697d207b73657420626f647920424f44597d3b206c6973742024692024626f6479]
proc p x $body
puts "CASE 7"
if {[catch {tcl::unsupported::disassemble proc p} assembly]} {puts "NO_DISASSEMBLER:$assembly"} else {puts $assembly}
set c [catch {p 0} result]
binary scan $result H* hex
puts "RESULT $c|$hex"
rename p {}
set body [binary format H* 666f72207b7d207b317d207b627265616b7d207b73657420626f647920424f44597d3b2072657475726e204146544552]
proc p x $body
puts "CASE 8"
if {[catch {tcl::unsupported::disassemble proc p} assembly]} {puts "NO_DISASSEMBLER:$assembly"} else {puts $assembly}
set c [catch {p 0} result]
binary scan $result H* hex
puts "RESULT $c|$hex"
rename p {}
set body [binary format H* 6361746368207b6572726f72204348494c447d20726573756c74206f7074696f6e733b206c6973742024726573756c74205b646963742067657420246f7074696f6e73202d636f64655d]
proc p x $body
puts "CASE 9"
if {[catch {tcl::unsupported::disassemble proc p} assembly]} {puts "NO_DISASSEMBLER:$assembly"} else {puts $assembly}
set c [catch {p 0} result]
binary scan $result H* hex
puts "RESULT $c|$hex"
rename p {}
set body [binary format H* 636174636820247820726573756c74206f7074696f6e733b206c6973742024726573756c74205b646963742067657420246f7074696f6e73202d636f64655d]
proc p x $body
puts "CASE 10"
if {[catch {tcl::unsupported::disassemble proc p} assembly]} {puts "NO_DISASSEMBLER:$assembly"} else {puts $assembly}
set c [catch {p 0} result]
binary scan $result H* hex
puts "RESULT $c|$hex"
rename p {}
set body [binary format H* 6361746368207b72657475726e202d6c6576656c2030202d636f6465206572726f72204241447d20726573756c743b2072657475726e2024726573756c74]
proc p x $body
puts "CASE 11"
if {[catch {tcl::unsupported::disassemble proc p} assembly]} {puts "NO_DISASSEMBLER:$assembly"} else {puts $assembly}
set c [catch {p 0} result]
binary scan $result H* hex
puts "RESULT $c|$hex"
rename p {}
set body [binary format H* 666f72656163682069207b4120427d207b736574206c6173742024697d3b2072657475726e20246c617374]
proc p x $body
puts "CASE 12"
if {[catch {tcl::unsupported::disassemble proc p} assembly]} {puts "NO_DISASSEMBLER:$assembly"} else {puts $assembly}
set c [catch {p 0} result]
binary scan $result H* hex
puts "RESULT $c|$hex"
rename p {}
set body [binary format H* 747279207b6572726f72204348494c447d206f6e206572726f72207b726573756c74206f7074696f6e737d207b72657475726e2024726573756c747d]
proc p x $body
puts "CASE 13"
if {[catch {tcl::unsupported::disassemble proc p} assembly]} {puts "NO_DISASSEMBLER:$assembly"} else {puts $assembly}
set c [catch {p 0} result]
binary scan $result H* hex
puts "RESULT $c|$hex"
rename p {}
set body [binary format H* 747279207b72657475726e2056414c55457d2066696e616c6c79207b7365742066696e616c2046494e414c7d]
proc p x $body
puts "CASE 14"
if {[catch {tcl::unsupported::disassemble proc p} assembly]} {puts "NO_DISASSEMBLER:$assembly"} else {puts $assembly}
set c [catch {p 0} result]
binary scan $result H* hex
puts "RESULT $c|$hex"
rename p {}

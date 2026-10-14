set missing [catch {proc ::missing::p {argument} {set ::written VALUE}} message]
puts [list missing $missing [llength [info commands ::missing::p]]]
namespace eval ::present {}
set present [catch {proc ::present::p {argument} {set ::written VALUE}} message]
puts [list created $present [llength [info commands ::present::p]]]

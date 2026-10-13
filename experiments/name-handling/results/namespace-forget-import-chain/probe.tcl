proc field {name value} {
    binary scan $value H* encoded
    puts "FIELD $name $encoded"
}
set setup_code [catch {
    namespace eval UA {proc g {} {}; namespace export g}
    namespace eval UB {namespace import ::UA::g; namespace export g}
    namespace eval UC {namespace import ::UB::g; namespace export g}
    namespace eval UD {namespace import ::UC::g}
    namespace eval IA {proc g {} {}; namespace export g}
    namespace eval IB {namespace import ::IA::g; namespace export g}
    namespace eval IC {namespace import ::IB::g; namespace export g}
    namespace eval ID {namespace import ::IC::g}
    namespace eval MA {proc g {} {}; namespace export g}
    namespace eval MB {namespace import ::MA::g; namespace export g}
    namespace eval MC {namespace import ::MB::g; namespace export g}
    namespace eval MD {namespace import ::MC::g}
    namespace eval TA {proc g {} {}; namespace export *}
    namespace eval TB {namespace import ::TA::g}
    rename ::TB::g ::TA::h
    namespace eval TD {namespace import ::TA::h}
    namespace eval OA {proc g {} {}; namespace export g}
    namespace eval OD {namespace import ::OA::g}
    rename ::OD::g ::OD::renamed
} setup_result]
field setup_code $setup_code
field setup_result $setup_result
if {$setup_code} {exit 0}
set ultimate_code [catch {
    namespace eval UD {namespace forget ::UA::g}
    list [llength [info commands ::UD::g]] [llength [info commands ::UC::g]] [llength [info commands ::UA::g]]
} ultimate_result]
field ultimate_code $ultimate_code
field ultimate_result $ultimate_result
set immediate_code [catch {
    namespace eval ID {namespace forget ::IC::g}
    list [llength [info commands ::ID::g]] [llength [info commands ::IC::g]] [llength [info commands ::IA::g]]
} immediate_result]
field immediate_code $immediate_code
field immediate_result $immediate_result
set middle_code [catch {
    namespace eval MD {namespace forget ::MB::g}
    list [llength [info commands ::MD::g]] [llength [info commands ::MB::g]] [llength [info commands ::MA::g]]
} middle_result]
field middle_code $middle_code
field middle_result $middle_result
set tail_precedence_code [catch {
    namespace eval TD {namespace forget ::TA::h}
    list [llength [info commands ::TD::h]] [namespace origin ::TD::h] [llength [info commands ::TA::h]]
} tail_precedence_result]
field tail_precedence_code $tail_precedence_code
field tail_precedence_result $tail_precedence_result
set own_name_code [catch {
    namespace eval OD {namespace forget renamed}
    list [llength [info commands ::OD::renamed]] [llength [info commands ::OA::g]]
} own_name_result]
field own_name_code $own_name_code
field own_name_result $own_name_result

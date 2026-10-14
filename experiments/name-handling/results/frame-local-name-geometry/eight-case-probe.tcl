proc run_case {label formals body argv} {
    set createCode [catch [list proc observed $formals $body] result]
    set invokeCode unavailable
    if {$createCode == 0} {
        set invokeCode [catch [linsert $argv 0 observed] result]
    }
    puts [list $label $createCode $invokeCode $result]
    catch {rename observed {}}
}
run_case empty_bound [list {}] {return ${}} [list VALUE]
run_case empty_unbound {} {return ${}} {}
run_case empty_written {} {set {} VALUE; return ${}} {}
run_case dollar_bound [list {$name}] {return ${$name}} [list VALUE]
run_case dollar_not_bound [list name] {return ${$name}} [list VALUE]
run_case unbraced_name_not_bound [list {$name}] {return $name} [list VALUE]
run_case qualified_unbound {} {return ${inner::value}} {}
run_case qualified_written {} {set {inner::value} VALUE; return ${inner::value}} {}

puts [list patchlevel [info patchlevel]]
foreach formal {n ::n ns::n {a(b)} args} {
    catch {rename p {}}
    set code [catch [list proc p [list $formal] {return accepted}] result]
    set present [expr {[llength [info commands p]] != 0}]
    puts [list formal $formal create_code $code result $result present $present]
    if {$present} {
        puts [list formal $formal retained_formals [info args p]]
        set invoked [catch {p ARG} value]
        puts [list formal $formal invoke_code $invoked result $value]
        rename p {}
    }
}

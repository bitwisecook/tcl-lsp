namespace eval ::a { namespace eval b { proc p {} {return ab} } ; puts [b::p] ;# CALL p
}

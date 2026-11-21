namespace eval ::p::q {}; namespace eval ::p::r {}; namespace eval ::p::q::s {}
puts [list [namespace exists ::p::q] [namespace exists p::q] [lsort [namespace children ::p]] [namespace children ::p q*] [namespace parent ::p::q::s] [namespace parent ::]]

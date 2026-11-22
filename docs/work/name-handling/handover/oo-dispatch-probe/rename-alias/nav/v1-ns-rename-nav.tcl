namespace eval ::ns { proc p {} {return nsp}; rename p q; puts [q] }

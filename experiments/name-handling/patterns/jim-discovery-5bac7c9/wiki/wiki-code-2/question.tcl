
        namespace eval n {
            variable x namespace
            proc build {} {set x local; namespace code {set x}}
        }
        eval [n::build]
    

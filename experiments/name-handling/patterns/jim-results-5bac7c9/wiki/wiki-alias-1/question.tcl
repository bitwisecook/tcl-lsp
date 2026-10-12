
        namespace eval n {namespace export take; proc take x {list old $x}}
        namespace eval consumer {namespace import ::n::take}
        interp alias {} alias {} n::take fixed
        rename n::take n::saved
        proc n::take x {list replacement $x}
        list [alias] [consumer::take fixed]
    

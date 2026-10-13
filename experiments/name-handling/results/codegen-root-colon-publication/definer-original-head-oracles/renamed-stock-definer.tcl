rename proc stock_proc
stock_proc :audit {} {return ORIGINAL}
set answer [:audit]
puts [list $answer [info commands audit]]

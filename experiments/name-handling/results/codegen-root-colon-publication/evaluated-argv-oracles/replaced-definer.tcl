rename proc stock_proc
stock_proc proc args {return SHADOW}
puts [list [proc :audit {} {return ORIGINAL}] [info commands :audit]]

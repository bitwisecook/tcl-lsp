# 9.0: %d/%x without size modifier truncate to 32 bits everywhere
puts [format %x 0x100000001]
puts [format %d 4294967297]
puts [format %lx 0x100000001]

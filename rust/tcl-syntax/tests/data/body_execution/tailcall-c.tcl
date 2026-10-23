set available [expr {[llength [info commands tailcall]] != 0}]
puts "available $available"
if {$available} {
    proc scheduled_target {} {return TARGET}
    puts "root-empty [catch {tailcall}]"
    puts "root-target [catch {tailcall scheduled_target}]"
    proc inspect_scheduled {} {
        puts "pending [catch {tailcall scheduled_target}]"
        puts "after 1"
        return ORIGINAL
    }
    set code [catch inspect_scheduled result]
    puts "outer $code $result"
    proc inspect_empty {} {
        puts "empty [catch {tailcall}]"
        return EMPTY
    }
    set code [catch inspect_empty result]
    puts "empty-outer $code $result"
}

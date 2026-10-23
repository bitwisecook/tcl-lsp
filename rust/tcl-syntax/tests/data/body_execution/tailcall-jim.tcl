puts "available [expr {[llength [info commands tailcall]] != 0}]"
proc scheduled_target {} {return TARGET}
puts "root-empty [catch {tailcall}]"
puts "root-target [catch {tailcall scheduled_target}]"
proc inspect_scheduled {} {
    puts "empty [catch {tailcall}]"
    puts "missing [catch {tailcall nonexistent_target}]"
    puts "pending [catch -eval {tailcall scheduled_target}]"
    return DONE
}
set code [catch inspect_scheduled result]
puts "outer $code $result"

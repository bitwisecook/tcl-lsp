# TIP 677: const
puts [catch {const PI 3} r]:$r
puts [catch {set PI 4} r]:$r
puts [catch {info constant PI} r]:$r
puts [catch {info consts} r]:$r
puts [catch {unset PI} r]:$r

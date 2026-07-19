oo::class create Ticket956 {
    method fetch {key} { return $key }
    method get {key} { return [my fetch $key] }
    method unused {} { return nothing }
    method close {} { return closed }
}
set obj [Ticket956 new]
puts [$obj get foo]
puts [$obj get bar]
set ch [open /tmp/x w]
$ch close

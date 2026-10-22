oo::class create Factory {
    self method make {} { return [Factory new] }
}
oo::class create SubFactory {
    superclass Factory
}
puts [catch {SubFactory make} e]:$e
puts [Factory make]

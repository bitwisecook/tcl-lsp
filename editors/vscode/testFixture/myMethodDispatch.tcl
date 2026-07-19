oo::class create Bar957 {
    method getOptions {key} { return $key }
    method get {key} { return [my getOptions $key] }
}

# Original object and member name data

Use the existing BIG-IP interpreter and retain its exact version, raw source,
stdout, stderr and completion. Do not install a class package or rewrite names.

First check `info commands ::oo::class`. If absent, record this source family as
unsupported. If present, run `oracles/probe.tcl` unchanged and report whether
braced `$literal` selects a distinct object from `literal`, whether the quoted
scalar receiver selects its actual object, and whether the literal spaced
method remains callable while the explicit class-method mask takes effect.

This does not establish Native compiler or entered method-frame applicability.

# tcl-bigip-schema

BIG-IP configuration kinds, property descriptors, reference edges, shipped
base identities and versioned profile defaults. The data is independent of
the Tcl command registry. `tcl-bigip` exposes it to generic tooling through
its `schema` module. Object snapshots are JSON values; presentation and
canonical escaping belong to the caller's snapshot serializer.

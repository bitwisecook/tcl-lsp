# Original array reads and short-circuit guards

All thirty native observations agree on C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4,
9.1.0 and Jim `0.84-10-g82fb4d0`. An absent same element skips its read; a
conditionally set same element is safe for both branch directions; querying
`a(safe)` does not guard a different `a(missing)` element when the root is live.
Full inputs, catch statuses and interpreter binary hashes are in `native.json`.

A separate small public-API probe links the exact production rlib from the
first read-local attempt. It confirms the no-array-write case has no original
RHS variable access: the existence query is proven false. Adding the possible
`set a(safe) 1` produces an original read at 75..83 with an Element key under
the original activation and a missing SSA version. The owner unit's positive
fixture must therefore make the element MayBound to test a real read-local
guard. No physical/SSA proof or original occurrence is manufactured.

The first red owner receipt is retained unchanged. The corrected MayBound
fixture and additional trace/element controls await their scheduled build;
this boundary evidence makes no new product-fix claim.

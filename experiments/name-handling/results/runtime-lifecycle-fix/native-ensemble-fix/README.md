# Native namespace and ensemble questions

Each directory pairs one question with its exact input/test definition and scoped
results. Raw captures and original receipts remain unchanged.

| Question | Measurement kind |
| --- | --- |
| [Does mutually entangled namespace deletion leave the global namespace intact under the real init.tcl and tcltest.tcl?](questions/entangled-deletion-tcltest/README.md) | Direct Tcl question |
| [Does the global set command survive mutually entangled child deletion?](questions/global-survival/README.md) | Direct Tcl question |
| [Can a retained namespace object still reach a child after its parent is unpublished, while a newly constructed value misses?](questions/cached-versus-fresh/README.md) | Direct Tcl question |
| [Which namespace supplies an implicit ensemble target when global and caller commands shadow its name?](questions/implicit-ensemble-target/README.md) | Direct Tcl question |
| [Does explicit native-core deletion preserve the global namespace across entangled command traces?](questions/core-entangled-deletion/README.md) | Consumer check |
| [Does explicit native-core child teardown distinguish cached namespace objects from fresh lookup?](questions/core-cached-versus-fresh/README.md) | Consumer check |
| [Does the native ensemble consumer use the selected release and actual ensemble namespace?](questions/core-implicit-ensemble-target/README.md) | Consumer check |
| [Does the native consumer retire parent commands before visiting every child namespace?](questions/core-parent-child-order/README.md) | Consumer check |
| [Does the native consumer validate all delete operands before retirement and relookup after callbacks?](questions/core-delete-validation/README.md) | Consumer check |
| [Does a retained namespace token survive retirement without retaining its interpreter?](questions/core-retired-object-lifetime/README.md) | Consumer check |
| [Does the namespace-object consumer match its 100 selected C producer/getter vectors?](questions/core-producer-getter-windows/README.md) | Consumer check |

[Build provenance](build-provenance/README.md) identifies the frozen subject and
retrievable source snapshot separately from the experiments. C controls remain
distinct from Rust consumer checks. Unreached lookup, startup refusal, missing
provider and library-initialization differences stay explicit.

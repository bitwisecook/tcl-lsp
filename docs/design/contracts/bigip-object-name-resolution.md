# Authored BIG-IP object-name resolution

`tcl_bigip::object_names` owns matching of literal slash-domain object keys
for configuration tooling and simulation setup. These policies describe local
software contracts. They do not establish BIG-IP TMM, tmsh, C Tcl or Jim Tcl
lookup semantics. Object paths are not Tcl namespace paths and are not
filesystem paths; repeated slashes, colons and `..` remain literal bytes.

Call `object_name_candidates` with an explicit `ObjectNamePolicy` and a domain
already restricted to the actual configuration, module and object type. The
result retains every match in the highest-precedence tier in input order.
`unique` refuses absent or ambiguous matches.
`first_exact_in_order_or_unique` preserves ordered exact rows and requires a
unique fallback; duplicate exact rows retain order without claiming uniqueness. `first_in_order` explicitly
selects an ordered tooling candidate; that result supplies no unique identity
or appliance execution authority.

| Consumer | Policy | Selection and order |
| --- | --- | --- |
| Lint, typed graph, conversion, packet enrichment, Wireshark, iRule context | `Configuration` | Exact, authored default partition, Common, slash-delimited suffix; first in the supplied object order |
| GTM packet enrichment | `CommonThenSuffix` | Exact, Common-prefixed key, suffix; first source/map candidate |
| Generic graph objects | `ExactOrSuffix` | Exact and suffix together; first candidate after module and object-type filtering |
| TLS attachment | `ExactThenOwnerPartitionOrReferenceLeaf` | Exact, referring virtual server's partition, matching reference leaf; unique at the selected tier |
| TLS inherited object references | `ExactThenReferenceLeaf` | Exact, matching reference leaf; unique |
| TLS supplied certificate/key material | `ExactThenReferenceLeaf` | Exact, matching reference leaf; first map candidate |
| Simulation virtual server | `ExactThenShortNameOrCommon` | Exact, then short name or Common-prefixed path together; first source candidate |
| Simulation rule source | `ExactThenShortName` | Exact first in source order (duplicates retained), then a leaf equal to the complete reference; unique fallback, followed by `RuleIdentity` validation |
| Exact typed configuration joins | `Exact` | Exact keys only; first source candidate |

`owner_partition_reference_paths` supplies the generic registry validator's
bounded exact alternatives: an absolute reference remains itself; a relative
reference uses its owner's partition, or the explicit default when absent,
then Common. This purpose does not search arbitrary partitions. The validator
still applies the registry's allowed target kinds and its independent factory
object inventory.

`reference_without_numeric_service` preserves IPv6 and nonnumeric keys while
removing a single numeric `:port`. Its `EmptyServiceSuffix` policy keeps the
graph's trailing-colon key and the validator's empty-service treatment
independent. Callers retain their actual grammar extraction and address-kind
eligibility before entering this helper.

Configuration joins and source-table matches are readonly candidates. They do
not grant Tcl command resolution, execution, rename safety or complete appliance
coverage. Appliance conformance tests require original measured results for the
specific surface, firmware, partition, event and operands.

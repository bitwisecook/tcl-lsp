# BIG-IP object path resolution questions

Status: unanswered. The local object-name matching policies are authored
software behavior; they are not measured appliance resolution.

Collect answers in this branch under
`experiments/name-handling/f5/answers/object-path-resolution/`. Retain raw
stdout, stderr, exit status, exact fixtures/operands and a manifest of SHA-256
hashes. Record BIG-IP firmware/build, TMM Tcl version, configuration partition,
source iRule full path, event, object type and actual reached operation. Do not
use guessed Tcl behavior or tmsh results as a substitute for a TMM observation.
Record unsupported, unreached and refused operations separately.

Existing read-only inventory and firmware queries can establish the available
fixture domain. Object creation, attachment and traffic probes require the
BIG-IP chat's existing authorisation and an isolated test configuration; do not
modify production objects to answer these questions.

| Question | Required independent observations |
| --- | --- |
| Does a relative `pool` reference choose the iRule's partition, virtual server's partition, Common or another domain? | Distinct existing pools with the same leaf; record selected pool identity and exact event/argv |
| What happens if only another partition contains the leaf? | Capture compile/attach acceptance and reached runtime result independently; do not infer first-match behavior |
| What happens with same-leaf objects in multiple nonlocal partitions? | Exact positive controls and the relative reference, including any ambiguity error and error options |
| Are tmsh property references and TMM dynamic object references resolved alike? | Keep tmsh parse/store/readback and reached TMM operations separate |
| Do nested folders, repeated slash bytes and a literal colon change matching? | Exact path controls plus relative operands; record rejected object declarations independently from lookup failures |
| Is an absolute path ever resolved by another partition's suffix? | Exact available and exact missing path controls, with another partition containing the same leaf |
| How do rule-source and virtual-server attachment references treat ambiguous short names? | Record configured full paths, parser/attach result and reached execution separately |
| Do monitor, node, profile, GTM pool/server and certificate/key references share the same scope? | Measure each available object family independently; unsupported families remain unanswered |
| Are numeric `:port`, empty service suffix and IPv6 interpreted as object-key components or address/service syntax? | Original declarations and exact operands per allowed object family; report grammar rejection before lookup |

Answer documents must distinguish raw observation from interpretation and
identify every unanswered cell. Any new conformance assertion must cite the
actual observed input/result. No expected appliance result is supplied here.

# Shared F5 deployment base

[base.scf](base.scf) contains an owned folder and shared backend pool.
[base.json](base.json) defines the placeholder network, endpoint roles,
ports and lab preconditions. It is a replay scaffold, not a saved historical
appliance configuration. Every question keeps its actual historical source
and configuration hash independently.

| Role | Placeholder | Source of binding |
| --- | --- | --- |
| Self-IP | 192.168.1.11 | Existing lab network; not configured by this SCF |
| Backend | 192.168.1.30 | Shared pool and server binding |
| Client | 192.168.1.33 | Client execution host |
| Virtual server | 192.168.1.100 | Selected test configuration |

Use the materialiser to change the `/24` or host assignments. Its address
listing distinguishes configured fields from declared external roles.
Additional VIPs appear only when the selected configuration needs them.

Historical address projections are keyed by the retained address token's
digest. They allow reconstruction without copying a deployment-address table
into every question. The capture store retains the existing source bytes and
their sanitisation statements; the reconstruction itself is not a measurement.

The base has no test procedures, static cells, seeded namespace variables or
test virtual-server attachment. Group/test materialisation supplies the source
and required profile/event attachments. Preserve Common, folder and
cross-partition relationships where the question depends on them.

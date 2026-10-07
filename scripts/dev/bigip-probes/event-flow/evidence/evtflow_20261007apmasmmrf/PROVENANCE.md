# Evidence provenance

This bundle records the `evtflow_20261007apmasmmrf` run against
`bigip.bitwisecook.org`. Fixture manifests name source commit `0a66990ee` and
contain the exact byte sizes and SHA-256 values checked before load.

## Repository-safe evidence

- `fixtures/` contains the exact generated appliance inputs and manifests.
- `dev/` contains raw client responses, backend observations, the persistent
  lab build inputs and generated probe-only PKI. The private keys under
  `dev/pki/private/` were generated solely for this disposable public test
  fixture.
- `appliance/repository-safe/probe-only.scf` contains only owned
  `/Common/__tcl_lsp_evtflow_*` objects.
- `appliance/repository-safe/ltm-events.log` and `apm-events.log` are filtered
  from the raw appliance logs without rewriting matching lines.
- `appliance/repository-safe/probe-rejections.log` contains exact matching
  loader/runtime rejection lines from `/var/log/ltm`.
- `appliance/help/` contains the on-box tmsh and iRules help consulted for the
  measured configuration.
- `appliance/report-generator-model.json` is the parsed model produced from
  `probe-only.scf`.

The hardware output had its appliance registration credential replaced with
the literal line `Registration credential omitted from repository evidence`.
No other repository-safe appliance metadata was rewritten.

## Protected appliance evidence

The complete SCF, encrypted UCS, qkview, adjacent generated UCS password and
raw continuous logs remain at:

```text
/var/tmp/evtflow_20261007apmasmmrf/
```

The full-system files are mode 600. Their sizes, timestamps and SHA-256 values
are in `appliance/repository-safe/protected-artifact-inventory.txt` and
`protected-artifact-hashes.txt`. They are not committed because full appliance
artifacts contain pre-existing credentials, licence data and other material
outside the generated probe scope.

The appliance master key was not collected. SSH keys, GitHub credentials,
appliance management credentials and pre-existing authentication material are
not part of this bundle.

## Hostname provenance

All new lab configuration, generated certificates, LDAP directory entries,
requests and current appliance metadata use `bitwisecook.org`. Older evidence
from the preceding event-flow run retains its original raw hostname bytes so
that source evidence is not silently altered.

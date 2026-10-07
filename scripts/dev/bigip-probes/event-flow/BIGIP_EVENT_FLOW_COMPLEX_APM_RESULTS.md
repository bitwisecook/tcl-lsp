# BIG-IP complex APM authentication results

## Result

A single retained HTTPS virtual server now protects path families with public,
Samba Active Directory, OpenLDAP, Keycloak authorization-code, and Samba AD
plus appliance-generated OTP branches. The virtual server uses SNAT automap.
Successful requests reached the backend and the backend observed
`192.168.9.24`, the BIG-IP self/VIP address, as its peer.

The requested OpenID Connect mode is **not safe on the tested appliance**.
With a valid Keycloak token response, enabling OpenID processing reproducibly
terminated `apmd` and wrote a core. The retained policy therefore uses the
same Keycloak authorization-code exchange with `openid-connect disabled`.
The exact OIDC-enabling mutation is retained as an opt-in crash probe, not as
the live state.

## Appliance and topology

The measured device was:

- hostname: `bigip.bitwisecook.org`;
- platform: BIG-IP Virtual Edition, platform `Z100`, Q35 virtual hardware;
- product: BIG-IP;
- version: **21.1.0.1**;
- build: **0.0.26**;
- edition/hotfix: **Point Release 1**;
- FIPS module: Cryptographic Module for BIG-IP;
- topology: standalone device `/Common/bigip1`, management IP
  `192.168.9.24`, no configured config-sync or failover peer;
- TMM process: PID `16500`;
- actual TMM roster: `0:0`, `0:1`, `0:2`, and `0:3`.

The complete version output is
[sys-version.txt](evidence/evtflow_20261007apmcomplex/appliance/final/sys-version.txt),
with [platform.txt](evidence/evtflow_20261007apmcomplex/appliance/final/platform.txt),
[cm-device.txt](evidence/evtflow_20261007apmcomplex/appliance/final/cm-device.txt),
and [tmm-info.txt](evidence/evtflow_20261007apmcomplex/appliance/final/tmm-info.txt).

The data path was:

```text
client 192.168.9.80 or isolated client 192.168.9.253
  -> HTTPS VIP 192.168.9.24:18840, SNAT automap
  -> HTTP backend 192.168.9.80:18710

authentication sources on the directly connected lab subnet:
  OpenLDAP       192.168.9.250:1389
  Samba AD DC    192.168.9.251:88/389 and related AD ports
  Keycloak       192.168.9.252:8443
```

Samba AD is retained as privileged Debian LXC VMID `104` because both tested
Podman storage variants failed while applying SYSVOL ACLs. The Keycloak,
OpenLDAP, and curl client containers use the internal
`evtflow-auth-macvlan`. The LXC and client route captures demonstrate that no
default route is present.

## Tested source and byte provenance

The fixture manifest records source commit
`69a23bad2592558b879c9b758bb444a965d911b5`. Sources are ASCII/LF except for
the copied PEM certificate. The final appliance pre-load check accepted every
SHA-256 entry.

| File | SHA-256 |
| --- | --- |
| `keycloak-ca.crt` | `47bbf9eaeb786c1b1b869d07638320656927786462a7e572bd5070079ff98d56` |
| `rule.conf` | `8e2fa9676c7d8349d6902848d57e1fa8ba9fa1ea08fe80ba6913e0fe53145331` |
| `config.conf` | `d314653faec4a52be48a8a0c585596b48c035b1b44c9d098c3b617d338ea8567` |
| `manifest.json` | `14e65bc69c08f73c299d4d0025dfae185c35a5f43ce3dca7adeb4a8295ab5c6a` |

The authoritative machine-readable values are
[SHA256SUMS](evidence/evtflow_20261007apmcomplex/fixtures/EFO/SHA256SUMS) and
[manifest.json](evidence/evtflow_20261007apmcomplex/fixtures/EFO/manifest.json).
The complete evidence tree has its own
[SHA256SUMS](evidence/evtflow_20261007apmcomplex/SHA256SUMS).

## Policy

The access policy selects a branch from `session.server.landinguri`:

| Path | Policy branch | Successful state |
| --- | --- | --- |
| `/public/*` | no credential agent | `public` |
| `/ad/*` | form logon, Samba AD auth | `ad` |
| `/ldap/*` | form logon, OpenLDAP auth | `ldap` |
| `/oidc/*` | Keycloak OAuth authorization code | `oidc` |
| `/mfa/*` | form logon, Samba AD auth, OTP generate, OTP form, OTP verify | `mfa` |

Each successful branch invokes a distinct iRule Event agent. The iRule stores
the resulting scope in `session.custom.evtflow_scope`. Subsequent protected
requests with a different required scope receive a local HTTP 403. This is a
measured path-binding guard around the per-session policy; it is not a claim
that an APM per-request policy was tested.

The exact retained configuration is in
[config.conf](evidence/evtflow_20261007apmcomplex/fixtures/EFO/config.conf),
[rule.conf](evidence/evtflow_20261007apmcomplex/fixtures/EFO/rule.conf), and
the appliance serializations under
[appliance/final](evidence/evtflow_20261007apmcomplex/appliance/final/).

## Results matrix

| Case | Measured result | Backend / TMM evidence |
| --- | --- | --- |
| Public request | APM created a session, selected public, allowed | backend 200; completion `0:0`, server connection `0:1` |
| Valid Samba AD password | `session.ad.last.authresult=1`; allowed | backend 200; completion `0:2`, server connection `0:1` |
| Invalid Samba AD password | AD reported preauthentication failure and policy deny | no backend request; completion `0:1` |
| Valid OpenLDAP password | `session.ldap.last.authresult=1`; allowed | backend 200; completion and server connection `0:3` |
| Existing AD session requests `/mfa/*` | local 403, `required=mfa`, `granted=ad` | `SCOPE_DENY` on `0:1`; no backend request |
| AD password plus generated OTP | AD passed, OTP issued, submitted OTP passed | backend 200; completion `0:2`, server connection `0:3` |
| OTP branch using `session.otp.last.result` | expression did not select success; fallback deny | completion deny on `0:3` |
| OTP branch using `session.otp.verify.last.authresult` | selected success | completion allow and backend 200 |
| OAuth request with bad client secret | Keycloak returned token endpoint HTTP 401; clean policy deny | APM logged `HTTP error 401`; no core |
| Keycloak authorization code, OpenID disabled | APM logged OAuth Client success and access token was non-empty | backend 200; final completion `0:2`, server connection `0:1` |
| Keycloak authorization code, OpenID enabled | token response reached JWT validation, then `apmd` cored | reproduced twice; client observed TLS EOF; no backend request |

The backend observations show `peer=["192.168.9.24", ...]` for public, AD,
LDAP, MFA, and Keycloak requests in
[backend-efo.jsonl](evidence/evtflow_20261007apmcomplex/dev/runtime/backend-efo.jsonl).
This proves the translated backend peer and completed return path rather than
inferring it from virtual-server configuration.

## OIDC failure

With `openid-connect enabled`, the valid token response reached these frames
before `apmd` terminated:

```text
OAuth::updateOpenIDStats
OAuth::updateStats
OAuth::validateJwtToken
OAuthAgent::processJwtToken
OAuthAgent::validateIDToken
OAuthAgent::getAccessToken
```

The failure reproduced twice at 04:15:23 and 04:15:33 appliance time. The
service status subsequently reported four total restarts, and the retained
core is `/var/core/apmd.bld0.0.26.core.gz` (48,867,956 bytes). The core is
kept on the disposable appliance and is deliberately not committed because a
process-memory image can contain unrelated credentials. The stack and core
metadata are preserved in
[apm-efo-filtered.log](evidence/evtflow_20261007apmcomplex/appliance/logs/apm-efo-filtered.log)
and
[apmd-core-location.txt](evidence/evtflow_20261007apmcomplex/appliance/final/apmd-core-location.txt).

Disabling only OpenID processing made the otherwise identical Keycloak
authorization-code exchange succeed without another restart. The retained
[`enable-oidc-crash-probe.sh`](auth-lab/complex/enable-oidc-crash-probe.sh)
contains the exact opt-in mutation and recovery command.

## Configuration rejections and counterexamples

These are literal appliance results, not inferred schema rules:

| Input | Exact result |
| --- | --- |
| OAuth provider with `http://` authorization URI | `Invalid authentication_uri ... The value must start with "https://".` |
| OAuth server without a DNS resolver, despite literal-IP endpoint URIs | `AAA OAuth Server ... must specify DNS resolver.` |
| Entry item containing the route branches | `access policy item ... of entry type must have exactly one rule` |
| Route item's last rule captioned `Public` | `very last rule as fallback (with expression or caption = 'fallback')` |
| One logon-page agent referenced by multiple policy items | `cannot share an agent ... with other access policy item` |
| OTP success checked as `session.otp.last.result` | accepted at load; runtime fallback deny |
| Empty-string expression passed through shell quoting | serialized as `ne `; runtime expression syntax error |

All rejected load outputs are retained under
[appliance](evidence/evtflow_20261007apmcomplex/appliance/), including
`load-initial-rejected`, `load-dns-required-rejected`,
`load-fallback-rejected`, `load-agent-share-rejected`,
`load-start-type-rejected`, and `load-entry-branch-rejected`. The accepted
canonical load is in
[load-final-canonical](evidence/evtflow_20261007apmcomplex/appliance/load-final-canonical/).

## TMM coverage

`RULE_INIT` logged independently on `0:0`, `0:1`, `0:2`, and `0:3`, matching
the four-entry `tmsh show sys tmm-info` roster. Traffic-associated policy and
server events also reached every actual unit:

- `0:0`: public policy completion and Keycloak policy completion;
- `0:1`: AD backend connection and cross-scope denial;
- `0:2`: AD and MFA policy completion, final Keycloak completion;
- `0:3`: LDAP completion/backend and MFA backend connection.

The raw event records are in
[ltm-efo-filtered.log](evidence/evtflow_20261007apmcomplex/appliance/logs/ltm-efo-filtered.log)
and the continuous captures are
[ltm-continuous.log](evidence/evtflow_20261007apmcomplex/appliance/ltm-continuous.log)
and
[apm-continuous.log](evidence/evtflow_20261007apmcomplex/appliance/apm-continuous.log).

## Reproduction

Provision the Samba controller on the Proxmox host and the isolated services
on `dev.bitwisecook.org`:

```sh
scripts/dev/bigip-probes/event-flow/auth-lab/complex/provision-samba-lxc.sh
scripts/dev/bigip-probes/event-flow/auth-lab/complex/run-complex-auth-lab.sh
```

Generate and verify the appliance fixture:

```sh
scripts/dev/bigip-probes/event-flow/generate-complex-apm-fixture.py \
  --run EFO \
  --out /var/tmp/evtflow-efo-complex-fixture \
  --vip 192.168.9.24 --vip-port 18840 \
  --backend 192.168.9.80 --backend-port 18710 \
  --ldap 192.168.9.250 --ldap-port 1389 \
  --ad 192.168.9.251 \
  --oidc 192.168.9.252 --oidc-port 8443 \
  --clientssl /Common/__tcl_lsp_evtflow_efd_clientssl \
  --keycloak-cert keycloak.crt

cd /var/tmp/evtflow-efo-complex-fixture
sha256sum -c SHA256SUMS
```

On BIG-IP, after transferring exact fixture bytes:

```sh
scripts/dev/bigip-probes/event-flow/load-complex-apm-fixture.sh \
  /var/tmp/evtflow-efo-complex-fixture \
  /var/tmp/evtflow-efo-complex-load \
  EFO
```

The isolated client executes the Keycloak browser flow with:

```sh
podman exec evtflow-auth-client /usr/local/bin/oidc-flow.sh
```

The generated fixture identities are documented in
[auth-lab/complex/README.md](auth-lab/complex/README.md). They are disposable
test data and do not authenticate to existing infrastructure.

## Evidence and retained state

The committed evidence root is
[evidence/evtflow_20261007apmcomplex](evidence/evtflow_20261007apmcomplex/).
Raw protocol responses are under
[dev/responses](evidence/evtflow_20261007apmcomplex/dev/responses/), container
and backend state under
[dev/runtime](evidence/evtflow_20261007apmcomplex/dev/runtime/), and LXC/AD
state under
[proxmox](evidence/evtflow_20261007apmcomplex/proxmox/).

Retained intentionally:

- the EFO BIG-IP virtual, pool, rule, APM policy/agents, AAA objects, OAuth
  requests/provider/server, resolver, and generated Keycloak CA object;
- Samba AD LXC VMID `104` at `192.168.9.251`;
- `evtflow-openldap`, `evtflow-keycloak`, and `evtflow-auth-client`;
- the event-flow backend process serving port `18710`;
- the appliance-only `apmd` core named above.

Cleanup completed:

- both continuous log-tail processes were stopped after capture;
- the failed stopped Podman Samba container and its unused volume were
  removed;
- the accidental full-system SCF archive was removed; only the owned-object
  excerpt remains in the repository.

No SSH keys, GitHub credentials, appliance login credentials, licence data,
or appliance master keys are present in the committed evidence. Full UCS,
qkview, process core, and unfiltered full-system SCF artifacts are excluded
because they can contain authentication material not generated by this probe.

## Coverage limits

- The retained path guard is an iRule plus a per-session access policy; an APM
  per-request policy/subroutine implementation was not measured here.
- Keycloak OIDC ID-token validation and UserInfo did not complete because the
  appliance process terminated first. The OAuth fallback proves transport,
  browser redirect, code receipt, server-side token exchange, policy allow,
  SNAT, and backend return only.
- AD group-query branching was not added; the Samba group exists and is
  retained for a follow-up query-agent branch.
- OTP delivery was test-only: the assigned value was logged by an iRule Event
  agent and submitted by the client. No SMS, email, or external OTP delivery
  system was measured.

## Primary references

- [F5 BIG-IP 21.1 OAuth client/resource-server configuration](https://techdocs.f5.com/en-us/bigip-21-1-0/big-ip-access-policy-manager-oauth-configuration/apm-oauth-client-and-resource-server.html)
- [F5 APM OTP session variables](https://techdocs.f5.com/kb/en-us/products/big-ip_apm/manuals/product/apm-config-11-4-0/apm_config_sessionvars.html)
- [F5 application-layer step-up authentication example](https://techdocs.f5.com/en-us/bigip-21-1-0/big-ip-access-policy-manager-per-request-policies/using-step-up-authentication/example-application-layer-step-up-auth.html)

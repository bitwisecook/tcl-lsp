# Complex APM authentication sources

This lab adds two persistent, isolated authentication sources alongside the
existing OpenLDAP fixture:

- Samba AD DC `ad.lab.bitwisecook.org`, `192.168.9.251`, realm
  `LAB.BITWISECOOK.ORG`;
- Keycloak OIDC provider `oidc.lab.bitwisecook.org`, `192.168.9.252`, realm
  `evtflow`.

The Samba controller is a privileged Debian LXC because its SYSVOL needs
filesystem ACLs and extended attributes that the Podman storage driver does
not preserve. Keycloak attaches to `evtflow-auth-macvlan`, which has no
gateway. Both retained services can communicate only with the directly
connected `192.168.9.0/24` lab. The recipes verify the absence of an external
route.

All identities below are generated disposable probe data:

| Source | Identity | Password/secret |
| --- | --- | --- |
| Samba administrator | `Administrator` | `Evtflow-Ad-Admin-2026!` |
| Samba user | `EVTFLOW\\evtflow-ad` | `Evtflow-Ad-User-2026!` |
| Samba group | `evtflow-employees` | n/a |
| Keycloak administrator | `evtflow-admin` | `Evtflow-Oidc-Admin-2026!` |
| Keycloak user | `evtflow-oidc` | `Evtflow-Oidc-User-2026!` |
| OIDC client | `evtflow-apm` | `evtflow-oidc-client-secret-2026` |

Deploy Samba on the Proxmox host, then Keycloak on
`dev.bitwisecook.org`:

```sh
./provision-samba-lxc.sh
./run-complex-auth-lab.sh
```

The retained `evtflow-auth-client` container uses
[`client/oidc-flow.sh`](client/oidc-flow.sh) for the browser-style OIDC
authorization-code flow.

The retained appliance policy leaves OpenID token processing disabled because
the enabled variant reproducibly cores `apmd` on the tested release after a
successful token response. `enable-oidc-crash-probe.sh` records the exact
opt-in mutation; do not leave that mutation active.

The services are intentionally persistent. To remove only these owned
fixtures later:

```sh
pct stop 104
pct destroy 104 --purge
sudo podman rm --force evtflow-keycloak evtflow-auth-client
```

Do not remove `evtflow-auth-macvlan` while the peer OpenLDAP fixture uses it.

# Disposable APM authentication source

`run-openldap.sh` starts a persistent OpenLDAP test source on
`192.168.9.250:1389`. Its macvlan network is internal and has no default
gateway, so the container can answer BIG-IP on the directly connected lab
subnet but cannot initiate traffic beyond it. Pulling and building the image
happen before the container joins that network.

The committed credentials are intentionally generated test-probe data:

- bind DN: `cn=admin,dc=bitwisecook,dc=org`
- bind password: `evtflow-admin-2026`
- test user DN: `uid=evtflow,ou=people,dc=bitwisecook,dc=org`
- test username: `evtflow`
- test password: `evtflow-pass-2026`

No appliance, SSH, GitHub, or pre-existing authentication credential belongs
in this directory.

The derived image is pinned to `docker.io/bitnamilegacy/openldap:2.6.8`; its
exact resolved digest is captured with each evidence run.

Run on the development host:

```sh
./run-openldap.sh
```

The container is deliberately retained as part of the reusable event-flow lab.
To remove only this owned container and network later:

```sh
sudo podman rm --force evtflow-openldap
sudo podman network rm evtflow-auth-macvlan
```

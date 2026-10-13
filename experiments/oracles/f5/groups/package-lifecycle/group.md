# package-lifecycle

20 retained configuration recipes share [group.scf](group.scf) and the [base](../../base/base.md).

The JSON group descriptor and compressed case table own the reconstruction inputs. Each case stores only changed source slices against its historical reconstruction template. The deployment group SCF contains an owned folder; it does not load the reconstruction template or execute its probe bodies. The F5 CLI supplies address rewriting and object comparisons. These configurations are replay projections; their generation is not an appliance acceptance or execution measurement.

For a stateful case, retain and follow the historical driver order. Source channels, failed loads, actual worker identities and cleanup limits remain separate. Do not deploy the group and another case concurrently without proving their owned names and destinations are disjoint.

```sh
python3 experiments/oracles/f5/materialise.py addresses --group package-lifecycle
python3 experiments/oracles/f5/materialise.py materialise --group package-lifecycle --case CASE --network 192.168.9.0/24 --output /tmp/f5-case
```

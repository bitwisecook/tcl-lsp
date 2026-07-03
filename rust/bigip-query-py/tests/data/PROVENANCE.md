# Test / demo data provenance

Real F5 BIG-IP UCS archives are almost never published publicly — they embed
private keys, password hashes and other secrets, and the `.ucs` extension is
also used by several unrelated formats (game locale strings, Xilinx pin
constraints, …), so there is no corpus of genuine BIG-IP UCS files to download.

Instead, these fixtures are built from **real, published F5 BIG-IP example
configurations** and packaged into valid UCS archives (a UCS is just a
gzip-compressed tar of the device's `/config` tree — see `build_ucs.py`).

| File | Source config | Origin |
|------|---------------|--------|
| `device-01.bigip.conf` | `tests/archive_generator/archive1/config/bigip.conf` | [`f5devcentral/f5-corkscrew`](https://github.com/f5devcentral/f5-corkscrew) — F5's official UCS/qkview parsing tool. TMOS 15.1.8.2; 9 virtuals, 9 pools, 11 nodes, 5 iRules, monitors, data-groups, LTM policies, client/server-SSL profiles, APM/ASM objects. |
| `device-02.bigip.conf` | `test_bigip.conf` | [`DumpySquare/f5-fasting`](https://github.com/DumpySquare/f5-fasting) — an F5 field engineer's realistic test config. TMOS 15.1.0.4. |

The `device-NN.bigip_base.conf` files are small `sys global-settings` stanzas
that give each synthetic device a distinct hostname, mirroring the
`config/bigip_base.conf` member a real UCS carries.

Nothing here contains real secrets, customer data, or private keys.

## Regenerating

```
python build_ucs.py     # rebuilds lab-device-01.ucs / lab-device-02.ucs
```

from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
    BigipPropertySpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "gtm_pool_a",
            module="gtm",
            object_types=("pool a",),
        ),
        header_types=(("gtm", "pool a"),),
        properties=(
            BigipPropertySpec(name="alternate-mode", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="dynamic", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="fallback-ip", value_type="string"),
            BigipPropertySpec(name="fallback-mode", value_type="string"),
            BigipPropertySpec(name="limit-max-bps", value_type="integer"),
            BigipPropertySpec(
                name="limit-max-bps-status", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="limit-max-connections", value_type="integer"),
            BigipPropertySpec(
                name="limit-max-connections-status",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="limit-max-pps", value_type="integer"),
            BigipPropertySpec(
                name="limit-max-pps-status", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="load-balancing-mode", value_type="string"),
            BigipPropertySpec(
                name="manual-resume", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="max-answers-returned", value_type="integer"),
            BigipPropertySpec(name="members", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="depends-on", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="member-order", value_type="integer"),
            BigipPropertySpec(
                name="monitor",
                value_type="reference",
                repeated=True,
                allow_none=True,
                references=(
                    "gtm_monitor_bigip",
                    "gtm_monitor_bigip_link",
                    "gtm_monitor_external",
                    "gtm_monitor_firepass",
                    "gtm_monitor_ftp",
                    "gtm_monitor_gateway_icmp",
                    "gtm_monitor_gtp",
                    "gtm_monitor_http",
                    "gtm_monitor_https",
                    "gtm_monitor_imap",
                    "gtm_monitor_ldap",
                    "gtm_monitor_mssql",
                    "gtm_monitor_mysql",
                    "gtm_monitor_nntp",
                    "gtm_monitor_none",
                    "gtm_monitor_oracle",
                    "gtm_monitor_pop3",
                    "gtm_monitor_postgresql",
                    "gtm_monitor_radius",
                    "gtm_monitor_radius_accounting",
                    "gtm_monitor_real_server",
                    "gtm_monitor_scripted",
                    "gtm_monitor_sip",
                    "gtm_monitor_smtp",
                    "gtm_monitor_snmp",
                    "gtm_monitor_snmp_link",
                    "gtm_monitor_soap",
                    "gtm_monitor_tcp",
                    "gtm_monitor_tcp_half_open",
                    "gtm_monitor_udp",
                    "gtm_monitor_wap",
                    "gtm_monitor_wmi",
                ),
            ),
            BigipPropertySpec(name="ratio", value_type="integer"),
            BigipPropertySpec(name="metadata", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="persist", value_type="enum", enum_values=("true", "false")),
            BigipPropertySpec(name="value", value_type="string"),
            BigipPropertySpec(
                name="min-members-up-mode",
                value_type="enum",
                enum_values=("off", "number", "percent"),
            ),
            BigipPropertySpec(name="min-members-up-value", value_type="integer"),
            BigipPropertySpec(name="qos-hit-ratio", value_type="integer"),
            BigipPropertySpec(name="qos-hops", value_type="integer"),
            BigipPropertySpec(name="qos-kilobytes-second", value_type="integer"),
            BigipPropertySpec(name="qos-lcs", value_type="integer"),
            BigipPropertySpec(name="qos-packet-rate", value_type="integer"),
            BigipPropertySpec(name="qos-rtt", value_type="integer"),
            BigipPropertySpec(name="qos-topology", value_type="integer"),
            BigipPropertySpec(name="qos-vs-capacity", value_type="integer"),
            BigipPropertySpec(name="qos-vs-score", value_type="integer"),
            BigipPropertySpec(name="ttl", value_type="integer"),
            BigipPropertySpec(
                name="verify-member-availability",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )

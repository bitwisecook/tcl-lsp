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
            "gtm_link",
            module="gtm",
            object_types=("link",),
        ),
        header_types=(("gtm", "link"),),
        properties=(
            BigipPropertySpec(name="cost-segments", value_type="string"),
            BigipPropertySpec(name="datacenter", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="duplex-billing", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="limit-max-inbound-bps", value_type="integer"),
            BigipPropertySpec(
                name="limit-max-inbound-bps-status",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="limit-max-outbound-bps", value_type="integer"),
            BigipPropertySpec(
                name="limit-max-outbound-bps-status",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="limit-max-total-bps", value_type="integer"),
            BigipPropertySpec(
                name="limit-max-total-bps-status",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="link-ratio", value_type="integer"),
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
            BigipPropertySpec(name="prepaid-segment", value_type="integer"),
            BigipPropertySpec(name="device-name", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="translation", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="service-provider", value_type="string"),
            BigipPropertySpec(
                name="uplink-address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="weighting", value_type="enum", enum_values=("price", "ratio")),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )

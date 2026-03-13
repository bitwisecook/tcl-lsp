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
            "gtm_server",
            module="gtm",
            object_types=("server",),
        ),
        header_types=(("gtm", "server"),),
        properties=(
            BigipPropertySpec(name="datacenter", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="explicit-link-name", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="translation", value_type="string"),
            BigipPropertySpec(
                name="expose-route-domains", value_type="enum", enum_values=("no", "yes")
            ),
            BigipPropertySpec(name="iq-allow-path", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(
                name="iq-allow-service-check", value_type="enum", enum_values=("no", "yes")
            ),
            BigipPropertySpec(name="iq-allow-snmp", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(name="iquery-cipher-list", value_type="string"),
            BigipPropertySpec(name="iquery-minimum-tls-version", value_type="string"),
            BigipPropertySpec(name="limit-cpu-usage", value_type="integer"),
            BigipPropertySpec(
                name="limit-cpu-usage-status",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="limit-mem-avail", value_type="integer"),
            BigipPropertySpec(
                name="limit-mem-avail-status",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
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
            BigipPropertySpec(
                name="link-discovery", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="value", value_type="string"),
            BigipPropertySpec(name="persist", value_type="enum", enum_values=("true", "false")),
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
            BigipPropertySpec(
                name="prober-fallback",
                value_type="enum",
                allow_none=True,
                enum_values=(
                    "inherit",
                    "any-available",
                    "inside-datacenter",
                    "outside-datacenter",
                    "pool",
                    "none",
                ),
            ),
            BigipPropertySpec(
                name="prober-pool",
                value_type="reference",
                allow_none=True,
                references=("ltm_pool",),
            ),
            BigipPropertySpec(
                name="prober-preference",
                value_type="enum",
                enum_values=("inherit", "inside-datacenter", "outside-datacenter", "pool"),
            ),
            BigipPropertySpec(name="product", value_type="string"),
            BigipPropertySpec(
                name="virtual-server-discovery",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="virtual-servers", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="depends-on", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="destination", value_type="string"),
            BigipPropertySpec(name="ltm-name", value_type="string"),
            BigipPropertySpec(
                name="translation-address",
                value_type="string",
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="translation-port", value_type="integer", min_value=0, max_value=65535
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )

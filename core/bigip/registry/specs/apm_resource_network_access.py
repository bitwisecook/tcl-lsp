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
            "apm_resource_network_access",
            module="apm",
            object_types=("resource network-access",),
        ),
        header_types=(("apm", "resource network-access"),),
        properties=(
            BigipPropertySpec(
                name="address-space-dhcp-requests-excluded",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="address-space-exclude",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="address-space-exclude-dns-name",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(
                name="address-space-exclude-subnet",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(
                name="address-space-include",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="address-space-include-dns-name",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(
                name="address-space-include-subnet",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(
                name="address-space-loc-dns-servers-excluded",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="address-space-local-subnets-excluded",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="address-space-protect",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="app-service", value_type="string", allow_none=True),
            BigipPropertySpec(name="application-launch", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="application-launch-warning",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="auto-launch", value_type="enum", enum_values=("false", "true")),
            BigipPropertySpec(name="client-interface-speed", value_type="integer", allow_none=True),
            BigipPropertySpec(
                name="client-ip-filter-engine",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="client-power-management",
                value_type="enum",
                enum_values=("ignore", "prevent", "terminate"),
            ),
            BigipPropertySpec(
                name="client-proxy",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="client-proxy-address", value_type="unknown"),
            BigipPropertySpec(
                name="client-proxy-enforce-subnets",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="client-proxy-exclusion-list",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(
                name="client-proxy-ignore-auto-config-error",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="client-proxy-local-bypass",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="client-proxy-port", value_type="integer", allow_none=True),
            BigipPropertySpec(name="client-proxy-script", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="client-proxy-use-http-pac",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="client-proxy-use-local-proxy",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="client-traffic-classifier",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(
                name="compression",
                value_type="enum",
                allow_none=True,
                enum_values=("gzip", "none"),
            ),
            BigipPropertySpec(name="customization-group", value_type="string", allow_none=True),
            BigipPropertySpec(name="description", value_type="string", allow_none=True),
            BigipPropertySpec(name="dns-primary", value_type="unknown"),
            BigipPropertySpec(name="dns-secondary", value_type="unknown"),
            BigipPropertySpec(name="dns-suffix", value_type="string", allow_none=True),
            BigipPropertySpec(name="drive-mapping", value_type="string", allow_none=True),
            BigipPropertySpec(name="dtls", value_type="enum", enum_values=("false", "true")),
            BigipPropertySpec(name="dtls-port", value_type="integer", allow_none=True),
            BigipPropertySpec(
                name="execute-logoff-scripts",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="idle-timeout-threshold", value_type="integer", allow_none=True),
            BigipPropertySpec(name="idle-timeout-window", value_type="integer", allow_none=True),
            BigipPropertySpec(
                name="ipv6-address-space-exclude-subnet",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(
                name="ipv6-address-space-include-subnet",
                value_type="string",
                allow_none=True,
            ),
            BigipPropertySpec(name="ipv6-dns-primary", value_type="unknown"),
            BigipPropertySpec(name="ipv6-dns-secondary", value_type="unknown"),
            BigipPropertySpec(name="ipv6-leasepool-name", value_type="string", allow_none=True),
            BigipPropertySpec(name="leasepool-name", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="location-specific",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="microsoft-network-client",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="microsoft-network-server",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="network-tunnel",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="optimized-app",
                value_type="list",
                allow_none=True,
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
            ),
            BigipPropertySpec(
                name="provide-client-cert",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="proxy-arp", value_type="enum", enum_values=("false", "true")),
            BigipPropertySpec(
                name="split-tunneling",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(name="static-host", value_type="string", allow_none=True),
            BigipPropertySpec(
                name="supported-ip-version",
                value_type="enum",
                enum_values=("ipv4", "ipv4-ipv6"),
            ),
            BigipPropertySpec(
                name="sync-with-active-directory",
                value_type="enum",
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                enum_values=(
                    "app-tunnel",
                    "last",
                    "network-access",
                    "remote-desktop",
                    "web-application",
                ),
            ),
            BigipPropertySpec(name="wins-primary", value_type="unknown"),
            BigipPropertySpec(name="wins-secondary", value_type="unknown"),
        ),
    )

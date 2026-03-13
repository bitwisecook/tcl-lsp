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
            "net_routing_bgp",
            module="net",
            object_types=("routing bgp",),
        ),
        header_types=(("net", "routing bgp"),),
        properties=(
            BigipPropertySpec(
                name="allow-infinite-hold-time",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="always-compare-med", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="bestpath", value_type="string"),
            BigipPropertySpec(
                name="as-path-ignore",
                value_type="enum",
                in_sections=("bestpath",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="compare-confed-aspath",
                value_type="enum",
                in_sections=("bestpath",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="compare-originator-id",
                value_type="enum",
                in_sections=("bestpath",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="compare-routerid",
                value_type="enum",
                in_sections=("bestpath",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="med", value_type="string", in_sections=("bestpath",)),
            BigipPropertySpec(
                name="confed",
                value_type="enum",
                in_sections=("med",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="missing-as-worst",
                value_type="enum",
                in_sections=("med",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="remove-recv-med",
                value_type="enum",
                in_sections=("med",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="remove-send-med",
                value_type="enum",
                in_sections=("med",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="tie-break-on-age",
                value_type="enum",
                in_sections=("bestpath",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="client-to-client-reflection",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="cluster-id", value_type="integer"),
            BigipPropertySpec(name="confederation", value_type="string"),
            BigipPropertySpec(
                name="identifier", value_type="integer", in_sections=("confederation",)
            ),
            BigipPropertySpec(
                name="peers", value_type="boolean", in_sections=("confederation",), allow_none=True
            ),
            BigipPropertySpec(name="dampening", value_type="string"),
            BigipPropertySpec(
                name="reachability-half-life", value_type="integer", in_sections=("dampening",)
            ),
            BigipPropertySpec(name="reuse", value_type="integer", in_sections=("dampening",)),
            BigipPropertySpec(
                name="route-map", value_type="boolean", in_sections=("dampening",), allow_none=True
            ),
            BigipPropertySpec(
                name="state",
                value_type="enum",
                in_sections=("dampening",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="suppress", value_type="integer", in_sections=("dampening",)),
            BigipPropertySpec(
                name="suppress-max", value_type="integer", in_sections=("dampening",)
            ),
            BigipPropertySpec(
                name="unreachability-half-life", value_type="integer", in_sections=("dampening",)
            ),
            BigipPropertySpec(name="default-local-preference", value_type="integer"),
            BigipPropertySpec(name="description", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="deterministic-med", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="enabled", value_type="enum", enum_values=("true", "false")),
            BigipPropertySpec(
                name="enforce-first-as", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="fast-external-failover",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="graceful-restart", value_type="string"),
            BigipPropertySpec(
                name="graceful-reset",
                value_type="enum",
                in_sections=("graceful-restart",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="restart-time", value_type="integer", in_sections=("graceful-restart",)
            ),
            BigipPropertySpec(
                name="stalepath-time", value_type="integer", in_sections=("graceful-restart",)
            ),
            BigipPropertySpec(name="graceful-shutdown", value_type="string"),
            BigipPropertySpec(
                name="capable",
                value_type="enum",
                in_sections=("graceful-shutdown",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="local-preference", value_type="integer", in_sections=("graceful-shutdown",)
            ),
            BigipPropertySpec(
                name="mode",
                value_type="enum",
                in_sections=("graceful-shutdown",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="hold-time", value_type="integer"),
            BigipPropertySpec(name="keep-alive", value_type="integer"),
            BigipPropertySpec(name="local-as", value_type="integer"),
            BigipPropertySpec(
                name="log-neighbor-changes", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="profile", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="route-domain",
                value_type="reference",
                allow_none=True,
                references=("net_route_domain",),
            ),
            BigipPropertySpec(name="router-id", value_type="integer"),
            BigipPropertySpec(name="scan-time", value_type="integer"),
            BigipPropertySpec(
                name="synchronization", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="update-delay", value_type="integer"),
            BigipPropertySpec(name="view", value_type="enum", enum_values=("disabled", "enabled")),
            BigipPropertySpec(
                name="address-family",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="auto-summary",
                value_type="enum",
                in_sections=("address-family",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="distance", value_type="string", in_sections=("address-family",)
            ),
            BigipPropertySpec(name="external", value_type="integer", in_sections=("distance",)),
            BigipPropertySpec(name="internal", value_type="integer", in_sections=("distance",)),
            BigipPropertySpec(name="local", value_type="integer", in_sections=("distance",)),
            BigipPropertySpec(
                name="network-synchronization",
                value_type="enum",
                in_sections=("address-family",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="aggregate-address",
                value_type="enum",
                in_sections=("address-family",),
                enum_values=("add", "delete", "modify", "replace-all-with"),
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(
                name="as-set",
                value_type="enum",
                in_sections=("aggregate-address",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="summary-only",
                value_type="enum",
                in_sections=("aggregate-address",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="redistribute",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="route-map",
                value_type="boolean",
                in_sections=("redistribute",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="distance",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="access-list", value_type="boolean", in_sections=("distance",), allow_none=True
            ),
            BigipPropertySpec(name="distance", value_type="integer", in_sections=("distance",)),
            BigipPropertySpec(
                name="neighbor",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="advertisement-interval", value_type="integer", in_sections=("neighbor",)
            ),
            BigipPropertySpec(
                name="allow-infinite-hold-time",
                value_type="enum",
                in_sections=("neighbor",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="as-origination-interval", value_type="integer", in_sections=("neighbor",)
            ),
            BigipPropertySpec(name="capability", value_type="string", in_sections=("neighbor",)),
            BigipPropertySpec(
                name="dynamic",
                value_type="enum",
                in_sections=("capability",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="route-refresh",
                value_type="enum",
                in_sections=("capability",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="capability-negotiate", value_type="string", in_sections=("neighbor",)
            ),
            BigipPropertySpec(
                name="override",
                value_type="enum",
                in_sections=("capability-negotiate",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="state",
                value_type="enum",
                in_sections=("capability-negotiate",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="strict-match",
                value_type="enum",
                in_sections=("capability-negotiate",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="collide-established",
                value_type="enum",
                in_sections=("neighbor",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="connect-timer", value_type="integer", in_sections=("neighbor",)
            ),
            BigipPropertySpec(
                name="description", value_type="boolean", in_sections=("neighbor",), allow_none=True
            ),
            BigipPropertySpec(
                name="ebgp-multihop", value_type="integer", in_sections=("neighbor",)
            ),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                in_sections=("neighbor",),
                enum_values=("true", "false"),
            ),
            BigipPropertySpec(
                name="enforce-multihop",
                value_type="enum",
                in_sections=("neighbor",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="fall-over", value_type="boolean", in_sections=("neighbor",), allow_none=True
            ),
            BigipPropertySpec(
                name="graceful-shutdown", value_type="string", in_sections=("neighbor",)
            ),
            BigipPropertySpec(
                name="timer", value_type="integer", in_sections=("graceful-shutdown",)
            ),
            BigipPropertySpec(name="hold-time", value_type="integer", in_sections=("neighbor",)),
            BigipPropertySpec(name="keep-alive", value_type="integer", in_sections=("neighbor",)),
            BigipPropertySpec(name="local-as", value_type="integer", in_sections=("neighbor",)),
            BigipPropertySpec(
                name="passive",
                value_type="enum",
                in_sections=("neighbor",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="password", value_type="boolean", in_sections=("neighbor",), allow_none=True
            ),
            BigipPropertySpec(
                name="peer-group", value_type="boolean", in_sections=("neighbor",), allow_none=True
            ),
            BigipPropertySpec(
                name="port",
                value_type="integer",
                in_sections=("neighbor",),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(name="remote-as", value_type="integer", in_sections=("neighbor",)),
            BigipPropertySpec(name="restart-time", value_type="integer", in_sections=("neighbor",)),
            BigipPropertySpec(
                name="update-source",
                value_type="boolean",
                in_sections=("neighbor",),
                allow_none=True,
            ),
            BigipPropertySpec(name="version", value_type="integer", in_sections=("neighbor",)),
            BigipPropertySpec(
                name="vlan",
                value_type="reference",
                in_sections=("neighbor",),
                allow_none=True,
                references=("net_vlan",),
            ),
            BigipPropertySpec(
                name="address-family",
                value_type="enum",
                in_sections=("neighbor",),
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="activate",
                value_type="enum",
                in_sections=("address-family",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="allow-as-in",
                value_type="boolean",
                in_sections=("address-family",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="as-override",
                value_type="enum",
                in_sections=("address-family",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="attribute-unchanged", value_type="string", in_sections=("address-family",)
            ),
            BigipPropertySpec(
                name="as-path",
                value_type="enum",
                in_sections=("attribute-unchanged",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="med",
                value_type="enum",
                in_sections=("attribute-unchanged",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="next-hop",
                value_type="enum",
                in_sections=("attribute-unchanged",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="capability", value_type="string", in_sections=("address-family",)
            ),
            BigipPropertySpec(
                name="graceful-restart",
                value_type="enum",
                in_sections=("capability",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="orf", value_type="string", in_sections=("capability",)),
            BigipPropertySpec(
                name="prefix-list", value_type="boolean", in_sections=("orf",), allow_none=True
            ),
            BigipPropertySpec(
                name="default-originate", value_type="string", in_sections=("address-family",)
            ),
            BigipPropertySpec(
                name="route-map",
                value_type="boolean",
                in_sections=("default-originate",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="state",
                value_type="enum",
                in_sections=("default-originate",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="distribute-list", value_type="string", in_sections=("address-family",)
            ),
            BigipPropertySpec(
                name="in", value_type="boolean", in_sections=("distribute-list",), allow_none=True
            ),
            BigipPropertySpec(
                name="out", value_type="boolean", in_sections=("distribute-list",), allow_none=True
            ),
            BigipPropertySpec(
                name="filter-list", value_type="string", in_sections=("address-family",)
            ),
            BigipPropertySpec(
                name="in", value_type="boolean", in_sections=("filter-list",), allow_none=True
            ),
            BigipPropertySpec(
                name="out", value_type="boolean", in_sections=("filter-list",), allow_none=True
            ),
            BigipPropertySpec(
                name="maximum-prefix", value_type="string", in_sections=("address-family",)
            ),
            BigipPropertySpec(
                name="threshold",
                value_type="boolean",
                in_sections=("maximum-prefix",),
                allow_none=True,
            ),
            BigipPropertySpec(name="value", value_type="integer", in_sections=("maximum-prefix",)),
            BigipPropertySpec(
                name="warning-only",
                value_type="enum",
                in_sections=("maximum-prefix",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="next-hop-self",
                value_type="enum",
                in_sections=("address-family",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="prefix-list", value_type="string", in_sections=("address-family",)
            ),
            BigipPropertySpec(
                name="in", value_type="boolean", in_sections=("prefix-list",), allow_none=True
            ),
            BigipPropertySpec(
                name="out", value_type="boolean", in_sections=("prefix-list",), allow_none=True
            ),
            BigipPropertySpec(
                name="remove-private-as",
                value_type="enum",
                in_sections=("address-family",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="route-map", value_type="string", in_sections=("address-family",)
            ),
            BigipPropertySpec(
                name="in", value_type="boolean", in_sections=("route-map",), allow_none=True
            ),
            BigipPropertySpec(
                name="out", value_type="boolean", in_sections=("route-map",), allow_none=True
            ),
            BigipPropertySpec(
                name="route-reflector-client",
                value_type="enum",
                in_sections=("address-family",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="route-server-client",
                value_type="enum",
                in_sections=("address-family",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="send-community",
                value_type="boolean",
                in_sections=("address-family",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="soft-reconfiguration-inbound",
                value_type="enum",
                in_sections=("address-family",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="unsuppress-map",
                value_type="boolean",
                in_sections=("address-family",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="weight",
                value_type="boolean",
                in_sections=("address-family",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="network",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="backdoor",
                value_type="enum",
                in_sections=("network",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="route-map", value_type="boolean", in_sections=("network",), allow_none=True
            ),
            BigipPropertySpec(
                name="peer-group",
                value_type="enum",
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="advertisement-interval", value_type="integer", in_sections=("peer-group",)
            ),
            BigipPropertySpec(
                name="allow-infinite-hold-time",
                value_type="enum",
                in_sections=("peer-group",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="as-origination-interval", value_type="integer", in_sections=("peer-group",)
            ),
            BigipPropertySpec(name="capability", value_type="string", in_sections=("peer-group",)),
            BigipPropertySpec(
                name="capability-negotiate", value_type="string", in_sections=("peer-group",)
            ),
            BigipPropertySpec(
                name="collide-established",
                value_type="enum",
                in_sections=("peer-group",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="connect-timer", value_type="integer", in_sections=("peer-group",)
            ),
            BigipPropertySpec(
                name="description",
                value_type="boolean",
                in_sections=("peer-group",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="ebgp-multihop", value_type="integer", in_sections=("peer-group",)
            ),
            BigipPropertySpec(
                name="enabled",
                value_type="enum",
                in_sections=("peer-group",),
                enum_values=("true", "false"),
            ),
            BigipPropertySpec(
                name="enforce-multihop",
                value_type="enum",
                in_sections=("peer-group",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="fall-over", value_type="boolean", in_sections=("peer-group",), allow_none=True
            ),
            BigipPropertySpec(
                name="graceful-shutdown", value_type="string", in_sections=("peer-group",)
            ),
            BigipPropertySpec(name="hold-time", value_type="integer", in_sections=("peer-group",)),
            BigipPropertySpec(name="keep-alive", value_type="integer", in_sections=("peer-group",)),
            BigipPropertySpec(name="local-as", value_type="integer", in_sections=("peer-group",)),
            BigipPropertySpec(
                name="passive",
                value_type="enum",
                in_sections=("peer-group",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="password", value_type="boolean", in_sections=("peer-group",), allow_none=True
            ),
            BigipPropertySpec(
                name="port",
                value_type="integer",
                in_sections=("peer-group",),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(name="remote-as", value_type="integer", in_sections=("peer-group",)),
            BigipPropertySpec(
                name="restart-time", value_type="integer", in_sections=("peer-group",)
            ),
            BigipPropertySpec(
                name="update-source",
                value_type="boolean",
                in_sections=("peer-group",),
                allow_none=True,
            ),
            BigipPropertySpec(name="version", value_type="integer", in_sections=("peer-group",)),
            BigipPropertySpec(
                name="address-family",
                value_type="enum",
                in_sections=("peer-group",),
                enum_values=("add", "delete", "modify", "replace-all-with"),
            ),
        ),
    )

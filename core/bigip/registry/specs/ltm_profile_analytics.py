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
            "ltm_profile_analytics",
            module="ltm",
            object_types=("profile analytics",),
        ),
        header_types=(("ltm", "profile analytics"),),
        properties=(
            BigipPropertySpec(
                name="alerts",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="name", value_type="string", in_sections=("alerts",)),
            BigipPropertySpec(name="granularity", value_type="string", in_sections=("name",)),
            BigipPropertySpec(name="metric", value_type="string", in_sections=("name",)),
            BigipPropertySpec(
                name="average-response-throughput", value_type="string", in_sections=("name",)
            ),
            BigipPropertySpec(name="average-tps", value_type="string", in_sections=("name",)),
            BigipPropertySpec(
                name="max-server-latency", value_type="string", in_sections=("name",)
            ),
            BigipPropertySpec(name="sample-period", value_type="integer", in_sections=("name",)),
            BigipPropertySpec(name="threshold", value_type="integer", in_sections=("name",)),
            BigipPropertySpec(
                name="threshold-relation",
                value_type="enum",
                in_sections=("name",),
                enum_values=("above", "below"),
            ),
            BigipPropertySpec(
                name="captured-traffic-external-logging",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="captured-traffic-internal-logging",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="collect-page-load-time",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="collect-geo", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="collect-http-throughput",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="collect-http-timing-metrics",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="collect-ip", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="collect-max-tps-and-throughput",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="collect-methods", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="collect-response-codes",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="collect-server-latency",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="collect-subnets", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="collect-url", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="collect-user-agent", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="collect-user-sessions", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="collected-stats-external-logging",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="collected-stats-internal-logging",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_analytics",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="external-logging-publisher", value_type="string"),
            BigipPropertySpec(
                name="notification-by-email", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="notification-by-snmp", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="notification-by-syslog",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="notification-email-addresses", value_type="boolean", allow_none=True
            ),
            BigipPropertySpec(
                name="publish-irule-statistics",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="sampling", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="session-cookie-security",
                value_type="enum",
                enum_values=("always-secure", "ssl-only", "never-secure"),
            ),
            BigipPropertySpec(name="session-timeout-minutes", value_type="integer"),
            BigipPropertySpec(name="smtp-config", value_type="string"),
            BigipPropertySpec(name="subnet-masks", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="name", value_type="string"),
            BigipPropertySpec(name="subnet", value_type="string", in_sections=("name",)),
            BigipPropertySpec(
                name="countries-for-stat-collection",
                value_type="enum",
                enum_values=("add", "delete"),
            ),
            BigipPropertySpec(
                name="ips-for-stat-collection", value_type="enum", enum_values=("add", "delete")
            ),
            BigipPropertySpec(
                name="subnets-for-stat-collection", value_type="enum", enum_values=("add", "delete")
            ),
            BigipPropertySpec(
                name="urls-for-stat-collection", value_type="enum", enum_values=("add", "delete")
            ),
            BigipPropertySpec(name="traffic-capture", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="captured-protocols",
                value_type="enum",
                in_sections=("name",),
                enum_values=("all", "http", "https"),
            ),
            BigipPropertySpec(
                name="client-ips", value_type="boolean", in_sections=("name",), allow_none=True
            ),
            BigipPropertySpec(
                name="dos-activity", value_type="enum", enum_values=("any", "mitigated-by-dosl7")
            ),
            BigipPropertySpec(name="methods", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="node-addresses", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="request-captured-parts",
                value_type="enum",
                allow_none=True,
                enum_values=("all", "body", "headers", "none"),
            ),
            BigipPropertySpec(name="request-content-filter-search-part", value_type="string"),
            BigipPropertySpec(name="none", value_type="string"),
            BigipPropertySpec(
                name="request-content-filter-search-string", value_type="boolean", allow_none=True
            ),
            BigipPropertySpec(
                name="response-captured-parts",
                value_type="enum",
                allow_none=True,
                enum_values=("all", "body", "headers", "none"),
            ),
            BigipPropertySpec(name="response-codes", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="response-content-filter-search-part", value_type="string"),
            BigipPropertySpec(name="headers", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="response-content-filter-search-string", value_type="boolean", allow_none=True
            ),
            BigipPropertySpec(
                name="url-filter-type",
                value_type="enum",
                enum_values=("all", "black-list", "white-list"),
            ),
            BigipPropertySpec(name="url-path-prefixes", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="user-agent-substrings", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="virtual-servers", value_type="boolean", allow_none=True),
        ),
    )

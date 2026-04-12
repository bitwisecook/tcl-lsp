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
            "security_log_profile",
            module="security",
            object_types=("log profile",),
        ),
        header_types=(("security", "log profile"),),
        properties=(
            BigipPropertySpec(
                name="antifraud",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="name", value_type="string", in_sections=("antifraud",)),
            BigipPropertySpec(
                name="encode-fields",
                value_type="enum",
                in_sections=("name",),
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="events",
                value_type="enum",
                in_sections=("antifraud",),
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                in_sections=("events",),
                enum_values=("alert", "login"),
            ),
            BigipPropertySpec(name="format", value_type="string", in_sections=("type",)),
            BigipPropertySpec(
                name="type",
                value_type="reference",
                in_sections=("format",),
                allow_none=True,
                enum_values=("none", "default", "user-defined"),
                references=("auth_user",),
            ),
            BigipPropertySpec(name="user-template", value_type="string", in_sections=("format",)),
            BigipPropertySpec(name="rate-limit", value_type="integer", in_sections=("type",)),
            BigipPropertySpec(
                name="rate-limit-template", value_type="string", in_sections=("antifraud",)
            ),
            BigipPropertySpec(
                name="remote-publisher",
                value_type="boolean",
                in_sections=("antifraud",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="application",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="name", value_type="string", in_sections=("application",)),
            BigipPropertySpec(
                name="facility",
                value_type="enum",
                in_sections=("name",),
                enum_values=(
                    "local0",
                    "local1",
                    "local2",
                    "local3",
                    "local4",
                    "local5",
                    "local6",
                    "local7",
                ),
            ),
            BigipPropertySpec(
                name="filter",
                value_type="enum",
                in_sections=("name",),
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="key", value_type="string", in_sections=("filter",)),
            BigipPropertySpec(name="search-all", value_type="string", in_sections=("filter",)),
            BigipPropertySpec(
                name="values",
                value_type="enum",
                in_sections=("search-all",),
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="format", value_type="string", in_sections=("application",)),
            BigipPropertySpec(name="field-delimiter", value_type="string", in_sections=("format",)),
            BigipPropertySpec(name="field-format", value_type="string", in_sections=("format",)),
            BigipPropertySpec(
                name="fields",
                value_type="boolean",
                in_sections=("format",),
                repeated=True,
                allow_none=True,
            ),
            BigipPropertySpec(
                name="type",
                value_type="reference",
                in_sections=("application",),
                enum_values=("predefined", "user-defined"),
                references=("auth_user",),
            ),
            BigipPropertySpec(
                name="user-string", value_type="string", in_sections=("application",)
            ),
            BigipPropertySpec(
                name="guarantee-logging", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="guarantee-response-logging",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="local-storage", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="logic-operation", value_type="enum", enum_values=("and", "or")),
            BigipPropertySpec(
                name="maximum-entry-length",
                value_type="enum",
                enum_values=("1k", "2k", "10k", "64k"),
            ),
            BigipPropertySpec(name="maximum-header-size", value_type="integer"),
            BigipPropertySpec(name="maximum-query-size", value_type="integer"),
            BigipPropertySpec(name="maximum-request-size", value_type="integer"),
            BigipPropertySpec(
                name="protocol", value_type="enum", enum_values=("udp", "tcp", "tcp-rfc3195")
            ),
            BigipPropertySpec(
                name="remote-storage",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "remote", "splunk", "arcsight"),
            ),
            BigipPropertySpec(
                name="report-anomalies", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="response-logging",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "illegal", "all"),
            ),
            BigipPropertySpec(
                name="servers",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="built-in", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="dos-application",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="name", value_type="string", in_sections=("dos-application",)),
            BigipPropertySpec(name="local-publisher", value_type="string", in_sections=("name",)),
            BigipPropertySpec(name="remote-publisher", value_type="string", in_sections=("name",)),
            BigipPropertySpec(
                name="bot-defense",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="name", value_type="string", in_sections=("bot-defense",)),
            BigipPropertySpec(
                name="log-illegal-requests",
                value_type="enum",
                in_sections=("filter",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-challenged-requests",
                value_type="enum",
                in_sections=("filter",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-legal-requests",
                value_type="enum",
                in_sections=("filter",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-captcha-challenged-requests",
                value_type="enum",
                in_sections=("filter",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-bot-signature-matched-requests",
                value_type="enum",
                in_sections=("filter",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="flowspec", value_type="string"),
            BigipPropertySpec(
                name="log-publisher",
                value_type="boolean",
                in_sections=("flowspec",),
                allow_none=True,
            ),
            BigipPropertySpec(name="ip-intelligence", value_type="string"),
            BigipPropertySpec(
                name="aggregate-rate", value_type="integer", in_sections=("ip-intelligence",)
            ),
            BigipPropertySpec(
                name="log-publisher",
                value_type="boolean",
                in_sections=("ip-intelligence",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="log-translation-fields",
                value_type="enum",
                in_sections=("ip-intelligence",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-shun",
                value_type="enum",
                in_sections=("ip-intelligence",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-geo",
                value_type="enum",
                in_sections=("ip-intelligence",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-rtbh",
                value_type="enum",
                in_sections=("ip-intelligence",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-scrubber",
                value_type="enum",
                in_sections=("ip-intelligence",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="port-misuse", value_type="string"),
            BigipPropertySpec(
                name="log-publisher",
                value_type="boolean",
                in_sections=("port-misuse",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="aggregate-rate", value_type="integer", in_sections=("port-misuse",)
            ),
            BigipPropertySpec(name="traffic-statistics", value_type="string"),
            BigipPropertySpec(
                name="log-sctive-flows",
                value_type="enum",
                in_sections=("traffic-statistics",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-publisher",
                value_type="boolean",
                in_sections=("traffic-statistics",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="log-missed-flows",
                value_type="enum",
                in_sections=("traffic-statistics",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-reaped-flows",
                value_type="enum",
                in_sections=("traffic-statistics",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-syncookies",
                value_type="enum",
                in_sections=("traffic-statistics",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-syncookies-whitelist",
                value_type="enum",
                in_sections=("traffic-statistics",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="network",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "delete", "modify", "none", "replace-all-with"),
            ),
            BigipPropertySpec(name="name", value_type="string", in_sections=("network",)),
            BigipPropertySpec(
                name="log-acl-match-accept",
                value_type="enum",
                in_sections=("filter",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-acl-match-drop",
                value_type="enum",
                in_sections=("filter",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-acl-match-reject",
                value_type="enum",
                in_sections=("filter",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-ip-errors",
                value_type="enum",
                in_sections=("filter",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-tcp-errors",
                value_type="enum",
                in_sections=("filter",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-tcp-events",
                value_type="enum",
                in_sections=("filter",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-translation-fields",
                value_type="enum",
                in_sections=("filter",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-geo-always",
                value_type="enum",
                in_sections=("filter",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-uuid-field",
                value_type="enum",
                in_sections=("filter",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="rate-limit", value_type="string", in_sections=("name",)),
            BigipPropertySpec(
                name="acl-match-accept", value_type="integer", in_sections=("rate-limit",)
            ),
            BigipPropertySpec(
                name="acl-match-drop", value_type="integer", in_sections=("rate-limit",)
            ),
            BigipPropertySpec(
                name="acl-match-reject", value_type="integer", in_sections=("rate-limit",)
            ),
            BigipPropertySpec(name="ip-errors", value_type="integer", in_sections=("rate-limit",)),
            BigipPropertySpec(name="tcp-errors", value_type="integer", in_sections=("rate-limit",)),
            BigipPropertySpec(name="tcp-events", value_type="integer", in_sections=("rate-limit",)),
            BigipPropertySpec(
                name="aggregate-rate", value_type="integer", in_sections=("rate-limit",)
            ),
            BigipPropertySpec(name="format", value_type="string", in_sections=("name",)),
            BigipPropertySpec(
                name="field-list",
                value_type="reference",
                in_sections=("format",),
                allow_none=True,
                references=("ltm_rule",),
            ),
            BigipPropertySpec(
                name="field-list-delimiter", value_type="string", in_sections=("name",)
            ),
            BigipPropertySpec(
                name="type",
                value_type="reference",
                in_sections=("name",),
                allow_none=True,
                enum_values=("field-list", "none", "user-defined"),
                references=("auth_user",),
            ),
            BigipPropertySpec(name="user-defined", value_type="string", in_sections=("name",)),
            BigipPropertySpec(
                name="publisher", value_type="boolean", in_sections=("network",), allow_none=True
            ),
            BigipPropertySpec(name="nat", value_type="string"),
            BigipPropertySpec(
                name="end-inbound-session",
                value_type="enum",
                in_sections=("nat",),
                enum_values=("backup-allocation-only", "disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="errors",
                value_type="enum",
                in_sections=("nat",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="format", value_type="string", in_sections=("nat",)),
            BigipPropertySpec(
                name="end-inbound-session", value_type="string", in_sections=("format",)
            ),
            BigipPropertySpec(
                name="field-list",
                value_type="reference",
                in_sections=("end-inbound-session",),
                allow_none=True,
                references=("net_route_domain",),
            ),
            BigipPropertySpec(
                name="timestamp",
                value_type="reference",
                in_sections=("end-inbound-session",),
                references=("net_route_domain",),
            ),
            BigipPropertySpec(
                name="field-list-delimiter", value_type="string", in_sections=("format",)
            ),
            BigipPropertySpec(name="user-defined", value_type="string", in_sections=("format",)),
            BigipPropertySpec(
                name="end-outbound-session", value_type="string", in_sections=("nat",)
            ),
            BigipPropertySpec(
                name="field-list",
                value_type="reference",
                in_sections=("end-outbound-session",),
                allow_none=True,
                references=("net_route_domain",),
            ),
            BigipPropertySpec(
                name="timestamp",
                value_type="reference",
                in_sections=("end-outbound-session",),
                references=("net_route_domain",),
            ),
            BigipPropertySpec(
                name="field-list-delimiter", value_type="string", in_sections=("nat",)
            ),
            BigipPropertySpec(
                name="type",
                value_type="reference",
                in_sections=("nat",),
                allow_none=True,
                enum_values=("field-list", "none", "user-defined"),
                references=("auth_user",),
            ),
            BigipPropertySpec(name="user-defined", value_type="string", in_sections=("nat",)),
            BigipPropertySpec(name="errors", value_type="string"),
            BigipPropertySpec(
                name="field-list",
                value_type="reference",
                in_sections=("errors",),
                allow_none=True,
                references=("net_route_domain",),
            ),
            BigipPropertySpec(
                name="timestamp",
                value_type="reference",
                in_sections=("errors",),
                references=("net_route_domain",),
            ),
            BigipPropertySpec(name="field-list-delimiter", value_type="string"),
            BigipPropertySpec(
                name="type",
                value_type="reference",
                allow_none=True,
                enum_values=("field-list", "none", "user-defined"),
                references=("auth_user",),
            ),
            BigipPropertySpec(name="user-defined", value_type="string"),
            BigipPropertySpec(name="quota-exceeded", value_type="string"),
            BigipPropertySpec(
                name="field-list",
                value_type="reference",
                in_sections=("quota-exceeded",),
                allow_none=True,
                references=("net_route_domain",),
            ),
            BigipPropertySpec(
                name="timestamp",
                value_type="reference",
                in_sections=("quota-exceeded",),
                references=("net_route_domain",),
            ),
            BigipPropertySpec(name="start-inbound-session", value_type="string"),
            BigipPropertySpec(
                name="field-list",
                value_type="reference",
                in_sections=("start-inbound-session",),
                allow_none=True,
                references=("net_route_domain",),
            ),
            BigipPropertySpec(
                name="timestamp",
                value_type="reference",
                in_sections=("start-inbound-session",),
                references=("net_route_domain",),
            ),
            BigipPropertySpec(name="start-outbound-session", value_type="string"),
            BigipPropertySpec(
                name="field-list",
                value_type="reference",
                in_sections=("start-outbound-session",),
                allow_none=True,
                references=("net_route_domain",),
            ),
            BigipPropertySpec(
                name="timestamp",
                value_type="reference",
                in_sections=("start-outbound-session",),
                references=("net_route_domain",),
            ),
            BigipPropertySpec(name="log-publisher", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="log-subscriber-id", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="lsn-legacy-mode", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="rate-limit", value_type="string"),
            BigipPropertySpec(
                name="end-inbound-session", value_type="integer", in_sections=("rate-limit",)
            ),
            BigipPropertySpec(
                name="end-outbound-session", value_type="integer", in_sections=("rate-limit",)
            ),
            BigipPropertySpec(name="errors", value_type="integer", in_sections=("rate-limit",)),
            BigipPropertySpec(
                name="quota-exceeded", value_type="integer", in_sections=("rate-limit",)
            ),
            BigipPropertySpec(
                name="start-inbound-session", value_type="integer", in_sections=("rate-limit",)
            ),
            BigipPropertySpec(
                name="start-outbound-session", value_type="integer", in_sections=("rate-limit",)
            ),
            BigipPropertySpec(name="end-outbound-session", value_type="string"),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("end-outbound-session",),
                enum_values=("backup-allocation-only", "disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="elements",
                value_type="enum",
                in_sections=("end-outbound-session",),
                allow_none=True,
                enum_values=("add", "delete", "none", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("start-outbound-session",),
                enum_values=("backup-allocation-only", "disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="elements",
                value_type="enum",
                in_sections=("start-outbound-session",),
                allow_none=True,
                enum_values=("add", "delete", "none", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="protocol-dns",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "delete", "modify", "none", "replace-all-with"),
            ),
            BigipPropertySpec(name="name", value_type="string", in_sections=("protocol-dns",)),
            BigipPropertySpec(
                name="log-dns-drop",
                value_type="enum",
                in_sections=("filter",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-dns-filtered-drop",
                value_type="enum",
                in_sections=("filter",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-dns-malformed",
                value_type="enum",
                in_sections=("filter",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-dns-malicious",
                value_type="enum",
                in_sections=("filter",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-dns-reject",
                value_type="enum",
                in_sections=("filter",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="publisher",
                value_type="boolean",
                in_sections=("protocol-dns",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="protocol-dns-dos-publisher", value_type="boolean", allow_none=True
            ),
            BigipPropertySpec(
                name="protocol-sip",
                value_type="enum",
                allow_none=True,
                enum_values=("add", "delete", "modify", "none", "replace-all-with"),
            ),
            BigipPropertySpec(name="name", value_type="string", in_sections=("protocol-sip",)),
            BigipPropertySpec(
                name="log-sip-drop",
                value_type="enum",
                in_sections=("filter",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-sip-global-failures",
                value_type="enum",
                in_sections=("filter",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-sip-malformed",
                value_type="enum",
                in_sections=("filter",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-sip-redirection-responses",
                value_type="enum",
                in_sections=("filter",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-sip-request-failures",
                value_type="enum",
                in_sections=("filter",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="log-sip-server-errors",
                value_type="enum",
                in_sections=("filter",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="publisher",
                value_type="boolean",
                in_sections=("protocol-sip",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="protocol-sip-dos-publisher", value_type="boolean", allow_none=True
            ),
            BigipPropertySpec(name="dos-network-publisher", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="protocol-transfer",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="name", value_type="string", in_sections=("protocol-transfer",)),
            BigipPropertySpec(name="publisher", value_type="string", in_sections=("name",)),
        ),
    )

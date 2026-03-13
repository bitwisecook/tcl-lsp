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
            "security_dos_device_config",
            module="security",
            object_types=("dos device-config",),
        ),
        header_types=(("security", "dos device-config"),),
        properties=(
            BigipPropertySpec(name="auto-threshold-sensitivity", value_type="string"),
            BigipPropertySpec(name="ip-uncommon-protolist", value_type="string"),
            BigipPropertySpec(
                name="threshold-sensitivity",
                value_type="enum",
                enum_values=("low", "medium", "high"),
            ),
            BigipPropertySpec(
                name="custom-signatures",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="name", value_type="string", in_sections=("custom-signatures",)),
            BigipPropertySpec(
                name="manual-detection-threshold", value_type="integer", in_sections=("name",)
            ),
            BigipPropertySpec(
                name="manual-mitigation-threshold", value_type="integer", in_sections=("name",)
            ),
            BigipPropertySpec(
                name="state",
                value_type="enum",
                in_sections=("name",),
                enum_values=("disabled", "learn-only", "detect-only", "mitigate"),
            ),
            BigipPropertySpec(
                name="threshold-mode",
                value_type="enum",
                in_sections=("name",),
                enum_values=(
                    "fully-automatic",
                    "manual",
                    "manual-multiplier-mitigation",
                    "stress-based-mitigation",
                ),
            ),
            BigipPropertySpec(name="dos-device-vector", value_type="string"),
            BigipPropertySpec(
                name="allow-advertisement",
                value_type="enum",
                in_sections=("dos-device-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="allow-upstream-scrubbing",
                value_type="enum",
                in_sections=("dos-device-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="attacked-dst",
                value_type="enum",
                in_sections=("dos-device-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="auto-blacklisting",
                value_type="enum",
                in_sections=("dos-device-vector",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="auto-scrubbing",
                value_type="enum",
                in_sections=("dos-device-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="auto-threshold",
                value_type="enum",
                in_sections=("dos-device-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="bad-actor",
                value_type="enum",
                in_sections=("dos-device-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="blacklist-category", value_type="string", in_sections=("dos-device-vector",)
            ),
            BigipPropertySpec(
                name="blacklist-detection-seconds",
                value_type="integer",
                in_sections=("dos-device-vector",),
            ),
            BigipPropertySpec(
                name="blacklist-duration", value_type="integer", in_sections=("dos-device-vector",)
            ),
            BigipPropertySpec(
                name="ceiling", value_type="integer", in_sections=("dos-device-vector",)
            ),
            BigipPropertySpec(
                name="default-internal-rate-limit",
                value_type="integer",
                in_sections=("dos-device-vector",),
            ),
            BigipPropertySpec(
                name="detection-threshold-percent",
                value_type="integer",
                in_sections=("dos-device-vector",),
            ),
            BigipPropertySpec(
                name="detection-threshold-pps",
                value_type="integer",
                in_sections=("dos-device-vector",),
            ),
            BigipPropertySpec(
                name="enforce",
                value_type="enum",
                in_sections=("dos-device-vector",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="floor", value_type="integer", in_sections=("dos-device-vector",)
            ),
            BigipPropertySpec(
                name="packet-types",
                value_type="enum",
                in_sections=("dos-device-vector",),
                enum_values=("add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="dns-any-query", value_type="string", in_sections=("packet-types",)
            ),
            BigipPropertySpec(
                name="dns-mx-query", value_type="string", in_sections=("packet-types",)
            ),
            BigipPropertySpec(
                name="dns-ptr-query", value_type="string", in_sections=("packet-types",)
            ),
            BigipPropertySpec(
                name="dns-txt-query", value_type="string", in_sections=("packet-types",)
            ),
            BigipPropertySpec(name="ipfrag", value_type="string", in_sections=("packet-types",)),
            BigipPropertySpec(
                name="ipv6-any-other", value_type="boolean", in_sections=("packet-types",)
            ),
            BigipPropertySpec(
                name="sip-bye-method", value_type="string", in_sections=("packet-types",)
            ),
            BigipPropertySpec(
                name="sip-malformed", value_type="boolean", in_sections=("packet-types",)
            ),
            BigipPropertySpec(
                name="sip-options-method", value_type="string", in_sections=("packet-types",)
            ),
            BigipPropertySpec(
                name="sip-publish-method", value_type="string", in_sections=("packet-types",)
            ),
            BigipPropertySpec(
                name="suspicious", value_type="string", in_sections=("packet-types",)
            ),
            BigipPropertySpec(
                name="tcp-synack", value_type="string", in_sections=("packet-types",)
            ),
            BigipPropertySpec(
                name="packet-types",
                value_type="boolean",
                in_sections=("packet-types",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="per-dst-ip-detection-pps", value_type="integer", in_sections=("packet-types",)
            ),
            BigipPropertySpec(
                name="per-dst-ip-limit-pps", value_type="integer", in_sections=("packet-types",)
            ),
            BigipPropertySpec(
                name="per-source-ip-detection-pps",
                value_type="integer",
                in_sections=("packet-types",),
            ),
            BigipPropertySpec(
                name="per-source-ip-limit-pps", value_type="integer", in_sections=("packet-types",)
            ),
            BigipPropertySpec(
                name="scrubbing-category",
                value_type="boolean",
                in_sections=("packet-types",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="scrubbing-detection-seconds",
                value_type="integer",
                in_sections=("packet-types",),
            ),
            BigipPropertySpec(
                name="scrubbing-duration", value_type="integer", in_sections=("packet-types",)
            ),
            BigipPropertySpec(
                name="simulate-auto-threshold",
                value_type="enum",
                in_sections=("packet-types",),
                enum_values=("enable", "disable"),
            ),
            BigipPropertySpec(
                name="state",
                value_type="enum",
                in_sections=("packet-types",),
                enum_values=("disabled", "learn-only", "detect-only", "mitigate"),
            ),
            BigipPropertySpec(
                name="tscookie",
                value_type="enum",
                in_sections=("packet-types",),
                enum_values=("disable", "enable"),
            ),
            BigipPropertySpec(
                name="threshold-mode",
                value_type="enum",
                in_sections=("packet-types",),
                enum_values=(
                    "manual",
                    "stress-based-mitigation",
                    "fully-automatic",
                    "manual-multiplier-mitigation",
                ),
            ),
            BigipPropertySpec(
                name="valid-domains",
                value_type="enum",
                in_sections=("packet-types",),
                enum_values=("add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="dynamic-signatures", value_type="string"),
            BigipPropertySpec(
                name="detection",
                value_type="enum",
                in_sections=("dynamic-signatures",),
                enum_values=("disabled", "enabled", "learn-only"),
            ),
            BigipPropertySpec(
                name="mitigation",
                value_type="enum",
                in_sections=("dynamic-signatures",),
                allow_none=True,
                enum_values=("none", "low", "medium", "high"),
            ),
            BigipPropertySpec(
                name="scrubber-advertisement-period",
                value_type="integer",
                in_sections=("dynamic-signatures",),
            ),
            BigipPropertySpec(
                name="scrubber-category", value_type="string", in_sections=("dynamic-signatures",)
            ),
            BigipPropertySpec(
                name="scrubber-enable",
                value_type="enum",
                in_sections=("dynamic-signatures",),
                enum_values=("yes", "no"),
            ),
            BigipPropertySpec(
                name="network", value_type="string", in_sections=("dynamic-signatures",)
            ),
            BigipPropertySpec(
                name="detection",
                value_type="enum",
                in_sections=("network",),
                enum_values=("disabled", "enabled", "learn-only"),
            ),
            BigipPropertySpec(
                name="mitigation",
                value_type="enum",
                in_sections=("network",),
                allow_none=True,
                enum_values=("none", "low", "medium", "high", "manual-multiplier"),
            ),
            BigipPropertySpec(
                name="scrubber-advertisement-period", value_type="integer", in_sections=("network",)
            ),
            BigipPropertySpec(
                name="scrubber-category", value_type="string", in_sections=("network",)
            ),
            BigipPropertySpec(
                name="scrubber-enable",
                value_type="enum",
                in_sections=("network",),
                enum_values=("yes", "no"),
            ),
            BigipPropertySpec(name="dns", value_type="string", in_sections=("dynamic-signatures",)),
            BigipPropertySpec(
                name="detection",
                value_type="enum",
                in_sections=("dns",),
                enum_values=("disabled", "enabled", "learn-only"),
            ),
            BigipPropertySpec(
                name="mitigation",
                value_type="enum",
                in_sections=("dns",),
                allow_none=True,
                enum_values=("none", "low", "medium", "high", "manual-multiplier"),
            ),
            BigipPropertySpec(name="dns-dos-mitigation-percentage", value_type="integer"),
            BigipPropertySpec(name="log-publisher", value_type="string"),
            BigipPropertySpec(name="network-dos-mitigation-percentage", value_type="integer"),
            BigipPropertySpec(name="sip-dos-mitigation-percentage", value_type="integer"),
            BigipPropertySpec(
                name="syn-cookie-dsr-flow-reset-by",
                value_type="enum",
                allow_none=True,
                enum_values=("bigip", "client", "none"),
            ),
            BigipPropertySpec(
                name="syn-cookie-whitelist", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
            BigipPropertySpec(name="query-valid-domain", value_type="string"),
        ),
    )

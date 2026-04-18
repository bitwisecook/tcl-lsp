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
            "security_dos_profile",
            module="security",
            object_types=("dos profile",),
        ),
        header_types=(("security", "dos profile"),),
        properties=(
            BigipPropertySpec(
                name="application",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="bot-defense", value_type="string", in_sections=("application",)
            ),
            BigipPropertySpec(
                name="collect-stats",
                value_type="enum",
                in_sections=("bot-defense",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="cross-domain-requests",
                value_type="enum",
                in_sections=("bot-defense",),
                enum_values=("allow-all", "validate-bulk", "validate-upon-request"),
            ),
            BigipPropertySpec(
                name="external-domains",
                value_type="enum",
                in_sections=("bot-defense",),
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="grace-period", value_type="integer", in_sections=("application",)
            ),
            BigipPropertySpec(
                name="mode",
                value_type="enum",
                in_sections=("application",),
                enum_values=("always", "disabled", "during-attacks"),
            ),
            BigipPropertySpec(
                name="site-domains",
                value_type="enum",
                in_sections=("application",),
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="url-whitelist",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="browser-legit-enabled", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="browser-legit-captcha", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="bot-signatures", value_type="string"),
            BigipPropertySpec(
                name="categories",
                value_type="enum",
                in_sections=("bot-signatures",),
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="action", value_type="string", in_sections=("categories",)),
            BigipPropertySpec(
                name="check",
                value_type="enum",
                in_sections=("bot-signatures",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="disabled-signatures",
                value_type="enum",
                in_sections=("bot-signatures",),
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="captcha-response", value_type="string"),
            BigipPropertySpec(
                name="failure", value_type="string", in_sections=("captcha-response",)
            ),
            BigipPropertySpec(name="body", value_type="string", in_sections=("failure",)),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                in_sections=("failure",),
                enum_values=("custom", "default"),
            ),
            BigipPropertySpec(name="first", value_type="string", in_sections=("captcha-response",)),
            BigipPropertySpec(name="body", value_type="string", in_sections=("first",)),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                in_sections=("first",),
                enum_values=("custom", "default"),
            ),
            BigipPropertySpec(
                name="geolocations",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="heavy-urls", value_type="string"),
            BigipPropertySpec(
                name="automatic-detection",
                value_type="enum",
                in_sections=("heavy-urls",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="exclude",
                value_type="enum",
                in_sections=("heavy-urls",),
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="include",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="include-list",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="latency-threshold", value_type="integer"),
            BigipPropertySpec(
                name="protection", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="ip-whitelist",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="stress-based", value_type="string"),
            BigipPropertySpec(
                name="de-escalation-period", value_type="integer", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="escalation-period", value_type="integer", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="geo-captcha-challenge",
                value_type="enum",
                in_sections=("stress-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="geo-client-side-defense",
                value_type="enum",
                in_sections=("stress-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="geo-minimum-share", value_type="integer", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="geo-rate-limiting",
                value_type="enum",
                in_sections=("stress-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="geo-request-blocking-mode",
                value_type="enum",
                in_sections=("stress-based",),
                enum_values=("block-all", "rate-limit"),
            ),
            BigipPropertySpec(
                name="geo-share-increase-rate", value_type="integer", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="geo-maximum-auto-tps", value_type="integer", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="geo-minimum-auto-tps", value_type="integer", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="ip-captcha-challenge",
                value_type="enum",
                in_sections=("stress-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="ip-client-side-defense",
                value_type="enum",
                in_sections=("stress-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="ip-maximum-tps", value_type="integer", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="ip-minimum-tps", value_type="integer", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="ip-rate-limiting",
                value_type="enum",
                in_sections=("stress-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="ip-request-blocking-mode",
                value_type="enum",
                in_sections=("stress-based",),
                enum_values=("block-all", "rate-limit"),
            ),
            BigipPropertySpec(
                name="ip-tps-increase-rate", value_type="integer", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="ip-maximum-auto-tps", value_type="integer", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="ip-minimum-auto-tps", value_type="integer", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="mode",
                value_type="enum",
                in_sections=("stress-based",),
                enum_values=("off", "transparent", "blocking"),
            ),
            BigipPropertySpec(
                name="thresholds-mode",
                value_type="enum",
                in_sections=("stress-based",),
                enum_values=("manual", "automatic"),
            ),
            BigipPropertySpec(
                name="site-captcha-challenge",
                value_type="enum",
                in_sections=("stress-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="site-client-side-defense",
                value_type="enum",
                in_sections=("stress-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="site-maximum-tps", value_type="integer", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="site-minimum-tps", value_type="integer", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="site-rate-limiting",
                value_type="enum",
                in_sections=("stress-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="site-tps-increase-rate", value_type="integer", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="site-maximum-auto-tps", value_type="integer", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="site-minimum-auto-tps", value_type="integer", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="static-url-mitigation",
                value_type="enum",
                in_sections=("stress-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="url-captcha-challenge",
                value_type="enum",
                in_sections=("stress-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="url-client-side-defense",
                value_type="enum",
                in_sections=("stress-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="url-maximum-tps", value_type="integer", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="url-minimum-tps", value_type="integer", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="url-rate-limiting",
                value_type="enum",
                in_sections=("stress-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="url-tps-increase-rate", value_type="integer", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="url-maximum-auto-tps", value_type="integer", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="url-minimum-auto-tps", value_type="integer", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="url-enable-heavy",
                value_type="enum",
                in_sections=("stress-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="device-captcha-challenge",
                value_type="enum",
                in_sections=("stress-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="device-client-side-defense",
                value_type="enum",
                in_sections=("stress-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="device-maximum-tps", value_type="integer", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="device-minimum-tps", value_type="integer", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="device-rate-limiting",
                value_type="enum",
                in_sections=("stress-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="device-request-blocking-mode",
                value_type="enum",
                in_sections=("stress-based",),
                enum_values=("block-all", "rate-limit"),
            ),
            BigipPropertySpec(
                name="device-tps-increase-rate", value_type="integer", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="device-maximum-auto-tps", value_type="integer", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="device-minimum-auto-tps", value_type="integer", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="behavioral", value_type="string", in_sections=("stress-based",)
            ),
            BigipPropertySpec(
                name="dos-detection",
                value_type="enum",
                in_sections=("behavioral",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="mitigation-mode",
                value_type="enum",
                in_sections=("behavioral",),
                allow_none=True,
                enum_values=("none", "conservative", "standard", "aggressive"),
            ),
            BigipPropertySpec(
                name="signatures",
                value_type="enum",
                in_sections=("behavioral",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="signatures-approved-only",
                value_type="enum",
                in_sections=("behavioral",),
                enum_values=("disabled",),
            ),
            BigipPropertySpec(
                name="accelerated-signatures",
                value_type="enum",
                in_sections=("behavioral",),
                enum_values=("enables", "disabled"),
            ),
            BigipPropertySpec(
                name="tls-signatures",
                value_type="enum",
                in_sections=("behavioral",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="tls-fp",
                value_type="enum",
                in_sections=("behavioral",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="tcp-dump", value_type="string"),
            BigipPropertySpec(
                name="maximum-duration", value_type="integer", in_sections=("tcp-dump",)
            ),
            BigipPropertySpec(name="maximum-size", value_type="integer", in_sections=("tcp-dump",)),
            BigipPropertySpec(
                name="record-traffic",
                value_type="enum",
                in_sections=("tcp-dump",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="repetition-interval", value_type="integer", in_sections=("tcp-dump",)
            ),
            BigipPropertySpec(name="tps-based", value_type="string"),
            BigipPropertySpec(
                name="de-escalation-period", value_type="integer", in_sections=("tps-based",)
            ),
            BigipPropertySpec(
                name="escalation-period", value_type="integer", in_sections=("tps-based",)
            ),
            BigipPropertySpec(
                name="geo-captcha-challenge",
                value_type="enum",
                in_sections=("tps-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="geo-client-side-defense",
                value_type="enum",
                in_sections=("tps-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="geo-minimum-share", value_type="integer", in_sections=("tps-based",)
            ),
            BigipPropertySpec(
                name="geo-rate-limiting",
                value_type="enum",
                in_sections=("tps-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="geo-request-blocking-mode",
                value_type="enum",
                in_sections=("tps-based",),
                enum_values=("block-all", "rate-limit"),
            ),
            BigipPropertySpec(
                name="geo-share-increase-rate", value_type="integer", in_sections=("tps-based",)
            ),
            BigipPropertySpec(
                name="ip-captcha-challenge",
                value_type="enum",
                in_sections=("tps-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="ip-client-side-defense",
                value_type="enum",
                in_sections=("tps-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="ip-maximum-tps", value_type="integer", in_sections=("tps-based",)
            ),
            BigipPropertySpec(
                name="ip-minimum-tps", value_type="integer", in_sections=("tps-based",)
            ),
            BigipPropertySpec(
                name="ip-rate-limiting",
                value_type="enum",
                in_sections=("tps-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="ip-request-blocking-mode",
                value_type="enum",
                in_sections=("tps-based",),
                enum_values=("block-all", "rate-limit"),
            ),
            BigipPropertySpec(
                name="ip-tps-increase-rate", value_type="integer", in_sections=("tps-based",)
            ),
            BigipPropertySpec(
                name="ip-maximum-auto-tps", value_type="integer", in_sections=("tps-based",)
            ),
            BigipPropertySpec(
                name="ip-minimum-auto-tps", value_type="integer", in_sections=("tps-based",)
            ),
            BigipPropertySpec(
                name="mode",
                value_type="enum",
                in_sections=("tps-based",),
                enum_values=("off", "transparent", "blocking"),
            ),
            BigipPropertySpec(
                name="thresholds-mode",
                value_type="enum",
                in_sections=("tps-based",),
                enum_values=("manual", "automatic"),
            ),
            BigipPropertySpec(
                name="site-captcha-challenge",
                value_type="enum",
                in_sections=("tps-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="site-client-side-defense",
                value_type="enum",
                in_sections=("tps-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="site-maximum-tps", value_type="integer", in_sections=("tps-based",)
            ),
            BigipPropertySpec(
                name="site-minimum-tps", value_type="integer", in_sections=("tps-based",)
            ),
            BigipPropertySpec(
                name="site-rate-limiting",
                value_type="enum",
                in_sections=("tps-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="site-tps-increase-rate", value_type="integer", in_sections=("tps-based",)
            ),
            BigipPropertySpec(
                name="site-maximum-auto-tps", value_type="integer", in_sections=("tps-based",)
            ),
            BigipPropertySpec(
                name="site-minimum-auto-tps", value_type="integer", in_sections=("tps-based",)
            ),
            BigipPropertySpec(
                name="static-url-mitigation",
                value_type="enum",
                in_sections=("tps-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="url-captcha-challenge",
                value_type="enum",
                in_sections=("tps-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="url-client-side-defense",
                value_type="enum",
                in_sections=("tps-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="url-maximum-tps", value_type="integer", in_sections=("tps-based",)
            ),
            BigipPropertySpec(
                name="url-minimum-tps", value_type="integer", in_sections=("tps-based",)
            ),
            BigipPropertySpec(
                name="url-rate-limiting",
                value_type="enum",
                in_sections=("tps-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="url-tps-increase-rate", value_type="integer", in_sections=("tps-based",)
            ),
            BigipPropertySpec(
                name="url-maximum-auto-tps", value_type="integer", in_sections=("tps-based",)
            ),
            BigipPropertySpec(
                name="url-minimum-auto-tps", value_type="integer", in_sections=("tps-based",)
            ),
            BigipPropertySpec(
                name="url-enable-heavy",
                value_type="enum",
                in_sections=("tps-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="device-captcha-challenge",
                value_type="enum",
                in_sections=("tps-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="device-client-side-defense",
                value_type="enum",
                in_sections=("tps-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="device-maximum-tps", value_type="integer", in_sections=("tps-based",)
            ),
            BigipPropertySpec(
                name="device-minimum-tps", value_type="integer", in_sections=("tps-based",)
            ),
            BigipPropertySpec(
                name="device-rate-limiting",
                value_type="enum",
                in_sections=("tps-based",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="device-request-blocking-mode",
                value_type="enum",
                in_sections=("tps-based",),
                enum_values=("block-all", "rate-limit"),
            ),
            BigipPropertySpec(
                name="device-tps-increase-rate", value_type="integer", in_sections=("tps-based",)
            ),
            BigipPropertySpec(
                name="device-maximum-auto-tps", value_type="integer", in_sections=("tps-based",)
            ),
            BigipPropertySpec(
                name="device-minimum-auto-tps", value_type="integer", in_sections=("tps-based",)
            ),
            BigipPropertySpec(
                name="trigger-irule", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="single-page-application",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="scrubbing-enable", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="scrubbing-duration-sec", value_type="integer"),
            BigipPropertySpec(
                name="rtbh-enable", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="rtbh-duration-sec", value_type="integer"),
            BigipPropertySpec(name="fastl4-acceleration-profile", value_type="string"),
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
                enum_values=("detect-only", "disabled", "learn-only", "mitigate"),
            ),
            BigipPropertySpec(
                name="threshold-mode",
                value_type="enum",
                in_sections=("name",),
                enum_values=("fully-automatic", "manual", "stress-based-mitigation"),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="dos-network",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="dynamic-signatures", value_type="string", in_sections=("dos-network",)
            ),
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
                enum_values=("none", "low", "medium", "high", "manual-multiplier"),
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
                name="multiplier-mitigation-percentage",
                value_type="integer",
                in_sections=("dos-network",),
            ),
            BigipPropertySpec(
                name="network-attack-vector",
                value_type="enum",
                in_sections=("dos-network",),
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="attack-type", value_type="string", in_sections=("network-attack-vector",)
            ),
            BigipPropertySpec(
                name="icmpv4-flood", value_type="string", in_sections=("network-attack-vector",)
            ),
            BigipPropertySpec(
                name="ip-opt-frames", value_type="string", in_sections=("network-attack-vector",)
            ),
            BigipPropertySpec(
                name="non-tcp-connection",
                value_type="string",
                in_sections=("network-attack-vector",),
            ),
            BigipPropertySpec(
                name="tcp-half-open", value_type="string", in_sections=("network-attack-vector",)
            ),
            BigipPropertySpec(
                name="tcp-rst-flood", value_type="string", in_sections=("network-attack-vector",)
            ),
            BigipPropertySpec(
                name="tcp-bad-urg", value_type="string", in_sections=("network-attack-vector",)
            ),
            BigipPropertySpec(
                name="udp-flood", value_type="string", in_sections=("network-attack-vector",)
            ),
            BigipPropertySpec(
                name="enforce",
                value_type="enum",
                in_sections=("network-attack-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="auto-blacklisting",
                value_type="enum",
                in_sections=("network-attack-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="auto-threshold",
                value_type="enum",
                in_sections=("network-attack-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="allow-upstream-scrubbing",
                value_type="enum",
                in_sections=("network-attack-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="attacked-dst",
                value_type="enum",
                in_sections=("network-attack-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="auto-scrubbing",
                value_type="enum",
                in_sections=("network-attack-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="bad-actor",
                value_type="enum",
                in_sections=("network-attack-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="blacklist-detection-seconds",
                value_type="integer",
                in_sections=("network-attack-vector",),
            ),
            BigipPropertySpec(
                name="blacklist-duration",
                value_type="integer",
                in_sections=("network-attack-vector",),
            ),
            BigipPropertySpec(
                name="blacklist-category",
                value_type="string",
                in_sections=("network-attack-vector",),
            ),
            BigipPropertySpec(
                name="multiplier-mitigation-percentage",
                value_type="integer",
                in_sections=("network-attack-vector",),
            ),
            BigipPropertySpec(
                name="per-source-ip-detection-pps",
                value_type="integer",
                in_sections=("network-attack-vector",),
            ),
            BigipPropertySpec(
                name="per-source-ip-limit-pps",
                value_type="integer",
                in_sections=("network-attack-vector",),
            ),
            BigipPropertySpec(
                name="per-dst-ip-detection-pps",
                value_type="integer",
                in_sections=("network-attack-vector",),
            ),
            BigipPropertySpec(
                name="per-dst-ip-limit-pps",
                value_type="integer",
                in_sections=("network-attack-vector",),
            ),
            BigipPropertySpec(
                name="scrubbing-category",
                value_type="boolean",
                in_sections=("network-attack-vector",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="scrubbing-detection-seconds",
                value_type="integer",
                in_sections=("network-attack-vector",),
            ),
            BigipPropertySpec(
                name="scrubbing-duration",
                value_type="integer",
                in_sections=("network-attack-vector",),
            ),
            BigipPropertySpec(
                name="rate-increase", value_type="integer", in_sections=("network-attack-vector",)
            ),
            BigipPropertySpec(
                name="rate-limit", value_type="integer", in_sections=("network-attack-vector",)
            ),
            BigipPropertySpec(
                name="rate-threshold", value_type="integer", in_sections=("network-attack-vector",)
            ),
            BigipPropertySpec(
                name="packet-types", value_type="string", in_sections=("network-attack-vector",)
            ),
            BigipPropertySpec(
                name="tcp-synack", value_type="string", in_sections=("network-attack-vector",)
            ),
            BigipPropertySpec(
                name="dns-query-a", value_type="string", in_sections=("network-attack-vector",)
            ),
            BigipPropertySpec(
                name="dns-query-cname", value_type="string", in_sections=("network-attack-vector",)
            ),
            BigipPropertySpec(
                name="dns-query-srv", value_type="string", in_sections=("network-attack-vector",)
            ),
            BigipPropertySpec(
                name="sip-method-prack", value_type="string", in_sections=("network-attack-vector",)
            ),
            BigipPropertySpec(
                name="sip-method-invite",
                value_type="boolean",
                in_sections=("network-attack-vector",),
            ),
            BigipPropertySpec(
                name="sip-method-publish",
                value_type="string",
                in_sections=("network-attack-vector",),
            ),
            BigipPropertySpec(
                name="state",
                value_type="enum",
                in_sections=("network-attack-vector",),
                enum_values=("disabled", "learn-only", "detect-only", "mitigate"),
            ),
            BigipPropertySpec(
                name="suspicious",
                value_type="enum",
                in_sections=("network-attack-vector",),
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="threshold-mode",
                value_type="enum",
                in_sections=("network-attack-vector",),
                enum_values=("manual", "stress-based-mitigation", "fully-automatic"),
            ),
            BigipPropertySpec(
                name="protocol-dns",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="dns-query-vector",
                value_type="enum",
                in_sections=("protocol-dns",),
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="query-type", value_type="string", in_sections=("dns-query-vector",)
            ),
            BigipPropertySpec(name="other", value_type="string", in_sections=("dns-query-vector",)),
            BigipPropertySpec(
                name="enforce",
                value_type="enum",
                in_sections=("dns-query-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="auto-blacklisting",
                value_type="enum",
                in_sections=("dns-query-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="auto-threshold",
                value_type="enum",
                in_sections=("dns-query-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="allow-upstream-scrubbing",
                value_type="enum",
                in_sections=("dns-query-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="attacked-dst",
                value_type="enum",
                in_sections=("dns-query-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="auto-scrubbing",
                value_type="enum",
                in_sections=("dns-query-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="bad-actor",
                value_type="enum",
                in_sections=("dns-query-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="blacklist-detection-seconds",
                value_type="integer",
                in_sections=("dns-query-vector",),
            ),
            BigipPropertySpec(
                name="blacklist-duration", value_type="integer", in_sections=("dns-query-vector",)
            ),
            BigipPropertySpec(
                name="blacklist-category", value_type="string", in_sections=("dns-query-vector",)
            ),
            BigipPropertySpec(
                name="multiplier-mitigation-percentage",
                value_type="integer",
                in_sections=("dns-query-vector",),
            ),
            BigipPropertySpec(
                name="per-source-ip-detection-pps",
                value_type="integer",
                in_sections=("dns-query-vector",),
            ),
            BigipPropertySpec(
                name="per-source-ip-limit-pps",
                value_type="integer",
                in_sections=("dns-query-vector",),
            ),
            BigipPropertySpec(
                name="per-dst-ip-detection-pps",
                value_type="integer",
                in_sections=("dns-query-vector",),
            ),
            BigipPropertySpec(
                name="per-dst-ip-limit-pps", value_type="integer", in_sections=("dns-query-vector",)
            ),
            BigipPropertySpec(
                name="scrubbing-category",
                value_type="boolean",
                in_sections=("dns-query-vector",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="scrubbing-detection-seconds",
                value_type="integer",
                in_sections=("dns-query-vector",),
            ),
            BigipPropertySpec(
                name="scrubbing-duration", value_type="integer", in_sections=("dns-query-vector",)
            ),
            BigipPropertySpec(
                name="rate-increase", value_type="integer", in_sections=("dns-query-vector",)
            ),
            BigipPropertySpec(
                name="rate-limit", value_type="integer", in_sections=("dns-query-vector",)
            ),
            BigipPropertySpec(
                name="rate-threshold", value_type="integer", in_sections=("dns-query-vector",)
            ),
            BigipPropertySpec(
                name="state",
                value_type="enum",
                in_sections=("dns-query-vector",),
                enum_values=("disabled", "learn-only", "detect-only", "mitigate"),
            ),
            BigipPropertySpec(
                name="suspicious",
                value_type="enum",
                in_sections=("dns-query-vector",),
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="threshold-mode",
                value_type="enum",
                in_sections=("dns-query-vector",),
                enum_values=("manual", "stress-based-mitigation", "fully-automatic"),
            ),
            BigipPropertySpec(
                name="valid-domains",
                value_type="enum",
                in_sections=("dns-query-vector",),
                allow_none=True,
                enum_values=("none", "add", "delete"),
            ),
            BigipPropertySpec(
                name="multiplier-mitigation-percentage",
                value_type="integer",
                in_sections=("protocol-dns",),
            ),
            BigipPropertySpec(
                name="prot-err-attack-detection",
                value_type="integer",
                in_sections=("protocol-dns",),
            ),
            BigipPropertySpec(
                name="prot-err-atck-rate-incr", value_type="integer", in_sections=("protocol-dns",)
            ),
            BigipPropertySpec(
                name="protocol-sip",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="multiplier-mitigation-percentage",
                value_type="integer",
                in_sections=("protocol-sip",),
            ),
            BigipPropertySpec(
                name="prot-err-atck-rate-increase",
                value_type="integer",
                in_sections=("protocol-sip",),
            ),
            BigipPropertySpec(
                name="prot-err-atck-rate-threshold",
                value_type="integer",
                in_sections=("protocol-sip",),
            ),
            BigipPropertySpec(
                name="prot-err-attack-detection",
                value_type="integer",
                in_sections=("protocol-sip",),
            ),
            BigipPropertySpec(
                name="sip-attack-vector",
                value_type="enum",
                in_sections=("protocol-sip",),
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="type", value_type="string", in_sections=("sip-attack-vector",)),
            BigipPropertySpec(
                name="enforce",
                value_type="enum",
                in_sections=("sip-attack-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="auto-blacklisting",
                value_type="enum",
                in_sections=("sip-attack-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="auto-threshold",
                value_type="enum",
                in_sections=("sip-attack-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="allow-upstream-scrubbing",
                value_type="enum",
                in_sections=("sip-attack-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="attacked-dst",
                value_type="enum",
                in_sections=("sip-attack-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="auto-scrubbing",
                value_type="enum",
                in_sections=("sip-attack-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="bad-actor",
                value_type="enum",
                in_sections=("sip-attack-vector",),
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="blacklist-detection-seconds",
                value_type="integer",
                in_sections=("sip-attack-vector",),
            ),
            BigipPropertySpec(
                name="blacklist-duration", value_type="integer", in_sections=("sip-attack-vector",)
            ),
            BigipPropertySpec(
                name="blacklist-category", value_type="string", in_sections=("sip-attack-vector",)
            ),
            BigipPropertySpec(
                name="multiplier-mitigation-percentage",
                value_type="integer",
                in_sections=("sip-attack-vector",),
            ),
            BigipPropertySpec(
                name="per-source-ip-detection-pps",
                value_type="integer",
                in_sections=("sip-attack-vector",),
            ),
            BigipPropertySpec(
                name="per-source-ip-limit-pps",
                value_type="integer",
                in_sections=("sip-attack-vector",),
            ),
            BigipPropertySpec(
                name="per-dst-ip-detection-pps",
                value_type="integer",
                in_sections=("sip-attack-vector",),
            ),
            BigipPropertySpec(
                name="per-dst-ip-limit-pps",
                value_type="integer",
                in_sections=("sip-attack-vector",),
            ),
            BigipPropertySpec(
                name="scrubbing-category",
                value_type="boolean",
                in_sections=("sip-attack-vector",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="scrubbing-detection-seconds",
                value_type="integer",
                in_sections=("sip-attack-vector",),
            ),
            BigipPropertySpec(
                name="scrubbing-duration", value_type="integer", in_sections=("sip-attack-vector",)
            ),
            BigipPropertySpec(
                name="rate-increase", value_type="integer", in_sections=("sip-attack-vector",)
            ),
            BigipPropertySpec(
                name="rate-limit", value_type="integer", in_sections=("sip-attack-vector",)
            ),
            BigipPropertySpec(
                name="rate-threshold", value_type="integer", in_sections=("sip-attack-vector",)
            ),
            BigipPropertySpec(
                name="state",
                value_type="enum",
                in_sections=("sip-attack-vector",),
                enum_values=("disabled", "learn-only", "detect-only", "mitigate"),
            ),
            BigipPropertySpec(
                name="suspicious",
                value_type="enum",
                in_sections=("sip-attack-vector",),
                enum_values=("false", "true"),
            ),
            BigipPropertySpec(
                name="threshold-mode",
                value_type="enum",
                in_sections=("sip-attack-vector",),
                enum_values=(
                    "manual",
                    "manual-multiplier-mitigation",
                    "stress-based-mitigation",
                    "fully-automatic",
                ),
            ),
            BigipPropertySpec(name="whitelist", value_type="string"),
            BigipPropertySpec(name="http-whitelist", value_type="string"),
            BigipPropertySpec(name="reset-stats", value_type="list", repeated=True),
        ),
    )

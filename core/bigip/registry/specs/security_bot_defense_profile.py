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
            "security_bot_defense_profile",
            module="security",
            object_types=("bot-defense profile",),
        ),
        header_types=(("security", "bot-defense profile"),),
        properties=(
            BigipPropertySpec(
                name="api-access-strict-mitigation",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="anomaly-category-overrides",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("anomaly-category-overrides",),
                allow_none=True,
                enum_values=(
                    "alarm",
                    "block",
                    "captcha",
                    "none",
                    "tcp-reset",
                    "redirect-to-pool",
                    "honeypot-page",
                ),
            ),
            BigipPropertySpec(
                name="anomaly-overrides",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("anomaly-overrides",),
                allow_none=True,
                enum_values=(
                    "alarm",
                    "block",
                    "captcha",
                    "none",
                    "tcp-reset",
                    "redirect-to-pool",
                    "honeypot-page",
                ),
            ),
            BigipPropertySpec(name="blocking-page", value_type="string"),
            BigipPropertySpec(name="body", value_type="string", in_sections=("blocking-page",)),
            BigipPropertySpec(name="headers", value_type="string", in_sections=("blocking-page",)),
            BigipPropertySpec(
                name="status-code", value_type="integer", in_sections=("blocking-page",)
            ),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                in_sections=("blocking-page",),
                enum_values=("custom", "default"),
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
                name="class-overrides",
                value_type="enum",
                in_sections=("captcha-response",),
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="mitigation", value_type="string", in_sections=("class-overrides",)
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("mitigation",),
                allow_none=True,
                enum_values=(
                    "alarm",
                    "block",
                    "captcha",
                    "none",
                    "rate-limit",
                    "tcp-reset",
                    "redirect-to-pool",
                    "honeypot-page",
                ),
            ),
            BigipPropertySpec(
                name="rate-limit-tps", value_type="integer", in_sections=("mitigation",)
            ),
            BigipPropertySpec(
                name="verification", value_type="string", in_sections=("class-overrides",)
            ),
            BigipPropertySpec(
                name="action", value_type="boolean", in_sections=("verification",), allow_none=True
            ),
            BigipPropertySpec(
                name="cross-domain-requests",
                value_type="enum",
                enum_values=("allow-all", "validate-bulk", "validate-upon-request"),
            ),
            BigipPropertySpec(name="defaults-from", value_type="string"),
            BigipPropertySpec(name="description", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="deviceid-mode",
                value_type="enum",
                allow_none=True,
                enum_values=("generate-after-access", "generate-before-access", "none"),
            ),
            BigipPropertySpec(
                name="dos-attack-strict-mitigation",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="enforcement-mode", value_type="enum", enum_values=("blocking", "transparent")
            ),
            BigipPropertySpec(name="enforcement-readiness-period", value_type="integer"),
            BigipPropertySpec(
                name="external-domains",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="grace-period", value_type="integer"),
            BigipPropertySpec(
                name="micro-services",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("micro-services",),
                allow_none=True,
                enum_values=(
                    "alarm",
                    "block",
                    "captcha",
                    "none",
                    "tcp-reset",
                    "redirect-to-pool",
                    "honeypot-page",
                ),
            ),
            BigipPropertySpec(
                name="class-overrides",
                value_type="enum",
                in_sections=("micro-services",),
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="detection-threshold", value_type="integer"),
            BigipPropertySpec(name="detection-time", value_type="integer"),
            BigipPropertySpec(
                name="hostname",
                value_type="string",
                pattern="^(?=.{1,253}$)(?:[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?\\\\.)+[A-Za-z]{2,63}$",
            ),
            BigipPropertySpec(name="match-order", value_type="integer"),
            BigipPropertySpec(name="mitigation-time", value_type="integer"),
            BigipPropertySpec(name="type", value_type="string"),
            BigipPropertySpec(
                name="urls",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="match-order", value_type="integer", in_sections=("urls",)),
            BigipPropertySpec(name="url", value_type="string", in_sections=("urls",)),
            BigipPropertySpec(name="mobile-detection", value_type="string"),
            BigipPropertySpec(
                name="allow-android-rooted-device",
                value_type="enum",
                in_sections=("mobile-detection",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="allow-any-android-package",
                value_type="enum",
                in_sections=("mobile-detection",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="allow-any-ios-package",
                value_type="enum",
                in_sections=("mobile-detection",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="allow-emulators",
                value_type="enum",
                in_sections=("mobile-detection",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="allow-jailbroken-devices",
                value_type="enum",
                in_sections=("mobile-detection",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="android-publishers",
                value_type="enum",
                in_sections=("mobile-detection",),
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="block-debugger-enabled-device",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="client-side-challenge-mode", value_type="enum", enum_values=("cshui", "pass")
            ),
            BigipPropertySpec(
                name="ios-allowed-packages",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="signatures",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="perform-challenge-in-transparent",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="signature-category-overrides",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("signature-category-overrides",),
                allow_none=True,
                enum_values=(
                    "alarm",
                    "block",
                    "captcha",
                    "none",
                    "tcp-reset",
                    "redirect-to-pool",
                    "honeypot-page",
                ),
            ),
            BigipPropertySpec(
                name="signature-overrides",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="action",
                value_type="enum",
                in_sections=("signature-overrides",),
                allow_none=True,
                enum_values=(
                    "alarm",
                    "block",
                    "captcha",
                    "none",
                    "rate-limit",
                    "tcp-reset",
                    "redirect-to-pool",
                    "honeypot-page",
                ),
            ),
            BigipPropertySpec(
                name="rate-limit-tps", value_type="integer", in_sections=("signature-overrides",)
            ),
            BigipPropertySpec(
                name="signature-staging-upon-update",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="single-page-application",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="site-domains",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="staged-signatures",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="template", value_type="enum", enum_values=("balanced", "relaxed", "strict")
            ),
            BigipPropertySpec(
                name="whitelist",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="disable-mitigation",
                value_type="enum",
                in_sections=("whitelist",),
                enum_values=("no", "yes"),
            ),
            BigipPropertySpec(
                name="disable-verification",
                value_type="enum",
                in_sections=("whitelist",),
                enum_values=("no", "yes"),
            ),
            BigipPropertySpec(name="geolocation", value_type="string", in_sections=("whitelist",)),
            BigipPropertySpec(name="match-order", value_type="integer", in_sections=("whitelist",)),
            BigipPropertySpec(
                name="source-address",
                value_type="string",
                in_sections=("whitelist",),
                pattern="^(?:\\\\d{1,3}(?:\\\\.\\\\d{1,3}){3})(?:/\\\\d{1,2})?$",
            ),
            BigipPropertySpec(name="url", value_type="string", in_sections=("whitelist",)),
        ),
    )

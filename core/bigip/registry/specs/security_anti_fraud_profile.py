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
            "security_anti_fraud_profile",
            module="security",
            object_types=("anti-fraud profile",),
        ),
        header_types=(("security", "anti-fraud profile"),),
        properties=(
            BigipPropertySpec(
                name="alert-client-side-caching",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="alert-identifier", value_type="string"),
            BigipPropertySpec(name="alert-path", value_type="string"),
            BigipPropertySpec(
                name="alert-pool", value_type="reference", allow_none=True, references=("ltm_pool",)
            ),
            BigipPropertySpec(name="alert-publisher", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="alert-token-header", value_type="string"),
            BigipPropertySpec(name="app-layer-encryption", value_type="string"),
            BigipPropertySpec(
                name="fail-open",
                value_type="enum",
                in_sections=("app-layer-encryption",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="auto-transactions", value_type="string"),
            BigipPropertySpec(
                name="bot-score", value_type="integer", in_sections=("auto-transactions",)
            ),
            BigipPropertySpec(
                name="click-score", value_type="integer", in_sections=("auto-transactions",)
            ),
            BigipPropertySpec(
                name="integrity-fail-score",
                value_type="integer",
                in_sections=("auto-transactions",),
            ),
            BigipPropertySpec(
                name="min-mouse-move-count",
                value_type="integer",
                in_sections=("auto-transactions",),
            ),
            BigipPropertySpec(
                name="min-mouse-over-count",
                value_type="integer",
                in_sections=("auto-transactions",),
            ),
            BigipPropertySpec(
                name="min-report-score", value_type="integer", in_sections=("auto-transactions",)
            ),
            BigipPropertySpec(
                name="min-time-to-request", value_type="integer", in_sections=("auto-transactions",)
            ),
            BigipPropertySpec(
                name="not-human-score", value_type="integer", in_sections=("auto-transactions",)
            ),
            BigipPropertySpec(
                name="strong-integrity", value_type="string", in_sections=("auto-transactions",)
            ),
            BigipPropertySpec(
                name="hide-encrypted-parameters",
                value_type="enum",
                in_sections=("strong-integrity",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="parameter", value_type="string", in_sections=("strong-integrity",)
            ),
            BigipPropertySpec(
                name="tampered-cookie-score",
                value_type="integer",
                in_sections=("auto-transactions",),
            ),
            BigipPropertySpec(
                name="time-fail-score", value_type="integer", in_sections=("auto-transactions",)
            ),
            BigipPropertySpec(name="before-load-function", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="blocking-page", value_type="string"),
            BigipPropertySpec(
                name="response-body",
                value_type="boolean",
                in_sections=("blocking-page",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="response-headers", value_type="string", in_sections=("blocking-page",)
            ),
            BigipPropertySpec(
                name="cloud-service-pool",
                value_type="reference",
                allow_none=True,
                references=("ltm_pool",),
            ),
            BigipPropertySpec(name="config-location", value_type="string"),
            BigipPropertySpec(name="cookies", value_type="string"),
            BigipPropertySpec(
                name="application",
                value_type="enum",
                in_sections=("cookies",),
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="base-domain", value_type="string"),
            BigipPropertySpec(
                name="apply",
                value_type="enum",
                in_sections=("base-domain",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="exceptions",
                value_type="enum",
                in_sections=("base-domain",),
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="client-side", value_type="string"),
            BigipPropertySpec(name="client-side-lifetime", value_type="integer"),
            BigipPropertySpec(name="components-state", value_type="string"),
            BigipPropertySpec(name="components-state-lifetime", value_type="integer"),
            BigipPropertySpec(
                name="components-state-removal-protection",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="encryption-disabled", value_type="string"),
            BigipPropertySpec(name="encryption-disabled-lifetime", value_type="integer"),
            BigipPropertySpec(
                name="encryption-disabled-removal-protection",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="fingerprint", value_type="string"),
            BigipPropertySpec(name="fingerprint-lifetime", value_type="integer"),
            BigipPropertySpec(
                name="fingerprint-removal-protection",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="html-field-obfuscation", value_type="string"),
            BigipPropertySpec(name="html-field-obfuscation-lifetime", value_type="integer"),
            BigipPropertySpec(name="malware-forensic", value_type="string"),
            BigipPropertySpec(name="malware-forensic-lifetime", value_type="integer"),
            BigipPropertySpec(name="malware-guid", value_type="string"),
            BigipPropertySpec(name="malware-guid-lifetime", value_type="integer"),
            BigipPropertySpec(
                name="malware-guid-removal-protection",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="rules", value_type="string"),
            BigipPropertySpec(name="rules-lifetime", value_type="integer"),
            BigipPropertySpec(
                name="rules-removal-protection",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="secure-alert", value_type="string"),
            BigipPropertySpec(name="secure-alert-lifetime", value_type="integer"),
            BigipPropertySpec(
                name="secure-alert-removal-protection",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="secure-channel", value_type="string"),
            BigipPropertySpec(name="secure-channel-lifetime", value_type="integer"),
            BigipPropertySpec(
                name="secure-channel-removal-protection",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="secure-mode", value_type="enum", enum_values=("auto", "disabled", "enabled")
            ),
            BigipPropertySpec(name="transaction-data", value_type="string"),
            BigipPropertySpec(name="transaction-data-lifetime", value_type="integer"),
            BigipPropertySpec(name="user-inspection", value_type="string"),
            BigipPropertySpec(name="user-name", value_type="string"),
            BigipPropertySpec(name="user-name-lifetime", value_type="integer"),
            BigipPropertySpec(
                name="user-name-removal-protection",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="debug", value_type="string"),
            BigipPropertySpec(name="console-log", value_type="string", in_sections=("debug",)),
            BigipPropertySpec(
                name="client-ips",
                value_type="enum",
                in_sections=("console-log",),
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="user-agents",
                value_type="enum",
                in_sections=("debug",),
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="fingerprints",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="send-alert", value_type="string"),
            BigipPropertySpec(
                name="client-ips",
                value_type="enum",
                in_sections=("send-alert",),
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="user-agents",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="defaults-from", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="description", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="dummy-alert-html-maximum-length", value_type="integer"),
            BigipPropertySpec(
                name="encryption-staging-mode",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="collect",
                value_type="enum",
                in_sections=("fingerprint",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="location", value_type="string", in_sections=("fingerprint",)),
            BigipPropertySpec(name="forensic", value_type="string"),
            BigipPropertySpec(name="alert-path", value_type="string", in_sections=("forensic",)),
            BigipPropertySpec(
                name="client-domains",
                value_type="enum",
                in_sections=("forensic",),
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="cloud-config-path", value_type="string"),
            BigipPropertySpec(name="cloud-forensics-mode", value_type="integer"),
            BigipPropertySpec(name="cloud-remediation-mode", value_type="integer"),
            BigipPropertySpec(name="continue-element", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="exe-location", value_type="string"),
            BigipPropertySpec(name="html", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="self-post-location", value_type="string"),
            BigipPropertySpec(name="skip-element", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="skip-path", value_type="string"),
            BigipPropertySpec(
                name="geolocation", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="inject-main-javascript", value_type="string"),
            BigipPropertySpec(
                name="tag", value_type="string", in_sections=("inject-main-javascript",)
            ),
            BigipPropertySpec(name="javascript-grace-threshold", value_type="integer"),
            BigipPropertySpec(name="javascript-location", value_type="string"),
            BigipPropertySpec(name="javascript-removal-location", value_type="string"),
            BigipPropertySpec(name="local-syslog-publisher", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="malware", value_type="string"),
            BigipPropertySpec(
                name="allowed-domains",
                value_type="enum",
                in_sections=("malware",),
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="bait-check-generic", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="bait-location", value_type="string"),
            BigipPropertySpec(
                name="blacklist-words",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="detected-malware",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="name", value_type="string", in_sections=("detected-malware",)),
            BigipPropertySpec(
                name="baits",
                value_type="enum",
                in_sections=("name",),
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="name", value_type="string", in_sections=("baits",)),
            BigipPropertySpec(name="data-before", value_type="string", in_sections=("name",)),
            BigipPropertySpec(name="data-inject", value_type="string", in_sections=("name",)),
            BigipPropertySpec(name="trigger-url", value_type="string", in_sections=("name",)),
            BigipPropertySpec(name="name", value_type="string", in_sections=("trigger-url",)),
            BigipPropertySpec(
                name="position",
                value_type="enum",
                in_sections=("trigger-url",),
                enum_values=("alone", "any", "last"),
            ),
            BigipPropertySpec(
                name="blacklist-functions",
                value_type="enum",
                in_sections=("name",),
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="blacklist-js-words",
                value_type="enum",
                in_sections=("detected-malware",),
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="blacklist-urls",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="browser-cache", value_type="string"),
            BigipPropertySpec(
                name="blacklist-urls",
                value_type="enum",
                in_sections=("browser-cache",),
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="whitelist-urls",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="domain-availability", value_type="string"),
            BigipPropertySpec(
                name="blacklist-urls",
                value_type="enum",
                in_sections=("domain-availability",),
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="dom-signatures",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="name", value_type="string", in_sections=("dom-signatures",)),
            BigipPropertySpec(
                name="attribute-name", value_type="boolean", in_sections=("name",), allow_none=True
            ),
            BigipPropertySpec(name="hash-id", value_type="integer", in_sections=("name",)),
            BigipPropertySpec(
                name="html-tag", value_type="boolean", in_sections=("name",), allow_none=True
            ),
            BigipPropertySpec(
                name="match-type",
                value_type="enum",
                in_sections=("name",),
                enum_values=("contains", "is"),
            ),
            BigipPropertySpec(name="search-for", value_type="string", in_sections=("name",)),
            BigipPropertySpec(
                name="search-in",
                value_type="enum",
                in_sections=("name",),
                enum_values=("all", "attribute", "html", "js-global-variable", "text"),
            ),
            BigipPropertySpec(
                name="generic-whitelist-words",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="domain-availability-urls", value_type="boolean", allow_none=True
            ),
            BigipPropertySpec(
                name="external-sources-targets",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="flash-cookie-content", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="flash-cookie-location", value_type="string"),
            BigipPropertySpec(
                name="flash-cookies", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="inline-scripts-whitelist-signatures",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="removed-scripts", value_type="string"),
            BigipPropertySpec(
                name="blacklist-functions",
                value_type="enum",
                in_sections=("removed-scripts",),
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="whitelist-functions",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="same-domain-scripts-validation-header", value_type="string"),
            BigipPropertySpec(name="self-bait-header", value_type="string"),
            BigipPropertySpec(name="source-integrity-location", value_type="string"),
            BigipPropertySpec(name="web-rootkit", value_type="string"),
            BigipPropertySpec(
                name="blacklist-functions",
                value_type="enum",
                in_sections=("web-rootkit",),
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="mobilesafe", value_type="string"),
            BigipPropertySpec(
                name="alert-custom-config",
                value_type="boolean",
                in_sections=("mobilesafe",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="alert-threshold", value_type="integer", in_sections=("mobilesafe",)
            ),
            BigipPropertySpec(
                name="app-integrity", value_type="string", in_sections=("mobilesafe",)
            ),
            BigipPropertySpec(
                name="custom-config",
                value_type="boolean",
                in_sections=("app-integrity",),
                allow_none=True,
            ),
            BigipPropertySpec(name="android", value_type="string", in_sections=("app-integrity",)),
            BigipPropertySpec(name="score", value_type="integer", in_sections=("android",)),
            BigipPropertySpec(
                name="signature", value_type="boolean", in_sections=("android",), allow_none=True
            ),
            BigipPropertySpec(name="ios", value_type="string", in_sections=("app-integrity",)),
            BigipPropertySpec(
                name="hashes",
                value_type="enum",
                in_sections=("ios",),
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="value", value_type="string", in_sections=("hashes",)),
            BigipPropertySpec(
                name="version", value_type="boolean", in_sections=("value",), allow_none=True
            ),
            BigipPropertySpec(name="score", value_type="integer", in_sections=("ios",)),
            BigipPropertySpec(
                name="general-custom-config",
                value_type="boolean",
                in_sections=("mobilesafe",),
                allow_none=True,
            ),
            BigipPropertySpec(name="malware", value_type="string", in_sections=("mobilesafe",)),
            BigipPropertySpec(name="android", value_type="string", in_sections=("malware",)),
            BigipPropertySpec(
                name="custom-malware",
                value_type="enum",
                in_sections=("android",),
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="name", value_type="string", in_sections=("custom-malware",)),
            BigipPropertySpec(name="package", value_type="string", in_sections=("name",)),
            BigipPropertySpec(name="score", value_type="integer", in_sections=("name",)),
            BigipPropertySpec(
                name="custom-whitelist",
                value_type="enum",
                in_sections=("android",),
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="name", value_type="string", in_sections=("custom-whitelist",)),
            BigipPropertySpec(
                name="check-custom",
                value_type="enum",
                in_sections=("malware",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="check-generic",
                value_type="enum",
                in_sections=("malware",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="custom-config",
                value_type="boolean",
                in_sections=("malware",),
                allow_none=True,
            ),
            BigipPropertySpec(name="ios", value_type="string", in_sections=("malware",)),
            BigipPropertySpec(
                name="custom-malware",
                value_type="enum",
                in_sections=("ios",),
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="path", value_type="string", in_sections=("name",)),
            BigipPropertySpec(
                name="custom-whitelist",
                value_type="enum",
                in_sections=("ios",),
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="behaviour-analysis", value_type="string", in_sections=("malware",)
            ),
            BigipPropertySpec(
                name="score", value_type="integer", in_sections=("behaviour-analysis",)
            ),
            BigipPropertySpec(name="mitm", value_type="string", in_sections=("mobilesafe",)),
            BigipPropertySpec(
                name="certificate-custom-config",
                value_type="boolean",
                in_sections=("mitm",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="dns-custom-config",
                value_type="boolean",
                in_sections=("mitm",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="domains",
                value_type="enum",
                in_sections=("mitm",),
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="name", value_type="string", in_sections=("domains",)),
            BigipPropertySpec(name="dns", value_type="string", in_sections=("name",)),
            BigipPropertySpec(
                name="ip-ranges",
                value_type="enum",
                in_sections=("dns",),
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="spoofing-score", value_type="integer", in_sections=("name",)),
            BigipPropertySpec(name="certificate", value_type="string", in_sections=("domains",)),
            BigipPropertySpec(
                name="forging-score", value_type="integer", in_sections=("certificate",)
            ),
            BigipPropertySpec(name="hash", value_type="string", in_sections=("certificate",)),
            BigipPropertySpec(name="os-security", value_type="string"),
            BigipPropertySpec(name="android", value_type="string", in_sections=("os-security",)),
            BigipPropertySpec(
                name="untrusted-apps-score", value_type="integer", in_sections=("android",)
            ),
            BigipPropertySpec(
                name="versions",
                value_type="enum",
                in_sections=("android",),
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="priority", value_type="integer", in_sections=("versions",)),
            BigipPropertySpec(name="from", value_type="string", in_sections=("priority",)),
            BigipPropertySpec(name="score", value_type="integer", in_sections=("priority",)),
            BigipPropertySpec(name="to", value_type="string", in_sections=("priority",)),
            BigipPropertySpec(
                name="custom-config",
                value_type="boolean",
                in_sections=("os-security",),
                allow_none=True,
            ),
            BigipPropertySpec(name="ios", value_type="string", in_sections=("os-security",)),
            BigipPropertySpec(
                name="versions",
                value_type="enum",
                in_sections=("ios",),
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="rooting-jailbreak", value_type="string"),
            BigipPropertySpec(
                name="custom-config",
                value_type="boolean",
                in_sections=("rooting-jailbreak",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="jailbreak-score", value_type="integer", in_sections=("rooting-jailbreak",)
            ),
            BigipPropertySpec(
                name="rooting-score", value_type="integer", in_sections=("rooting-jailbreak",)
            ),
            BigipPropertySpec(name="phishing", value_type="string"),
            BigipPropertySpec(name="alert-path", value_type="string", in_sections=("phishing",)),
            BigipPropertySpec(
                name="allowed-elements",
                value_type="enum",
                in_sections=("phishing",),
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="allowed-referrers",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="application-css", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="application-css-locations",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="css-attribute-name", value_type="string"),
            BigipPropertySpec(name="css-location", value_type="string"),
            BigipPropertySpec(
                name="expiration-checks", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="image-location", value_type="string"),
            BigipPropertySpec(name="inject-css-element", value_type="string"),
            BigipPropertySpec(name="tag", value_type="string", in_sections=("inject-css-element",)),
            BigipPropertySpec(name="inject-css-link", value_type="string"),
            BigipPropertySpec(name="tag", value_type="string", in_sections=("inject-css-link",)),
            BigipPropertySpec(name="inject-inline-javascript", value_type="string"),
            BigipPropertySpec(
                name="tag", value_type="string", in_sections=("inject-inline-javascript",)
            ),
            BigipPropertySpec(
                name="protected-elements",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="referrer-checks", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="referrer-info-header", value_type="string"),
            BigipPropertySpec(name="risk-engine-path", value_type="string"),
            BigipPropertySpec(name="risk-engine-publisher", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="event", value_type="string", in_sections=("rules",)),
            BigipPropertySpec(
                name="generic-malware",
                value_type="reference",
                in_sections=("rules",),
                references=("auth_user",),
            ),
            BigipPropertySpec(
                name="server-side-missing-components", value_type="string", in_sections=("rules",)
            ),
            BigipPropertySpec(
                name="action",
                value_type="reference",
                in_sections=("server-side-missing-components",),
                enum_values=(
                    "block-user",
                    "forensic",
                    "inspection",
                    "redirect",
                    "remediation",
                    "route",
                    "web-service",
                ),
                references=("auth_user",),
            ),
            BigipPropertySpec(
                name="duration",
                value_type="integer",
                in_sections=("server-side-missing-components",),
            ),
            BigipPropertySpec(
                name="enforce-policy",
                value_type="enum",
                in_sections=("server-side-missing-components",),
                enum_values=("enforce", "time-limited", "unlimited"),
            ),
            BigipPropertySpec(
                name="min-score",
                value_type="integer",
                in_sections=("server-side-missing-components",),
            ),
            BigipPropertySpec(
                name="publisher",
                value_type="boolean",
                in_sections=("server-side-missing-components",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="payload",
                value_type="boolean",
                in_sections=("server-side-missing-components",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="pool",
                value_type="reference",
                in_sections=("server-side-missing-components",),
                allow_none=True,
                references=("ltm_pool",),
            ),
            BigipPropertySpec(
                name="url",
                value_type="boolean",
                in_sections=("server-side-missing-components",),
                allow_none=True,
            ),
            BigipPropertySpec(name="suggested-username-header", value_type="string"),
            BigipPropertySpec(
                name="trigger-irule", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="urls",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="name", value_type="string", in_sections=("urls",)),
            BigipPropertySpec(
                name="app-layer-encryption", value_type="string", in_sections=("name",)
            ),
            BigipPropertySpec(
                name="add-decoy-inputs",
                value_type="enum",
                in_sections=("app-layer-encryption",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="auto-complete-block",
                value_type="enum",
                in_sections=("app-layer-encryption",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="auto-complete-whitelist-functions",
                value_type="enum",
                in_sections=("app-layer-encryption",),
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="custom-encryption-function",
                value_type="boolean",
                in_sections=("name",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="fake-strokes",
                value_type="enum",
                in_sections=("name",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="full-ajax-encryption",
                value_type="enum",
                in_sections=("name",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="hide-password-revealer",
                value_type="enum",
                in_sections=("name",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="html-field-obfuscation",
                value_type="enum",
                in_sections=("name",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="real-time-encryption",
                value_type="enum",
                in_sections=("name",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="remove-element-ids",
                value_type="enum",
                in_sections=("name",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="remove-event-listeners",
                value_type="enum",
                in_sections=("name",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="stolen-creds",
                value_type="enum",
                in_sections=("name",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="substitute-value-function",
                value_type="boolean",
                in_sections=("name",),
                allow_none=True,
            ),
            BigipPropertySpec(name="auto-transactions", value_type="string", in_sections=("urls",)),
            BigipPropertySpec(
                name="attach-ajax-payload-to-alerts",
                value_type="enum",
                in_sections=("auto-transactions",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="browser",
                value_type="enum",
                in_sections=("auto-transactions",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="full-ajax-integrity",
                value_type="enum",
                in_sections=("auto-transactions",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="integrity-fail-max-score",
                value_type="integer",
                in_sections=("auto-transactions",),
            ),
            BigipPropertySpec(
                name="non-browser",
                value_type="enum",
                in_sections=("auto-transactions",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="strong-integrity-user-functions",
                value_type="enum",
                in_sections=("auto-transactions",),
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="submit-buttons",
                value_type="enum",
                in_sections=("urls",),
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="tampered-cookie-score", value_type="integer"),
            BigipPropertySpec(name="time-fail-score", value_type="integer"),
            BigipPropertySpec(
                name="custom-alerts",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="name", value_type="string", in_sections=("custom-alerts",)),
            BigipPropertySpec(
                name="attach-request-part",
                value_type="enum",
                in_sections=("name",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="component",
                value_type="enum",
                in_sections=("name",),
                enum_values=("auto-transactions", "malware", "mobilesafe", "phishing"),
            ),
            BigipPropertySpec(
                name="header-name", value_type="boolean", in_sections=("name",), allow_none=True
            ),
            BigipPropertySpec(
                name="malware-name", value_type="boolean", in_sections=("name",), allow_none=True
            ),
            BigipPropertySpec(
                name="message", value_type="boolean", in_sections=("name",), allow_none=True
            ),
            BigipPropertySpec(
                name="value", value_type="boolean", in_sections=("name",), allow_none=True
            ),
            BigipPropertySpec(
                name="destination-urls",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="fallback-to-base-url", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="include-query-string", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="inject-javascript", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="inject-javascript-removal", value_type="string"),
            BigipPropertySpec(
                name="tag", value_type="string", in_sections=("inject-javascript-removal",)
            ),
            BigipPropertySpec(name="login-response", value_type="string"),
            BigipPropertySpec(
                name="status-code",
                value_type="integer",
                in_sections=("login-response",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="domain-cookie",
                value_type="boolean",
                in_sections=("login-response",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="exclude-string",
                value_type="boolean",
                in_sections=("login-response",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="header",
                value_type="boolean",
                in_sections=("login-response",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="include-string",
                value_type="boolean",
                in_sections=("login-response",),
                allow_none=True,
            ),
            BigipPropertySpec(
                name="validation",
                value_type="enum",
                in_sections=("login-response",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="attach-html-to-alerts",
                value_type="enum",
                in_sections=("malware",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="auto-learn-form-tags",
                value_type="enum",
                in_sections=("malware",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="auto-learn-input-tags",
                value_type="enum",
                in_sections=("malware",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="auto-learn-script-tags",
                value_type="enum",
                in_sections=("malware",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="blocked-enter-key-detection",
                value_type="enum",
                in_sections=("malware",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="deferred-execution",
                value_type="enum",
                in_sections=("malware",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="domain-availability",
                value_type="enum",
                in_sections=("malware",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="enable-symbols",
                value_type="enum",
                in_sections=("malware",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="external-injection",
                value_type="enum",
                in_sections=("malware",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="generic-malware",
                value_type="enum",
                in_sections=("malware",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="manual-count-form-tags", value_type="integer", in_sections=("malware",)
            ),
            BigipPropertySpec(
                name="manual-count-input-tags", value_type="integer", in_sections=("malware",)
            ),
            BigipPropertySpec(
                name="manual-count-script-tags", value_type="integer", in_sections=("malware",)
            ),
            BigipPropertySpec(
                name="password-exfiltration-detection",
                value_type="enum",
                in_sections=("malware",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="rat-detection",
                value_type="enum",
                in_sections=("malware",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="removed-scripts-detection",
                value_type="enum",
                in_sections=("malware",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="same-domain-scripts-validation",
                value_type="enum",
                in_sections=("malware",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="self-bait",
                value_type="enum",
                in_sections=("malware",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="source-integrity",
                value_type="enum",
                in_sections=("malware",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="vbklip-detection",
                value_type="enum",
                in_sections=("malware",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="visibility-check",
                value_type="enum",
                in_sections=("malware",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="visibility-check-items",
                value_type="enum",
                in_sections=("malware",),
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="web-rootkit-detection", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="whitelist-dom-signatures",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="whitelist-words",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(
                name="mobilesafe-encryption", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="parameters",
                value_type="enum",
                allow_none=True,
                enum_values=("none", "add", "delete", "modify", "replace-all-with"),
            ),
            BigipPropertySpec(name="name", value_type="string", in_sections=("parameters",)),
            BigipPropertySpec(name="ajax-mapping", value_type="string", in_sections=("name",)),
            BigipPropertySpec(
                name="attach-to-vtoken-report",
                value_type="enum",
                in_sections=("name",),
                enum_values=("enabled", "disabled"),
                min_value=0,
                max_value=65535,
            ),
            BigipPropertySpec(
                name="check-integrity",
                value_type="enum",
                in_sections=("name",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="encrypt",
                value_type="enum",
                in_sections=("name",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="identify-as-username",
                value_type="enum",
                in_sections=("name",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="method", value_type="enum", in_sections=("name",), enum_values=("get", "post")
            ),
            BigipPropertySpec(
                name="mobilesafe-encrypt",
                value_type="enum",
                in_sections=("name",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="mobilesafe-entangle",
                value_type="enum",
                in_sections=("name",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="obfuscate",
                value_type="enum",
                in_sections=("name",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="priority", value_type="integer", in_sections=("name",)),
            BigipPropertySpec(
                name="protect-by-selector",
                value_type="enum",
                in_sections=("name",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="substitute-value",
                value_type="enum",
                in_sections=("name",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="type",
                value_type="enum",
                in_sections=("name",),
                enum_values=("explicit", "wildcard"),
            ),
            BigipPropertySpec(
                name="capture-users",
                value_type="enum",
                in_sections=("phishing",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="copy-detection",
                value_type="enum",
                in_sections=("phishing",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="css-protection",
                value_type="enum",
                in_sections=("phishing",),
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(
                name="field-types-to-send",
                value_type="enum",
                in_sections=("phishing",),
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="priority", value_type="integer"),
            BigipPropertySpec(name="type", value_type="enum", enum_values=("explicit", "wildcard")),
            BigipPropertySpec(
                name="users", value_type="enum", enum_values=("add", "delete", "modify")
            ),
            BigipPropertySpec(name="name", value_type="string", in_sections=("users",)),
            BigipPropertySpec(
                name="modes",
                value_type="enum",
                in_sections=("name",),
                enum_values=("add", "delete"),
            ),
            BigipPropertySpec(
                name="mode",
                value_type="enum",
                in_sections=("modes",),
                enum_values=("block", "forensic", "inspection", "remediation"),
            ),
            BigipPropertySpec(name="duration", value_type="integer", in_sections=("mode",)),
            BigipPropertySpec(
                name="enforce-policy",
                value_type="enum",
                in_sections=("mode",),
                enum_values=("enforce", "time-limited", "unlimited"),
            ),
            BigipPropertySpec(name="first-login-time", value_type="string", in_sections=("mode",)),
            BigipPropertySpec(
                name="whitelist-custom-alerts",
                value_type="enum",
                repeated=True,
                allow_none=True,
                enum_values=("none", "add", "delete", "replace-all-with"),
            ),
        ),
    )

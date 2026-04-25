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
            "gtm_global_settings_general",
            module="gtm",
            object_types=("global-settings general",),
        ),
        header_types=(("gtm", "global-settings general"),),
        properties=(
            BigipPropertySpec(
                name="allow-nxdomain-override", value_type="enum", enum_values=("enable", "disable")
            ),
            BigipPropertySpec(name="automatic-configuration-save-timeout", value_type="integer"),
            BigipPropertySpec(name="auto-discovery", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(name="auto-discovery-interval", value_type="integer"),
            BigipPropertySpec(
                name="cache-ldns-servers", value_type="enum", enum_values=("no", "yes")
            ),
            BigipPropertySpec(
                name="domain-name-check",
                value_type="enum",
                allow_none=True,
                enum_values=("allow-underscore", "none"),
            ),
            BigipPropertySpec(
                name="drain-persistent-requests", value_type="enum", enum_values=("no", "yes")
            ),
            BigipPropertySpec(
                name="forward-status", value_type="enum", enum_values=("enable", "disable")
            ),
            BigipPropertySpec(
                name="gtm-sets-recursion", value_type="enum", enum_values=("no", "yes")
            ),
            BigipPropertySpec(name="heartbeat-interval", value_type="integer"),
            BigipPropertySpec(
                name="ignore-ltm-rate-limit-modes", value_type="boolean", allow_none=True
            ),
            BigipPropertySpec(name="iquery-cipher-list", value_type="string"),
            BigipPropertySpec(
                name="iquery-crl-validation-depth",
                value_type="enum",
                enum_values=("full", "device"),
            ),
            BigipPropertySpec(name="iquery-minimum-tls-version", value_type="string"),
            BigipPropertySpec(
                name="iquery-reverify-on-crl-becoming-active",
                value_type="enum",
                enum_values=("no", "yes"),
            ),
            BigipPropertySpec(
                name="iquery-reverify-on-crl-expiring", value_type="enum", enum_values=("no", "yes")
            ),
            BigipPropertySpec(
                name="iquery-reverify-on-crl-file-update",
                value_type="enum",
                enum_values=("no", "yes"),
            ),
            BigipPropertySpec(
                name="iquery-use-expired-crls", value_type="enum", enum_values=("no", "yes")
            ),
            BigipPropertySpec(
                name="iquery-use-not-yet-active-crls", value_type="enum", enum_values=("no", "yes")
            ),
            BigipPropertySpec(
                name="iquery-use-revoked-certs",
                value_type="enum",
                enum_values=("never", "existing", "always"),
            ),
            BigipPropertySpec(
                name="monitor-disabled-objects", value_type="enum", enum_values=("no", "yes")
            ),
            BigipPropertySpec(name="nethsm-timeout", value_type="integer"),
            BigipPropertySpec(
                name="nsec3-types-bitmap-strict",
                value_type="enum",
                enum_values=("enable", "disable"),
            ),
            BigipPropertySpec(name="peer-leader", value_type="string"),
            BigipPropertySpec(
                name="send-wildcard-rrs", value_type="enum", enum_values=("enable", "disable")
            ),
            BigipPropertySpec(name="static-persist-cidr-ipv4", value_type="integer"),
            BigipPropertySpec(name="static-persist-cidr-ipv6", value_type="integer"),
            BigipPropertySpec(name="synchronization", value_type="enum", enum_values=("no", "yes")),
            BigipPropertySpec(name="synchronization-group-name", value_type="string"),
            BigipPropertySpec(name="synchronization-time-tolerance", value_type="integer"),
            BigipPropertySpec(name="synchronization-timeout", value_type="integer"),
            BigipPropertySpec(
                name="synchronize-zone-files", value_type="enum", enum_values=("no", "yes")
            ),
            BigipPropertySpec(name="synchronize-zone-files-timeout", value_type="integer"),
            BigipPropertySpec(
                name="topology-allow-zero-scores", value_type="enum", enum_values=("no", "yes")
            ),
            BigipPropertySpec(
                name="virtuals-depend-on-server-state", value_type="enum", enum_values=("no", "yes")
            ),
            BigipPropertySpec(name="wideip-zone-nameserver", value_type="string"),
        ),
    )

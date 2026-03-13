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
            "ltm_profile_fastl4",
            module="ltm",
            object_types=("profile fastl4",),
        ),
        header_types=(("ltm", "profile fastl4"),),
        properties=(
            BigipPropertySpec(
                name="defaults-from",
                value_type="reference",
                allow_none=True,
                references=("ltm_profile_fastl4",),
            ),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="hardware-syn-cookie", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="idle-timeout", value_type="integer"),
            BigipPropertySpec(name="ip-tos-to-client", value_type="integer"),
            BigipPropertySpec(name="ip-tos-to-server", value_type="integer"),
            BigipPropertySpec(name="keep-alive-interval", value_type="integer"),
            BigipPropertySpec(
                name="ip-df-mode", value_type="enum", enum_values=("preserve", "set", "clear")
            ),
            BigipPropertySpec(
                name="ip-ttl-mode",
                value_type="enum",
                enum_values=("proxy", "preserve", "decrement", "set"),
            ),
            BigipPropertySpec(name="ip-ttl-value", value_type="integer"),
            BigipPropertySpec(name="link-qos-to-client", value_type="integer"),
            BigipPropertySpec(name="link-qos-to-server", value_type="integer"),
            BigipPropertySpec(name="priority-to-client", value_type="integer"),
            BigipPropertySpec(name="priority-to-server", value_type="integer"),
            BigipPropertySpec(
                name="loose-close", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="loose-initialization", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="mss-override", value_type="integer"),
            BigipPropertySpec(
                name="pva-acceleration",
                value_type="enum",
                allow_none=True,
                enum_values=("full", "none", "partial", "dedicated"),
            ),
            BigipPropertySpec(name="pva-dynamic-client-packets", value_type="integer"),
            BigipPropertySpec(name="pva-dynamic-server-packets", value_type="integer"),
            BigipPropertySpec(
                name="pva-offload-dynamic", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="pva-offload-state", value_type="enum", enum_values=("embryonic", "establish")
            ),
            BigipPropertySpec(
                name="pva-offload-dynamic-priority",
                value_type="enum",
                enum_values=("enable", "disable"),
            ),
            BigipPropertySpec(
                name="pva-offload-initial-priority",
                value_type="enum",
                enum_values=("low", "medium", "high"),
            ),
            BigipPropertySpec(
                name="pva-flow-aging", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="pva-flow-evict", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="tcp-pva-whento-offload",
                value_type="enum",
                enum_values=("embryonic", "establish"),
            ),
            BigipPropertySpec(
                name="tcp-pva-offload-direction",
                value_type="enum",
                enum_values=("bidirectional", "client-to-server-only", "server-to-client-only"),
            ),
            BigipPropertySpec(
                name="other-pva-whento-offload",
                value_type="enum",
                enum_values=("after-packets-per-direction", "after-packets-both-direction"),
            ),
            BigipPropertySpec(
                name="other-pva-offload-direction",
                value_type="enum",
                enum_values=("bidirectional", "client-to-server-only", "server-to-client-only"),
            ),
            BigipPropertySpec(name="other-pva-clientpkts-threshold", value_type="integer"),
            BigipPropertySpec(name="other-pva-serverpkts-threshold", value_type="integer"),
            BigipPropertySpec(
                name="reassemble-fragments", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="reset-on-timeout", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="rtt-from-client", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="rtt-from-server", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="server-sack", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="server-timestamp", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="receive-window-size", value_type="string"),
            BigipPropertySpec(
                name="software-syn-cookie", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="syn-cookie-dsr-flow-reset-by",
                value_type="enum",
                allow_none=True,
                enum_values=("bigip", "client", "none"),
            ),
            BigipPropertySpec(
                name="syn-cookie-enable", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="syn-cookie-mss", value_type="integer"),
            BigipPropertySpec(
                name="syn-cookie-whitelist", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="tcp-close-timeout", value_type="integer"),
            BigipPropertySpec(
                name="tcp-generate-is", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="tcp-handshake-timeout", value_type="integer"),
            BigipPropertySpec(
                name="tcp-strip-sack", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="tcp-timestamp-mode",
                value_type="enum",
                enum_values=("preserve", "rewrite", "strip"),
            ),
            BigipPropertySpec(name="tcp-time-wait-timeout", value_type="integer"),
            BigipPropertySpec(
                name="tcp-wscale-mode",
                value_type="enum",
                enum_values=("preserve", "rewrite", "strip"),
            ),
            BigipPropertySpec(
                name="late-binding", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="explicit-flow-migration",
                value_type="enum",
                enum_values=("enabled", "disabled"),
            ),
            BigipPropertySpec(name="client-timeout", value_type="integer"),
            BigipPropertySpec(
                name="timeout-recovery", value_type="enum", enum_values=("disconnect", "fallback")
            ),
            BigipPropertySpec(
                name="reset-on-client-fin", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )

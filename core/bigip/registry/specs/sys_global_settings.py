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
            "sys_global_settings",
            module="sys",
            object_types=("global-settings",),
        ),
        header_types=(("sys", "global-settings"),),
        properties=(
            BigipPropertySpec(name="aws-access-key", value_type="string"),
            BigipPropertySpec(name="aws-secret-key", value_type="string"),
            BigipPropertySpec(name="aws-api-max-concurrency", value_type="integer"),
            BigipPropertySpec(name="file-blacklist-path-prefix", value_type="string"),
            BigipPropertySpec(name="file-blacklist-read-only-path-prefix", value_type="string"),
            BigipPropertySpec(name="file-whitelist-path-prefix", value_type="string"),
            BigipPropertySpec(name="console-inactivity-timeout", value_type="integer"),
            BigipPropertySpec(name="custom-addr", value_type="string"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(name="failsafe-action", value_type="string"),
            BigipPropertySpec(name="go-offline-restart-tm", value_type="string"),
            BigipPropertySpec(name="file-local-path-prefix", value_type="string"),
            BigipPropertySpec(
                name="gui-audit", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="gui-expired-cert-alert",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="gui-security-banner", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(name="gui-security-banner-text", value_type="string"),
            BigipPropertySpec(
                name="gui-setup", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="host-addr-mode",
                value_type="enum",
                enum_values=("custom", "management", "state-mirror"),
            ),
            BigipPropertySpec(
                name="hostname",
                value_type="string",
                pattern="^(?=.{1,253}$)(?:[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?\\\\.)+[A-Za-z]{2,63}$",
            ),
            BigipPropertySpec(name="hosts-allow-include", value_type="string"),
            BigipPropertySpec(
                name="lcd-display", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="net-reboot", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="ssh-session-limit", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="ssh-root-session-limit",
                value_type="enum",
                enum_values=("disabled", "enabled"),
            ),
            BigipPropertySpec(name="ssh-max-session-limit", value_type="integer"),
            BigipPropertySpec(name="ssh-max-session-limit-per-user", value_type="integer"),
            BigipPropertySpec(name="password-prompt", value_type="string"),
            BigipPropertySpec(
                name="mgmt-dhcp",
                value_type="enum",
                enum_values=("dhcpv4", "dhcpv6", "disabled", "enabled"),
            ),
            BigipPropertySpec(
                name="quiet-boot", value_type="enum", enum_values=("disabled", "enabled")
            ),
            BigipPropertySpec(
                name="remote-host",
                value_type="enum",
                enum_values=("add", "delete", "replace-all-with"),
            ),
            BigipPropertySpec(name="addr", value_type="string", in_sections=("remote-host",)),
            BigipPropertySpec(
                name="hostname",
                value_type="string",
                in_sections=("remote-host",),
                pattern="^(?=.{1,253}$)(?:[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?\\\\.)+[A-Za-z]{2,63}$",
            ),
            BigipPropertySpec(name="username-prompt", value_type="string"),
        ),
    )

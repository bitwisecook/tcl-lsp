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
            "cm_trust_domain",
            module="cm",
            object_types=("trust-domain",),
        ),
        header_types=(("cm", "trust-domain"),),
        properties=(
            BigipPropertySpec(
                name="add-device",
                value_type="unknown",
                shape_kind="object",
                block=(
                    BigipPropertySpec(
                        name="device-ip",
                        value_type="string",
                        in_sections=("add-device",),
                    ),
                    BigipPropertySpec(
                        name="device-name",
                        value_type="string",
                        in_sections=("add-device",),
                    ),
                    BigipPropertySpec(
                        name="device-port",
                        value_type="unknown",
                        in_sections=("add-device",),
                        usage_flags=frozenset(("optional",)),
                    ),
                    BigipPropertySpec(
                        name="password",
                        value_type="string",
                        in_sections=("add-device",),
                        required=True,
                    ),
                    BigipPropertySpec(
                        name="sha1-fingerprint",
                        value_type="string",
                        in_sections=("add-device",),
                        usage_flags=frozenset(("optional",)),
                    ),
                    BigipPropertySpec(
                        name="username",
                        value_type="string",
                        in_sections=("add-device",),
                        required=True,
                    ),
                ),
            ),
            BigipPropertySpec(name="device-ip", value_type="string", in_sections=("add-device",)),
            BigipPropertySpec(name="device-name", value_type="string", in_sections=("add-device",)),
            BigipPropertySpec(
                name="device-port",
                value_type="unknown",
                in_sections=("add-device",),
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(
                name="password",
                value_type="string",
                in_sections=("add-device",),
                required=True,
            ),
            BigipPropertySpec(
                name="sha1-fingerprint",
                value_type="string",
                in_sections=("add-device",),
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(
                name="username",
                value_type="string",
                in_sections=("add-device",),
                required=True,
            ),
            BigipPropertySpec(
                name="ca-devices",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(name="deprecated", value_type="unknown"),
            BigipPropertySpec(
                name="devices",
                value_type="list",
                list_operators=frozenset(("delete",)),
            ),
            BigipPropertySpec(
                name="md5-fingerprint",
                value_type="string",
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(
                name="non-ca-devices",
                value_type="list",
                list_operators=frozenset(("add", "delete", "modify", "replace-all-with")),
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(name="password", value_type="string", required=True),
            BigipPropertySpec(name="remove-device", value_type="string"),
            BigipPropertySpec(
                name="serial",
                value_type="string",
                usage_flags=frozenset(("deprecated",)),
            ),
            BigipPropertySpec(
                name="sha1-fingerprint",
                value_type="string",
                usage_flags=frozenset(("optional",)),
            ),
            BigipPropertySpec(name="username", value_type="string", required=True),
            BigipPropertySpec(
                name="ca-cert",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="ca-cert-bundle",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="ca-key",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="guid",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="status",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
            BigipPropertySpec(
                name="trust-group",
                value_type="string",
                usage_flags=frozenset(("read_only",)),
            ),
        ),
    )

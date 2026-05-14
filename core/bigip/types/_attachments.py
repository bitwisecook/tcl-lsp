"""Typed BIG-IP attachment values — profiles, persistence, etc.

Several F5 properties expose lists where each item is a keyed sub-
block whose body carries optional metadata.  Examples:

    profiles {
        /Common/clientssl { context clientside }
        /Common/serverssl { context serverside }
        /Common/http { }
    }

    persist {
        /Common/cookie { default yes }
        /Common/source_addr { }
    }

A flat ``tuple[str, ...]`` of full-paths drops the body — the
client-side/server-side distinction on a profile attachment, the
``default yes`` flag on a persistence attachment.  This module
models the per-item structure so the query DSL can ask

    .ltm.virtual[].profiles[] | select(.context == "clientside") | .name
    .ltm.virtual[].persist[] | select(.default) | .name

and the rewrite layer can update a single attachment safely without
collapsing the others.

The matching :class:`core.bigip.registry.value_specs.ProfileAttachmentSpec`
/ :class:`PersistenceAttachmentSpec` wire these into the registry's
list / reference dispatch.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class ProfileAttachment:
    """One ``profiles { ... }`` list item on an ltm virtual.

    ``path`` is the profile's full-path reference.  ``context`` is
    one of ``"clientside"`` / ``"serverside"`` / ``"all"`` /
    ``""`` (empty when omitted, meaning the profile applies to
    every direction).  ``raw`` carries the original ``{ ... }``
    body so a round trip through the rewrite layer keeps the
    user's spacing when no changes are made.
    """

    path: str = ""
    context: str = ""
    raw: str = ""

    @property
    def name(self) -> str:
        """Bare leaf name of the referenced profile."""
        return self.path.rsplit("/", 1)[-1] if self.path else ""

    @classmethod
    def from_raw(cls, path: str, body: str) -> "ProfileAttachment":
        """Build an attachment from the keyed-block parser output.

        *path* is the keyed-block's key (full-path reference).
        *body* is the brace body content (without the surrounding
        ``{`` / ``}``) — it may be empty when the attachment is
        bare.  Recognises the ``context <clientside|serverside|all>``
        property; other property tokens are preserved through the
        ``raw`` field but not surfaced as structured fields here.
        """
        context = ""
        for token in body.split():
            if token == "context":
                continue
            if context == "" and token in ("clientside", "serverside", "all"):
                context = token
                continue
        return cls(path=path, context=context, raw=body.strip())

    def __str__(self) -> str:
        if self.raw:
            return f"{self.path} {{ {self.raw} }}" if self.raw else f"{self.path} {{ }}"
        if self.context:
            return f"{self.path} {{ context {self.context} }}"
        return f"{self.path} {{ }}"


@dataclass(frozen=True, slots=True)
class PersistenceAttachment:
    """One ``persist { ... }`` list item on an ltm virtual.

    ``path`` is the persistence profile's full-path reference.
    ``default`` is the ``default yes`` flag (True when the
    attachment is the fall-back persistence profile, False when
    explicitly ``default no`` or omitted).  ``raw`` preserves the
    original body text for round trips.
    """

    path: str = ""
    default: bool = False
    raw: str = ""

    @property
    def name(self) -> str:
        return self.path.rsplit("/", 1)[-1] if self.path else ""

    @classmethod
    def from_raw(cls, path: str, body: str) -> "PersistenceAttachment":
        """Build an attachment from the keyed-block parser output.

        Recognises the ``default <yes|no>`` property.  Defaults to
        ``False`` when omitted (matching the F5 default: a
        persistence profile is only the fall-back when explicitly
        marked).
        """
        default = False
        parts = body.split()
        for i, token in enumerate(parts):
            if token == "default" and i + 1 < len(parts):
                value = parts[i + 1].lower()
                default = value == "yes" or value == "true"
        return cls(path=path, default=default, raw=body.strip())

    def __str__(self) -> str:
        if self.raw:
            return f"{self.path} {{ {self.raw} }}"
        if self.default:
            return f"{self.path} {{ default yes }}"
        return f"{self.path} {{ }}"


__all__ = ["PersistenceAttachment", "ProfileAttachment"]

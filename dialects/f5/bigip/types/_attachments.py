# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

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

The matching :class:`dialects.f5.bigip.registry.value_specs.ProfileAttachmentSpec`
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

    @property
    def full_path(self) -> str:
        """Alias for ``path`` so DSL queries that historically asked
        ``.profiles[].full-path`` (the legacy PathRef contract) keep
        working with the structured projection."""
        return self.path

    @classmethod
    def from_raw(cls, path: str, body: str) -> "ProfileAttachment":
        """Build an attachment from the keyed-block parser output.

        *path* is the keyed-block's key (full-path reference).
        *body* is the brace body content (without the surrounding
        ``{`` / ``}``) — it may be empty when the attachment is
        bare.  Recognises the ``context <clientside|serverside|all>``
        property as a strict ``key <value>`` pair (brace-aware so
        a nested ``{ ... }`` sub-block in the body doesn't taint
        the parse); other property tokens are preserved through the
        ``raw`` field but not surfaced as structured fields here.
        """
        context = ""
        tokens = _iter_top_level_tokens(body)
        for idx, tok in enumerate(tokens):
            if tok == "context" and idx + 1 < len(tokens):
                candidate = tokens[idx + 1]
                if candidate in ("clientside", "serverside", "all"):
                    context = candidate
                    break
        return cls(path=path, context=context, raw=body.strip())

    def __str__(self) -> str:
        # Prefer raw only when the structured field still reflects
        # what raw parsed to — a ``dataclasses.replace(context=...)``
        # otherwise leaves raw stale and silently renders the
        # pre-change body.
        if self.raw and ProfileAttachment.from_raw(self.path, self.raw).context == self.context:
            return f"{self.path} {{ {self.raw} }}"
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

    @property
    def full_path(self) -> str:
        """Alias for ``path`` — PathRef back-compat."""
        return self.path

    @classmethod
    def from_raw(cls, path: str, body: str) -> "PersistenceAttachment":
        """Build an attachment from the keyed-block parser output.

        Recognises the ``default <yes|no>`` property as a strict
        ``key <value>`` pair (brace-aware so nested sub-blocks
        can't taint the parse).  Defaults to ``False`` when
        omitted (matching the F5 default: a persistence profile is
        only the fall-back when explicitly marked).
        """
        default = False
        tokens = _iter_top_level_tokens(body)
        for idx, tok in enumerate(tokens):
            if tok == "default" and idx + 1 < len(tokens):
                value = tokens[idx + 1].lower()
                default = value in ("yes", "true")
                break
        return cls(path=path, default=default, raw=body.strip())

    def __str__(self) -> str:
        # Stale-raw guard: re-parse to confirm ``default`` still
        # matches; mutations via ``dataclasses.replace`` otherwise
        # render the pre-change body.
        if self.raw and PersistenceAttachment.from_raw(self.path, self.raw).default == self.default:
            return f"{self.path} {{ {self.raw} }}"
        if self.default:
            return f"{self.path} {{ default yes }}"
        return f"{self.path} {{ }}"


def _iter_top_level_tokens(body: str) -> list[str]:
    """Return *body*'s top-level tokens, skipping nested ``{ ... }``.

    The attachment parsers want to see ``context clientside`` /
    ``default yes`` at the top level only — nested sub-blocks
    (rare in attachments today, but valid syntactically) must not
    contribute tokens that could be mistaken for keys at the outer
    level.  ``{`` and ``}`` are themselves emitted so callers that
    care about brace markers can still see them; the common
    ``zip(tokens, tokens[1:])`` style consumer skips them
    naturally because they aren't recognised keys.
    """
    out: list[str] = []
    depth = 0
    for tok in body.split():
        if tok == "{":
            depth += 1
            out.append(tok)
            continue
        if tok == "}":
            if depth > 0:
                depth -= 1
            out.append(tok)
            continue
        if depth > 0:
            continue
        out.append(tok)
    return out


__all__ = ["PersistenceAttachment", "ProfileAttachment"]

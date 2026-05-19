"""Runtime feature flags and diagnostic/optimiser filter state."""

from __future__ import annotations

from dataclasses import dataclass, field

from analyser.irules_checks import DEFAULT_GENERIC_VARIABLE_PATTERNS
from shared.codes import default_disabled_diagnostics
from shared.optimisation_profiles import DEFAULT_EDITOR_PROFILE, profile_to_disabled


@dataclass
class FeatureConfig:
    """Runtime feature flags and diagnostic/optimiser filter state."""

    # Feature-level toggles
    hover_enabled: bool = True
    completion_enabled: bool = True
    diagnostics_enabled: bool = True
    semantic_tokens_enabled: bool = True
    code_actions_enabled: bool = True
    definition_enabled: bool = True
    references_enabled: bool = True
    document_symbols_enabled: bool = True
    folding_enabled: bool = True
    rename_enabled: bool = True
    signature_help_enabled: bool = True
    workspace_symbols_enabled: bool = True
    inlay_hints_enabled: bool = False
    call_hierarchy_enabled: bool = True
    document_links_enabled: bool = True
    selection_range_enabled: bool = True
    document_highlight_enabled: bool = True
    code_lens_enabled: bool = True
    workspace_file_ops_enabled: bool = True
    # Pull-model diagnostics are opt-in.  vscode-languageclient auto-enables
    # its pull flow whenever the server advertises ``diagnosticProvider`` in
    # ServerCapabilities, which disables the push pipeline that the existing
    # test suite and most clients rely on.  Users who explicitly want the
    # pull model can opt in via ``tclLsp.features.pullDiagnostics``.
    pull_diagnostics_enabled: bool = False
    # willSaveWaitUntil is off by default.  Editors that support a native
    # "format on save" mechanism (VS Code's editor.formatOnSave, JetBrains'
    # on-save actions, etc.) should use that instead — it routes through the
    # standard textDocument/formatting handler.  This toggle exists as a
    # fallback for editors without a native format-on-save mechanism.
    will_save_wait_until_enabled: bool = False
    progress_enabled: bool = True
    implementation_enabled: bool = True
    type_definition_enabled: bool = True
    declaration_enabled: bool = True
    linked_editing_range_enabled: bool = True

    # Per-code diagnostic filters -- codes present here are *disabled*.
    # Initialised from codes with ``default=False`` (opt-in diagnostics).
    disabled_diagnostics: set[str] = field(
        default_factory=lambda: set(default_disabled_diagnostics())
    )

    # Optimiser master switch, profile, and per-code filters.
    optimiser_enabled: bool = True
    optimiser_profile: str = DEFAULT_EDITOR_PROFILE.value
    disabled_optimisations: set[str] = field(
        default_factory=lambda: set(profile_to_disabled(DEFAULT_EDITOR_PROFILE))
    )

    # Shimmer detection master switch.
    shimmer_enabled: bool = True

    # XC translatability diagnostics (opt-in, for migration planning).
    xc_diagnostics_enabled: bool = False

    # Style: maximum line length for W111.
    line_length: int = 120

    # Style: W108 non-ASCII detection mode (``strict`` / ``confusables`` /
    # ``common`` / ``off``).  ``None`` inherits the process default set by
    # ``set_non_ascii_mode`` at server startup.
    non_ascii_mode: str | None = None

    # Per-folder ``libraryPaths`` for package_require resolution.  Each
    # entry is a filesystem directory containing Tcl packages (``pkgIndex.tcl``,
    # tcllib, etc.).  ``None`` means "inherit the workspace fallback list" so
    # documents outside every folder still see workspace-wide paths.
    library_paths: tuple[str, ...] | None = None

    # IRULE4002: regex patterns matching generic static:: / global variable
    # bare names (after stripping the ``static::`` prefix).  Empty list
    # disables the check.  Patterns are matched case-insensitively against
    # the full bare name.
    generic_variable_patterns: list[str] = field(
        default_factory=lambda: list(DEFAULT_GENERIC_VARIABLE_PATTERNS)
    )

    # True once the user explicitly sets ``tclLsp.dialect`` in settings.
    # When False, the server may auto-detect the dialect from the editor's
    # ``language_id``.
    dialect_explicitly_set: bool = False

    # Resolved dialect for this folder (or workspace fallback).  ``None``
    # means "inherit the process-default" — historically the only available
    # mode, but post-#407 each folder may carry its own dialect and the LSP
    # request handlers wrap their work in ``dialect_scope`` to apply it.
    dialect: str | None = None

    # Resolved extra command-names for this folder (sorted, deduplicated).
    # ``None`` means "inherit from the workspace fallback".
    extra_commands: tuple[str, ...] | None = None

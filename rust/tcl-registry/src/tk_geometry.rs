// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Tk geometry-manager semantics consumed by static UI tooling.
//!
//! A geometry manager and an exclusive geometry-container claimant are not
//! the same thing. `pack` and `grid` call `TkSetGeometryContainer` and cannot
//! both propagate geometry through one container. `place` manages content but
//! deliberately does not claim or resize the container. Keeping that fact in
//! the registry prevents consumers from naming commands or inventing a
//! universal mixed-manager rule.

/// Whether a geometry manager claims exclusive propagation ownership of its
/// effective container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TkGeometryContainerPolicy {
    /// The manager claims the container through Tk's geometry-container API.
    Exclusive,
    /// The manager positions content without claiming the container.
    Independent,
}

/// Whether `path` is a non-root Tk widget pathname.
///
/// Tk pathnames begin with `.` and every descendant component is nonempty.
/// Component spelling is otherwise deliberately open here: real code uses
/// hyphens and extension widgets may use other non-dot characters.
#[must_use]
pub fn is_widget_path(path: &str) -> bool {
    path.strip_prefix('.')
        .is_some_and(|tail| !tail.is_empty() && tail.split('.').all(|part| !part.is_empty()))
}

/// Whether `path` is the Tk root (`.`) or a non-root widget pathname.
#[must_use]
pub fn is_widget_path_or_root(path: &str) -> bool {
    path == "." || is_widget_path(path)
}

/// Source-only parent and component depth for a validated Tk widget pathname.
///
/// The root is valid and has no parent at depth zero. A non-root pathname has
/// its literal pathname parent and a one-based component depth. Invalid paths
/// return `None`; this projection says nothing about live Tk widgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TkWidgetPathParentDepth<'a> {
    /// Literal source parent, or `None` for the Tk root pathname.
    pub parent: Option<&'a str>,
    /// Number of non-root pathname components; zero for the root.
    pub component_depth: usize,
}

/// Project the source parent and component depth of a validated Tk pathname.
#[must_use]
pub fn widget_path_parent_depth(path: &str) -> Option<TkWidgetPathParentDepth<'_>> {
    if path == "." {
        return Some(TkWidgetPathParentDepth {
            parent: None,
            component_depth: 0,
        });
    }
    if !is_widget_path(path) {
        return None;
    }
    let component_depth = path.strip_prefix('.')?.split('.').count();
    let separator = path.rfind('.')?;
    let parent = if separator == 0 {
        "."
    } else {
        &path[..separator]
    };
    Some(TkWidgetPathParentDepth {
        parent: Some(parent),
        component_depth,
    })
}

/// Whether `candidate` is `ancestor` itself or one of its Tk pathname
/// descendants.
///
/// The root is special: appending a separator to `.` would produce `..`, but
/// every non-root Tk pathname already begins with the root dot.
#[must_use]
pub fn widget_path_is_within(candidate: &str, ancestor: &str) -> bool {
    is_widget_path_or_root(candidate)
        && is_widget_path_or_root(ancestor)
        && (candidate == ancestor
            || (ancestor == "." && candidate.starts_with('.'))
            || candidate
                .strip_prefix(ancestor)
                .is_some_and(|tail| tail.starts_with('.')))
}

/// Static semantics of a registry-declared Tk geometry manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TkGeometryManagerSpec {
    /// Policy used when checking managers that target one container.
    pub container_policy: TkGeometryContainerPolicy,
    /// Canonical option whose literal value overrides the pathname parent as
    /// the effective geometry container.
    pub container_option: Option<&'static str>,
    /// Whether the command's default (no-subcommand) form places widgets.
    pub direct_form: bool,
    /// Subcommand whose arguments place or reconfigure widgets.
    pub placement_subcommand: Option<&'static str>,
    /// Subcommands that stop managing the widgets named by their arguments.
    pub release_subcommands: &'static [&'static str],
}

/// `pack`: direct/configure place widgets; `forget` releases them.
pub const PACK_GEOMETRY: TkGeometryManagerSpec = TkGeometryManagerSpec {
    container_policy: TkGeometryContainerPolicy::Exclusive,
    container_option: Some("-in"),
    direct_form: true,
    placement_subcommand: Some("configure"),
    release_subcommands: &["forget"],
};

/// `grid`: direct/configure place widgets; `forget` and `remove` release them.
pub const GRID_GEOMETRY: TkGeometryManagerSpec = TkGeometryManagerSpec {
    container_policy: TkGeometryContainerPolicy::Exclusive,
    container_option: Some("-in"),
    direct_form: true,
    placement_subcommand: Some("configure"),
    release_subcommands: &["forget", "remove"],
};

/// `place`: direct/configure place widgets; `forget` releases them.
pub const PLACE_GEOMETRY: TkGeometryManagerSpec = TkGeometryManagerSpec {
    container_policy: TkGeometryContainerPolicy::Independent,
    container_option: Some("-in"),
    direct_form: true,
    placement_subcommand: Some("configure"),
    release_subcommands: &["forget"],
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_path_grammar_has_nonempty_dot_separated_components() {
        for path in [".main", ".main.child", ".my-widget.entry-1"] {
            assert!(is_widget_path(path), "{path}");
            assert!(is_widget_path_or_root(path), "{path}");
        }
        for path in ["", ".", "main", "..main", ".main.", ".main..child"] {
            assert!(!is_widget_path(path), "{path}");
        }
        assert!(is_widget_path_or_root("."));
        assert!(!is_widget_path_or_root("main"));
    }

    #[test]
    fn source_parent_depth_projection_distinguishes_root_valid_and_invalid_paths() {
        assert_eq!(
            widget_path_parent_depth("."),
            Some(TkWidgetPathParentDepth {
                parent: None,
                component_depth: 0,
            }),
        );
        assert_eq!(
            widget_path_parent_depth(".main"),
            Some(TkWidgetPathParentDepth {
                parent: Some("."),
                component_depth: 1,
            }),
        );
        assert_eq!(
            widget_path_parent_depth(".main.child.item-1"),
            Some(TkWidgetPathParentDepth {
                parent: Some(".main.child"),
                component_depth: 3,
            }),
        );
        for invalid in ["", "main", ".main..child", ".main."] {
            assert_eq!(widget_path_parent_depth(invalid), None, "{invalid:?}");
        }
    }

    #[test]
    fn root_and_component_ancestry_are_segment_aware() {
        assert!(widget_path_is_within(".", "."));
        assert!(widget_path_is_within(".main", "."));
        assert!(widget_path_is_within(".main.child", ".main"));
        assert!(!widget_path_is_within(".mainly", ".main"));
        assert!(!widget_path_is_within("main", "."));
    }
}

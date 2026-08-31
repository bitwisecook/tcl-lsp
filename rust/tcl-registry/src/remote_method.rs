// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Remote-method descriptors — commands that name a method implemented in
//! **another language**, reached over an RPC handle (issue #1707).
//!
//! The archetype, and the only family modelled today, is iRulesLX: an iRule
//! opens a handle onto a running Node.js extension and then calls a method the
//! extension registered by name.
//!
//! ```tcl
//! set RPC_HANDLE [ILX::init my_plugin my_extension]
//! set rpc_response [ILX::call $RPC_HANDLE my_js_function arg1 arg2]
//! ILX::notify $RPC_HANDLE my_js_function arg1
//! ```
//!
//! ```javascript
//! var f5 = require('f5-nodejs');
//! var ilx = new f5.ILXServer();
//! ilx.addMethod('my_js_function', function (req, res) { … });
//! ilx.listen();
//! ```
//!
//! VERIFIED against F5's documentation (fetched 2026-08-30):
//!
//! * <https://clouddocs.f5.com/api/irules/ILX__init.html> — "`ILX::init` [plugin
//!   name] [extension name]", "Creates a handle for future use by `ILX::call`
//!   and `ILX::notify`".
//! * <https://clouddocs.f5.com/api/irules/ILX__call.html> — "`ILX::call` \<ILX
//!   handle\> [-timeout n] \<method\> [optional arg]+", synchronous: it blocks
//!   "until receiving a response".
//! * <https://clouddocs.f5.com/api/irules/ILX__notify.html> (and the registry's
//!   own `ILX::notify` hover) — `ILX::notify HANDLE METHOD (ARGS)*`, delivery is
//!   "best effort and is not guaranteed", i.e. no reply is awaited.
//! * <https://clouddocs.f5.com/api/irules-lx/ILXServer.html> — "addMethod(name,
//!   callback)" — "Add a method handler".
//!
//! Why this is registry data rather than a walker branch: the LSP's navigation
//! layer resolves the *shape* — which word is the handle, which word is the
//! method, whether options sit in between — by reading these descriptors, so it
//! never matches on `"ILX::call"`.  A second RPC family (another dialect's
//! remote-call command pair) is a new [`RemoteFamily`] arm plus two specs, not
//! new provider code.  It also gives the dialect gate for free: `ILX::call` is
//! an `SpecSurface::IRULES` spec, so a stock Tcl registry has no such command
//! and therefore no descriptor to find — an ordinary Tcl document that happens
//! to define a proc named `ILX::call` resolves nothing here.

/// Which cross-language RPC family a descriptor belongs to.
///
/// The family selects how a consumer finds the *implementation* side: for
/// [`Self::IRulesLxNode`] that is an `ILXServer.addMethod("name", …)`
/// registration in an iRulesLX extension's Node.js entry point.  A consumer
/// that does not know a family abstains rather than guessing, which is what
/// keeps adding a family additive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RemoteFamily {
    /// iRulesLX: an iRule calling into a Node.js extension over the ILX RPC
    /// channel.
    IRulesLxNode,
}

/// Whether a call waits for the remote method's reply.
///
/// Kept distinct because the two commands are genuinely different operations —
/// `ILX::call` blocks for the reply and yields it, `ILX::notify` is fire and
/// forget — even though they share one method target (issue #1707 criterion 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RemoteDispatch {
    /// The iRule blocks until the extension replies, and the call evaluates to
    /// the reply (`ILX::call`).
    Synchronous,
    /// The call is queued best-effort and the iRule continues immediately; no
    /// reply is delivered (`ILX::notify`).
    Notification,
}

impl RemoteDispatch {
    /// A short human label for hover / telemetry.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Synchronous => "synchronous call",
            Self::Notification => "best-effort notification",
        }
    }
}

/// Where the method word sits in a remote call's argument list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MethodWord {
    /// Exactly this 0-based argument index (after the command word) — the
    /// layout of a command that takes no leading options
    /// (`ILX::notify HANDLE METHOD …`).
    At(u8),
    /// The first positional word at or after this 0-based index, once the
    /// command's own declared options and any `--` terminator are consumed —
    /// `ILX::call HANDLE ?-timeout ms? ?--? METHOD …`.  The consumer resolves
    /// it with [`crate::hover::first_positional_index`] over the spec's own
    /// [`options`](crate::spec::CommandSpec::options), so `-timeout`'s value
    /// word is never mistaken for the method.
    AfterOptions(u8),
}

/// How a command opens a handle onto a remote extension.
///
/// Both name words must be literal for a consumer to use the handle: the
/// association is a name pair, and a substituted word names nothing statically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteHandleSpec {
    /// The RPC family the handle belongs to.
    pub family: RemoteFamily,
    /// 0-based index of the word naming the enclosing **scope** — the ILX
    /// *plugin*.
    pub scope_arg: u8,
    /// 0-based index of the word naming the **extension** within that scope.
    pub extension_arg: u8,
    /// The exact argument count this layout requires.
    ///
    /// `ILX::init` also has an undocumented one-word spelling (the registry's
    /// own hover synopsis records `ILX::init (EXTENSION | (PLUGIN EXTENSION))`),
    /// but F5 documents only the two-word form and says nothing about which
    /// plugin the short form binds to.  Requiring the documented arity exactly
    /// makes the short form abstain instead of inventing an association
    /// (issue #1707 criterion 4).
    pub exact_argc: u8,
}

/// How a command invokes a named method through a handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteCallSpec {
    /// The RPC family the call belongs to.
    pub family: RemoteFamily,
    /// 0-based index of the word carrying the handle (`$RPC_HANDLE`).
    pub handle_arg: u8,
    /// Where the method name is written.
    pub method: MethodWord,
    /// Whether the call awaits a reply.
    pub dispatch: RemoteDispatch,
}

/// What role a command plays in a cross-language RPC family, when it plays one.
///
/// Attached to a command via [`CommandSpec::remote_method`].
///
/// [`CommandSpec::remote_method`]: crate::spec::CommandSpec::remote_method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteMethodRole {
    /// The command opens a handle onto a remote extension (`ILX::init`).
    OpensHandle(RemoteHandleSpec),
    /// The command invokes a named method through such a handle
    /// (`ILX::call`, `ILX::notify`).
    CallsMethod(RemoteCallSpec),
}

impl RemoteMethodRole {
    /// The family this role belongs to.
    #[must_use]
    pub const fn family(&self) -> RemoteFamily {
        match self {
            Self::OpensHandle(spec) => spec.family,
            Self::CallsMethod(spec) => spec.family,
        }
    }

    /// The handle-opening layout, when this role is one.
    #[must_use]
    pub const fn opens_handle(&self) -> Option<&RemoteHandleSpec> {
        match self {
            Self::OpensHandle(spec) => Some(spec),
            Self::CallsMethod(_) => None,
        }
    }

    /// The method-calling layout, when this role is one.
    #[must_use]
    pub const fn calls_method(&self) -> Option<&RemoteCallSpec> {
        match self {
            Self::CallsMethod(spec) => Some(spec),
            Self::OpensHandle(_) => None,
        }
    }
}

/// `ILX::init PLUGIN EXTENSION` — the iRulesLX handle constructor.
pub const ILX_INIT_HANDLE: RemoteMethodRole = RemoteMethodRole::OpensHandle(RemoteHandleSpec {
    family: RemoteFamily::IRulesLxNode,
    scope_arg: 0,
    extension_arg: 1,
    exact_argc: 2,
});

/// `ILX::call HANDLE ?-timeout ms? ?--? METHOD ?args …?` — the synchronous
/// iRulesLX RPC call.
pub const ILX_CALL_METHOD: RemoteMethodRole = RemoteMethodRole::CallsMethod(RemoteCallSpec {
    family: RemoteFamily::IRulesLxNode,
    handle_arg: 0,
    method: MethodWord::AfterOptions(1),
    dispatch: RemoteDispatch::Synchronous,
});

/// `ILX::notify HANDLE METHOD ?args …?` — the best-effort iRulesLX RPC
/// notification.  F5 documents no options for it, so the method word is at a
/// fixed index.
pub const ILX_NOTIFY_METHOD: RemoteMethodRole = RemoteMethodRole::CallsMethod(RemoteCallSpec {
    family: RemoteFamily::IRulesLxNode,
    handle_arg: 0,
    method: MethodWord::At(1),
    dispatch: RemoteDispatch::Notification,
});

#[cfg(test)]
mod tests {
    use super::{ILX_CALL_METHOD, ILX_INIT_HANDLE, ILX_NOTIFY_METHOD};
    use super::{MethodWord, RemoteDispatch, RemoteMethodRole};

    #[test]
    fn ilx_call_method_word_follows_options() {
        let RemoteMethodRole::CallsMethod(spec) = ILX_CALL_METHOD else {
            panic!("ILX::call calls a method");
        };
        assert_eq!(spec.method, MethodWord::AfterOptions(1));
        assert_eq!(spec.dispatch, RemoteDispatch::Synchronous);
    }

    #[test]
    fn ilx_notify_is_a_notification_at_a_fixed_index() {
        let RemoteMethodRole::CallsMethod(spec) = ILX_NOTIFY_METHOD else {
            panic!("ILX::notify calls a method");
        };
        assert_eq!(spec.method, MethodWord::At(1));
        assert_eq!(spec.dispatch, RemoteDispatch::Notification);
    }

    #[test]
    fn ilx_init_requires_the_documented_two_word_form() {
        let handle = ILX_INIT_HANDLE.opens_handle().expect("init opens a handle");
        assert_eq!(handle.exact_argc, 2);
        assert_eq!(handle.scope_arg, 0);
        assert_eq!(handle.extension_arg, 1);
        assert!(ILX_INIT_HANDLE.calls_method().is_none());
    }
}

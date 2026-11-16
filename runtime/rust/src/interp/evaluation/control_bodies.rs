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

//! Evaluating the body of a control command, `eval`, `uplevel` or a definition: located and unlocated bodies, body objects and command substitution.

use super::super::{native_jim_lookup, obj_bytes, CmdFrame, Code, FrameKind, Interp};
use super::{line_of, scan_list_offsets, OriginalBodyEvaluation, PreparedOriginalBodyEvaluation};
use crate::obj;
use crate::obj::TclObj;
use std::rc::Rc;
use tcl_runtime_api::CommandEvaluation;

impl Interp {
    /// Evaluate a TclOO definition body. With `src = Some((file, line_base))`
    /// (the body was defined while sourcing a file) it runs in a `type source`
    /// frame, so its commands — and the method bodies they define — report
    /// file-absolute `info frame` lines (TIP 280). Otherwise it runs inline.
    pub(crate) fn eval_def_body(&mut self, body: &[u8], src: Option<(Rc<[u8]>, u32)>) -> Code {
        if let Err(first) = self.clear_return_options_checked() {
            return self.refuse_native_execution(first);
        }
        match src {
            Some((file, line_base)) => {
                let mut frame = self.inherited_cmd_frame();
                frame.kind = FrameKind::Source;
                frame.file = Some(file);
                frame.line_base = line_base;
                frame.oo = None;
                self.eval_framed(body, frame)
            }
            None => self.eval_str(body),
        }
    }

    /// `uplevel`: evaluate `script` in the variable scope **and** namespace of
    /// frame `target_level` (restore caller ns + frame depth
    /// together), then restore. Transparent — the body's completion
    /// code (incl. `return`) propagates unchanged.
    pub(crate) fn eval_uplevel(&mut self, target_level: usize, script: &[u8]) -> Code {
        if let Err(first) = self.clear_return_options_checked() {
            return self.refuse_native_execution(first);
        }
        let prev_level = self.frames.borrow_mut().set_active_level(target_level);
        let prev_ns = self.current_ns.get();
        self.current_ns
            .set(self.frames.borrow().frame_ns(target_level));
        // The `uplevel` body is a fresh dynamically-evaluated script: `type
        // eval`, **no** file, body-relative lines (base 0) — but it keeps the
        // invoking proc's name and runs at the target call level (with no
        // `level` key, the redirected scope). Matches tclsh, where `uplevel`'s
        // body is not inlined into the proc bytecode.
        let mut frame = self.inherited_cmd_frame();
        frame.kind = FrameKind::Eval;
        frame.file = None;
        frame.line_base = 0;
        frame.level = target_level;
        frame.omit_level = true;
        let code = self.eval_framed(script, frame);
        self.frames.borrow_mut().set_active_level(prev_level);
        self.current_ns.set(prev_ns);
        code
    }

    /// [`Self::eval_control_body`] for a command whose caller knows whether the
    /// release compiles it ([`Self::release_compiles_control`]).
    pub(crate) fn eval_compiled_control_body(&mut self, body: *mut TclObj, compiled: bool) -> Code {
        self.compiled_control_body.set(Some(compiled));
        self.eval_control_body(body)
    }

    /// Evaluate an inline **control-command body** (`if`/`while`/`for`/`foreach`/
    /// …). A body that is a located literal (TIP 280) runs as its own
    /// line-advancing source frame at the body's file line, so `info frame`
    /// reports the executing command's true line; a pure list dispatches by
    /// element identity; a dynamic body (a script in a variable) runs as its own
    /// `type eval` level with body-relative lines (C's `TclEvalObjEx` always
    /// pushes a cmdframe — `info frame` reports `line 3` for the body's 3rd line,
    /// not the enclosing command's line).
    pub(crate) fn eval_control_body(&mut self, body: *mut TclObj) -> Code {
        let inline = self
            .compiled_control_body
            .take()
            .unwrap_or_else(|| self.in_proc());
        let snapshot = match obj::native_object_snapshot(body) {
            Ok(snapshot) => snapshot,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        let Some(protocol) = self.eval_object_protocol() else {
            return self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native control-body object dispatch",
                )
                .into(),
            );
        };
        if protocol.dispatches_list(
            tcl_registry::native_eval_object::EvalObjectPurpose::ControlBody,
            &snapshot,
        ) {
            let frame = self.unlocated_frame();
            return self.dispatch_list_obj(body, frame);
        }
        // Jim's control workers evaluate the same original object through
        // Script; Source primary fields are independent of C TIP280 locations.
        if self.invocation_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084)
        {
            return self.eval_generic_control_body(body);
        }
        // naming.mathop-procedure-source-controls
        // docs/design/analysis/name-resolution-proofs/mathop-procedure-source-controls.md
        if !self.in_proc()
            && protocol
                .compiles_source(tcl_registry::native_eval_object::EvalObjectPurpose::ControlBody)
        {
            return self.eval_generic_control_body(body);
        }
        if let Some((file, line)) = self.arg_loc(body) {
            // A command the release compiles inline (inside a proc where its
            // caller cannot say) runs its literal body in the **same** `info
            // frame` level — no new frame, the shared frame's line just
            // advances to each body command (C's bytecode inlining). One it
            // runs evaluates the body as its own frame (`TclEvalObjEx`),
            // matching tclsh's `info frame` depth.
            if inline {
                return self.eval_shared_located_body(body, line);
            }
            let mut frame = self.inherited_cmd_frame();
            frame.kind = FrameKind::Source;
            frame.file = file;
            frame.line_base = line.saturating_sub(1);
            let bytes = obj_bytes(body);
            return self.eval_framed(&bytes, frame);
        }
        // As a located body inside a procedure, compiled with the procedure
        // body holding it.
        if inline && self.compiles_locals() {
            self.next_proc_body.set(true);
        }
        self.eval_unlocated_body(&obj_bytes(body))
    }

    /// Evaluate a located-literal control body that is **inlined** into the
    /// enclosing frame (the in-proc case of [`eval_control_body`]). The enclosing
    /// frame is shared (no new `info frame` level); its `line_base` is shifted so
    /// the body's commands report their own file-absolute lines, then restored.
    /// The body literal lives in the same source as the enclosing frame, so the
    /// frame's `kind`/`file` already match — only the line mapping changes.
    pub(in super::super) fn eval_shared_located_body(
        &mut self,
        body: *mut TclObj,
        line: u32,
    ) -> Code {
        let saved = {
            let mut frames = self.cmd_frames.borrow_mut();
            frames.last_mut().map(|top| {
                let saved = (
                    top.line_base,
                    top.line,
                    std::mem::take(&mut top.cmd),
                    top.original_command.take(),
                );
                top.line_base = line.saturating_sub(1);
                saved
            })
        };
        let Some((line_base, line, cmd, original_command)) = saved else {
            // No enclosing frame to share (shouldn't happen under `in_proc`) —
            // fall back to a body-relative eval rather than panic.
            return self.eval_unlocated_body(&obj_bytes(body));
        };
        let bytes = obj_bytes(body);
        let code = self.eval_script_mode(&bytes, None, true);
        if let Some(top) = self.cmd_frames.borrow_mut().last_mut() {
            top.line_base = line_base;
            top.line = line;
            top.cmd = cmd;
            top.original_command = original_command;
        }
        code
    }

    /// The TIP 280 source location recorded for `obj` (the literal-argument
    /// location table), or `None` for a dynamic value. Lets a command that
    /// re-splits a literal (e.g. `switch`'s single-list-arg form) recover the
    /// enclosing file + the list word's line, to derive its sub-bodies' lines.
    pub(crate) fn arg_location(&self, obj: *mut TclObj) -> Option<(Option<Rc<[u8]>>, u32)> {
        self.arg_loc(obj)
    }

    /// Point the current shared frame's `line_base` at `line` (a condition word's
    /// source line) so command substitutions inside an `if`/`while`/`for`
    /// expression report their file-absolute line (TIP 280). Returns the previous
    /// `line_base` to hand back to [`restore_line_base`](Self::restore_line_base).
    #[cfg(have_tommath)]
    pub(crate) fn push_cond_line_base(&self, line: u32) -> Option<u32> {
        self.cmd_frames.borrow_mut().last_mut().map(|top| {
            let old = top.line_base;
            top.line_base = line.saturating_sub(1);
            old
        })
    }

    /// Restore a `line_base` saved by [`push_cond_line_base`](Self::push_cond_line_base).
    #[cfg(have_tommath)]
    pub(crate) fn restore_line_base(&self, old: u32) {
        if let Some(top) = self.cmd_frames.borrow_mut().last_mut() {
            top.line_base = old;
        }
    }

    /// The `(file, line)` of list element `index` within the literal `obj` — for
    /// a body that is a sub-element of a located list literal (an `apply` lambda's
    /// body, C's `TclListLines`). The element's line is the list word's line plus
    /// the newlines preceding the element. `None` when `obj` is a dynamic value
    /// or lacks a file (then the body is body-relative).
    pub(crate) fn list_element_location(
        &self,
        obj: *mut TclObj,
        index: usize,
    ) -> Option<(Rc<[u8]>, u32)> {
        let (Some(file), line) = self.arg_loc(obj)? else {
            return None;
        };
        let nl = scan_list_offsets(&obj_bytes(obj))?.get(index)?.0;
        Some((file, line + nl))
    }

    /// A `type eval`, no-file, body-relative `CmdFrame` (inheriting the enclosing
    /// proc/level) — the frame for a body with no source location (a dynamic
    /// script, or a canonical-list body).
    pub(crate) fn unlocated_frame(&self) -> CmdFrame {
        let mut frame = self.inherited_cmd_frame();
        frame.kind = FrameKind::Eval;
        frame.file = None;
        frame.line_base = 0;
        frame
    }

    /// Evaluate a body whose source location is unknown (C's switch `line = -1`
    /// case: a dynamically-built list body). It runs as `type eval` with **no
    /// file**, so a command defined inside (e.g. `proc`) is body-relative
    /// (`type proc`, line 1) rather than inheriting the enclosing file's lines.
    pub(crate) fn eval_unlocated_body(&mut self, body: &[u8]) -> Code {
        let frame = self.unlocated_frame();
        self.eval_framed(body, frame)
    }

    /// Evaluate a multi-arg `eval`/`uplevel` body (the args were space-joined
    /// into a fresh dynamic script) as its own `info frame` level. Such a body has
    /// no source location, so it is `type eval` with body-relative lines (C's
    /// `TclEvalObjEx` of a non-literal). The errorInfo `("eval" body line N)`
    /// frame is appended separately by the caller.
    pub(crate) fn eval_body(&mut self, script: &[u8]) -> Code {
        if let Err(first) = self.clear_return_options_checked() {
            return self.refuse_native_execution(first);
        }
        self.eval_unlocated_body(script)
    }

    /// Evaluate a `[...]` command substitution's inner `script`. A `[cmd]` is
    /// **not** a new `info frame` level (C compiles it into the enclosing
    /// command's bytecode — `info frame` depth is unchanged), but it *does*
    /// advance the reported `line`: a command inside the brackets reports the
    /// line it actually appears on, even when the bracket spans lines or follows
    /// a `\`-newline continuation. `script` is a sub-slice of `src` (it borrows
    /// from the parsed buffer), so its start offset — and thus its file-absolute
    /// line — comes straight from the original source: bs+nl continuations are
    /// real newlines there, so plain newline counting matches C's TIP 280 result
    /// without C's separate continuation-line `adjust` bookkeeping.
    ///
    /// The enclosing frame's location (`line_base`/`line`/`cmd`) is saved and
    /// restored around the inner eval so the rest of the enclosing command keeps
    /// reporting its own line once the substitution returns.
    pub(super) fn eval_command_subst(&mut self, src: &[u8], script: &[u8]) -> Code {
        let offset = (script.as_ptr() as usize).wrapping_sub(src.as_ptr() as usize);
        let saved = {
            let mut frames = self.cmd_frames.borrow_mut();
            match frames.last_mut() {
                Some(top) if offset <= src.len() => {
                    let saved = (
                        top.line_base,
                        top.line,
                        std::mem::take(&mut top.cmd),
                        top.original_command.take(),
                    );
                    // Shift the body-relative base so the inner script's line 1
                    // maps to the bracket's file-absolute line.
                    top.line_base = (top.line_base + line_of(src, offset)).saturating_sub(1);
                    Some(saved)
                }
                _ => None,
            }
        };
        // Share the enclosing frame but advance its line/cmd through the inner
        // commands (the third eval mode: no new frame, but line tracking on).
        let code = self.eval_script_mode(script, None, saved.is_some());
        // A script evaluated directly substitutes its own (C's `TclEvalEx`),
        // whose error leaves the enclosing command to log itself.
        if code == Code::Error && self.evaluation.get() == CommandEvaluation::Direct {
            self.clear_error_logged();
        }
        if let Some((line_base, line, cmd, original_command)) = saved {
            if let Some(top) = self.cmd_frames.borrow_mut().last_mut() {
                top.line_base = line_base;
                top.line = line;
                top.cmd = cmd;
                top.original_command = original_command;
            }
        }
        code
    }

    /// `eval` of a single body **object** — like [`eval_body`](Self::eval_body),
    /// but a literal obj with a recorded source location (TIP 280 LABC) runs as
    /// `type source` at its original file+line (the test-body case) rather than
    /// `type eval`.
    /// Generic command bodies always enter their original object activation.
    /// Enter an original control-body object at its retained optional source location.
    pub(crate) fn eval_original_control_body_location(
        &mut self,
        original: *mut TclObj,
        location: Option<(Option<Rc<[u8]>>, u32)>,
    ) -> Code {
        self.eval_original_control_body_location_with_purpose(
            tcl_registry::native_eval_object::EvalObjectPurpose::ControlBody,
            original,
            location,
        )
    }

    fn eval_original_control_body_location_with_purpose(
        &mut self,
        purpose: tcl_registry::native_eval_object::EvalObjectPurpose,
        original: *mut TclObj,
        location: Option<(Option<Rc<[u8]>>, u32)>,
    ) -> Code {
        let mut frame = self.inherited_cmd_frame();
        match location {
            Some((file, line)) => {
                frame.kind = FrameKind::Source;
                frame.file = file;
                frame.line_base = line.saturating_sub(1);
            }
            None => {
                frame.kind = FrameKind::Eval;
                frame.file = None;
                frame.line_base = 0;
            }
        }
        self.eval_original_body_framed(purpose, original, frame)
    }

    pub(crate) fn eval_generic_control_body(&mut self, original: *mut TclObj) -> Code {
        self.eval_original_control_body_location(original, self.arg_loc(original))
    }

    /// Only admitted Native control instructions select this dynamic-body entry.
    /// C EVAL_STK compiles the original header with a NULL invoker, even when
    /// that header retains a canonical List or an outer literal location.
    pub(in super::super) fn eval_compiled_stack_control_body(
        &mut self,
        original: *mut TclObj,
    ) -> Code {
        self.eval_original_control_body_location_with_purpose(
            tcl_registry::native_eval_object::EvalObjectPurpose::CompiledStack,
            original,
            None,
        )
    }

    pub(crate) fn eval_body_obj(&mut self, obj: *mut TclObj) -> Code {
        self.eval_body_obj_with_inventory(obj, &mut None)
    }

    pub(in super::super) fn eval_body_obj_with_inventory(
        &mut self,
        obj: *mut TclObj,
        inventory: &mut Option<Rc<native_jim_lookup::OriginalJimCommandInventoryReturn>>,
    ) -> Code {
        *inventory = None;
        if let Err(first) = self.clear_return_options_checked() {
            return self.refuse_native_execution(first);
        }
        let mut frame = self.inherited_cmd_frame();
        match self.arg_loc(obj) {
            // A located literal body keeps its file+line (`type source`).
            Some((file, line)) => {
                frame.kind = FrameKind::Source;
                frame.file = file;
                frame.line_base = line.saturating_sub(1);
            }
            // A dynamic body (`eval $script`) is `type eval`, body-relative — it
            // does not inherit the enclosing file's lines (C's `TclEvalObjEx`).
            None => {
                frame.kind = FrameKind::Eval;
                frame.file = None;
                frame.line_base = 0;
            }
        }
        self.eval_original_body_framed_with_inventory(
            tcl_registry::native_eval_object::EvalObjectPurpose::Eval,
            obj,
            frame,
            inventory,
        )
    }

    /// Evaluate an original object using its selected native dispatch recipe.
    pub(crate) fn eval_original_body_framed(
        &mut self,
        purpose: tcl_registry::native_eval_object::EvalObjectPurpose,
        original: *mut TclObj,
        frame: CmdFrame,
    ) -> Code {
        self.eval_original_body_framed_with_inventory(purpose, original, frame, &mut None)
    }

    fn eval_original_body_framed_with_inventory(
        &mut self,
        purpose: tcl_registry::native_eval_object::EvalObjectPurpose,
        original: *mut TclObj,
        frame: CmdFrame,
        inventory: &mut Option<Rc<native_jim_lookup::OriginalJimCommandInventoryReturn>>,
    ) -> Code {
        *inventory = None;
        let mut prepared = match self.prepare_original_body_evaluation(purpose, original) {
            Ok(prepared) => prepared,
            Err(code) => return code,
        };
        match &prepared.evaluation {
            OriginalBodyEvaluation::List | OriginalBodyEvaluation::Jim => {
                if !self.codegen_activation_enter() {
                    return Code::Error;
                }
                let code = if matches!(prepared.evaluation, OriginalBodyEvaluation::List) {
                    self.dispatch_list_obj(
                        prepared
                            .owner
                            .as_ref()
                            .expect("original body owner")
                            .as_ptr(),
                        frame,
                    )
                } else {
                    self.eval_native_jim_script_with_inventory(
                        prepared.owner.take().expect("original Jim body owner"),
                        frame,
                        inventory,
                    )
                };
                self.codegen_activation_leave(code);
                code
            }
            OriginalBodyEvaluation::Compiled(artifact) => {
                self.execute_original_c_body(artifact, frame)
            }
            OriginalBodyEvaluation::Source(bytes) => self.eval_framed(bytes, frame),
        }
    }

    /// Complete protocol, original-header and compiler selection before entering
    /// a recursive body. Keep setup/error temporaries out of its live host frame.
    #[inline(never)]
    fn prepare_original_body_evaluation(
        &mut self,
        purpose: tcl_registry::native_eval_object::EvalObjectPurpose,
        original: *mut TclObj,
    ) -> Result<Box<PreparedOriginalBodyEvaluation>, Code> {
        let owner = obj::Owned::retain(original);
        let Some(protocol) = self.eval_object_protocol() else {
            return Err(self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native script-object dispatch",
                )
                .into(),
            ));
        };
        let compiled_stack = matches!(
            purpose,
            tcl_registry::native_eval_object::EvalObjectPurpose::CompiledStack
        );
        if compiled_stack && !protocol.compiles_source(purpose) {
            return Err(self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native compiled-stack script-object dispatch",
                )
                .into(),
            ));
        }
        let snapshot = match obj::native_object_snapshot(owner.as_ptr()) {
            Ok(snapshot) => snapshot,
            Err(error) => return Err(self.report_cmd_error(error.into())),
        };
        let list_dispatch = protocol.dispatches_list(purpose, &snapshot);
        let jim_script = self.invocation_string_protocol()
            == Some(tcl_syntax::native_string::NativeStringProtocol::Jim084);
        if list_dispatch || jim_script {
            return Ok(Box::new(PreparedOriginalBodyEvaluation {
                evaluation: if list_dispatch {
                    OriginalBodyEvaluation::List
                } else {
                    OriginalBodyEvaluation::Jim
                },
                _snapshot: snapshot,
                owner: Some(owner),
            }));
        }
        if protocol.compiles_source(purpose)
            && self.native_invocation_dialect().tcl_version.is_some()
        {
            match self.prepare_original_c_body(owner.as_ptr(), self.current_ns.get(), None) {
                Ok(Some(artifact)) => {
                    return Ok(Box::new(PreparedOriginalBodyEvaluation {
                        evaluation: OriginalBodyEvaluation::Compiled(artifact),
                        _snapshot: snapshot,
                        owner: Some(owner),
                    }));
                }
                Ok(None) if compiled_stack => {
                    return Err(self.report_cmd_error(
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "native compiled-stack body artifact",
                        )
                        .into(),
                    ));
                }
                Ok(None) => {}
                Err(code) => return Err(code),
            }
        }
        match self.eval_object_bytes(owner.as_ptr()) {
            Ok(bytes) => Ok(Box::new(PreparedOriginalBodyEvaluation {
                evaluation: OriginalBodyEvaluation::Source(bytes),
                _snapshot: snapshot,
                owner: Some(owner),
            })),
            Err(error) => Err(self.report_cmd_error(error.into())),
        }
    }

    /// Dispatch a pure-list script object as a single command, using its element
    /// objects directly (no stringify/re-parse) — this preserves each element's
    /// `Tcl_Obj` identity, so a nested `eval`/`uplevel $bodyVar` still finds the
    /// body's TIP 280 source location.
    fn dispatch_list_obj(&mut self, original: *mut TclObj, mut frame: CmdFrame) -> Code {
        #[cfg(test)]
        self.observe_c84_error_episode("body.list_dispatch_enter", None);
        let owner = obj::Owned::retain(original);
        let Some(backing) = crate::list::native_list_backing(owner.as_ptr()) else {
            return self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "retained native List backing",
                )
                .into(),
            );
        };
        let elements = match backing.elements() {
            Ok(elements) => elements,
            Err(error) => return self.report_cmd_error(error.into()),
        };
        if elements.is_empty() {
            self.set_result_bytes(b"");
            return Code::Ok;
        }
        frame.original_command = Some(owner);
        frame.cmd.clear();
        frame.line = frame.line_base + 1;
        self.cmd_frames.borrow_mut().push(frame);
        let code = self.dispatch(&elements);
        #[cfg(test)]
        self.observe_c84_error_episode(
            "body.list_dispatch_return_error",
            Some(code == Code::Error),
        );
        self.cmd_frames.borrow_mut().pop();
        code
    }

    /// `uplevel` of a single body **object** — the redirected-scope variant of
    /// [`eval_body_obj`](Self::eval_body_obj). A located literal keeps its source
    /// provenance; a dynamic body is `type eval`, no file.
    pub(crate) fn eval_uplevel_obj(&mut self, target_level: usize, obj: *mut TclObj) -> Code {
        if let Err(first) = self.clear_return_options_checked() {
            return self.refuse_native_execution(first);
        }
        let loc = self.arg_loc(obj);
        let prev_level = self.frames.borrow_mut().set_active_level(target_level);
        let prev_ns = self.current_ns.get();
        self.current_ns
            .set(self.frames.borrow().frame_ns(target_level));
        let mut frame = self.inherited_cmd_frame();
        frame.level = target_level;
        frame.omit_level = true;
        match loc {
            Some((file, line)) => {
                frame.kind = FrameKind::Source;
                frame.file = file;
                frame.line_base = line.saturating_sub(1);
            }
            None => {
                frame.kind = FrameKind::Eval;
                frame.file = None;
                frame.line_base = 0;
            }
        }
        let code = self.eval_original_body_framed(
            tcl_registry::native_eval_object::EvalObjectPurpose::UpLevel,
            obj,
            frame,
        );
        self.frames.borrow_mut().set_active_level(prev_level);
        self.current_ns.set(prev_ns);
        code
    }
}

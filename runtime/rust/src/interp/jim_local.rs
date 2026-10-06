// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original argv evaluation and frame-owned Jim local cleanup names.

use super::{Code, Command, Interp};
use crate::obj::{self, Owned, TclObj};
use tcl_syntax::value::ValueOps;

struct LocalScope(Interp);
impl Drop for LocalScope {
    fn drop(&mut self) {
        self.0.jim_local_depth.set(self.0.jim_local_depth.get() - 1);
    }
}
struct UpcallScope {
    interp: Interp,
    token: u64,
}
impl Drop for UpcallScope {
    fn drop(&mut self) {
        self.interp
            .namespaces
            .borrow_mut()
            .leave_jim_upcall(self.token);
    }
}

impl Interp {
    pub(super) fn install_jim_local_commands(&mut self) {
        if self
            .native_invocation_dialect()
            .native_jim_local_protocol()
            .is_none()
        {
            return;
        }
        for (name, handler) in [
            (b"local".as_slice(), local as super::BuiltinFn),
            (b"upcall".as_slice(), upcall as super::BuiltinFn),
        ] {
            if self
                .namespaces
                .borrow()
                .resolve(crate::namespace::GLOBAL, name)
                .is_none()
            {
                self.register_builtin(name, handler);
            }
        }
    }
    pub(super) fn clean_current_jim_local_commands(&mut self) {
        let names = self.frames.borrow_mut().take_jim_local_commands();
        for name in names.into_iter().rev() {
            // Lookup caches can own a namespace, but cleanup's hash comparison
            // is the original flat name; it does not perform GetCommand again.
            let bytes = obj::bytes_of(name.as_ptr());
            self.namespaces.borrow_mut().clean_jim_local_key(&bytes);
        }
    }
}
fn local(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if interp
        .native_invocation_dialect()
        .native_jim_local_protocol()
        .is_none()
    {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("Jim local command").into(),
        );
    }
    if argv.len() < 2 {
        return interp.wrong_args(b"local cmd ?args ...?");
    }
    interp.jim_local_depth.set(interp.jim_local_depth.get() + 1);
    let scope = LocalScope(interp.clone());
    let code = interp.dispatch(&argv[1..]);
    drop(scope);
    if code != Code::Ok {
        return code;
    }
    let original = interp.get_obj_result();
    match interp.resolve_original_command(original) {
        Ok(Some(_)) => {
            interp
                .frames
                .borrow_mut()
                .retain_jim_local_command(Owned::retain(original));
            Code::Ok
        }
        Ok(None) => {
            let bytes = match interp.native_string_bytes(&original) {
                Ok(bytes) => bytes,
                Err(error) => return interp.report_cmd_error(error.into()),
            };
            let recipe = tcl_syntax::native_jim_local::NativeJimLocalProtocol::jim084();
            interp.error(&recipe.missing_cleanup_command(&bytes))
        }
        Err(error) => interp.report_cmd_error(error.into()),
    }
}
fn upcall(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    let Some(recipe) = interp
        .native_invocation_dialect()
        .native_jim_local_protocol()
    else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable("Jim upcall command").into(),
        );
    };
    if argv.len() < 2 {
        return interp.wrong_args(b"upcall cmd ?args ...?");
    }
    let selected = match interp.resolve_original_command(argv[1]) {
        Ok(selected) => selected,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    if let Some((command, Some(token))) = selected {
        if recipe.accepts_upcall(
            matches!(command, Command::Proc(_)),
            interp.namespaces.borrow().jim_has_previous(token),
        ) {
            // This local holds the selected actual worker throughout the scope;
            // the cache itself owns neither it nor its previous worker.
            interp.namespaces.borrow_mut().enter_jim_upcall(token);
            let scope = UpcallScope {
                interp: interp.clone(),
                token,
            };
            let code = interp.dispatch(&argv[1..]);
            drop(scope);
            return code;
        }
    }
    let bytes = match interp.native_string_bytes(&argv[1]) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    interp.error(&recipe.missing_previous_command(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_frames_and_nested_upcalls_match_original_native_lineage() {
        let fixture =
            include_str!("../../../../rust/tcl-syntax/tests/data/native_jim_local/behaviour.tsv");
        let mut checked = 0;
        for row in fixture.lines() {
            let fields: Vec<_> = row.split('\t').collect();
            let script = fields[2];
            let expected: Vec<u8> = fields[1]
                .as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect();
            crate::counters::reset();
            {
                let mut interp = Interp::new();
                interp.set_dialect_profile(crate::environment::profile_for_dialect("jim"));
                assert_eq!(
                    interp.eval_str(script.as_bytes()),
                    Code::Ok,
                    "{script}: {:?}",
                    interp.result_bytes()
                );
                assert_eq!(interp.result_bytes(), expected, "{script}");
            }
            assert_eq!(crate::counters::finalize(), 0, "{script}");
            checked += 1;
        }
        assert_eq!(checked, 5);
    }
}

//! Owned Jim interpreter retirement. Frame callbacks run before command
//! storage is retired; object roles retain their native release order.

/// An original object reference owned by the Jim interpreter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JimInterpreterObjectRole {
    /// The interpreter's empty object.
    Empty,
    /// The shared true integer.
    True,
    /// The shared false integer.
    False,
    /// Current command result.
    Result,
    /// Current stack trace.
    StackTrace,
    /// Current error procedure.
    ErrorProc,
    /// The original unknown-handler name.
    Unknown,
    /// The original deferred-script variable name.
    Defer,
    /// The null-script object.
    NullScript,
    /// Current source filename.
    CurrentFilename,
}

/// Physical interpreter owners consumed by selected Jim retirement.
pub trait JimInterpreterTeardown {
    /// Run pending frame defers while commands are live, then release frames.
    fn release_frames(&mut self);
    /// Retire the original command table and its procedure owners.
    fn release_commands(&mut self);
    /// Release exactly this interpreter's reference, outside storage guards.
    fn release_object(&mut self, role: JimInterpreterObjectRole);
    /// Invalidate cached command references after object-role retirement.
    fn invalidate_procedure_epoch(&mut self);
    /// Release packages and interpreter association data.
    fn release_packages_and_associations(&mut self);
    /// Release the original trace-command reference, if installed.
    fn release_trace_command(&mut self);
}

/// Consume real interpreter owners in `Jim_FreeInterp` order. The caller
/// selects this operation only for an independently authenticated Jim engine.
pub fn teardown_jim_interpreter(owners: &mut impl JimInterpreterTeardown) {
    use JimInterpreterObjectRole as Role;
    owners.release_frames();
    owners.release_commands();
    for role in [
        Role::Empty,
        Role::True,
        Role::False,
        Role::Result,
        Role::StackTrace,
        Role::ErrorProc,
        Role::Unknown,
        Role::Defer,
        Role::NullScript,
        Role::CurrentFilename,
    ] {
        owners.release_object(role);
    }
    owners.invalidate_procedure_epoch();
    owners.release_packages_and_associations();
    owners.release_trace_command();
}

/// Release a native call frame's original references after local-command
/// cleanup and before its variable table. Namespace storage may use a distinct
/// concrete owner type; no reference is cloned or reconstructed here.
pub fn release_jim_call_frame_objects<T, N>(
    parameters: Option<T>,
    body: Option<T>,
    namespace: Option<N>,
) {
    drop(parameters);
    drop(body);
    drop(namespace);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn departing_frame_releases_original_objects_before_variable_owners() {
        use std::{cell::RefCell, rc::Rc};
        struct Owner(&'static str, Rc<RefCell<Vec<&'static str>>>);
        impl Drop for Owner {
            fn drop(&mut self) {
                self.1.borrow_mut().push(self.0);
            }
        }
        let events = Rc::new(RefCell::new(Vec::new()));
        let variables = Owner("variables", Rc::clone(&events));
        release_jim_call_frame_objects(
            Some(Owner("parameters", Rc::clone(&events))),
            Some(Owner("body", Rc::clone(&events))),
            Some(Owner("namespace", Rc::clone(&events))),
        );
        assert_eq!(*events.borrow(), ["parameters", "body", "namespace"]);
        drop(variables);
        assert_eq!(
            *events.borrow(),
            ["parameters", "body", "namespace", "variables"]
        );
    }

    #[derive(Debug, PartialEq, Eq)]
    enum Event {
        Frames,
        Commands,
        Object(JimInterpreterObjectRole),
        Epoch,
        Packages,
        Trace,
    }

    struct Owners(Vec<Event>);
    impl JimInterpreterTeardown for Owners {
        fn release_frames(&mut self) {
            self.0.push(Event::Frames);
        }
        fn release_commands(&mut self) {
            self.0.push(Event::Commands);
        }
        fn release_object(&mut self, role: JimInterpreterObjectRole) {
            self.0.push(Event::Object(role));
        }
        fn invalidate_procedure_epoch(&mut self) {
            self.0.push(Event::Epoch);
        }
        fn release_packages_and_associations(&mut self) {
            self.0.push(Event::Packages);
        }
        fn release_trace_command(&mut self) {
            self.0.push(Event::Trace);
        }
    }

    #[test]
    fn interpreter_callbacks_precede_command_and_original_object_retirement() {
        use JimInterpreterObjectRole as R;
        let mut owners = Owners(Vec::new());
        teardown_jim_interpreter(&mut owners);
        assert_eq!(
            owners.0,
            vec![
                Event::Frames,
                Event::Commands,
                Event::Object(R::Empty),
                Event::Object(R::True),
                Event::Object(R::False),
                Event::Object(R::Result),
                Event::Object(R::StackTrace),
                Event::Object(R::ErrorProc),
                Event::Object(R::Unknown),
                Event::Object(R::Defer),
                Event::Object(R::NullScript),
                Event::Object(R::CurrentFilename),
                Event::Epoch,
                Event::Packages,
                Event::Trace,
            ]
        );
    }
}

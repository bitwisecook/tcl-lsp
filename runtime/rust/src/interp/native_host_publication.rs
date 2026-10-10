// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Owned native command-address receipts over the interpreter's actual tables.
//! Immediate registration uses the shared engine publication contract. There is
//! no deferred native overlay or original-object bridge on this adapter.

use super::{Command, Interp, InterpState, NsId};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use tcl_engine_api::{
    CommandPublicationKey, CommandPublicationPurpose, CommandPublicationService, EngineError,
    HostCommand, PreparedCommandPublication,
};
use tcl_syntax::naming::{NamePolicyAuthority, NamePolicyProtocol};

#[derive(Clone)]
pub(crate) struct HostRegistration {
    pub(crate) command: Rc<dyn HostCommand>,
    pub(crate) retiring: Rc<Cell<bool>>,
}

pub(crate) type HostCommands = Rc<RefCell<HashMap<u64, HostRegistration>>>;

pub(crate) struct PublicationService {
    interpreter: Weak<InterpState>,
    context: NsId,
    protocol: NamePolicyProtocol,
    hosts: HostCommands,
}

struct PublicationReceipt {
    interpreter: Weak<InterpState>,
    original: Rc<[u8]>,
    key: CommandPublicationKey,
    protocol: NamePolicyProtocol,
    context: NsId,
    purpose: CommandPublicationPurpose,
    generation: Option<u64>,
    consumed: Cell<bool>,
}

fn refusal(reason: &'static str) -> EngineError {
    EngineError::ExecutionRefusal(reason.into())
}

impl PublicationService {
    pub(crate) fn open(interp: &Interp, hosts: HostCommands) -> Result<Rc<Self>, EngineError> {
        let protocol = interp
            .name_policy_protocol()
            .filter(|policy| policy.authority() == NamePolicyAuthority::Native)
            .ok_or_else(|| refusal("native command publication policy is unavailable"))?;
        if !interp.namespaces().namespace_is_live(interp.current_ns()) {
            return Err(refusal("native command publication scope has retired"));
        }
        Ok(Rc::new(Self {
            interpreter: Rc::downgrade(&interp.0),
            context: interp.current_ns(),
            protocol,
            hosts,
        }))
    }

    fn interpreter(&self) -> Result<Interp, EngineError> {
        let interp = Interp(
            self.interpreter
                .upgrade()
                .ok_or_else(|| refusal("native command publication interpreter has retired"))?,
        );
        if interp.name_policy_protocol() != Some(self.protocol)
            || !interp.namespaces().namespace_is_live(self.context)
        {
            return Err(refusal(
                "native command publication policy or scope has changed",
            ));
        }
        Ok(interp)
    }

    fn classify_retirement(&self, interp: &Interp, generation: u64) -> Result<(), EngineError> {
        let namespaces = interp.namespaces();
        let mut selected = vec![generation];
        loop {
            let previous = selected.len();
            for candidate in namespaces.native_command_generations() {
                if selected.contains(&candidate) {
                    continue;
                }
                if let Some((
                    Command::Imported {
                        source_generation, ..
                    },
                    _,
                )) = namespaces.native_command_at_node(candidate)
                {
                    if selected.contains(&source_generation) {
                        selected.push(candidate);
                    }
                }
            }
            if previous == selected.len() {
                break;
            }
        }
        if interp.traces.borrow().cmd_traces.iter().any(|trace| {
            trace.ops & crate::cmd_trace::ops::DELETE != 0
                && trace.token.is_some_and(|token| selected.contains(&token))
        }) {
            return Err(refusal(
                "native command retirement requires synchronous guest traces",
            ));
        }
        for token in selected {
            if interp.coros.borrow().contains_key(&token) {
                return Err(refusal(
                    "native command retirement requires a coroutine handoff",
                ));
            }
            match namespaces
                .native_command_at_node(token)
                .map(|(command, _)| command)
            {
                Some(
                    Command::ChildInterp(_)
                    | Command::OoObject(_)
                    | Command::OoMy(_)
                    | Command::OoMyClass(_),
                ) => {
                    return Err(refusal(
                        "native command retirement requires guest lifecycle execution",
                    ));
                }
                Some(Command::ObjCmd(command))
                    if command.delete_proc.is_some()
                        && !self.hosts.borrow().contains_key(&token) =>
                {
                    return Err(refusal(
                        "native command retirement callback effects are unclassified",
                    ));
                }
                _ => {}
            }
        }
        Ok(())
    }
}

impl CommandPublicationService for PublicationService {
    fn prepare(
        &self,
        original: &[u8],
        purpose: CommandPublicationPurpose,
    ) -> Result<PreparedCommandPublication, EngineError> {
        let interp = self.interpreter()?;
        let slot = match purpose {
            CommandPublicationPurpose::CreateCCommand => interp
                .namespaces_mut()
                .command_c_api_publication_at(self.context, original)
                .ok_or_else(|| refusal("native C command publication address is unavailable"))?,
            CommandPublicationPurpose::DeleteCCommand => {
                let generation = interp
                    .namespaces()
                    .resolve_generation(self.context, original);
                if let Some(generation) = generation {
                    interp
                        .namespaces()
                        .native_command_slot_at_node(generation)
                        .ok_or_else(|| refusal("native command deletion slot is unavailable"))?
                } else {
                    (usize::MAX, original.to_vec())
                }
            }
        };
        let generation = (slot.0 != usize::MAX)
            .then(|| interp.namespaces().command_generation(slot.0, &slot.1))
            .flatten();
        if let Some(generation) = generation {
            self.classify_retirement(&interp, generation)?;
        }
        let key = CommandPublicationKey {
            owner: interp.native_command_interpreter.owner,
            interpreter: interp.native_command_interpreter.interpreter,
            namespace: if slot.0 == usize::MAX {
                u64::MAX
            } else {
                u64::try_from(slot.0).map_err(|_| refusal("native namespace token is exhausted"))?
            },
            simple: Rc::from(slot.1),
        };
        let original = Rc::from(original);
        let receipt = Rc::new(PublicationReceipt {
            interpreter: self.interpreter.clone(),
            original: Rc::clone(&original),
            key: key.clone(),
            protocol: self.protocol,
            context: self.context,
            purpose,
            generation,
            consumed: Cell::new(false),
        });
        Ok(PreparedCommandPublication {
            original,
            key,
            receipt,
        })
    }

    fn observed_presence(
        &self,
        publication: &PreparedCommandPublication,
    ) -> Result<bool, EngineError> {
        let interp = self.interpreter()?;
        let receipt = authenticate(&interp, publication, None)?;
        if let Some(generation) = receipt.generation {
            self.classify_retirement(&interp, generation)?;
        }
        Ok(receipt.generation.is_some())
    }
}

fn authenticate<'a>(
    interp: &Interp,
    publication: &'a PreparedCommandPublication,
    purpose: Option<CommandPublicationPurpose>,
) -> Result<&'a PublicationReceipt, EngineError> {
    let receipt = publication
        .receipt
        .downcast_ref::<PublicationReceipt>()
        .ok_or_else(|| refusal("foreign native command publication receipt"))?;
    let issuer = receipt
        .interpreter
        .upgrade()
        .ok_or_else(|| refusal("native command publication interpreter has retired"))?;
    if !Rc::ptr_eq(&issuer, &interp.0)
        || publication.key != receipt.key
        || publication.original != receipt.original
    {
        return Err(refusal(
            "foreign or altered native command publication receipt",
        ));
    }
    if receipt.consumed.get() || purpose.is_some_and(|purpose| purpose != receipt.purpose) {
        return Err(refusal(
            "native command publication receipt was consumed or has another purpose",
        ));
    }
    let namespaces = interp.namespaces();
    if interp.name_policy_protocol() != Some(receipt.protocol)
        || !namespaces.namespace_is_live(receipt.context)
    {
        return Err(refusal(
            "native command publication policy or scope has changed",
        ));
    }
    if receipt.key.namespace != u64::MAX {
        let namespace = usize::try_from(receipt.key.namespace)
            .map_err(|_| refusal("native command publication namespace token is unavailable"))?;
        if !namespaces.namespace_is_live(namespace) {
            return Err(refusal(
                "native command publication namespace incarnation has retired",
            ));
        }
        if namespaces.command_generation(namespace, &receipt.key.simple) != receipt.generation {
            return Err(refusal("native command publication binding has changed"));
        }
    }
    Ok(receipt)
}

pub(crate) fn consume(
    interp: &Interp,
    hosts: HostCommands,
    publication: &PreparedCommandPublication,
    purpose: CommandPublicationPurpose,
) -> Result<Option<(NsId, Rc<[u8]>, Option<u64>)>, EngineError> {
    let receipt = authenticate(interp, publication, Some(purpose))?;
    if let Some(generation) = receipt.generation {
        let service = PublicationService {
            interpreter: receipt.interpreter.clone(),
            context: receipt.context,
            protocol: receipt.protocol,
            hosts,
        };
        service.classify_retirement(interp, generation)?;
    }
    receipt.consumed.set(true);
    if receipt.key.namespace == u64::MAX {
        return Ok(None);
    }
    let namespace = usize::try_from(receipt.key.namespace)
        .map_err(|_| refusal("native namespace token is unavailable"))?;
    Ok(Some((
        namespace,
        Rc::clone(&receipt.key.simple),
        receipt.generation,
    )))
}

/// Recheck the actual incumbent after host callbacks, before any guest lifecycle.
pub(crate) fn ensure_retirement(
    interp: &Interp,
    hosts: HostCommands,
    generation: Option<u64>,
) -> Result<(), EngineError> {
    let Some(generation) = generation else {
        return Ok(());
    };
    let protocol = interp
        .name_policy_protocol()
        .ok_or_else(|| refusal("command retirement policy is unavailable"))?;
    let service = PublicationService {
        interpreter: Rc::downgrade(&interp.0),
        context: interp.current_ns(),
        protocol,
        hosts,
    };
    service.classify_retirement(interp, generation)
}

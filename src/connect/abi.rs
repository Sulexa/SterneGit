use std::fmt;

use sternegit_protocol::ABI;
use wasmtime::{Caller, Linker};

use super::{CallError, call::Call};
use crate::plugin::{FINISH, Import, LOG, NOW, TAKE, Ty, fault, guest_memory, length, link_common, span};

const SEND: Import = Import {
    name: "send",
    params: &[Ty::I32, Ty::I32, Ty::I32, Ty::I32],
    results: &[Ty::I32],
};

const WAIT: Import = Import {
    name: "wait",
    params: &[Ty::I32],
    results: &[Ty::I32],
};

pub(super) const IMPORTS: [Import; 6] = [TAKE, SEND, WAIT, FINISH, LOG, NOW];

#[derive(Debug)]
pub(super) struct Halted(pub(super) CallError);

impl fmt::Display for Halted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for Halted {}

pub(super) fn link(linker: &mut Linker<Call>) -> wasmtime::Result<()> {
    link_common(linker)?;
    linker.func_wrap(ABI, SEND.name, send)?;
    linker.func_wrap_async(ABI, WAIT.name, |caller: Caller<'_, Call>, (id,): (i32,)| {
        Box::new(wait(caller, id))
    })?;
    Ok(())
}

fn send(
    mut caller: Caller<'_, Call>,
    head: i32,
    head_len: i32,
    body: i32,
    body_len: i32,
) -> wasmtime::Result<i32> {
    let memory = guest_memory(&caller)?;
    let (bytes, call) = memory.data_and_store_mut(&mut caller);
    let head = &bytes[span("send", bytes.len(), head, length(head_len))?];
    let body = &bytes[span("send", bytes.len(), body, length(body_len))?];
    let id = call.send(head, body).map_err(|problem| fault("send", problem))?;
    Ok(id as i32)
}

async fn wait(mut caller: Caller<'_, Call>, id: i32) -> wasmtime::Result<i32> {
    let call = caller.data_mut();
    let response = match call.requests.claim(id as u32) {
        Ok(Some(response)) => response,
        Ok(None) => call
            .arrival(id as u32)
            .await
            .map_err(|error| wasmtime::Error::new(Halted(error)))?,
        Err(problem) => return Err(fault("wait", problem)),
    };
    Ok(call.deliver(response))
}

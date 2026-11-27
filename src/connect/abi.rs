use sternegit_protocol::ABI;
use wasmi::{Caller, Linker, ValType};

use super::call::Call;
use crate::plugin::{FINISH, Import, Interrupt, LOG, NOW, TAKE, fault, guest_memory, length, link_common, span};

const SEND: Import = Import {
    name: "send",
    params: &[ValType::I32, ValType::I32, ValType::I32, ValType::I32],
    results: &[ValType::I32],
};

const WAIT: Import = Import {
    name: "wait",
    params: &[ValType::I32],
    results: &[ValType::I32],
};

pub(super) const IMPORTS: [Import; 6] = [TAKE, SEND, WAIT, FINISH, LOG, NOW];

pub(super) fn link(linker: &mut Linker<Call>) -> Result<(), wasmi::errors::LinkerError> {
    link_common(linker)?;
    linker.func_wrap(ABI, SEND.name, send)?;
    linker.func_wrap(ABI, WAIT.name, wait)?;
    Ok(())
}

fn send(
    mut caller: Caller<'_, Call>,
    head: i32,
    head_len: i32,
    body: i32,
    body_len: i32,
) -> Result<i32, wasmi::Error> {
    let memory = guest_memory(&caller)?;
    let (bytes, call) = memory.data_and_store_mut(&mut caller);
    let head = &bytes[span("send", bytes.len(), head, length(head_len))?];
    let body = &bytes[span("send", bytes.len(), body, length(body_len))?];
    let id = call.send(head, body).map_err(|problem| fault("send", problem))?;
    Ok(id as i32)
}

fn wait(mut caller: Caller<'_, Call>, id: i32) -> Result<i32, wasmi::Error> {
    let call = caller.data_mut();
    match call.requests.claim(id as u32) {
        Ok(Some(response)) => Ok(call.deliver(response)),
        Ok(None) => Err(wasmi::Error::host(Interrupt::Wait(id as u32))),
        Err(problem) => Err(fault("wait", problem)),
    }
}

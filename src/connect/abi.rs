use std::{fmt, ops::Range};

use fastgit_protocol::ABI;
use wasmi::{Caller, Linker, Memory, errors::HostError};

use super::{call::Call, sandbox::MEMORY_EXPORT};

#[derive(Debug)]
pub(super) enum Interrupt {
    Wait(u32),
    Fault(String),
}

impl fmt::Display for Interrupt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Interrupt::Wait(id) => write!(f, "waiting for request {id}"),
            Interrupt::Fault(message) => f.write_str(message),
        }
    }
}

impl HostError for Interrupt {}

pub(super) fn link(linker: &mut Linker<Call>) -> Result<(), wasmi::errors::LinkerError> {
    linker.func_wrap(ABI, "take", take)?;
    linker.func_wrap(ABI, "send", send)?;
    linker.func_wrap(ABI, "wait", wait)?;
    linker.func_wrap(ABI, "finish", finish)?;
    linker.func_wrap(ABI, "log", log)?;
    linker.func_wrap(ABI, "now", now)?;
    Ok(())
}

fn take(mut caller: Caller<'_, Call>, into: i32) -> Result<(), wasmi::Error> {
    let memory = guest_memory(&caller)?;
    let (bytes, call) = memory.data_and_store_mut(&mut caller);
    let span = span("take", bytes.len(), into, call.waiting.len())?;
    bytes[span].copy_from_slice(&call.waiting);
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

fn finish(mut caller: Caller<'_, Call>, out: i32, len: i32) -> Result<(), wasmi::Error> {
    let memory = guest_memory(&caller)?;
    let (bytes, call) = memory.data_and_store_mut(&mut caller);
    let output = &bytes[span("finish", bytes.len(), out, length(len))?];
    call.finish(output).map_err(|problem| fault("finish", problem))
}

fn log(mut caller: Caller<'_, Call>, level: i32, text: i32, len: i32) -> Result<(), wasmi::Error> {
    let memory = guest_memory(&caller)?;
    let (bytes, call) = memory.data_and_store_mut(&mut caller);
    let text = &bytes[span("log", bytes.len(), text, length(len))?];
    call.log(level, text);
    Ok(())
}

fn now(caller: Caller<'_, Call>) -> i64 {
    caller.data().now()
}

fn guest_memory(caller: &Caller<'_, Call>) -> Result<Memory, wasmi::Error> {
    caller
        .data()
        .memory
        .ok_or_else(|| fault("memory", format!("the module exports no {MEMORY_EXPORT}")))
}

fn length(len: i32) -> usize {
    len as u32 as usize
}

fn span(import: &str, memory: usize, at: i32, len: usize) -> Result<Range<usize>, wasmi::Error> {
    let start = at as u32 as usize;
    start
        .checked_add(len)
        .filter(|end| *end <= memory)
        .map(|end| start..end)
        .ok_or_else(|| {
            fault(
                import,
                format!("{len} bytes at {start} lie outside the connector's {memory} bytes of memory"),
            )
        })
}

fn fault(import: &str, problem: String) -> wasmi::Error {
    wasmi::Error::host(Interrupt::Fault(format!("{import}: {problem}")))
}

use std::{fmt, ops::Range};

use sternegit_protocol::ABI;
use wasmtime::{Caller, Linker, Memory};

use super::MEMORY_EXPORT;

#[derive(Debug)]
pub(crate) struct Fault(pub(crate) String);

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Fault {}

pub(crate) trait Guest: Send + Sized + 'static {
    fn memory(&self) -> Option<Memory>;
    fn waiting(&self) -> &[u8];
    fn finish(&mut self, output: &[u8]) -> Result<(), String>;
    fn log(&mut self, level: i32, text: &[u8]);
    fn now(&self) -> i64;
}

pub(crate) fn link_common<T: Guest>(linker: &mut Linker<T>) -> wasmtime::Result<()> {
    linker.func_wrap(ABI, "take", take::<T>)?;
    linker.func_wrap(ABI, "finish", finish::<T>)?;
    linker.func_wrap(ABI, "log", log::<T>)?;
    linker.func_wrap(ABI, "now", now::<T>)?;
    Ok(())
}

fn take<T: Guest>(mut caller: Caller<'_, T>, into: i32) -> wasmtime::Result<()> {
    let memory = guest_memory(&caller)?;
    let (bytes, guest) = memory.data_and_store_mut(&mut caller);
    let waiting = guest.waiting();
    let span = span("take", bytes.len(), into, waiting.len())?;
    bytes[span].copy_from_slice(waiting);
    Ok(())
}

fn finish<T: Guest>(mut caller: Caller<'_, T>, out: i32, len: i32) -> wasmtime::Result<()> {
    let memory = guest_memory(&caller)?;
    let (bytes, guest) = memory.data_and_store_mut(&mut caller);
    let output = &bytes[span("finish", bytes.len(), out, length(len))?];
    guest.finish(output).map_err(|problem| fault("finish", problem))
}

fn log<T: Guest>(mut caller: Caller<'_, T>, level: i32, text: i32, len: i32) -> wasmtime::Result<()> {
    let memory = guest_memory(&caller)?;
    let (bytes, guest) = memory.data_and_store_mut(&mut caller);
    let text = &bytes[span("log", bytes.len(), text, length(len))?];
    guest.log(level, text);
    Ok(())
}

fn now<T: Guest>(caller: Caller<'_, T>) -> i64 {
    caller.data().now()
}

pub(crate) fn guest_memory<T: Guest>(caller: &Caller<'_, T>) -> wasmtime::Result<Memory> {
    caller
        .data()
        .memory()
        .ok_or_else(|| fault("memory", format!("the module exports no {MEMORY_EXPORT}")))
}

pub(crate) fn length(len: i32) -> usize {
    len as u32 as usize
}

pub(crate) fn span(import: &str, memory: usize, at: i32, len: usize) -> wasmtime::Result<Range<usize>> {
    let start = at as u32 as usize;
    start
        .checked_add(len)
        .filter(|end| *end <= memory)
        .map(|end| start..end)
        .ok_or_else(|| {
            fault(
                import,
                format!("{len} bytes at {start} lie outside the module's {memory} bytes of memory"),
            )
        })
}

pub(crate) fn fault(import: &str, problem: String) -> wasmtime::Error {
    wasmtime::Error::new(Fault(format!("{import}: {problem}")))
}

pub(crate) fn trap_message(error: &wasmtime::Error) -> String {
    match error.downcast_ref::<Fault>() {
        Some(fault) => fault.0.clone(),
        None => match error.downcast_ref::<wasmtime::Trap>() {
            Some(trap) => trap.to_string(),
            None => error.root_cause().to_string(),
        },
    }
}

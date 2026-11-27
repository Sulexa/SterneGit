use sternegit_protocol::ABI;
use wasmi::{
    CompilationMode, Config, EnforcedLimits, Engine, ExternType, Module, StoreLimits, StoreLimitsBuilder, TrapCode,
    ValType,
    errors::{ErrorKind, InstantiationError, MemoryError, TableError},
};

use super::Stop;

pub(crate) const MEMORY_EXPORT: &str = "memory";
pub(crate) const ENTRY_EXPORT: &str = "sternegit_call";
pub(crate) const FUEL_PER_SLICE: u64 = 20_000_000;

const TABLES: usize = 4;
const TABLE_ELEMENTS: usize = 20_000;
const MAX_RECURSION_DEPTH: usize = 4096;

pub(crate) struct Import {
    pub(crate) name: &'static str,
    pub(crate) params: &'static [ValType],
    pub(crate) results: &'static [ValType],
}

pub(crate) const TAKE: Import = Import {
    name: "take",
    params: &[ValType::I32],
    results: &[],
};

pub(crate) const FINISH: Import = Import {
    name: "finish",
    params: &[ValType::I32, ValType::I32],
    results: &[],
};

pub(crate) const LOG: Import = Import {
    name: "log",
    params: &[ValType::I32, ValType::I32, ValType::I32],
    results: &[],
};

pub(crate) const NOW: Import = Import {
    name: "now",
    params: &[],
    results: &[ValType::I64],
};

pub(crate) fn engine() -> Engine {
    let mut config = Config::default();
    config
        .consume_fuel(true)
        .compilation_mode(CompilationMode::LazyTranslation)
        .allow_start_fn(false)
        .wasm_multi_memory(false)
        .set_max_recursion_depth(MAX_RECURSION_DEPTH)
        .enforced_limits(EnforcedLimits::strict());
    Engine::new(&config)
}

pub(crate) fn limits(memory_bytes: usize) -> StoreLimits {
    StoreLimitsBuilder::new()
        .memory_size(memory_bytes)
        .memories(1)
        .instances(1)
        .tables(TABLES)
        .table_elements(TABLE_ELEMENTS)
        .trap_on_grow_failure(true)
        .build()
}

pub(crate) fn check_module(module: &Module, offered: &[Import]) -> Result<(), Stop> {
    refuse_foreign_imports(module, offered)?;
    require_exports(module)
}

pub(crate) fn stop_for(error: &wasmi::Error, memory_bytes: usize) -> Option<Stop> {
    if ran_out_of_memory(error) {
        Some(Stop::OutOfMemory {
            limit_bytes: memory_bytes,
        })
    } else if error.as_trap_code() == Some(TrapCode::OutOfFuel) {
        Some(Stop::OutOfFuel)
    } else {
        None
    }
}

fn refuse_foreign_imports(module: &Module, offered: &[Import]) -> Result<(), Stop> {
    for import in module.imports() {
        let matching = (import.module() == ABI)
            .then(|| offered.iter().find(|offered| offered.name == import.name()))
            .flatten();
        let fits = match (matching, import.ty()) {
            (Some(offered), ExternType::Func(ty)) => ty.params() == offered.params && ty.results() == offered.results,
            _ => false,
        };
        if !fits {
            return Err(Stop::Refused(format!(
                "The module imports {}::{}, which SterneGit doesn't offer",
                import.module(),
                import.name()
            )));
        }
    }
    Ok(())
}

fn require_exports(module: &Module) -> Result<(), Stop> {
    let exported = |name: &str| {
        module
            .exports()
            .find(|export| export.name() == name)
            .map(|export| export.ty().clone())
    };
    if !matches!(exported(MEMORY_EXPORT), Some(ExternType::Memory(_))) {
        return Err(Stop::Refused(format!("The module exports no {MEMORY_EXPORT}")));
    }
    match exported(ENTRY_EXPORT) {
        Some(ExternType::Func(ty)) if ty.params() == [ValType::I32] && ty.results() == [ValType::I32] => Ok(()),
        _ => Err(Stop::Refused(format!("The module exports no {ENTRY_EXPORT}(i32) -> i32"))),
    }
}

fn ran_out_of_memory(error: &wasmi::Error) -> bool {
    let memory_denied = |error: &MemoryError| {
        matches!(
            error,
            MemoryError::ResourceLimiterDeniedAllocation | MemoryError::OutOfSystemMemory
        )
    };
    let table_denied = |error: &TableError| matches!(error, TableError::ResourceLimiterDeniedAllocation);
    error.as_trap_code() == Some(TrapCode::GrowthOperationLimited)
        || match error.kind() {
            ErrorKind::Memory(error)
            | ErrorKind::Instantiation(InstantiationError::FailedToInstantiateMemory(error)) => memory_denied(error),
            ErrorKind::Table(error)
            | ErrorKind::Instantiation(InstantiationError::FailedToInstantiateTable(error)) => table_denied(error),
            _ => false,
        }
}

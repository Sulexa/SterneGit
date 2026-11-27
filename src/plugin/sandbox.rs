use std::sync::OnceLock;

use wasmtime::{
    Config, Engine, ExternType, Module, ResourceLimiter, Trap, ValType,
    wasmparser::{Parser, Payload},
};

use super::Stop;

pub(crate) const MEMORY_EXPORT: &str = "memory";
pub(crate) const ENTRY_EXPORT: &str = "sternegit_call";
pub(crate) const FUEL_PER_SLICE: u64 = 20_000_000;

const TABLES: usize = 4;
const TABLE_ELEMENTS: usize = 20_000;
const WASM_STACK_BYTES: usize = 1024 * 1024;
const HOST_STACK_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ty {
    I32,
    I64,
}

impl Ty {
    fn is(self, ty: &ValType) -> bool {
        match self {
            Ty::I32 => matches!(ty, ValType::I32),
            Ty::I64 => matches!(ty, ValType::I64),
        }
    }
}

pub(crate) struct Import {
    pub(crate) name: &'static str,
    pub(crate) params: &'static [Ty],
    pub(crate) results: &'static [Ty],
}

pub(crate) const TAKE: Import = Import {
    name: "take",
    params: &[Ty::I32],
    results: &[],
};

pub(crate) const FINISH: Import = Import {
    name: "finish",
    params: &[Ty::I32, Ty::I32],
    results: &[],
};

pub(crate) const LOG: Import = Import {
    name: "log",
    params: &[Ty::I32, Ty::I32, Ty::I32],
    results: &[],
};

pub(crate) const NOW: Import = Import {
    name: "now",
    params: &[],
    results: &[Ty::I64],
};

pub(crate) fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| {
        let mut config = Config::new();
        config
            .consume_fuel(true)
            .wasm_multi_memory(false)
            .max_wasm_stack(WASM_STACK_BYTES)
            .async_stack_size(WASM_STACK_BYTES + HOST_STACK_BYTES);
        Engine::new(&config).expect("SterneGit's engine settings are valid")
    })
}

pub(crate) fn compile(wasm: &[u8], offered: &[Import]) -> Result<Module, Stop> {
    refuse_start_function(wasm)?;
    let module = Module::new(engine(), wasm).map_err(|error| Stop::Refused(format!("The module can't be loaded: {error}")))?;
    refuse_foreign_imports(&module, offered)?;
    require_exports(&module)?;
    Ok(module)
}

pub(crate) struct Limits {
    memory_bytes: usize,
    denied: bool,
}

impl Limits {
    pub(crate) fn new(memory_bytes: usize) -> Self {
        Self {
            memory_bytes,
            denied: false,
        }
    }

    pub(crate) fn stop_for(&self, error: &wasmtime::Error) -> Option<Stop> {
        if self.denied {
            Some(Stop::OutOfMemory {
                limit_bytes: self.memory_bytes,
            })
        } else if matches!(error.downcast_ref::<Trap>(), Some(Trap::OutOfFuel)) {
            Some(Stop::OutOfFuel)
        } else {
            None
        }
    }

    fn deny(&mut self) -> wasmtime::Result<bool> {
        self.denied = true;
        Err(wasmtime::Error::msg("the module needed more memory than SterneGit allows"))
    }
}

impl ResourceLimiter for Limits {
    fn memory_growing(&mut self, _: usize, desired: usize, _: Option<usize>) -> wasmtime::Result<bool> {
        if desired > self.memory_bytes { self.deny() } else { Ok(true) }
    }

    fn table_growing(&mut self, _: usize, desired: usize, _: Option<usize>) -> wasmtime::Result<bool> {
        if desired > TABLE_ELEMENTS { self.deny() } else { Ok(true) }
    }

    fn instances(&self) -> usize {
        1
    }

    fn tables(&self) -> usize {
        TABLES
    }

    fn memories(&self) -> usize {
        1
    }
}

fn refuse_start_function(wasm: &[u8]) -> Result<(), Stop> {
    for payload in Parser::new(0).parse_all(wasm) {
        match payload {
            Ok(Payload::StartSection { .. }) => {
                return Err(Stop::Refused("The module has a start function, which SterneGit doesn't run".to_owned()));
            }
            Ok(_) => {}
            Err(error) => return Err(Stop::Refused(format!("The module can't be loaded: {error}"))),
        }
    }
    Ok(())
}

fn refuse_foreign_imports(module: &Module, offered: &[Import]) -> Result<(), Stop> {
    for import in module.imports() {
        let matching = (import.module() == sternegit_protocol::ABI)
            .then(|| offered.iter().find(|offered| offered.name == import.name()))
            .flatten();
        let fits = match (matching, import.ty()) {
            (Some(offered), ExternType::Func(ty)) => {
                same_types(ty.params(), offered.params) && same_types(ty.results(), offered.results)
            }
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

fn same_types(found: impl ExactSizeIterator<Item = ValType>, offered: &[Ty]) -> bool {
    found.len() == offered.len() && found.zip(offered).all(|(found, offered)| offered.is(&found))
}

fn require_exports(module: &Module) -> Result<(), Stop> {
    if !matches!(module.get_export(MEMORY_EXPORT), Some(ExternType::Memory(_))) {
        return Err(Stop::Refused(format!("The module exports no {MEMORY_EXPORT}")));
    }
    let entry_fits = match module.get_export(ENTRY_EXPORT) {
        Some(ExternType::Func(ty)) => same_types(ty.params(), &[Ty::I32]) && same_types(ty.results(), &[Ty::I32]),
        _ => false,
    };
    if entry_fits {
        Ok(())
    } else {
        Err(Stop::Refused(format!("The module exports no {ENTRY_EXPORT}(i32) -> i32")))
    }
}

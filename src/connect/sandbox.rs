use wasmi::{
    CompilationMode, Config, EnforcedLimits, Engine, StoreLimits, StoreLimitsBuilder, ValType,
};

pub(super) const MEMORY_EXPORT: &str = "memory";
pub(super) const ENTRY_EXPORT: &str = "fastgit_call";
pub(super) const FUEL_PER_SLICE: u64 = 20_000_000;
pub(super) const MEMORY_LIMIT_BYTES: usize = 64 * 1024 * 1024;

const TABLES: usize = 4;
const TABLE_ELEMENTS: usize = 20_000;
const MAX_RECURSION_DEPTH: usize = 4096;

pub(super) struct Import {
    pub(super) name: &'static str,
    pub(super) params: &'static [ValType],
    pub(super) results: &'static [ValType],
}

pub(super) const IMPORTS: [Import; 6] = [
    Import {
        name: "take",
        params: &[ValType::I32],
        results: &[],
    },
    Import {
        name: "send",
        params: &[ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        results: &[ValType::I32],
    },
    Import {
        name: "wait",
        params: &[ValType::I32],
        results: &[ValType::I32],
    },
    Import {
        name: "finish",
        params: &[ValType::I32, ValType::I32],
        results: &[],
    },
    Import {
        name: "log",
        params: &[ValType::I32, ValType::I32, ValType::I32],
        results: &[],
    },
    Import {
        name: "now",
        params: &[],
        results: &[ValType::I64],
    },
];

pub(super) fn engine() -> Engine {
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

pub(super) fn limits() -> StoreLimits {
    StoreLimitsBuilder::new()
        .memory_size(MEMORY_LIMIT_BYTES)
        .memories(1)
        .instances(1)
        .tables(TABLES)
        .table_elements(TABLE_ELEMENTS)
        .trap_on_grow_failure(true)
        .build()
}

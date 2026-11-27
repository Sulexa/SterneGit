use crate::host;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn code(self) -> i32 {
        match self {
            Level::Debug => 0,
            Level::Info => 1,
            Level::Warn => 2,
            Level::Error => 3,
        }
    }
}

pub fn log(level: Level, message: &str) {
    host::log(level, message);
}

pub fn now() -> i64 {
    host::now()
}

#[cfg(target_arch = "wasm32")]
pub(crate) fn report_panics() {
    static HOOKED: std::sync::Once = std::sync::Once::new();
    HOOKED.call_once(|| {
        std::panic::set_hook(Box::new(|panic| log(Level::Error, &panic.to_string())));
    });
}

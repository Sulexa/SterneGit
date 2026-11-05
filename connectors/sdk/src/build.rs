pub const STACK_BYTES: u32 = 128 * 1024;

pub fn link_small_stack() {
    if std::env::var("CARGO_CFG_TARGET_ARCH").is_ok_and(|arch| arch == "wasm32") {
        println!("cargo:rustc-link-arg=-zstack-size={STACK_BYTES}");
    }
}

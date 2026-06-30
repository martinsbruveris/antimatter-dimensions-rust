fn main() {
    // A PyO3 `extension-module` deliberately does not link libpython — the host
    // interpreter supplies the `_Py*` symbols when it imports the module. On macOS
    // the final cdylib link must therefore be told to leave those symbols undefined
    // and resolve them dynamically at load time. `maturin` passes these flags for
    // us, but a plain `cargo build` / `cargo test` does not, and PyO3's own build
    // script can't add them (build-script link args apply only to the crate that
    // owns them). Without this, the workspace build fails with
    // "ld: symbol(s) not found for architecture arm64" for `_PyBytes_AsString` etc.
    //
    // Linux allows undefined symbols in a shared object by default and Windows links
    // pythonXY.lib, so this is needed on macOS only. We read CARGO_CFG_TARGET_OS (the
    // *target*) rather than #[cfg] (the host) so cross-compiles stay correct.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-cdylib-link-arg=-undefined");
        println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
    }
}

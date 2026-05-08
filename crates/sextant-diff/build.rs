//! Build script for `sextant-diff`.
//!
//! On Windows targets, libgit2-sys's static lib references `advapi32`
//! symbols (registry, security info, crypto). The `advapi32.lib` import
//! library used to land on the link line transitively, but a 2026
//! Windows-runner / MSVC update stopped providing it implicitly. Emit
//! the link directive explicitly so the linker can find
//! `RegOpenKeyExW`, `GetNamedSecurityInfoW`, `CryptGenRandom`, etc.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-lib=advapi32");
    }
}

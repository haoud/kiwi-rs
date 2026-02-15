fn main() {
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    println!("cargo:rustc-link-arg=-Tsrc/arch/{arch}/config/link.ld");
    println!("cargo:rerun-if-changed=src/arch/{arch}/config/link.ld");
    println!("cargo:rerun-if-changed=build.rs");
}

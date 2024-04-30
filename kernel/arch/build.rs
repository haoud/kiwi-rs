use std::{env, fs, path::PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    fs::write(
        out_dir.join("link.ld"),
        include_bytes!("src/riscv64/config/link.ld"),
    )
    .unwrap();
    println!("cargo:rustc-link-search={}", out_dir.display());
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=link.ld");
}

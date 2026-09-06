use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let lib_dir = manifest_dir.join("deps/SDL2/lib/x64");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    // Windows loads DLLs from the executable's own directory, so copy
    // SDL2.dll next to whatever binary cargo is about to produce.
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let profile_dir = out_dir.ancestors().nth(3).unwrap();
    let _ = fs::copy(lib_dir.join("SDL2.dll"), profile_dir.join("SDL2.dll"));
}

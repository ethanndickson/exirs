use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rustc-link-search=native=exirs-sys/exip/bin/lib/");
    println!("cargo:rustc-link-lib=static=exip");
    let bindings = bindgen::Builder::default()
        .header("exip-wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .unwrap();
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings.write_to_file(out.join("bindings.rs")).unwrap();
}

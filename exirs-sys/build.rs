use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rustc-link-search=native=exirs-sys/exip/bin/lib/");
    println!("cargo:rustc-link-lib=exip");
    Command::new("make")
        .env_remove("TARGET")
        .arg("-C")
        .arg("exip/build/gcc")
        .arg("all")
        .status()
        .expect("Failed to make");
    let bindings = bindgen::Builder::default()
        .header("exip-wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .unwrap();
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings.write_to_file(out.join("bindings.rs")).unwrap();
}

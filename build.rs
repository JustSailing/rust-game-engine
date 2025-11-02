use core::panic;
use std::process::Command;

fn main() {
    println!("cargo:rustc-link-arg=-fuse-ld=mold");
    println!("cargo:rustc-link-lib=X11");
    println!("cargo:rustc-link-lib=vulkan");
    compile_shaders();
    println!("cargo:rerun-if-changed=bin/assets/shaders");
    println!("cargo:rerun-if-changed=assets/shaders");
}

fn compile_shaders() {
    let bash_script_path = "compile_shaders.sh";

    let output = Command::new("bash")
        .arg(bash_script_path)
        .status()
        .expect("Failed to execute bash script");

    if !output.success() {
        eprintln!("Bash script execution failed!");
        panic!()
    }
}

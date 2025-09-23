use std::process::Command;

fn main() {
    println!("cargo:rustc-link-lib=X11");
    println!("cargo:rustc-link-lib=vulkan");
    compile_shaders();
    println!("cargo:rerun-if-changed=bin/assets/shaders");
    println!("cargo:rerun-if-changed=assets/shaders");
}

fn compile_shaders() {
    let bash_script_path = "compile_shaders.sh";

    let output = Command::new("bash") // Or "bash"
        .arg(bash_script_path)
        .status() // Use .status() if you only need the exit code, or .output() for stdout/stderr
        .expect("Failed to execute bash script");

    // Check if the command executed successfully
    if !output.success() {
        eprintln!("Bash script execution failed!");
        // You might want to panic! here to stop the build
    }
}

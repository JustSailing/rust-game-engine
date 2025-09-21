use std::process::Command;

fn main() {
    compile_shaders();
    println!("cargo:rustc-link-lib=X11");
    println!("cargo:rustc-link-lib=vulkan")
}

fn compile_shaders() {
    let bash_script_path = "compile_shaders.sh";

    // Execute the bash script using sh or bash
    #[cfg(debug_assertions)]
    let output = Command::new("bash") // Or "bash"
        .arg(bash_script_path)
        .arg("debug")
        .status() // Use .status() if you only need the exit code, or .output() for stdout/stderr
        .expect("Failed to execute bash script");

    #[cfg(not(debug_assertions))]
    let output = Command::new("bash") // Or "bash"
        .arg(bash_script_path)
        .arg("release")
        .status() // Use .status() if you only need the exit code, or .output() for stdout/stderr
        .expect("Failed to execute bash script");

    // Check if the command executed successfully
    if !output.success() {
        eprintln!("Bash script execution failed!");
        // You might want to panic! here to stop the build
    }
}

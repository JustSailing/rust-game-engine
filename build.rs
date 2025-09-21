use shaderc::{CompileOptions, Compiler};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

fn main() {
    compile_shaders();
    println!("cargo:rustc-link-lib=X11");
    println!("cargo:rustc-link-lib=vulkan")
}

fn compile_shaders() {
    let paths = fs::read_dir("assets/shaders").unwrap();
    let compiler = Compiler::new().unwrap();
    let options = CompileOptions::new().unwrap();

    // println!("The current directory is: {}", current_dir.display());
    for p in paths {
        let current_dir = env::current_dir().unwrap();
        println!("current dir: {:?}", current_dir);
        let name = p.unwrap().path();
        println!("input file: {:?}", name);
        let s = name.to_str().unwrap();
        let spl = s.split(".").collect::<Vec<_>>();
        #[cfg(debug_assertions)]
        let output_dir = Path::new("target/debug");
        #[cfg(not(debug_assertions))]
        let output_dir = Path::new("target/release");
        let mut output_file = current_dir.join(output_dir).join(name.as_path());
        output_file.set_extension("spv");
        println!("output file: {:?}", output_file);
        if spl[spl.len() - 2] == "vert" {
            let mut file = std::fs::File::create(&output_file).unwrap();
            let shader_source = fs::read_to_string(&name).unwrap();
            let artifact = compiler
                .compile_into_spirv(
                    &shader_source,
                    shaderc::ShaderKind::Vertex,
                    name.to_str().unwrap(),
                    "main",
                    Some(&options),
                )
                .unwrap();

            file.write_all(artifact.as_binary_u8()).unwrap();
        } else if spl[spl.len() - 2] == "frag" {
            let mut file = std::fs::File::create(&output_file).unwrap();
            let shader_source = fs::read_to_string(&name).unwrap();
            let artifact = compiler
                .compile_into_spirv(
                    &shader_source,
                    shaderc::ShaderKind::Vertex,
                    name.to_str().unwrap(),
                    "main",
                    Some(&options),
                )
                .unwrap();

            file.write_all(artifact.as_binary_u8()).unwrap();
        }
    }
}

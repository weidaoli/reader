use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=src/main.rs");
    
    if env::var("CROSS_COMPILE").is_ok() {
        println!("cargo:warning=开始跨平台编译...");
        
        let output_dir = "target/cross-compile";
        if !Path::new(output_dir).exists() {
            fs::create_dir_all(output_dir).expect("无法创建输出目录");
        }
        
        let target_triple = env::var("TARGET").unwrap_or_else(|_| String::from("unknown"));
        println!("cargo:warning=当前目标平台: {}", target_triple);
        
        if compile_for_target("x86_64-pc-windows-gnu", output_dir, true) {
            println!("cargo:warning=Windows (x64) 版本编译成功!");
        } else {
            println!("cargo:warning=Windows (x64) 版本编译失败!");
        }
        
        if compile_for_target("x86_64-unknown-linux-gnu", output_dir, false) {
            println!("cargo:warning=Linux (x64) 版本编译成功!");
        } else {
            println!("cargo:warning=Linux (x64) 版本编译失败!");
        }
        
        if compile_for_target("x86_64-apple-darwin", output_dir, false) {
            println!("cargo:warning=macOS (x64) 版本编译成功!");
        } else {
            println!("cargo:warning=macOS (x64) 版本编译失败!");
        }

        println!("cargo:warning=跨平台编译完成! 可执行文件位于 {} 目录", output_dir);
    }
}

fn compile_for_target(target: &str, output_dir: &str, is_windows: bool) -> bool {
    println!("cargo:warning=正在编译 {} 版本...", target);
    
    let status = Command::new("rustup")
        .args(&["target", "add", target])
        .status();

    if let Err(e) = status {
        println!("cargo:warning=无法添加目标平台 {}: {}", target, e);
        return false;
    }
    
    let status = Command::new("cargo")
        .args(&["build", "--release", "--target", target])
        .status();

    match status {
        Ok(exit_status) if exit_status.success() => {
            
            let binary_name = if is_windows { "clireader.exe" } else { "clireader" };
            let src_path = format!("target/{}/release/{}", target, binary_name);
            let dst_name = if is_windows {
                format!("clireader-{}.exe", target)
            } else {
                format!("clireader-{}", target)
            };
            let dst_path = format!("{}/{}", output_dir, dst_name);

            match fs::copy(&src_path, &dst_path) {
                Ok(_) => {
                    println!("cargo:warning=已复制 {} 到 {}", src_path, dst_path);
                    true
                },
                Err(e) => {
                    println!("cargo:warning=无法复制 {}: {}", src_path, e);
                    false
                }
            }
        },
        Ok(_) => {
            println!("cargo:warning=编译 {} 失败", target);
            false
        },
        Err(e) => {
            println!("cargo:warning=编译 {} 时出错: {}", target, e);
            false
        }
    }
}
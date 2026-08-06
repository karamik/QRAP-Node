use std::env;
use std::process::Command;

fn main() {
    let target = env::var("TARGET").unwrap();

    // Termux / Android — skip C++ compilation
    if target.contains("aarch64") || target.contains("android") {
        return;
    }

    // AWS-F1 feature detected via cargo env var
    if env::var("CARGO_FEATURE_AWS_F1").is_ok() {
        let has_opencl = Command::new("pkg-config")
            .args(["--exists", "OpenCL"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        let mut build = cc::Build::new();
        build
            .cpp(true)
            .include("src/aws_f1/kernels")
            .flag("-std=c++14")
            .flag("-O2");

        if has_opencl {
            build.file("src/aws_f1/host/qrap_f1_host.cpp");
            println!("cargo:rustc-link-lib=OpenCL");
        } else {
            build.file("src/aws_f1/host/stub.cpp");
            println!("cargo:warning=OpenCL not found, using stub F1 host");
        }
        build.compile("qrap_f1_host");
    }
}

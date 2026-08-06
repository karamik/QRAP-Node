use std::env;

fn main() {
    let target = env::var("TARGET").unwrap();

    // Termux / Android — skip C++ compilation, no FPGA toolchain
    if target.contains("aarch64") || target.contains("android") {
        return;
    }

    // AWS-F1 feature detected via cargo env var
    if env::var("CARGO_FEATURE_AWS_F1").is_ok() {
        cc::Build::new()
            .cpp(true)
            .file("src/aws_f1/host/qrap_f1_host.cpp")
            .include("src/aws_f1/kernels")
            .flag("-std=c++14")
            .flag("-O2")
            .compile("qrap_f1_host");
        println!("cargo:rustc-link-lib=OpenCL");
    }
}

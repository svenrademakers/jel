use std::env;

fn main() {
    env::set_var("CC", "clang");
    env::set_var("CXX", "clang++");

    println!("cargo:rustc-link-lib=avcodec");
    println!("cargo:rustc-link-lib=avformat");
    println!("cargo:rustc-link-lib=avutil");
    println!("cargo:rustc-link-lib=swscale");
    println!("cargo:rustc-link-lib=swresample");
    println!("cargo:rustc-link-lib=avfilter");

    println!("cargo:rerun-if-changed=src/screen_grabber.cpp");
    println!("cargo:rerun-if-changed=src/tracing.hpp");
    cc::Build::new()
        .cpp(true)
        .file("src/screen_grabber.cpp")
        .cpp_set_stdlib("c++")
        .flag("-std=c++23")
        .flag("-O3")
        .compile("screen_grabber");
}

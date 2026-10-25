//! Compiles the vendored reference decoder.
//!
//! It is C++ and it stays C++: `vendor/s4285-ref` is upstream unmodified, and
//! the only thing built here is our own shim around it.

fn main() {
    println!("cargo:rerun-if-changed=src/shim.cc");
    println!("cargo:rerun-if-changed=../../vendor/s4285-ref/include");
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .include("../../vendor/s4285-ref/include")
        .file("src/shim.cc")
        .warnings(false)
        .compile("s4285ref");
}

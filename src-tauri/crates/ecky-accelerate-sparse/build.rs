fn main() {
    println!("cargo:rerun-if-changed=native/accelerate_sparse.cpp");
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .flag_if_supported("-O3")
        .file("native/accelerate_sparse.cpp")
        .compile("ecky_accelerate_sparse");
    println!("cargo:rustc-link-lib=framework=Accelerate");
}

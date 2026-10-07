fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-arg=-T{manifest}/link.ld");
    println!("cargo:rerun-if-changed=link.ld");
}

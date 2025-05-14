fn main() {
    cc::Build::new()
        .std("c11")
        .file("amnis/renderdoc.c")
        .compile("amnis-rdoc");
    println!("cargo::rerun-if-changed=amnis/renderdoc.c");
    // eprintln!()
}
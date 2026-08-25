fn main() {
    println!("cargo:rerun-if-changed=assets/packs");
    println!("cargo:rerun-if-changed=assets/sounds");
}

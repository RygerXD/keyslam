fn main() {
    println!("cargo:rerun-if-changed=assets/images");
    println!("cargo:rerun-if-changed=assets/sounds");
}

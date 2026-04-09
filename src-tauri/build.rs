fn main() {
    // screencapturekit requires linking to Swift concurrency libraries
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
    tauri_build::build()
}

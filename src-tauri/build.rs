fn main() {
    // `tauri_build::build()` only tells Cargo to rerun this script when
    // `tauri.conf.json` changes, not when the icon files it points at do.
    // Icons are embedded into the binary at compile time (via
    // `tauri::generate_context!()`), so without this, regenerating icons
    // (e.g. with `cargo tauri icon`) has no effect until something else
    // forces a rebuild.
    println!("cargo:rerun-if-changed=icons");

    tauri_build::build()
}

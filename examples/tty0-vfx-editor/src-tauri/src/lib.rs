pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tty0-vfx-editor");
}

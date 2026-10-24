// 六つの窓　同じ景色を　映しけり
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("retroflex failed to start");
}

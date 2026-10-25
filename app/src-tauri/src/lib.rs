mod offline;

// 六つの窓　同じ景色を　映しけり
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    // 二つ目の　窓は開かず　一つ目へ
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            use tauri::Manager;
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }));
    }

    builder
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_haptics::init())
        .invoke_handler(tauri::generate_handler![offline::kept, offline::keep, offline::forget])
        .setup(|_app| {
            // 棚ひとつ　窓から覗く　許しあり
            use tauri::Manager;
            if let Ok(dir) = offline::shelf(_app.handle()) {
                let _ = _app.asset_protocol_scope().allow_directory(dir, false);
            }
            // 呼び名をば　この窓宛と　届け出る
            #[cfg(any(windows, target_os = "linux"))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                let _ = _app.deep_link().register_all();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("retroflex failed to start");
}

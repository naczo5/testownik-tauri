use std::fs;
use tauri::{AppHandle, Manager};

mod import_export;
use import_export::{import_old_base, import_new_base, export_to_anki};

fn copy_dir_missing_files(src: impl AsRef<std::path::Path>, dst: impl AsRef<std::path::Path>) -> std::io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let target_path = dst.as_ref().join(entry.file_name());
        if ty.is_dir() {
            copy_dir_missing_files(entry.path(), target_path)?;
        } else {
            if !target_path.exists() {
                fs::copy(entry.path(), target_path)?;
            }
        }
    }
    Ok(())
}

#[tauri::command]
fn get_app_data_dir(app: AppHandle) -> String {
    app.path().app_data_dir().unwrap().to_string_lossy().to_string()
}

#[tauri::command]
fn get_bases_index(app: AppHandle) -> Result<String, String> {
    let data_dir = app.path().app_data_dir().unwrap().join("data");
    let file_path = data_dir.join("bases.json");
    fs::read_to_string(file_path).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_base(app: AppHandle, slug: String) -> Result<String, String> {
    let data_dir = app.path().app_data_dir().unwrap().join("data");
    let file_path = data_dir.join(format!("{}.json", slug));
    fs::read_to_string(file_path).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_base(app: AppHandle, slug: String, content: String) -> Result<(), String> {
    let data_dir = app.path().app_data_dir().unwrap().join("data");
    let file_path = data_dir.join(format!("{}.json", slug));
    fs::write(file_path, content).map_err(|e| e.to_string())
}

#[cfg(target_os = "linux")]
fn set_linux_default_env_var(name: &str, value: &str) {
    if std::env::var_os(name).is_none() {
        std::env::set_var(name, value);
    }
}

#[cfg(target_os = "linux")]
pub fn apply_linux_startup_workarounds() {
    set_linux_default_env_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    set_linux_default_env_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");

    let is_appimage = std::env::var_os("APPIMAGE").is_some() || std::env::var_os("APPDIR").is_some();
    if is_appimage {
        set_linux_default_env_var("GDK_BACKEND", "x11");
        set_linux_default_env_var("LIBGL_ALWAYS_SOFTWARE", "1");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    apply_linux_startup_workarounds();

    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir().unwrap();
            let data_dir = app_data_dir.join("data");
            let bazy_dir = app_data_dir.join("bazy");

            if let Ok(resource_dir) = app.path().resource_dir() {
                let mut res_data = resource_dir.join("resources").join("data");
                let mut res_bazy = resource_dir.join("resources").join("bazy");

                if !res_data.exists() {
                    res_data = resource_dir.join("data");
                    res_bazy = resource_dir.join("bazy");
                }

                if res_data.exists() {
                    copy_dir_missing_files(&res_data, &data_dir)?;
                }
                if res_bazy.exists() {
                    copy_dir_missing_files(&res_bazy, &bazy_dir)?;
                }
            }
            Ok(())
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_app_data_dir,
            get_bases_index,
            get_base,
            save_base,
            import_old_base,
            import_new_base,
            export_to_anki
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

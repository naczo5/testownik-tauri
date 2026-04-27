// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    testownik_app_lib::apply_linux_startup_workarounds();
    testownik_app_lib::run()
}

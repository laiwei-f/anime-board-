#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::Command;

#[tauri::command]
fn open_local_target(raw_path: String) {
    let decoded = urlencoding::decode(&raw_path).unwrap_or(std::borrow::Cow::Borrowed(&raw_path));
    if let Some(idx) = decoded.find(":\\").or_else(|| decoded.find(":/")) {
        if idx >= 1 {
            let clean_path = decoded[idx - 1..]
                .split(&['"', '\'', '&', '?'][..])
                .next()
                .unwrap_or("")
                .replace('/', "\\");
            if !clean_path.is_empty() {
                let _ = Command::new("cmd")
                    .args(["/C", "start", "", &clean_path])
                    .spawn();
            }
        }
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![open_local_target])
        .on_page_load(|window, _| {
            let init_script = r#"
                document.addEventListener('click', (e) => {
                    const btn = e.target.closest('#btnTestLocalOpen');
                    if (btn) {
                        e.preventDefault();
                        e.stopPropagation();
                        window.__TAURI__.core.invoke('open_local_target', { rawPath: 'C:\\Windows' });
                        return;
                    }
                    const link = e.target.closest('a');
                    if (link && link.href && link.href.startsWith('animeshelf:')) {
                        e.preventDefault();
                        window.__TAURI__.core.invoke('open_local_target', { rawPath: link.href });
                    }
                }, true);
            "#;
            let _ = window.eval(init_script);
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

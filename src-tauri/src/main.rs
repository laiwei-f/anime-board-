#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tauri::{WebviewUrl, WebviewWindowBuilder};

fn get_custom_data_dir() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    let config_file = exe_dir.join("data_path.txt");
    if let Ok(custom_path) = fs::read_to_string(&config_file) {
        let trimmed = custom_path.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    exe_dir.join("Data")
}

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

#[tauri::command]
fn open_data_folder() {
    let data_dir = get_custom_data_dir();
    let _ = fs::create_dir_all(&data_dir);
    let _ = Command::new("explorer")
        .arg(data_dir.to_string_lossy().to_string())
        .spawn();
}

#[tauri::command]
fn get_current_data_path() -> String {
    get_custom_data_dir().to_string_lossy().to_string()
}

#[tauri::command]
fn set_custom_data_path(new_path: String) -> Result<String, String> {
    let trimmed = new_path.trim();
    if trimmed.is_empty() {
        return Err("路径不能为空".to_string());
    }
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    let config_file = exe_dir.join("data_path.txt");
    fs::write(&config_file, trimmed).map_err(|e| e.to_string())?;
    let _ = fs::create_dir_all(trimmed);
    Ok("保存成功，重启软件后生效".to_string())
}

fn main() {
    let data_dir = get_custom_data_dir();
    let _ = fs::create_dir_all(&data_dir);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            open_local_target,
            open_data_folder,
            get_current_data_path,
            set_custom_data_path
        ])
        .setup(move |app| {
            let _win = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("追番养肥看板 Pro")
                .inner_size(1360.0, 860.0)
                .data_directory(data_dir)
                .build()?;
            Ok(())
        })
        .on_page_load(|window, _| {
            let init_script = r#"
                // 1. 拦截本地文件与测试按钮
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

                // 2. 右下角悬浮栏：打开数据目录 + 自定义目录
                if (!document.getElementById('tauri-float-right')) {
                    const barRight = document.createElement('div');
                    barRight.id = 'tauri-float-right';
                    Object.assign(barRight.style, {
                        position: 'fixed', right: '16px', bottom: '16px', zIndex: '99999',
                        display: 'flex', gap: '6px'
                    });

                    const btnOpen = document.createElement('button');
                    btnOpen.innerText = '📂 打开数据目录';
                    const btnSet = document.createElement('button');
                    btnSet.innerText = '⚙️ 自定义目录';

                    [btnOpen, btnSet].forEach(b => {
                        Object.assign(b.style, {
                            padding: '6px 10px', fontSize: '12px', color: '#fff',
                            backgroundColor: 'rgba(79, 70, 229, 0.85)', border: 'none',
                            borderRadius: '6px', cursor: 'pointer', boxShadow: '0 2px 8px rgba(0,0,0,0.2)'
                        });
                    });

                    btnOpen.onclick = () => window.__TAURI__.core.invoke('open_data_folder');
                    btnSet.onclick = async () => {
                        const cur = await window.__TAURI__.core.invoke('get_current_data_path');
                        const next = prompt('请输入新的数据存储文件夹路径（例如 D:\\AnimeData）：', cur);
                        if (next && next.trim() !== cur) {
                            const msg = await window.__TAURI__.core.invoke('set_custom_data_path', { newPath: next });
                            alert(msg + '\n新路径：' + next + '\n（注：更改后请关闭软件重新打开，并重新导入一次 JSON 备份）');
                        }
                    };

                    barRight.appendChild(btnOpen);
                    barRight.appendChild(btnSet);
                    document.body.appendChild(barRight);
                }

                // 3. 左下角独立悬浮栏：一键清空数据（连续五次确认防误触）
                if (!document.getElementById('tauri-float-left')) {
                    const barLeft = document.createElement('div');
                    barLeft.id = 'tauri-float-left';
                    Object.assign(barLeft.style, {
                        position: 'fixed', left: '16px', bottom: '16px', zIndex: '99999',
                        display: 'flex', gap: '6px'
                    });

                    const btnClear = document.createElement('button');
                    btnClear.innerText = '🗑️ 清空所有数据';
                    Object.assign(btnClear.style, {
                        padding: '6px 10px', fontSize: '12px', color: '#fff',
                        backgroundColor: 'rgba(220, 38, 38, 0.85)', border: 'none',
                        borderRadius: '6px', cursor: 'pointer', boxShadow: '0 2px 8px rgba(0,0,0,0.2)'
                    });
                    btnClear.onmouseenter = () => btnClear.style.backgroundColor = 'rgba(220, 38, 38, 1)';
                    btnClear.onmouseleave = () => btnClear.style.backgroundColor = 'rgba(220, 38, 38, 0.85)';

                    btnClear.onclick = async () => {
                        const warnings = [
                            '【第 1 / 5 次确认】\n确定要清空软件内的所有追番数据和本地缓存吗？',
                            '【第 2 / 5 次确认】\n警告：清空后所有未导出的记录将无法恢复！是否继续？',
                            '【第 3 / 5 次确认】\n再次提醒：请确保你已经提前导出了 JSON 备份文件！确定继续清空吗？',
                            '【第 4 / 5 次确认】\n即将抹除所有 LocalStorage 和 IndexedDB 数据库，真的确定吗？',
                            '【第 5 / 5 次最终确认】\n最后一次确认：点击「确定」将立刻彻底清空全部数据！'
                        ];
                        for (let i = 0; i < warnings.length; i++) {
                            if (!confirm(warnings[i])) {
                                return; // 任意一步点取消，立刻安全中止
                            }
                        }

                        localStorage.clear();
                        sessionStorage.clear();
                        if (window.indexedDB && indexedDB.databases) {
                            const dbs = await indexedDB.databases();
                            for (const db of dbs) {
                                if (db.name) indexedDB.deleteDatabase(db.name);
                            }
                        }
                        alert('✅ 全部本地缓存与数据库已彻底清空！\n（提示：请勿直接刷新页面，直接点击页面上的「导入备份」即可导入你的 JSON 数据）');
                    };

                    barLeft.appendChild(btnClear);
                    document.body.appendChild(barLeft);
                }
            "#;
            let _ = window.eval(init_script);
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

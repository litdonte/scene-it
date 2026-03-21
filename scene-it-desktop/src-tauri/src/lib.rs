// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use scene_it_engine::Storyboard;

const STORYBOARD_JSON: &str = include_str!("../../../data/screenplay1.json");

#[tauri::command]
fn get_storyboard() -> Storyboard {
    serde_json::from_str(STORYBOARD_JSON).expect("Failed to deserialize storyboard")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_storyboard])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Temporary greeting command — will be replaced by real commands in M2+.
#[tauri::command]
pub fn greet(name: String) -> String {
    format!("Hello, {name}! You've been greeted from Rust!")
}

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
struct Port(u16);

fn main() {
    // let runtime = tauri::async_runtime::block_on(async {
    //     let config = backend::read_config();
    //     let runtime = KasukuRuntime::new(&config).await.unwrap();
    //     runtime
    // });
    // tauri::async_runtime::spawn(async move {
    //     app(8080, runtime).await;
    // });
    tauri::Builder::default()
        .manage(Port(8080))
        .invoke_handler(tauri::generate_handler![get_port])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// A command to get the used port, instead of 3000.
#[tauri::command]
fn get_port(port: tauri::State<Port>) -> Result<String, String> {
    Ok(format!("{}", port.0))
}

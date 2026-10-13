mod cli;
mod ipc;
mod platform;
mod secrets;
mod selection;
mod shortcuts;
mod state;
mod tray;
mod window;

use tauri::{Manager, WindowEvent};

use state::AppState;

pub fn run() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{}", cli::HELP);
        return;
    }

    let result = tauri::Builder::default()
        // Must be first: a second launch forwards its arguments to the running instance.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            let trigger = cli::parse(argv.get(1..).unwrap_or_default()).unwrap_or(window::Trigger::Show);
            window::trigger(app, trigger);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .invoke_handler(tauri::generate_handler![
            ipc::search,
            ipc::search_slash,
            ipc::chat,
            ipc::list_commands,
            ipc::execute,
            ipc::get_config,
            ipc::save_config,
            ipc::provider_status,
            ipc::set_api_key,
            ipc::delete_api_key,
            ipc::test_provider,
            ipc::list_models,
            ipc::platform_info,
            ipc::copy_text,
            ipc::replace_selection,
            ipc::hide_palette,
            ipc::open_settings,
            ipc::list_history,
            ipc::delete_history_entry,
            ipc::clear_history,
        ])
        .on_window_event(|win, event| {
            if win.label() != window::PALETTE {
                return;
            }
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = win.hide();
                }
                WindowEvent::Focused(false) => {
                    if let Some(state) = win.app_handle().try_state::<AppState>() {
                        if state.config().hide_on_blur && !state.recently_shown() {
                            let _ = win.hide();
                        }
                    }
                }
                _ => {}
            }
        })
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let state = AppState::init(app.handle())?;
            let config = state.config();
            app.manage(state);

            tray::create(app.handle())?;
            shortcuts::register_all(app.handle());
            ipc::sync_autostart(app.handle(), &config);

            if let Some(trigger) = cli::parse(&args) {
                window::trigger(app.handle(), trigger);
            }
            Ok(())
        })
        .run(tauri::generate_context!());

    if let Err(e) = result {
        eprintln!("keymint: fatal error: {e}");
        std::process::exit(1);
    }
}

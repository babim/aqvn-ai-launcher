pub mod app;
pub mod config;
#[cfg(not(target_os = "android"))]
pub mod connector;
pub mod proxy;
#[cfg(not(target_os = "android"))]
pub mod rfc;
#[cfg(not(target_os = "android"))]
pub mod ssh;

use tauri::{Manager, WebviewWindowBuilder};

/// Runs the application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::new().build());

    #[cfg(not(target_os = "android"))]
    {
        builder = builder.plugin(tauri_plugin_global_shortcut::Builder::new().build());
        builder = builder.menu(app::context_menu::build_context_menu);
        builder = builder.on_menu_event(|app, event| {
            let id = event.id().as_ref();
            let window = app.get_webview_window("main");
            let _ = app::context_menu::handle_menu_id(app, window.as_ref(), id);
        });
    }

    #[cfg(not(target_os = "android"))]
    {
        builder = builder.on_window_event(|window, event| {
            if window.label() == "main" {
                match event {
                    tauri::WindowEvent::Resized(_) | tauri::WindowEvent::Moved(_) => {
                        if let (Ok(factor), Ok(pos), Ok(size), Ok(is_maximized)) = (
                            window.scale_factor(),
                            window.outer_position(),
                            window.inner_size(),
                            window.is_maximized(),
                        ) {
                            if size.width > 0 && size.height > 0 {
                                let logical_size = size.to_logical::<f64>(factor);
                                let logical_pos = pos.to_logical::<f64>(factor);

                                let current_ws = window
                                    .app_handle()
                                    .try_state::<config::ConfigWatcher>()
                                    .and_then(|w| w.current().window_state);

                                let (x, y, width, height) = if is_maximized {
                                    if let Some(prev) = current_ws {
                                        (prev.x, prev.y, prev.width, prev.height)
                                    } else {
                                        (
                                            logical_pos.x,
                                            logical_pos.y,
                                            logical_size.width,
                                            logical_size.height,
                                        )
                                    }
                                } else {
                                    (
                                        logical_pos.x,
                                        logical_pos.y,
                                        logical_size.width,
                                        logical_size.height,
                                    )
                                };

                                let new_state = config::WindowState {
                                    width,
                                    height,
                                    x,
                                    y,
                                    is_maximized,
                                };

                                if current_ws.as_ref() != Some(&new_state) {
                                    if let Some(debouncer) =
                                        window.app_handle().try_state::<config::GeometryDebouncer>()
                                    {
                                        debouncer.debounce(new_state);
                                    } else {
                                        let _ = config::persist_window_geometry(
                                            window.app_handle(),
                                            new_state,
                                        );
                                    }
                                }
                            }
                        }
                    }
                    tauri::WindowEvent::Focused(false) => {
                        if let Some(debouncer) =
                            window.app_handle().try_state::<config::GeometryDebouncer>()
                        {
                            debouncer.flush();
                        }
                    }
                    tauri::WindowEvent::CloseRequested { api, .. } => {
                        if let Some(debouncer) =
                            window.app_handle().try_state::<config::GeometryDebouncer>()
                        {
                            debouncer.flush();
                        }
                        let close_to_tray = window
                            .app_handle()
                            .try_state::<config::ConfigWatcher>()
                            .map(|w| w.current().close_to_tray)
                            .unwrap_or(true);
                        if close_to_tray {
                            api.prevent_close();
                            let _ = window.hide();
                            eprintln!("aqvn: main window hidden to tray");
                        }
                    }
                    _ => {}
                }
            }
        });
    }

    builder
        .setup(|app| {
            let initial_config =
                config::AppConfig::load_from_store(app.handle()).unwrap_or_default();
            #[cfg(not(target_os = "android"))]
            connector::handlers::files::set_workspace_path(Some(&initial_config.workspace_path));
            #[cfg(not(target_os = "android"))]
            connector::handlers::files::sync_from_app_config(&initial_config);
            let watcher = config::ConfigWatcher::new(initial_config.clone());
            app.manage(watcher.clone());
            let debouncer = config::GeometryDebouncer::new(app.handle().clone());
            app.manage(debouncer);

            // Local bridge proxy: loopback-only, kernel-chosen port. Binds before
            // the window exists so the resolved port can be injected into the
            // page as `window.__AQVN_PROXY__`.
            let (proxy_listener, proxy_state, proxy_info) =
                tauri::async_runtime::block_on(proxy::server::bind_with_config(&initial_config))?;
            app.manage(proxy_info.clone());
            app.manage(proxy_state.clone());
            app.manage(proxy_state.cache().clone());

            let serve_state = proxy_state.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(err) = proxy::server::serve(proxy_listener, serve_state).await {
                    eprintln!("Bridge proxy error: {err}");
                }
            });
            let watch_state = proxy_state.clone();
            let watch_config = watcher.clone();
            tauri::async_runtime::spawn(async move {
                proxy::server::watch_target(watch_config, watch_state).await;
            });

            #[cfg(not(target_os = "android"))]
            {
                // RFC bridge: binds only when the master toggle is enabled (default OFF)
                // and restarts its listener on any settings change (hot reload).
                let supervisor = watcher.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(err) = rfc::server::run_supervised(supervisor).await {
                        eprintln!("{err}");
                    }
                });

                // SSH bridge: binds only when the master toggle is enabled and password is set
                // (default OFF) and restarts its listener on any settings change (hot reload).
                let ssh_supervisor = watcher.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(err) = ssh::server::run_supervised(ssh_supervisor).await {
                        eprintln!("{err}");
                    }
                });

                // Outbound Gateway Hub client: connects to upstream /ws
                let (gateway_manager, rx_reconnect) = connector::client::GatewayManager::new();
                app.manage(gateway_manager.clone());
                let gateway_supervisor = watcher.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(err) = connector::client::run_supervised_with_reconnect(
                        gateway_supervisor,
                        gateway_manager,
                        rx_reconnect,
                    )
                    .await
                    {
                        eprintln!("Gateway supervisor error: {err}");
                    }
                });
            }

            // The settings UI writes through the store plugin; poll the settings file so
            // those writes reach the watcher without an app restart.
            let poll_handle = app.handle().clone();
            let poll_watcher = watcher.clone();
            let poll_initial = initial_config.clone();
            let store_file = config::store_path(&poll_handle).ok();
            tauri::async_runtime::spawn(async move {
                let mut last = poll_initial;
                #[cfg(not(target_os = "android"))]
                let mut last_gateway_connected = app::tray::is_gateway_connected();
                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    let next = store_file
                        .as_deref()
                        .and_then(config::load_from_file)
                        .or_else(|| config::AppConfig::load_from_store(&poll_handle).ok());
                    let Some(next) = next else { continue };
                    #[cfg(not(target_os = "android"))]
                    let server_url_changed = next.server_url != last.server_url;
                    #[cfg(not(target_os = "android"))]
                    let server_profiles_changed = next.server_profiles != last.server_profiles;
                    #[cfg(not(target_os = "android"))]
                    let workspace_path_changed = next.workspace_path != last.workspace_path;
                    if next != last {
                        #[cfg(not(target_os = "android"))]
                        if next.show_menu_bar != last.show_menu_bar {
                            if let Some(main_win) = poll_handle.get_webview_window("main") {
                                if next.show_menu_bar {
                                    if let Ok(menu) =
                                        app::context_menu::build_context_menu(&poll_handle)
                                    {
                                        let _ = main_win.set_menu(menu);
                                    }
                                } else {
                                    let _ = main_win.remove_menu();
                                }
                            }
                        }
                        #[cfg(not(target_os = "android"))]
                        if server_url_changed {
                            if let Some(main_win) = poll_handle.get_webview_window("main") {
                                let _ = main_win.eval("window.location.reload();");
                            }
                        }
                        last = next.clone();
                        #[cfg(not(target_os = "android"))]
                        connector::handlers::files::sync_from_app_config(&last);
                        poll_watcher.update(next);
                    }

                    #[cfg(not(target_os = "android"))]
                    {
                        let current_gw = app::tray::is_gateway_connected();
                        if server_url_changed
                            || server_profiles_changed
                            || current_gw != last_gateway_connected
                        {
                            last_gateway_connected = current_gw;
                            let _ = app::tray::update_tray_status(
                                &poll_handle,
                                &last.server_url,
                                current_gw,
                            );
                        }
                        if workspace_path_changed {
                            app::tray::set_workspace_path(&last.workspace_path);
                        }
                    }
                }
            });

            let window_config = app
                .config()
                .app
                .windows
                .iter()
                .find(|w| w.label == "main")
                .expect("main window config not found in tauri.conf.json");

            let mut container_config =
                app::links::configure_container_window(window_config, proxy_info.port);

            if let Some(ref ws) = initial_config.window_state {
                config::apply_window_state(&mut container_config, ws);
            }

            let nav_handle = app.handle().clone();
            #[allow(unused_mut)]
            let mut window_builder =
                WebviewWindowBuilder::from_config(app.handle(), &container_config)?
                    .initialization_script(proxy::server::init_script(proxy_info.port))
                    .zoom_hotkeys_enabled(false);

            #[cfg(not(target_os = "android"))]
            if let Some(ref ws) = initial_config.window_state {
                window_builder = window_builder
                    .inner_size(ws.width, ws.height)
                    .position(ws.x, ws.y)
                    .maximized(ws.is_maximized);
            }

            let _main_window = window_builder
                .on_navigation(move |url| app::links::handle_navigation(&nav_handle, url))
                .on_document_title_changed(|window, title| {
                    // Rust-side sync needs no `core:window:allow-set-title`.
                    let _ = window.set_title(&title);
                })
                .build()?;

            #[cfg(not(target_os = "android"))]
            if !initial_config.show_menu_bar {
                let _ = _main_window.remove_menu();
            }

            #[cfg(not(target_os = "android"))]
            {
                app::tray::init(app.handle())?;
                app::hotkey::register(app.handle(), &initial_config.global_hotkey);
                app::hotkey::watch(app.handle().clone(), watcher.clone());
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app::set_tray_busy,
            app::notify::notify_task_complete,
            app::open_settings,
            app::open_help,
            app::open_about,
            app::close_current_window,
            app::context_menu::show_context_menu,
            app::context_menu::toggle_fullscreen,
            app::clear_disk_cache,
            app::reconnect_network
        ])
        .run(tauri::generate_context!())
        .expect("failed to run AQVN Workspace");
}

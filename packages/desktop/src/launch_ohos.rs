use crate::Config;
use crate::{
    app::App,
    ipc::{IpcMethod, UserWindowEvent},
};
use dioxus_core::*;
use dioxus_document::eval;
use std::any::Any;
use tao::event::{Event, StartCause, WindowEvent};

fn load_debug_env_file() {
    if !cfg!(debug_assertions) {
        return;
    }

    let Some(session_dir) = dioxus_cli_config::session_cache_dir() else {
        return;
    };

    let env_file = session_dir.join(".env");
    let env_contents = match std::fs::read_to_string(env_file) {
        std::result::Result::Ok(contents) => contents,
        Err(_) => return,
    };

    for entry in env_contents.lines() {
        if let Some((key, value)) = entry.trim().split_once('=') {
            unsafe { std::env::set_var(key, value) };
        }
    }
}

/// Launch the WebView and run the event loop, with configuration and root props.
///
/// This will block the main thread, and *must* be spawned on the main thread. This function does not assume any runtime
/// and is equivalent to calling launch_with_props with the tokio feature disabled.
pub fn launch_virtual_dom_blocking(virtual_dom: VirtualDom, mut desktop_config: Config) -> () {
    load_debug_env_file();

    let mut custom_event_handler = desktop_config.custom_event_handler.take();
    let (event_loop, app) = App::new(desktop_config, virtual_dom);
    let app = Box::leak(Box::new(app));

    event_loop.run(move |window_event, event_loop, control_flow| {
        // Set the control flow and check if any events need to be handled in the app itself
        app.tick(&window_event);

        if let Some(ref mut f) = custom_event_handler {
            f(&window_event, event_loop)
        }

        match window_event {
            Event::NewEvents(StartCause::Init) => app.handle_start_cause_init(),
            Event::LoopDestroyed => app.handle_loop_destroyed(),
            Event::WindowEvent {
                event, window_id, ..
            } => match event {
                WindowEvent::CloseRequested => app.handle_close_requested(window_id),
                WindowEvent::Destroyed { .. } => app.window_destroyed(window_id),
                WindowEvent::Resized(new_size) => app.resize_window(window_id, new_size),
                _ => {}
            },

            Event::UserEvent(event) => match event {
                UserWindowEvent::Poll(id) => app.poll_vdom(id),
                UserWindowEvent::NewWindow => app.handle_new_window(),
                UserWindowEvent::CloseWindow(id) => app.handle_close_requested(id),
                UserWindowEvent::Shutdown => app.control_flow = tao::event_loop::ControlFlow::Exit,

                #[cfg(all(
                    any(target_os = "windows", target_os = "linux", target_os = "macos"),
                    not(target_env = "ohos")
                ))]
                UserWindowEvent::GlobalHotKeyEvent(evnt) => app.handle_global_hotkey(evnt),

                #[cfg(all(
                    any(target_os = "windows", target_os = "linux", target_os = "macos"),
                    not(target_env = "ohos")
                ))]
                UserWindowEvent::MudaMenuEvent(evnt) => app.handle_menu_event(evnt),

                #[cfg(all(
                    any(target_os = "windows", target_os = "linux", target_os = "macos"),
                    not(target_env = "ohos")
                ))]
                UserWindowEvent::TrayMenuEvent(evnt) => app.handle_tray_menu_event(evnt),

                #[cfg(all(
                    any(target_os = "windows", target_os = "linux", target_os = "macos"),
                    not(target_env = "ohos")
                ))]
                UserWindowEvent::TrayIconEvent(evnt) => app.handle_tray_icon_event(evnt),

                #[cfg(all(feature = "devtools", debug_assertions))]
                UserWindowEvent::HotReloadEvent(msg) => app.handle_hot_reload_msg(msg),

                // Windows-only drag-n-drop fix events. We need to call the interpreter drag-n-drop code.
                UserWindowEvent::WindowsDragDrop(id) => {
                    if let Some(webview) = app.webviews.get(&id) {
                        webview.dom.in_scope(ScopeId::ROOT, || {
                            eval("window.interpreter.handleWindowsDragDrop();");
                        });
                    }
                }
                UserWindowEvent::WindowsDragLeave(id) => {
                    if let Some(webview) = app.webviews.get(&id) {
                        webview.dom.in_scope(ScopeId::ROOT, || {
                            eval("window.interpreter.handleWindowsDragLeave();");
                        });
                    }
                }
                UserWindowEvent::WindowsDragOver(id, x_pos, y_pos) => {
                    if let Some(webview) = app.webviews.get(&id) {
                        webview.dom.in_scope(ScopeId::ROOT, || {
                            let e = eval(
                                r#"
                                    const xPos = await dioxus.recv();
                                    const yPos = await dioxus.recv();
                                    window.interpreter.handleWindowsDragOver(xPos, yPos)
                                    "#,
                            );

                            _ = e.send(x_pos);
                            _ = e.send(y_pos);
                        });
                    }
                }

                UserWindowEvent::Ipc { id, msg } => match msg.method() {
                    IpcMethod::Initialize => app.handle_initialize_msg(id),
                    IpcMethod::UserEvent => {}
                    IpcMethod::Query => app.handle_query_msg(msg, id),
                    IpcMethod::BrowserOpen => app.handle_browser_open(msg),
                    IpcMethod::Other(_) => {}
                },
            },
            _ => {}
        }

        *control_flow = app.control_flow;
    })
}

/// Launches the WebView and runs the event loop, with configuration and root props.
pub fn launch_virtual_dom(virtual_dom: VirtualDom, desktop_config: Config) -> () {
    {
        launch_virtual_dom_blocking(virtual_dom, desktop_config);
    }
}

/// Launches the WebView and runs the event loop, with configuration and root props.
pub fn launch(
    root: fn() -> Element,
    contexts: Vec<Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
    platform_config: Vec<Box<dyn Any>>,
) -> () {
    let mut virtual_dom = VirtualDom::new(root);

    for context in contexts {
        virtual_dom.insert_any_root_context(context());
    }

    let platform_config = *platform_config
        .into_iter()
        .find_map(|cfg| cfg.downcast::<Config>().ok())
        .unwrap_or_default();

    launch_virtual_dom(virtual_dom, platform_config)
}

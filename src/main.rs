use std::collections::HashSet;

use active_win_pos_rs::get_active_window;
use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use log;
use serde_json::Value;
use tokio::io::{stdin, AsyncBufReadExt, BufReader};
use tokio::signal::ctrl_c;
use tokio::time;

const AGENT_NAME: &str = "mnemnk-application";
const KIND: &str = "application";

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct AgentConfig {
    /// Interval in seconds
    interval: u64,

    /// Applications to ignore
    ignore: Vec<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            interval: 10,
            ignore: Vec::default(),
        }
    }
}

impl From<&str> for AgentConfig {
    fn from(s: &str) -> Self {
        let mut config = AgentConfig::default();
        if let Value::Object(c) = serde_json::from_str(s).unwrap_or(Value::Null) {
            if let Some(interval) = c.get("interval") {
                config.interval = interval.as_u64().unwrap();
            }
            if let Some(ignore) = c.get("ignore") {
                config.ignore = ignore
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().to_string())
                    .collect();
            }
        }
        config
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
struct ApplicationEvent {
    t: i64,
    // process_id: i64,
    // path: String,
    name: String,
    title: String,
    x: i64,
    y: i64,
    width: i64,
    height: i64,
    text: String,
}

struct ApplicationAgent {
    config: AgentConfig,
    last_event: Option<ApplicationEvent>,
    ignore: HashSet<String>,
    notify_rx: tokio::sync::mpsc::Receiver<()>,
}

impl ApplicationAgent {
    fn new(config: AgentConfig, notify_rx: tokio::sync::mpsc::Receiver<()>) -> Self {
        Self {
            ignore: config.ignore.iter().cloned().collect(),
            config,
            last_event: None,
            notify_rx,
        }
    }

    async fn run(&mut self) -> Result<()> {
        let mut interval = time::interval(time::Duration::from_secs(self.config.interval));
        let mut last_interval_period = self.config.interval;

        let mut reader = BufReader::new(stdin());
        let mut line = String::new();

        // Main loop with graceful shutdown
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.execute_task().await {
                        log::error!("Failed to execute task: {}", e);
                    }
                    if last_interval_period != self.config.interval {
                        interval = time::interval(time::Duration::from_secs(self.config.interval));
                        last_interval_period = self.config.interval;
                    }
                }

                _ = reader.read_line(&mut line) => {
                    if let Err(e) = self.process_line(&line).await {
                        log::error!("Failed to process line: {}", e);
                    }
                    line.clear();
                }

                _ = ctrl_c() => {
                    log::info!("\nShutting down mnemnk-application.");
                    std::process::exit(0);
                }

                // (macOS) check on NSWorkspaceDidActivateApplicationNotification
                // TODO: add support for other platforms
                _ = self.notify_rx.recv() => {
                    log::debug!("Received NSWorkspace notification");
                    if let Err(e) = self.execute_task().await {
                        log::error!("Failed to execute task: {}", e);
                    }
                }
            }
        }
    }

    async fn execute_task(&mut self) -> Result<()> {
        let app_event = check_application().await;

        if self.is_same(&app_event) {
            log::debug!("Same as the last application event");
            return Ok(());
        }

        if self.is_ignored(&app_event) {
            log::debug!("Ignored application event");
            return Ok(());
        }

        if let Some(app_event) = app_event {
            // debug!("check_application: {:?}", app_event);
            let app_event_json = serde_json::to_string(&app_event)?;
            println!(".OUT {} {}", KIND, app_event_json);
        }
        Ok(())
    }

    fn is_same(&mut self, app_event: &Option<ApplicationEvent>) -> bool {
        if let Some(app_event) = app_event {
            if let Some(last_event) = &self.last_event {
                if app_event.x == last_event.x
                    && app_event.y == last_event.y
                    && app_event.width == last_event.width
                    && app_event.height == last_event.height
                    && app_event.text == last_event.text
                {
                    return true;
                }
            }
        }
        self.last_event = app_event.clone();
        false
    }

    fn is_ignored(&self, app_event: &Option<ApplicationEvent>) -> bool {
        if let Some(app_event) = app_event {
            if self.ignore.contains(&app_event.name) {
                return true;
            }
        }
        false
    }

    async fn process_line(&mut self, line: &str) -> Result<()> {
        log::debug!("process_line: {}", line);

        if let Some((cmd, args)) = parse_line(line) {
            match cmd {
                ".CONFIG" => {
                    let config = AgentConfig::from(args);
                    log::info!("Updated config: {:?}", config);
                    self.ignore = config.ignore.iter().cloned().collect();
                    self.config = config;
                }
                ".QUIT" => {
                    log::info!("QUIT {}.", AGENT_NAME);
                    std::process::exit(0);
                }
                _ => {
                    log::error!("Unknown command: {}", cmd);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Parser)]
pub struct Args {
    #[arg(short = 'c', long = "config", help = "JSON config string")]
    config: Option<String>,
}

async fn check_application() -> Option<ApplicationEvent> {
    const MAX_TITLE_LEN: usize = 250;

    log::debug!("check_application");
    match get_active_window() {
        Ok(mut win) => {
            // sanitize app_name and title
            if win.app_name.is_empty() {
                return None;
            }
            if win.title.chars().count() > MAX_TITLE_LEN {
                win.title = win.title.chars().take(MAX_TITLE_LEN).collect();
            };

            let text = format!("{} {}", win.app_name, win.title).trim().to_string();
            let info = ApplicationEvent {
                t: Utc::now().timestamp_millis(),
                // process_id: win.process_id as i64,
                // path: path,
                name: win.app_name,
                title: win.title,
                x: win.position.x as i64,
                y: win.position.y as i64,
                width: win.position.width as i64,
                height: win.position.height as i64,
                text: text,
            };
            Some(info)
        }
        _ => {
            log::error!("Failed to get active window");
            None
        }
    }
}

fn parse_line(line: &str) -> Option<(&str, &str)> {
    if line.is_empty() {
        return None;
    }

    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    if let Some((cmd, args)) = line.split_once(" ") {
        Some((cmd, args))
    } else {
        Some((line, ""))
    }
}

#[cfg(target_os = "macos")]
mod macos {
    // based on https://github.com/dimusic/active-window-macos-example

    use cocoa::base::{nil, id};
    use cocoa::foundation::NSAutoreleasePool;
    use objc::runtime::{Object, Sel};
    use objc::{class, msg_send, sel, sel_impl};
    use std::sync::OnceLock;
    use tokio::sync::mpsc::Sender;

    pub static NOTIFY_SENDER: OnceLock<Sender<()>> = OnceLock::new();

    extern "C" {
        static NSWorkspaceDidActivateApplicationNotification: *mut Object;
    }

    #[no_mangle]
    pub extern "C" fn notify_active_app_changed() {
        if let Some(sender) = NOTIFY_SENDER.get() {
            let _ = sender.try_send(());
        }
    }

    extern "C" fn application_did_finish_launching(this: &mut Object, _sel: Sel, _notif: id) {
        unsafe {
            let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
            let notification_center: id = msg_send![workspace, notificationCenter];
            let _: () = msg_send![notification_center,
                addObserver: this
                selector: sel!(workspace_app_activated:)
                name: NSWorkspaceDidActivateApplicationNotification
                object: nil];
        }
    }

    extern "C" fn handle_workspace_app_activated(_this: &mut Object, _sel: Sel, _notif: id) {
        notify_active_app_changed();
    }

    pub fn run_macos_event_loop() {
        unsafe {
            let pool = cocoa::foundation::NSAutoreleasePool::new(cocoa::base::nil);
            let cls = {
                let mut decl = objc::declare::ClassDecl::new("AppDelegate", class!(NSObject)).unwrap();
                decl.add_method(
                    sel!(applicationDidFinishLaunching:),
                    application_did_finish_launching as extern "C" fn(&mut Object, Sel, id),
                );
                decl.add_method(
                    sel!(workspace_app_activated:),
                    handle_workspace_app_activated as extern "C" fn(&mut Object, Sel, id),
                );
                decl.register()
            };
            let app = cocoa::appkit::NSApplication::sharedApplication(cocoa::base::nil);
            let delegate: *mut Object = msg_send![cls, new];
            let () = msg_send![app, setDelegate: delegate];
            let () = msg_send![app, run];
            pool.drain();
        }
    }
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();
    let config = args.config.as_deref().unwrap_or_default().into();

    log::info!("Starting {}.", AGENT_NAME);

    let (_tx, rx) = tokio::sync::mpsc::channel(100);

    #[cfg(target_os = "macos")]
    {
        macos::NOTIFY_SENDER.set(_tx).unwrap();
    }

    let runtime = tokio::runtime::Runtime::new()?;
    let agent_handle = std::thread::spawn(move || {
         let mut agent = ApplicationAgent::new(config, rx);
         runtime.block_on(async { agent.run().await }).unwrap();
    });

    #[cfg(target_os = "macos")]
    {
        macos::run_macos_event_loop();
    }

    agent_handle.join().unwrap();

    Ok(())
}

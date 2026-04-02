use eframe::egui;
use eframe::egui::Color32;
use eframe::egui::{FontFamily, FontId, Rounding, TextStyle, Visuals};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::{broadcast, mpsc};

mod event;
mod app;
mod agent;

mod ai_provider;
mod config;
mod database; 
mod arena;

pub use event::Event;
pub use agent::{generate_procedural_personality, spawn_agents_for_directory, ProgramAgent};
use ai_provider::{run_ai_engine, AiRequest};
use config::Config;
use database::Database;
use app::GridApp;


fn main() -> eframe::Result<()> {
    // Load configuration at the very beginning
    let config = match Config::load() {
        Ok(cfg) => cfg,
        Err(_) => return Ok(()), // Exit gracefully if config was created or is invalid
    };

    // Manually start the Tokio runtime in the background
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    let rt_handle = rt.handle().clone();
    let rt_handle_clone = rt.handle().clone();
    let shared_config = Arc::new(Mutex::new(config));

    let typing_agents = Arc::new(Mutex::new(HashSet::<String>::new()));

    let (tx, mut monitor_rx) = broadcast::channel::<Event>(100);

    // Central AI Engine Channel
    let (ai_tx, ai_rx) = mpsc::channel::<AiRequest>(32);

    // Central AI Engine Task (Processes LLM requests sequentially)
    let tx_for_ai = tx.clone();
    let config_for_ai = shared_config.clone();
    let ai_task = rt.spawn(run_ai_engine(ai_rx, tx_for_ai, config_for_ai));

    // Shared state between Tokio (async) and eframe (sync UI)
    let messages = Arc::new(Mutex::new(Vec::new()));
    let messages_clone = messages.clone();
    let typing_agents_for_listener = typing_agents.clone();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 500.0]),
        ..Default::default()
    };

    // eframe::run_native takes over the main thread and blocks until the window closes
    eframe::run_native(
        "The Grid",
        options,
        Box::new(move |cc| { // This move takes ownership of variables from main
            let ctx = cc.egui_ctx.clone();

            // Apply a 90s Solaris Terminal retro theme
            let mut visuals = Visuals::dark();
            visuals.panel_fill = Color32::from_rgb(0, 0, 0); // Pure black background
            visuals.window_fill = Color32::from_rgb(0, 0, 0);
            visuals.faint_bg_color = Color32::from_rgb(20, 20, 20);
            visuals.extreme_bg_color = Color32::from_rgb(10, 10, 10);
            visuals.override_text_color = Some(Color32::from_rgb(170, 170, 170)); // Classic light gray text
            
            // Sharp edges for a digital, terminal look
            let rounding = Rounding::same(0.0);
            visuals.window_rounding = rounding;
            visuals.menu_rounding = rounding;
            visuals.widgets.noninteractive.rounding = rounding;
            visuals.widgets.inactive.rounding = rounding;
            visuals.widgets.hovered.rounding = rounding;
            visuals.widgets.active.rounding = rounding;
            visuals.widgets.open.rounding = rounding;
            
            ctx.set_visuals(visuals);

            // Force a Monospace font for the entire UI
            let mut style = (*ctx.style()).clone();
            style.text_styles.insert(TextStyle::Body, FontId::new(16.0, FontFamily::Monospace));
            style.text_styles.insert(TextStyle::Button, FontId::new(16.0, FontFamily::Monospace));
            style.text_styles.insert(TextStyle::Heading, FontId::new(18.0, FontFamily::Monospace));
            ctx.set_style(style);

            // Background listener task: Reads from the broadcast channel and wakes up the UI
            rt_handle_clone.spawn(async move {
                while let Ok(event) = monitor_rx.recv().await {
                    let mut typing_guard = typing_agents_for_listener.lock().unwrap();
                    match event.action.as_str() {
                        "is_typing" => {
                            typing_guard.insert(event.sender.clone());
                        }
                        "stops_typing" => {
                            typing_guard.remove(&event.sender);
                            // Don't add this event to the message list
                        }
                        _ => {
                            // Any other action from an agent stops them from "typing"
                            typing_guard.remove(&event.sender);
                            if let Ok(mut msgs) = messages_clone.lock() {
                                msgs.push(event);
                                if msgs.len() > 500 {
                                    msgs.remove(0); // Prevent infinite UI freeze by capping message display
                                }
                            }
                        }
                    }
                    // Force egui to repaint to show the new message/typing status immediately
                    ctx.request_repaint();
                }
            });

            let current_dir = std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "/".to_string());

            // Check if DB exists and load it automatically
            let initial_db = if Path::new("the_grid.db").exists() {
                Database::new().ok().map(|db| Arc::new(Mutex::new(db)))
            } else {
                None
            };

            // Initial agent spawning
            let (initial_tasks, initial_names) = spawn_agents_for_directory(&current_dir, &rt_handle, tx.clone(), ai_tx.clone(), initial_db.clone());

            let _ = tx.send(Event {
                sender: "System".to_string(),
                action: "announces".to_string(),
                content: "The Grid is now online.".to_string(),
            });

            let user_name = whoami::username();

            // A palette of classic 90s terminal colors for the agents
            let color_palette = vec![
                Color32::from_rgb(0, 255, 0),     // Terminal Green
                Color32::from_rgb(0, 255, 255),   // Terminal Cyan
                Color32::from_rgb(255, 170, 0),   // Amber / Orange
                Color32::from_rgb(255, 0, 255),   // Magenta
                Color32::from_rgb(255, 255, 85),  // Bright Yellow
                Color32::from_rgb(255, 85, 85),   // Bright Red
                Color32::from_rgb(170, 170, 255), // Light Blue
            ];

            // A palette of emojis for the agents
            let emoji_palette: Vec<String> = ["🤖", "👽", "👾", "🧠", "⚙️", "📡", "💡", "💾", "💿", "🕹️", "🎛️", "🖥️", "💻", "🖱️", "🔌"]
                .iter().map(|&s| s.to_string()).collect();

            Box::new(GridApp {
                tx, messages,
                input: String::new(),
                current_dir, user_name,
                rt_handle, agent_tasks: initial_tasks,
                shared_config,
                typing_agents: typing_agents.clone(),
                ai_tx,
                ai_task,
                agent_names: initial_names,
                db: initial_db,
                show_map: false,
                colors: HashMap::new(),
                color_palette,
                next_color_index: 0,
                show_emojis: false,
                emoji_palette,
                show_thoughts: true,
                show_feels: true,
                invoked_tools: HashSet::new(),
                rel_cache: HashMap::new(),
                last_rel_update: Instant::now(),
                last_active_agent: None,
                map_user_pos: egui::pos2(0.0, 0.0),
                file_positions: HashMap::new(),
            })
        }),
    )
}
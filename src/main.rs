use eframe::egui;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

const ICON_DATA: &[u8] = include_bytes!("rustedrace.ico");

mod http_parser;
mod race_engine;
mod request_builder;
mod loading_screen;
mod workflow_race;
mod replay_race_simple;

use loading_screen::LoadingScreen;
use workflow_race::{WorkflowConfig, WorkflowEngine, ExecutionMode, WorkflowResult};
use replay_race_simple::{ReplayConfig, ReplayEngine, ReplayResult, ExecutionMode as ReplayExecutionMode, RaceType};

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "RustedRace",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_pixels_per_point(0.8);
            Ok(Box::new(RustedRaceApp::default()))
        }),
    )
}

fn load_icon() -> egui::IconData {
    if let Ok(image_bytes) = std::fs::read("src/rustedrace.png") {
        if let Ok(img) = image::load_from_memory(&image_bytes) {
            let rgba_img = img.to_rgba8();
            let width = rgba_img.width();
            let height = rgba_img.height();
            return egui::IconData {
                rgba: rgba_img.into_raw(),
                width,
                height,
            };
        }
    }
    
    // Fallback icon
    let size = 32;
    let mut rgba = Vec::with_capacity(size * size * 4);
    for y in 0..size {
        for x in 0..size {
            let center_x = size as f32 / 2.0;
            let center_y = size as f32 / 2.0;
            let distance = ((x as f32 - center_x).powi(2) + (y as f32 - center_y).powi(2)).sqrt();
            if distance < size as f32 / 2.0 - 2.0 {
                rgba.extend_from_slice(&[255, 107, 53, 255]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    egui::IconData {
        rgba,
        width: size as u32,
        height: size as u32,
    }
}

#[derive(PartialEq)]
enum RaceTab {
    ReplayRace,
    WorkflowRace,
}

struct RustedRaceApp {
    loading_screen: Option<LoadingScreen>,
    show_loading: bool,
    current_tab: RaceTab,
    raw_request: String,
    concurrency: String,
    is_running: bool,
    error_message: String,
    // Dynamic wordlist features
    wordlists: Vec<(String, Vec<String>)>, // (path, words)
    // Workflow race features
    workflow_config: WorkflowConfig,
    workflow_results: Arc<Mutex<Option<WorkflowResult>>>,
    workflow_raw_requests: HashMap<String, String>, // request_id -> raw_request
    selected_request_index: usize,
    // Replay race features
    replay_config: ReplayConfig,
    replay_results: Arc<Mutex<Option<ReplayResult>>>,
}

impl Default for RustedRaceApp {
    fn default() -> Self {
        Self {
            loading_screen: Some(LoadingScreen::new()),
            show_loading: true,
            current_tab: RaceTab::ReplayRace,
            raw_request: String::new(),
            concurrency: "10".to_string(),
            is_running: false,
            error_message: String::new(),
            wordlists: vec![(String::new(), Vec::new())], // Start with one empty wordlist
            workflow_config: WorkflowConfig::default(),
            workflow_results: Arc::new(Mutex::new(None)),
            workflow_raw_requests: HashMap::new(),
            selected_request_index: 0,
            replay_config: ReplayConfig::default(),
            replay_results: Arc::new(Mutex::new(None)),
        }
    }
}

impl eframe::App for RustedRaceApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.show_loading {
            // Transparent background for loading
            ctx.set_visuals(egui::Visuals {
                window_fill: egui::Color32::TRANSPARENT,
                panel_fill: egui::Color32::TRANSPARENT,
                ..egui::Visuals::dark()
            });
            
            if let Some(loading_screen) = &mut self.loading_screen {
                if loading_screen.show(ctx) {
                    ctx.request_repaint();
                    return;
                } else {
                    self.show_loading = false;
                    self.loading_screen = None;
                    
                    // Set normal window properties
                    ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::Vec2::new(1200.0, 800.0)));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Title("RustedRace - Race Condition Vulnerability Exploitation Toolkit".to_string()));
                    
                    // Reset visuals to normal
                    ctx.set_visuals(egui::Visuals::dark());
                }
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.heading("RustedRace");
            });
            ui.add_space(5.0);
        });

        egui::SidePanel::right("results_panel").resizable(true).default_width(400.0).show(ctx, |ui| {
            match self.current_tab {
                RaceTab::ReplayRace => {
                    if let Ok(results) = self.replay_results.try_lock() {
                        if let Some(result) = results.as_ref() {
                            ui.horizontal(|ui| {
                                ui.heading("🔄 Replay Race Results");
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("🗑️ Clear").clicked() {
                                        self.is_running = false;
                                        // Use try_lock to avoid blocking/crashing
                                        if let Ok(mut results) = self.replay_results.try_lock() {
                                            *results = None;
                                        }
                                    }
                                });
                            });
                            
                            ui.group(|ui| {
                                ui.label(format!("📈 Total: {}", result.total_requests));
                                ui.colored_label(egui::Color32::GREEN, format!("✅ Success: {}", result.success_count));
                                ui.colored_label(egui::Color32::RED, format!("❌ Failure: {}", result.failure_count));
                                ui.colored_label(egui::Color32::YELLOW, format!("⚠️ Errors: {}", result.error_count));
                                ui.label(format!("⏱️ Duration: {:.2}s", result.total_duration.as_secs_f64()));
                                
                                // Race type detection
                                let race_color = match result.race_type {
                                    RaceType::QuotaRace => egui::Color32::LIGHT_RED,
                                    RaceType::DoubleSpend => egui::Color32::RED,
                                    RaceType::ResourceRace => egui::Color32::YELLOW,
                                    RaceType::LostUpdate => egui::Color32::LIGHT_BLUE,
                                    RaceType::Unknown => egui::Color32::GRAY,
                                };
                                ui.colored_label(race_color, format!("🎯 Type: {:?}", result.race_type));
                            });
                            
                            ui.separator();
                            
                            // Responses
                            if !result.responses.is_empty() {
                                ui.label(format!("📋 Responses ({}):", result.responses.len()));
                                egui::ScrollArea::vertical()
                                    .max_height(ui.available_height() - 20.0)
                                    .auto_shrink([false; 2])
                                    .show(ui, |ui| {
                                        for (_i, response) in result.responses.iter().enumerate() {
                                            let status_color = match response.status_code {
                                                200..=299 => egui::Color32::GREEN,
                                                400..=499 => egui::Color32::YELLOW,
                                                500..=599 => egui::Color32::RED,
                                                0 => egui::Color32::GRAY,
                                                _ => egui::Color32::WHITE,
                                            };
                                            
                                            ui.collapsing(format!("#{} - {} (T{})", response.request_id, response.status_code, response.thread_id), |ui| {
                                                ui.colored_label(status_color, format!("Status: {}", response.status_code));
                                                ui.label(format!("Thread: {}", response.thread_id));
                                                ui.label(format!("Time: {:.3}s", response.duration.as_secs_f64()));
                                                ui.label(format!("Size: {} bytes", response.body.len()));
                                                
                                                ui.separator();
                                                
                                                ui.collapsing("📥 Response Body", |ui| {
                                                    let mut body_text = response.body.clone();
                                                    ui.add(egui::TextEdit::multiline(&mut body_text)
                                                        .desired_rows(5)
                                                        .interactive(false));
                                                });
                                            });
                                        }
                                    });
                            }
                            
                            self.is_running = false;
                        }
                    }
                }
                RaceTab::WorkflowRace => {
                    if let Ok(results) = self.workflow_results.try_lock() {
                        if let Some(result) = results.as_ref() {
                            ui.horizontal(|ui| {
                                ui.heading("🔄 Workflow Results");
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.button("🗑️ Clear").clicked() {
                                        self.is_running = false;
                                        // Use try_lock to avoid blocking/crashing
                                        if let Ok(mut results) = self.workflow_results.try_lock() {
                                            *results = None;
                                        }
                                    }
                                });
                            });
                            
                            ui.group(|ui| {
                                ui.label(format!("📈 Total: {}", result.total_requests));
                                ui.colored_label(egui::Color32::GREEN, format!("✅ Success: {}", result.success_count));
                                ui.colored_label(egui::Color32::RED, format!("❌ Failure: {}", result.failure_count));
                                ui.colored_label(egui::Color32::YELLOW, format!("⚠️ Errors: {}", result.error_count));
                                ui.label(format!("⏱️ Duration: {:.2}s", result.total_duration.as_secs_f64()));
                            });
                            
                            ui.separator();
                            
                            // Anomalies
                            if !result.anomalies.is_empty() {
                                ui.label("🚨 Anomalies Detected:");
                                for anomaly in &result.anomalies {
                                    ui.colored_label(egui::Color32::LIGHT_RED, format!("• {}", anomaly));
                                }
                                ui.separator();
                            }
                            
                            // Responses
                            if !result.responses.is_empty() {
                                ui.label(format!("📋 Responses ({}):", result.responses.len()));
                                egui::ScrollArea::vertical()
                                    .max_height(ui.available_height() - 20.0)
                                    .auto_shrink([false; 2])
                                    .show(ui, |ui| {
                                        for (i, response) in result.responses.iter().enumerate() {
                                            let status_color = match response.status_code {
                                                200..=299 => egui::Color32::GREEN,
                                                400..=499 => egui::Color32::YELLOW,
                                                500..=599 => egui::Color32::RED,
                                                0 => egui::Color32::GRAY,
                                                _ => egui::Color32::WHITE,
                                            };
                                            
                                            ui.collapsing(format!("#{} - {} ({})", i + 1, response.request_name, response.status_code), |ui| {
                                                ui.label(format!("Request: {}", response.request_name));
                                                ui.colored_label(status_color, format!("Status: {}", response.status_code));
                                                ui.label(format!("Thread: {}", response.thread_id));
                                                ui.label(format!("Time: {:.3}s", response.duration.as_secs_f64()));
                                                ui.label(format!("Size: {} bytes", response.body.len()));
                                                
                                                ui.separator();
                                                
                                                ui.collapsing("📥 Response Body", |ui| {
                                                    let mut body_text = response.body.clone();
                                                    ui.add(egui::TextEdit::multiline(&mut body_text)
                                                        .desired_rows(5)
                                                        .interactive(false));
                                                });
                                            });
                                        }
                                    });
                            }
                            
                            self.is_running = false;
                        }
                    }
                }

            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.current_tab, RaceTab::ReplayRace, "🔄 Replay Race");
                ui.selectable_value(&mut self.current_tab, RaceTab::WorkflowRace, "🔀 Workflow Race");
            });
            
            ui.separator();
            
            match self.current_tab {
                RaceTab::ReplayRace => self.show_replay_race_tab(ui),
                RaceTab::WorkflowRace => self.show_workflow_race_tab(ui),
            }
        });
    }
}

impl RustedRaceApp {
    fn show_replay_race_tab(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Request Configuration
            ui.group(|ui| {
                ui.heading("🔧 Request Configuration");
                ui.add_space(10.0);
                
                ui.label("Raw HTTP Request (paste from Burp Suite):");
                ui.add_sized([ui.available_width(), 200.0], 
                    egui::TextEdit::multiline(&mut self.raw_request)
                        .hint_text("POST /api/endpoint HTTP/1.1\nHost: example.com\nContent-Type: application/json\n\n{\"data\":\"value\"}")
                );
                
                ui.add_space(10.0);
                
                if ui.button("🔍 Parse Request").clicked() {
                    self.parse_burp_request();
                }
            });
            
            ui.add_space(15.0);
            
            // Dynamic Wordlist Configuration
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.heading("📁 Wordlist Configuration");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("➕ Add Wordlist").clicked() {
                            self.wordlists.push((String::new(), Vec::new()));
                        }
                    });
                });
                ui.add_space(10.0);
                
                let mut to_remove = None;
                let mut to_load = None;
                let wordlists_len = self.wordlists.len();
                
                for (i, (path, words)) in self.wordlists.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("{{{{UNIQUE{}}}}} file:", i + 1));
                        ui.add_sized([200.0, 20.0], egui::TextEdit::singleline(path));
                        
                        if ui.button("📂 Load").clicked() {
                            if let Some(file_path) = rfd::FileDialog::new()
                                .add_filter("Text files", &["txt"])
                                .pick_file() {
                                *path = file_path.display().to_string();
                                to_load = Some(i);
                            }
                        }
                        
                        ui.label(format!("({} items)", words.len()));
                        
                        if wordlists_len > 1 && ui.button("🗑").clicked() {
                            to_remove = Some(i);
                        }
                    });
                }
                
                if let Some(index) = to_remove {
                    self.wordlists.remove(index);
                }
                
                if let Some(index) = to_load {
                    self.load_wordlist_file(index);
                }
            });
            
            ui.add_space(15.0);
            
            // Execution Configuration
            ui.group(|ui| {
                ui.heading("⚡ Execution Configuration");
                ui.add_space(10.0);
                
                ui.horizontal(|ui| {
                    ui.label("Threads:");
                    let mut thread_str = self.replay_config.thread_count.to_string();
                    if ui.add_sized([80.0, 20.0], egui::TextEdit::singleline(&mut thread_str)).changed() {
                        if let Ok(threads) = thread_str.parse::<usize>() {
                            self.replay_config.thread_count = threads;
                        }
                    }
                    
                    ui.separator();
                    ui.label("Total Requests:");
                    let mut total_str = self.replay_config.total_requests.to_string();
                    if ui.add_sized([80.0, 20.0], egui::TextEdit::singleline(&mut total_str)).changed() {
                        if let Ok(total) = total_str.parse::<usize>() {
                            self.replay_config.total_requests = total;
                        }
                    }
                });
                
                ui.add_space(10.0);
                
                ui.horizontal(|ui| {
                    ui.label("Mode:");
                    egui::ComboBox::from_label("")
                        .selected_text(match self.replay_config.execution_mode {
                            ReplayExecutionMode::Burst => "Burst",
                            ReplayExecutionMode::Wave => "Wave", 
                            ReplayExecutionMode::Random => "Random",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.replay_config.execution_mode, ReplayExecutionMode::Burst, "Burst");
                            ui.selectable_value(&mut self.replay_config.execution_mode, ReplayExecutionMode::Wave, "Wave");
                            ui.selectable_value(&mut self.replay_config.execution_mode, ReplayExecutionMode::Random, "Random");
                        });
                });
            });
            
            ui.add_space(15.0);
            
            // Action Buttons
            ui.horizontal(|ui| {
                if !self.is_running {
                    if ui.add_sized([150.0, 35.0], egui::Button::new("🚀 Start Replay Race")).clicked() {
                        self.start_replay_race();
                    }
                } else {
                    ui.add_sized([150.0, 35.0], egui::Button::new("⏸️ Running...")).on_disabled_hover_text("Race test in progress");
                    if ui.add_sized([100.0, 35.0], egui::Button::new("🛑 Stop")).clicked() {
                        self.is_running = false;
                    }
                }
                
                if self.is_running {
                    ui.spinner();
                    ui.label("Running replay race...");
                }
            });
            
            ui.add_space(10.0);
            
            // Status Messages
            if !self.error_message.is_empty() {
                ui.group(|ui| {
                    if self.error_message.starts_with("✓") {
                        ui.colored_label(egui::Color32::GREEN, &self.error_message);
                    } else if self.error_message.starts_with("❌") {
                        ui.colored_label(egui::Color32::RED, &self.error_message);
                    } else {
                        ui.colored_label(egui::Color32::YELLOW, &self.error_message);
                    }
                });
            }
        });
    }

    fn show_workflow_race_tab(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Request Configuration Section
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.heading("🔄 Multi-Request Workflow Configuration");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("➕ Add Request").clicked() {
                            self.workflow_config.requests.push(workflow_race::WorkflowRequest::default());
                        }
                    });
                });
                ui.add_space(10.0);
                
                // Request tabs
                ui.horizontal(|ui| {
                    for (i, request) in self.workflow_config.requests.iter().enumerate() {
                        let label = if request.name.is_empty() {
                            format!("Request {}", i + 1)
                        } else {
                            request.name.clone()
                        };
                        
                        if ui.selectable_label(self.selected_request_index == i, label).clicked() {
                            self.selected_request_index = i;
                        }
                    }
                });
                
                ui.separator();
                
                // Selected request configuration
                if self.selected_request_index < self.workflow_config.requests.len() {
                    let mut should_remove = false;
                    let requests_len = self.workflow_config.requests.len();
                    
                    let mut current_request = self.workflow_config.requests[self.selected_request_index].clone();
                    
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.add_sized([150.0, 20.0], egui::TextEdit::singleline(&mut current_request.name));
                        
                        ui.separator();
                        ui.checkbox(&mut current_request.enabled, "Enabled");
                        
                        ui.separator();
                        ui.label("Count:");
                        ui.add_sized([60.0, 20.0], egui::DragValue::new(&mut current_request.request_count).range(1..=100));
                        
                        ui.separator();
                        if ui.button("🗑 Remove").clicked() && requests_len > 1 {
                            should_remove = true;
                        }
                    });
                    
                    ui.add_space(10.0);
                    
                    ui.label("Raw HTTP Request (paste from Burp Suite):");
                    
                    // Store raw request separately to avoid infinite reconstruction
                    if !self.workflow_raw_requests.contains_key(&current_request.id) {
                        self.workflow_raw_requests.insert(current_request.id.clone(), String::new());
                    }
                    
                    let raw_request = self.workflow_raw_requests.get_mut(&current_request.id).unwrap();
                    
                    if ui.add_sized([ui.available_width(), 150.0], 
                        egui::TextEdit::multiline(raw_request)
                            .hint_text("POST /api/endpoint HTTP/1.1\nHost: example.com\nContent-Type: application/json\n\n{\"data\":\"value\"}")
                    ).changed() {
                        // Parse the raw request back to structured format
                        if let Ok(parsed) = http_parser::parse_burp_request(&raw_request) {
                            current_request.method = parsed.method;
                            current_request.url = parsed.url;
                            current_request.headers = parsed.headers;
                            current_request.body = parsed.body;
                        }
                    }
                    
                    // Update the original request with changes
                    self.workflow_config.requests[self.selected_request_index] = current_request;
                    
                    // Handle removal
                    if should_remove {
                        self.workflow_config.requests.remove(self.selected_request_index);
                        if self.selected_request_index >= self.workflow_config.requests.len() {
                            self.selected_request_index = self.workflow_config.requests.len().saturating_sub(1);
                        }
                    }
                }
            });
            
            ui.add_space(15.0);
            
            // Dynamic Wordlist Configuration (shared with replay race)
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.heading("📁 Wordlist Configuration");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("➕ Add Wordlist").clicked() {
                            self.wordlists.push((String::new(), Vec::new()));
                        }
                    });
                });
                ui.add_space(10.0);
                
                let mut to_remove = None;
                let mut to_load = None;
                let wordlists_len = self.wordlists.len();
                
                for (i, (path, words)) in self.wordlists.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("{{{{UNIQUE{}}}}} file:", i + 1));
                        ui.add_sized([200.0, 20.0], egui::TextEdit::singleline(path));
                        
                        if ui.button("📂 Load").clicked() {
                            if let Some(file_path) = rfd::FileDialog::new()
                                .add_filter("Text files", &["txt"])
                                .pick_file() {
                                *path = file_path.display().to_string();
                                to_load = Some(i);
                            }
                        }
                        
                        ui.label(format!("({} items)", words.len()));
                        
                        if wordlists_len > 1 && ui.button("🗑").clicked() {
                            to_remove = Some(i);
                        }
                    });
                }
                
                if let Some(index) = to_remove {
                    self.wordlists.remove(index);
                }
                
                if let Some(index) = to_load {
                    self.load_wordlist_file(index);
                }
            });
            
            ui.add_space(15.0);
            
            // Execution Configuration
            ui.group(|ui| {
                ui.heading("⚡ Execution Configuration");
                ui.add_space(10.0);
                
                ui.horizontal(|ui| {
                    ui.label("Concurrency:");
                    ui.add_sized([80.0, 20.0], egui::TextEdit::singleline(&mut self.concurrency));
                    
                    ui.separator();
                    ui.label("Mode:");
                    egui::ComboBox::from_label("")
                        .selected_text(match self.workflow_config.execution_mode {
                            ExecutionMode::Burst => "Burst",
                            ExecutionMode::Wave => "Wave",
                            ExecutionMode::Random => "Random",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.workflow_config.execution_mode, ExecutionMode::Burst, "Burst");
                            ui.selectable_value(&mut self.workflow_config.execution_mode, ExecutionMode::Wave, "Wave");
                            ui.selectable_value(&mut self.workflow_config.execution_mode, ExecutionMode::Random, "Random");
                        });
                });
                
                ui.add_space(10.0);
                
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.workflow_config.synchronize, "🔄 Synchronize start");
                    ui.separator();
                    ui.checkbox(&mut self.workflow_config.shared_session, "🍪 Shared session");
                });
            });
            
            ui.add_space(15.0);
            
            // Action Buttons
            ui.horizontal(|ui| {
                if !self.is_running {
                    if ui.add_sized([150.0, 35.0], egui::Button::new("🚀 Start Workflow Race")).clicked() {
                        self.start_workflow_race();
                    }
                } else {
                    ui.add_sized([150.0, 35.0], egui::Button::new("⏸️ Running...")).on_disabled_hover_text("Workflow test in progress");
                    if ui.add_sized([100.0, 35.0], egui::Button::new("🛑 Stop")).clicked() {
                        self.is_running = false;
                    }
                }
                
                if self.is_running {
                    ui.spinner();
                    ui.label("Running workflow race...");
                }
            });
            
            ui.add_space(10.0);
            
            // Status Messages
            if !self.error_message.is_empty() {
                ui.group(|ui| {
                    if self.error_message.starts_with("✓") {
                        ui.colored_label(egui::Color32::GREEN, &self.error_message);
                    } else if self.error_message.starts_with("❌") {
                        ui.colored_label(egui::Color32::RED, &self.error_message);
                    } else {
                        ui.colored_label(egui::Color32::YELLOW, &self.error_message);
                    }
                });
            }
        });
    }

    // fn show_session_race_tab(&mut self, ui: &mut egui::Ui) {
    //     ui.label("Session Race - Coming Soon");
    // }

    // fn show_websocket_race_tab(&mut self, ui: &mut egui::Ui) {
    //     ui.label("WebSocket Race - Coming Soon");
    // }

    fn parse_burp_request(&mut self) {
        self.error_message.clear();
        
        match http_parser::parse_burp_request(&self.raw_request) {
            Ok(parsed) => {
                self.replay_config.request.method = parsed.method;
                self.replay_config.request.url = parsed.url;
                self.replay_config.request.headers = parsed.headers;
                self.replay_config.request.body = parsed.body;
                self.error_message = "✓ Request parsed successfully".to_string();
            }
            Err(e) => {
                self.error_message = format!("❌ Parse error: {}", e);
            }
        }
    }

    fn load_wordlist_file(&mut self, index: usize) {
        if index >= self.wordlists.len() {
            return;
        }
        
        let path = &self.wordlists[index].0;
        if path.is_empty() {
            self.error_message = "❌ Please select a file".to_string();
            return;
        }
        
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let words: Vec<String> = content
                    .lines()
                    .map(|line| line.trim().to_string())
                    .filter(|line| !line.is_empty())
                    .collect();
                
                self.wordlists[index].1 = words;
                self.error_message = format!("✓ Loaded {} words from UNIQUE{}", 
                    self.wordlists[index].1.len(), index + 1);
            }
            Err(e) => {
                self.error_message = format!("❌ Failed to load wordlist: {}", e);
            }
        }
    }

    fn start_replay_race(&mut self) {
        if self.raw_request.is_empty() {
            self.error_message = "❌ Please enter a raw HTTP request".to_string();
            return;
        }

        if self.replay_config.thread_count == 0 || self.replay_config.total_requests == 0 {
            self.error_message = "❌ Thread count and total requests must be greater than 0".to_string();
            return;
        }

        self.is_running = true;
        self.error_message = "🚀 Starting replay race test...".to_string();

        let config = self.replay_config.clone();
        let results = Arc::clone(&self.replay_results);
        let wordlists: Vec<Vec<String>> = self.wordlists.iter().map(|(_, words)| words.clone()).collect();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let mut engine = ReplayEngine::new(config);
                engine.set_wordlists(wordlists);
                let result = engine.execute().await;
                
                // Safely update results
                if let Ok(mut results_guard) = results.lock() {
                    *results_guard = Some(result);
                }
            });
        });
    }

    fn start_workflow_race(&mut self) {
        let concurrency = match self.concurrency.parse::<usize>() {
            Ok(n) if n > 0 && n <= 100 => n,
            _ => {
                self.error_message = "❌ Concurrency must be between 1 and 100 for workflow tests".to_string();
                return;
            }
        };

        // Validate requests
        let enabled_requests: Vec<_> = self.workflow_config.requests.iter()
            .filter(|req| req.enabled && !req.url.is_empty())
            .collect();
        
        if enabled_requests.len() < 2 {
            self.error_message = "❌ At least 2 enabled requests with URLs are required".to_string();
            return;
        }

        self.is_running = true;
        self.error_message = "🔄 Starting workflow race test...".to_string();

        let mut config = self.workflow_config.clone();
        config.concurrency = concurrency;
        let results = Arc::clone(&self.workflow_results);

        // Use tokio runtime for async execution
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let engine = WorkflowEngine::new(config);
                let result = engine.execute().await;
                *results.lock().unwrap() = Some(result);
            });
        });
    }
}

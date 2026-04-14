use std::sync::{Arc, Mutex};
use std::time::Duration;

use btleplug::api::Peripheral as _;

use crate::api;
use crate::flash::FlashProgress;
use crate::protocol;
use crate::telemetry;

#[derive(clap::Args)]
pub struct FirmwareArgs {
    /// VIN of the target bike.
    #[arg(long, env = "FUTURIST_VIN")]
    vin: String,

    /// Sold-on date (YYYYMMDD). Defaults to the epoch fallback.
    #[arg(long, env = "FUTURIST_SOLD_ON", default_value = protocol::SOLD_DATE_DEFAULT)]
    sold_on: String,

    /// Seconds to scan for the bike before giving up.
    #[arg(long, default_value_t = 30)]
    scan_timeout: u64,

    /// Stark account email (for checking available updates).
    #[arg(long, env = "FUTURIST_EMAIL")]
    email: Option<String>,

    /// Stark account password.
    #[arg(long, env = "FUTURIST_PASSWORD")]
    password: Option<String>,
}

struct InstalledVersions {
    ble_version: u16,
    blob_fs: String,
    blob_server: String,
    components: Vec<(String, String, String)>, // (name, installed, available)
    vcu_pic: String,
    vcu_top: String,
    vcu_bottom: String,
    vcu_fwfs: String,
    vcu_serial: u32,
    batt_pos_fw: String,
    batt_neg_fw: String,
    batt_serial: u32,
}

#[derive(Default)]
enum UpdateCheckState {
    #[default]
    NotLoggedIn,
    LoggingIn,
    CheckingUpdates,
    Available(Box<AvailableUpdates>),
    Error(String),
}

struct AvailableUpdates {
    vin: String,
    bike_firmware: Option<api::UpdateDetail>,
}

struct SharedState {
    status_msg: String,
    connected: bool,
    installed: Option<InstalledVersions>,
    wifi_ssid: String,
    wifi_password: String,
    update_state: UpdateCheckState,
    flash_status: Option<FlashProgress>,
}

pub fn run(args: FirmwareArgs) -> anyhow::Result<()> {
    let wifi_ssid = crate::crypto::wifi_ssid(&args.vin);
    let wifi_password = crate::crypto::wifi_password(&args.vin, &args.sold_on);

    // Pre-fill login fields from CLI args / env vars.
    let initial_email = args.email.clone().unwrap_or_default();
    let initial_password = args.password.clone().unwrap_or_default();

    let state = Arc::new(Mutex::new(SharedState {
        status_msg: "connecting...".to_string(),
        connected: false,
        installed: None,
        wifi_ssid,
        wifi_password,
        update_state: UpdateCheckState::default(),
        flash_status: None,
    }));

    // BLE task — reads installed firmware from the bike.
    let bg_state = Arc::clone(&state);
    let vin = args.vin.clone();
    let sold_on = args.sold_on.clone();
    let scan_timeout = args.scan_timeout;

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async move {
            ble_task(bg_state, &vin, &sold_on, scan_timeout).await;
        });
    });

    // If credentials were provided on the command line, start the update
    // check immediately in the background.
    if !initial_email.is_empty() && !initial_password.is_empty() {
        let api_state = Arc::clone(&state);
        let email = initial_email.clone();
        let password = initial_password.clone();
        api_state.lock().unwrap().update_state = UpdateCheckState::LoggingIn;

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt.block_on(async move {
                api_task(api_state, &email, &password).await;
            });
        });
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([700.0, 550.0])
            .with_title("futurist — firmware"),
        ..Default::default()
    };

    eframe::run_native(
        "futurist-firmware",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(FirmwareApp {
                state,
                email_buf: initial_email,
                password_buf: initial_password,
                blob_path_buf: String::new(),
                esp_top_path_buf: String::new(),
            }) as Box<dyn eframe::App>)
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {}", e))
}

// ---------------------------------------------------------------------------
// Background tasks
// ---------------------------------------------------------------------------

async fn ble_task(state: Arc<Mutex<SharedState>>, vin: &str, sold_on: &str, scan_timeout: u64) {
    let pin = crate::crypto::generate_pin(vin, sold_on);
    state.lock().unwrap().status_msg = format!("scanning... (PIN: {})", pin);

    let bike =
        match crate::ble::scan_and_connect(vin, sold_on, Duration::from_secs(scan_timeout)).await {
            Ok(b) => b,
            Err(e) => {
                state.lock().unwrap().status_msg = format!("connection failed: {}", e);
                return;
            }
        };

    state.lock().unwrap().status_msg = "reading firmware versions...".to_string();

    let peripheral = bike.peripheral();
    let versions_char = bike.characteristic(protocol::UUID_VERSIONS);
    let vcu_ver_char = bike.characteristic(protocol::UUID_VCU_VERSIONS);
    let batt_fw_char = bike.characteristic(protocol::UUID_BATT_FW_VERSION);

    let mut installed = InstalledVersions {
        ble_version: 0,
        blob_fs: String::new(),
        blob_server: String::new(),
        components: Vec::new(),
        vcu_pic: String::new(),
        vcu_top: String::new(),
        vcu_bottom: String::new(),
        vcu_fwfs: String::new(),
        vcu_serial: 0,
        batt_pos_fw: String::new(),
        batt_neg_fw: String::new(),
        batt_serial: 0,
    };

    if let Some(c) = versions_char
        && let Ok(data) = peripheral.read(c).await
        && let Some(v) = telemetry::BikeVersions::parse(&data)
    {
        installed.ble_version = v.ble_version;
        installed.blob_fs = v.blob_fs;
        installed.blob_server = v.blob_server;
        installed.components = v
            .components
            .into_iter()
            .map(|c| (c.name.to_string(), c.version, c.available))
            .collect();
    }

    if let Some(c) = vcu_ver_char
        && let Ok(data) = peripheral.read(c).await
        && let Some(v) = crate::decode::vcu::VcuVersions::parse(&data)
    {
        installed.vcu_pic = v.pic_vcu;
        installed.vcu_top = v.top_vcu;
        installed.vcu_bottom = v.bottom_vcu;
        installed.vcu_fwfs = v.fwfs;
        installed.vcu_serial = v.serial_number;
    }

    if let Some(c) = batt_fw_char
        && let Ok(data) = peripheral.read(c).await
        && let Some(v) = crate::decode::battery::BatteryFirmwareVersion::parse(&data)
    {
        installed.batt_pos_fw = v.pos_version;
        installed.batt_neg_fw = v.neg_version;
        installed.batt_serial = v.serial;
    }

    let mut s = state.lock().unwrap();
    s.installed = Some(installed);
    s.connected = true;
    s.status_msg = "connected — firmware info loaded".to_string();
}

async fn api_task(state: Arc<Mutex<SharedState>>, email: &str, password: &str) {
    state.lock().unwrap().update_state = UpdateCheckState::LoggingIn;

    let client = match api::StarkApi::sign_in(email, password).await {
        Ok(c) => c,
        Err(e) => {
            state.lock().unwrap().update_state =
                UpdateCheckState::Error(format!("Login failed: {e}"));
            return;
        }
    };

    state.lock().unwrap().update_state = UpdateCheckState::CheckingUpdates;

    match client.check_for_updates().await {
        Ok(resp) => {
            let available = AvailableUpdates {
                vin: resp.vin,
                bike_firmware: resp.firmware.and_then(|f| f.bike_firmware),
            };
            state.lock().unwrap().update_state = UpdateCheckState::Available(Box::new(available));
        }
        Err(e) => {
            state.lock().unwrap().update_state =
                UpdateCheckState::Error(format!("Update check failed: {e}"));
        }
    }
}

async fn flash_task(
    state: Arc<Mutex<SharedState>>,
    blob_path: Option<String>,
    esp_top_path: Option<String>,
) {
    let blob_data = if let Some(ref p) = blob_path {
        match std::fs::read(p) {
            Ok(d) => Some(d),
            Err(e) => {
                state.lock().unwrap().flash_status = Some(FlashProgress::Failed(format!(
                    "Failed to read blob file: {e}"
                )));
                return;
            }
        }
    } else {
        None
    };

    let esp_data = if let Some(ref p) = esp_top_path {
        match std::fs::read(p) {
            Ok(d) => Some(d),
            Err(e) => {
                state.lock().unwrap().flash_status = Some(FlashProgress::Failed(format!(
                    "Failed to read ESP-TOP file: {e}"
                )));
                return;
            }
        }
    } else {
        None
    };

    let progress_state = Arc::clone(&state);
    let result = crate::flash::flash(esp_data.as_deref(), blob_data.as_deref(), move |p| {
        progress_state.lock().unwrap().flash_status = Some(p);
    })
    .await;

    if let Err(e) = result {
        state.lock().unwrap().flash_status = Some(FlashProgress::Failed(e.to_string()));
    }
}

// ---------------------------------------------------------------------------
// GUI
// ---------------------------------------------------------------------------

struct FirmwareApp {
    state: Arc<Mutex<SharedState>>,
    email_buf: String,
    password_buf: String,
    blob_path_buf: String,
    esp_top_path_buf: String,
}

impl eframe::App for FirmwareApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let state = self.state.lock().unwrap();

        egui::TopBottomPanel::top("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let dot = if state.connected { "🟢" } else { "🔴" };
                ui.label(dot);
                ui.label(&state.status_msg);
            });
        });

        // Need to drop the lock before drawing the central panel, because the
        // login button needs &mut self to spawn the API task.
        let is_not_logged_in = matches!(state.update_state, UpdateCheckState::NotLoggedIn);
        let is_error = matches!(state.update_state, UpdateCheckState::Error(_));
        drop(state);

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.draw_wifi_section(ui);

                ui.add_space(12.0);

                self.draw_installed_section(ui);

                ui.add_space(12.0);

                self.draw_updates_section(ui, is_not_logged_in || is_error);

                ui.add_space(12.0);

                self.draw_flash_section(ui);
            });
        });

        ctx.request_repaint();
    }
}

impl FirmwareApp {
    fn draw_wifi_section(&self, ui: &mut egui::Ui) {
        let state = self.state.lock().unwrap();

        ui.heading("Firmware Update Wi-Fi");
        ui.label(
            "To flash firmware, the bike exposes a Wi-Fi AP. Connect to it from your computer.",
        );
        ui.add_space(4.0);
        ui.group(|ui| {
            egui::Grid::new("wifi_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("SSID:");
                    ui.monospace(&state.wifi_ssid);
                    ui.end_row();
                    ui.label("Password:");
                    ui.monospace(&state.wifi_password);
                    ui.end_row();
                    ui.label("Bike IP:");
                    ui.monospace("192.168.167.1");
                    ui.end_row();
                    ui.label("Console port:");
                    ui.monospace("7");
                    ui.end_row();
                    ui.label("Blob port:");
                    ui.monospace("777");
                    ui.end_row();
                    ui.label("ESP top port:");
                    ui.monospace("877");
                    ui.end_row();
                });
        });
    }

    fn draw_installed_section(&self, ui: &mut egui::Ui) {
        let state = self.state.lock().unwrap();

        ui.heading("Installed Firmware");

        if let Some(ref v) = state.installed {
            ui.add_space(4.0);

            ui.group(|ui| {
                ui.strong("Bike");
                egui::Grid::new("bike_ver_grid")
                    .num_columns(2)
                    .spacing([20.0, 2.0])
                    .show(ui, |ui| {
                        ui.label("BLE version:");
                        ui.monospace(format!("{}", v.ble_version));
                        ui.end_row();
                        ui.label("Blob FS:");
                        ui.monospace(&v.blob_fs);
                        ui.end_row();
                        ui.label("Blob server:");
                        ui.monospace(&v.blob_server);
                        ui.end_row();
                    });
            });

            ui.add_space(4.0);

            ui.group(|ui| {
                ui.strong("VCU");
                egui::Grid::new("vcu_ver_grid")
                    .num_columns(2)
                    .spacing([20.0, 2.0])
                    .show(ui, |ui| {
                        ui.label("PIC:");
                        ui.monospace(&v.vcu_pic);
                        ui.end_row();
                        ui.label("ESP Top:");
                        ui.monospace(&v.vcu_top);
                        ui.end_row();
                        ui.label("ESP Bottom:");
                        ui.monospace(&v.vcu_bottom);
                        ui.end_row();
                        ui.label("FWFS:");
                        ui.monospace(&v.vcu_fwfs);
                        ui.end_row();
                        ui.label("Serial:");
                        ui.monospace(format!("{}", v.vcu_serial));
                        ui.end_row();
                    });
            });

            ui.add_space(4.0);

            ui.group(|ui| {
                ui.strong("Battery BMS");
                egui::Grid::new("batt_ver_grid")
                    .num_columns(2)
                    .spacing([20.0, 2.0])
                    .show(ui, |ui| {
                        ui.label("Positive FW:");
                        ui.monospace(&v.batt_pos_fw);
                        ui.end_row();
                        ui.label("Negative FW:");
                        ui.monospace(&v.batt_neg_fw);
                        ui.end_row();
                        ui.label("Serial:");
                        ui.monospace(format!("{}", v.batt_serial));
                        ui.end_row();
                    });
            });

            if !v.components.is_empty() {
                ui.add_space(4.0);

                ui.group(|ui| {
                    ui.strong("Components");
                    egui::Grid::new("comp_ver_grid")
                        .num_columns(3)
                        .spacing([20.0, 2.0])
                        .show(ui, |ui| {
                            ui.strong("Component");
                            ui.strong("Installed");
                            ui.strong("Available");
                            ui.end_row();
                            for (name, ver, avail) in &v.components {
                                ui.label(name);
                                ui.monospace(ver);
                                ui.monospace(avail);
                                ui.end_row();
                            }
                        });
                });
            }
        } else {
            ui.label("Connecting to bike...");
        }
    }

    fn draw_updates_section(&mut self, ui: &mut egui::Ui, show_login: bool) {
        ui.heading("Available Updates");

        if show_login {
            // Show any previous error.
            {
                let state = self.state.lock().unwrap();
                if let UpdateCheckState::Error(ref msg) = state.update_state {
                    ui.colored_label(egui::Color32::from_rgb(255, 100, 100), msg);
                    ui.add_space(4.0);
                }
            }

            ui.label("Log in with your Stark account to check for available firmware updates.");
            ui.add_space(4.0);

            ui.group(|ui| {
                egui::Grid::new("login_grid")
                    .num_columns(2)
                    .spacing([10.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Email:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.email_buf)
                                .desired_width(300.0)
                                .hint_text("you@example.com"),
                        );
                        ui.end_row();
                        ui.label("Password:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.password_buf)
                                .desired_width(300.0)
                                .password(true),
                        );
                        ui.end_row();
                    });

                ui.add_space(6.0);

                let can_login = !self.email_buf.is_empty() && !self.password_buf.is_empty();
                if ui
                    .add_enabled(can_login, egui::Button::new("Log in & check for updates"))
                    .clicked()
                {
                    let api_state = Arc::clone(&self.state);
                    let email = self.email_buf.clone();
                    let password = self.password_buf.clone();

                    api_state.lock().unwrap().update_state = UpdateCheckState::LoggingIn;

                    std::thread::spawn(move || {
                        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
                        rt.block_on(async move {
                            api_task(api_state, &email, &password).await;
                        });
                    });
                }
            });
        } else {
            let state = self.state.lock().unwrap();

            match &state.update_state {
                UpdateCheckState::LoggingIn => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Logging in...");
                    });
                }
                UpdateCheckState::CheckingUpdates => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Checking for updates...");
                    });
                }
                UpdateCheckState::Available(updates) => {
                    ui.label(format!("VIN: {}", updates.vin));
                    ui.add_space(4.0);

                    if let Some(ref fw) = updates.bike_firmware {
                        ui.group(|ui| {
                            ui.strong("Bike Firmware Update Available");
                            egui::Grid::new("avail_fw_grid")
                                .num_columns(2)
                                .spacing([20.0, 2.0])
                                .show(ui, |ui| {
                                    ui.label("Name:");
                                    ui.monospace(&fw.name);
                                    ui.end_row();
                                    ui.label("Type:");
                                    ui.monospace(&fw.kind);
                                    ui.end_row();
                                    ui.label("Build version:");
                                    ui.monospace(&fw.build_version);
                                    ui.end_row();
                                    ui.label("Version number:");
                                    ui.monospace(&fw.version_number);
                                    ui.end_row();
                                    ui.label("Status:");
                                    ui.monospace(&fw.status);
                                    ui.end_row();
                                    ui.label("S3 bucket:");
                                    ui.monospace(&fw.file.bucket);
                                    ui.end_row();
                                    ui.label("S3 key:");
                                    ui.monospace(&fw.file.key);
                                    ui.end_row();
                                    ui.label("Download URL:");
                                    ui.add(
                                        egui::Label::new(egui::RichText::new(&fw.url).monospace())
                                            .wrap_mode(egui::TextWrapMode::Truncate),
                                    );
                                    ui.end_row();
                                });
                        });
                    } else {
                        ui.label("No bike firmware update available — you're up to date.");
                    }
                }
                UpdateCheckState::NotLoggedIn | UpdateCheckState::Error(_) => {
                    // Handled by the show_login branch above.
                }
            }
        }
    }

    fn draw_flash_section(&mut self, ui: &mut egui::Ui) {
        ui.heading("Flash from File");
        ui.label(
            "Connect your computer to the bike's Wi-Fi AP first, then select \
             files to flash.",
        );
        ui.add_space(4.0);

        let flash_in_progress = {
            let state = self.state.lock().unwrap();
            state
                .flash_status
                .as_ref()
                .is_some_and(|p| !matches!(p, FlashProgress::Done | FlashProgress::Failed(_)))
        };

        ui.group(|ui| {
            egui::Grid::new("flash_grid")
                .num_columns(2)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Blob file (.fwb):");
                    ui.add_enabled(
                        !flash_in_progress,
                        egui::TextEdit::singleline(&mut self.blob_path_buf)
                            .desired_width(400.0)
                            .hint_text("/path/to/firmware.fwb"),
                    );
                    ui.end_row();

                    ui.label("ESP-TOP file (.bin):");
                    ui.add_enabled(
                        !flash_in_progress,
                        egui::TextEdit::singleline(&mut self.esp_top_path_buf)
                            .desired_width(400.0)
                            .hint_text("/path/to/vcu-lfs-top.bin (optional)"),
                    );
                    ui.end_row();
                });

            ui.add_space(6.0);

            let has_any_file = !self.blob_path_buf.is_empty() || !self.esp_top_path_buf.is_empty();
            let can_flash = has_any_file && !flash_in_progress;

            if ui
                .add_enabled(can_flash, egui::Button::new("Flash firmware"))
                .clicked()
            {
                let flash_state = Arc::clone(&self.state);
                let blob_path = if self.blob_path_buf.is_empty() {
                    None
                } else {
                    Some(self.blob_path_buf.clone())
                };
                let esp_path = if self.esp_top_path_buf.is_empty() {
                    None
                } else {
                    Some(self.esp_top_path_buf.clone())
                };

                flash_state.lock().unwrap().flash_status = Some(FlashProgress::EnteringFlashMenu);

                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
                    rt.block_on(async move {
                        flash_task(flash_state, blob_path, esp_path).await;
                    });
                });
            }
        });

        // Show flash progress.
        let state = self.state.lock().unwrap();
        if let Some(ref progress) = state.flash_status {
            ui.add_space(4.0);
            match progress {
                FlashProgress::Done => {
                    ui.colored_label(
                        egui::Color32::from_rgb(100, 255, 100),
                        "Flash completed successfully.",
                    );
                }
                FlashProgress::Failed(msg) => {
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 100, 100),
                        format!("Flash failed: {msg}"),
                    );
                }
                other => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(format!("{other}"));
                    });
                }
            }
        }
    }
}

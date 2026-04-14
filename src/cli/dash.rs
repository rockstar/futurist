use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tokio::sync::Mutex;

use crate::protocol;
use crate::telemetry::{self, DecodedTelemetry};

#[derive(clap::Args)]
pub struct DashArgs {
    /// VIN of the target bike.
    #[arg(long, env = "FUTURIST_VIN")]
    vin: String,

    /// Sold-on date (YYYYMMDD). Defaults to the epoch fallback.
    #[arg(long, env = "FUTURIST_SOLD_ON", default_value = protocol::SOLD_DATE_DEFAULT)]
    sold_on: String,

    /// Seconds to scan for the bike before giving up.
    #[arg(long, default_value_t = 30)]
    scan_timeout: u64,
}

/// Shared state between the BLE telemetry task and the egui render loop.
struct SharedState {
    telemetry: DecodedTelemetry,
    status_msg: String,
    connected: bool,
    /// Set by the UI to request a reconnection attempt.
    retry: AtomicBool,
}

pub fn run(args: DashArgs) -> anyhow::Result<()> {
    let state = Arc::new(Mutex::new(SharedState {
        telemetry: DecodedTelemetry::default(),
        status_msg: "connecting...".to_string(),
        connected: false,
        retry: AtomicBool::new(false),
    }));

    // Spawn the tokio runtime + BLE task in a background thread.
    // egui needs the main thread for its event loop on macOS.
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

    // Run the egui window on the main thread.
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("futurist"),
        ..Default::default()
    };

    eframe::run_native(
        "futurist",
        options,
        Box::new(move |_cc| Ok(Box::new(DashApp { state }) as Box<dyn eframe::App>)),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {}", e))
}

async fn ble_task(state: Arc<Mutex<SharedState>>, vin: &str, sold_on: &str, scan_timeout: u64) {
    use futures::StreamExt;

    let pin = crate::crypto::generate_pin(vin, sold_on);

    loop {
        // Reset telemetry for a fresh connection.
        {
            let mut s = state.lock().await;
            s.telemetry = DecodedTelemetry::default();
            s.connected = false;
            s.status_msg = format!("scanning... (PIN: {})", pin);
        }

        let bike = {
            let result =
                crate::ble::scan_and_connect(vin, sold_on, Duration::from_secs(scan_timeout)).await;
            match result {
                Ok(b) => b,
                Err(e) => {
                    state.lock().await.status_msg = format!("{}  — click to retry", e);
                    wait_for_retry(&state).await;
                    continue;
                }
            }
        };

        state.lock().await.status_msg = "subscribing...".to_string();

        let mut stream = {
            let result = telemetry::subscribe(&bike).await;
            match result {
                Ok(s) => s,
                Err(e) => {
                    state.lock().await.status_msg = format!("{}  — click to retry", e);
                    wait_for_retry(&state).await;
                    continue;
                }
            }
        };

        {
            let mut s = state.lock().await;
            s.status_msg = format!("connected to {}", bike.vin());
            s.connected = true;
        }

        while let Some(frame) = stream.next().await {
            let mut s = state.lock().await;
            s.telemetry.update(&frame);
        }

        // Stream ended — bike disconnected.
        {
            let mut s = state.lock().await;
            s.status_msg = "disconnected  — click to reconnect".to_string();
            s.connected = false;
        }
        wait_for_retry(&state).await;
    }
}

async fn wait_for_retry(state: &Arc<Mutex<SharedState>>) {
    loop {
        {
            let s = state.lock().await;
            if s.retry.swap(false, Ordering::SeqCst) {
                return;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

struct DashApp {
    state: Arc<Mutex<SharedState>>,
}

impl eframe::App for DashApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let state = self.state.blocking_lock();

        egui::TopBottomPanel::top("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let dot = if state.connected { "🟢" } else { "🔴" };
                ui.label(dot);
                if state.connected {
                    ui.label(&state.status_msg);
                } else {
                    // Clickable when not connected — triggers retry.
                    let label =
                        egui::Label::new(egui::RichText::new(&state.status_msg).underline())
                            .sense(egui::Sense::click());
                    if ui.add(label).clicked() {
                        state.retry.store(true, Ordering::SeqCst);
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let t = &state.telemetry;

            ui.heading("Ride");
            egui::Grid::new("ride_grid")
                .num_columns(2)
                .spacing([40.0, 8.0])
                .show(ui, |ui| {
                    big_value(
                        ui,
                        "Speed",
                        &fmt_opt_f32(t.speed.as_ref().and_then(|s| s.speed_kmh), 1),
                        "km/h",
                    );
                    big_value(
                        ui,
                        "RPM",
                        &fmt_opt_u16(t.speed.as_ref().and_then(|s| s.motor_rpm)),
                        "",
                    );
                    ui.end_row();

                    big_value(
                        ui,
                        "Throttle",
                        &fmt_opt_u16(t.throttle.as_ref().and_then(|th| th.position)),
                        "",
                    );
                    big_value(
                        ui,
                        "Power",
                        &fmt_opt_i16(t.estimations.as_ref().and_then(|e| e.motor_power_w)),
                        "W",
                    );
                    ui.end_row();

                    big_value(
                        ui,
                        "Mode",
                        &t.ride_mode.map(|m| m.to_string()).unwrap_or("-".into()),
                        "",
                    );
                    big_value(
                        ui,
                        "Range",
                        &fmt_opt_u16(t.estimations.as_ref().and_then(|e| e.range_km)),
                        "km",
                    );
                    ui.end_row();
                });

            ui.separator();
            ui.heading("Battery");
            egui::Grid::new("battery_grid")
                .num_columns(2)
                .spacing([40.0, 8.0])
                .show(ui, |ui| {
                    big_value(
                        ui,
                        "Level",
                        &t.battery_percent
                            .map(|p| format!("{}%", p))
                            .unwrap_or("-".into()),
                        "",
                    );
                    big_value(
                        ui,
                        "SOC",
                        &fmt_opt_u16(t.batt_soc.as_ref().and_then(|s| s.soc)),
                        "",
                    );
                    ui.end_row();

                    big_value(
                        ui,
                        "SOH",
                        &fmt_opt_u16(t.batt_soc.as_ref().and_then(|s| s.soh)),
                        "",
                    );
                    big_value(
                        ui,
                        "DC Bus",
                        &fmt_opt_f32(t.batt_soc.as_ref().and_then(|s| s.dc_bus_v), 1),
                        "V",
                    );
                    ui.end_row();

                    big_value(
                        ui,
                        "Current",
                        &fmt_opt_i16(t.batt_signals.as_ref().and_then(|s| s.current)),
                        "A",
                    );
                    if let Some(ref p) = t.batt_params {
                        big_value(ui, "Config", &format!("{}s{}p", p.series, p.parallel), "");
                    } else {
                        big_value(ui, "Config", "-", "");
                    }
                    ui.end_row();
                });

            ui.separator();
            ui.heading("Temperatures");
            egui::Grid::new("temp_grid")
                .num_columns(2)
                .spacing([40.0, 8.0])
                .show(ui, |ui| {
                    if let Some(ref inv) = t.inv_temps {
                        big_value(ui, "Motor", &fmt_opt_f32(inv.motor.sensor1, 1), "°C");
                        big_value(ui, "IGBT", &fmt_opt_f32(inv.igbt.sensor1, 1), "°C");
                    } else {
                        big_value(ui, "Motor", "-", "°C");
                        big_value(ui, "IGBT", "-", "°C");
                    }
                    ui.end_row();

                    if let Some(ref pcb) = t.inv_pcb {
                        big_value(ui, "PCB", &fmt_opt_u16(pcb.pcb_temp), "");
                        big_value(ui, "Humidity", &fmt_opt_f32(pcb.pcb_humidity_pct, 1), "%");
                    } else {
                        big_value(ui, "PCB", "-", "");
                        big_value(ui, "Humidity", "-", "%");
                    }
                    ui.end_row();
                });

            ui.separator();
            ui.heading("Totals");
            egui::Grid::new("totals_grid")
                .num_columns(2)
                .spacing([40.0, 8.0])
                .show(ui, |ui| {
                    if let Some(ref tot) = t.totals {
                        let odo_km = tot.odometer_m.map(|m| format!("{:.1}", m as f64 / 1000.0));
                        big_value(ui, "Odometer", &odo_km.unwrap_or("-".into()), "km");
                        big_value(ui, "Energy", &fmt_opt_u32(tot.watt_hours), "Wh");
                        ui.end_row();
                        big_value(ui, "Ride Time", &fmt_opt_u32(tot.total_time_secs), "s");
                        big_value(ui, "Airtime", &fmt_opt_u32(tot.airtime_secs), "s");
                    } else {
                        big_value(ui, "Odometer", "-", "km");
                        big_value(ui, "Energy", "-", "Wh");
                        ui.end_row();
                        big_value(ui, "Ride Time", "-", "s");
                        big_value(ui, "Airtime", "-", "s");
                    }
                    ui.end_row();
                });

            // Status flags at the bottom.
            if let Some(ref s) = t.status {
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    status_pill(ui, "DRIVE", s.drive());
                    status_pill(ui, "ARMED", s.armed_throttle());
                    status_pill(ui, "CHARGING", s.is_charging());
                    status_pill(ui, "CHARGER", s.charger_connected());
                    status_pill(ui, "FAN", s.fan_on());
                    status_pill(ui, "PUMP", s.pump_on());
                });
            }

            if let Some(ref id) = t.identity {
                ui.separator();
                ui.label(format!("VIN: {}  Sold: {}", id.vin, id.sold_date));
            }
        });

        // Continuous repaint so telemetry updates appear immediately.
        ctx.request_repaint();
    }
}

fn big_value(ui: &mut egui::Ui, label: &str, value: &str, unit: &str) {
    ui.vertical(|ui| {
        ui.label(egui::RichText::new(label).small().weak());
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(value).size(28.0).strong());
            if !unit.is_empty() {
                ui.label(egui::RichText::new(unit).small().weak());
            }
        });
    });
}

fn status_pill(ui: &mut egui::Ui, label: &str, active: bool) {
    let color = if active {
        egui::Color32::from_rgb(50, 200, 50)
    } else {
        egui::Color32::from_rgb(80, 80, 80)
    };
    let text = egui::RichText::new(label).small().color(color);
    ui.label(text);
}

fn fmt_opt_u16(v: Option<u16>) -> String {
    v.map(|n| n.to_string()).unwrap_or("-".into())
}

fn fmt_opt_i16(v: Option<i16>) -> String {
    v.map(|n| n.to_string()).unwrap_or("-".into())
}

fn fmt_opt_u32(v: Option<u32>) -> String {
    v.map(|n| n.to_string()).unwrap_or("-".into())
}

fn fmt_opt_f32(v: Option<f32>, decimals: usize) -> String {
    v.map(|n| format!("{n:.decimals$}")).unwrap_or("-".into())
}

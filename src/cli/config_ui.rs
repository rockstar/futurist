use std::sync::{Arc, Mutex};
use std::time::Duration;

use btleplug::api::{Characteristic, Peripheral as _, WriteType};
use futures::StreamExt;
use tokio::time::timeout;

use crate::presets;
use crate::protocol;

#[derive(clap::Args)]
pub struct ConfigUiArgs {
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

// ---------------------------------------------------------------------------
// Data models
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct MapSlot {
    power_hp: u8,
    regen: u8,
    curve: u8,
}

impl Default for MapSlot {
    fn default() -> Self {
        Self {
            power_hp: 40,
            regen: 70,
            curve: 0,
        }
    }
}

#[derive(Clone)]
struct CurveData {
    index: u8,
    /// Whether this curve was readable from the bike.
    readable: bool,
    torque: Vec<u16>,
    regen: Vec<u16>,
}

impl CurveData {
    fn empty(index: u8) -> Self {
        Self {
            index,
            readable: false,
            torque: vec![1000; 15],
            regen: vec![1000; 15],
        }
    }
}

#[derive(Clone, Default)]
struct MiscData {
    maps: u8,
    inactive_timeout: u16,
    auto_power_off: u16,
}

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Maps,
    Curves,
    Misc,
}

struct SharedState {
    status_msg: String,
    connected: bool,
    write_msg: Option<String>,

    // Maps
    slots: Vec<MapSlot>,
    maps_dirty: bool,
    write_maps_requested: bool,

    // Curves
    curves: Vec<CurveData>,
    curves_dirty: bool,
    write_curves_requested: bool,

    // Misc
    misc: MiscData,
    misc_dirty: bool,
    write_misc_requested: bool,

    // Shared signals
    read_requested: bool,
}

pub fn run(args: ConfigUiArgs) -> anyhow::Result<()> {
    let state = Arc::new(Mutex::new(SharedState {
        status_msg: "connecting...".to_string(),
        connected: false,
        write_msg: None,
        slots: (0..5).map(|_| MapSlot::default()).collect(),
        maps_dirty: false,
        write_maps_requested: false,
        curves: (0..5).map(CurveData::empty).collect(),
        curves_dirty: false,
        write_curves_requested: false,
        misc: MiscData::default(),
        misc_dirty: false,
        write_misc_requested: false,
        read_requested: false,
    }));

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

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("futurist — config"),
        ..Default::default()
    };

    eframe::run_native(
        "futurist-config",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(ConfigApp {
                state,
                active_tab: Tab::Maps,
            }) as Box<dyn eframe::App>)
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {}", e))
}

// ---------------------------------------------------------------------------
// BLE background task
// ---------------------------------------------------------------------------

const TYPE_MAP_CONFIG: u8 = 0;
const TYPE_CURVES_CONFIG: u8 = 1;
const TYPE_MISC_CONFIG: u8 = 3;

async fn ble_task(state: Arc<Mutex<SharedState>>, vin: &str, sold_on: &str, scan_timeout: u64) {
    let pin = crate::crypto::generate_pin(vin, sold_on);
    {
        let mut s = state.lock().unwrap();
        s.status_msg = format!("scanning... (PIN: {})", pin);
    }

    let bike =
        match crate::ble::scan_and_connect(vin, sold_on, Duration::from_secs(scan_timeout)).await {
            Ok(b) => b,
            Err(e) => {
                state.lock().unwrap().status_msg = format!("connection failed: {}", e);
                return;
            }
        };

    let peripheral = bike.peripheral();
    let vcu_config_char = match bike.characteristic(protocol::UUID_VCU_CONFIG) {
        Some(c) => c.clone(),
        None => {
            state.lock().unwrap().status_msg = "VCU Config characteristic not found".to_string();
            return;
        }
    };

    if let Err(e) = peripheral.subscribe(&vcu_config_char).await {
        state.lock().unwrap().status_msg = format!("subscribe failed: {}", e);
        return;
    }

    let mut notifications = match peripheral.notifications().await {
        Ok(n) => n,
        Err(e) => {
            state.lock().unwrap().status_msg = format!("notifications failed: {}", e);
            return;
        }
    };

    {
        let mut s = state.lock().unwrap();
        s.status_msg = format!("connected to {}", bike.vin());
        s.connected = true;
    }

    // Initial read of all config types.
    read_all_configs(peripheral, &vcu_config_char, &mut notifications, &state).await;

    state.lock().unwrap().status_msg = "connected — all configs loaded".to_string();

    // Event loop: wait for UI signals.
    loop {
        let (wants_read, wants_write_maps, wants_write_curves, wants_write_misc) = {
            let mut s = state.lock().unwrap();
            let r = s.read_requested;
            let wm = s.write_maps_requested;
            let wc = s.write_curves_requested;
            let wmsc = s.write_misc_requested;
            s.read_requested = false;
            s.write_maps_requested = false;
            s.write_curves_requested = false;
            s.write_misc_requested = false;
            (r, wm, wc, wmsc)
        };

        if wants_write_maps {
            write_all_maps(peripheral, &vcu_config_char, &mut notifications, &state).await;
        }

        if wants_write_curves {
            write_all_curves(peripheral, &vcu_config_char, &mut notifications, &state).await;
        }

        if wants_write_misc {
            write_misc(peripheral, &vcu_config_char, &mut notifications, &state).await;
        }

        if wants_read {
            read_all_configs(peripheral, &vcu_config_char, &mut notifications, &state).await;
            state.lock().unwrap().status_msg = "connected — all configs loaded".to_string();
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

async fn read_config_response(
    notifications: &mut (impl futures::Stream<Item = btleplug::api::ValueNotification> + Unpin),
    expected_type: u8,
) -> Option<Vec<u8>> {
    timeout(Duration::from_secs(5), async {
        while let Some(notif) = notifications.next().await {
            if notif.uuid == protocol::UUID_VCU_CONFIG
                && notif.value.len() >= 2
                && notif.value[0] == 2
                && notif.value[1] == expected_type
            {
                return Some(notif.value[3..].to_vec());
            }
        }
        None
    })
    .await
    .ok()
    .flatten()
}

async fn read_all_maps(
    peripheral: &btleplug::platform::Peripheral,
    char: &Characteristic,
    notifications: &mut (impl futures::Stream<Item = btleplug::api::ValueNotification> + Unpin),
    state: &Arc<Mutex<SharedState>>,
) {
    state.lock().unwrap().status_msg = "reading map configs...".to_string();

    for slot in 0u8..5 {
        let request = [0x00, TYPE_MAP_CONFIG, slot];
        if peripheral
            .write(char, &request, WriteType::WithResponse)
            .await
            .is_err()
        {
            continue;
        }
        if let Some(data) = read_config_response(notifications, TYPE_MAP_CONFIG).await {
            if data.len() < 6 {
                continue;
            }
            let torque_raw = i16::from_le_bytes([data[1], data[2]]);
            let response_slot = data[0] as usize;
            let mut s = state.lock().unwrap();
            if response_slot < s.slots.len() {
                s.slots[response_slot] = MapSlot {
                    power_hp: (torque_raw as f32 / 1.25) as u8,
                    regen: i16::from_le_bytes([data[3], data[4]]) as u8,
                    curve: data[5],
                };
            }
        }
    }

    let mut s = state.lock().unwrap();
    s.maps_dirty = false;
    s.write_msg = None;
}

async fn read_all_curves(
    peripheral: &btleplug::platform::Peripheral,
    char: &Characteristic,
    notifications: &mut (impl futures::Stream<Item = btleplug::api::ValueNotification> + Unpin),
    state: &Arc<Mutex<SharedState>>,
) {
    state.lock().unwrap().status_msg = "reading throttle curves...".to_string();

    for idx in 0u8..5 {
        let request = [0x00, TYPE_CURVES_CONFIG, idx];
        if peripheral
            .write(char, &request, WriteType::WithResponse)
            .await
            .is_err()
        {
            continue;
        }
        if let Some(data) = read_config_response(notifications, TYPE_CURVES_CONFIG).await {
            if data.is_empty() {
                // Curve 0 (built-in) returns empty.
                let mut s = state.lock().unwrap();
                if (idx as usize) < s.curves.len() {
                    s.curves[idx as usize] = CurveData::empty(idx);
                }
                continue;
            }
            if data.len() >= 61 {
                let curve_index = data[0] as usize;
                let mut torque = Vec::with_capacity(15);
                let mut regen = Vec::with_capacity(15);
                for i in 0..15 {
                    let off = i * 4;
                    torque.push(u16::from_le_bytes([data[off + 1], data[off + 2]]));
                    regen.push(u16::from_le_bytes([data[off + 3], data[off + 4]]));
                }
                let mut s = state.lock().unwrap();
                if curve_index < s.curves.len() {
                    s.curves[curve_index] = CurveData {
                        index: curve_index as u8,
                        readable: true,
                        torque,
                        regen,
                    };
                }
            }
        }
    }
}

async fn read_all_configs(
    peripheral: &btleplug::platform::Peripheral,
    char: &Characteristic,
    notifications: &mut (impl futures::Stream<Item = btleplug::api::ValueNotification> + Unpin),
    state: &Arc<Mutex<SharedState>>,
) {
    read_all_maps(peripheral, char, notifications, state).await;
    read_all_curves(peripheral, char, notifications, state).await;
    read_single_config(
        peripheral,
        char,
        notifications,
        state,
        TYPE_MISC_CONFIG,
        "misc",
    )
    .await;
}

async fn read_single_config(
    peripheral: &btleplug::platform::Peripheral,
    char: &Characteristic,
    notifications: &mut (impl futures::Stream<Item = btleplug::api::ValueNotification> + Unpin),
    state: &Arc<Mutex<SharedState>>,
    config_type: u8,
    label: &str,
) {
    state.lock().unwrap().status_msg = format!("reading {} config...", label);

    let request = [0x00, config_type];
    if peripheral
        .write(char, &request, WriteType::WithResponse)
        .await
        .is_err()
    {
        return;
    }
    if let Some(data) = read_config_response(notifications, config_type).await {
        let mut s = state.lock().unwrap();
        match config_type {
            TYPE_MISC_CONFIG if data.len() >= 5 => {
                s.misc = MiscData {
                    maps: data[0],
                    inactive_timeout: u16::from_le_bytes([data[1], data[2]]),
                    auto_power_off: u16::from_le_bytes([data[3], data[4]]),
                };
                s.misc_dirty = false;
            }
            _ => {}
        }
    }
}

async fn write_all_maps(
    peripheral: &btleplug::platform::Peripheral,
    char: &Characteristic,
    notifications: &mut (impl futures::Stream<Item = btleplug::api::ValueNotification> + Unpin),
    state: &Arc<Mutex<SharedState>>,
) {
    state.lock().unwrap().status_msg = "writing map configs...".to_string();

    let slots: Vec<MapSlot> = state.lock().unwrap().slots.clone();

    for (i, slot) in slots.iter().enumerate() {
        let torque_raw = (slot.power_hp as f32 * 1.25) as i16;
        let regen = slot.regen as i16;
        let torque_bytes = torque_raw.to_le_bytes();
        let regen_bytes = regen.to_le_bytes();

        let request = [
            0x01,
            TYPE_MAP_CONFIG,
            i as u8,
            0x01,
            torque_bytes[0],
            torque_bytes[1],
            regen_bytes[0],
            regen_bytes[1],
            slot.curve,
        ];
        if let Err(e) = peripheral
            .write(char, &request, WriteType::WithResponse)
            .await
        {
            let mut s = state.lock().unwrap();
            s.write_msg = Some(format!("write failed on slot {}: {}", i, e));
            s.status_msg = "write failed".to_string();
            return;
        }

        let _ = read_config_response(notifications, TYPE_MAP_CONFIG).await;
    }

    let mut s = state.lock().unwrap();
    s.maps_dirty = false;
    s.write_msg = Some("maps written successfully!".to_string());
    s.status_msg = "connected — maps saved".to_string();
}

async fn write_all_curves(
    peripheral: &btleplug::platform::Peripheral,
    char: &Characteristic,
    notifications: &mut (impl futures::Stream<Item = btleplug::api::ValueNotification> + Unpin),
    state: &Arc<Mutex<SharedState>>,
) {
    state.lock().unwrap().status_msg = "writing curves...".to_string();

    let curves: Vec<CurveData> = state.lock().unwrap().curves.clone();

    for curve in &curves {
        // Skip curve 0 (built-in, not writable) and unreadable curves.
        if curve.index == 0 || !curve.readable {
            continue;
        }

        // Write format: [readWrite=1, type=1, save, curve_index, pad(0x7FFF), pad(0x7FFF),
        //                 15x torque(i16 LE), 15x regen(i16 LE)]
        let mut payload = Vec::with_capacity(68);
        payload.push(0x01); // readWrite = write
        payload.push(TYPE_CURVES_CONFIG);
        payload.push(0x01); // save
        payload.push(curve.index);
        // Two i16 padding values (0x7FFF)
        payload.extend_from_slice(&0x7FFFi16.to_le_bytes());
        payload.extend_from_slice(&0x7FFFi16.to_le_bytes());
        for &t in &curve.torque {
            payload.extend_from_slice(&(t as i16).to_le_bytes());
        }
        for &r in &curve.regen {
            payload.extend_from_slice(&(r as i16).to_le_bytes());
        }

        if let Err(e) = peripheral
            .write(char, &payload, WriteType::WithResponse)
            .await
        {
            let mut s = state.lock().unwrap();
            s.write_msg = Some(format!("curve {} write failed: {}", curve.index, e));
            s.status_msg = "write failed".to_string();
            return;
        }

        let _ = read_config_response(notifications, TYPE_CURVES_CONFIG).await;
    }

    let mut s = state.lock().unwrap();
    s.curves_dirty = false;
    s.write_msg = Some("curves written successfully!".to_string());
    s.status_msg = "connected — curves saved".to_string();
}

async fn write_misc(
    peripheral: &btleplug::platform::Peripheral,
    char: &Characteristic,
    notifications: &mut (impl futures::Stream<Item = btleplug::api::ValueNotification> + Unpin),
    state: &Arc<Mutex<SharedState>>,
) {
    state.lock().unwrap().status_msg = "writing misc config...".to_string();

    let misc = state.lock().unwrap().misc.clone();

    let timeout_bytes = misc.inactive_timeout.to_le_bytes();
    let power_off_bytes = misc.auto_power_off.to_le_bytes();

    // Write: [readWrite=1, type=3, save=1, maps, inactive_lo, inactive_hi, poweroff_lo, poweroff_hi]
    let request = [
        0x01,
        TYPE_MISC_CONFIG,
        0x01, // save
        misc.maps,
        timeout_bytes[0],
        timeout_bytes[1],
        power_off_bytes[0],
        power_off_bytes[1],
    ];
    if let Err(e) = peripheral
        .write(char, &request, WriteType::WithResponse)
        .await
    {
        let mut s = state.lock().unwrap();
        s.write_msg = Some(format!("misc write failed: {}", e));
        s.status_msg = "write failed".to_string();
        return;
    }

    let _ = read_config_response(notifications, TYPE_MISC_CONFIG).await;

    let mut s = state.lock().unwrap();
    s.misc_dirty = false;
    s.write_msg = Some("misc config saved!".to_string());
    s.status_msg = "connected — misc saved".to_string();
}

// ---------------------------------------------------------------------------
// egui app
// ---------------------------------------------------------------------------

struct ConfigApp {
    state: Arc<Mutex<SharedState>>,
    active_tab: Tab,
}

impl eframe::App for ConfigApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut state = self.state.lock().unwrap();

        // Status bar
        egui::TopBottomPanel::top("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let dot = if state.connected { "🟢" } else { "🔴" };
                ui.label(dot);
                ui.label(&state.status_msg);
            });
        });

        // Tab bar
        egui::TopBottomPanel::top("tab_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, Tab::Maps, "Power Modes");
                ui.selectable_value(&mut self.active_tab, Tab::Curves, "Power Curves");
                ui.selectable_value(&mut self.active_tab, Tab::Misc, "Settings");
            });
        });

        // Fixed action bar at the bottom — always visible.
        egui::TopBottomPanel::bottom("action_bar").show(ctx, |ui| match self.active_tab {
            Tab::Maps => render_maps_actions(ui, &mut state),
            Tab::Curves => render_curves_actions(ui, &mut state),
            Tab::Misc => render_misc_actions(ui, &mut state),
        });

        // Scrollable tab content fills the rest.
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| match self.active_tab {
                Tab::Maps => render_maps_tab(ui, &mut state),
                Tab::Curves => render_curves_tab(ui, &mut state),
                Tab::Misc => render_misc_tab(ui, &mut state),
            });
        });

        ctx.request_repaint();
    }
}

fn render_maps_tab(ui: &mut egui::Ui, state: &mut SharedState) {
    ui.heading("Power Modes");
    ui.label("Adjust power, regen, and throttle curve for each map slot.");
    ui.add_space(8.0);

    // Preset buttons.
    ui.horizontal(|ui| {
        ui.label("Load preset to all slots:");
        for preset in presets::PRESETS {
            if ui.button(preset.name).clicked() {
                for slot in &mut state.slots {
                    slot.power_hp = preset.power_hp;
                    slot.regen = preset.regen;
                    slot.curve = preset.curve;
                }
                state.maps_dirty = true;
            }
        }
    });
    ui.add_space(8.0);

    // Per-slot editors.
    let mut changed = false;
    for (i, slot) in state.slots.iter_mut().enumerate() {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.strong(format!("Slot {}", i));
                ui.add_space(20.0);

                ui.label("Power:");
                let mut hp = slot.power_hp as f32;
                let slider = egui::Slider::new(&mut hp, 0.0..=presets::MAX_POWER_HP as f32)
                    .suffix(" hp")
                    .fixed_decimals(0);
                if ui.add(slider).changed() {
                    slot.power_hp = hp as u8;
                    changed = true;
                }

                ui.label("Regen:");
                let mut regen = slot.regen as f32;
                let slider = egui::Slider::new(&mut regen, 0.0..=presets::MAX_REGEN as f32)
                    .suffix("%")
                    .fixed_decimals(0);
                if ui.add(slider).changed() {
                    slot.regen = regen as u8;
                    changed = true;
                }

                ui.label("Curve:");
                let mut curve = slot.curve as usize;
                if egui::ComboBox::from_id_salt(format!("curve_{}", i))
                    .width(50.0)
                    .show_index(ui, &mut curve, 5, |i| format!("{}", i))
                    .changed()
                {
                    slot.curve = curve as u8;
                    changed = true;
                }
            });
        });
    }
    if changed {
        state.maps_dirty = true;
    }
}

fn render_maps_actions(ui: &mut egui::Ui, state: &mut SharedState) {
    ui.horizontal(|ui| {
        let apply = egui::Button::new("Apply to bike");
        if ui
            .add_enabled(state.maps_dirty && state.connected, apply)
            .clicked()
        {
            state.write_maps_requested = true;
        }

        if ui
            .add_enabled(state.connected, egui::Button::new("Re-read from bike"))
            .clicked()
        {
            state.read_requested = true;
        }

        if state.maps_dirty {
            ui.label(
                egui::RichText::new("unsaved changes")
                    .color(egui::Color32::YELLOW)
                    .small(),
            );
        }

        if let Some(ref msg) = state.write_msg {
            let color = if msg.contains("failed") {
                egui::Color32::RED
            } else {
                egui::Color32::GREEN
            };
            ui.label(egui::RichText::new(msg).color(color));
        }
    });
}

/// RPM breakpoints for the 15 curve points (0, 1000, 2000, ..., 14000).
const RPM_LABELS: [&str; 15] = [
    "0", "1k", "2k", "3k", "4k", "5k", "6k", "7k", "8k", "9k", "10k", "11k", "12k", "13k", "14k",
];

fn render_curves_tab(ui: &mut egui::Ui, state: &mut SharedState) {
    ui.heading("Power Curves");
    ui.label(
        "Each point sets the % of available power at that RPM (1000 = 100%). \
         The 15 points span 0 to 14,000 RPM in 1k steps.",
    );
    ui.add_space(8.0);

    let mut changed = false;

    egui::ScrollArea::vertical().show(ui, |ui| {
        for curve in &mut state.curves {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.strong(format!("Curve {}", curve.index));
                    if !curve.readable {
                        ui.label(
                            egui::RichText::new("(built-in default, not editable)")
                                .weak()
                                .italics(),
                        );
                    }
                });

                if !curve.readable {
                    return;
                }

                // Graph visualization.
                let available_width = ui.available_width();
                let height = 160.0;
                let (response, painter) =
                    ui.allocate_painter(egui::vec2(available_width, height), egui::Sense::hover());
                let rect = response.rect;

                painter.rect_filled(rect, 4.0, egui::Color32::from_gray(25));

                let margin_x = 40.0; // room for Y axis labels
                let margin_top = 8.0;
                let margin_bottom = 32.0; // room for X axis labels
                let margin_right = 8.0;
                let inner = egui::Rect::from_min_max(
                    egui::pos2(rect.left() + margin_x, rect.top() + margin_top),
                    egui::pos2(rect.right() - margin_right, rect.bottom() - margin_bottom),
                );

                let max_val = 1100.0f32;

                // Character zone shading.
                let zones = [
                    (
                        0.0,
                        0.28,
                        "Off the line",
                        egui::Color32::from_rgba_premultiplied(50, 120, 50, 15),
                    ),
                    (
                        0.28,
                        0.57,
                        "Mid-range",
                        egui::Color32::from_rgba_premultiplied(120, 120, 50, 15),
                    ),
                    (
                        0.57,
                        1.0,
                        "Top end",
                        egui::Color32::from_rgba_premultiplied(120, 50, 50, 15),
                    ),
                ];
                for (start_frac, end_frac, label, color) in &zones {
                    let x0 = inner.left() + start_frac * inner.width();
                    let x1 = inner.left() + end_frac * inner.width();
                    let zone_rect = egui::Rect::from_min_max(
                        egui::pos2(x0, inner.top()),
                        egui::pos2(x1, inner.bottom()),
                    );
                    painter.rect_filled(zone_rect, 0.0, *color);
                    painter.text(
                        egui::pos2((x0 + x1) / 2.0, inner.top() + 4.0),
                        egui::Align2::CENTER_TOP,
                        label,
                        egui::FontId::proportional(9.0),
                        egui::Color32::from_gray(90),
                    );
                }

                // Y axis: power % labels.
                for pct in [0, 25, 50, 75, 100] {
                    let y = inner.bottom() - (pct as f32 * 10.0 / max_val) * inner.height();
                    // Grid line.
                    painter.line_segment(
                        [egui::pos2(inner.left(), y), egui::pos2(inner.right(), y)],
                        egui::Stroke::new(
                            if pct == 100 { 1.0 } else { 0.5 },
                            egui::Color32::from_gray(if pct == 100 { 80 } else { 45 }),
                        ),
                    );
                    // Label.
                    painter.text(
                        egui::pos2(inner.left() - 4.0, y),
                        egui::Align2::RIGHT_CENTER,
                        format!("{}%", pct),
                        egui::FontId::proportional(9.0),
                        egui::Color32::from_gray(100),
                    );
                }

                // X axis: feel labels (primary) with RPM (secondary).
                let x_labels = [
                    (0, "Start"),
                    (3, "Low"),
                    (7, "Mid"),
                    (11, "High"),
                    (14, "Max"),
                ];
                for (idx, label) in &x_labels {
                    let x = inner.left() + (*idx as f32 / 14.0) * inner.width();
                    painter.text(
                        egui::pos2(x, inner.bottom() + 3.0),
                        egui::Align2::CENTER_TOP,
                        label,
                        egui::FontId::proportional(10.0),
                        egui::Color32::from_gray(140),
                    );
                    painter.text(
                        egui::pos2(x, inner.bottom() + 15.0),
                        egui::Align2::CENTER_TOP,
                        RPM_LABELS[*idx],
                        egui::FontId::proportional(8.0),
                        egui::Color32::from_gray(70),
                    );
                }

                // Draw the curves.
                draw_curve_line_in(
                    &painter,
                    inner,
                    &curve.torque,
                    max_val,
                    egui::Color32::from_rgb(50, 220, 50),
                );
                draw_curve_line_in(
                    &painter,
                    inner,
                    &curve.regen,
                    max_val,
                    egui::Color32::from_rgb(255, 150, 50),
                );

                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::from_rgb(50, 220, 50), "● Torque");
                    ui.colored_label(egui::Color32::from_rgb(255, 150, 50), "● Regen");
                });

                // Preset shape buttons.
                ui.horizontal(|ui| {
                    ui.label("Shape:");
                    if ui.small_button("Flat (100%)").clicked() {
                        // Full power at all RPMs — the default.
                        curve.torque = vec![1000; 15];
                        curve.regen = vec![1000; 15];
                        changed = true;
                    }
                    if ui.small_button("Low-end tame").clicked() {
                        // Reduced power at low RPM, full at high RPM.
                        // Gentler off the line but full top-end.
                        curve.torque = vec![
                            300, 400, 500, 600, 700, 800, 850, 900, 940, 960, 980, 990, 1000, 1000,
                            1000,
                        ];
                        curve.regen = vec![1000; 15];
                        changed = true;
                    }
                    if ui.small_button("Mid-range boost").clicked() {
                        // Moderate low-end, peaks in the mid-range, slight taper.
                        curve.torque = vec![
                            500, 600, 750, 900, 1000, 1000, 1000, 1000, 1000, 950, 900, 850, 800,
                            750, 700,
                        ];
                        curve.regen = vec![1000; 15];
                        changed = true;
                    }
                    if ui.small_button("Top-end only").clicked() {
                        // Very little at low RPM, ramps up for high RPM.
                        curve.torque = vec![
                            100, 150, 200, 250, 350, 450, 550, 650, 750, 850, 900, 950, 980, 1000,
                            1000,
                        ];
                        curve.regen = vec![1000; 15];
                        changed = true;
                    }
                });

                // Editable point values.
                egui::CollapsingHeader::new("Edit points by RPM")
                    .id_salt(format!("edit_{}", curve.index))
                    .show(ui, |ui| {
                        ui.label("Torque (% × 10 at each RPM):");
                        ui.horizontal_wrapped(|ui| {
                            for (i, val) in curve.torque.iter_mut().enumerate() {
                                let mut v = *val as f32;
                                let drag = egui::DragValue::new(&mut v)
                                    .range(0.0..=1000.0)
                                    .speed(5.0)
                                    .prefix(format!("{}:", RPM_LABELS[i]));
                                if ui.add(drag).changed() {
                                    *val = v as u16;
                                    changed = true;
                                }
                            }
                        });
                        ui.label("Regen (% × 10 at each RPM):");
                        ui.horizontal_wrapped(|ui| {
                            for (i, val) in curve.regen.iter_mut().enumerate() {
                                let mut v = *val as f32;
                                let drag = egui::DragValue::new(&mut v)
                                    .range(0.0..=1000.0)
                                    .speed(5.0)
                                    .prefix(format!("{}:", RPM_LABELS[i]));
                                if ui.add(drag).changed() {
                                    *val = v as u16;
                                    changed = true;
                                }
                            }
                        });
                    });
            });
            ui.add_space(4.0);
        }
    });

    if changed {
        state.curves_dirty = true;
    }
}

/// Action bar rendered in the bottom panel for the curves tab.
fn render_curves_actions(ui: &mut egui::Ui, state: &mut SharedState) {
    ui.horizontal(|ui| {
        let apply = egui::Button::new("Apply curves to bike");
        if ui
            .add_enabled(state.curves_dirty && state.connected, apply)
            .clicked()
        {
            state.write_curves_requested = true;
        }

        if ui
            .add_enabled(state.connected, egui::Button::new("Re-read from bike"))
            .clicked()
        {
            state.read_requested = true;
        }

        if state.curves_dirty {
            ui.label(
                egui::RichText::new("unsaved changes")
                    .color(egui::Color32::YELLOW)
                    .small(),
            );
        }

        if let Some(ref msg) = state.write_msg {
            let color = if msg.contains("failed") {
                egui::Color32::RED
            } else {
                egui::Color32::GREEN
            };
            ui.label(egui::RichText::new(msg).color(color));
        }
    });
}

fn render_misc_tab(ui: &mut egui::Ui, state: &mut SharedState) {
    ui.heading("Settings");
    ui.add_space(8.0);

    let mut changed = false;

    ui.group(|ui| {
        ui.label(format!("Maps configured: {}", state.misc.maps));
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Inactive timeout:");
            let mut v = state.misc.inactive_timeout as f32;
            if ui
                .add(egui::DragValue::new(&mut v).range(0.0..=3600.0).suffix("s"))
                .changed()
            {
                state.misc.inactive_timeout = v as u16;
                changed = true;
            }
        });

        ui.horizontal(|ui| {
            ui.label("Auto power off:");
            let mut v = state.misc.auto_power_off as f32;
            if ui
                .add(egui::DragValue::new(&mut v).range(0.0..=7200.0).suffix("s"))
                .changed()
            {
                state.misc.auto_power_off = v as u16;
                changed = true;
            }
            let mins = state.misc.auto_power_off / 60;
            ui.label(format!("({}m)", mins));
        });
    });

    if changed {
        state.misc_dirty = true;
    }
}

fn render_misc_actions(ui: &mut egui::Ui, state: &mut SharedState) {
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                state.misc_dirty && state.connected,
                egui::Button::new("Apply"),
            )
            .clicked()
        {
            state.write_misc_requested = true;
        }
        if ui
            .add_enabled(state.connected, egui::Button::new("Re-read from bike"))
            .clicked()
        {
            state.read_requested = true;
        }
        if state.misc_dirty {
            ui.label(
                egui::RichText::new("unsaved changes")
                    .color(egui::Color32::YELLOW)
                    .small(),
            );
        }
        if let Some(ref msg) = state.write_msg {
            let color = if msg.contains("failed") {
                egui::Color32::RED
            } else {
                egui::Color32::GREEN
            };
            ui.label(egui::RichText::new(msg).color(color));
        }
    });
}

/// Draw a curve line within an already-computed inner rectangle.
fn draw_curve_line_in(
    painter: &egui::Painter,
    inner: egui::Rect,
    values: &[u16],
    max_val: f32,
    color: egui::Color32,
) {
    if values.is_empty() {
        return;
    }
    let n = values.len();

    let points: Vec<egui::Pos2> = values
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let x = inner.left() + (i as f32 / (n - 1) as f32) * inner.width();
            let y = inner.bottom() - (v as f32 / max_val) * inner.height();
            egui::pos2(x, y)
        })
        .collect();

    // Fill under the curve with a subtle tint.
    let mut fill_points = points.clone();
    fill_points.push(egui::pos2(inner.right(), inner.bottom()));
    fill_points.push(egui::pos2(inner.left(), inner.bottom()));
    let fill_color =
        egui::Color32::from_rgba_premultiplied(color.r() / 3, color.g() / 3, color.b() / 3, 30);
    painter.add(egui::Shape::convex_polygon(
        fill_points,
        fill_color,
        egui::Stroke::NONE,
    ));

    // Draw the line.
    for window in points.windows(2) {
        painter.line_segment([window[0], window[1]], egui::Stroke::new(2.5, color));
    }

    // Draw dots at each point.
    for &pt in &points {
        painter.circle_filled(pt, 3.5, color);
    }
}

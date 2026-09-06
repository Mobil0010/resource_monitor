#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::collections::VecDeque;
#[cfg(target_os = "macos")]
use std::ffi::{c_char, c_void};
use std::process::Command;
#[cfg(target_os = "macos")]
use std::sync::OnceLock;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align, Align2, Color32, FontId, Layout, Margin, RichText, Sense, Stroke, StrokeKind, Vec2,
};
use sysinfo::{Components, Disks, Networks, ProcessesToUpdate, System};

const BLUE: Color32 = Color32::from_rgb(92, 145, 255);
const GREEN: Color32 = Color32::from_rgb(68, 196, 130);
const PURPLE: Color32 = Color32::from_rgb(170, 112, 255);
const ORANGE: Color32 = Color32::from_rgb(246, 162, 74);
const HISTORY: usize = 60;
const UPDATE_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
const RELEASE_API: &str = "https://api.github.com/repos/Mobil0010/resource_monitor/releases/latest";

#[cfg(target_os = "macos")]
static MAC_REOPEN_REQUESTED: AtomicBool = AtomicBool::new(false);
#[cfg(target_os = "macos")]
static MAC_EGUI_CONTEXT: OnceLock<egui::Context> = OnceLock::new();

#[cfg(target_os = "macos")]
#[link(name = "objc")]
unsafe extern "C" {
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> *const c_void;
    fn class_addMethod(
        class: *mut c_void,
        selector: *const c_void,
        implementation: *const c_void,
        types: *const c_char,
    ) -> i8;
}

#[cfg(target_os = "macos")]
unsafe extern "C" fn mac_application_should_handle_reopen(
    _: *mut c_void,
    _: *const c_void,
    _: *mut c_void,
    _: i8,
) -> i8 {
    MAC_REOPEN_REQUESTED.store(true, Ordering::Release);
    if let Some(ctx) = MAC_EGUI_CONTEXT.get() {
        ctx.request_repaint();
    }
    1
}

#[cfg(target_os = "macos")]
fn install_macos_reopen_handler(ctx: &egui::Context) {
    let _ = MAC_EGUI_CONTEXT.set(ctx.clone());
    // winit이 등록한 앱 델리게이트는 유지하고, 구현하지 않은 Dock 재열기 콜백만 추가합니다.
    unsafe {
        let class = objc_getClass(c"WinitApplicationDelegate".as_ptr());
        if !class.is_null() {
            let selector =
                sel_registerName(c"applicationShouldHandleReopen:hasVisibleWindows:".as_ptr());
            class_addMethod(
                class,
                selector,
                mac_application_should_handle_reopen as *const () as *const c_void,
                c"c@:@c".as_ptr(),
            );
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Page {
    Overview,
    Cpu,
    Gpu,
    Memory,
    Disks,
    Processes,
    Network,
    Settings,
}

impl Page {
    const ALL: [Self; 8] = [
        Self::Overview,
        Self::Cpu,
        Self::Gpu,
        Self::Memory,
        Self::Disks,
        Self::Processes,
        Self::Network,
        Self::Settings,
    ];
    fn key(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Cpu => "cpu",
            Self::Gpu => "gpu",
            Self::Memory => "memory",
            Self::Disks => "disks",
            Self::Processes => "processes",
            Self::Network => "network",
            Self::Settings => "settings",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Language {
    English,
    Korean,
    Japanese,
}

impl Language {
    const ALL: [Self; 3] = [Self::English, Self::Korean, Self::Japanese];
    fn native_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Korean => "한국어",
            Self::Japanese => "日本語",
        }
    }
    fn from_code(code: u8) -> Self {
        match code {
            1 => Self::Korean,
            2 => Self::Japanese,
            _ => Self::English,
        }
    }
    fn code(self) -> u8 {
        Self::ALL
            .iter()
            .position(|value| *value == self)
            .unwrap_or(0) as u8
    }
}

#[derive(Clone, Copy, PartialEq)]
enum PopupPosition {
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl PopupPosition {
    const ALL: [Self; 6] = [
        Self::TopLeft,
        Self::TopCenter,
        Self::TopRight,
        Self::BottomLeft,
        Self::BottomCenter,
        Self::BottomRight,
    ];
    fn key(self) -> &'static str {
        match self {
            Self::TopLeft => "top_left",
            Self::TopCenter => "top_center",
            Self::TopRight => "top_right",
            Self::BottomLeft => "bottom_left",
            Self::BottomCenter => "bottom_center",
            Self::BottomRight => "bottom_right",
        }
    }
    fn from_code(code: u8) -> Self {
        Self::ALL
            .get(code as usize)
            .copied()
            .unwrap_or(Self::TopRight)
    }
    fn code(self) -> u8 {
        Self::ALL
            .iter()
            .position(|value| *value == self)
            .unwrap_or(2) as u8
    }
}

struct GpuInfo {
    name: String,
    detail: String,
    usage: f32,
    memory: u64,
    details_rx: Option<Receiver<String>>,
    usage_rx: Option<Receiver<String>>,
}

impl GpuInfo {
    fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = sender.send(gpu_command_output(true));
        });
        Self {
            name: "GPU".into(),
            detail: platform().into(),
            usage: 0.0,
            memory: 0,
            details_rx: Some(receiver),
            usage_rx: None,
        }
    }
    fn refresh(&mut self) {
        if let Some(receiver) = &self.details_rx {
            match receiver.try_recv() {
                Ok(text) => {
                    self.name = extract_after(&text, "Chipset Model:")
                        .or_else(|| extract_after(&text, "Name="))
                        .unwrap_or_else(|| "GPU".into());
                    self.detail = extract_after(&text, "Total Number of Cores:")
                        .map(|cores| format!("{cores} cores"))
                        .unwrap_or_else(|| platform().into());
                    self.details_rx = None;
                }
                Err(TryRecvError::Disconnected) => self.details_rx = None,
                Err(TryRecvError::Empty) => {}
            }
        }
        if let Some(receiver) = &self.usage_rx {
            match receiver.try_recv() {
                Ok(text) => {
                    self.usage = extract_number(&text, "Device Utilization %")
                        .or_else(|| extract_number(&text, "GPU_USAGE"))
                        .unwrap_or(self.usage)
                        .clamp(0.0, 100.0);
                    self.memory = extract_number(&text, "Alloc system memory")
                        .map(|value| value as u64)
                        .unwrap_or(self.memory);
                    self.usage_rx = None;
                }
                Err(TryRecvError::Disconnected) => self.usage_rx = None,
                Err(TryRecvError::Empty) => {}
            }
        }
        if self.usage_rx.is_none() {
            let (sender, receiver) = mpsc::channel();
            std::thread::spawn(move || {
                let _ = sender.send(gpu_command_output(false));
            });
            self.usage_rx = Some(receiver);
        }
    }
}

fn gpu_command_output(details: bool) -> String {
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = details;
    #[cfg(target_os = "macos")]
    let output = if details {
        Command::new("system_profiler")
            .arg("SPDisplaysDataType")
            .output()
    } else {
        Command::new("ioreg")
            .args(["-r", "-d", "1", "-w", "0", "-c", "AGXAccelerator"])
            .output()
    };
    #[cfg(target_os = "windows")]
    let output = if details {
        Command::new("powershell").args(["-NoProfile", "-Command", "Get-CimInstance Win32_VideoController | Select-Object -First 1 | ForEach-Object { 'Name=' + $_.Name }"]).output()
    } else {
        Command::new("powershell").args(["-NoProfile", "-Command", "$v=(Get-Counter '\\GPU Engine(*)\\Utilization Percentage' -ErrorAction SilentlyContinue).CounterSamples.CookedValue | Measure-Object -Sum; 'GPU_USAGE=' + $v.Sum"]).output()
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let output = Command::new("sh").args(["-c", "true"]).output();
    output
        .ok()
        .map(|value| String::from_utf8_lossy(&value.stdout).into_owned())
        .unwrap_or_default()
}

fn extract_after(text: &str, key: &str) -> Option<String> {
    text.lines()
        .find_map(|line| {
            line.split_once(key)
                .map(|(_, value)| value.trim().trim_matches('"').to_owned())
        })
        .filter(|value| !value.is_empty())
}

fn extract_number(text: &str, key: &str) -> Option<f32> {
    let start = text.find(key)? + key.len();
    let value = text[start..].trim_start_matches(|c: char| !c.is_ascii_digit() && c != '.');
    value
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .next()?
        .parse()
        .ok()
}

struct App {
    sys: System,
    components: Components,
    gpu: GpuInfo,
    disks: Disks,
    networks: Networks,
    page: Page,
    last: Instant,
    last_temperature_refresh: Instant,
    cpu: VecDeque<f32>,
    gpu_history: VecDeque<f32>,
    memory: VecDeque<f32>,
    down: VecDeque<f32>,
    up: VecDeque<f32>,
    disk_history: VecDeque<f32>,
    process_history: VecDeque<f32>,
    search: String,
    dark: bool,
    popup: bool,
    popup_closed: Arc<AtomicBool>,
    exiting: bool,
    popup_position: PopupPosition,
    popup_cpu: bool,
    popup_gpu: bool,
    popup_memory: bool,
    popup_disk: bool,
    popup_processes: bool,
    popup_network: bool,
    popup_opacity: f32,
    language: Language,
    popup_graphs: bool,
    confirm_exit: bool,
    autostart: bool,
    settings_message: Option<String>,
    refresh_secs: u64,
    saved_settings: String,
    update_rx: Option<Receiver<Option<UpdateInfo>>>,
    last_update_check: Instant,
    available_update: Option<UpdateInfo>,
    dismissed_update: Option<String>,
    update_download_rx: Option<Receiver<Result<std::path::PathBuf, String>>>,
    downloaded_update: Option<std::path::PathBuf>,
    update_download_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct UpdateInfo {
    version: String,
    asset_url: String,
    asset_name: String,
}

#[derive(Clone)]
struct SavedSettings {
    dark: bool,
    popup: bool,
    popup_position: PopupPosition,
    popup_cpu: bool,
    popup_gpu: bool,
    popup_memory: bool,
    popup_disk: bool,
    popup_processes: bool,
    popup_network: bool,
    popup_opacity: f32,
    language: Language,
    popup_graphs: bool,
    refresh_secs: u64,
}

impl SavedSettings {
    fn defaults(dark: bool) -> Self {
        Self {
            dark,
            popup: false,
            popup_position: PopupPosition::TopRight,
            popup_cpu: true,
            popup_gpu: false,
            popup_memory: true,
            popup_disk: false,
            popup_processes: false,
            popup_network: true,
            popup_opacity: 0.92,
            language: Language::English,
            popup_graphs: true,
            refresh_secs: 2,
        }
    }

    fn load(system_dark: bool) -> Self {
        let mut settings = Self::defaults(system_dark);
        let Ok(value) = std::fs::read_to_string(settings_file()) else {
            return settings;
        };
        settings.apply(&value);
        settings
    }

    fn apply(&mut self, value: &str) {
        for line in value.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key {
                "dark" => self.dark = parse_bool(value).unwrap_or(self.dark),
                "popup" => self.popup = parse_bool(value).unwrap_or(self.popup),
                "popup_position" => {
                    self.popup_position = value
                        .parse::<u8>()
                        .ok()
                        .map(PopupPosition::from_code)
                        .unwrap_or(self.popup_position)
                }
                "popup_cpu" => self.popup_cpu = parse_bool(value).unwrap_or(self.popup_cpu),
                "popup_gpu" => self.popup_gpu = parse_bool(value).unwrap_or(self.popup_gpu),
                "popup_memory" => {
                    self.popup_memory = parse_bool(value).unwrap_or(self.popup_memory)
                }
                "popup_disk" => self.popup_disk = parse_bool(value).unwrap_or(self.popup_disk),
                "popup_processes" => {
                    self.popup_processes = parse_bool(value).unwrap_or(self.popup_processes)
                }
                "popup_network" => {
                    self.popup_network = parse_bool(value).unwrap_or(self.popup_network)
                }
                "popup_opacity" => {
                    if let Ok(opacity) = value.parse::<f32>()
                        && opacity.is_finite()
                    {
                        self.popup_opacity = opacity.clamp(0.30, 1.0);
                    }
                }
                "language" => {
                    self.language = value
                        .parse::<u8>()
                        .ok()
                        .map(Language::from_code)
                        .unwrap_or(self.language)
                }
                "popup_graphs" => {
                    self.popup_graphs = parse_bool(value).unwrap_or(self.popup_graphs)
                }
                "refresh_secs" => {
                    self.refresh_secs = value
                        .parse::<u64>()
                        .unwrap_or(self.refresh_secs)
                        .clamp(1, 10)
                }
                _ => {}
            }
        }
    }

    fn encode(&self) -> String {
        format!(
            concat!(
                "version=1\n",
                "dark={}\n",
                "popup={}\n",
                "popup_position={}\n",
                "popup_cpu={}\n",
                "popup_gpu={}\n",
                "popup_memory={}\n",
                "popup_disk={}\n",
                "popup_processes={}\n",
                "popup_network={}\n",
                "popup_opacity={:.3}\n",
                "language={}\n",
                "popup_graphs={}\n",
                "refresh_secs={}\n"
            ),
            u8::from(self.dark),
            u8::from(self.popup),
            self.popup_position.code(),
            u8::from(self.popup_cpu),
            u8::from(self.popup_gpu),
            u8::from(self.popup_memory),
            u8::from(self.popup_disk),
            u8::from(self.popup_processes),
            u8::from(self.popup_network),
            self.popup_opacity,
            self.language.code(),
            u8::from(self.popup_graphs),
            self.refresh_secs,
        )
    }
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_fonts(&cc.egui_ctx);
        #[cfg(target_os = "macos")]
        install_macos_reopen_handler(&cc.egui_ctx);
        let system_dark = matches!(
            cc.egui_ctx
                .system_theme()
                .unwrap_or_else(|| cc.egui_ctx.theme()),
            egui::Theme::Dark
        );
        let settings = SavedSettings::load(system_dark);
        let dark = settings.dark;
        set_style(&cc.egui_ctx, dark);
        let saved_settings = settings.encode();
        let update_rx = Some(start_update_check(cc.egui_ctx.clone()));
        let mut app = Self {
            sys: System::new(),
            components: Components::new_with_refreshed_list(),
            gpu: GpuInfo::new(),
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            page: Page::Overview,
            last: Instant::now() - Duration::from_secs(2),
            last_temperature_refresh: Instant::now() - Duration::from_secs(10),
            cpu: VecDeque::new(),
            gpu_history: VecDeque::new(),
            memory: VecDeque::new(),
            down: VecDeque::new(),
            up: VecDeque::new(),
            disk_history: VecDeque::new(),
            process_history: VecDeque::new(),
            search: String::new(),
            dark,
            popup: settings.popup,
            popup_closed: Arc::new(AtomicBool::new(false)),
            exiting: false,
            popup_position: settings.popup_position,
            popup_cpu: settings.popup_cpu,
            popup_gpu: settings.popup_gpu,
            popup_memory: settings.popup_memory,
            popup_disk: settings.popup_disk,
            popup_processes: settings.popup_processes,
            popup_network: settings.popup_network,
            popup_opacity: settings.popup_opacity,
            language: settings.language,
            popup_graphs: settings.popup_graphs,
            confirm_exit: false,
            autostart: autostart_enabled(),
            settings_message: None,
            refresh_secs: settings.refresh_secs,
            saved_settings,
            update_rx,
            last_update_check: Instant::now(),
            available_update: None,
            dismissed_update: None,
            update_download_rx: None,
            downloaded_update: None,
            update_download_error: None,
        };
        app.refresh();
        app
    }

    fn current_settings(&self) -> SavedSettings {
        SavedSettings {
            dark: self.dark,
            popup: self.popup,
            popup_position: self.popup_position,
            popup_cpu: self.popup_cpu,
            popup_gpu: self.popup_gpu,
            popup_memory: self.popup_memory,
            popup_disk: self.popup_disk,
            popup_processes: self.popup_processes,
            popup_network: self.popup_network,
            popup_opacity: self.popup_opacity,
            language: self.language,
            popup_graphs: self.popup_graphs,
            refresh_secs: self.refresh_secs,
        }
    }

    fn save_settings_if_changed(&mut self) {
        // 종료를 위해 팝업을 숨기는 내부 상태는 사용자의 팝업 설정을 덮어쓰지 않습니다.
        if self.exiting {
            return;
        }
        let value = self.current_settings().encode();
        if value == self.saved_settings {
            return;
        }
        match save_settings(&value) {
            Ok(()) => {
                self.saved_settings = value;
                self.settings_message = None;
            }
            Err(error) => self.settings_message = Some(error),
        }
    }

    fn poll_update_check(&mut self, ctx: &egui::Context) {
        if let Some(receiver) = &self.update_rx {
            match receiver.try_recv() {
                Ok(update) => {
                    self.available_update = update.filter(|value| {
                        self.dismissed_update.as_deref() != Some(value.version.as_str())
                    });
                    self.update_rx = None;
                }
                Err(TryRecvError::Disconnected) => self.update_rx = None,
                Err(TryRecvError::Empty) => {}
            }
        }
        if self.update_rx.is_none() && self.last_update_check.elapsed() >= UPDATE_INTERVAL {
            self.update_rx = Some(start_update_check(ctx.clone()));
            self.last_update_check = Instant::now();
        }
    }

    fn poll_update_download(&mut self) {
        let Some(receiver) = &self.update_download_rx else {
            return;
        };
        match receiver.try_recv() {
            Ok(Ok(path)) => {
                self.downloaded_update = Some(path);
                self.update_download_rx = None;
            }
            Ok(Err(error)) => {
                self.update_download_error = Some(error);
                self.update_download_rx = None;
            }
            Err(TryRecvError::Disconnected) => {
                self.update_download_error = Some("Update download stopped.".to_owned());
                self.update_download_rx = None;
            }
            Err(TryRecvError::Empty) => {}
        }
    }

    fn refresh(&mut self) {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        if self.last_temperature_refresh.elapsed() >= Duration::from_secs(10) {
            self.components.refresh(true);
            self.last_temperature_refresh = Instant::now();
        }
        let popup_visible = self.popup;
        if self.page == Page::Processes || (popup_visible && self.popup_processes) {
            self.sys.refresh_processes(ProcessesToUpdate::All, true);
        }
        if matches!(self.page, Page::Overview | Page::Gpu) || (popup_visible && self.popup_gpu) {
            self.gpu.refresh();
        }
        if matches!(self.page, Page::Overview | Page::Disks) || (popup_visible && self.popup_disk) {
            self.disks.refresh(true);
        }
        if matches!(self.page, Page::Overview | Page::Network)
            || (popup_visible && self.popup_network)
        {
            self.networks.refresh(true);
        }
        let cpu = self.sys.global_cpu_usage();
        let mem = pct(self.sys.used_memory(), self.sys.total_memory());
        let down = self.networks.iter().map(|(_, n)| n.received()).sum::<u64>() as f32;
        let up = self
            .networks
            .iter()
            .map(|(_, n)| n.transmitted())
            .sum::<u64>() as f32;
        push(&mut self.cpu, cpu);
        push(&mut self.gpu_history, self.gpu.usage);
        push(&mut self.memory, mem);
        push(&mut self.down, down);
        push(&mut self.up, up);
        let disk_total: u64 = self.disks.iter().map(|disk| disk.total_space()).sum();
        let disk_used: u64 = self
            .disks
            .iter()
            .map(|disk| disk.total_space().saturating_sub(disk.available_space()))
            .sum();
        push(&mut self.disk_history, pct(disk_used, disk_total));
        push(
            &mut self.process_history,
            self.sys.processes().len().min(100) as f32,
        );
        self.last = Instant::now();
    }

    fn sidebar(&mut self, root: &mut egui::Ui) {
        egui::Panel::left("nav")
            .exact_size(205.0)
            .frame(
                egui::Frame::new()
                    .fill(if self.dark {
                        Color32::from_rgb(20, 22, 27)
                    } else {
                        Color32::from_rgb(239, 241, 245)
                    })
                    .inner_margin(Margin::symmetric(16, 20)),
            )
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("RM").strong().size(19.0).color(BLUE));
                    ui.label(RichText::new("Resource Monitor").strong().size(15.0));
                });
                ui.add_space(25.0);
                for page in Page::ALL {
                    if navigation_button(ui, page, self.page == page, self.language).clicked() {
                        self.page = page;
                    }
                    ui.add_space(3.0);
                }
                ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                    ui.label(
                        RichText::new(format!("{}  •  Live", platform()))
                            .color(GREEN)
                            .size(12.0),
                    );
                    ui.add_space(8.0);
                    if ui
                        .add_sized(
                            [ui.available_width(), 34.0],
                            egui::Button::new(
                                RichText::new(tr(self.language, "quit_app"))
                                    .color(Color32::from_rgb(235, 95, 95)),
                            ),
                        )
                        .clicked()
                    {
                        self.confirm_exit = true;
                    }
                    ui.add_space(8.0);
                });
            });
    }

    fn overview(&self, ui: &mut egui::Ui) {
        let cpu = self.sys.global_cpu_usage();
        let memory = pct(self.sys.used_memory(), self.sys.total_memory());
        let total = self.disks.iter().map(|d| d.total_space()).sum();
        let used = self
            .disks
            .iter()
            .map(|d| d.total_space().saturating_sub(d.available_space()))
            .sum();
        let down = self.networks.iter().map(|(_, n)| n.received()).sum::<u64>();
        let up = self
            .networks
            .iter()
            .map(|(_, n)| n.transmitted())
            .sum::<u64>();
        ui.columns(5, |c| {
            metric(&mut c[0], "CPU", cpu, BLUE);
            metric(&mut c[1], "GPU", self.gpu.usage, GREEN);
            metric(&mut c[2], tr(self.language, "memory"), memory, PURPLE);
            metric(
                &mut c[3],
                tr(self.language, "disk"),
                pct(used, total),
                ORANGE,
            );
            info(
                &mut c[4],
                tr(self.language, "network"),
                &format!("↓ {}  ↑ {}", rate(down), rate(up)),
                GREEN,
            );
        });
        ui.add_space(14.0);
        ui.columns(2, |c| {
            chart(
                &mut c[0],
                tr(self.language, "cpu_history"),
                &self.cpu,
                100.0,
                BLUE,
                true,
            );
            chart(
                &mut c[1],
                tr(self.language, "memory_history"),
                &self.memory,
                100.0,
                PURPLE,
                true,
            );
        });
        ui.add_space(14.0);
        card(ui, |ui| {
            ui.label(
                RichText::new(tr(self.language, "system_info"))
                    .strong()
                    .size(16.0),
            );
            ui.add_space(10.0);
            egui::Grid::new("system")
                .spacing([30.0, 10.0])
                .show(ui, |ui| {
                    pair(
                        ui,
                        tr(self.language, "operating_system"),
                        &format!(
                            "{} {}",
                            System::name().unwrap_or_default(),
                            System::os_version().unwrap_or_default()
                        ),
                    );
                    pair(
                        ui,
                        tr(self.language, "host_name"),
                        &System::host_name().unwrap_or_default(),
                    );
                    ui.end_row();
                    pair(
                        ui,
                        tr(self.language, "kernel"),
                        &System::kernel_version().unwrap_or_default(),
                    );
                    pair(ui, tr(self.language, "uptime"), &uptime(System::uptime()));
                    ui.end_row();
                    pair(
                        ui,
                        tr(self.language, "temperatures"),
                        &sensor_summary(&self.components),
                    );
                    ui.end_row();
                });
        });
    }

    fn cpu_page(&self, ui: &mut egui::Ui) {
        let brand = self
            .sys
            .cpus()
            .first()
            .map(|c| c.brand())
            .unwrap_or("Unknown CPU");
        hero(
            ui,
            tr(self.language, "processor"),
            brand,
            &temperature_text(temperature_for(&self.components, "cpu"))
                .map(|temperature| format!("{:.1}%  •  {temperature}", self.sys.global_cpu_usage()))
                .unwrap_or_else(|| format!("{:.1}%", self.sys.global_cpu_usage())),
            BLUE,
        );
        ui.add_space(14.0);
        chart(
            ui,
            tr(self.language, "cpu_last_60"),
            &self.cpu,
            100.0,
            BLUE,
            true,
        );
        ui.add_space(14.0);
        card(ui, |ui| {
            ui.label(
                RichText::new(tr(self.language, "logical_processors"))
                    .strong()
                    .size(16.0),
            );
            ui.add_space(10.0);
            egui::Grid::new("cores")
                .num_columns(2)
                .spacing([16.0, 9.0])
                .show(ui, |ui| {
                    for (i, cpu) in self.sys.cpus().iter().enumerate() {
                        ui.label(format!("{} {}", tr(self.language, "core"), i + 1));
                        bar(ui, cpu.cpu_usage(), BLUE);
                        ui.end_row();
                    }
                });
        });
    }

    fn gpu_page(&self, ui: &mut egui::Ui) {
        hero(
            ui,
            tr(self.language, "graphics_processor"),
            &format!("{} • {}", self.gpu.name, self.gpu.detail),
            &temperature_text(temperature_for(&self.components, "gpu"))
                .map(|temperature| format!("{:.1}%  •  {temperature}", self.gpu.usage))
                .unwrap_or_else(|| format!("{:.1}%", self.gpu.usage)),
            GREEN,
        );
        ui.add_space(14.0);
        chart(
            ui,
            tr(self.language, "gpu_last_60"),
            &self.gpu_history,
            100.0,
            GREEN,
            true,
        );
        ui.add_space(14.0);
        ui.columns(2, |c| {
            info(
                &mut c[0],
                tr(self.language, "gpu_name"),
                &self.gpu.name,
                GREEN,
            );
            info(
                &mut c[1],
                tr(self.language, "allocated_memory"),
                &bytes(self.gpu.memory),
                PURPLE,
            );
        });
    }

    fn memory_page(&self, ui: &mut egui::Ui) {
        let usage = pct(self.sys.used_memory(), self.sys.total_memory());
        hero(
            ui,
            tr(self.language, "physical_memory"),
            &format!("{} total", bytes(self.sys.total_memory())),
            &temperature_text(temperature_for(&self.components, "memory"))
                .map(|temperature| format!("{usage:.1}%  •  {temperature}"))
                .unwrap_or_else(|| format!("{usage:.1}%")),
            PURPLE,
        );
        ui.add_space(14.0);
        chart(
            ui,
            tr(self.language, "memory_last_60"),
            &self.memory,
            100.0,
            PURPLE,
            true,
        );
        ui.add_space(14.0);
        ui.columns(4, |c| {
            info(
                &mut c[0],
                tr(self.language, "used"),
                &bytes(self.sys.used_memory()),
                PURPLE,
            );
            info(
                &mut c[1],
                tr(self.language, "available"),
                &bytes(self.sys.available_memory()),
                GREEN,
            );
            info(
                &mut c[2],
                tr(self.language, "swap_used"),
                &bytes(self.sys.used_swap()),
                ORANGE,
            );
            info(
                &mut c[3],
                tr(self.language, "swap_total"),
                &bytes(self.sys.total_swap()),
                BLUE,
            );
        });
    }

    fn disks_page(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "{}: {}",
                    tr(self.language, "mounted_volumes"),
                    self.disks.len()
                ))
                .weak(),
            );
            if let Some(temperature) = temperature_text(temperature_for(&self.components, "disk")) {
                ui.label(
                    RichText::new(format!(
                        "• {}: {temperature}",
                        tr(self.language, "temperature")
                    ))
                    .color(ORANGE),
                );
            }
        });
        ui.add_space(10.0);
        for d in &self.disks {
            let total = d.total_space();
            let used = total.saturating_sub(d.available_space());
            card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(d.name().to_string_lossy())
                                .strong()
                                .size(16.0),
                        );
                        ui.label(RichText::new(d.mount_point().display().to_string()).weak());
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(format!(
                            "{}: {}",
                            tr(self.language, "free"),
                            bytes(d.available_space())
                        ));
                    });
                });
                ui.add_space(10.0);
                bar(ui, pct(used, total), ORANGE);
                ui.label(
                    RichText::new(format!(
                        "{}: {} / {}",
                        tr(self.language, "used"),
                        bytes(used),
                        bytes(total)
                    ))
                    .weak()
                    .size(12.0),
                );
            });
            ui.add_space(10.0);
        }
    }

    fn processes_page(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "{}: {}",
                    tr(self.language, "process_count"),
                    self.sys.processes().len()
                ))
                .strong(),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_sized(
                    [240.0, 30.0],
                    egui::TextEdit::singleline(&mut self.search)
                        .hint_text(tr(self.language, "search_processes")),
                );
            });
        });
        ui.add_space(10.0);
        let search = self.search.to_lowercase();
        let mut rows: Vec<_> = self
            .sys
            .processes()
            .values()
            .filter_map(|p| {
                let name = p.name().to_string_lossy().into_owned();
                (search.is_empty() || name.to_lowercase().contains(&search))
                    .then(|| (name, p.pid().to_string(), p.cpu_usage(), p.memory()))
            })
            .collect();
        rows.sort_by(|a, b| b.2.total_cmp(&a.2).then(b.3.cmp(&a.3)));
        card(ui, |ui| {
            egui::Grid::new("processes")
                .num_columns(4)
                .striped(true)
                .min_col_width(90.0)
                .spacing([24.0, 8.0])
                .show(ui, |ui| {
                    for h in [
                        tr(self.language, "process"),
                        "PID",
                        "CPU",
                        tr(self.language, "memory"),
                    ] {
                        ui.label(RichText::new(h).strong().weak().size(10.0));
                    }
                    ui.end_row();
                    for (name, pid, cpu, memory) in rows.iter().take(250) {
                        ui.label(name);
                        ui.label(RichText::new(pid).weak());
                        ui.label(format!("{cpu:.1}%"));
                        ui.label(bytes(*memory));
                        ui.end_row();
                    }
                });
        });
    }

    fn network_page(&self, ui: &mut egui::Ui) {
        let down = self.networks.iter().map(|(_, n)| n.received()).sum::<u64>();
        let up = self
            .networks
            .iter()
            .map(|(_, n)| n.transmitted())
            .sum::<u64>();
        ui.columns(2, |c| {
            info(&mut c[0], tr(self.language, "download"), &rate(down), GREEN);
            info(&mut c[1], tr(self.language, "upload"), &rate(up), BLUE);
        });
        ui.add_space(14.0);
        let max = self
            .down
            .iter()
            .chain(self.up.iter())
            .copied()
            .fold(1.0, f32::max);
        ui.columns(2, |c| {
            chart(
                &mut c[0],
                tr(self.language, "download_last_60"),
                &self.down,
                max,
                GREEN,
                false,
            );
            chart(
                &mut c[1],
                tr(self.language, "upload_last_60"),
                &self.up,
                max,
                BLUE,
                false,
            );
        });
        ui.add_space(14.0);
        card(ui, |ui| {
            egui::Grid::new("network")
                .num_columns(5)
                .striped(true)
                .spacing([22.0, 8.0])
                .show(ui, |ui| {
                    for h in ["INTERFACE", "DOWN / S", "UP / S", "TOTAL DOWN", "TOTAL UP"] {
                        ui.label(RichText::new(h).strong().weak().size(10.0));
                    }
                    ui.end_row();
                    for (name, n) in &self.networks {
                        ui.label(name);
                        ui.label(bytes(n.received()));
                        ui.label(bytes(n.transmitted()));
                        ui.label(bytes(n.total_received()));
                        ui.label(bytes(n.total_transmitted()));
                        ui.end_row();
                    }
                });
        });
    }

    fn settings_page(&mut self, ui: &mut egui::Ui) {
        let lang = self.language;
        card(ui, |ui| {
            ui.label(RichText::new(tr(lang, "language")).strong().size(17.0));
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                for language in Language::ALL {
                    ui.selectable_value(&mut self.language, language, language.native_name());
                }
            });
        });
        ui.add_space(12.0);
        let lang = self.language;
        card(ui, |ui| {
            ui.label(RichText::new(tr(lang, "popup_title")).strong().size(17.0));
            ui.label(RichText::new(tr(lang, "popup_description")).weak());
            ui.add_space(14.0);
            ui.checkbox(&mut self.popup, tr(lang, "popup_enable"));
        });
        ui.add_space(12.0);
        ui.add_enabled_ui(self.popup, |ui| {
            card(ui, |ui| {
                ui.label(RichText::new(tr(lang, "visible_items")).strong().size(15.0));
                ui.add_space(10.0);
                ui.columns(3, |c| {
                    c[0].checkbox(&mut self.popup_cpu, tr(lang, "cpu_usage"));
                    c[1].checkbox(&mut self.popup_gpu, tr(lang, "gpu_usage"));
                    c[2].checkbox(&mut self.popup_memory, tr(lang, "memory_usage"));
                });
                ui.columns(3, |c| {
                    c[0].checkbox(&mut self.popup_disk, tr(lang, "disk_usage"));
                    c[1].checkbox(&mut self.popup_processes, tr(lang, "process_count"));
                    c[2].checkbox(&mut self.popup_network, tr(lang, "network_speed"));
                });
                ui.checkbox(&mut self.popup_graphs, tr(lang, "popup_graphs"));
            });
            ui.add_space(12.0);
            card(ui, |ui| {
                ui.label(
                    RichText::new(tr(lang, "screen_position"))
                        .strong()
                        .size(15.0),
                );
                ui.add_space(10.0);
                egui::Grid::new("popup_positions")
                    .num_columns(3)
                    .spacing([18.0, 10.0])
                    .show(ui, |ui| {
                        for (index, position) in PopupPosition::ALL.into_iter().enumerate() {
                            ui.radio_value(
                                &mut self.popup_position,
                                position,
                                tr(lang, position.key()),
                            );
                            if index % 3 == 2 {
                                ui.end_row();
                            }
                        }
                    });
            });
            ui.add_space(12.0);
            card(ui, |ui| {
                ui.label(RichText::new(tr(lang, "opacity")).strong().size(15.0));
                ui.add(egui::Slider::new(&mut self.popup_opacity, 0.30..=1.0).show_value(true));
            });
        });
        ui.add_space(12.0);
        card(ui, |ui| {
            ui.label(
                RichText::new(tr(lang, "refresh_interval"))
                    .strong()
                    .size(15.0),
            );
            ui.add_space(6.0);
            ui.add(
                egui::Slider::new(&mut self.refresh_secs, 1..=10)
                    .suffix(tr(lang, "seconds_suffix")),
            );
        });
        ui.add_space(12.0);
        card(ui, |ui| {
            ui.label(RichText::new(tr(lang, "startup")).strong().size(15.0));
            let mut enabled = self.autostart;
            if ui
                .checkbox(&mut enabled, tr(lang, "startup_enable"))
                .changed()
            {
                match set_autostart(enabled) {
                    Ok(()) => {
                        self.autostart = enabled;
                        self.settings_message = None;
                    }
                    Err(error) => self.settings_message = Some(error),
                }
            }
            if let Some(message) = &self.settings_message {
                ui.label(
                    RichText::new(message)
                        .color(Color32::from_rgb(235, 95, 95))
                        .size(12.0),
                );
            }
        });
    }

    fn show_popup(&mut self, ctx: &egui::Context) {
        if self.popup_closed.swap(false, Ordering::Relaxed) {
            self.popup = false;
        }
        if !self.popup {
            return;
        }
        let monitor = ctx
            .input(|i| i.viewport().monitor_size)
            .unwrap_or(Vec2::new(1920.0, 1080.0));
        let width = 292.0;
        let shown = [
            self.popup_cpu,
            self.popup_gpu,
            self.popup_memory,
            self.popup_disk,
            self.popup_processes,
            self.popup_network,
        ];
        let count = shown.into_iter().filter(|shown| *shown).count().max(1);
        let temperatures = metric_temperatures(&self.components);
        let graph_count = [0, 1, 2, 5]
            .into_iter()
            .filter(|index| shown[*index])
            .count();
        let graph_rows = if self.popup_graphs {
            graph_count.div_ceil(2)
        } else {
            0
        };
        let height = 48.0 + count as f32 * 31.0 + graph_rows as f32 * 76.0;
        let margin = 18.0;
        let top = if cfg!(target_os = "macos") {
            42.0
        } else {
            margin
        };
        let bottom = monitor.y
            - height
            - if cfg!(target_os = "windows") {
                58.0
            } else {
                margin
            };
        let center = (monitor.x - width) / 2.0;
        let right = monitor.x - width - margin;
        let position = match self.popup_position {
            PopupPosition::TopLeft => [margin, top],
            PopupPosition::TopCenter => [center, top],
            PopupPosition::TopRight => [right, top],
            PopupPosition::BottomLeft => [margin, bottom],
            PopupPosition::BottomCenter => [center, bottom],
            PopupPosition::BottomRight => [right, bottom],
        };
        let disk_total: u64 = self.disks.iter().map(|d| d.total_space()).sum();
        let disk_used: u64 = self
            .disks
            .iter()
            .map(|d| d.total_space().saturating_sub(d.available_space()))
            .sum();
        let values = [
            self.sys.global_cpu_usage(),
            self.gpu.usage,
            pct(self.sys.used_memory(), self.sys.total_memory()),
            pct(disk_used, disk_total),
            self.sys.processes().len() as f32,
            0.0,
        ];
        let network_down = self.networks.iter().map(|(_, n)| n.received()).sum::<u64>();
        let network_up = self
            .networks
            .iter()
            .map(|(_, n)| n.transmitted())
            .sum::<u64>();
        let histories = [
            self.cpu.clone(),
            self.gpu_history.clone(),
            self.memory.clone(),
            self.disk_history.clone(),
            self.process_history.clone(),
            self.down.clone(),
        ];
        let dark = self.dark;
        let opacity = self.popup_opacity;
        let lang = self.language;
        let graphs = self.popup_graphs;
        let builder = egui::ViewportBuilder::default()
            .with_title("Resource Monitor Popup")
            .with_inner_size([width, height])
            .with_position(position)
            .with_resizable(false)
            .with_decorations(false)
            .with_always_on_top()
            .with_taskbar(false)
            .with_mouse_passthrough(true)
            .with_transparent(true);
        // 부모 창의 최소화/복원 중에도 팝업은 별도의 렌더링 콜백을 사용합니다.
        // 작은 읽기 전용 스냅샷을 전달하여 UI 사이에 잠금이나 중첩 렌더링이 없습니다.
        let popup_closed = Arc::clone(&self.popup_closed);
        let refresh_interval = Duration::from_secs(self.refresh_secs);
        ctx.show_viewport_deferred(
            egui::ViewportId::from_hash_of("monitor_popup"),
            builder,
            move |ui, _| {
                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::new()
                            .fill(if dark {
                                Color32::from_rgba_unmultiplied(25, 27, 32, (opacity * 255.0) as u8)
                            } else {
                                Color32::from_rgba_unmultiplied(
                                    248,
                                    249,
                                    251,
                                    (opacity * 255.0) as u8,
                                )
                            })
                            .corner_radius(if cfg!(target_os = "macos") { 12 } else { 4 })
                            .inner_margin(Margin::same(14)),
                    )
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Resource Monitor").strong().size(14.0));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(RichText::new("● LIVE").color(GREEN).size(9.0));
                            });
                        });
                        ui.separator();
                        let labels = [
                            "CPU",
                            "GPU",
                            tr(lang, "memory"),
                            tr(lang, "disk"),
                            tr(lang, "processes"),
                            tr(lang, "network"),
                        ];
                        let colors = [BLUE, GREEN, PURPLE, ORANGE, GREEN, GREEN];
                        for index in 0..6 {
                            if shown[index] {
                                let mut value = if index < 4 {
                                    format!("{:.1}%", values[index])
                                } else if index == 4 {
                                    format!("{:.0}", values[index])
                                } else {
                                    format!("↓ {}  ↑ {}", rate(network_down), rate(network_up))
                                };
                                if let Some(temperature) = temperature_text(temperatures[index]) {
                                    value.push_str(&format!("  •  {temperature}"));
                                }
                                popup_row(ui, labels[index], &value, colors[index]);
                            }
                        }
                        if graphs {
                            ui.add_space(6.0);
                            let indices: Vec<_> = [0, 1, 2, 5]
                                .into_iter()
                                .filter(|index| shown[*index])
                                .collect();
                            for pair in indices.chunks(2) {
                                ui.columns(2, |columns| {
                                    for (column, index) in pair.iter().enumerate() {
                                        mini_chart(
                                            &mut columns[column],
                                            labels[*index],
                                            &histories[*index],
                                            colors[*index],
                                        );
                                    }
                                });
                                ui.add_space(5.0);
                            }
                        }
                    });
                if ui.ctx().input(|i| i.viewport().close_requested()) {
                    popup_closed.store(true, Ordering::Relaxed);
                    ui.ctx().request_repaint_of(egui::ViewportId::ROOT);
                }
                ui.ctx().request_repaint_after(refresh_interval);
                ui.ctx()
                    .request_repaint_after_for(refresh_interval, egui::ViewportId::ROOT);
            },
        );
        // 값 또는 설정이 갱신되면 다음 주기까지 기다리지 않고 팝업에 반영합니다.
        ctx.request_repaint_of(egui::ViewportId::from_hash_of("monitor_popup"));
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        #[cfg(target_os = "macos")]
        if MAC_REOPEN_REQUESTED.swap(false, Ordering::AcqRel) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        self.poll_update_check(ctx);
        self.poll_update_download();
        ctx.request_repaint_after(Duration::from_secs(self.refresh_secs));
    }

    fn ui(&mut self, root: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        #[cfg(target_os = "macos")]
        if !self.exiting && ctx.input(|input| input.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        #[cfg(not(target_os = "macos"))]
        if self.popup && !self.exiting && ctx.input(|input| input.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
        if self.last.elapsed() >= Duration::from_secs(self.refresh_secs) {
            self.refresh();
        }
        #[cfg(target_os = "macos")]
        egui::Panel::top("mac_titlebar")
            .exact_size(28.0)
            .frame(
                egui::Frame::new()
                    .fill(if self.dark {
                        Color32::from_rgb(45, 44, 43)
                    } else {
                        Color32::from_rgb(235, 233, 231)
                    })
                    .inner_margin(Margin::ZERO),
            )
            .show(root, |_| {});
        self.sidebar(root);
        egui::Panel::top("top")
            .exact_size(67.0)
            .frame(
                egui::Frame::new()
                    .fill(background(self.dark))
                    .inner_margin(Margin::symmetric(24, 13)),
            )
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(tr(self.language, self.page.key()))
                                .strong()
                                .size(22.0),
                        );
                        ui.label(
                            RichText::new(tr(self.language, "live_performance"))
                                .weak()
                                .size(12.0),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .button(if self.dark {
                                tr(self.language, "light")
                            } else {
                                tr(self.language, "dark")
                            })
                            .clicked()
                        {
                            self.dark = !self.dark;
                            set_style(&ctx, self.dark);
                        }
                        ui.label(
                            RichText::new(refresh_label(self.language, self.refresh_secs))
                                .weak()
                                .size(12.0),
                        );
                    });
                });
            });
        if let Some(update) = self.available_update.clone() {
            egui::Panel::top("update_available")
                .exact_size(if self.update_download_error.is_some() {
                    64.0
                } else {
                    46.0
                })
                .frame(
                    egui::Frame::new()
                        .fill(if self.dark {
                            Color32::from_rgb(24, 50, 39)
                        } else {
                            Color32::from_rgb(224, 247, 235)
                        })
                        .inner_margin(Margin::symmetric(24, 8)),
                )
                .show(root, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "{} {}",
                                tr(self.language, "update_available"),
                                update.version
                            ))
                            .strong()
                            .color(GREEN),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button(tr(self.language, "later")).clicked() {
                                self.dismissed_update = Some(update.version.clone());
                                self.available_update = None;
                            }
                            if let Some(path) = self.downloaded_update.clone() {
                                if ui.button(tr(self.language, "install_update")).clicked()
                                    && let Err(error) = launch_update(&path)
                                {
                                    self.update_download_error = Some(error);
                                }
                            } else if self.update_download_rx.is_some() {
                                ui.add_enabled(
                                    false,
                                    egui::Button::new(tr(self.language, "downloading_update")),
                                );
                            } else if ui
                                .button(tr(self.language, "download_install_update"))
                                .clicked()
                            {
                                self.update_download_error = None;
                                self.update_download_rx =
                                    Some(start_update_download(update.clone(), ui.ctx().clone()));
                            }
                            if self.update_download_error.is_some()
                                && ui.button(tr(self.language, "update_retry")).clicked()
                            {
                                self.update_download_error = None;
                                self.update_download_rx =
                                    Some(start_update_download(update.clone(), ui.ctx().clone()));
                            }
                        });
                    });
                    if let Some(error) = &self.update_download_error {
                        ui.label(
                            RichText::new(error)
                                .color(Color32::from_rgb(235, 95, 95))
                                .size(10.0),
                        );
                    }
                });
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(background(self.dark))
                    .inner_margin(Margin::symmetric(24, 20)),
            )
            .show(root, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.page {
                        Page::Overview => self.overview(ui),
                        Page::Cpu => self.cpu_page(ui),
                        Page::Gpu => self.gpu_page(ui),
                        Page::Memory => self.memory_page(ui),
                        Page::Disks => self.disks_page(ui),
                        Page::Processes => self.processes_page(ui),
                        Page::Network => self.network_page(ui),
                        Page::Settings => self.settings_page(ui),
                    });
            });
        self.show_popup(&ctx);
        if self.confirm_exit {
            let mut answer = None;
            egui::Modal::new(egui::Id::new("exit_confirmation")).show(&ctx, |ui| {
                ui.heading(tr(self.language, "quit_title"));
                ui.label(tr(self.language, "quit_question"));
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button(tr(self.language, "no")).clicked() {
                        answer = Some(false);
                    }
                    if ui.button(tr(self.language, "yes")).clicked() {
                        answer = Some(true);
                    }
                });
            });
            match answer {
                Some(true) => {
                    self.exiting = true;
                    self.confirm_exit = false;
                    self.popup = false;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Some(false) => self.confirm_exit = false,
                None => {}
            }
        }
        self.save_settings_if_changed();
        ctx.request_repaint_after(
            Duration::from_secs(self.refresh_secs).saturating_sub(self.last.elapsed()),
        );
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        Color32::TRANSPARENT.to_normalized_gamma_f32()
    }
}

#[derive(Clone)]
struct PopupConfig {
    opacity: f32,
    position: PopupPosition,
    shown: [bool; 6],
    graphs: bool,
    language: Language,
    dark: bool,
    refresh_secs: u64,
}

impl PopupConfig {
    fn parse(value: &str) -> Option<Self> {
        let mut parts = value.trim().split(':');
        let opacity = parts.next()?.parse().ok()?;
        let position = PopupPosition::from_code(parts.next()?.parse().ok()?);
        let flags = parts.next()?;
        let mut shown = [false; 6];
        for (slot, value) in shown.iter_mut().zip(flags.bytes()) {
            *slot = value == b'1';
        }
        Some(Self {
            opacity,
            position,
            shown,
            graphs: flags.as_bytes().get(6) == Some(&b'1'),
            language: Language::from_code(parts.next()?.parse().ok()?),
            dark: parts.next()?.parse::<u8>().ok()? != 0,
            refresh_secs: parts
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or(2)
                .clamp(1, 10),
        })
    }

    fn window_height(&self, has_temperature: bool) -> f32 {
        let count = self.shown.into_iter().filter(|shown| *shown).count().max(1);
        let graph_count = [0, 1, 2, 5]
            .into_iter()
            .filter(|index| self.shown[*index])
            .count();
        let graph_rows = if self.graphs {
            graph_count.div_ceil(2)
        } else {
            0
        };
        48.0 + (count + usize::from(has_temperature)) as f32 * 31.0 + graph_rows as f32 * 76.0
    }
}

struct PopupApp {
    config: PopupConfig,
    sys: System,
    components: Components,
    gpu: GpuInfo,
    disks: Disks,
    networks: Networks,
    last: Instant,
    last_temperature_refresh: Instant,
    histories: [VecDeque<f32>; 6],
    config_text: String,
    last_config_check: Instant,
    layout_dirty: bool,
}

impl PopupApp {
    fn new(cc: &eframe::CreationContext<'_>, config: PopupConfig) -> Self {
        configure_fonts(&cc.egui_ctx);
        set_style(&cc.egui_ctx, config.dark);
        let mut app = Self {
            config,
            sys: System::new_all(),
            components: Components::new_with_refreshed_list(),
            gpu: GpuInfo::new(),
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            last: Instant::now() - Duration::from_secs(2),
            last_temperature_refresh: Instant::now() - Duration::from_secs(10),
            histories: std::array::from_fn(|_| VecDeque::new()),
            config_text: String::new(),
            last_config_check: Instant::now() - Duration::from_secs(1),
            layout_dirty: true,
        };
        app.refresh();
        app
    }
    fn values(&self) -> [f32; 6] {
        let total: u64 = self.disks.iter().map(|d| d.total_space()).sum();
        let used: u64 = self
            .disks
            .iter()
            .map(|d| d.total_space().saturating_sub(d.available_space()))
            .sum();
        [
            self.sys.global_cpu_usage(),
            self.gpu.usage,
            pct(self.sys.used_memory(), self.sys.total_memory()),
            pct(used, total),
            self.sys.processes().len() as f32,
            self.networks
                .iter()
                .map(|(_, n)| n.received() + n.transmitted())
                .sum::<u64>() as f32,
        ]
    }
    fn refresh(&mut self) {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        if self.last_temperature_refresh.elapsed() >= Duration::from_secs(10) {
            self.components.refresh(true);
            self.last_temperature_refresh = Instant::now();
            self.layout_dirty = true;
        }
        self.sys.refresh_processes(ProcessesToUpdate::All, true);
        self.gpu.refresh();
        self.disks.refresh(true);
        self.networks.refresh(true);
        let values = self.values();
        for (history, value) in self.histories.iter_mut().zip(values) {
            push(history, value);
        }
        self.last = Instant::now();
    }
}

impl eframe::App for PopupApp {
    fn ui(&mut self, root: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = root.ctx().clone();
        if shutdown_file().exists() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if self.last_config_check.elapsed() >= Duration::from_millis(100) {
            self.last_config_check = Instant::now();
            if let Ok(value) = std::fs::read_to_string(popup_config_file())
                && value != self.config_text
                && let Some(config) = PopupConfig::parse(&value)
            {
                if config.dark != self.config.dark {
                    set_style(&ctx, config.dark);
                }
                self.config = config;
                self.config_text = value;
                self.layout_dirty = true;
            }
        }
        if self.last.elapsed() >= Duration::from_secs(self.config.refresh_secs) {
            self.refresh();
        }
        let monitor = ctx
            .input(|i| i.viewport().monitor_size)
            .unwrap_or(Vec2::new(1920.0, 1080.0));
        let temperatures = metric_temperatures(&self.components);
        let size = Vec2::new(292.0, self.config.window_height(false));
        let margin = 18.0;
        let center = (monitor.x - size.x) / 2.0;
        let right = monitor.x - size.x - margin;
        let top = if cfg!(target_os = "macos") {
            42.0
        } else {
            margin
        };
        let bottom = monitor.y
            - size.y
            - if cfg!(target_os = "windows") {
                58.0
            } else {
                margin
            };
        let pos = match self.config.position {
            PopupPosition::TopLeft => [margin, top],
            PopupPosition::TopCenter => [center, top],
            PopupPosition::TopRight => [right, top],
            PopupPosition::BottomLeft => [margin, bottom],
            PopupPosition::BottomCenter => [center, bottom],
            PopupPosition::BottomRight => [right, bottom],
        };
        if self.layout_dirty {
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(Vec2::new(
                292.0,
                self.config.window_height(false),
            )));
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos.into()));
            self.layout_dirty = false;
        }
        let values = self.values();
        let network_down = self.networks.iter().map(|(_, n)| n.received()).sum::<u64>();
        let network_up = self
            .networks
            .iter()
            .map(|(_, n)| n.transmitted())
            .sum::<u64>();
        let alpha = (self.config.opacity * 255.0) as u8;
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(if self.config.dark {
                        Color32::from_rgba_unmultiplied(25, 27, 32, alpha)
                    } else {
                        Color32::from_rgba_unmultiplied(248, 249, 251, alpha)
                    })
                    .corner_radius(if cfg!(target_os = "macos") { 12 } else { 4 })
                    .inner_margin(Margin::same(14)),
            )
            .show(root, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Resource Monitor").strong().size(14.0));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new("● LIVE").color(GREEN).size(9.0));
                    });
                });
                ui.separator();
                let labels = [
                    "CPU",
                    "GPU",
                    tr(self.config.language, "memory"),
                    tr(self.config.language, "disk"),
                    tr(self.config.language, "processes"),
                    tr(self.config.language, "network"),
                ];
                let colors = [BLUE, GREEN, PURPLE, ORANGE, GREEN, GREEN];
                for i in 0..6 {
                    if self.config.shown[i] {
                        let mut value = if i < 4 {
                            format!("{:.1}%", values[i])
                        } else if i == 4 {
                            format!("{:.0}", values[i])
                        } else {
                            format!("↓ {}  ↑ {}", rate(network_down), rate(network_up))
                        };
                        if let Some(temperature) = temperature_text(temperatures[i]) {
                            value.push_str(&format!("  •  {temperature}"));
                        }
                        popup_row(ui, labels[i], &value, colors[i]);
                    }
                }
                if self.config.graphs {
                    ui.add_space(6.0);
                    let indices: Vec<_> = [0, 1, 2, 5]
                        .into_iter()
                        .filter(|index| self.config.shown[*index])
                        .collect();
                    for pair in indices.chunks(2) {
                        ui.columns(2, |columns| {
                            for (column, index) in pair.iter().enumerate() {
                                mini_chart(
                                    &mut columns[column],
                                    labels[*index],
                                    &self.histories[*index],
                                    colors[*index],
                                );
                            }
                        });
                        ui.add_space(5.0);
                    }
                }
            });
        ctx.request_repaint_after(Duration::from_millis(100));
    }
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        Color32::TRANSPARENT.to_normalized_gamma_f32()
    }
}

fn mini_chart(ui: &mut egui::Ui, label: &str, values: &VecDeque<f32>, color: Color32) {
    ui.label(RichText::new(label).weak().size(9.0));
    let (r, p) = ui.allocate_painter(Vec2::new(ui.available_width(), 46.0), Sense::hover());
    if values.len() > 1 {
        let points = values
            .iter()
            .enumerate()
            .map(|(i, v)| {
                egui::pos2(
                    egui::lerp(
                        r.rect.left()..=r.rect.right(),
                        i as f32 / (HISTORY - 1) as f32,
                    ),
                    egui::lerp(r.rect.bottom()..=r.rect.top(), (*v / 100.0).clamp(0.0, 1.0)),
                )
            })
            .collect();
        p.add(egui::Shape::line(points, Stroke::new(1.5, color)));
    }
}

fn shutdown_file() -> std::path::PathBuf {
    std::env::temp_dir().join("resource_monitor.shutdown")
}

fn popup_config_file() -> std::path::PathBuf {
    std::env::temp_dir().join("resource_monitor.popup.conf")
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.trim() {
        "1" | "true" => Some(true),
        "0" | "false" => Some(false),
        _ => None,
    }
}

fn settings_file() -> std::path::PathBuf {
    #[cfg(target_os = "macos")]
    {
        return std::env::var_os("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("Library/Application Support/Resource Monitor/settings.conf");
    }
    #[cfg(target_os = "windows")]
    {
        return std::env::var_os("APPDATA")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("Resource Monitor/settings.conf");
    }
    #[allow(unreachable_code)]
    std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
                .join(".config")
        })
        .join("resource-monitor/settings.conf")
}

fn save_settings(value: &str) -> Result<(), String> {
    let path = settings_file();
    let parent = path
        .parent()
        .ok_or_else(|| "설정 파일 경로를 만들 수 없습니다.".to_owned())?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    std::fs::write(path, value).map_err(|error| error.to_string())
}

fn start_update_check(ctx: egui::Context) -> Receiver<Option<UpdateInfo>> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let update = fetch_latest_update().ok().flatten();
        let _ = sender.send(update);
        ctx.request_repaint();
    });
    receiver
}

fn fetch_latest_update() -> Result<Option<UpdateInfo>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(8)))
        .build()
        .into();
    let mut response = agent
        .get(RELEASE_API)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header(
            "User-Agent",
            concat!("ResourceMonitor/", env!("CARGO_PKG_VERSION")),
        )
        .call()
        .map_err(|error| error.to_string())?;
    let value: serde_json::Value = response
        .body_mut()
        .read_json()
        .map_err(|error| error.to_string())?;
    Ok(latest_update_from_json(&value))
}

fn latest_update_from_json(value: &serde_json::Value) -> Option<UpdateInfo> {
    let tag = value.get("tag_name")?.as_str()?;
    let version_text = tag.strip_prefix('v').unwrap_or(tag);
    let latest = semver::Version::parse(version_text).ok()?;
    let current = semver::Version::parse(env!("CARGO_PKG_VERSION")).ok()?;
    if latest <= current {
        return None;
    }
    let url = value.get("html_url")?.as_str()?;
    if !url.starts_with("https://github.com/Mobil0010/resource_monitor/releases/tag/") {
        return None;
    }
    let suffix = if cfg!(target_os = "macos") {
        "-macOS-Universal.dmg"
    } else if cfg!(target_os = "windows") {
        "-Windows-Setup.exe"
    } else {
        return None;
    };
    let asset = value.get("assets")?.as_array()?.iter().find(|asset| {
        asset
            .get("name")
            .and_then(|name| name.as_str())
            .is_some_and(|name| name.starts_with("ResourceMonitor-") && name.ends_with(suffix))
    })?;
    let asset_name = asset.get("name")?.as_str()?;
    let asset_url = asset.get("browser_download_url")?.as_str()?;
    if !asset_url.starts_with("https://github.com/Mobil0010/resource_monitor/releases/download/")
        || asset_name.contains(['/', '\\'])
    {
        return None;
    }
    Some(UpdateInfo {
        version: format!("v{latest}"),
        asset_url: asset_url.to_owned(),
        asset_name: asset_name.to_owned(),
    })
}

fn start_update_download(
    update: UpdateInfo,
    ctx: egui::Context,
) -> Receiver<Result<std::path::PathBuf, String>> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = download_update(&update);
        let _ = sender.send(result);
        ctx.request_repaint();
    });
    receiver
}

fn download_update(update: &UpdateInfo) -> Result<std::path::PathBuf, String> {
    let directory = std::env::temp_dir().join("ResourceMonitorUpdate");
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let path = directory.join(&update.asset_name);
    let partial = directory.join(format!("{}.part", update.asset_name));
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(300)))
        .build()
        .into();
    let mut response = agent
        .get(&update.asset_url)
        .header(
            "User-Agent",
            concat!("ResourceMonitor/", env!("CARGO_PKG_VERSION")),
        )
        .call()
        .map_err(|error| error.to_string())?;
    let mut file = std::fs::File::create(&partial).map_err(|error| error.to_string())?;
    std::io::copy(&mut response.body_mut().as_reader(), &mut file)
        .map_err(|error| error.to_string())?;
    std::fs::rename(&partial, &path).map_err(|error| error.to_string())?;
    Ok(path)
}

fn launch_update(path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(path).spawn();
    #[cfg(target_os = "windows")]
    let result = {
        use std::os::windows::process::CommandExt;
        Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg("")
            .arg(path)
            .creation_flags(0x08000000)
            .spawn()
    };
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let result: std::io::Result<std::process::Child> = Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "unsupported platform",
    ));
    result.map(|_| ()).map_err(|error| error.to_string())
}

#[cfg(test)]
mod settings_tests {
    use super::*;

    #[test]
    fn settings_round_trip() {
        let expected = SavedSettings {
            dark: false,
            popup: true,
            popup_position: PopupPosition::BottomCenter,
            popup_cpu: false,
            popup_gpu: true,
            popup_memory: false,
            popup_disk: true,
            popup_processes: true,
            popup_network: false,
            popup_opacity: 0.617,
            language: Language::Japanese,
            popup_graphs: false,
            refresh_secs: 7,
        };
        let encoded = expected.encode();
        let mut actual = SavedSettings::defaults(true);
        actual.apply(&encoded);
        assert_eq!(actual.encode(), encoded);
    }

    #[test]
    fn invalid_values_are_ignored_or_clamped() {
        let mut settings = SavedSettings::defaults(true);
        settings.apply("dark=invalid\npopup_opacity=9\nrefresh_secs=0\nlanguage=99\n");
        assert!(settings.dark);
        assert_eq!(settings.popup_opacity, 1.0);
        assert_eq!(settings.refresh_secs, 1);
        assert_eq!(settings.language, Language::English);
    }

    #[test]
    fn newer_github_release_is_detected() {
        let suffix = if cfg!(target_os = "macos") {
            "-macOS-Universal.dmg"
        } else {
            "-Windows-Setup.exe"
        };
        let asset_name = format!("ResourceMonitor-99.2.1{suffix}");
        let value = serde_json::json!({
            "tag_name": "v99.2.1",
            "html_url": "https://github.com/Mobil0010/resource_monitor/releases/tag/v99.2.1",
            "assets": [{
                "name": asset_name,
                "browser_download_url": format!("https://github.com/Mobil0010/resource_monitor/releases/download/v99.2.1/{asset_name}")
            }]
        });
        assert_eq!(
            latest_update_from_json(&value),
            Some(UpdateInfo {
                version: "v99.2.1".into(),
                asset_url: format!(
                    "https://github.com/Mobil0010/resource_monitor/releases/download/v99.2.1/{asset_name}"
                ),
                asset_name,
            })
        );
    }

    #[test]
    fn old_invalid_or_untrusted_releases_are_ignored() {
        for value in [
            serde_json::json!({
                "tag_name": env!("CARGO_PKG_VERSION"),
                "html_url": "https://github.com/Mobil0010/resource_monitor/releases/tag/current"
            }),
            serde_json::json!({
                "tag_name": "not-a-version",
                "html_url": "https://github.com/Mobil0010/resource_monitor/releases/tag/test"
            }),
            serde_json::json!({
                "tag_name": "v99.0.0",
                "html_url": "https://example.com/download"
            }),
        ] {
            assert_eq!(latest_update_from_json(&value), None);
        }
    }
}

fn autostart_enabled() -> bool {
    #[cfg(target_os = "macos")]
    {
        return launch_agent_path().is_file();
    }
    #[cfg(target_os = "windows")]
    {
        return Command::new("reg")
            .args([
                "query",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                "ResourceMonitor",
            ])
            .output()
            .is_ok_and(|o| o.status.success());
    }
    #[allow(unreachable_code)]
    false
}
#[cfg(target_os = "macos")]
fn launch_agent_path() -> std::path::PathBuf {
    std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_default()
        .join("Library/LaunchAgents/com.resource-monitor.plist")
}
fn set_autostart(enabled: bool) -> Result<(), String> {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let _ = enabled;
    #[cfg(target_os = "macos")]
    {
        let path = launch_agent_path();
        if enabled {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let xml = format!(
                r#"<?xml version="1.0" encoding="UTF-8"?><!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd"><plist version="1.0"><dict><key>Label</key><string>com.resource-monitor</string><key>ProgramArguments</key><array><string>{}</string></array><key>RunAtLoad</key><true/></dict></plist>"#,
                exe.display()
            );
            std::fs::write(path, xml).map_err(|e| e.to_string())?;
        } else if path.exists() {
            std::fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        return Ok(());
    }
    #[cfg(target_os = "windows")]
    {
        let key = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
        let status = if enabled {
            Command::new("reg")
                .args([
                    "add",
                    key,
                    "/v",
                    "ResourceMonitor",
                    "/t",
                    "REG_SZ",
                    "/d",
                    &exe.display().to_string(),
                    "/f",
                ])
                .status()
        } else {
            Command::new("reg")
                .args(["delete", key, "/v", "ResourceMonitor", "/f"])
                .status()
        }
        .map_err(|e| e.to_string())?;
        return status
            .success()
            .then_some(())
            .ok_or("시작 프로그램 설정에 실패했습니다.".into());
    }
    #[allow(unreachable_code)]
    Err("이 운영체제에서는 자동 실행을 지원하지 않습니다.".into())
}

fn set_style(ctx: &egui::Context, dark: bool) {
    ctx.set_theme(if dark {
        egui::ThemePreference::Dark
    } else {
        egui::ThemePreference::Light
    });
    let mut v = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    v.panel_fill = background(dark);
    v.window_fill = background(dark);
    v.extreme_bg_color = if dark {
        Color32::from_rgb(19, 21, 25)
    } else {
        Color32::from_rgb(235, 237, 242)
    };
    v.selection.bg_fill = Color32::from_rgb(67, 105, 182);
    ctx.set_visuals(v);
}

fn configure_fonts(ctx: &egui::Context) {
    let candidates: &[&str] = if cfg!(target_os = "macos") {
        &[
            "/System/Library/Fonts/Supplemental/AppleGothic.ttf",
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
            "/System/Library/Fonts/AppleSDGothicNeo.ttc",
        ]
    } else if cfg!(target_os = "windows") {
        &[
            "C:\\Windows\\Fonts\\malgun.ttf",
            "C:\\Windows\\Fonts\\meiryo.ttc",
            "C:\\Windows\\Fonts\\gulim.ttc",
        ]
    } else {
        &[
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        ]
    };

    let mut fonts = egui::FontDefinitions::default();
    let mut added = false;
    for (index, data) in candidates
        .iter()
        .filter_map(|path| std::fs::read(path).ok())
        .enumerate()
    {
        let name = format!("system-cjk-{index}");
        fonts
            .font_data
            .insert(name.clone(), egui::FontData::from_owned(data).into());
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            fonts.families.entry(family).or_default().push(name.clone());
        }
        added = true;
    }
    if added {
        ctx.set_fonts(fonts);
    }
}

fn background(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(27, 29, 35)
    } else {
        Color32::from_rgb(248, 249, 251)
    }
}

fn navigation_button(
    ui: &mut egui::Ui,
    page: Page,
    selected: bool,
    lang: Language,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 38.0), Sense::click());
    let visuals = ui.visuals();
    let background = if selected {
        Color32::from_rgb(67, 126, 202)
    } else if response.hovered() {
        visuals.widgets.hovered.bg_fill
    } else {
        visuals.widgets.inactive.bg_fill
    };
    let radius = if cfg!(target_os = "macos") { 4.0 } else { 2.0 };
    ui.painter().rect_filled(rect, radius, background);

    let color = if selected {
        Color32::WHITE
    } else {
        visuals.text_color()
    };
    let center = egui::pos2(rect.left() + 16.0, rect.center().y);
    paint_navigation_icon(ui.painter(), page, center, color);
    ui.painter().text(
        egui::pos2(rect.left() + 40.0, rect.center().y),
        Align2::LEFT_CENTER,
        tr(lang, page.key()),
        FontId::proportional(14.0),
        color,
    );
    response
}

fn paint_navigation_icon(p: &egui::Painter, page: Page, c: egui::Pos2, color: Color32) {
    let stroke = Stroke::new(1.6, color);
    match page {
        Page::Overview => {
            for (x, y) in [(-4.0, -4.0), (2.0, -4.0), (-4.0, 2.0), (2.0, 2.0)] {
                p.rect_stroke(
                    egui::Rect::from_min_size(c + egui::vec2(x, y), Vec2::splat(4.0)),
                    0.8,
                    stroke,
                    StrokeKind::Inside,
                );
            }
        }
        Page::Cpu => {
            p.circle_stroke(c, 4.5, stroke);
            p.circle_filled(c, 1.5, color);
            for d in [
                egui::vec2(0.0, -8.0),
                egui::vec2(0.0, 8.0),
                egui::vec2(-8.0, 0.0),
                egui::vec2(8.0, 0.0),
            ] {
                p.line_segment([c + d * 0.72, c + d], stroke);
            }
        }
        Page::Gpu => {
            p.rect_stroke(
                egui::Rect::from_center_size(c, Vec2::new(14.0, 10.0)),
                2.0,
                stroke,
                StrokeKind::Inside,
            );
            p.circle_stroke(c, 3.0, stroke);
            p.circle_filled(c, 1.0, color);
        }
        Page::Memory => {
            p.rect_stroke(
                egui::Rect::from_center_size(c, Vec2::new(11.0, 9.0)),
                1.5,
                stroke,
                StrokeKind::Inside,
            );
            for x in [-4.0, 0.0, 4.0] {
                p.line_segment([c + egui::vec2(x, -7.0), c + egui::vec2(x, -4.5)], stroke);
                p.line_segment([c + egui::vec2(x, 4.5), c + egui::vec2(x, 7.0)], stroke);
            }
        }
        Page::Disks => {
            p.circle_stroke(c, 7.0, stroke);
            p.circle_filled(c, 2.0, color);
            p.line_segment([c + egui::vec2(3.5, 3.5), c + egui::vec2(6.0, 6.0)], stroke);
        }
        Page::Processes => {
            for y in [-5.0, 0.0, 5.0] {
                p.circle_filled(c + egui::vec2(-6.0, y), 1.3, color);
                p.line_segment([c + egui::vec2(-2.5, y), c + egui::vec2(7.0, y)], stroke);
            }
        }
        Page::Network => {
            p.line_segment(
                [c + egui::vec2(-3.0, 7.0), c + egui::vec2(-3.0, -7.0)],
                stroke,
            );
            p.line_segment(
                [c + egui::vec2(-6.0, -4.0), c + egui::vec2(-3.0, -7.0)],
                stroke,
            );
            p.line_segment(
                [c + egui::vec2(3.0, -7.0), c + egui::vec2(3.0, 7.0)],
                stroke,
            );
            p.line_segment([c + egui::vec2(3.0, 7.0), c + egui::vec2(6.0, 4.0)], stroke);
        }
        Page::Settings => {
            p.circle_stroke(c, 5.0, stroke);
            p.circle_filled(c, 1.6, color);
            for d in [
                egui::vec2(0.0, -8.0),
                egui::vec2(0.0, 8.0),
                egui::vec2(-8.0, 0.0),
                egui::vec2(8.0, 0.0),
            ] {
                p.line_segment([c + d * 0.62, c + d], Stroke::new(2.4, color));
            }
        }
    }
}

fn card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    let dark = ui.visuals().dark_mode;
    egui::Frame::new()
        .fill(if dark {
            Color32::from_rgb(35, 38, 45)
        } else {
            Color32::WHITE
        })
        .stroke(Stroke::new(
            1.0,
            if dark {
                Color32::from_rgb(53, 56, 65)
            } else {
                Color32::from_rgb(224, 226, 232)
            },
        ))
        .corner_radius(if cfg!(target_os = "macos") { 12 } else { 4 })
        .inner_margin(Margin::same(18))
        .show(ui, add);
}
fn metric(ui: &mut egui::Ui, title: &str, value: f32, color: Color32) {
    card(ui, |ui| {
        ui.label(RichText::new(title).weak().size(11.0));
        ui.label(
            RichText::new(format!("{value:.1}%"))
                .strong()
                .size(25.0)
                .color(color),
        );
        ui.add_space(7.0);
        bar(ui, value, color);
    });
}
fn info(ui: &mut egui::Ui, title: &str, value: &str, color: Color32) {
    card(ui, |ui| {
        ui.label(RichText::new(title).weak().size(11.0));
        ui.add_space(6.0);
        ui.label(RichText::new(value).strong().size(18.0).color(color));
        ui.add_space(10.0);
    });
}
fn hero(ui: &mut egui::Ui, title: &str, subtitle: &str, value: &str, color: Color32) {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(title).strong().size(20.0));
                ui.label(RichText::new(subtitle).weak());
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new(value).strong().size(29.0).color(color));
            });
        });
    });
}
fn bar(ui: &mut egui::Ui, value: f32, color: Color32) {
    let (r, p) = ui.allocate_painter(
        Vec2::new(ui.available_width().max(80.0), 7.0),
        Sense::hover(),
    );
    p.rect_filled(r.rect, 4.0, ui.visuals().faint_bg_color);
    let fill = egui::Rect::from_min_size(
        r.rect.min,
        Vec2::new(
            r.rect.width() * (value / 100.0).clamp(0.0, 1.0),
            r.rect.height(),
        ),
    );
    p.rect_filled(fill, 4.0, color);
}
fn chart(
    ui: &mut egui::Ui,
    title: &str,
    values: &VecDeque<f32>,
    max: f32,
    color: Color32,
    percent: bool,
) {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(title).strong().size(15.0));
            if let Some(v) = values.back() {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(if percent {
                            format!("{v:.1}%")
                        } else {
                            rate(*v as u64)
                        })
                        .color(color)
                        .strong(),
                    );
                });
            }
        });
        ui.add_space(10.0);
        let (r, p) = ui.allocate_painter(Vec2::new(ui.available_width(), 145.0), Sense::hover());
        let rect = r.rect;
        let grid = if ui.visuals().dark_mode {
            Color32::from_gray(55)
        } else {
            Color32::from_gray(225)
        };
        for row in 0..=4 {
            let y = egui::lerp(rect.top()..=rect.bottom(), row as f32 / 4.0);
            p.line_segment(
                [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
                Stroke::new(1.0, grid),
            );
        }
        if values.len() > 1 {
            let points = values
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    egui::pos2(
                        egui::lerp(rect.left()..=rect.right(), i as f32 / (HISTORY - 1) as f32),
                        egui::lerp(
                            rect.bottom()..=rect.top(),
                            (*v / max.max(1.0)).clamp(0.0, 1.0),
                        ),
                    )
                })
                .collect();
            p.add(egui::Shape::line(points, Stroke::new(2.2, color)));
        }
    });
}
fn pair(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(RichText::new(label).weak().size(12.0));
    ui.label(RichText::new(value).strong());
}
fn popup_row(ui: &mut egui::Ui, label: &str, value: &str, color: Color32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).weak());
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(value).strong().color(color));
        });
    });
}
fn push(q: &mut VecDeque<f32>, v: f32) {
    if q.len() == HISTORY {
        q.pop_front();
    }
    q.push_back(v);
}
fn pct(a: u64, b: u64) -> f32 {
    if b == 0 {
        0.0
    } else {
        a as f32 / b as f32 * 100.0
    }
}
fn bytes(n: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < 4 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{v:.0} {}", units[i])
    } else {
        format!("{v:.1} {}", units[i])
    }
}
fn rate(n: u64) -> String {
    format!("{}/s", bytes(n))
}
fn temperature_for(components: &Components, category: &str) -> Option<f32> {
    let aliases: &[&str] = match category {
        "cpu" => &["cpu", "peci", "processor", "soc", "computer"],
        "gpu" => &["gpu", "graphics"],
        "memory" => &["memory", "ram", "dimm", "dram"],
        "disk" => &["disk", "ssd", "nvme", "storage", "drive"],
        _ => &[],
    };
    components
        .iter()
        .filter(|component| {
            let label = component.label().to_ascii_lowercase();
            aliases.iter().any(|alias| label.contains(alias))
        })
        .filter_map(|component| component.temperature())
        .filter(|value| value.is_finite() && (-20.0..=150.0).contains(value))
        .max_by(f32::total_cmp)
}
fn temperature_text(value: Option<f32>) -> Option<String> {
    value.map(|temperature| format!("{temperature:.0} °C"))
}
fn metric_temperatures(components: &Components) -> [Option<f32>; 6] {
    [
        temperature_for(components, "cpu"),
        temperature_for(components, "gpu"),
        temperature_for(components, "memory"),
        temperature_for(components, "disk"),
        None,
        None,
    ]
}
fn sensor_summary(components: &Components) -> String {
    let values: Vec<_> = components
        .iter()
        .filter_map(|component| {
            let temperature = component.temperature()?;
            (temperature.is_finite() && (-20.0..=150.0).contains(&temperature))
                .then(|| format!("{}: {temperature:.0} °C", component.label()))
        })
        .take(6)
        .collect();
    if values.is_empty() {
        "—".to_owned()
    } else {
        values.join("  •  ")
    }
}
fn uptime(s: u64) -> String {
    let d = s / 86400;
    let h = s % 86400 / 3600;
    let m = s % 3600 / 60;
    if d > 0 {
        format!("{d}d {h}h {m}m")
    } else {
        format!("{h}h {m}m")
    }
}
fn platform() -> &'static str {
    if cfg!(target_os = "macos") {
        "macOS"
    } else if cfg!(target_os = "windows") {
        "Windows"
    } else {
        "Desktop"
    }
}

fn tr<'a>(lang: Language, key: &'a str) -> &'a str {
    match (lang, key) {
        (Language::Korean, "overview") => "개요",
        (Language::Japanese, "overview") => "概要",
        (Language::Korean, "memory") => "메모리",
        (Language::Japanese, "memory") => "メモリ",
        (Language::Korean, "disks") => "디스크",
        (Language::Japanese, "disks") => "ディスク",
        (Language::Korean, "disk") => "디스크",
        (Language::Japanese, "disk") => "ディスク",
        (Language::Korean, "processes") => "프로세스",
        (Language::Japanese, "processes") => "プロセス",
        (Language::Korean, "network") => "네트워크",
        (Language::Japanese, "network") => "ネットワーク",
        (Language::Korean, "settings") => "설정",
        (Language::Japanese, "settings") => "設定",
        (Language::Korean, "language") => "언어",
        (Language::Japanese, "language") => "言語",
        (Language::Korean, "popup_title") => "팝업 설정",
        (Language::Japanese, "popup_title") => "ポップアップ設定",
        (Language::Korean, "popup_description") => {
            "선택한 시스템 정보를 작은 창으로 계속 표시합니다."
        }
        (Language::Japanese, "popup_description") => {
            "選択したシステム情報を小さなウィンドウに表示します。"
        }
        (Language::Korean, "popup_enable") => "팝업 사용",
        (Language::Japanese, "popup_enable") => "ポップアップを使用",
        (Language::Korean, "visible_items") => "표시할 정보",
        (Language::Japanese, "visible_items") => "表示する情報",
        (Language::Korean, "cpu_usage") => "CPU 사용률",
        (Language::Japanese, "cpu_usage") => "CPU 使用率",
        (Language::Korean, "memory_usage") => "메모리 사용률",
        (Language::Japanese, "memory_usage") => "メモリ使用率",
        (Language::Korean, "disk_usage") => "디스크 사용률",
        (Language::Japanese, "disk_usage") => "ディスク使用率",
        (Language::Korean, "process_count") => "프로세스 수",
        (Language::Japanese, "process_count") => "プロセス数",
        (Language::Korean, "network_speed") => "네트워크 속도",
        (Language::Japanese, "network_speed") => "ネットワーク速度",
        (Language::Korean, "screen_position") => "화면 위치",
        (Language::Japanese, "screen_position") => "画面位置",
        (Language::Korean, "opacity") => "팝업 투명도",
        (Language::Japanese, "opacity") => "ポップアップの透明度",
        (Language::Korean, "top_left") => "왼쪽 상단",
        (Language::Japanese, "top_left") => "左上",
        (Language::Korean, "top_center") => "상단 가운데",
        (Language::Japanese, "top_center") => "上中央",
        (Language::Korean, "top_right") => "오른쪽 상단",
        (Language::Japanese, "top_right") => "右上",
        (Language::Korean, "bottom_left") => "왼쪽 하단",
        (Language::Japanese, "bottom_left") => "左下",
        (Language::Korean, "bottom_center") => "하단 가운데",
        (Language::Japanese, "bottom_center") => "下中央",
        (Language::Korean, "bottom_right") => "오른쪽 하단",
        (Language::Japanese, "bottom_right") => "右下",
        (Language::Korean, "live_performance") => "실시간 시스템 성능",
        (Language::Japanese, "live_performance") => "リアルタイムシステム性能",
        (Language::Korean, "light") => "라이트",
        (Language::Japanese, "light") => "ライト",
        (Language::Korean, "dark") => "다크",
        (Language::Japanese, "dark") => "ダーク",
        (Language::Korean, "refresh_second") => "1초마다 새로고침",
        (Language::Japanese, "refresh_second") => "1秒ごとに更新",
        (Language::Korean, "cpu_history") => "CPU 사용 기록",
        (Language::Japanese, "cpu_history") => "CPU 使用履歴",
        (Language::Korean, "memory_history") => "메모리 사용 기록",
        (Language::Japanese, "memory_history") => "メモリ使用履歴",
        (Language::Korean, "system_info") => "시스템 정보",
        (Language::Japanese, "system_info") => "システム情報",
        (Language::Korean, "operating_system") => "운영체제",
        (Language::Japanese, "operating_system") => "オペレーティングシステム",
        (Language::Korean, "host_name") => "호스트 이름",
        (Language::Japanese, "host_name") => "ホスト名",
        (Language::Korean, "kernel") => "커널",
        (Language::Japanese, "kernel") => "カーネル",
        (Language::Korean, "uptime") => "가동 시간",
        (Language::Japanese, "uptime") => "稼働時間",
        (Language::Korean, "processor") => "프로세서",
        (Language::Japanese, "processor") => "プロセッサ",
        (Language::Korean, "cpu_last_60") => "CPU 사용률 — 최근 60초",
        (Language::Japanese, "cpu_last_60") => "CPU 使用率 — 過去60秒",
        (Language::Korean, "logical_processors") => "논리 프로세서",
        (Language::Japanese, "logical_processors") => "論理プロセッサ",
        (Language::Korean, "core") => "코어",
        (Language::Japanese, "core") => "コア",
        (Language::Korean, "physical_memory") => "물리 메모리",
        (Language::Japanese, "physical_memory") => "物理メモリ",
        (Language::Korean, "memory_last_60") => "메모리 사용률 — 최근 60초",
        (Language::Japanese, "memory_last_60") => "メモリ使用率 — 過去60秒",
        (Language::Korean, "used") => "사용 중",
        (Language::Japanese, "used") => "使用中",
        (Language::Korean, "available") => "사용 가능",
        (Language::Japanese, "available") => "使用可能",
        (Language::Korean, "swap_used") => "스왑 사용",
        (Language::Japanese, "swap_used") => "スワップ使用量",
        (Language::Korean, "swap_total") => "전체 스왑",
        (Language::Japanese, "swap_total") => "スワップ合計",
        (Language::Korean, "mounted_volumes") => "마운트된 볼륨",
        (Language::Japanese, "mounted_volumes") => "マウント済みボリューム",
        (Language::Korean, "free") => "여유 공간",
        (Language::Japanese, "free") => "空き容量",
        (Language::Korean, "search_processes") => "프로세스 검색",
        (Language::Japanese, "search_processes") => "プロセスを検索",
        (Language::Korean, "process") => "프로세스",
        (Language::Japanese, "process") => "プロセス",
        (Language::Korean, "download") => "다운로드",
        (Language::Japanese, "download") => "ダウンロード",
        (Language::Korean, "upload") => "업로드",
        (Language::Japanese, "upload") => "アップロード",
        (Language::Korean, "download_last_60") => "다운로드 — 최근 60초",
        (Language::Japanese, "download_last_60") => "ダウンロード — 過去60秒",
        (Language::Korean, "upload_last_60") => "업로드 — 최근 60초",
        (Language::Japanese, "upload_last_60") => "アップロード — 過去60秒",
        (Language::Korean, "popup_graphs") => "미니 그래프",
        (Language::Japanese, "popup_graphs") => "ミニグラフ",
        (Language::Korean, "refresh_interval") => "새로고침 간격",
        (Language::Japanese, "refresh_interval") => "更新間隔",
        (Language::Korean, "seconds_suffix") => "초",
        (Language::Japanese, "seconds_suffix") => "秒",
        (Language::Korean, "gpu_usage") => "GPU 사용률",
        (Language::Japanese, "gpu_usage") => "GPU 使用率",
        (Language::Korean, "graphics_processor") => "그래픽 프로세서",
        (Language::Japanese, "graphics_processor") => "グラフィックスプロセッサ",
        (Language::Korean, "gpu_last_60") => "GPU 사용률 — 최근 60초",
        (Language::Japanese, "gpu_last_60") => "GPU 使用率 — 過去60秒",
        (Language::Korean, "gpu_name") => "GPU 모델",
        (Language::Japanese, "gpu_name") => "GPU モデル",
        (Language::Korean, "allocated_memory") => "할당 메모리",
        (Language::Japanese, "allocated_memory") => "割り当てメモリ",
        (Language::Korean, "startup") => "시스템 시작",
        (Language::Japanese, "startup") => "システム起動",
        (Language::Korean, "startup_enable") => "부팅할 때 자동 실행",
        (Language::Japanese, "startup_enable") => "起動時に自動実行",
        (Language::Korean, "quit_app") => "프로그램 종료",
        (Language::Japanese, "quit_app") => "アプリを終了",
        (Language::Korean, "quit_title") => "완전히 종료하시겠습니까?",
        (Language::Japanese, "quit_title") => "完全に終了しますか？",
        (Language::Korean, "quit_question") => "메인 창과 팝업을 모두 종료합니다.",
        (Language::Japanese, "quit_question") => "メイン画面とポップアップを終了します。",
        (Language::Korean, "yes") => "예",
        (Language::Japanese, "yes") => "はい",
        (Language::Korean, "no") => "아니오",
        (Language::Japanese, "no") => "いいえ",
        (Language::Korean, "update_available") => "새 버전을 사용할 수 있습니다:",
        (Language::Japanese, "update_available") => "新しいバージョンがあります:",
        (Language::Korean, "download_update") => "업데이트 다운로드",
        (Language::Japanese, "download_update") => "アップデートをダウンロード",
        (Language::Korean, "download_install_update") => "앱에서 다운로드",
        (Language::Japanese, "download_install_update") => "アプリでダウンロード",
        (Language::Korean, "downloading_update") => "다운로드 중…",
        (Language::Japanese, "downloading_update") => "ダウンロード中…",
        (Language::Korean, "install_update") => "업데이트 설치",
        (Language::Japanese, "install_update") => "アップデートをインストール",
        (Language::Korean, "update_retry") => "다시 시도",
        (Language::Japanese, "update_retry") => "再試行",
        (Language::Korean, "temperature") => "온도",
        (Language::Japanese, "temperature") => "温度",
        (Language::Korean, "temperatures") => "온도 센서",
        (Language::Japanese, "temperatures") => "温度センサー",
        (Language::Korean, "later") => "나중에",
        (Language::Japanese, "later") => "後で",
        (_, "overview") => "Overview",
        (_, "cpu") => "CPU",
        (_, "memory") => "Memory",
        (_, "disks") => "Disks",
        (_, "disk") => "Disk",
        (_, "processes") => "Processes",
        (_, "network") => "Network",
        (_, "settings") => "Settings",
        (_, "language") => "Language",
        (_, "popup_title") => "Popup settings",
        (_, "popup_description") => "Keep selected system information visible in a small window.",
        (_, "popup_enable") => "Enable popup",
        (_, "visible_items") => "Visible information",
        (_, "cpu_usage") => "CPU usage",
        (_, "memory_usage") => "Memory usage",
        (_, "disk_usage") => "Disk usage",
        (_, "process_count") => "Process count",
        (_, "network_speed") => "Network speed",
        (_, "screen_position") => "Screen position",
        (_, "opacity") => "Popup opacity",
        (_, "top_left") => "Top left",
        (_, "top_center") => "Top center",
        (_, "top_right") => "Top right",
        (_, "bottom_left") => "Bottom left",
        (_, "bottom_center") => "Bottom center",
        (_, "bottom_right") => "Bottom right",
        (_, "live_performance") => "Live system performance",
        (_, "light") => "Light",
        (_, "dark") => "Dark",
        (_, "refresh_second") => "Refreshes every second",
        (_, "cpu_history") => "CPU history",
        (_, "memory_history") => "Memory history",
        (_, "system_info") => "System information",
        (_, "operating_system") => "Operating system",
        (_, "host_name") => "Host name",
        (_, "kernel") => "Kernel",
        (_, "uptime") => "Uptime",
        (_, "processor") => "Processor",
        (_, "cpu_last_60") => "CPU usage — last 60 seconds",
        (_, "logical_processors") => "Logical processors",
        (_, "core") => "Core",
        (_, "physical_memory") => "Physical memory",
        (_, "memory_last_60") => "Memory usage — last 60 seconds",
        (_, "used") => "Used",
        (_, "available") => "Available",
        (_, "swap_used") => "Swap used",
        (_, "swap_total") => "Swap total",
        (_, "mounted_volumes") => "Mounted volumes",
        (_, "free") => "Free",
        (_, "search_processes") => "Search processes",
        (_, "process") => "Process",
        (_, "download") => "Download",
        (_, "upload") => "Upload",
        (_, "download_last_60") => "Download — last 60 seconds",
        (_, "upload_last_60") => "Upload — last 60 seconds",
        (_, "popup_graphs") => "Mini graphs",
        (_, "refresh_interval") => "Refresh interval",
        (_, "seconds_suffix") => " seconds",
        (_, "gpu") => "GPU",
        (_, "gpu_usage") => "GPU usage",
        (_, "graphics_processor") => "Graphics processor",
        (_, "gpu_last_60") => "GPU usage — last 60 seconds",
        (_, "gpu_name") => "GPU model",
        (_, "allocated_memory") => "Allocated memory",
        (_, "startup") => "System startup",
        (_, "startup_enable") => "Run automatically at startup",
        (_, "quit_app") => "Quit application",
        (_, "quit_title") => "Quit completely?",
        (_, "quit_question") => "This closes both the main window and popup.",
        (_, "yes") => "Yes",
        (_, "no") => "No",
        (_, "update_available") => "A new version is available:",
        (_, "download_update") => "Download update",
        (_, "download_install_update") => "Download in app",
        (_, "downloading_update") => "Downloading…",
        (_, "install_update") => "Install update",
        (_, "update_retry") => "Retry",
        (_, "temperature") => "Temperature",
        (_, "temperatures") => "Temperature sensors",
        (_, "later") => "Later",
        _ => key,
    }
}

fn refresh_label(language: Language, seconds: u64) -> String {
    match language {
        Language::English => format!("Refreshes every {seconds} seconds"),
        Language::Korean => format!("{seconds}초마다 새로고침"),
        Language::Japanese => format!("{seconds}秒ごとに更新"),
    }
}

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|arg| arg == "--popup") {
        let opacity = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(0.92);
        let position =
            PopupPosition::from_code(args.get(3).and_then(|v| v.parse().ok()).unwrap_or(2));
        let flags = args.get(4).map(String::as_str).unwrap_or("1010011");
        let mut shown = [false; 6];
        for (slot, value) in shown.iter_mut().zip(flags.bytes()) {
            *slot = value == b'1';
        }
        let graphs = flags.as_bytes().get(6) == Some(&b'1');
        let language = Language::from_code(args.get(5).and_then(|v| v.parse().ok()).unwrap_or(0));
        let dark = args.get(6).and_then(|v| v.parse::<u8>().ok()).unwrap_or(1) != 0;
        let refresh_secs = args
            .get(7)
            .and_then(|v| v.parse().ok())
            .unwrap_or(2)
            .clamp(1, 10);
        let count = shown.into_iter().filter(|v| *v).count().max(1);
        let rows = if graphs { (count + 1) / 2 } else { 0 };
        let height = 48.0 + count as f32 * 31.0 + rows as f32 * 76.0;
        let config = PopupConfig {
            opacity,
            position,
            shown,
            graphs,
            language,
            dark,
            refresh_secs,
        };
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("Resource Monitor Popup")
                .with_inner_size([292.0, height])
                .with_resizable(false)
                .with_decorations(false)
                .with_always_on_top()
                .with_taskbar(false)
                .with_mouse_passthrough(true)
                .with_transparent(true),
            renderer: native_renderer(),
            ..Default::default()
        };
        return eframe::run_native(
            "Resource Monitor Popup",
            options,
            Box::new(move |cc| Ok(Box::new(PopupApp::new(cc, config)))),
        );
    }
    let viewport = egui::ViewportBuilder::default()
        .with_title("Resource Monitor")
        .with_inner_size([1180.0, 760.0])
        .with_min_inner_size([900.0, 620.0])
        .with_transparent(true);
    #[cfg(target_os = "macos")]
    let viewport = viewport
        .with_fullsize_content_view(true)
        .with_titlebar_shown(false)
        .with_title_shown(true)
        .with_titlebar_buttons_shown(true);
    #[cfg(not(target_os = "macos"))]
    let viewport = viewport.with_titlebar_shown(true);
    let options = eframe::NativeOptions {
        viewport,
        centered: true,
        renderer: native_renderer(),
        ..Default::default()
    };
    eframe::run_native(
        "Resource Monitor",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

#[cfg(target_os = "windows")]
fn native_renderer() -> eframe::Renderer {
    eframe::Renderer::Glow
}

#[cfg(not(target_os = "windows"))]
fn native_renderer() -> eframe::Renderer {
    eframe::Renderer::Wgpu
}

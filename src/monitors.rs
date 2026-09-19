use eframe::egui::{self, Vec2};

use super::PopupPosition;

#[derive(Clone, Copy)]
pub(super) struct MonitorArea {
    pub(super) left: f32,
    pub(super) top: f32,
    pub(super) width: f32,
    pub(super) height: f32,
}

#[cfg(target_os = "windows")]
mod windows {
    use super::{MonitorArea, PopupPosition, popup_position};
    use eframe::egui::Vec2;
    use std::ffi::c_void;

    #[repr(C)]
    struct WinRect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    struct WinMonitorInfo {
        size: u32,
        monitor: WinRect,
        work: WinRect,
        flags: u32,
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn EnumDisplayMonitors(
            dc: *mut c_void,
            clip: *const WinRect,
            callback: unsafe extern "system" fn(
                *mut c_void,
                *mut c_void,
                *mut WinRect,
                isize,
            ) -> i32,
            data: isize,
        ) -> i32;
        fn GetMonitorInfoW(monitor: *mut c_void, info: *mut WinMonitorInfo) -> i32;
        fn FindWindowW(class_name: *const u16, window_name: *const u16) -> *mut c_void;
        fn ShowWindow(window: *mut c_void, command: i32) -> i32;
        fn SetForegroundWindow(window: *mut c_void) -> i32;
        fn GetWindowRect(window: *mut c_void, rect: *mut WinRect) -> i32;
        fn SetWindowPos(
            window: *mut c_void,
            insert_after: *mut c_void,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            flags: u32,
        ) -> i32;
    }

    unsafe extern "system" fn collect(
        monitor: *mut c_void,
        _: *mut c_void,
        _: *mut WinRect,
        data: isize,
    ) -> i32 {
        let mut info = WinMonitorInfo {
            size: std::mem::size_of::<WinMonitorInfo>() as u32,
            monitor: WinRect {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            work: WinRect {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            flags: 0,
        };
        if unsafe { GetMonitorInfoW(monitor, &mut info) } != 0 {
            let monitors = unsafe { &mut *(data as *mut Vec<(bool, MonitorArea)>) };
            monitors.push((
                info.flags & 1 != 0,
                MonitorArea {
                    left: info.work.left as f32,
                    top: info.work.top as f32,
                    width: (info.work.right - info.work.left) as f32,
                    height: (info.work.bottom - info.work.top) as f32,
                },
            ));
        }
        1
    }

    pub(super) fn areas() -> Vec<MonitorArea> {
        let mut found: Vec<(bool, MonitorArea)> = Vec::new();
        unsafe {
            EnumDisplayMonitors(
                std::ptr::null_mut(),
                std::ptr::null(),
                collect,
                &mut found as *mut _ as isize,
            );
        }
        found.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| a.1.left.total_cmp(&b.1.left))
                .then_with(|| a.1.top.total_cmp(&b.1.top))
        });
        found.into_iter().map(|(_, area)| area).collect()
    }

    pub(super) fn set_popup_position(area: MonitorArea, position: PopupPosition) {
        const FLAGS: u32 = 0x0001 | 0x0004 | 0x0010;
        let title: Vec<u16> = "Resource Monitor Popup\0".encode_utf16().collect();
        unsafe {
            let window = FindWindowW(std::ptr::null(), title.as_ptr());
            let mut rect = WinRect {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            if window.is_null() || GetWindowRect(window, &mut rect) == 0 {
                return;
            }
            let size = Vec2::new(
                (rect.right - rect.left) as f32,
                (rect.bottom - rect.top) as f32,
            );
            let [x, y] = popup_position(area, size, position);
            let (x, y) = (x.round() as i32, y.round() as i32);
            if rect.left != x || rect.top != y {
                SetWindowPos(window, std::ptr::null_mut(), x, y, 0, 0, FLAGS);
            }
        }
    }

    pub(super) fn focus_main_window() {
        const SW_RESTORE: i32 = 9;
        let title: Vec<u16> = "Resource Monitor\0".encode_utf16().collect();
        unsafe {
            let window = FindWindowW(std::ptr::null(), title.as_ptr());
            if !window.is_null() {
                ShowWindow(window, SW_RESTORE);
                SetForegroundWindow(window);
            }
        }
    }
}

pub(super) fn areas(ctx: &egui::Context) -> Vec<MonitorArea> {
    #[cfg(target_os = "windows")]
    {
        let found = windows::areas();
        if !found.is_empty() {
            return found;
        }
    }
    let size = ctx
        .input(|i| i.viewport().monitor_size)
        .unwrap_or(Vec2::new(1920.0, 1080.0));
    vec![MonitorArea {
        left: 0.0,
        top: 0.0,
        width: size.x,
        height: size.y,
    }]
}

pub(super) fn popup_position(area: MonitorArea, size: Vec2, position: PopupPosition) -> [f32; 2] {
    let margin = 18.0;
    let top = area.top
        + if cfg!(target_os = "macos") {
            42.0
        } else {
            margin
        };
    let bottom = area.top + area.height - size.y - margin;
    let left = area.left + margin;
    let center = area.left + (area.width - size.x) / 2.0;
    let right = area.left + area.width - size.x - margin;
    match position {
        PopupPosition::TopLeft => [left, top],
        PopupPosition::TopCenter => [center, top],
        PopupPosition::TopRight => [right, top],
        PopupPosition::BottomLeft => [left, bottom],
        PopupPosition::BottomCenter => [center, bottom],
        PopupPosition::BottomRight => [right, bottom],
    }
}

#[cfg(target_os = "windows")]
pub(super) fn set_popup_position(area: MonitorArea, position: PopupPosition) {
    windows::set_popup_position(area, position);
}

#[cfg(target_os = "windows")]
pub(super) fn focus_main_window() {
    windows::focus_main_window();
}

#[cfg(not(target_os = "windows"))]
pub(super) fn set_popup_position(_: MonitorArea, _: PopupPosition) {}

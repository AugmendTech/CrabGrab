use std::{ffi::OsString, hash::Hash, os::{raw::c_void, windows::ffi::OsStringExt}, sync::Arc};

use windows::Win32::{Foundation::{BOOL, LPARAM, RECT, TRUE}, Graphics::Gdi::{EnumDisplayMonitors, HDC, HMONITOR}, System::{ProcessStatus::GetModuleFileNameExW, Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ}}, UI::WindowsAndMessaging::{EnumWindows, GetWindowDisplayAffinity, GetWindowRect, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsWindow, IsWindowVisible, WDA_EXCLUDEFROMCAPTURE}};

pub use windows::Win32::Foundation::HWND;

use crate::{prelude::{CapturableContentError, CapturableContentFilter, CapturableWindow}, util::{Point, Rect, Size}};

use super::AutoHandle;

#[derive(Debug, Clone)]
pub struct WindowsCapturableWindow(pub(crate) HWND);

fn hwnd_pid(hwnd: HWND) -> u32 {
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid as *mut _));
        pid
    }
}

impl WindowsCapturableWindow {
    pub fn from_impl(hwnd: HWND) -> Self {
        Self(hwnd)
    }

    pub fn id(&self) -> u32 {
        todo!("Getting ID not yet implemented for windows");
        return 0;
    }

    pub fn title(&self) -> String {
        unsafe {
            let text_length = GetWindowTextLengthW(self.0);
            if text_length == 0 {
                return "".into();
            }
            let mut text_buffer = vec![0u16; text_length as usize + 1];
            let text_length = GetWindowTextW(self.0, &mut text_buffer[..]);
            if (text_length as usize) < text_buffer.len() {
                text_buffer.truncate(text_length as usize);
            }
            String::from_utf16_lossy(&text_buffer).to_string()
        }
    }

    pub fn rect(&self) -> Rect {
        unsafe {
            let mut rect = RECT::default();
            let _ = GetWindowRect(self.0, &mut rect);
            Rect {
                origin: Point {
                    x: rect.left as f64,
                    y: rect.top as f64
                },
                size: Size {
                    width: (rect.right - rect.left) as f64,
                    height: (rect.bottom - rect.top) as f64,
                }
            }
        }
    }

    pub fn application(&self) -> WindowsCapturableApplication {
        WindowsCapturableApplication(hwnd_pid(self.0))
    }

    pub fn is_visible(&self) -> bool {
        unsafe { IsWindowVisible(self.0).as_bool() }
    }
}

impl Hash for WindowsCapturableWindow {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.0.hash(state);
    }
}

impl PartialEq for WindowsCapturableWindow {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for WindowsCapturableWindow {}

#[derive(Clone, Debug)]
pub struct WindowsCapturableDisplay(pub(crate) HMONITOR, pub(crate) RECT);

impl WindowsCapturableDisplay {
    pub fn from_impl(monitor: (HMONITOR, RECT)) -> Self {
        Self(monitor.0, monitor.1)
    }

    pub fn rect(&self) -> Rect {
        Rect {
            origin: Point {
                x: self.1.left as f64,
                y: self.1.top as f64
            },
            size: Size {
                width: (self.1.right - self.1.left) as f64,
                height: (self.1.bottom - self.1.top) as f64
            }
        }
    }
}



impl Hash for WindowsCapturableDisplay {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.0.hash(state);
    }
}

impl PartialEq for WindowsCapturableDisplay {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for WindowsCapturableDisplay {}

#[derive(Clone, Debug)]
pub struct WindowsCapturableApplication(pub(crate) u32);

impl WindowsCapturableApplication {
    pub fn identifier(&self) -> String {
        unsafe {
            let process = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, self.0);
            if process.is_err() {
                return "".into();
            }
            let process = process.unwrap();
            // TODO: If OpenProcess fails we could fall back to GetProcessHandleFromHwnd, in oleacc.dll
            //       Alternatively, it might be better to use the accessibility APIs.
            let process = AutoHandle(process);
            let mut process_name = vec![0u16; 64];
            let mut len = GetModuleFileNameExW (process.0, None, process_name.as_mut_slice()) as usize;
            while len == process_name.len() - 1 {
                process_name = vec![0u16; process_name.len() * 2];
                len = GetModuleFileNameExW (process.0, None, process_name.as_mut_slice()) as usize;
            }

            if len == 0 {
                return "".into();
            }

            let os_string = OsString::from_wide(&process_name[..len as usize]);
            let path = std::path::Path::new(&os_string);
            let file_name = path.file_name();

            if let Some(file_name) = file_name {
                if let Some(name_str) = file_name.to_str() {
                    return name_str.to_string()
                }
            }

            let result = String::from_utf16(&process_name[..len as usize]);
            result.unwrap_or("".into())
        }
    }

    pub fn name(&self) -> String {
        self.identifier()
    }

    pub fn pid(&self) -> i32 {
        self.0 as i32
    }
}

pub struct WindowsCapturableContent {
    pub(crate) windows: Vec<HWND>,
    pub(crate) displays: Vec<(HMONITOR, RECT)>,
}

unsafe extern "system" fn enum_windows_callback(window: HWND, windows_ptr_raw: LPARAM) -> BOOL {
    let windows: &mut Vec<HWND> = &mut *(windows_ptr_raw.0 as *mut c_void as *mut _);
    windows.push(window);
    TRUE
}

unsafe extern "system" fn enum_monitors_callback(monitor: HMONITOR, _: HDC, rect: *mut RECT, monitors_ptr_raw: LPARAM) -> BOOL {
    let monitors: &mut Vec<(HMONITOR, RECT)> = &mut *(monitors_ptr_raw.0 as *mut c_void as *mut _);
    monitors.push((monitor, *rect));
    TRUE
}

impl WindowsCapturableContent {
    pub async fn new(filter: CapturableContentFilter) -> Result<Self, CapturableContentError> {
        let mut displays = Vec::<(HMONITOR, RECT)>::new();
        let mut windows = Vec::<HWND>::new();
        unsafe {
            if filter.displays {
                EnumDisplayMonitors(HDC(0), None, Some(enum_monitors_callback), LPARAM(&mut displays as *mut _ as *mut c_void as isize));
            }
            if let Some(window_filter) = filter.windows {
                let _ = EnumWindows(Some(enum_windows_callback), LPARAM(&mut windows as *mut _ as *mut c_void as isize));
                windows = windows.iter().filter(|hwnd| {
                    if !IsWindow(**hwnd).as_bool() {
                        return false;
                    }
                    if window_filter.onscreen_only && !IsWindowVisible(**hwnd).as_bool() {
                        return false;
                    }
                    let mut window_display_affinity = 0;
                    if GetWindowDisplayAffinity(**hwnd, &mut window_display_affinity as *mut _).is_ok() {
                        if (window_display_affinity & WDA_EXCLUDEFROMCAPTURE.0) != 0 {
                            return false;
                        }
                    }
                    if !filter.impl_capturable_content_filter.filter_window_handle(hwnd) {
                        return false;
                    }
                    // TODO: filter desktop windows
                    true
                }).map(|hwnd| *hwnd).collect();
            }
        }
        Ok(WindowsCapturableContent {
            windows,
            displays,
        })
    }
}

/// Windows-specific extensions for capturable windows
pub trait WindowsCapturableWindowExt {
    /// Get the HWND for this capturable window.
    fn get_window_handle(&self) -> HWND;
    /// Get a capturable window from an HWND
    fn from_window_handle(window_handle: HWND) -> Result<CapturableWindow, CapturableContentError>;
}

impl WindowsCapturableWindowExt for CapturableWindow {
    fn get_window_handle(&self) -> HWND {
        self.impl_capturable_window.0
    }

    fn from_window_handle(window_handle: HWND) -> Result<Self, CapturableContentError> {
        if !unsafe { IsWindow(window_handle).as_bool() } {
            return Err(CapturableContentError::Other(format!("HWND {:016X} is not a window", window_handle.0)));
        }
        let mut window_display_affinity = 0;
        if unsafe { GetWindowDisplayAffinity(window_handle, &mut window_display_affinity as *mut _).is_ok() } {
            if (window_display_affinity & WDA_EXCLUDEFROMCAPTURE.0) != 0 {
                return Err(CapturableContentError::Other(format!("HWND {:016X} is not capturable a window", window_handle.0)));
            }
        }
        return Ok(CapturableWindow {
            impl_capturable_window: WindowsCapturableWindow(window_handle)
        })
    }
}

#[derive(Clone)]
pub(crate) struct WindowsCapturableContentFilter {
    excluded_window_handles: Option<Arc<[HWND]>>,
}

impl Default for WindowsCapturableContentFilter {
    fn default() -> Self {
        Self {
            excluded_window_handles: None,
        }
    }
}

impl WindowsCapturableContentFilter {
    pub(crate) const DEFAULT: Self = Self {
        excluded_window_handles: None,
    };
    pub(crate) const NORMAL_WINDOWS: Self = Self::DEFAULT;

    fn filter_window_handle(&self, window_handle: &HWND) -> bool {
        if let Some(excluded_window_handles) = &self.excluded_window_handles {
            if excluded_window_handles.contains(window_handle) {
                return false;
            }
        }
        true
    }
}

pub trait WindowsCapturableContentFilterExt: Sized {
    fn with_exclude_window_handles(self, window_handles: &[HWND]) -> Self;
}

impl WindowsCapturableContentFilterExt for CapturableContentFilter {
    fn with_exclude_window_handles(self, excluded_window_handles: &[HWND]) -> Self {
        let mut new_excluded_window_handles_list = vec![];
        if let Some(current_excluded_window_handles) = &self.impl_capturable_content_filter.excluded_window_handles {
            for window_handle in current_excluded_window_handles.iter() {
                new_excluded_window_handles_list.push(window_handle.to_owned());
            }
        }
        for window_handle in excluded_window_handles.iter() {
            new_excluded_window_handles_list.push((*window_handle).to_owned());
        }
        Self {
            impl_capturable_content_filter: WindowsCapturableContentFilter {
                excluded_window_handles: Some(new_excluded_window_handles_list.into_boxed_slice().into()),
                ..self.impl_capturable_content_filter
            },
            ..self
        }
    }
}


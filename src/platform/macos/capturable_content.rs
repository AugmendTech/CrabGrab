use std::{cell::Cell, fmt::Debug, hash::Hash, sync::Arc};

use futures::channel::oneshot;
use libc::getpid;
use parking_lot::Mutex;

use crate::{capturable_content::{CapturableContentError, CapturableContentFilter}, prelude::{CapturableContent, CapturableWindow}, util::{Point, Rect, Size}};

use super::objc_wrap::{get_window_description, get_window_levels, CGMainDisplayID, CGWindowID, SCDisplay, SCRunningApplication, SCShareableContent, SCWindow};

pub struct MacosCapturableContent {
    pub windows: Vec<SCWindow>,
    pub excluding_windows: Vec<SCWindow>,
    pub displays: Vec<SCDisplay>,
}

impl MacosCapturableContent {
    pub async fn new(filter: CapturableContentFilter) -> Result<Self, CapturableContentError> {
        // Force core graphics initialization
        unsafe { CGMainDisplayID() };
        let (exclude_desktop, onscreen_only) = filter.windows.map_or((false, true), |filter| (!filter.desktop_windows, filter.onscreen_only));
        let (tx, rx) = oneshot::channel();
        let mut tx = Mutex::new(Some(tx));
        SCShareableContent::get_shareable_content_with_completion_handler(exclude_desktop, onscreen_only, move |result| {
            if let Some(tx) = tx.lock().take() {
                let _ = tx.send(result);
            }
        });

        match rx.await {
            Ok(Ok(content)) => {
                let windows = content.windows()
                    .into_iter()
                    .filter(|window| filter.impl_capturable_content_filter.filter_scwindow(window))
                    .collect();
                let excluding_windows = content.windows()
                    .into_iter()
                    .filter(|window| !filter.impl_capturable_content_filter.filter_scwindow(window))
                    .collect();
                let displays = content.displays()
                    .into_iter()
                    .filter(|display| filter.impl_capturable_content_filter.filter_scdisplay(display))
                    .collect();
                Ok(Self {
                    windows,
                    displays,
                    excluding_windows,
                })
            },
            Ok(Err(error)) => {
                Err(CapturableContentError::Other(format!("SCShareableContent returned error code: {}", error.code())))
            }
            Err(error) => Err(CapturableContentError::Other(format!("Failed to receive SCSharableContent result from completion handler future: {}", error.to_string()))),
        }
    }
}

#[derive(Clone)]
pub struct MacosCapturableWindow {
    pub(crate) window: SCWindow
}

impl MacosCapturableWindow {
    pub fn from_impl(window: SCWindow) -> Self {
        Self {
            window
        }
    }

    pub fn id(&self) -> u32 {
        self.window.id().0
    }

    pub fn title(&self) -> String {
        self.window.title()
    }

    pub fn rect(&self) -> Rect {
        let frame = self.window.frame();
        Rect {
            origin: Point {
                x: frame.origin.x,
                y: frame.origin.y,
            },
            size: Size {
                width: frame.size.x,
                height: frame.size.y
            }
        }
    }

    pub fn application(&self) -> MacosCapturableApplication {
        MacosCapturableApplication {
            running_application: self.window.owning_application()
        }
    }

    pub fn is_visible(&self) -> bool {
        self.window.on_screen()
    }
}

impl Debug for MacosCapturableWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MacosCapturableWindow").field("window", &self.window.title()).finish()
    }
}

impl PartialEq for MacosCapturableWindow {
    fn eq(&self, other: &Self) -> bool {
        self.window.id().0 == other.window.id().0
    }
}

impl Hash for MacosCapturableWindow {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.window.id().0.hash(state);
    }
}

impl Eq for MacosCapturableWindow {}

#[derive(Clone)]
pub struct MacosCapturableDisplay {
    pub(crate) display: SCDisplay
}

impl MacosCapturableDisplay {
    pub fn from_impl(display: SCDisplay) -> Self {
        Self {
            display
        }
    }

    pub fn rect(&self) -> Rect {
        let frame = self.display.frame();
        Rect {
            origin: Point {
                x: frame.origin.x,
                y: frame.origin.y,
            },
            size: Size {
                width: frame.size.x,
                height: frame.size.y
            }
        }
    }
}

impl PartialEq for MacosCapturableDisplay {
    fn eq(&self, other: &Self) -> bool {
        self.display.raw_id() == other.display.raw_id()
    }
}

impl Hash for MacosCapturableDisplay {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.display.raw_id().hash(state)
    }
}

impl Eq for MacosCapturableDisplay {}

impl Debug for MacosCapturableDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MacosCapturableDisplay").field("display", &self.display.raw_id()).finish()
    }
}

#[derive()]
pub struct MacosCapturableApplication {
    pub(crate) running_application: SCRunningApplication,
}

impl MacosCapturableApplication {
    pub fn identifier(&self) -> String {
        self.running_application.bundle_identifier()
    }

    pub fn name(&self) -> String {
        self.running_application.application_name()
    }

    pub fn pid(&self) -> i32 {
        self.running_application.pid()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
/// Represents the "window level" of a native Mac OS window. Windows within the same level are ordered above or below levels that are above below or above this level respectively.
pub enum MacosWindowLevel {
    BelowDesktop      =  0,
    Desktop           =  1,
    DesktopIcon       =  2,
    Backstop          =  3,
    Normal            =  4,
    Floating          =  5,
    TornOffMenu       =  6,
    Dock              =  7,
    MainMenu          =  8,
    Status            =  9,
    ModalPanel        = 10,
    PopupMenu         = 11,
    Dragging          = 12,
    ScreenSaver       = 13,
    Overlay           = 14,
    Help              = 15,
    Utility           = 16,
    Cursor            = 17,
    AssistiveTechHigh = 18,
}

/// A capturable window with mac-os specific features
pub trait MacosCapturableWindowExt {
    /// Get the window layer of this window
    fn get_window_layer(&self) -> Result<i32, CapturableContentError>;

    /// Get the window level of this window
    fn get_window_level(&self) -> Result<MacosWindowLevel, CapturableContentError>;

    /// Get the native window id for this capturable window.
    /// This is the `CGWindowID` for this window.
    fn get_window_id(&self) -> u32;

    /// Try and convert the given CGWindowID to a capturable window.
    fn from_window_id(window_id: u32) -> impl std::future::Future<Output = Result<CapturableWindow, CapturableContentError>>;
}

fn get_window_layer(window_id: u32) -> Result<i32, ()> {
    let window_description = get_window_description(CGWindowID(window_id))?;
    Ok(window_description.window_layer)
}

fn get_window_level(window_id: u32) -> Result<MacosWindowLevel, ()> {
    let window_levels = get_window_levels();
    let level = get_window_layer(window_id)?;
    Ok(
        if (level < window_levels.desktop) {
            MacosWindowLevel::BelowDesktop
        } else if (level < window_levels.desktop_icon) {
            MacosWindowLevel::Desktop
        } else if (level < window_levels.backstop) {
            MacosWindowLevel::DesktopIcon
        } else if (level < window_levels.normal) {
            MacosWindowLevel::Backstop
        } else if (level < window_levels.floating) {
            MacosWindowLevel::Normal
        } else if (level < window_levels.torn_off_menu) {
            MacosWindowLevel::Floating
        } else if (level < window_levels.modal_panel) {
            MacosWindowLevel::TornOffMenu
        } else if (level < window_levels.utility) {
            MacosWindowLevel::ModalPanel
        } else if (level < window_levels.dock) {
            MacosWindowLevel::Utility
        } else if (level < window_levels.main_menu) {
            MacosWindowLevel::Dock
        } else if (level < window_levels.status) {
            MacosWindowLevel::MainMenu
        } else if (level < window_levels.pop_up_menu) {
            MacosWindowLevel::Status
        } else if (level < window_levels.overlay) {
            MacosWindowLevel::PopupMenu
        } else if (level < window_levels.help) {
            MacosWindowLevel::Overlay
        } else if (level < window_levels.dragging) {
            MacosWindowLevel::Help
        } else if (level < window_levels.screen_saver) {
            MacosWindowLevel::Dragging
        } else if (level < window_levels.assistive_tech_high) {
            MacosWindowLevel::ScreenSaver
        } else if (level < window_levels.cursor) {
            MacosWindowLevel::AssistiveTechHigh
        } else {
            MacosWindowLevel::Cursor
        }
    )
}

impl MacosCapturableWindowExt for CapturableWindow {
    fn get_window_layer(&self) -> Result<i32, CapturableContentError> {
        get_window_layer(self.impl_capturable_window.window.id().0)
            .map_err(|_| CapturableContentError::Other(("Failed to retreive window layer".to_string())))
    }

    fn get_window_level(&self) -> Result<MacosWindowLevel, CapturableContentError> {
        get_window_level(self.impl_capturable_window.window.id().0)
            .map_err(|_| CapturableContentError::Other(("Failed to retreive window level".to_string())))
    }

    fn get_window_id(&self) -> u32 {
        self.impl_capturable_window.window.id().0
     }
 
     fn from_window_id(window_id: u32) -> impl std::future::Future<Output = Result<CapturableWindow, CapturableContentError>> {
         async move {
             let content = CapturableContent::new(CapturableContentFilter::ALL_WINDOWS).await?;
             for window in content.windows().into_iter() {
                 if window.get_window_id() == window_id {
                     return Ok(window.clone());
                 }
             }
             Err(CapturableContentError::Other(format!("No capturable window with id: {} found", window_id)))
         }
     }
}

#[derive(Clone)]
pub(crate) struct MacosCapturableContentFilter {
    pub window_level_range: (Option<MacosWindowLevel>, Option<MacosWindowLevel>),
    pub excluded_bundle_ids: Option<Arc<[String]>>,
    pub excluded_window_ids: Option<Arc<[u32]>>,
}

impl Default for MacosCapturableContentFilter {
    fn default() -> Self {
        Self {
            window_level_range: (None, None),
            excluded_bundle_ids: None,
            excluded_window_ids: None,
        }
    }
}

impl MacosCapturableContentFilter {
    fn filter_scwindow(&self, window: &SCWindow) -> bool {
        let mut allow = true;
        if self.window_level_range != (None, None) {
            if let Ok(level) = get_window_level(window.id().0) {
                allow &= match &self.window_level_range {
                    (Some(min), Some(max)) => (level >= *min) && (level <= *max),
                    (Some(min), None) => level >= *min,
                    (None, Some(max)) => level <= *max,
                    (None, None) => unreachable!(),
                };
            }
        }
        if let Some(excluded_bundle_ids) = &self.excluded_bundle_ids {
            let bundle_id = window.owning_application().bundle_identifier();
            if excluded_bundle_ids.contains(&bundle_id.to_lowercase()) {
                allow = false;
            }
        }
        if let Some(excluded_window_ids) = &self.excluded_window_ids {
            if excluded_window_ids.contains(&window.id().0) {
                allow = false;
            }
        }
        allow
    }

    fn filter_scdisplay(&self, display: &SCDisplay) -> bool {
        true
    }

    pub const DEFAULT: Self = MacosCapturableContentFilter {
        window_level_range: (None, None),
        excluded_bundle_ids: None,
        excluded_window_ids: None,
    };

    pub const NORMAL_WINDOWS: Self = MacosCapturableContentFilter {
        window_level_range: (Some(MacosWindowLevel::Normal), Some(MacosWindowLevel::TornOffMenu)),
        excluded_bundle_ids: None,
        excluded_window_ids: None,
    };
}

/// A capturable content filter with Mac OS specific options
pub trait MacosCapturableContentFilterExt: Sized {
    /// Set the range of "window levels" to filter to (inclusive)
    fn with_window_level_range(self, min: Option<MacosWindowLevel>, max: Option<MacosWindowLevel>) -> Result<Self, CapturableContentError>;
    /// Exclude windows who's applications have the provided bundle ids
    fn with_exclude_bundle_ids(self, bundle_id: &[&str]) -> Self;
    /// Exclude windows with the given CGWindowIDs
    fn with_exclude_window_ids(self, window_ids: &[u32]) -> Self;
}

impl MacosCapturableContentFilterExt for CapturableContentFilter {
    fn with_window_level_range(self, min: Option<MacosWindowLevel>, max: Option<MacosWindowLevel>) -> Result<Self, CapturableContentError> {
        match (&min, &max) {
            (Some(min_level), Some(max_level)) => {
                if *min_level as i32 > *max_level as i32 {
                    return Err(CapturableContentError::Other(format!("Invalid window level range: minimum level: {:?} is greater than maximum level: {:?}", *min_level, *max_level)));
                }
            },
            _ => {}
        }
        Ok(Self {
            impl_capturable_content_filter: MacosCapturableContentFilter {
                window_level_range: (min, max),
                ..self.impl_capturable_content_filter
            },
            ..self
        })
    }

    fn with_exclude_bundle_ids(self, excluded_bundle_ids: &[&str]) -> Self {
        let mut new_bundle_id_list = vec![];
        if let Some(current_bundle_ids) = &self.impl_capturable_content_filter.excluded_bundle_ids {
            for bundle_id in current_bundle_ids.iter() {
                new_bundle_id_list.push(bundle_id.to_owned());
            }
        }
        for bundle_id in excluded_bundle_ids.iter() {
            new_bundle_id_list.push((*bundle_id).to_lowercase());
        }
        Self {
            impl_capturable_content_filter: MacosCapturableContentFilter {
                excluded_bundle_ids: Some(new_bundle_id_list.into_boxed_slice().into()),
                ..self.impl_capturable_content_filter
            },
            ..self
        }
    }

    fn with_exclude_window_ids(self, excluded_window_ids: &[u32]) -> Self {
        let mut new_excluded_window_id_list = vec![];
        if let Some(current_excluded_window_ids) = &self.impl_capturable_content_filter.excluded_window_ids {
            for window_id in current_excluded_window_ids.iter() {
                new_excluded_window_id_list.push(*window_id);
            }
        }
        for window_id in excluded_window_ids.iter() {
            new_excluded_window_id_list.push(*window_id);
        }
        Self {
            impl_capturable_content_filter: MacosCapturableContentFilter {
                excluded_window_ids: Some(new_excluded_window_id_list.into_boxed_slice().into()),
                ..self.impl_capturable_content_filter
            },
            ..self
        }
    }
}
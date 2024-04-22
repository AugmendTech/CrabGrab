use std::{cell::Cell, fmt::Debug, hash::Hash};

use futures::channel::oneshot;
use libc::getpid;
use parking_lot::Mutex;

use crate::{capturable_content::{CapturableContentFilter, CapturableContentError}, util::{Rect, Point, Size}};

use super::objc_wrap::{CGMainDisplayID, SCDisplay, SCRunningApplication, SCShareableContent, SCWindow};

pub struct MacosCapturableContent {
    pub windows: Vec<SCWindow>,
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
            println!("get_shareable_content_with_completion_handler - handler");
            if let Some(tx) = tx.lock().take() {
                let _ = tx.send(result);
            }
        });

        match rx.await {
            Ok(Ok(content)) => {
                let windows = content.windows();
                let displays = content.displays();
                Ok(Self {
                    windows,
                    displays,
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
}

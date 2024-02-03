use std::cell::Cell;

use futures::channel::oneshot;

use crate::{capturable_content::{CapturableContentFilter, CapturableContentError}, util::{Rect, Point, Size}};

use super::objc_wrap::{SCShareableContent, SCWindow, SCDisplay};

pub struct MacosCapturableContent {
    pub windows: Vec<SCWindow>,
    pub displays: Vec<SCDisplay>,
}

impl MacosCapturableContent {
    pub async fn new(filter: CapturableContentFilter) -> Result<Self, CapturableContentError> {
        let (exclude_desktop, onscreen_only) = filter.windows.map_or((false, true), |filter| (!filter.desktop_windows, filter.onscreen_only));
        let (tx, rx) = oneshot::channel();
        let tx = Cell::new(Some(tx));
        SCShareableContent::get_shareable_content_with_completion_handler(exclude_desktop, onscreen_only, move |result| {
            if let Some(tx) = tx.take() {
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
            Err(_) => Err(CapturableContentError::Other("Failed to receive SCSharableContent result from completion handler future".into())),
        }
    }
}

#[derive(Clone)]
pub struct MacosCapturableWindow {
    window: SCWindow
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
}

pub struct MacosCapturableDisplay {
    display: SCDisplay
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

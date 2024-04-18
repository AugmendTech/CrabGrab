use std::{error::Error, fmt::{Debug, Display}};

use crate::{platform::platform_impl::{ImplCapturableApplication, ImplCapturableContent, ImplCapturableDisplay, ImplCapturableWindow}, util::Rect};

/// Represents an error that occured when enumerating capturable content
#[derive(Debug, Clone)]
pub enum CapturableContentError {
    Other(String)
}

impl Display for CapturableContentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Other(message) => f.write_fmt(format_args!("CapturableContentError::Other(\"{}\")", message))
        }
    }
}

impl Error for CapturableContentError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

/// Selects the kind of windows to enumerate for capture
pub struct CapturableWindowFilter {
    /// Desktop windows are elements of the desktop environment, E.G. the dock on macos or the start bar on windows.
    pub desktop_windows: bool,
    /// Whether to restrict to onscreen windows
    pub onscreen_only: bool,
}

impl Default for CapturableWindowFilter {
    fn default() -> Self {
        Self { desktop_windows: false, onscreen_only: true }
    }
}

/// Selects the kind of capturable content to enumerate
pub struct CapturableContentFilter {
    /// What kind of capturable windows, if Some, to enumerate
    pub windows: Option<CapturableWindowFilter>,
    /// Whether to enumerate capturable displays
    pub displays: bool,
}

impl CapturableContentFilter {
    /// Whether this filter allows any capturable content
    pub fn is_empty(&self) -> bool {
        !(
            self.windows.is_some() ||
            self.displays
        )
    }

    pub const DISPLAYS: Self = CapturableContentFilter {
        windows: None,
        displays: true,
    };

    pub const ALL_WINDOWS: Self = CapturableContentFilter {
        windows: Some(CapturableWindowFilter {
            desktop_windows: true,
            onscreen_only: false,
        }),
        displays: false,
    };

    pub const EVERYTHING: Self = CapturableContentFilter {
        windows: Some(CapturableWindowFilter {
            desktop_windows: true,
            onscreen_only: false,
        }),
        displays: true,
    };

    pub const NORMAL_WINDOWS: Self = CapturableContentFilter {
        windows: Some(CapturableWindowFilter {
            desktop_windows: false,
            onscreen_only: true
        }),
        displays: false,
    };

    pub const EVERYTHING_NORMAL: Self = CapturableContentFilter {
        windows: Some(CapturableWindowFilter {
            desktop_windows: false,
            onscreen_only: true,
        }),
        displays: true,
    };
}

pub struct CapturableContent {
    impl_capturable_content: ImplCapturableContent
}

unsafe impl Send for CapturableContent {}
unsafe impl Sync for CapturableContent {}

/// An iterator over capturable windows
pub struct CapturableWindowIterator<'content> {
    content: &'content CapturableContent,
    i: usize
}

impl Iterator for CapturableWindowIterator<'_> {
    type Item = CapturableWindow;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i < self.content.impl_capturable_content.windows.len() {
            let i = self.i;
            self.i += 1;
            Some(CapturableWindow { impl_capturable_window: ImplCapturableWindow::from_impl(self.content.impl_capturable_content.windows[i].clone()) })
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.i, Some(self.content.impl_capturable_content.windows.len()))
    }
}

impl ExactSizeIterator for CapturableWindowIterator<'_> {
}

/// An iterator over capturable displays
pub struct CapturableDisplayIterator<'content> {
    content: &'content CapturableContent,
    i: usize
}

impl Iterator for CapturableDisplayIterator<'_> {
    type Item = CapturableDisplay;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i < self.content.impl_capturable_content.displays.len() {
            let i = self.i;
            self.i += 1;
            Some(CapturableDisplay { impl_capturable_display: ImplCapturableDisplay::from_impl(self.content.impl_capturable_content.displays[i].clone()) })
        } else {
            None
        }
    }
    
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.i, Some(self.content.impl_capturable_content.displays.len()))
    }
}

impl ExactSizeIterator for CapturableDisplayIterator<'_> {
    fn len(&self) -> usize {
        self.content.impl_capturable_content.displays.len()
    }
}

impl CapturableContent {
    /// Requests capturable content from the OS
    /// 
    /// Note that the returned capturable content may be stale - for example, a window enumerated in this capturable content
    /// may have been closed before it is used to open a stream, and creating a stream for that window will result in an error.
    pub async fn new(filter: CapturableContentFilter) -> Result<Self, CapturableContentError> {
        Ok(Self {
            impl_capturable_content: ImplCapturableContent::new(filter).await?
        })
    }

    /// Get an iterator over the capturable windows
    pub fn windows<'a>(&'a self) -> CapturableWindowIterator<'a> {
        CapturableWindowIterator { content: self, i: 0 }
    }

    /// Get an iterator over the capturable displays
    pub fn displays<'a>(&'a self) -> CapturableDisplayIterator<'a> {
        CapturableDisplayIterator { content: self, i: 0 }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum Capturable {
    Window(CapturableWindow),
    Display(CapturableDisplay),
}

/// Represents a capturable application window
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct CapturableWindow {
    pub(crate) impl_capturable_window: ImplCapturableWindow
}

unsafe impl Send for CapturableWindow {}
unsafe impl Sync for CapturableWindow {}

impl CapturableWindow {
    /// Gets the title of the window
    pub fn title(&self) -> String {
        self.impl_capturable_window.title()
    }

    /// Gets the virtual screen rectangle of the window
    pub fn rect(&self) -> Rect {
        self.impl_capturable_window.rect()
    }

    /// Gets the application that owns this window
    pub fn application(&self) -> CapturableApplication {
        CapturableApplication {
            impl_capturable_application: self.impl_capturable_window.application()
        }
    }
}

/// Represents a capturable display
#[derive(Debug, Clone)]
pub struct CapturableDisplay {
    pub(crate) impl_capturable_display: ImplCapturableDisplay
}

impl CapturableDisplay {
    /// Gets the virtual screen rectangle of this display
    /// 
    /// Note: Currently on windows, this is only evaluated at the time of display enumeration
    pub fn rect(&self) -> Rect {
        self.impl_capturable_display.rect()
    }
}

unsafe impl Send for CapturableDisplay {}
unsafe impl Sync for CapturableDisplay {}

// Represents an application with capturable windows
pub struct CapturableApplication {
    impl_capturable_application: ImplCapturableApplication
}

impl CapturableApplication {
    /// Gets the "identifier" of the application
    /// 
    /// On Macos, this is the application bundle, and on windows, this is the application file name
    pub fn identifier(&self) -> String {
        self.impl_capturable_application.identifier()
    }
}

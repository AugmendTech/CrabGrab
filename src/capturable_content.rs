use std::{error::Error, fmt::{Debug, Display}};

use crate::{platform::platform_impl::{ImplCapturableApplication, ImplCapturableContent, ImplCapturableDisplay, ImplCapturableWindow}, util::Rect};

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

pub struct CapturableWindowFilter {
    pub desktop_windows: bool,
    pub onscreen_only: bool,
}

impl Default for CapturableWindowFilter {
    fn default() -> Self {
        Self { desktop_windows: false, onscreen_only: true }
    }
}

pub struct CapturableContentFilter {
    pub windows: Option<CapturableWindowFilter>,
    pub displays: bool,
}

impl CapturableContentFilter {
    pub fn is_empty(&self) -> bool {
        !(
            self.windows.is_some() ||
            self.displays
        )
    }
}

pub struct CapturableContent {
    impl_capturable_content: ImplCapturableContent
}

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
    pub async fn new(filter: CapturableContentFilter) -> Result<Self, CapturableContentError> {
        Ok(Self {
            impl_capturable_content: ImplCapturableContent::new(filter).await?
        })
    }

    pub fn windows<'a>(&'a self) -> CapturableWindowIterator<'a> {
        CapturableWindowIterator { content: self, i: 0 }
    }

    pub fn displays<'a>(&'a self) -> CapturableDisplayIterator<'a> {
        CapturableDisplayIterator { content: self, i: 0 }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum Capturable {
    Window(CapturableWindow),
    Display(CapturableDisplay),
}

#[derive(Debug, Clone)]
pub struct CapturableWindow {
    pub(crate) impl_capturable_window: ImplCapturableWindow
}

impl CapturableWindow {
    pub fn title(&self) -> String {
        self.impl_capturable_window.title()
    }

    pub fn rect(&self) -> Rect {
        self.impl_capturable_window.rect()
    }

    pub fn application(&self) -> CapturableApplication {
        CapturableApplication {
            impl_capturable_application: self.impl_capturable_window.application()
        }
    }
}

#[derive(Debug, Clone)]
pub struct CapturableDisplay {
    pub(crate) impl_capturable_display: ImplCapturableDisplay
}

impl CapturableDisplay {
    pub fn rect(&self) -> Rect {
        self.impl_capturable_display.rect()
    }
}

pub struct CapturableApplication {
    impl_capturable_application: ImplCapturableApplication
}

impl CapturableApplication {
    pub fn identifier(&self) -> String {
        self.impl_capturable_application.identifier()
    }
}

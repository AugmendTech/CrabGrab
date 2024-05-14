mod platform;
use std::{error::Error, fmt::Display};

pub use platform::take_screenshot;

#[derive(Debug)]
/// Represents an error while taking a screenshot
pub enum ScreenshotError {
    Other(String)
}

unsafe impl Send for ScreenshotError {}
unsafe impl Sync for ScreenshotError {}

impl Display for ScreenshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Other(error) => f.write_fmt(format_args!("ScreenshotError::Other({})", error)),
        }
    }
}

impl Error for ScreenshotError {
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

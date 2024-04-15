mod platform;
pub use platform::take_screenshot;

#[derive(Debug)]
pub enum ScreenshotError {
    Other(String)
}

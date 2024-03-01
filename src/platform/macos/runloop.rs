use super::objc_wrap::NSApplication;

pub struct MacosRunloop {
    app: NSApplication
}

impl MacosRunloop {
    pub fn new() -> Self {
        Self {
            app: NSApplication::shared()
        }
    }

    pub fn run(&self) {
        self.app.run();
    }
}

use crate::platform::platform_impl::ImplRunloop;

#[derive(Debug, Copy, Clone)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

impl Size {
    pub fn scaled(&self, scale: f64) -> Self {
        Self {
            width: self.width * scale,
            height: self.height * scale
        }
    }

    pub fn scaled_2d(&self, scale: (f64, f64)) -> Self {
        Self {
            width: self.width * scale.0,
            height: self.height * scale.1
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const ZERO: Point = Point {
        x: 0.0, 
        y: 0.0
    };

    pub fn scaled(&self, scale: f64) -> Self {
        Self {
            x: self.x * scale,
            y: self.y * scale
        }
    }

    pub fn scaled_2d(&self, scale: (f64, f64)) -> Self {
        Self {
            x: self.x * scale.0,
            y: self.y * scale.1
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Rect {
    pub fn scaled(&self, scale: f64) -> Self {
        Self {
            origin: self.origin.scaled(scale),
            size: self.size.scaled(scale)
        }
    }

    pub fn scaled_2d(&self, scale: (f64, f64)) -> Self {
        Self {
            origin: self.origin.scaled_2d(scale),
            size: self.size.scaled_2d(scale)
        }
    }
}

pub struct Runloop {
    impl_runloop: ImplRunloop
}

impl Runloop {
    pub fn new() -> Self {
        Self {
            impl_runloop: ImplRunloop::new()
        }
    }

    pub fn run(self) {
        self.impl_runloop.run()
    }
}

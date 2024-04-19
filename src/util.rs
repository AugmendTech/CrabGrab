/// Represents a 2D size
#[derive(Debug, Copy, Clone)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

impl Size {
    /// scale the size uniformly by some value
    pub fn scaled(&self, scale: f64) -> Self {
        Self {
            width: self.width * scale,
            height: self.height * scale
        }
    }

    /// scale the size non-uniformly in x and y
    pub fn scaled_2d(&self, scale: (f64, f64)) -> Self {
        Self {
            width: self.width * scale.0,
            height: self.height * scale.1
        }
    }
}

/// Represents a 2D point
#[derive(Debug, Copy, Clone)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    /// The point at (0, 0)
    pub const ZERO: Point = Point {
        x: 0.0, 
        y: 0.0
    };

    /// Scale the point uniformly by some value
    pub fn scaled(&self, scale: f64) -> Self {
        Self {
            x: self.x * scale,
            y: self.y * scale
        }
    }

    /// Scale the point non-uniformly in x and y
    pub fn scaled_2d(&self, scale: (f64, f64)) -> Self {
        Self {
            x: self.x * scale.0,
            y: self.y * scale.1
        }
    }
}

/// Represents an axis-aligned rectangle
#[derive(Debug, Copy, Clone)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Rect {
    /// Scale the rectangle uniformly
    pub fn scaled(&self, scale: f64) -> Self {
        Self {
            origin: self.origin.scaled(scale),
            size: self.size.scaled(scale)
        }
    }

    /// Scale the rectangle non-uniformly in x and y
    pub fn scaled_2d(&self, scale: (f64, f64)) -> Self {
        Self {
            origin: self.origin.scaled_2d(scale),
            size: self.size.scaled_2d(scale)
        }
    }
}

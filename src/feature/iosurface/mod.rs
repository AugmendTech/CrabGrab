#![cfg(feature = "iosurface")]

use objc::runtime::Object;

pub struct IoSurface {
    raw: *mut Object,
}

impl IoSurface {
    pub fn get_raw() -> *mut Object;
}

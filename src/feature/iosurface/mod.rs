#![cfg(feature = "iosurface")]

use objc::runtime::Object;

pub struct IoSurface {

}

impl IoSurface {
    pub fn get_raw() -> *mut Object;
}

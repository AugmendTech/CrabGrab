#![allow(unused)]

#[link(name = "ScreenCaptureKit", kind = "framework")]
#[link(name = "CoreGraphics", kind = "framework")]
#[link(name = "CoreMedia", kind = "framework")]
#[link(name = "CoreVideo", kind = "framework")]
#[link(name = "IOSurface", kind = "framework")]
#[link(name = "System", kind = "dylib")]
#[link(name = "Foundation", kind = "framework")]
#[link(name = "AppKit", kind = "framework")]
#[link(name = "ApplicationServices", kind = "framework")]
#[link(name = "AVFoundation", kind = "framework")]
extern "C" {}

use block::ConcreteBlock;
use libc::{c_void, strlen};
use objc::{msg_send, sel, sel_impl, class, runtime::Object, Encode, Encoding};
use objc2::runtime::Bool;

type CFTypeRef = *const c_void;
type CFStringRef = CFTypeRef;

extern "C" {
    fn CFRetain(x: CFTypeRef) -> CFTypeRef;
    fn CFRelease(x: CFTypeRef);
}

#[repr(C)]
pub struct NSString(CFStringRef);

impl NSString {
    pub fn from_ref_unretained(r: CFStringRef) -> Self {
        unsafe { CFRetain(r); }
        Self(r)
    }

    pub fn from_ref_retained(r: CFStringRef) -> Self {
        Self(r)
    }

    pub fn as_string(&self) -> String {
        unsafe {
            let c_str: *const i8 = msg_send![self.0 as *mut Object, UTF8String];
            let len = strlen(c_str);
            let bytes = std::slice::from_raw_parts(c_str as *const u8, len);
            String::from_utf8_lossy(bytes).into_owned()
        }
    }
}

unsafe impl Encode for NSString {
    fn encode() -> Encoding {
        unsafe { Encoding::from_str("^@\"NSString\"") }
    }
}

#[repr(C)]
pub struct NSError(*mut Object);
unsafe impl Send for NSError {}

impl NSError {
    pub fn code(&self) -> isize {
        unsafe { msg_send![self.0, code] }
    }

    pub fn domain(&self) -> String {
        unsafe {
            let domain_cfstringref: CFStringRef = msg_send![self.0, domain];
            NSString::from_ref_retained(domain_cfstringref).as_string()
        }
    }
}

impl Drop for NSError {
    fn drop(&mut self) {
        unsafe { let _: () = msg_send![self.0, release]; };
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct NSArrayRef(*mut Object);

impl NSArrayRef {
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

unsafe impl Encode for NSArrayRef {
    fn encode() -> objc::Encoding {
        unsafe { Encoding::from_str("@\"NSArray\"") }
    }
}

#[repr(C)]
pub struct NSArray(*mut Object);

impl NSArray {
    pub (crate) fn from_ref(r: NSArrayRef) -> Self {
        Self::from_id_unretained(r.0)
    }

    pub(crate) fn from_id_unretained(id: *mut Object) -> Self {
        unsafe { let _: () = msg_send![id, retain]; }
        Self(id)
    }

    pub(crate) fn from_id_retained(id: *mut Object) -> Self {
        Self(id)
    }

    pub fn count(&self) -> usize {
        unsafe { msg_send![self.0, count] }
    }

    pub fn obj_at_index<T: 'static>(&self, i: usize) -> T {
        unsafe { msg_send![self.0, objectAtIndex: i] }
    }
}

impl Drop for NSArray {
    fn drop(&mut self) {
        unsafe { let _: () = msg_send![self.0, release]; }
    }
}

type CGFloat = f64;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct CGPoint {
    pub x: CGFloat,
    pub y: CGFloat,
}

impl CGPoint {
    pub const ZERO: CGPoint = CGPoint { x: 0.0, y: 0.0 };
    pub const INF: CGPoint = CGPoint { x: std::f64::INFINITY, y: std::f64::INFINITY };
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct CGSize {
    pub x: CGFloat,
    pub y: CGFloat,
}

impl CGSize {
    pub const ZERO: CGSize = CGSize { x: 0.0, y: 0.0 };
}

unsafe impl Encode for CGSize {
    fn encode() -> Encoding {
        unsafe { Encoding::from_str("{CGSize=\"width\"d\"height\"d}") }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct CGRect {
    pub origin: CGPoint,
    pub size: CGSize,
}

impl CGRect {
    pub const ZERO: CGRect = CGRect {
        origin: CGPoint::ZERO,
        size: CGSize::ZERO
    };

    pub const NULL: CGRect = CGRect {
        origin: CGPoint::INF,
        size: CGSize::ZERO
    };

    pub fn contains(&self, p: CGPoint) -> bool {
        p.x >= self.origin.x &&
        p.y >= self.origin.y &&
        p.x <= (self.origin.x + self.size.x) &&
        p.y <= (self.origin.y + self.size.y)
    }
}

unsafe impl Encode for CGRect {
    fn encode() -> Encoding {
        unsafe { Encoding::from_str("{CGRect=\"origin\"{CGPoint=\"x\"d\"y\"d}\"size\"{CGSize=\"width\"d\"height\"d}}") }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CGWindowID(pub u32);

impl CGWindowID {
    pub fn raw(&self) -> u32 {
        self.0
    }
}

unsafe impl Send for CGWindowID {}

#[repr(C)]
pub struct SCWindow(*mut Object);
unsafe impl Send for SCWindow {}

impl SCWindow {
    pub(crate) fn from_id_unretained(id: *mut Object) -> Self {
        unsafe { let _: () = msg_send![id, retain]; }
        Self(id)
    }

    pub(crate) fn from_id_retained(id: *mut Object) -> Self {
        Self(id)
    }

    pub(crate) fn id(&self) -> CGWindowID {
        unsafe { 
            let id: u32 = msg_send![self.0, windowID];
            CGWindowID(id)
        }
    }

    pub fn title(&self) -> String {
        unsafe {
            let title_cfstringref: CFStringRef = msg_send![self.0, title];
            NSString::from_ref_unretained(title_cfstringref).as_string()
        }
    }

    pub fn frame(&self) -> CGRect {
        unsafe {
            *(*self.0).get_ivar("_frame")
        }
    }
}

impl Clone for SCWindow {
    fn clone(&self) -> Self {
        unsafe { let _: () = msg_send![self.0, retain]; }
        SCWindow(self.0)
    }
}

impl Drop for SCWindow {
    fn drop(&mut self) {
        unsafe { let _: () = msg_send![self.0, release]; }
    }
}

#[repr(C)]
pub struct SCDisplay(*mut Object);
unsafe impl Send for SCDisplay {}

impl SCDisplay {
    pub(crate) fn from_id_unretained(id: *mut Object) -> Self {
        unsafe { let _: () = msg_send![id, retain]; }
        Self(id)
    }

    pub(crate) fn from_id_retained(id: *mut Object) -> Self {
        Self(id)
    }

    pub fn frame(&self) -> CGRect {
        unsafe {
            *(*self.0).get_ivar("_frame")
        }
    }
}

impl Clone for SCDisplay {
    fn clone(&self) -> Self {
        unsafe { let _: () = msg_send![self.0, retain]; }
        SCDisplay(self.0)
    }
}

impl Drop for SCDisplay {
    fn drop(&mut self) {
        unsafe { let _: () = msg_send![self.0, release]; }
    }
}

#[repr(C)]
pub struct SCShareableContent(*mut Object);
unsafe impl Send for SCShareableContent {}
unsafe impl Sync for SCShareableContent {}

impl SCShareableContent {
    pub fn get_shareable_content_with_completion_handler(
        excluding_desktop_windows: bool,
        onscreen_windows_only: bool,
        completion_handler: impl Fn(Result<SCShareableContent, NSError>) + Send + 'static,
    ) {
        let completion_handler = Box::new(completion_handler);
        let handler_block = ConcreteBlock::new(move |sc_shareable_content: *mut Object, error: *mut Object| {
            if !error.is_null() {
                let error = NSError(error);
                (completion_handler)(Err(error));
            } else {
                unsafe { let _:() = msg_send![sc_shareable_content, retain]; }
                let sc_shareable_content = SCShareableContent(sc_shareable_content);
                (completion_handler)(Ok(sc_shareable_content));
            }
        }).copy();

        unsafe {
            let _: () = msg_send![
                class!(SCShareableContent),
                getShareableContentExcludingDesktopWindows: Bool::from_raw(excluding_desktop_windows)
                onScreenWindowsOnly: Bool::from_raw(onscreen_windows_only)
                completionHandler: handler_block
            ];
        }
    }

    pub fn windows(&self) -> Vec<SCWindow> {
        let mut windows = Vec::new();
        unsafe {
            let windows_nsarray_ref: NSArrayRef = *(*self.0).get_ivar("_windows");
            if !windows_nsarray_ref.is_null() {
                let windows_ns_array = NSArray::from_ref(windows_nsarray_ref);
                let count = windows_ns_array.count();
                for i in 0..count {
                    let window_id: *mut Object = windows_ns_array.obj_at_index(i);
                    windows.push(SCWindow::from_id_unretained(window_id));
                }
            }
        }
        windows
    }

    pub fn displays(&self) -> Vec<SCDisplay> {
        let mut displays = Vec::new();
        unsafe {
            let displays_ref: NSArrayRef = *(*self.0).get_ivar("_displays");
            if !displays_ref.is_null() {
                let displays_ns_array = NSArray::from_ref(displays_ref);
                let count = displays_ns_array.count();
                for i in 0..count {
                    let display_id: *mut Object = displays_ns_array.obj_at_index(i);
                    displays.push(SCDisplay::from_id_unretained(display_id));
                }
            }
        }
        displays
    }
}

impl Drop for SCShareableContent {
    fn drop(&mut self) {
        unsafe { let _: () = msg_send![self.0, release]; }
    }
}

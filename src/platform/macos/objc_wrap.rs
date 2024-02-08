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

use std::{ffi::CString, ops::{Add, Mul, Sub}};

use block::ConcreteBlock;
use libc::{c_void, strlen};
use objc::{class, declare::MethodImplementation, msg_send, runtime::{Object, Sel, BOOL}, sel, sel_impl, Encode, Encoding};
use objc2::runtime::Bool;

use crate::prelude::{StreamCreateError, StreamError, StreamEvent};

use lazy_static::lazy_static;
use parking_lot::Mutex;

type CFTypeRef = *const c_void;
type CFStringRef = CFTypeRef;
type CGColorRef = CFTypeRef;
type CMSampleBufferRef = CFTypeRef;
type CFAllocatorRef = CFTypeRef;
type OSStatus = i32;

extern "C" {

    static kCFAllocatorNull: CFTypeRef;

    fn CFRetain(x: CFTypeRef) -> CFTypeRef;
    fn CFRelease(x: CFTypeRef);

    fn CGColorGetConstantColor(color_name: CFStringRef) -> CGColorRef;

    static kCGColorBlack: CFStringRef;
    static kCGColorWhite: CFStringRef;
    static kCGColorClear: CFStringRef;

    fn CMTimeMake(value: i64, timescale: i32) -> CMTime;
    fn CMTimeMakeWithEpoch(value: i64, timescale: i32, epoch: i64) -> CMTime;
    fn CMTimeMakeWithSeconds(seconds: f64, preferred_timescale: i32) -> CMTime;
    fn CMTimeGetSeconds(time: CMTime) -> f64;

    fn CMTimeAdd(lhs: CMTime, rhs: CMTime) -> CMTime;
    fn CMTimeSubtract(lhs: CMTime, rhs: CMTime) -> CMTime;
    fn CMTimeMultiply(lhs: CMTime, rhs: i32) -> CMTime;
    fn CMTimeMultiplyByFloat64(time: CMTime, multiplier: f64) -> CMTime;
    fn CMTimeMultiplyByRatio(time: CMTime, multiplier: i32, divisor: i32) -> CMTime;
    fn CMTimeConvertScale(time: CMTime, new_timescale: i32, rounding_method: u32) -> CMTime;
    fn CMTimeCompare(time1: CMTime, time2: CMTime) -> i32;

    static kCMTimeInvalid: CMTime;
    static kCMTimeIndefinite: CMTime;
    static kCMTimePositiveInfinity: CMTime;
    static kCMTimeNegativeInfinity: CMTime;
    static kCMTimeZero: CMTime;

    fn CMSampleBufferCreateCopy(allocator: CFAllocatorRef, original: CMSampleBufferRef, new: *mut CMSampleBufferRef) -> OSStatus;
}

const SCSTREAM_ERROR_CODE_USER_STOPPED: isize = -3817;

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
    pub(crate) fn from_id_unretained(id: *mut Object) -> Self {
        unsafe { let _: () = msg_send![id, retain]; }
        Self(id)
    }

    pub(crate) fn from_id_retained(id: *mut Object) -> Self {
        Self(id)
    }

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
    pub (crate) fn new() -> Self {
        unsafe {
            let id: *mut Object = msg_send![class!(NSArray), new];
            Self::from_id_retained(id)
        }
    }

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

    pub fn raw_id(&self) -> u32 {
        unsafe {
            *(*self.0).get_ivar("_displayID")
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
                let error = NSError::from_id_retained(error);
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

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub(crate) struct OSType([u8; 4]);

unsafe impl Encode for OSType {
    fn encode() -> Encoding {
        unsafe { Encoding::from_str("I") }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub enum SCStreamPixelFormat {
    BGRA8888,
    L10R,
    V420,
    F420,
}

impl SCStreamPixelFormat {
    pub(crate) fn to_ostype(&self) -> OSType {
        match self {
            Self::BGRA8888 => OSType(['B' as u8, 'G' as u8, 'R' as u8, 'A' as u8]),
            Self::L10R     => OSType(['l' as u8, '1' as u8, '0' as u8, 'r' as u8]),
            Self::V420     => OSType(['4' as u8, '2' as u8, '0' as u8, 'v' as u8]),
            Self::F420     => OSType(['4' as u8, '2' as u8, '0' as u8, 'f' as u8]),
        }
    }
}


#[derive(Copy, Clone, Debug)]
pub enum SCStreamBackgroundColor {
    Black,
    White,
    Clear,
}

#[repr(C)]
pub struct SCStreamConfiguration(*mut Object);
unsafe impl Send for SCStreamConfiguration {}
unsafe impl Sync for SCStreamConfiguration {}

impl SCStreamConfiguration {
    pub fn new() -> Self {
        unsafe {
            let instance: *mut Object = msg_send![class!(SCStreamConfiguration), alloc];
            let instance: *mut Object = msg_send![instance, init];
            Self(instance)
        }
    }

    pub fn set_size(&mut self, size: CGSize) {
        let CGSize { x, y } = size;
        unsafe {
            let _: () = msg_send![self.0, setWidth: x];
            let _: () = msg_send![self.0, setHeight: y];
        }
    }

    pub fn set_source_rect(&mut self, source_rect: CGRect) {
        unsafe {
            let _: () = msg_send![self.0, setSourceRect: source_rect];
        }
    }

    pub fn set_scales_to_fit(&mut self, scale_to_fit: bool) {
        unsafe {
            let _: () = msg_send![self.0, setScalesToFit: scale_to_fit];
        }
    }

    pub fn set_pixel_format(&mut self, format: SCStreamPixelFormat) {
        unsafe {
            (*self.0).set_ivar("_pixelFormat", format.to_ostype());
        }
    }

    pub fn set_background_color(&mut self, bg_color: SCStreamBackgroundColor) {
        unsafe {
            let bg_color_name = match bg_color {
                SCStreamBackgroundColor::Black => kCGColorBlack,
                SCStreamBackgroundColor::White => kCGColorWhite,
                SCStreamBackgroundColor::Clear => kCGColorClear,
            };
            let bg_color = CGColorGetConstantColor(bg_color_name);
            (*self.0).set_ivar("_backgroundColor", bg_color);
        }
    }

    pub fn set_queue_depth(&mut self, queue_depth: isize) {
        unsafe {
            let _: () = msg_send![self.0, setQueueDepth: queue_depth];
        }
    }

    pub fn set_minimum_time_interval(&mut self, interval: CMTime) {
        unsafe {
            (*self.0).set_ivar("_minimumFrameInterval", interval);
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: SCStreamSampleRate) {
        unsafe {
            (*self.0).set_ivar("_sampleRate", sample_rate.to_isize());
        }
    }

    pub fn set_show_cursor(&mut self, show_cursor: bool) {
        unsafe {
            (*self.0).set_ivar("_showsCursor", show_cursor);
        }
    }

    pub fn set_capture_audio(&mut self, capture_audio: bool) {
        unsafe {
            (*self.0).set_ivar("_capturesAudio", BOOL::from(capture_audio));
        }
    }

    pub fn set_channel_count(&mut self, channel_count: isize) {
        unsafe {
            (*self.0).set_ivar("_channelCount", channel_count);
        }
    }

    pub fn set_exclude_current_process_audio(&mut self, exclude_current_process_audio: bool) {
        unsafe {
            (*self.0).set_ivar("_excludesCurrentProcessAudio", exclude_current_process_audio);
        }
    }

    pub fn set_preserves_aspect_ratio(&mut self, preserve_aspect_ratio: bool) {
        unsafe {
            (*self.0).set_ivar("_setpreservesAspectRatio", preserve_aspect_ratio);
        }
    }
}

impl Clone for SCStreamConfiguration {
    fn clone(&self) -> Self {
        unsafe { let _: () = msg_send![self.0, retain]; }
        SCStreamConfiguration(self.0)
    }
}

impl Drop for SCStreamConfiguration {
    fn drop(&mut self) {
        unsafe { let _: () = msg_send![self.0, release]; }
    }
}


#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct CMTime {
    value: i64,
    scale: i32,
    epoch: i64,
    flags: u32,
}

pub const K_CMTIME_FLAGS_VALID                   : u32 = 1 << 0;
pub const K_CMTIME_FLAGS_HAS_BEEN_ROUNDED        : u32 = 1 << 0;
pub const K_CMTIME_FLAGS_POSITIVE_INFINITY       : u32 = 1 << 0;
pub const K_CMTIME_FLAGS_NEGATIVE_INFINITY       : u32 = 1 << 0;
pub const K_CMTIME_FLAGS_INDEFINITE              : u32 = 1 << 0;
pub const K_CMTIME_FLAGS_IMPLIED_VALUE_FLAG_MASK : u32 = 
    K_CMTIME_FLAGS_VALID                    |
    K_CMTIME_FLAGS_HAS_BEEN_ROUNDED         |
    K_CMTIME_FLAGS_POSITIVE_INFINITY        |
    K_CMTIME_FLAGS_NEGATIVE_INFINITY        |
    K_CMTIME_FLAGS_INDEFINITE
    ;

const K_CMTIME_ROUNDING_METHOD_ROUND_HALF_AWAY_FROM_ZERO: u32 = 1;
const K_CMTIME_ROUNDING_METHOD_ROUND_TOWARD_ZERO: u32 = 2;
const K_CMTIME_ROUNDING_METHOD_ROUND_AWAY_FROM_ZERO: u32 = 3;
const K_CMTIME_ROUNDING_METHOD_QUICKTIME: u32 = 4;
const K_CMTIME_ROUNDING_METHOD_TOWARD_POSITIVE_INFINITY: u32 = 5;
const K_CMTIME_ROUNDING_METHOD_TOWARD_NEGATIVE_INFINITY: u32 = 6;

#[derive(Copy, Clone, Debug)]
pub enum CMTimeRoundingMethod {
    HalfAwayFromZero,
    TowardZero,
    AwayFromZero,
    QuickTime,
    TowardPositiveInfinity,
    TowardNegativeInfinity,
}

impl CMTimeRoundingMethod {
    pub(crate) fn to_u32(&self) -> u32 {
        match self {
            Self::HalfAwayFromZero       => K_CMTIME_ROUNDING_METHOD_ROUND_HALF_AWAY_FROM_ZERO,
            Self::TowardZero             => K_CMTIME_ROUNDING_METHOD_ROUND_TOWARD_ZERO,
            Self::AwayFromZero           => K_CMTIME_ROUNDING_METHOD_ROUND_AWAY_FROM_ZERO,
            Self::QuickTime              => K_CMTIME_ROUNDING_METHOD_QUICKTIME,
            Self::TowardPositiveInfinity => K_CMTIME_ROUNDING_METHOD_TOWARD_POSITIVE_INFINITY,
            Self::TowardNegativeInfinity => K_CMTIME_ROUNDING_METHOD_TOWARD_NEGATIVE_INFINITY,
        }
    }
}

impl Default for CMTimeRoundingMethod {
    fn default() -> Self {
        Self::HalfAwayFromZero
    }
}

impl Add for CMTime {
    type Output = CMTime;

    fn add(self, rhs: Self) -> Self {
        unsafe { CMTimeAdd(self, rhs) }
    }
}

impl Sub for CMTime {
    type Output = CMTime;

    fn sub(self, rhs: Self) -> Self {
        unsafe { CMTimeSubtract(self, rhs) }
    }
}

impl Mul<i32> for CMTime {
    type Output = Self;

    fn mul(self, rhs: i32) -> Self {
        unsafe { CMTimeMultiply(self, rhs) }
    }
}

impl Mul<f64> for CMTime {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self {
        unsafe { CMTimeMultiplyByFloat64(self, rhs) }
    }
}

impl PartialEq for CMTime {
    fn eq(&self, other: &Self) -> bool {
        unsafe {
            CMTimeCompare(*self, *other) == 0
        }
    }
}

impl PartialOrd for CMTime {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        unsafe {
            let self_op_other = CMTimeCompare(*self, *other);
            match self_op_other {
                -1 => Some(std::cmp::Ordering::Less),
                0 => Some(std::cmp::Ordering::Equal),
                1 => Some(std::cmp::Ordering::Greater),
                _ => None
            }
        }
    }
}

unsafe impl Encode for CMTime {
    fn encode() -> Encoding {
        unsafe { Encoding::from_str("{?=\"value\"q\"timescale\"i\"flags\"I\"epoch\"q}") }
    }
}

impl CMTime {
    pub fn new_with_seconds(seconds: f64, timescale: i32) -> Self {
        unsafe { CMTimeMakeWithSeconds(seconds, timescale) }
    }

    pub const fn is_valid(&self) -> bool {
        self.flags & K_CMTIME_FLAGS_VALID != 0
    }

    pub const fn is_invalid(&self) -> bool {
        ! self.is_valid()
    }

    pub const fn is_indefinite(&self) -> bool {
        self.is_valid() &&
        (self.flags & K_CMTIME_FLAGS_INDEFINITE != 0)
    }

    pub const fn is_positive_infinity(&self) -> bool {
        self.is_valid() &&
        (self.flags & K_CMTIME_FLAGS_POSITIVE_INFINITY != 0)
    }

    pub const fn is_negative_infinity(&self) -> bool {
        self.is_valid() &&
        (self.flags & K_CMTIME_FLAGS_NEGATIVE_INFINITY != 0)
    }

    pub const fn is_numeric(&self) -> bool {
        self.is_valid() &&
        ! self.is_indefinite() &&
        ! self.is_positive_infinity() &&
        ! self.is_negative_infinity()
    }

    pub const fn has_been_rounded(&self) -> bool {
        self.flags & K_CMTIME_FLAGS_HAS_BEEN_ROUNDED != 0
    }

    pub fn convert_timescale(&self, new_timescale: i32, rounding_method: CMTimeRoundingMethod) -> Self {
        unsafe { CMTimeConvertScale(*self, new_timescale, rounding_method.to_u32()) }
    }

    pub fn multiply_by_ratio(&self, multiplier: i32, divisor: i32) -> Self {
        unsafe { CMTimeMultiplyByRatio(*self, multiplier, divisor) }
    }

    pub fn seconds_f64(&self) -> f64 {
        unsafe { CMTimeGetSeconds(*self) }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum SCStreamSampleRate {
    R8000,
    R16000,
    R24000,
    R48000,
}

impl SCStreamSampleRate {
    pub(crate) fn to_isize(&self) -> isize {
        match self {
            Self::R8000  => 8000,
            Self::R16000 => 16000,
            Self::R24000 => 24000,
            Self::R48000 => 48000,
        }
    }
}

#[repr(C)]
pub struct SCContentFilter(*mut Object);

unsafe impl Send for SCContentFilter {}
unsafe impl Sync for SCContentFilter {}

impl SCContentFilter {
    pub fn new_with_desktop_independent_window(window: SCWindow) -> Self {
        unsafe {
            let id: *mut Object = msg_send![class!(SCContentFilter), alloc];
            let _: *mut Object = msg_send![id, initWithDesktopIndependentWindow: window.0];
            Self(id)
        }
    }

    pub fn new_with_display_excluding_windows(display: SCDisplay, excluding_windows: NSArray) -> Self {
        unsafe {
            let id: *mut Object = msg_send![class!(SCContentFilter), alloc];
            let _: *mut Object = msg_send![id, initWithDisplay: display.0 excludingWindows: excluding_windows.0];
            Self(id)
        }
    }
}

impl Clone for SCContentFilter {
    fn clone(&self) -> Self {
        unsafe { let _: () = msg_send![self.0, retain]; }
        SCContentFilter(self.0)
    }
}

impl Drop for SCContentFilter {
    fn drop(&mut self) {
        unsafe { let _: () = msg_send![self.0, release]; }
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct SCStreamHandler(*mut Object);

unsafe impl Encode for SCStreamHandler {
    fn encode() -> Encoding {
        unsafe { Encoding::from_str("@\"<SCStreamDelegate, SCStreamOutput>\"") }
    }
}

#[repr(C)]
struct SCStreamOutputTypeEncoded(isize);

unsafe impl Encode for SCStreamOutputTypeEncoded {
    fn encode() -> Encoding {
        unsafe { Encoding::from_str("{SCStreamOutputType}") }
    }
}

impl SCStreamOutputTypeEncoded {
    fn into_output_type(&self) -> Option<SCStreamOutputType> {
        match self.0 {
            0 => Some(SCStreamOutputType::Screen),
            1 => Some(SCStreamOutputType::Audio),
            _ => None
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum SCStreamOutputType {
    Audio,
    Screen,
}

impl SCStreamOutputType {
    pub fn to_isize(&self) -> isize {
        match *self {
            Self::Screen => 0,
            Self::Audio => 1,
        }
    }
}

pub enum SCStreamCallbackError {
    SampleBufferCopyFailed,
    StreamStopped,
    Other(NSError)
}

#[repr(C)]
struct SCStreamCallbackContainer {
    callback: Box<dyn FnMut(Result<(CMSampleBuffer, SCStreamOutputType), SCStreamCallbackError>) + Send + 'static>
}

struct SCStreamOutputDidOutputSampleBufferOfType;

impl MethodImplementation for SCStreamOutputDidOutputSampleBufferOfType {
    type Callee = Object;

    type Ret = ();

    type Args = (*mut Object, CMSampleBufferRef, SCStreamOutputTypeEncoded);

    fn imp(self) -> objc::runtime::Imp {
        unsafe { std::mem::transmute::<_, unsafe extern fn()>(sc_stream_output_did_output_sample_buffer_of_type as unsafe extern fn (_, _, _, _, _)) }
    }
}


unsafe extern fn sc_stream_output_did_output_sample_buffer_of_type(this: *mut Object, _sel: Sel, stream: *mut Object, buffer: CMSampleBufferRef, output_type: SCStreamOutputTypeEncoded) {
    unsafe {
        let callback_container_ptr: *mut c_void = *(*this).get_ivar("_callback_container_ptr");
        let mut callback_container = Box::from_raw(callback_container_ptr as *mut SCStreamCallbackContainer);
        if let Some(output_type) = output_type.into_output_type() {
            match CMSampleBuffer::copy_from_ref(buffer) {
                Ok(buffer) => (callback_container.callback)(Ok((buffer, output_type))),
                Err(()) => (callback_container.callback)(Err(SCStreamCallbackError::SampleBufferCopyFailed)),
            }
            
        }
        std::mem::forget(callback_container);
    }
}

struct SCStreamHandlerDidStopWithError;

impl MethodImplementation for SCStreamHandlerDidStopWithError {
    type Callee = Object;

    type Ret = ();

    type Args = (*mut Object, *mut Object);

    fn imp(self) -> objc::runtime::Imp {
       unsafe { std::mem::transmute::<_, unsafe extern "C" fn()>(sc_stream_handler_did_stop_with_error as extern "C" fn(_, _, _, _)) }
    }
}

extern "C" fn sc_stream_handler_did_stop_with_error(this: *mut Object, _sel: Sel, stream: *mut Object, error: *mut Object) -> () {
    unsafe {
        let callback_container_ptr: *mut c_void = *(*this).get_ivar("_callback_container_ptr");
        let mut callback_container = Box::from_raw(callback_container_ptr as *mut SCStreamCallbackContainer);
        let error = NSError::from_id_retained(error);
        if error.code() == SCSTREAM_ERROR_CODE_USER_STOPPED {
            (callback_container.callback)(Err(SCStreamCallbackError::StreamStopped));
        } else {
            (callback_container.callback)(Err(SCStreamCallbackError::Other(error)));
        }
        std::mem::forget(callback_container);
    }
}

struct SCStreamHandlerDealloc;

impl MethodImplementation for SCStreamHandlerDealloc {
    type Callee = Object;

    type Ret = ();

    type Args = ();

    fn imp(self) -> objc::runtime::Imp {
       unsafe { std::mem::transmute::<_, unsafe extern "C" fn()>(sc_stream_handler_dealloc as extern fn(_, _)) }
    }
}

extern "C" fn sc_stream_handler_dealloc(this: *mut Object, _sel: Sel) -> () {
    unsafe {
        let callback_container_ptr: *mut c_void = *(*this).get_ivar("_callback_container_ptr");
        let callback_container = Box::from_raw(callback_container_ptr as *mut SCStreamCallbackContainer);
        drop(callback_container);
    }
}

#[repr(C)]
pub struct SCStreamDelegate(*mut Object);

lazy_static! {
    static ref SCSTREAMDELEGATE_CLASS_INIT: Mutex<bool> = Mutex::new(false);
}

impl SCStreamDelegate {
    pub fn new(callback: impl FnMut(Result<(CMSampleBuffer, SCStreamOutputType), SCStreamCallbackError>) + 'static + Send) -> Self {
        Self::register_class();
        unsafe {
            let id: *mut Object = msg_send![class!(SCStreamHandler), alloc];
            let _: *mut Object = msg_send![class!(SCStreamHandler), init];
            let callback_container_ptr = Box::leak(Box::new(SCStreamCallbackContainer {
                callback: Box::new(callback)
            })) as *mut _ as *mut c_void;
            (*id).set_ivar("_callback_container_ptr", callback_container_ptr);
        }
        todo!()
    }

    fn register_class() {
        let mut init_flag = SCSTREAMDELEGATE_CLASS_INIT.lock();
        if *init_flag {
            return;
        }
        // init class
        if let Some(mut class) = objc::declare::ClassDecl::new("SCStreamHandler", class!(NSObject)) {
            unsafe {
                class.add_method(sel!(stream:didOutputSampleBuffer:ofType:), SCStreamOutputDidOutputSampleBufferOfType);
                class.add_method(sel!(stream:didStopWithError:), SCStreamHandlerDidStopWithError);
                class.add_method(sel!(dealloc), SCStreamHandlerDealloc);
                
                let sc_stream_delegate_name = CString::new("SCStreamDelegate").unwrap();
                class.add_protocol(&*objc::runtime::objc_getProtocol(sc_stream_delegate_name.as_ptr()));
                
                let sc_stream_output_name = CString::new("SCStreamOutput").unwrap();
                class.add_protocol(&*objc::runtime::objc_getProtocol(sc_stream_output_name.as_ptr()));

                class.add_ivar::<*mut c_void>("_callback_container_ptr");
                
                class.register();
            }
        }
    }
}


impl Clone for SCStreamDelegate {
    fn clone(&self) -> Self {
        unsafe { let _: () = msg_send![self.0, retain]; }
        SCStreamDelegate(self.0)
    }
}

impl Drop for SCStreamDelegate {
    fn drop(&mut self) {
        unsafe { let _: () = msg_send![self.0, release]; }
    }
}

#[repr(C)]
pub struct SCStream(*mut Object);

impl SCStream {
    pub fn new(filter: SCContentFilter, config: SCStreamConfiguration, delegate: SCStreamDelegate) -> Result<Self, StreamCreateError> {
        Err(StreamCreateError::Other("SCStream::new(...) unimplemented!".into()))
    }

    pub fn start(&mut self) -> Result<Self, StreamCreateError> {
        Err(StreamCreateError::Other("SCStream::start() unimplemented!".into()))
    }
}

unsafe impl Encode for SCStream {
    fn encode() -> Encoding {
        unsafe { Encoding::from_str("@") }
    }
}

impl Clone for SCStream {
    fn clone(&self) -> Self {
        unsafe { let _: () = msg_send![self.0, retain]; }
        SCStream(self.0)
    }
}

impl Drop for SCStream {
    fn drop(&mut self) {
        unsafe { let _: () = msg_send![self.0, release]; }
    }
}

pub struct CMSampleBuffer(CMSampleBufferRef);

impl CMSampleBuffer {
    pub fn copy_from_ref(r: CMSampleBufferRef) -> Result<Self, ()> {
        unsafe { 
            let mut new_ref: CMSampleBufferRef = std::ptr::null();
            let status = CMSampleBufferCreateCopy(kCFAllocatorNull, r, &mut new_ref as *mut _);
            if status != 0 {
                Err(())
            } else {
                Ok(CMSampleBuffer(new_ref))
            }
        }
    }
}

impl Clone for CMSampleBuffer {
    fn clone(&self) -> Self {
        unsafe { CFRetain(self.0); }
        Self(self.0)
    }
}

impl Drop for CMSampleBuffer {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0); }
    }
}

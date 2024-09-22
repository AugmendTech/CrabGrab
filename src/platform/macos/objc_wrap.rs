#![allow(unused)]
#![allow(non_upper_case_globals)]

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

use std::{cell::RefCell, collections::HashMap, ffi::CString, ops::{Add, Mul, Sub}, ptr::{addr_of_mut, null, null_mut, NonNull}, sync::Arc, time::{Duration, Instant}};

use block2::{ffi::Class, Block, RcBlock, StackBlock};
use libc::{c_void, strlen};
use objc2::{class, declare::ClassBuilder, ffi::{objc_getClass, objc_getProtocol}, msg_send, rc::Id, runtime::{AnyClass, AnyObject, AnyProtocol, Bool, Ivar, Sel}, sel, Encode, Encoding, RefEncode};
use mach2::mach_time::{mach_timebase_info, mach_timebase_info_data_t};

use crate::{prelude::{AudioSampleRate, StreamCreateError, StreamError, StreamEvent, StreamStopError}};

use lazy_static::lazy_static;
use parking_lot::Mutex;

use super::ImplPixelFormat;

type CFTypeRef = *const c_void;
type CFStringRef = CFTypeRef;
type CMSampleBufferRef = CFTypeRef;
type CFAllocatorRef = CFTypeRef;
type CFDictionaryRef = CFTypeRef;
type CMFormatDescriptionRef = CFTypeRef;
type CMBlockBufferRef = CFTypeRef;
type CFArrayRef = CFTypeRef;
type OSStatus = i32;
type CGDisplayStreamRef = CFTypeRef;
type CGDisplayStreamUpdateRef = CFTypeRef;
pub(crate) type IOSurfaceRef = CFTypeRef;
type CGDictionaryRef = CFTypeRef;
type CFBooleanRef = CFTypeRef;
type CFNumberRef = CFTypeRef;
type CVPixelBufferRef = CFTypeRef;
type CGImageRef = CFTypeRef;
type CGDataProviderRef = CFTypeRef;
type CFDataRef = CFTypeRef;

#[repr(C)]
struct CFStringRefEncoded(CFStringRef);

unsafe impl Encode for CFStringRefEncoded {
    const ENCODING: Encoding = Encoding::Pointer(&Encoding::Struct("__CFString", &[]));
}

extern "C" {

    static kCFAllocatorNull: CFTypeRef;
    static kCFAllocatorDefault: CFTypeRef;

    fn CFRetain(x: CFTypeRef) -> CFTypeRef;
    fn CFRelease(x: CFTypeRef);

    pub(crate) static kCFBooleanTrue: CFBooleanRef;
    pub(crate) static kCFBooleanFalse: CFBooleanRef;

    //CFNumberRef CFNumberCreate(CFAllocatorRef allocator, CFNumberType theType, const void *valuePtr);
    fn CFNumberCreate(allocator: CFAllocatorRef, the_type: isize, value_ptr: *const c_void) -> CFNumberRef;
    fn CFNumberGetValue(number: CFNumberRef, number_type: isize, value_ptr: *mut c_void) -> Bool;

    fn CGColorGetConstantColor(color_name: CFStringRef) -> CGColorRef;

    pub(crate) fn CGRectMakeWithDictionaryRepresentation(dictionary: CGDictionaryRef, rect: *mut CGRect) -> bool;

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
    fn CMSampleBufferIsValid(sbuf: CMSampleBufferRef) -> Bool;
    fn CMSampleBufferGetNumSamples(sbuf: CMSampleBufferRef) -> isize;
    fn CMSampleBufferGetPresentationTimeStamp(sbuf: CMSampleBufferRef) -> CMTime;
    fn CMSampleBufferGetDuration(sbuf: CMSampleBufferRef) -> CMTime;
    fn CMSampleBufferGetFormatDescription(sbuf: CMSampleBufferRef) -> CMFormatDescriptionRef;
    fn CMSampleBufferGetSampleAttachmentsArray(sbuf: CMSampleBufferRef, create_if_necessary: Bool) -> CFArrayRef;

    fn CMFormatDescriptionGetMediaType(fdesc: CMFormatDescriptionRef) -> OSType;
    fn CMAudioFormatDescriptionGetStreamBasicDescription(afdesc: CMFormatDescriptionRef) -> *const AudioStreamBasicDescription;
    fn CMSampleBufferGetAudioBufferListWithRetainedBlockBuffer(sbuf: CMSampleBufferRef, buffer_list_size_needed_out: *mut usize, buffer_list_out: *mut AudioBufferList, buffer_list_size: usize, block_buffer_structure_allocator: CFAllocatorRef, block_buffer_block_allocator: CFAllocatorRef, flags: u32, block_buffer_out: *mut CMBlockBufferRef) -> OSStatus;
    fn CMSampleBufferGetImageBuffer(sbuffer: CMSampleBufferRef) -> CVPixelBufferRef;
    fn CVPixelBufferGetIOSurface(pixel_buffer: CVPixelBufferRef) -> IOSurfaceRef;
    fn CVPixelBufferGetWidth(pixel_buffer: CVPixelBufferRef) -> usize;
    fn CVPixelBufferGetHeight(pixel_buffer: CVPixelBufferRef) -> usize;
    fn CVBufferRetain(buffer: CVPixelBufferRef) -> CVPixelBufferRef;
    fn CVBufferRelease(buffer: CVPixelBufferRef) -> CVPixelBufferRef;

    fn CFArrayCreateMutable(allocator: CFAllocatorRef, capacity: isize, callbacks: *const CFArrayCallBacks) -> CFArrayRef;
    fn CFArrayGetCount(array: CFArrayRef) -> i32;
    fn CFArrayGetValueAtIndex(array: CFArrayRef, index: i32) -> CFTypeRef;
    fn CFArrayAppendValue(array: CFArrayRef, value: *const c_void);

    fn CFStringCreateWithBytes(allocator: CFTypeRef, bytes: *const u8, byte_count: isize, encoding: u32, contains_byte_order_marker: bool) -> CFStringRef;

    fn CFDictionaryGetValue(dict: CFDictionaryRef, value: CFTypeRef) -> CFTypeRef;

    fn CGPreflightScreenCaptureAccess() -> bool;
    fn CGRequestScreenCaptureAccess() -> bool;

    //CGDisplayStreamRef CGDisplayStreamCreateWithDispatchQueue(CGDirectDisplayID display, size_t outputWidth, size_t outputHeight, int32_t pixelFormat, CFDictionaryRef properties, dispatch_queue_t queue, CGDisplayStreamFrameAvailableHandler handler);
    fn CGDisplayStreamCreateWithDispatchQueue(display_id: u32, output_width: usize, output_height: usize, pixel_format: i32, properties: CFDictionaryRef, dispatch_queue: *mut AnyObject, handler: *const c_void) -> CGDisplayStreamRef;
    fn CGDisplayStreamStart(stream: CGDisplayStreamRef) -> i32;
    fn CGDisplayStreamStop(stream: CGDisplayStreamRef) -> i32;

    pub(crate) fn CGMainDisplayID() -> u32;
    
    fn CGDisplayScreenSize(display: u32) -> CGSize;

    fn CGRectCreateDictionaryRepresentation(rect: CGRect) -> CFDictionaryRef;

    pub(crate) fn CGWindowListCreateImage(screen_bounds: CGRect, options: u32, window_id: u32, image_options: u32) -> CGImageRef;

    static kCGWindowLayer: CFStringRef;

    fn CGWindowListCreateDescriptionFromArray(window_array: CFArrayRef) -> CFArrayRef;

    fn CGImageRetain(image: CGImageRef);
    fn CGImageRelease(image: CGImageRef);
    fn CGImageGetWidth(image: CGImageRef) -> usize;
    fn CGImageGetHeight(image: CGImageRef) -> usize;
    fn CGImageGetDataProvider(image: CGImageRef) -> CGDataProviderRef;
    fn CGImageGetBitsPerComponent(image: CGImageRef) -> usize;
    fn CGImageGetBitsPerPixel(image: CGImageRef) -> usize;
    fn CGImageGetBytesPerRow(image: CGImageRef) -> usize;
    fn CGImageGetPixelFormatInfo(image: CGImageRef) -> u32;
    fn CGImageGetBitmapInfo(image: CGImageRef) -> u32;

    fn CGDataProviderRetain(data_provider: CGDataProviderRef);
    fn CGDataProviderRelease(data_provider: CGDataProviderRef);
    fn CGDataProviderCopyData(data_provider: CGDataProviderRef) -> CFDataRef;

    fn CGWindowLevelForKey(key: i32) -> i32;

    pub(crate) fn IOSurfaceIncrementUseCount(r: IOSurfaceRef);
    pub(crate) fn IOSurfaceDecrementUseCount(r: IOSurfaceRef);

    fn IOSurfaceGetPixelFormat(surface: IOSurfaceRef) -> OSType;
    fn IOSurfaceGetPlaneCount(surface: IOSurfaceRef) -> usize;

    fn IOSurfaceGetWidth(surface: IOSurfaceRef) -> usize;
    fn IOSurfaceGetHeight(surface: IOSurfaceRef) -> usize;

    fn IOSurfaceLock(surface: IOSurfaceRef, options: u32, seed: *mut u32) -> i32;
    fn IOSurfaceUnlock(surface: IOSurfaceRef, options: u32, seed: *mut u32) -> i32;

    fn IOSurfaceGetBaseAddressOfPlane(surface: IOSurfaceRef, plane: usize) -> *mut c_void;
    fn IOSurfaceGetBaseAddress(surface: IOSurfaceRef) -> *mut c_void;

    fn IOSurfaceGetBytesPerRow(surface: IOSurfaceRef) -> usize;
    fn IOSurfaceGetBytesPerRowOfPlane(surface: IOSurfaceRef, plane: usize) -> usize;

    fn IOSurfaceGetHeightOfPlane(surface: IOSurfaceRef, plane: usize) -> usize;
    fn IOSurfaceGetWidthOfPlane(surface: IOSurfaceRef, plane: usize) -> usize;
    
    fn IOSurfaceGetElementWidthOfPlane(surface: IOSurfaceRef, plane: usize) -> usize;
    fn IOSurfaceGetBytesPerElementOfPlane(surface: IOSurfaceRef, plane: usize) -> usize;
    fn IOSurfaceGetNumberOfComponentsOfPlane(surface: IOSurfaceRef, plane: usize) -> usize;
    fn IOSurfaceGetNameOfComponentOfPlane(surface: IOSurfaceRef, plane: usize, component: usize) -> i32;
    fn IOSurfaceGetTypeOfComponentOfPlane(surface: IOSurfaceRef, plane: usize, component: usize) -> i32;
    fn IOSurfaceGetBitDepthOfComponentOfPlane(surface: IOSurfaceRef, plane: usize, component: usize) -> usize;
    fn IOSurfaceGetBitOffsetOfComponentOfPlane(surface: IOSurfaceRef, plane: usize, component: usize) -> usize;
    fn IOSurfaceCopyValue(surface: IOSurfaceRef, value: CFStringRef) -> *mut c_void;

    static mut _dispatch_queue_attr_concurrent: c_void;

    fn dispatch_queue_create(label: *const std::ffi::c_char, attr: DispatchQueueAttr) -> DispatchQueue;
    fn dispatch_retain(AnyObject: *mut AnyObject);
    fn dispatch_release(AnyObject: *mut AnyObject);

    pub(crate) static SCStreamFrameInfoStatus       : CFStringRef;
    pub(crate) static SCStreamFrameInfoDisplayTime  : CFStringRef;
    pub(crate) static SCStreamFrameInfoScaleFactor  : CFStringRef;
    pub(crate) static SCStreamFrameInfoContentScale : CFStringRef;
    pub(crate) static SCStreamFrameInfoContentRect  : CFStringRef;
    pub(crate) static SCStreamFrameInfoBoundingRect : CFStringRef;
    pub(crate) static SCStreamFrameInfoScreenRect   : CFStringRef;
    pub(crate) static SCStreamFrameInfoDirtyRects   : CFStringRef;

    pub(crate) static kCGDisplayStreamSourceRect          : CFStringRef;
    pub(crate) static kCGDisplayStreamDestinationRect     : CFStringRef;
    pub(crate) static kCGDisplayStreamPreserveAspectRatio : CFStringRef;
    pub(crate) static kCGDisplayStreamColorSpace          : CFStringRef;
    pub(crate) static kCGDisplayStreamMinimumFrameTime    : CFStringRef;
    pub(crate) static kCGDisplayStreamShowCursor          : CFStringRef;
    pub(crate) static kCGDisplayStreamQueueDepth          : CFStringRef;
    pub(crate) static kCGDisplayStreamYCbCrMatrix         : CFStringRef;

    static kCGDisplayStreamYCbCrMatrix_ITU_R_709_2     : CFStringRef;
    static kCGDisplayStreamYCbCrMatrix_ITU_R_601_4     : CFStringRef;
    static kCGDisplayStreamYCbCrMatrix_SMPTE_240M_1995 : CFStringRef;

    static NSDeviceSize: CFStringRef;

    pub(crate) static CGRectNull     : CGRect;
    pub(crate) static CGRectInfinite : CGRect;

    pub(crate) static kIOSurfaceCacheMode: CFStringRef;
}

const SCSTREAM_ERROR_CODE_USER_STOPPED: isize = -3817;

pub const kAudioFormatFlagIsFloat          : u32 = 1 << 0;
pub const kAudioFormatFlagIsBigEndian     : u32 = 1 << 1;
pub const kAudioFormatFlagIsPacked         : u32 = 1 << 3;
pub const kAudioFormatFlagIsNonInterleaved : u32 = 1 << 5;
#[cfg(target_endian = "big")]
pub const kAudioFormatNativeEndian         : u32 = kAudioFormatFlagIsBigEndian;
#[cfg(target_endian = "little")]
pub const kAudioFormatNativeEndian         : u32 = 0;

pub const kAudioFormatFlagsCanonical       : u32 = kAudioFormatFlagIsFloat | kAudioFormatFlagIsPacked | kAudioFormatNativeEndian;

pub const kCMSampleBufferFlag_AudioBufferList_Assure16ByteAlignment: u32 = 1 << 0;

pub const kCMSampleBufferError_ArrayTooSmall: i32 = -12737;

pub const kCFStringEncodingUTF8: u32 = 0x08000100;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub(crate) struct CGColorRef(CFTypeRef);

unsafe impl Encode for CGColorRef {
    const ENCODING: Encoding = Encoding::Pointer(&c_void::ENCODING_REF);
}

#[repr(C)]
pub(crate) struct NSString(pub(crate) *mut AnyObject);

unsafe impl Encode for NSString {
    const ENCODING: Encoding = Encoding::Object;
}

unsafe impl RefEncode for NSString {
    const ENCODING_REF: Encoding = Self::ENCODING;
}

impl NSString {
    pub(crate) fn new(s: &str) -> Self {
        unsafe {
            let bytes = s.as_bytes();
            let instance = CFStringCreateWithBytes(std::ptr::null(), bytes.as_ptr(), bytes.len() as isize, kCFStringEncodingUTF8, false);
            NSString(instance as *mut AnyObject)
        }
    }

    pub(crate) fn from_ref_unretained(r: CFStringRef) -> Self {
        unsafe { CFRetain(r); }
        Self(r as *mut AnyObject)
    }

    pub(crate) fn from_ref_retained(r: CFStringRef) -> Self {
        Self(r as *mut AnyObject)
    }

    pub(crate) fn from_id_unretained(id: *mut AnyObject) -> Self {
        unsafe {
            let _: *mut AnyObject = msg_send![id, retain];
            Self(id)
        }
    }

    pub(crate) fn as_string(&self) -> String {
        unsafe {
            let c_str: *const i8 = msg_send![self.0, UTF8String];
            let len = strlen(c_str);
            let bytes = std::slice::from_raw_parts(c_str as *const u8, len);
            String::from_utf8_lossy(bytes).into_owned()
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub(crate) struct NSError(*mut AnyObject);
unsafe impl Send for NSError {}

unsafe impl Encode for NSError {
    const ENCODING: Encoding = Encoding::Object;
}

impl NSError {
    pub(crate) fn from_id_unretained(id: *mut AnyObject) -> Self {
        unsafe { let _: *mut AnyObject = msg_send![id, retain]; }
        Self(id)
    }

    pub(crate) fn from_id_retained(id: *mut AnyObject) -> Self {
        Self(id)
    }

    /*pub(crate) fn new_with_domain_code_userinfo(domain: NSString, code: u32, userinfo: NSDictionary) -> Self {

    }*/

    pub(crate) fn code(&self) -> isize {
        unsafe { msg_send![self.0, code] }
    }

    pub(crate) fn domain(&self) -> String {
        unsafe {
            let domain_cfstringref: CFStringRef = msg_send![self.0, domain];
            NSString::from_ref_retained(domain_cfstringref).as_string()
        }
    }

    pub fn description(&self) -> String {
        unsafe { NSString::from_id_unretained(msg_send![self.0, localizedDescription]).as_string() }
    }

    pub fn reason(&self) -> String {
        unsafe { NSString::from_id_unretained(msg_send![self.0, localizedFailureReason]).as_string() }
    }

    //pub(crate) fn user_info(&self) -> 
}

impl Drop for NSError {
    fn drop(&mut self) {
        unsafe { let _: () = msg_send![self.0, release]; };
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub(crate) struct NSArrayRef(*mut AnyObject);

impl NSArrayRef {
    pub(crate) fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

#[repr(C)]
pub(crate) struct NSArray(*mut AnyObject);

impl NSArray {
    pub(crate) fn new() -> Self {
        unsafe {
            let id: *mut AnyObject = msg_send![class!(NSArray), new];
            Self::from_id_retained(id)
        }
    }

    pub(crate) fn new_mutable() -> Self {
        unsafe {
            let id: *mut AnyObject = msg_send![class!(NSMutableArray), alloc];
            let id: *mut AnyObject = msg_send![id, init];
            Self::from_id_retained(id)
        }
    }

    pub(crate) fn from_ref(r: NSArrayRef) -> Self {
        Self::from_id_unretained(r.0)
    }

    pub(crate) fn from_id_unretained(id: *mut AnyObject) -> Self {
        unsafe { let _: *mut AnyObject = msg_send![id, retain]; }
        Self(id)
    }

    pub(crate) fn from_id_retained(id: *mut AnyObject) -> Self {
        Self(id)
    }

    pub(crate) fn count(&self) -> usize {
        unsafe { msg_send![self.0, count] }
    }

    pub(crate) fn add_object<T: 'static + Encode>(&mut self, object: T) {
        unsafe {
            let _: () = msg_send![self.0, addObject: object];
        }
    }

    pub(crate) fn obj_at_index<T: 'static + Encode>(&self, i: usize) -> T {
        unsafe { msg_send![self.0, objectAtIndex: i] }
    }
}

impl Drop for NSArray {
    fn drop(&mut self) {
        unsafe { let _: () = msg_send![self.0, release]; }
    }
}


#[repr(C)]
#[derive(Copy, Clone)]
pub(crate) struct NSDictionaryEncoded(*mut AnyObject);

impl NSDictionaryEncoded {
    pub(crate) fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

#[repr(C)]
pub(crate) struct NSDictionary(pub(crate) *mut AnyObject);

impl NSDictionary {
    pub(crate) fn new() -> Self {
        unsafe {
            let id: *mut AnyObject = msg_send![class!(NSDictionary), new];
            Self::from_id_retained(id)
        }
    }

    pub(crate) fn new_mutable() -> Self {
        unsafe {
            let id: *mut AnyObject = msg_send![class!(NSMutableDictionary), new];
            Self::from_id_retained(id)
        }
    }

    pub(crate) fn from_ref_unretained(r: CGDictionaryRef) -> Self {
        Self::from_id_unretained(r as *mut AnyObject)
    }

    pub(crate) fn from_encoded(e: NSDictionaryEncoded) -> Self {
        Self::from_id_unretained(e.0)
    }

    pub(crate) fn from_id_unretained(id: *mut AnyObject) -> Self {
        unsafe { let _: *mut AnyObject = msg_send![id, retain]; }
        Self(id)
    }

    pub(crate) fn from_id_retained(id: *mut AnyObject) -> Self {
        Self(id)
    }

    pub(crate) fn count(&self) -> usize {
        unsafe { msg_send![self.0, count] }
    }

    pub(crate) fn all_keys(&self) -> NSArray {
        unsafe {
            let keys: *mut AnyObject = msg_send![self.0, allKeys];
            NSArray::from_id_retained(keys)
        }
    }

    pub(crate) fn value_for_key(&self, key: CFStringRef) -> *mut AnyObject {
        unsafe {
            msg_send![self.0, valueForKey: key as *mut NSString]
        }
    }

    pub(crate) fn set_object_for_key(&mut self, object: *mut AnyObject, key: *mut AnyObject) {
        unsafe {
            let _: () = msg_send![self.0, setAnyObject: object, forKey: key];
        }
    }
}

impl Drop for NSDictionary {
    fn drop(&mut self) {
        unsafe { let _: () = msg_send![self.0, release]; }
    }
}

impl Clone for NSDictionary {
    fn clone(&self) -> Self {
        Self::from_id_unretained(self.0)
    }
}

type CGFloat = f64;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct CGPoint {
    pub(crate) x: CGFloat,
    pub(crate) y: CGFloat,
}

unsafe impl Encode for CGPoint {
    const ENCODING: Encoding = Encoding::Struct("CGPoint", &[Encoding::Double, Encoding::Double]);
}

impl CGPoint {
    pub(crate) const ZERO: CGPoint = CGPoint { x: 0.0, y: 0.0 };
    pub(crate) const INF: CGPoint = CGPoint { x: std::f64::INFINITY, y: std::f64::INFINITY };
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct CGSize {
    pub(crate) x: CGFloat,
    pub(crate) y: CGFloat,
}

unsafe impl Encode for CGSize {
    const ENCODING: Encoding = Encoding::Struct("CGSize", &[Encoding::Double, Encoding::Double]);
}

impl CGSize {
    pub(crate) const ZERO: CGSize = CGSize { x: 0.0, y: 0.0 };
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct CGRect {
    pub(crate) origin: CGPoint,
    pub(crate) size: CGSize,
}

unsafe impl Encode for CGRect {
    const ENCODING: Encoding = Encoding::Struct("CGRect", &[CGPoint::ENCODING, CGSize::ENCODING]);
}

impl CGRect {
    pub(crate) const ZERO: CGRect = CGRect {
        origin: CGPoint::ZERO,
        size: CGSize::ZERO
    };

    pub(crate) const NULL: CGRect = CGRect {
        origin: CGPoint::INF,
        size: CGSize::ZERO
    };

    pub(crate) fn contains(&self, p: CGPoint) -> bool {
        p.x >= self.origin.x &&
        p.y >= self.origin.y &&
        p.x <= (self.origin.x + self.size.x) &&
        p.y <= (self.origin.y + self.size.y)
    }

    pub(crate) fn create_dicitonary_representation(&self) -> NSDictionary {
        unsafe {
            NSDictionary::from_ref_unretained(CGRectCreateDictionaryRepresentation(*self))
        }
    }

    pub(crate) fn create_from_dictionary_representation(dictionary: &NSDictionary) -> Self {
        unsafe {
            let mut rect = CGRect::default();
            CGRectMakeWithDictionaryRepresentation(dictionary.0 as *const c_void, &mut rect as *mut _);
            return rect;
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) struct CGWindowID(pub(crate) u32);

impl CGWindowID {
    pub(crate) fn raw(&self) -> u32 {
        self.0
    }
}

unsafe impl Send for CGWindowID {}

#[repr(C)]
pub(crate) struct SCWindow(*mut AnyObject);
unsafe impl Send for SCWindow {}

unsafe impl Encode for SCWindow {
    const ENCODING: Encoding = Encoding::Object;
}

impl SCWindow {
    pub(crate) fn from_id_unretained(id: *mut AnyObject) -> Self {
        unsafe { let _: *mut AnyObject = msg_send![id, retain]; }
        Self(id)
    }

    pub(crate) fn from_id_retained(id: *mut AnyObject) -> Self {
        Self(id)
    }

    pub(crate) fn id(&self) -> CGWindowID {
        unsafe { 
            let id: u32 = msg_send![self.0, windowID];
            CGWindowID(id)
        }
    }

    pub(crate) fn title(&self) -> String {
        unsafe {
            let title_nsstring: *mut AnyObject = msg_send![self.0, title];
            NSString::from_id_unretained(title_nsstring).as_string()
        }
    }

    pub(crate) fn frame(&self) -> CGRect {
        unsafe {
            // This ugly hack is necessary because the obc2 encoding for CGRect doesn't have field names like CoreGraphic's internal CGRect
            let offset = (*self.0).class().instance_variable("_frame").unwrap().offset();
            let raw_self_ptr = self.0 as *const c_void;
            let raw_frame_ptr = raw_self_ptr.byte_add(offset as usize);
            let frame_ptr = raw_frame_ptr as *const CGRect;
            *frame_ptr
        }
    }

    pub(crate) fn owning_application(&self) -> SCRunningApplication {
        unsafe {
            let scra_id: *mut AnyObject = msg_send![self.0, owningApplication];
            SCRunningApplication::from_id_unretained(scra_id)
        }
    }

    pub(crate) fn on_screen(&self) -> bool {
        unsafe {
            msg_send![self.0, onScreen]
        }
    }
}

impl Clone for SCWindow {
    fn clone(&self) -> Self {
        unsafe { let _: *mut AnyObject = msg_send![self.0, retain]; }
        SCWindow(self.0)
    }
}

impl Drop for SCWindow {
    fn drop(&mut self) {
        unsafe { let _: () = msg_send![self.0, release]; }
    }
}

#[repr(C)]
pub(crate) struct SCDisplay(*mut AnyObject);
unsafe impl Send for SCDisplay {}

impl SCDisplay {
    pub(crate) fn from_id_unretained(id: *mut AnyObject) -> Self {
        unsafe { let _: *mut AnyObject = msg_send![id, retain]; }
        Self(id)
    }

    pub(crate) fn from_id_retained(id: *mut AnyObject) -> Self {
        Self(id)
    }

    pub(crate) fn frame(&self) -> CGRect {
        unsafe {
            let offset = (*self.0).class().instance_variable("_frame").unwrap().offset();
            let raw_self_ptr = self.0 as *const c_void;
            let raw_frame_ptr = raw_self_ptr.byte_add(offset as usize);
            let frame_ptr = raw_frame_ptr as *const CGRect;
            *frame_ptr
        }
    }

    pub(crate) fn raw_id(&self) -> u32 {
        unsafe {
            *(*self.0).class().instance_variable("_displayID").unwrap().load(&*self.0)
        }
    }
}

impl Clone for SCDisplay {
    fn clone(&self) -> Self {
        unsafe { let _: *mut AnyObject = msg_send![self.0, retain]; }
        SCDisplay(self.0)
    }
}

impl Drop for SCDisplay {
    fn drop(&mut self) {
        unsafe { let _: () = msg_send![self.0, release]; }
    }
}

#[repr(C)]
pub(crate) struct SCShareableContent(*mut AnyObject);
unsafe impl Send for SCShareableContent {}
unsafe impl Sync for SCShareableContent {}

impl SCShareableContent {
    pub(crate) fn get_shareable_content_with_completion_handler(
        excluding_desktop_windows: bool,
        onscreen_windows_only: bool,
        completion_handler: impl Fn(Result<SCShareableContent, NSError>) + Send + 'static,
    ) {
        let completion_handler = Arc::new(completion_handler);
        let handler_block = RcBlock::new(move |sc_shareable_content: Option<NonNull<AnyObject>>, error: Option<NonNull<AnyObject>>| {
            unsafe {
                if let Some(mut error) = error {
                    (completion_handler)(Err(NSError::from_id_unretained(error.as_mut())));
                } else {
                    let mut sc_shareable_content = sc_shareable_content.unwrap();
                    unsafe { let _: *mut AnyObject = msg_send![sc_shareable_content, retain]; }
                    let sc_shareable_content = SCShareableContent(sc_shareable_content.as_mut());
                    (completion_handler)(Ok(sc_shareable_content));
                }
            }
        });
        unsafe {
            let _: () = msg_send![
                class!(SCShareableContent),
                getShareableContentExcludingDesktopWindows: Bool::new(excluding_desktop_windows)
                onScreenWindowsOnly: Bool::new(onscreen_windows_only)
                completionHandler: &*handler_block
            ];
        }
    }

    pub(crate) fn windows(&self) -> Vec<SCWindow> {
        let mut windows = Vec::new();
        unsafe {
            let windows_ivar = class!(SCShareableContent).instance_variable("_windows").expect("Expected _windows ivar on SCShareableContent");
            let windows_nsarray_ref = NSArrayRef(*windows_ivar.load_mut(&mut *self.0));
            if !windows_nsarray_ref.is_null() {
                let windows_ns_array = NSArray::from_ref(windows_nsarray_ref);
                let count = windows_ns_array.count();
                for i in 0..count {
                    let window_id: *mut AnyObject = windows_ns_array.obj_at_index(i);
                    windows.push(SCWindow::from_id_unretained(window_id));
                }
            }
        }
        windows
    }

    pub(crate) fn displays(&self) -> Vec<SCDisplay> {
        let mut displays = Vec::new();
        unsafe {
            let displays_ivar = class!(SCShareableContent).instance_variable("_displays").expect("Expected _displays ivar on SCShareableContent");
            let displays_ref = NSArrayRef(*displays_ivar.load_mut(&mut *self.0));
            if !displays_ref.is_null() {
                let displays_ns_array = NSArray::from_ref(displays_ref);
                let count = displays_ns_array.count();
                for i in 0..count {
                    let display_id: *mut AnyObject = displays_ns_array.obj_at_index(i);
                    displays.push(SCDisplay::from_id_unretained(display_id));
                }
            }
        }
        displays
    }

    pub(crate) fn applications(&self) -> Vec<SCRunningApplication> {
        let mut applications = Vec::new();
        unsafe {
            let applications_ivar = class!(SCShareableContent).instance_variable("_applications").expect("Expected _applications ivar on SCShareableContent");
            let applicaitons_ref = NSArrayRef(*applications_ivar.load_mut(&mut *self.0));
            if !applicaitons_ref.is_null() {
                let applications_array = NSArray::from_ref(applicaitons_ref);
                let count = applications_array.count();
                for i in 0..count {
                    let application_id: *mut AnyObject = applications_array.obj_at_index(i);
                    applications.push(SCRunningApplication::from_id_unretained(application_id));
                }
            }
        }
        applications
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

impl OSType {
    pub fn as_i32(&self) -> i32 {
        unsafe { std::mem::transmute(self.0) }
    }

    pub fn as_u32(&self) -> u32 {
        unsafe { std::mem::transmute(self.0) }
    }
}

unsafe impl Encode for OSType {
    const ENCODING: Encoding = Encoding::UInt;
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub(crate) enum SCStreamPixelFormat {
    BGRA8888,
    L10R,
    V420,
    F420,
}

impl SCStreamPixelFormat {
    pub(crate) fn to_ostype(&self) -> OSType {
        match self {
            Self::BGRA8888 => OSType(['A' as u8, 'R' as u8, 'G' as u8, 'B' as u8]),
            Self::L10R     => OSType(['r' as u8, '0' as u8, '1' as u8, 'l' as u8]),
            Self::V420     => OSType(['v' as u8, '0' as u8, '2' as u8, '4' as u8]),
            Self::F420     => OSType(['f' as u8, '0' as u8, '2' as u8, '4' as u8]),
        }
    }
}


#[derive(Copy, Clone, Debug)]
pub(crate) enum SCStreamBackgroundColor {
    Black,
    White,
    Clear,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum SCStreamColorMatrix {
    ItuR709_2,
    ItuR601_4,
    Smpte240M1995,
}

impl SCStreamColorMatrix {
    pub(crate) fn to_cfstringref(&self) -> CFStringRef {
        unsafe {
            match self {
                Self::ItuR709_2 => kCGDisplayStreamYCbCrMatrix_ITU_R_709_2,
                Self::ItuR601_4 => kCGDisplayStreamYCbCrMatrix_ITU_R_709_2,
                Self::Smpte240M1995 => kCGDisplayStreamYCbCrMatrix_SMPTE_240M_1995,
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SCCaptureResolutionType {
    SCCaptureResolutionAutomatic = 0,
    SCCaptureResolutionBest      = 1,
    SCCaptureResolutionNominal   = 2,
}

impl SCCaptureResolutionType {
    fn to_isize(&self) -> isize {
        *self as isize
    }
}

#[repr(C)]
pub(crate) struct SCStreamConfiguration(pub(crate) *mut AnyObject);
unsafe impl Send for SCStreamConfiguration {}
unsafe impl Sync for SCStreamConfiguration {}

impl SCStreamConfiguration {
    pub(crate) fn new() -> Self {
        unsafe {
            let instance: *mut AnyObject = msg_send![class!(SCStreamConfiguration), alloc];
            let instance: *mut AnyObject = msg_send![instance, init];
            Self(instance)
        }
    }

    pub(crate) fn set_size(&mut self, size: CGSize) {
        let dest_rect = CGRect {
            size,
            origin: CGPoint::ZERO,
        };
        unsafe {
            //let _: () = msg_send![self.0, setDestinationRect: dest_rect];
            let _: () = msg_send![self.0, setWidth: size.x as usize];
            let _: () = msg_send![self.0, setHeight: size.y as usize];
        }
    }

    pub(crate) fn set_source_rect(&mut self, source_rect: CGRect) {
        unsafe {
            let _: () = msg_send![self.0, setSourceRect: source_rect];
        }
    }

    pub(crate) fn set_scales_to_fit(&mut self, scale_to_fit: bool) {
        unsafe {
            let _: () = msg_send![self.0, setScalesToFit: Bool::new(scale_to_fit)];
        }
    }

    pub(crate) fn set_pixel_format(&mut self, format: SCStreamPixelFormat) {
        unsafe {
            let pixelformat_ivar = class!(SCStreamConfiguration).instance_variable("_pixelFormat").expect("_pixelFormat ivar on SCStreamConfiguration");
            *pixelformat_ivar.load_mut(&mut *self.0) = format.to_ostype();
        }
    }

    pub(crate) fn set_color_matrix(&mut self, color_matrix: SCStreamColorMatrix) {
        unsafe {
            let _: () = msg_send![self.0, setColorMatrix: CFStringRefEncoded(color_matrix.to_cfstringref())];
        }
    }

    pub(crate) fn set_resolution_type(&mut self, resolution_type: SCCaptureResolutionType) -> Result<(), ()> {
        unsafe {
            let has_property: Bool = msg_send![self.0, respondsToSelector: sel!(setCaptureResolution:)];
            if !has_property.as_bool() {
                return Err(())
            } else {
                let _: () = msg_send![self.0, setCaptureResolution: resolution_type.to_isize()];
                Ok(())
            }
        }
    }

    pub(crate) fn set_background_color(&mut self, bg_color: SCStreamBackgroundColor) {
        unsafe {
            let bg_color_name = match bg_color {
                SCStreamBackgroundColor::Black => kCGColorBlack,
                SCStreamBackgroundColor::White => kCGColorWhite,
                SCStreamBackgroundColor::Clear => kCGColorClear,
            };
            let bg_color = CGColorGetConstantColor(bg_color_name);
            let bg_color_ivar = class!(SCStreamConfiguration).instance_variable("_backgroundColor").expect("_backgroundColor ivar on SCStreamConfiguration");
            *bg_color_ivar.load_mut(&mut *self.0) = bg_color;
        }
    }

    pub(crate) fn set_queue_depth(&mut self, queue_depth: isize) {
        unsafe {
            let _: () = msg_send![self.0, setQueueDepth: queue_depth];
        }
    }

    pub(crate) fn set_minimum_time_interval(&mut self, interval: CMTime) {
        unsafe {
            let minimum_frame_interval_ivar = class!(SCStreamConfiguration).instance_variable("_minimumFrameInterval").expect("_minimumFrameInterval ivar on SCStreamConfiguration");
            let offset = minimum_frame_interval_ivar.offset();
            let raw_self_ptr = self.0 as *mut c_void;
            let raw_frame_ptr = raw_self_ptr.byte_add(offset as usize);
            let frame_ptr = raw_frame_ptr as *mut CMTime;
            *frame_ptr = interval;
        }
    }

    pub(crate) fn set_sample_rate(&mut self, sample_rate: SCStreamSampleRate) {
        unsafe {
            let sample_rate_ivar = class!(SCStreamConfiguration).instance_variable("_sampleRate").expect("_sampleRate ivar on SCStreamConfiguration");
            *sample_rate_ivar.load_mut(&mut *self.0) = sample_rate.to_isize();
        }
    }

    pub(crate) fn set_show_cursor(&mut self, show_cursor: bool) {
        unsafe {
            let show_cursor_ivar = class!(SCStreamConfiguration).instance_variable("_showsCursor").expect("_showsCursor ivar on SCStreamConfiguration");
            *show_cursor_ivar.load_mut(&mut *self.0) = Bool::new(show_cursor);
        }
    }

    pub(crate) fn set_capture_audio(&mut self, capture_audio: bool) {
        unsafe {
            let captures_audio_ivar = class!(SCStreamConfiguration).instance_variable("_capturesAudio").expect("_capturesAudio ivar on SCStreamConfiguration");
            *captures_audio_ivar.load_mut(&mut *self.0) = Bool::new(capture_audio);
        }
    }

    pub(crate) fn set_channel_count(&mut self, channel_count: isize) {
        unsafe {
            let channel_count_ivar = class!(SCStreamConfiguration).instance_variable("_channelCount").expect("_channelCount ivar on SCStreamConfiguration");
            *channel_count_ivar.load_mut(&mut *self.0) = channel_count
        }
    }

    pub(crate) fn set_exclude_current_process_audio(&mut self, exclude_current_process_audio: bool) {
        unsafe {
            let exclude_current_process_audio_ivar = class!(SCStreamConfiguration).instance_variable("_excludesCurrentProcessAudio").expect("_excludesCurrentProcessAudio ivar on SCStreamConfiguration");
            *exclude_current_process_audio_ivar.load_mut(&mut *self.0) = Bool::new(exclude_current_process_audio);
        }
    }
}

impl Clone for SCStreamConfiguration {
    fn clone(&self) -> Self {
        unsafe { let _: *mut AnyObject = msg_send![self.0, retain]; }
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
pub(crate) struct CMTime {
    value: i64,
    scale: i32,
    flags: u32,
    epoch: i64,
}

unsafe impl Encode for CMTime {
    const ENCODING: Encoding = Encoding::Struct("?", &[
        Encoding::LongLong,
        Encoding::Int,
        Encoding::UInt,
        Encoding::LongLong,
    ]);
}

pub(crate) const K_CMTIME_FLAGS_VALID                   : u32 = 1 << 0;
pub(crate) const K_CMTIME_FLAGS_HAS_BEEN_ROUNDED        : u32 = 1 << 0;
pub(crate) const K_CMTIME_FLAGS_POSITIVE_INFINITY       : u32 = 1 << 0;
pub(crate) const K_CMTIME_FLAGS_NEGATIVE_INFINITY       : u32 = 1 << 0;
pub(crate) const K_CMTIME_FLAGS_INDEFINITE              : u32 = 1 << 0;
pub(crate) const K_CMTIME_FLAGS_IMPLIED_VALUE_FLAG_MASK : u32 = 
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
pub(crate) enum CMTimeRoundingMethod {
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

impl CMTime {
    pub(crate) fn new_with_seconds(seconds: f64, timescale: i32) -> Self {
        unsafe { CMTimeMakeWithSeconds(seconds, timescale) }
    }

    pub(crate) const fn is_valid(&self) -> bool {
        self.flags & K_CMTIME_FLAGS_VALID != 0
    }

    pub(crate) const fn is_invalid(&self) -> bool {
        ! self.is_valid()
    }

    pub(crate) const fn is_indefinite(&self) -> bool {
        self.is_valid() &&
        (self.flags & K_CMTIME_FLAGS_INDEFINITE != 0)
    }

    pub(crate) const fn is_positive_infinity(&self) -> bool {
        self.is_valid() &&
        (self.flags & K_CMTIME_FLAGS_POSITIVE_INFINITY != 0)
    }

    pub(crate) const fn is_negative_infinity(&self) -> bool {
        self.is_valid() &&
        (self.flags & K_CMTIME_FLAGS_NEGATIVE_INFINITY != 0)
    }

    pub(crate) const fn is_numeric(&self) -> bool {
        self.is_valid() &&
        ! self.is_indefinite() &&
        ! self.is_positive_infinity() &&
        ! self.is_negative_infinity()
    }

    pub(crate) const fn has_been_rounded(&self) -> bool {
        self.flags & K_CMTIME_FLAGS_HAS_BEEN_ROUNDED != 0
    }

    pub(crate) fn convert_timescale(&self, new_timescale: i32, rounding_method: CMTimeRoundingMethod) -> Self {
        unsafe { CMTimeConvertScale(*self, new_timescale, rounding_method.to_u32()) }
    }

    pub(crate) fn multiply_by_ratio(&self, multiplier: i32, divisor: i32) -> Self {
        unsafe { CMTimeMultiplyByRatio(*self, multiplier, divisor) }
    }

    pub(crate) fn seconds_f64(&self) -> f64 {
        unsafe { CMTimeGetSeconds(*self) }
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum SCStreamSampleRate {
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
#[derive(Debug)]
pub(crate) struct SCContentFilter(pub(crate) *mut AnyObject);

unsafe impl Send for SCContentFilter {}
unsafe impl Sync for SCContentFilter {}

impl SCContentFilter {
    pub(crate) fn new_with_desktop_independent_window(window: &SCWindow) -> Self {
        unsafe {
            let id: *mut AnyObject = msg_send![class!(SCContentFilter), alloc];
            let _: *mut AnyObject = msg_send![id, initWithDesktopIndependentWindow: window.clone()];
            Self(id)
        }
    }

    pub(crate) fn new_with_display_excluding_window(display: SCDisplay, windows: Option<Vec<SCWindow>>) -> Self {
        unsafe {

            let mut windows_nsarray = NSArray::new_mutable();
            if let Some(windows) = windows {
                for w in windows {
                    windows_nsarray.add_object(w.0);
                }
            }

            let id: *mut AnyObject = msg_send![class!(SCContentFilter), alloc];
            let _: *mut AnyObject = msg_send![id, initWithDisplay: display.0 excludingWindows: windows_nsarray.0];
            Self(id)
        }
    }

    pub(crate) fn new_with_display_excluding_apps_excepting_windows(display: SCDisplay, excluded_applications: NSArray, excepting_windows: NSArray) -> Self {
        unsafe {
            let id: *mut AnyObject = msg_send![class!(SCContentFilter), alloc];
            let id: *mut AnyObject = msg_send![id, initWithDisplay: display.0 excludingApplications: excluded_applications.0 exceptingWindows: excepting_windows.0];
            Self(id)
        }
    }
}

impl Clone for SCContentFilter {
    fn clone(&self) -> Self {
        unsafe { let _: *mut AnyObject = msg_send![self.0, retain]; }
        SCContentFilter(self.0)
    }
}

impl Drop for SCContentFilter {
    fn drop(&mut self) {
        unsafe { let _: () = msg_send![self.0, release]; }
    }
}

pub(crate) enum SCStreamCallbackError {
    SampleBufferCopyFailed,
    StreamStopped,
    Other(NSError)
}

#[repr(C)]
struct SCStreamCallbackContainer {
    callback: Box<dyn FnMut(Result<(CMSampleBuffer, SCStreamOutputType), SCStreamCallbackError>) + Send + 'static>
}

unsafe impl RefEncode for SCStreamCallbackContainer {
    const ENCODING_REF: Encoding = Encoding::Void;
}

impl SCStreamCallbackContainer {
    pub fn new(callback: impl FnMut(Result<(CMSampleBuffer, SCStreamOutputType), SCStreamCallbackError>) + Send + 'static) -> Self {
        Self {
            callback: Box::new(callback)
        }
    }

    pub fn call_output(&mut self, sample_buffer: CMSampleBuffer, output_type: SCStreamOutputType) {
        (self.callback)(Ok((sample_buffer, output_type)));
    }

    pub fn call_error(&mut self, error: SCStreamCallbackError) {
        (self.callback)(Err(error));
    }
}

#[derive(Copy, Clone, Debug)]
pub enum SCStreamOutputType {
    Screen,
    Audio,
}

impl SCStreamOutputType {
    fn to_encoded(&self) -> SCStreamOutputTypeEncoded {
        SCStreamOutputTypeEncoded(match *self {
            Self::Screen => 0,
            Self::Audio => 1,
        })
    }

    fn from_encoded(x: isize) -> Option<Self> {
        match x {
            0 => Some(Self::Screen),
            1 => Some(Self::Audio),
            _ => None
        }
    }
}

#[repr(C)]
struct SCStreamOutputTypeEncoded(isize);

unsafe impl Encode for SCStreamOutputTypeEncoded {
    const ENCODING: Encoding = Encoding::LongLong;
}

#[repr(C)]
struct SCStreamEncoded(*mut AnyObject);

extern fn sc_stream_output_did_output_sample_buffer_of_type(this: *mut AnyObject, _sel: Sel, stream: SCStream, buffer: CMSampleBufferRef, output_type: SCStreamOutputTypeEncoded) {
    unsafe {
        let callback_container_ivar = SCStreamHandler::get_class().instance_variable("callback_container_ptr").expect("Expected callback_container_ptr ivar on SCStreamHandler");
        let callback_container: *mut SCStreamCallbackContainer = *callback_container_ivar.load::<*mut c_void>(&mut *this) as *mut _;
        if let Ok(sample_buffer) = CMSampleBuffer::copy_from_ref(buffer) {
            let output_type = SCStreamOutputType::from_encoded(output_type.0).unwrap();
            (&mut *callback_container).call_output(sample_buffer, output_type);
        } else {
            (&mut *callback_container).call_error(SCStreamCallbackError::SampleBufferCopyFailed);
        }
        std::mem::forget(stream);
    }
}

extern fn sc_stream_handler_did_stop_with_error(this: *mut AnyObject, _sel: Sel, stream: SCStream, error: NSError) -> () {
    unsafe {
        let callback_container_ivar = SCStreamHandler::get_class().instance_variable("callback_container_ptr").expect("Expected callback_container_ptr ivar on SCStreamHandler");
        let callback_container: *mut SCStreamCallbackContainer = *callback_container_ivar.load(&mut *this);
        (&mut *callback_container).call_error(SCStreamCallbackError::StreamStopped);
        std::mem::forget(error);
        std::mem::forget(stream);
    }
}

extern fn sc_stream_handler_dealloc(this: *mut AnyObject, _sel: Sel) {
    unsafe {
        let callback_container_ivar = SCStreamHandler::get_class().instance_variable("callback_container_ptr").expect("Expected callback_container_ptr ivar on SCStreamHandler");
        let callback_container: *mut SCStreamCallbackContainer = *callback_container_ivar.load(&mut *this);
        let callback_container: Box<SCStreamCallbackContainer> = Box::from_raw(callback_container);
        drop(callback_container);
    }
}

#[repr(C)]
pub(crate) struct SCStreamHandler(*mut AnyObject);

impl SCStreamHandler {
    pub fn new(callback: impl FnMut(Result<(CMSampleBuffer, SCStreamOutputType), SCStreamCallbackError>) + Send + 'static) -> Self {
        let class = Self::get_class();
        let callback_container_ptr = Box::leak(Box::new(SCStreamCallbackContainer::new(callback)));
        unsafe {
            let instance: *mut AnyObject = msg_send![class!(SCStreamHandler), alloc];
            let instance: *mut AnyObject = msg_send![instance, init];
            let callback_container_ivar = class.instance_variable("callback_container_ptr").expect("Expected callback_container_ptr ivar on SCStreamHandler");
            *callback_container_ivar.load_mut(&mut *instance) = callback_container_ptr as *mut _ as *mut c_void;
            Self(instance)
        }
    }

    fn get_class() -> &'static AnyClass {
        unsafe {
            if let Some(mut class) = ClassBuilder::new("SCStreamHandler", class!(NSObject)) {
                class.add_method(sel!(stream:didOutputSampleBuffer:ofType:), sc_stream_output_did_output_sample_buffer_of_type as extern fn (*mut AnyObject, Sel, SCStream, CMSampleBufferRef, SCStreamOutputTypeEncoded));
                class.add_method(sel!(stream:didStopWithError:), sc_stream_handler_did_stop_with_error as extern fn(*mut AnyObject, Sel, SCStream, NSError));
                class.add_method(sel!(dealloc), sc_stream_handler_dealloc as extern fn(*mut AnyObject, Sel));

                class.add_ivar::<*mut c_void>("callback_container_ptr");
                
                return class.register();
            } else {
                return class!(SCStreamHandler);
            }
        }
    }
}


#[repr(C)]
#[derive(Debug)]
pub struct SCStream(*mut AnyObject);

unsafe impl Encode for SCStream {
    const ENCODING: Encoding = Encoding::Object;
}

unsafe impl Sync for SCStream {}
unsafe impl Send for SCStream {}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct SCStreamDelegate(*mut AnyObject);

unsafe impl Encode for SCStreamDelegate {
    const ENCODING: Encoding = Encoding::Object;
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct SCStreamOutput(*mut AnyObject);

unsafe impl Encode for SCStreamOutput {
    const ENCODING: Encoding = Encoding::Object;
}

impl SCStream {
    pub fn preflight_access() -> bool {
        unsafe { CGPreflightScreenCaptureAccess() }
    }

    pub async fn request_access() -> bool {
        async {
            unsafe { CGRequestScreenCaptureAccess() }
        }.await
    }

    pub fn from_id(id: *mut AnyObject) -> Self {
        unsafe { let _: *mut AnyObject = msg_send![id, retain]; }
        Self(id)
    }

    pub fn is_nil(&self) -> bool {
        self.0.is_null()
    }

    pub fn new(filter: SCContentFilter, config: SCStreamConfiguration, handler_queue: DispatchQueue, handler: SCStreamHandler) -> Result<Self, String> {
        unsafe {
            let instance: *mut AnyObject = msg_send![class!(SCStream), alloc];
            let instance: *mut AnyObject = msg_send![instance, initWithFilter: filter.0 configuration: config.0 delegate: SCStreamDelegate(handler.0)];
            let mut error: *mut AnyObject = std::ptr::null_mut();
            let result: bool = msg_send![instance, addStreamOutput: SCStreamOutput(handler.0) type: SCStreamOutputType::Screen.to_encoded() sampleHandlerQueue: handler_queue error: &mut error as *mut _];
            if !error.is_null() {
                let error = NSError::from_id_retained(error);
                let _: () = msg_send![instance, release];
                return Err(format!("SCStream error: {}, reason: {}", error.description(), error.reason()));
            }
            Ok(SCStream(instance))
        }
    }

    pub fn start(&mut self) {
        unsafe {
            let _: () = msg_send![self.0, startCaptureWithCompletionHandler: &*StackBlock::new(Box::new(
                |error: *mut AnyObject| {
                    if !error.is_null() {
                        let error =  NSError::from_id_unretained(error);
                        println!("startCaptureWithCompletionHandler error: {:?}, reason: {:?}", error.description(), error.reason());
                    }
                }
            )).copy()];
        }
    }

    pub fn stop(&mut self) {
        unsafe {
            let _: () = msg_send![self.0, stopCaptureWithCompletionHandler: &*StackBlock::new(Box::new(
                |error: *mut AnyObject| {
                    if !error.is_null() {
                        let error =  NSError::from_id_unretained(error);
                        println!("stopCaptureWithCompletionHandler error: {:?}, reason: {:?}", error.description(), error.reason());
                    }
                }
            )).copy()];
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub(crate) struct CMSampleBuffer(CMSampleBufferRef);

unsafe impl Send for CMSampleBuffer {}

impl CMSampleBuffer {
    pub(crate) fn copy_from_ref(r: CMSampleBufferRef) -> Result<Self, ()> {
        unsafe { 
            let mut new_ref: CMSampleBufferRef = std::ptr::null();
            let status = CMSampleBufferCreateCopy(kCFAllocatorDefault, r, &mut new_ref as *mut _);
            if status != 0 {
                Err(())
            } else {
                Ok(CMSampleBuffer(new_ref))
            }
        }
    }

    pub(crate) fn get_presentation_timestamp(&self) -> CMTime {
        unsafe { CMSampleBufferGetPresentationTimeStamp(self.0) }
    }

    pub(crate) fn get_duration(&self) -> CMTime {
        unsafe { CMSampleBufferGetDuration(self.0) }
    }

    pub(crate) fn get_format_description(&self) -> CMFormatDescription {
        let format_desc_ref = unsafe { CMSampleBufferGetFormatDescription(self.0) };
        CMFormatDescription::from_ref_unretained(format_desc_ref)
    }

    // CMSampleBufferGetAudioBufferListWithRetainedBlockBuffer
    pub(crate) unsafe fn get_audio_buffer_list_with_block_buffer(&self) -> Result<(AudioBufferList, CMBlockBuffer), ()> {
        let mut audio_buffer_list = AudioBufferList::default();
        let mut block_buffer: CMBlockBufferRef = std::ptr::null();
        let status = CMSampleBufferGetAudioBufferListWithRetainedBlockBuffer(
            self.0,
            std::ptr::null_mut(),
            &mut audio_buffer_list as *mut _,
            std::mem::size_of::<AudioBufferList>(),
            kCFAllocatorNull,
            kCFAllocatorNull,
            kCMSampleBufferFlag_AudioBufferList_Assure16ByteAlignment,
            &mut block_buffer as *mut _
        );
        if status != 0 {
            return Err(());
        }
        Ok((audio_buffer_list, CMBlockBuffer::from_ref_retained(block_buffer)))
    }

    pub(crate) fn get_sample_attachment_array(&self) -> Vec<CFDictionary> {
        unsafe {
            let attachment_array_ref = CMSampleBufferGetSampleAttachmentsArray(self.0, false.into());
            if attachment_array_ref.is_null() {
                return vec![];
            }
            let attachments_array = CFArray::from_ref_unretained(attachment_array_ref);
            let mut attachments = Vec::new();
            for i in 0..attachments_array.get_count() {
                attachments.push(CFDictionary::from_ref_unretained(attachments_array.get_value_at_index(i)));
            }
            attachments
        }
    }

    pub(crate) fn get_image_buffer(&self) -> Option<CVPixelBuffer> {
        unsafe {
            let buffer_ref = CMSampleBufferGetImageBuffer(self.0);
            if buffer_ref.is_null() {
                None
            } else {
                Some(CVPixelBuffer::from_ref_unretained(buffer_ref))
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

#[repr(C)]
pub(crate) struct CFDictionary(CFTypeRef);

impl CFDictionary {
    pub(crate) fn from_ref_retained(r: CFDictionaryRef) -> Self {
        Self(r)
    }

    pub(crate) fn from_ref_unretained(r: CFDictionaryRef) -> Self {
        unsafe { CFRetain(r); }
        Self(r)
    }

    pub(crate) fn get_value(&self, key: CFTypeRef) -> CFTypeRef {
        unsafe { CFDictionaryGetValue(self.0, key) }
    }
}

impl Clone for CFDictionary {
    fn clone(&self) -> Self {
        Self::from_ref_unretained(self.0)
    }
}

impl Drop for CFDictionary {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0); }
    }
}

pub(crate) enum CMMediaType {
    Audio,
    Video,
}

impl CMMediaType {
    pub(crate) fn from_ostype(ostype: OSType) -> Option<Self> {
        match ostype.0.map(|x| x as char) {
            ['v', 'i', 'd', 'e'] => Some(Self::Video),
            ['s', 'o', 'u', 'n'] => Some(Self::Audio),
            _ => None,
        }
    }
}

#[repr(C)]
pub(crate) struct CMFormatDescription(CMFormatDescriptionRef);

impl CMFormatDescription {
    pub(crate) fn from_ref_retained(r: CMFormatDescriptionRef) -> Self {
        Self(r)
    }

    pub(crate) fn from_ref_unretained(r: CMFormatDescriptionRef) -> Self {
        unsafe { CFRetain(r); }
        Self(r)
    }

    pub(crate) fn get_media_type(&self) -> OSType {
        unsafe { CMFormatDescriptionGetMediaType(self.0) }
    }

    pub(crate) fn as_audio_format_description(&self) -> Option<CMAudioFormatDescription> {
        match CMMediaType::from_ostype(self.get_media_type()) {
            Some(CMMediaType::Audio) => {
                Some(CMAudioFormatDescription::from_ref_unretained(self.0))
            },
            _ => None
        }
    }
}

impl Drop for CMFormatDescription {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0); }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub(crate) struct AudioStreamBasicDescription {
    pub sample_rate: f64,
    pub format_id: u32,
    pub format_flags: u32,
    pub bytes_per_packet: u32,
    pub frames_per_packet: u32,
    pub bytes_per_frame: u32,
    pub channels_per_frame: u32,
    pub bits_per_channel: u32,
    pub reserved: u32,
}

#[repr(C)]
pub(crate) struct CMAudioFormatDescription(CMFormatDescriptionRef);

impl CMAudioFormatDescription {
    pub(crate) fn from_ref_retained(r: CMFormatDescriptionRef) -> Self {
        Self(r)
    }

    pub(crate) fn from_ref_unretained(r: CMFormatDescriptionRef) -> Self {
        unsafe { CFRetain(r); }
        Self(r)
    }

    pub(crate) fn get_basic_stream_description(&self) -> &'_ AudioStreamBasicDescription {
        unsafe { &*CMAudioFormatDescriptionGetStreamBasicDescription(self.0) as &_ }
    }
}

impl Drop for CMAudioFormatDescription {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0); }
    }
}

pub(crate) struct AVAudioFormat(*mut AnyObject);

unsafe impl Encode for AVAudioFormat {
    const ENCODING: Encoding = Encoding::Object;
}

impl AVAudioFormat {
    pub fn new_with_standard_format_sample_rate_channels(sample_rate: f64, channel_count: u32) -> Self {
        unsafe {
            let id: *mut AnyObject = msg_send![class!(AVAudioFormat), alloc];
            let _: *mut AnyObject = msg_send![id, initStandardFormatWithSampleRate: sample_rate channels: channel_count];
            Self(id)
        }
    }
}

impl Drop for AVAudioFormat {
    fn drop(&mut self) {
        unsafe { let _: () = msg_send![self.0, release]; }
    }
}

#[repr(C)]
pub(crate) struct AudioBuffer {
    number_channels: u32,
    data_byte_size: u32,
    data: *mut c_void,
}

unsafe impl Encode for AudioBuffer {
    const ENCODING: Encoding = Encoding::Struct("AudioBuffer", &[
        Encoding::UInt,
        Encoding::UInt,
        Encoding::Pointer(&Encoding::Void),
    ]);
}

#[repr(C)]
pub(crate) struct AudioBufferList {
    number_buffers: u32,
    buffers: *mut AudioBuffer,
}

unsafe impl Encode for AudioBufferList {
    const ENCODING: Encoding = Encoding::Struct("AudioBufferList", &[
        Encoding::UInt,
        Encoding::Pointer(&AudioBuffer::ENCODING)
    ]);
}

unsafe impl RefEncode for AudioBufferList {
    const ENCODING_REF: Encoding = Self::ENCODING;
}

impl Default for AudioBufferList {
    fn default() -> Self {
        Self {
            number_buffers: 0,
            buffers: std::ptr::null_mut()
        }
    }
}

#[repr(C)]
pub(crate) struct CMBlockBuffer(CMBlockBufferRef);


impl CMBlockBuffer {
    pub(crate) fn from_ref_retained(r: CMBlockBufferRef) -> Self {
        Self(r)
    }

    pub(crate) fn from_ref_unretained(r: CMBlockBufferRef) -> Self {
        unsafe { CFRetain(r); }
        Self(r)
    }
}

impl Drop for CMBlockBuffer {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0); }
    }
}

pub(crate) struct AVAudioPCMBuffer(*mut AnyObject);

impl AVAudioPCMBuffer {
    pub fn new_with_format_buffer_list_no_copy_deallocator(format: AVAudioFormat, buffer_list_no_copy: *const AudioBufferList) -> Result<Self, ()> {
        unsafe {
            let id: *mut AnyObject = msg_send![class!(AVAudioPCMBuffer), alloc];
            let result: *mut AnyObject = msg_send![id, initWithPCMFormat: format bufferListNoCopy: buffer_list_no_copy];
            if result.is_null() {
                let _: () = msg_send![id, release];
                Err(())
            } else {
                Ok(Self(id))
            }
        }
    }

    pub fn stride(&self) -> usize {
        unsafe { msg_send![self.0, stride] }
    }

    pub fn frame_capacity(&self) -> usize {
        unsafe { msg_send![self.0, frameCapacity] }
    }

    pub fn channel_count(&self) -> usize {
        unsafe { msg_send![self.0, stride] }
    }

    pub fn f32_buffer(&self, channel: usize) -> Option<*const f32> {
        let channel_count = self.stride();
        if channel >= channel_count {
            return None;
        }
        unsafe {
            let all_channels_data_ptr: *const *const f32 = msg_send![self.0, floatChannelData];
            if all_channels_data_ptr.is_null() {
                return None;
            }
            let all_channels_data = std::slice::from_raw_parts(all_channels_data_ptr, channel_count);
            let channel_data = all_channels_data[channel];
            if channel_data.is_null() {
                None
            } else {
                Some(channel_data)
            }
        }
    }

    pub fn i32_buffer(&self, channel: usize) -> Option<*const i32> {
        let channel_count = self.stride();
        if channel >= channel_count {
            return None;
        }
        unsafe {
            let all_channels_data_ptr: *const *const i32 = msg_send![self.0, int32ChannelData];
            if all_channels_data_ptr.is_null() {
                return None;
            }
            let all_channels_data = std::slice::from_raw_parts(all_channels_data_ptr, channel_count);
            let channel_data = all_channels_data[channel];
            if channel_data.is_null() {
                None
            } else {
                Some(channel_data)
            }
        }
    }

    pub fn i16_buffer(&self, channel: usize) -> Option<*const i16> {
        let channel_count = self.stride();
        if channel >= channel_count {
            return None;
        }
        unsafe {
            let all_channels_data_ptr: *const *const i16 = msg_send![self.0, int32ChannelData];
            if all_channels_data_ptr.is_null() {
                return None;
            }
            let all_channels_data = std::slice::from_raw_parts(all_channels_data_ptr, channel_count);
            let channel_data = all_channels_data[channel];
            if channel_data.is_null() {
                None
            } else {
                Some(channel_data)
            }
        }
    }
}

/*
typedef const void *	(*CFArrayRetainCallBack)(CFAllocatorRef allocator, const void *value);
typedef void		(*CFArrayReleaseCallBack)(CFAllocatorRef allocator, const void *value);
typedef CFStringRef	(*CFArrayCopyDescriptionCallBack)(const void *value);
typedef Boolean		(*CFArrayEqualCallBack)(const void *value1, const void *value2);
typedef struct {
    CFIndex				version;
    CFArrayRetainCallBack		retain;
    CFArrayReleaseCallBack		release;
    CFArrayCopyDescriptionCallBack	copyDescription;
    CFArrayEqualCallBack		equal;
} CFArrayCallBacks;
*/
#[repr(C)]
struct CFArrayCallBacks {
    version: isize,
    retain: *const extern "C" fn(allocator: CFAllocatorRef, value: *const c_void) -> *const c_void,
    release: *const extern "C" fn(allocator: CFAllocatorRef, value: *const c_void),
    copy_description: *const extern "C" fn(value: *const c_void) -> CFStringRef,
    equal: *const extern "C" fn(value_1: *const c_void, value_2: *const c_void) -> bool,
}

#[repr(C)]
struct CFArray(CFArrayRef);

impl CFArray {
    pub(crate) fn new_mutable(capacity: Option<isize>, callbacks: Option<&CFArrayCallBacks>) -> Result<Self, ()> {
        unsafe {
            let new_array = CFArrayCreateMutable(kCFAllocatorDefault, capacity.unwrap_or(0), callbacks.map(|callbacks| callbacks as *const _).unwrap_or(std::ptr::null()));
            if new_array.is_null() {
                Err(())
            } else {
                Ok(Self::from_ref_retained(new_array))
            }
        }
    }

    pub(crate) fn append_value(&mut self, value: *const c_void) {
        unsafe {
            CFArrayAppendValue(self.0, value);
        }
    }

    pub(crate) fn from_ref_unretained(r: CFStringRef) -> Self {
        unsafe { CFRetain(r); }
        Self(r)
    }

    pub(crate) fn from_ref_retained(r: CFStringRef) -> Self {
        Self(r)
    }

    pub(crate) fn get_count(&self) -> i32 {
        unsafe { CFArrayGetCount(self.0) }
    }

    pub(crate) fn get_value_at_index(&self, index: i32) -> *const c_void {
        unsafe { CFArrayGetValueAtIndex(self.0, index) }
    }
}

impl Clone for CFArray {
    fn clone(&self) -> Self {
        CFArray::from_ref_unretained(self.0)
    }
}

impl Drop for CFArray {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0); }
    }
}




#[repr(C)]
#[derive(Debug)]
pub struct DispatchQueue(*mut AnyObject);

unsafe impl Encode for DispatchQueue {
    const ENCODING: Encoding = Encoding::Object;
}

impl DispatchQueue {
    pub fn make_concurrent(name: String) -> Self {
        let cstring_name = CString::new(name.as_str()).unwrap();
        unsafe { dispatch_queue_create(cstring_name.as_ptr(), DispatchQueueAttr(addr_of_mut!(_dispatch_queue_attr_concurrent))) }
    }

    pub fn make_serial(name: String) -> Self {
        let cstring_name = CString::new(name.as_str()).unwrap();
        unsafe { dispatch_queue_create(cstring_name.as_ptr(), DispatchQueueAttr(0 as *mut c_void)) }
    }

    pub fn make_null() -> Self {
        DispatchQueue(std::ptr::null_mut())
    }
}

impl Drop for DispatchQueue {
    fn drop(&mut self) {
        if self.0.is_null() {
            return;
        }
        unsafe { dispatch_release(self.0) };
    }
}

impl Clone for DispatchQueue {
    fn clone(&self) -> Self {
        if self.0.is_null() {
            return Self(std::ptr::null_mut());
        }
        unsafe { dispatch_retain(self.0); }
        Self(self.0)
    }
}

#[repr(C)]
struct DispatchQueueAttr(*mut c_void);

pub(crate) struct SCRunningApplication(pub(crate) *mut AnyObject);

impl SCRunningApplication {
    pub(crate) fn from_id_unretained(id: *mut AnyObject) -> Self {
        unsafe { let _: *mut AnyObject = msg_send![id, retain]; }
        Self(id)
    }

    pub(crate) fn from_id_retained(id: *mut AnyObject) -> Self {
        Self(id)
    }

    pub(crate) fn pid(&self) -> i32 {
        unsafe {
            msg_send![self.0, processID]
        }
    }

    pub(crate) fn application_name(&self) -> String {
        unsafe {
            let app_name_cfstringref: CFStringRef = msg_send![self.0, applicationName];
            NSString::from_ref_unretained(app_name_cfstringref).as_string()
        }
    }

    pub(crate) fn bundle_identifier(&self) -> String {
        unsafe {
            let bundle_id_nsstring: *mut AnyObject = msg_send![self.0, bundleIdentifier];
            NSString::from_id_unretained(bundle_id_nsstring).as_string()
        }
    }
}

impl Clone for SCRunningApplication {
    fn clone(&self) -> Self {
        Self::from_id_unretained(self.0)
    }
}

impl Drop for SCRunningApplication {
    fn drop(&mut self) {
        unsafe { let _: () = msg_send![self.0, release]; }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum CGDisplayStreamFrameStatus {
    Complete,
    Idle,
    Blank,
    Stopped,
}

impl CGDisplayStreamFrameStatus {
    pub fn from_i32(x: i32) -> Option<Self> {
        match x {
            0 => Some(Self::Complete),
            1 => Some(Self::Idle),
            2 => Some(Self::Blank), 
            3 => Some(Self::Stopped),
            _ => None
        }
    }
}

pub(crate) struct CGDisplayStream{
    stream_ref: CGDisplayStreamRef,
    callback_block: RcBlock<dyn Fn(i32, u64, IOSurfaceRef, CGDisplayStreamUpdateRef)>,
}

impl CGDisplayStream {
    pub fn new(callback: impl Fn(CGDisplayStreamFrameStatus, Duration, IOSurface) + 'static, display_id: u32, size: (usize, usize), pixel_format: SCStreamPixelFormat, options_dict: NSDictionary, dispatch_queue: DispatchQueue) -> Self {
        let absolute_time_start = Arc::new(Mutex::new(None));
        let callback = Arc::new(callback);
        let callback_block = StackBlock::new(move |status: i32, display_time: u64, iosurface_ref: IOSurfaceRef, stream_update_ref: CGDisplayStreamUpdateRef| {
            if let Some(status) = CGDisplayStreamFrameStatus::from_i32(status) {
                let abs_time_start_opt = { *absolute_time_start.lock() };
                let relative_time = if let Some(absolute_time_start) = abs_time_start_opt {
                    display_time.saturating_sub(absolute_time_start)
                } else {
                    *absolute_time_start.lock() = Some(display_time);
                    0
                };
                unsafe {
                    let mut timebase_info: mach_timebase_info_data_t = Default::default();
                    mach_timebase_info(&mut timebase_info as *mut _);
                    let time_ns = ((relative_time as u128 * timebase_info.numer as u128) / timebase_info.denom as u128);
                    let time = Duration::from_nanos(time_ns as u64);
                    let io_surface = IOSurface::from_ref_unretained(iosurface_ref);
                    (callback)(status, time, io_surface);
                }
            }
        }).copy();
        unsafe {
            let pixel_format = pixel_format.to_ostype();
            let display_id = CGMainDisplayID();
            let stream_ref = CGDisplayStreamCreateWithDispatchQueue(display_id, size.0, size.1, pixel_format.as_i32(), std::ptr::null_mut(), dispatch_queue.0, &*callback_block as *const _ as *const c_void);
            Self {
                stream_ref,
                callback_block
            }
        }
    }

    pub fn start(&self) -> Result<(), ()> {
        let error_code = unsafe { CGDisplayStreamStart(self.stream_ref) };
        if error_code == 0 {
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn stop(&self) -> Result<(), ()> {
        let error_code = unsafe { CGDisplayStreamStop(self.stream_ref) };
        if error_code == 0 {
            Ok(())
        } else {
            Err(())
        }
    }
}

impl Clone for CGDisplayStream {
    fn clone(&self) -> Self {
        unsafe {
            CFRetain(self.stream_ref);
        }
        CGDisplayStream {
            stream_ref: self.stream_ref,
            callback_block: self.callback_block.clone()
        }
    }
}

impl Drop for CGDisplayStream {
    fn drop(&mut self) {
        unsafe {
            CFRelease(self.stream_ref);
        }
    }
}



#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CVPixelFormat {
    RGB888,
    BGR888,
    ARGB8888,
    BGRA8888,
    ABGR8888,
    RGBA8888,
    V420,
    F420,
    Other,
}

impl CVPixelFormat {
    pub(crate) fn from_ostype(ostype: &OSType) -> Option<Self> {
        unsafe {
            let value = std::mem::transmute::<_, u32>(*ostype);
            Some(match value {
                0x00000018 => Self::RGB888,
                0x32344247 => Self::BGR888,
                0x00000020 => Self::ARGB8888,
                0x42475241 => Self::BGRA8888,
                0x41424752 => Self::ABGR8888,
                0x52474241 => Self::RGBA8888,
                0x34323076 => Self::V420,
                _ => {
                    return None;
                }
            })
        }
    }
}

pub(crate) struct IOSurface(pub(crate) IOSurfaceRef);

const IOSURFACELOCK_READONLY  : u32 = 1;
const IOSURFACELOCK_AVOIDSYNC : u32 = 2;

const SYS_IOKIT              : i32 = 0x38 << 26;
const SUB_IOKIT_COMMON       : i32 = 0;
const KIO_RETURN_CANNOT_LOCK : i32 = SYS_IOKIT | SUB_IOKIT_COMMON | 0x2cc;

pub enum IOSurfaceLockError {
    CannotLock,
    Other
}

pub struct IOSurfaceLockGaurd(IOSurfaceRef, u32);

impl IOSurfaceLockGaurd {
    pub(crate) fn get_base_address_of_plane(&self, plane: usize) -> Option<*const c_void> {
        unsafe { 
            let ptr = IOSurfaceGetBaseAddressOfPlane(self.0, plane);
            if ptr.is_null() {
                None
            } else {
                Some(ptr)
            }
        }
    }

    pub(crate) fn get_base_address(&self) -> Option<*const c_void> {
        unsafe {
            let ptr = IOSurfaceGetBaseAddress(self.0);
            if ptr.is_null() {
                None
            } else {
                Some(ptr)
            }
        }
    }
}

impl Drop for IOSurfaceLockGaurd {
    fn drop(&mut self) {
        unsafe {
            let mut seed = 0u32;
            IOSurfaceUnlock(self.0, self.1, &mut seed as *mut _);
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IOSurfaceComponentName {
    Unknown,
    Alpha,
    Red,
    Green,
    Blue,
    Luma,
    ChromaRed,
    ChromaBlue,
}

const kIOSurfaceComponentNameUnknown    : i32 = 0;
const kIOSurfaceComponentNameAlpha      : i32 = 1;
const kIOSurfaceComponentNameRed        : i32 = 2;
const kIOSurfaceComponentNameGreen      : i32 = 3;
const kIOSurfaceComponentNameBlue       : i32 = 4;
const kIOSurfaceComponentNameLuma       : i32 = 5;
const kIOSurfaceComponentNameChromaRed  : i32 = 6;
const kIOSurfaceComponentNameChromaBlue : i32 = 7;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IOSurfaceComponentType {
    Unknown,
    UInt,
    SInt,
    Float,
    SNorm,
}

const kIOSurfaceComponentTypeUnknown          : i32 = 0;
const kIOSurfaceComponentTypeUnsignedInteger  : i32 = 1;
const kIOSurfaceComponentTypeSignedInteger    : i32 = 2;
const kIOSurfaceComponentTypeFloat            : i32 = 3;
const kIOSurfaceComponentTypeSignedNormalized : i32 = 4;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IOSurfaceCacheMode {
    Default,
    Inhibit,
    WriteThrough,
    Copyback,
    WriteCombine,
    CopybackInner,
    PostedWrite,
    RealTime,
    PostedReordered,
    PostedCombinedReordered,
}

impl IOSurface {
    fn from_ref_unretained(r: IOSurfaceRef) -> Self {
        unsafe { IOSurfaceIncrementUseCount(r); }
        Self(r)
    }

    pub(crate) fn get_pixel_format(&self) -> Option<CVPixelFormat> {
        unsafe {
            let pixel_format_ostype = IOSurfaceGetPixelFormat(self.0);
            CVPixelFormat::from_ostype(&pixel_format_ostype)
        }
    }

    pub(crate) fn get_bytes_per_row(&self) -> usize {
        unsafe {
            IOSurfaceGetBytesPerRow(self.0)
        }
    }

    pub(crate) fn get_bytes_per_row_of_plane(&self, plane: usize) -> usize {
        unsafe {
            IOSurfaceGetBytesPerRowOfPlane(self.0, plane)
        }
    }

    pub(crate) fn get_width(&self) -> usize {
        unsafe {
            IOSurfaceGetWidth(self.0)
        }
    }

    pub(crate) fn get_height(&self) -> usize {
        unsafe {
            IOSurfaceGetHeight(self.0)
        }
    }

    pub(crate) fn get_height_of_plane(&self, plane: usize) -> usize {
        unsafe {
            IOSurfaceGetHeightOfPlane(self.0, plane)
        }
    }

    pub(crate) fn get_width_of_plane(&self, plane: usize) -> usize {
        unsafe {
            IOSurfaceGetWidthOfPlane(self.0, plane)
        }
    }

    pub(crate) fn get_plane_count(&self) -> usize {
        unsafe {
            IOSurfaceGetPlaneCount(self.0)
        }
    }

    pub(crate) fn get_plane_component_count(&self, plane: usize) -> usize {
        unsafe {
            IOSurfaceGetNumberOfComponentsOfPlane(self.0, plane)
        }
    }

    pub(crate) fn get_plane_component_name(&self, plane: usize, component: usize) -> IOSurfaceComponentName {
        unsafe {
            match IOSurfaceGetNameOfComponentOfPlane(self.0, plane, component) {
                kIOSurfaceComponentNameRed        => IOSurfaceComponentName::Red,
                kIOSurfaceComponentNameGreen      => IOSurfaceComponentName::Green,
                kIOSurfaceComponentNameBlue       => IOSurfaceComponentName::Blue,
                kIOSurfaceComponentNameAlpha      => IOSurfaceComponentName::Alpha,
                kIOSurfaceComponentNameChromaRed  => IOSurfaceComponentName::ChromaRed,
                kIOSurfaceComponentNameChromaBlue => IOSurfaceComponentName::ChromaBlue,
                kIOSurfaceComponentNameLuma       => IOSurfaceComponentName::Luma,
                _                                 => IOSurfaceComponentName::Unknown
            }
        }
    }

    pub(crate) fn get_plane_component_type(&self, plane: usize, component: usize) -> IOSurfaceComponentType {
        unsafe {
            match IOSurfaceGetTypeOfComponentOfPlane(self.0, plane, component) {
                kIOSurfaceComponentTypeSignedInteger    => IOSurfaceComponentType::SInt,
                kIOSurfaceComponentTypeUnsignedInteger  => IOSurfaceComponentType::UInt,
                kIOSurfaceComponentTypeFloat            => IOSurfaceComponentType::Float,
                kIOSurfaceComponentTypeSignedNormalized => IOSurfaceComponentType::SNorm,
                _                                       => IOSurfaceComponentType::Unknown,
            }
        }
    }

    pub(crate) fn get_plane_component_bit_depth(&self, plane: usize, component: usize) -> usize {
        unsafe {
            IOSurfaceGetBitDepthOfComponentOfPlane(self.0, plane, component)
        }
    }

    pub(crate) fn get_plane_component_bit_offset(&self, plane: usize, component: usize) -> usize {
        unsafe {
            IOSurfaceGetBitOffsetOfComponentOfPlane(self.0, plane, component)
        }
    }

    pub(crate) fn lock(&self, read_only: bool, avoid_sync: bool) -> Result<IOSurfaceLockGaurd, IOSurfaceLockError> {
        unsafe {
            let options = 
                if read_only  { IOSURFACELOCK_READONLY  } else { 0 } |
                if avoid_sync { IOSURFACELOCK_AVOIDSYNC } else { 0 }
            ;
            match IOSurfaceLock(self.0, options, std::ptr::null_mut()) {
                0                      => Ok(IOSurfaceLockGaurd(self.0, options)),
                KIO_RETURN_CANNOT_LOCK => Err(IOSurfaceLockError::CannotLock),
                _                      => Err(IOSurfaceLockError::Other)
            }
        }
    }
}

impl Clone for IOSurface {
    fn clone(&self) -> Self {
        Self::from_ref_unretained(self.0)
    }
}

impl Drop for IOSurface {
    fn drop(&mut self) {
        unsafe {
            IOSurfaceDecrementUseCount(self.0);
        }
    }
}

/*
kCFNumberSInt8Type = 1,
kCFNumberSInt16Type = 2,
kCFNumberSInt32Type = 3,
kCFNumberSInt64Type = 4,
kCFNumberFloat32Type = 5,
kCFNumberFloat64Type = 6,	/* 64-bit IEEE 754 */
/* Basic C types */
kCFNumberCharType = 7,
kCFNumberShortType = 8,
kCFNumberIntType = 9,
kCFNumberLongType = 10,
kCFNumberLongLongType = 11,
kCFNumberFloatType = 12,
kCFNumberDoubleType = 13,
/* Other */
kCFNumberCFIndexType = 14,
*/

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum CFNumberType {
    I8,
    I16,
    I32,
    I64,
    F32,
    F64,
    Char,
    Short,
    Int,
    Long,
    LongLong,
    Float,
    Double,
    CFIndex,
    NSInteger,
    CGFloat,
}

impl CFNumberType {
    pub(crate) fn to_isize(self) -> isize {
        match self {
            Self::I8 => 1,
            Self::I16 => 2,
            Self::I32 => 3,
            Self::I64 => 4,
            Self::F32 => 5,
            Self::F64 => 6,
            Self::Char => 7,
            Self::Short => 8,
            Self::Int => 9,
            Self::Long => 10,
            Self::LongLong => 11,
            Self::Float => 12,
            Self::Double => 13,
            Self::CFIndex => 14,
            Self::NSInteger => 15,
            Self::CGFloat => 16,
        }
    }

    pub(crate) fn from_i32(x: i32) -> Option<Self> {
        Some(match x {
            1 => Self::I8,
            2 => Self::I16,
            3 => Self::I32,
            4 => Self::I64,
            5 => Self::F32,
            6 => Self::F64,
            7 => Self::Char,
            8 => Self::Short,
            9 => Self::Int,
            10 => Self::Long,
            11 => Self::LongLong,
            12 => Self::Float,
            13 => Self::Double,
            14 => Self::CFIndex,
            15 => Self::NSInteger,
            16 => Self::CGFloat,
            _ => return None,
        })
    }
}

const kCFNumberSInt32Type: isize = 3;

#[repr(C)]
pub(crate) struct CFNumber(pub(crate) CFNumberRef);

impl CFNumber {
    pub fn new_f32(x: f32) -> Self {
        unsafe {
            let r = CFNumberCreate(kCFAllocatorNull, CFNumberType::F32.to_isize(), &x as *const f32 as *const c_void);
            Self(r)
        }
    }

    pub fn new_i32(x: i32) -> Self {
        unsafe {
            let r = CFNumberCreate(kCFAllocatorNull, CFNumberType::I32.to_isize(), &x as *const i32 as *const c_void);
            Self(r)
        }
    }

    pub fn as_i32(&self) -> Result<i32, ()> {
        unsafe {
            let mut x: i32 = 0;
            if CFNumberGetValue(self.0, kCFNumberSInt32Type, &mut x as *mut i32 as *mut _).as_bool() {
                Ok(x)
            } else {
                Err(())
            }
        }
    }

    pub fn from_ref_unretained(r: CFNumberRef) -> Self {
        unsafe { CFRetain(r); }
        Self(r)
    }
}

impl Clone for CFNumber {
    fn clone(&self) -> Self {
        unsafe { CFRetain(self.0); }
        CFNumber(self.0)
    }
}

impl Drop for CFNumber {
    fn drop(&mut self) {
        unsafe {
            CFRelease(self.0);
        }
    }
}

pub struct NSNumber(pub(crate) *mut AnyObject);

impl NSNumber {
    pub(crate) fn from_id_unretained(id: *mut AnyObject) -> Self {
        unsafe { let _: *mut AnyObject = msg_send![id, retain]; }
        Self(id)
    }

    pub(crate) fn from_id_retained(id: *mut AnyObject) -> Self {
        Self(id)
    }

    pub(crate) fn new_isize(x: isize) -> Self {
        unsafe {
            let id: *mut AnyObject = msg_send![class!(NSNumber), numberWithInteger: x];
            Self(id)
        }
    }

    pub(crate) fn new_f32(x: f32) -> Self {
        unsafe {
            let id: *mut AnyObject = msg_send![class!(NSNumber), numberWithFloat: x];
            Self(id)
        }
    }

    pub(crate) fn as_i32(&self) -> i32 {
        unsafe {
            msg_send![self.0, intValue]
        }
    }

    pub(crate) fn as_f64(&self) -> f64 {
        unsafe {
            msg_send![self.0, doubleValue]
        }
    }

}

impl Clone for NSNumber {
    fn clone(&self) -> Self {
        Self::from_id_unretained(self.0)
    }
}

impl Drop for NSNumber {
    fn drop(&mut self) {
        unsafe { let _: () = msg_send![self.0, release]; }
    }
}

pub struct NSApplication(*mut AnyObject);

impl NSApplication {
    fn from_id_unretained(id: *mut AnyObject) -> Self {
        unsafe { let _: *mut AnyObject = msg_send![id, retain]; }
        Self(id)
    }

    fn from_id_retained(id: *mut AnyObject) -> Self {
        Self(id)
    }

    pub fn shared() -> Self {
        unsafe {
            let id: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
            Self::from_id_unretained(id)
        }
    }

    pub fn run(&self) {
        unsafe {
            let _: () = msg_send![self.0, run];
        }
    }
}

impl Clone for NSApplication {
    fn clone(&self) -> Self {
        Self::from_id_unretained(self.0)
    }
}

impl Drop for NSApplication {
    fn drop(&mut self) {
        unsafe { let _: () = msg_send![self.0, release]; }
    }
}

pub enum SCFrameStatus {
    Complete,
    Idle,
    Blank,
    Started,
    Suspended,
    Stopped,
}

impl SCFrameStatus {
    pub fn from_i32(x: i32) -> Option<Self> {
        match x {
            0 => Some(Self::Complete),
            1 => Some(Self::Idle),
            2 => Some(Self::Blank),
            3 => Some(Self::Suspended),
            4 => Some(Self::Started),
            5 => Some(Self::Stopped),
            _ => None,
        }
    }
}

pub struct CVPixelBuffer(CVPixelBufferRef);

impl CVPixelBuffer {
    pub fn from_ref_unretained(r: CVPixelBufferRef) -> Self {
        unsafe { CFRetain(r); }
        Self(r)
    }

    pub fn from_ref_retained(r: CVPixelBufferRef) -> Self {
        Self(r)
    }

    pub fn get_iosurface_ptr(&self) -> Option<*const c_void> {
        unsafe {
            let iosurface_ptr = CVPixelBufferGetIOSurface(self.0);
            if iosurface_ptr.is_null() {
                None
            } else {
                Some(iosurface_ptr)
            }
        }
    }

    pub fn get_iosurface(&self) -> Option<IOSurface> {
        unsafe {
            let iosurface_ptr = CVPixelBufferGetIOSurface(self.0);
            if iosurface_ptr.is_null() {
                None
            } else {
                Some(IOSurface::from_ref_unretained(iosurface_ptr))
            }
        }
    }

    pub fn get_width(&self) -> usize {
        unsafe {
            CVPixelBufferGetWidth(self.0)
        }
    }

    pub fn get_height(&self) -> usize {
        unsafe {
            CVPixelBufferGetHeight(self.0)
        }
    }
}

impl Clone for CVPixelBuffer {
    fn clone(&self) -> Self {
        Self::from_ref_unretained(self.0)
    }
}

impl Drop for CVPixelBuffer {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0); }
    }
}


#[repr(C)]
struct NSValue(*mut AnyObject);

#[allow(unused)]
impl NSValue {
    fn from_id(id: *mut AnyObject) -> Self {
        Self(id)
    }

    fn size_value(&self) -> CGSize {
        unsafe {
            msg_send![self.0, sizeValue]
        }
    }

    fn unsigned_int_value(&self) -> u32 {
        unsafe {
            msg_send![self.0, unsignedIntValue]
        }
    }
}

impl Drop for NSValue {
    fn drop(&mut self) {
        unsafe { let _: () = msg_send![self.0, release]; }
    }
}

#[repr(C)]
pub struct NSScreen(*mut AnyObject);

impl NSScreen {
    pub(crate) fn screens() -> Vec<NSScreen> {
        unsafe {
            let screens_ns_array = NSArray::from_id_unretained(msg_send![class!(NSScreen), screens]);
            let mut screens_out = Vec::new();
            for i in 0..screens_ns_array.count() {
                let screen: *mut AnyObject = screens_ns_array.obj_at_index(i);
                screens_out.push(NSScreen(screen));
            }
            return screens_out;
        }
    }

    fn device_description(&self) -> NSDictionary {
        unsafe { NSDictionary::from_id_retained(msg_send![self.0, deviceDescription]) }
    }

    pub(crate) fn dpi(&self) -> f64 {
        let ns_screen_number_string = NSString::new("NSScreenNumber");
        let device_description = self.device_description();
        let pixel_size_value = NSValue(unsafe { device_description.value_for_key(NSDeviceSize) });
        if pixel_size_value.0.is_null() {
            std::mem::forget(pixel_size_value);
            return 72.0;
        }
        let pixel_size = pixel_size_value.size_value();
        let screen_number_ptr = device_description.value_for_key(ns_screen_number_string.0 as CFStringRef);
        if screen_number_ptr.is_null() {
            return 72.0;
        }
        let screen_number_num = NSNumber::from_id_unretained(screen_number_ptr);
        let screen_number = screen_number_num.as_i32() as u32;
        let physical_size = unsafe { CGDisplayScreenSize(screen_number) };
        let mut backing_scale_factor: f64 = unsafe { msg_send![self.0, backingScaleFactor] };
        if backing_scale_factor == 0.0 {
            backing_scale_factor = 1.0;
        }
        std::mem::forget(pixel_size_value);
        std::mem::forget(screen_number_num);
        std::mem::forget(device_description);
        (pixel_size.x / physical_size.x) * 25.4 * backing_scale_factor as f64
    }

    pub(crate) fn frame(&self) -> CGRect {
        unsafe { msg_send![self.0, frame] }
    }
}

#[derive(Debug)]
pub struct CGImage(CGImageRef);

unsafe impl Send for CGImage {}

impl CGImage {
    pub fn from_ref_unretained(r: CGImageRef) -> Self {
        unsafe { CGImageRetain(r); }
        Self(r)
    }

    pub fn from_ref_retained(r: CGImageRef) -> Self {
        Self(r)
    }

    pub fn width(&self) -> usize {
        unsafe { CGImageGetWidth(self.0) }
    }

    pub fn height(&self) -> usize {
        unsafe { CGImageGetHeight(self.0) }
    }

    pub fn bits_per_component(&self) -> usize {
        unsafe { CGImageGetBitsPerComponent(self.0) }
    }

    pub fn bits_per_pixel(&self) -> usize {
        unsafe { CGImageGetBitsPerPixel(self.0) }
    }

    pub fn bytes_per_row(&self) -> usize {
        unsafe { CGImageGetBytesPerRow(self.0) }
    }

    pub fn get_bitmap_info(&self) -> CGBitmapInfo {
        unsafe {
            let bitmap_info_raw = CGImageGetBitmapInfo(self.0);
            CGBitmapInfo {
                alpha: match bitmap_info_raw & kCGBitmapInfoAlphaMask {
                    kCGImageAlphaFirst => Some(CGBitmapAlphaInfo::First),
                    kCGImageAlphaLast => Some(CGBitmapAlphaInfo::Last),
                    kCGImageAlphaNoneSkipFirst => Some(CGBitmapAlphaInfo::NoneSkipFirst),
                    kCGImageAlphaNoneSkipLast => Some(CGBitmapAlphaInfo::NoneSkipLast),
                    kCGImageAlphaPremultipliedFirst => Some(CGBitmapAlphaInfo::PremultipliedFirst),
                    kCGImageAlphaPremultipliedLast => Some(CGBitmapAlphaInfo::PremultipliedLast),
                    _ => None,
                },
                byte_order: match bitmap_info_raw & kCGBitmapInfoByteOrderMask {
                    kCGBitmapInfoByteOrder16Little => CGBitmapByteOrder::B16Little,
                    kCGBitmapInfoByteOrder16Big    => CGBitmapByteOrder::B16Big,
                    kCGBitmapInfoByteOrder32Little => CGBitmapByteOrder::B32Little,
                    kCGBitmapInfoByteOrder32Big    => CGBitmapByteOrder::B32Big,
                    _                              => CGBitmapByteOrder::B8,
                },
                float: (bitmap_info_raw & kCGBitmapFloatInfoMask) != 0
            }
        }
    }

    pub fn get_pixel_format(&self) -> CGImagePixelFormat {
        unsafe {
            let pixel_format_raw = CGImageGetPixelFormatInfo(self.0);
            match pixel_format_raw & kCGImagePixelFormatMask {
                kCGImagePixelFormatRGB101010 => CGImagePixelFormat::Rgb101010,
                kCGImagePixelFormatRGB555    => CGImagePixelFormat::Rgb555,
                kCGImagePixelFormatRGB565    => CGImagePixelFormat::Rgb565,
                kCGImagePixelFormatRGB101010 => CGImagePixelFormat::RgbCif10,
                _                            => CGImagePixelFormat::Packed,
            }
        }
    }

    pub fn get_data_provider(&self) -> CGDataProvider {
        unsafe {
            let dataprovider_ref = CGImageGetDataProvider(self.0);
            CGDataProvider::from_ref_unretained(dataprovider_ref)
        }
    }
}

impl Clone for CGImage {
    fn clone(&self) -> Self {
        Self::from_ref_unretained(self.0)
    }
}

impl Drop for CGImage {
    fn drop(&mut self) {
        unsafe { CGImageRelease(self.0); }
    }
}

const kCGBitmapInfoAlphaMask: u32 = 0x1F;
const kCGBitmapInfoByteOrderMask: u32 = 7 << 12;
const kCGBitmapFloatInfoMask: u32 = 0xF << 8;
const kCGImagePixelFormatMask: u32 = 0xF << 16;

const kCGBitmapInfoFloatComponents: u32 = 1 << 8;

const kCGBitmapInfoByteOrder16Little: u32 = 1 << 12;
const kCGBitmapInfoByteOrder16Big: u32 = 3 << 12;
const kCGBitmapInfoByteOrder32Little: u32 = 2 << 12;
const kCGBitmapInfoByteOrder32Big: u32 = 4 << 12;

const kCGImageAlphaNone: u32 = 0;
const kCGImageAlphaPremultipliedLast: u32 = 1;
const kCGImageAlphaPremultipliedFirst: u32 = 2;
const kCGImageAlphaLast: u32 = 3;
const kCGImageAlphaFirst: u32 = 4;
const kCGImageAlphaNoneSkipLast: u32 = 5;
const kCGImageAlphaNoneSkipFirst: u32 = 6;
const kCGImageAlphaOnly: u32 = 7;

const kCGImagePixelFormatPacked    : u32 = 0 << 16;
const kCGImagePixelFormatRGB555    : u32 = 1 << 16;
const kCGImagePixelFormatRGB565    : u32 = 2 << 16;
const kCGImagePixelFormatRGB101010 : u32 = 3 << 16;
const kCGImagePixelFormatRGBCIF10  : u32 = 4 << 16;

pub enum CGBitmapAlphaInfo {
    PremultipliedLast,
    PremultipliedFirst,
    Last,
    First,
    NoneSkipLast,
    NoneSkipFirst,
    AlphaOnly
}

pub enum CGBitmapByteOrder {
    B8,
    B16Little,
    B16Big,
    B32Little,
    B32Big,
}

pub enum CGImagePixelFormat {
    Packed,
    Rgb555,
    Rgb565,
    Rgb101010,
    RgbCif10,
}

pub struct CGBitmapInfo {
    alpha: Option<CGBitmapAlphaInfo>,
    byte_order: CGBitmapByteOrder,
    float: bool,
}

pub struct CGDataProvider(CGDataProviderRef);

impl CGDataProvider {
    fn from_ref_unretained(r: CGDataProviderRef) -> Self {
        unsafe { CGDataProviderRetain(r); }
        Self(r)
    }

    fn from_ref_retained(r: CGDataProviderRef) -> Self {
        Self(r)
    }

    fn copy_data(&self) -> CFData {
        unsafe {
            let cfdata_ref = CGDataProviderCopyData(self.0);
            CFData::from_ref_retained(cfdata_ref)
        }
    }
}

impl Clone for CGDataProvider {
    fn clone(&self) -> Self {
        Self::from_ref_unretained(self.0)
    }
}

impl Drop for CGDataProvider {
    fn drop(&mut self) {
        unsafe { CGDataProviderRelease(self.0); }
    }
}

struct CFData(CFDataRef);

impl CFData {
    fn from_ref_unretained(r: CGDataProviderRef) -> Self {
        unsafe { CFRetain(r); }
        Self(r)
    }

    fn from_ref_retained(r: CGDataProviderRef) -> Self {
        Self(r)
    }
}

impl Drop for CFData {
    fn drop(&mut self) {
        unsafe { CFRelease(self.0); }
    }
}

pub const SCContentSharingPickerModeSingleWindow         : usize = 1 << 0;
pub const SCContentSharingPickerModeMultipleWindows      : usize = 1 << 1;
pub const SCContentSharingPickerModeSingleApplication    : usize = 1 << 2;
pub const SCContentSharingPickerModeMultipleApplications : usize = 1 << 3;
pub const SCContentSharingPickerModeSingleDisplay        : usize = 1 << 4;

pub struct SCContentSharingPickerConfiguration(*mut AnyObject);

impl SCContentSharingPickerConfiguration {
    pub fn new() -> Self {
        unsafe {
            let id: *mut AnyObject = msg_send![class!(SCContentSharingPickerConfiguration), alloc];
            let id: *mut AnyObject = msg_send![id, init];
            Self::from_id_retained(id)
        }
    }

    pub fn from_id_retained(id: *mut AnyObject) -> Self {
        Self(id)
    }

    pub fn from_id_unretained(id: *mut AnyObject) -> Self {
        unsafe {
            let _: *mut AnyObject = msg_send![id, retain];
        }
        Self(id)
    }

    pub fn set_changing_selected_content_allowed(&self, allowed: bool) {
        unsafe {
            let _: () = msg_send![self.0, setAllowsChangingSelectedContent: allowed];
        }
    }

    pub fn set_allowed_picker_modes(&self, allowed_picker_modes: usize) {
        unsafe {
            let _: () = msg_send![self.0, setAllowedPickerModes: allowed_picker_modes];
        }
    }
    
}

impl Drop for SCContentSharingPickerConfiguration {
    fn drop(&mut self) {
        unsafe {
            let _: () = msg_send![self.0, release];
        }
    }
}

impl Clone for SCContentSharingPickerConfiguration {
    fn clone(&self) -> Self {
        Self::from_id_unretained(self.0)
    }
}

#[repr(C)]
pub struct SCContentSharingPickerObserver(*mut AnyObject);

unsafe impl Encode for SCContentSharingPickerObserver {
    const ENCODING: Encoding = Encoding::Object;
}

extern fn sc_content_sharing_picker_observer_did_cancel_for_stream(this: *mut AnyObject, _sel: Sel, picker: *mut AnyObject, stream: *mut AnyObject) {
    //println!("sc_content_sharing_picker_observer_did_cancel_for_stream");
}

extern fn sc_content_sharing_picker_observer_did_update_filter_for_stream(this: *mut AnyObject, _sel: Sel, picker: *mut AnyObject, filter: *mut AnyObject, stream: *mut AnyObject) {
    //println!("sc_content_sharing_picker_observer_did_update_filter_for_stream");
    //debug_objc_AnyObject(filter);
}

extern fn sc_content_sharing_picker_observer_start_did_fail_with_error(this: *mut AnyObject, _sel: Sel, error: *mut AnyObject) {
    //println!("sc_content_sharing_picker_observer_start_did_fail_with_error");
}

extern fn sc_content_sharing_picker_observer_dealloc(this: *mut AnyObject, _sel: Sel) {
    unsafe {
        
        let callback_container: Box<SCContentSharingPickerCallbackContainer> = Box::from_raw(*(&*this).class().instance_variable("callback_container_ptr").unwrap().load::<*mut c_void>(&*this) as *mut SCContentSharingPickerCallbackContainer);
        drop(callback_container);
    }
}

impl SCContentSharingPickerObserver {
    fn get_class() -> &'static AnyClass {
        unsafe {
            if let Some(mut class) = ClassBuilder::new("SCContentSharingPickerObserverImpl", class!(NSAnyObject)) {
                class.add_method(sel!(contentSharingPicker:didCancelForStream:), sc_content_sharing_picker_observer_did_cancel_for_stream as extern fn (*mut AnyObject, Sel, *mut AnyObject, *mut AnyObject));
                class.add_method(sel!(contentSharingPicker:didUpdateWithFilter:forStream:), sc_content_sharing_picker_observer_did_update_filter_for_stream as extern fn (*mut AnyObject, Sel, *mut AnyObject, *mut AnyObject, *mut AnyObject));
                class.add_method(sel!(contentSharingPickerStartDidFailWithError:), sc_content_sharing_picker_observer_start_did_fail_with_error as extern fn (*mut AnyObject, Sel, *mut AnyObject));
                class.add_method(sel!(dealloc), sc_content_sharing_picker_observer_dealloc as extern fn (*mut AnyObject, Sel));

                class.add_ivar::<*mut c_void>("callback_container_ptr");
                
                class.register()
            } else {
                class!(SCContentSharingPickerObserverImpl)
            }
        }
    }

    pub fn from_id_retained(id: *mut AnyObject) -> Self {
        Self(id)
    }

    pub fn from_id_unretained(id: *mut AnyObject) -> Self {
        unsafe {
            let _: *mut AnyObject = msg_send![id, retain];
        }
        Self(id)
    }

    pub fn new(callback: impl FnMut(Result<SCContentSharingPickerEvent, NSError>) + Send + 'static) -> Self {
        unsafe {
            let callback_container = Box::new(SCContentSharingPickerCallbackContainer::new(callback));
            let callback_container_ptr = Box::leak(callback_container) as *mut _ as *mut c_void;

            let class = Self::get_class();
            let id: *mut AnyObject = msg_send![class, alloc];
            let id: *mut AnyObject = msg_send![id, init];
            let callback_container_ptr_ivar = class!(SCShareableContent).instance_variable("callback_container_ptr").expect("Expected callback_container_ptr ivar on SCContentSharingPickerObserver");
            *callback_container_ptr_ivar.load_mut(&mut *id) = callback_container_ptr;

            Self(id)
        }
    }
}

impl Drop for SCContentSharingPickerObserver {
    fn drop(&mut self) {
        unsafe {
            let _: () = msg_send![self.0, release];
        }
    }
}

impl Clone for SCContentSharingPickerObserver {
    fn clone(&self) -> Self {
        Self::from_id_unretained(self.0)
    }
}

#[derive(Debug)]
pub enum SCContentSharingPickerEvent {
    DidUpdate {filter: SCContentFilter, stream: Option<SCStream> },
    Cancelled,
}

#[repr(C)]
struct SCContentSharingPickerCallbackContainer {
    callback: Box<dyn FnMut(Result<SCContentSharingPickerEvent, NSError>) + Send + 'static>
}

impl SCContentSharingPickerCallbackContainer {
    pub fn new(callback: impl FnMut(Result<SCContentSharingPickerEvent, NSError>) + Send + 'static) -> Self {
        Self {
            callback: Box::new(callback)
        }
    }

    pub fn call_cancelled(&mut self, picker: SCContentSharingPicker, stream: Option<SCStream>) {
    }

    pub fn call_did_update_with_filter(&mut self, picker: SCContentSharingPicker, filter: SCContentFilter, stream: Option<SCStream>) {
    }

    pub fn call_error(&mut self, error: NSError) {
    }
}

pub enum SCShareableContentStyle {
    Application,
    Display,
    Window,
    None
}

pub struct SCContentSharingPicker(*mut AnyObject);

impl SCContentSharingPicker {
    pub fn shared() -> Self {
        unsafe {
            let id: *mut AnyObject = msg_send![class!(SCContentSharingPicker), sharedPicker];
            Self::from_id_unretained(id)
        }
    }

    fn from_id_retained(id: *mut AnyObject) -> Self {
        Self(id)
    }

    fn from_id_unretained(id: *mut AnyObject) -> Self {
        unsafe {
            let _: *mut AnyObject = msg_send![id, retain];
        }
        Self(id)
    }

    pub fn set_active(&self, active: bool) {
        unsafe {
            let _: () = msg_send![self.0, setActive: active];
        }
    }

    pub fn add(&self, observer: SCContentSharingPickerObserver) {
        unsafe {
            let _: () = msg_send![self.0, addObserver: observer];
        }
    }

    pub fn remove(&self, observer: SCContentSharingPickerObserver) {
        unsafe {
            let _: () = msg_send![self.0, removeObserver: observer];
        }
    }

    pub fn present(&self) {
        unsafe {
            let _: () = msg_send![self.0, present];
        }
    }

    pub fn present_using_content_style(&self, content_style: SCShareableContentStyle) {
        let style: usize = match content_style {
            SCShareableContentStyle::Application => 3,
            SCShareableContentStyle::Window => 1,
            SCShareableContentStyle::Display => 2,
            SCShareableContentStyle::None => 0,
        };
        unsafe {
            let _: () = msg_send![self.0, presentPickerUsingContentStyle: style];
        }
    }

    pub fn set_configuration_for_stream(&self, configuration: SCContentSharingPickerConfiguration, for_stream: Option<&SCStream>) {
        unsafe {
            if let Some(stream) = for_stream {
                let _: () = msg_send![self.0, setConfiguration: configuration.0 forStream: stream.0];
            } else {
                let _: () = msg_send![self.0, setConfiguration: configuration.0];
            }
        }
    }
}

impl Drop for SCContentSharingPicker {
    fn drop(&mut self) {
        unsafe {
            let _: () = msg_send![self.0, release];
        }
    }
}

impl Clone for SCContentSharingPicker {
    fn clone(&self) -> Self {
        Self::from_id_unretained(self.0)
    }
}

pub struct SCScreenshotManager();

unsafe impl Send for SCScreenshotManager {}

impl SCScreenshotManager {
    pub fn class_exists() -> bool {
        AnyClass::get("SCScreenshotManager").is_some()
    }

    pub fn capture_image_with_filter_and_configuration(filter: &SCContentFilter, config: &SCStreamConfiguration, completion_handler: impl FnMut(Result<CGImage, NSError>) + Send + 'static) {
        let completion_handler = Mutex::new(completion_handler);
        let completion_block = RcBlock::new(move |image: CGImageRef, error: *mut AnyObject| {
            if error.is_null() {
                (completion_handler.lock())(Ok(CGImage::from_ref_unretained(image)));
            } else {
                (completion_handler.lock())(Err(NSError::from_id_unretained(error)));
            }
        });
        unsafe {
            let _: () = msg_send![
                class!(SCScreenshotManager),
                captureImageWithFilter: filter.0
                configuration: config.0
                completionHandler: &*completion_block
            ];
        }
    }

    pub fn capture_samplebuffer_with_filter_and_configuration(filter: SCContentFilter, config: SCStreamConfiguration, completion_handler: impl FnMut(Result<CMSampleBuffer, String>) + Send + 'static) {
        unsafe {
            let completion_handler = Arc::new(Mutex::new(completion_handler));
            let completion_block = StackBlock::new(move |sample_buffer: CMSampleBufferRef, error: *mut AnyObject| {
                if error.is_null() {
                    (completion_handler.lock())(
                        CMSampleBuffer::copy_from_ref(sample_buffer)
                            .map_err(|_| "Failed to copy sample buffer".to_string())
                    );
                } else {
                    let error = NSError::from_id_unretained(error).description();
                    (completion_handler.lock())(
                        Err(error)
                    );
                }
            }).copy();
            let mut error: *mut AnyObject = std::ptr::null_mut();
            let _: () = msg_send![
                class!(SCScreenshotManager),
                captureSampleBufferWithFilter: filter.0
                configuration: config.0
                completionHandler: &*completion_block
            ];
        }
    }
}

pub(crate) struct WindowDescription {
    pub window_layer: i32,
}

pub(crate) fn get_window_description(window: CGWindowID) -> Result<WindowDescription, ()> {
    let mut window_array = CFArray::new_mutable(None, None)?;
    window_array.append_value(window.0 as isize as *const c_void);
    unsafe {
        let description_array = CGWindowListCreateDescriptionFromArray(window_array.0);
        if description_array.is_null() {
            return Err(())
        }
        let descriptions = CFArray::from_ref_retained(description_array);
        if descriptions.get_count() == 0 {
            return Err(());
        }
        let description_dictionary = descriptions.get_value_at_index(0);
        if description_dictionary.is_null() {
            return Err(());
        }
        let description = CFDictionary::from_ref_unretained(description_dictionary);
        let window_layer_nsnumber = description.get_value(kCGWindowLayer);
        if window_layer_nsnumber.is_null() {
            return Err(());
        }
        let window_layer = NSNumber::from_id_unretained(window_layer_nsnumber as *mut AnyObject);
        
        Ok(WindowDescription {
            window_layer: window_layer.as_i32(),
        })
    }
}

#[derive(Copy, Clone, Debug)]
pub struct WindowLevels {
    pub base                : i32,
    pub minimum             : i32,
    pub desktop             : i32,
    pub desktop_icon        : i32,
    pub backstop            : i32,
    pub normal              : i32,
    pub floating            : i32,
    pub torn_off_menu       : i32,
    pub modal_panel         : i32,
    pub utility             : i32,
    pub dock                : i32,
    pub main_menu           : i32,
    pub status              : i32,
    pub pop_up_menu         : i32,
    pub overlay             : i32,
    pub help                : i32,
    pub dragging            : i32,
    pub screen_saver        : i32,
    pub assistive_tech_high : i32,
    pub cursor              : i32,
    pub maximum             : i32,
}

const kCGBaseWindowLevelKey              : i32 =  0;
const kCGMinimumWindowLevelKey           : i32 =  1;
const kCGDesktopWindowLevelKey           : i32 =  2;
const kCGBackstopMenuLevelKey            : i32 =  3;
const kCGNormalWindowLevelKey            : i32 =  4;
const kCGFloatingWindowLevelKey          : i32 =  5;
const kCGTornOffMenuWindowLevelKey       : i32 =  6;
const kCGDockWindowLevelKey              : i32 =  7;
const kCGMainMenuWindowLevelKey          : i32 =  8;
const kCGStatusWindowLevelKey            : i32 =  9;
const kCGModalPanelWindowLevelKey        : i32 = 10;
const kCGPopUpMenuWindowLevelKey         : i32 = 11;
const kCGDraggingWindowLevelKey          : i32 = 12;
const kCGScreenSaverWindowLevelKey       : i32 = 13;
const kCGMaximumWindowLevelKey           : i32 = 14;
const kCGOverlayWindowLevelKey           : i32 = 15;
const kCGHelpWindowLevelKey              : i32 = 16;
const kCGUtilityWindowLevelKey           : i32 = 17;
const kCGDesktopIconWindowLevelKey       : i32 = 18;
const kCGCursorWindowLevelKey            : i32 = 19;
const kCGAssistiveTechHighWindowLevelKey : i32 = 20;

fn get_window_level_for_key(key: i32) -> i32 {
    unsafe { CGWindowLevelForKey(key) }
}

lazy_static! {
    static ref WINDOW_LEVELS: WindowLevels = {
        WindowLevels {
            base                : get_window_level_for_key(kCGBaseWindowLevelKey),
            minimum             : get_window_level_for_key(kCGMinimumWindowLevelKey),
            desktop             : get_window_level_for_key(kCGDesktopWindowLevelKey),
            desktop_icon        : get_window_level_for_key(kCGDesktopIconWindowLevelKey),
            backstop            : get_window_level_for_key(kCGBackstopMenuLevelKey),
            normal              : get_window_level_for_key(kCGNormalWindowLevelKey),
            floating            : get_window_level_for_key(kCGFloatingWindowLevelKey),
            torn_off_menu       : get_window_level_for_key(kCGTornOffMenuWindowLevelKey),
            dock                : get_window_level_for_key(kCGDockWindowLevelKey),
            main_menu           : get_window_level_for_key(kCGMainMenuWindowLevelKey),
            status              : get_window_level_for_key(kCGStatusWindowLevelKey),
            modal_panel         : get_window_level_for_key(kCGModalPanelWindowLevelKey),
            pop_up_menu         : get_window_level_for_key(kCGPopUpMenuWindowLevelKey),
            dragging            : get_window_level_for_key(kCGDraggingWindowLevelKey),
            screen_saver        : get_window_level_for_key(kCGScreenSaverWindowLevelKey),
            maximum             : get_window_level_for_key(kCGMaximumWindowLevelKey),
            overlay             : get_window_level_for_key(kCGOverlayWindowLevelKey),
            help                : get_window_level_for_key(kCGHelpWindowLevelKey),
            utility             : get_window_level_for_key(kCGUtilityWindowLevelKey),
            cursor              : get_window_level_for_key(kCGCursorWindowLevelKey),
            assistive_tech_high : get_window_level_for_key(kCGAssistiveTechHighWindowLevelKey),
        }
    };
}

pub(crate) fn get_window_levels() -> &'static WindowLevels {
    &*WINDOW_LEVELS
}

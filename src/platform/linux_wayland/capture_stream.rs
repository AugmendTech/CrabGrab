use std::{
    cell::RefCell,
    ffi::{c_void, CString},
    mem::size_of,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
        Arc,
    },
    thread::JoinHandle,
    time::Duration,
};

use pipewire::{
    context::Context,
    main_loop::MainLoop,
    spa::{
        self,
        param::{
            self,
            format::{self, MediaSubtype, MediaType},
            video::{VideoFormat, VideoInfoRaw},
            ParamType,
        },
        pod::{self, Object, Pod, Property},
        sys::{
            spa_buffer, spa_meta_bitmap, spa_meta_cursor, spa_meta_header, SPA_META_Cursor,
            SPA_META_Header, SPA_PARAM_META_size, SPA_PARAM_META_type, SPA_LOG_LEVEL_TRACE,
        },
        utils::{ChoiceFlags, Direction, Fraction, Rectangle, SpaTypes},
    },
    stream::{Stream, StreamFlags, StreamRef, StreamState},
};

use crate::{
    frame::VideoFrame,
    prelude::{
        CaptureConfig, CapturePixelFormat, StreamCreateError, StreamError, StreamEvent,
        StreamStopError,
    },
};

use super::frame::WaylandVideoFrame;

const INVALID_CURSOR_ID: u32 = 0;

macro_rules! cursor_metadata_size {
    ($w:expr, $h:expr) => {
        (size_of::<spa_meta_cursor>() + size_of::<spa_meta_bitmap>() + $w * $h * 4) as i32
    };
}

fn serialize_pod_object(obj: Object) -> Result<Vec<u8>, StreamCreateError> {
    let vals: Vec<u8> = pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &pod::Value::Object(obj),
    )
    .map_err(|e| StreamCreateError::Other(e.to_string()))?
    .0
    .into_inner();

    Ok(vals)
}

#[derive(Debug, PartialEq)]
struct CursorBitmap {
    pub format: VideoFormat,
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub bytes_per_pixel: usize,
}

#[derive(Default, Debug, PartialEq)]
struct PwMetas<'a> {
    pub header: Option<&'a spa_meta_header>,
    pub cursor: Option<&'a spa_meta_cursor>,
    pub cursor_bitmap: Option<CursorBitmap>,
}

impl<'a> PwMetas<'a> {
    pub unsafe fn from_raw(raw: &'a *mut spa_buffer) -> Self {
        let mut metas = Self::default();
        for meta in std::slice::from_raw_parts((*(*raw)).metas, (*(*raw)).n_metas as usize) {
            match meta.type_ {
                #[allow(non_upper_case_globals)]
                SPA_META_Header => {
                    metas.header = Some(&*(meta.data as *const spa_meta_header));
                }
                #[allow(non_upper_case_globals)]
                SPA_META_Cursor => {
                    let cursor = &*(meta.data as *const spa_meta_cursor);
                    // Cursor bitmap are only sent when the cursor sprite is different from the previous
                    if cursor.id != INVALID_CURSOR_ID && cursor.bitmap_offset > 0 {
                        let bitmap = (cursor as *const spa_meta_cursor)
                            .byte_offset(cursor.bitmap_offset as isize)
                            as *const spa_meta_bitmap;
                        let bitmap_data = std::slice::from_raw_parts(
                            (bitmap as *const u8).byte_offset((*bitmap).offset as isize),
                            (*bitmap).size.height as usize * (*bitmap).stride as usize,
                        );
                        metas.cursor_bitmap = Some(CursorBitmap {
                            format: param::video::VideoFormat((*bitmap).format),
                            data: bitmap_data.to_vec(),
                            width: (*bitmap).size.width,
                            height: (*bitmap).size.height,
                            bytes_per_pixel: (*bitmap).stride as usize / (*bitmap).size.width as usize,
                        });
                    }

                    metas.cursor = Some(cursor);
                }
                _ => {}
            }
        }

        metas
    }
}

#[derive(Default)]
struct PwDatas<'a> {
    pub data: &'a [u8],
}

impl<'a> PwDatas<'a> {
    pub unsafe fn from_raw(raw: &'a *mut spa_buffer) -> Vec<PwDatas<'a>> {
        std::slice::from_raw_parts((*(*raw)).datas, (*(*raw)).n_datas as usize)
            .iter()
            .map(|data| PwDatas {
                data: std::slice::from_raw_parts(data.data as *mut u8, data.maxsize as usize),
            })
            .collect::<Vec<PwDatas<'a>>>()
    }
}

struct WaylandCapturerUD {
    pub format: VideoInfoRaw,
    pub format_negotiating: Rc<RefCell<bool>>,
    pub show_cursor_as_metadata: bool,
    pub start_time: i64,
    pub callback: Box<dyn FnMut(Result<StreamEvent, StreamError>) + Send + 'static>,
    pub should_run: Arc<AtomicBool>,
    pub cursor_bitmap: Option<CursorBitmap>,
}

pub struct WaylandCaptureStream {
    handle: Option<JoinHandle<()>>,
    should_run: Arc<AtomicBool>,
}

impl WaylandCaptureStream {
    pub fn supported_pixel_formats() -> &'static [CapturePixelFormat] {
        &[CapturePixelFormat::Bgra8888]
    }

    pub fn check_access(_borderless: bool) -> Option<WaylandCaptureAccessToken> {
        Some(WaylandCaptureAccessToken(()))
    }

    pub async fn request_access(_borderless: bool) -> Option<WaylandCaptureAccessToken> {
        Some(WaylandCaptureAccessToken(()))
    }

    fn pod_supported_pixel_formats() -> pod::Property {
        pod::property!(
            format::FormatProperties::VideoFormat,
            Choice,
            Enum,
            Id,
            VideoFormat::BGRx, // Big-endian
            VideoFormat::BGRA, // Big-endian
            VideoFormat::RGBA, // Big-endian
            VideoFormat::RGBx, // Big-endian
        )
    }

    fn pod_supported_resolutions() -> pod::Property {
        pod::property!(
            format::FormatProperties::VideoSize,
            Choice,
            Range,
            Rectangle,
            Rectangle {
                width: 512,
                height: 512
            },
            Rectangle {
                width: 1,
                height: 1
            },
            Rectangle {
                width: 15360,
                height: 8640
            }
        )
    }

    fn pod_supported_framerates() -> pod::Property {
        pod::property!(
            format::FormatProperties::VideoFramerate,
            Choice,
            Range,
            Fraction,
            Fraction { num: 30, denom: 1 },
            Fraction { num: 0, denom: 1 },
            Fraction { num: 512, denom: 1 }
        )
    }

    fn state_changed(
        _stream: &StreamRef,
        ud: &mut WaylandCapturerUD,
        _old: StreamState,
        new: StreamState,
    ) {
        match new {
            StreamState::Error(e) => {
                (*ud.callback)(Err(StreamError::Other(e)));
                ud.should_run.store(false, Ordering::SeqCst);
            }
            StreamState::Unconnected => {
                (*ud.callback)(Ok(StreamEvent::End));
                ud.should_run.store(false, Ordering::SeqCst);
            }
            _ => {}
        }
    }

    fn param_changed(stream: &StreamRef, ud: &mut WaylandCapturerUD, id: u32, param: Option<&Pod>) {
        let Some(param) = param else {
            return;
        };

        use pipewire::spa::param;
        if id != ParamType::Format.as_raw() {
            return;
        }

        match param::format_utils::parse_format(param) {
            Ok((media_type, media_subtype)) => {
                if media_type != MediaType::Video || media_subtype != MediaSubtype::Raw {
                    return;
                }
            }
            Err(e) => {
                unsafe {
                    pipewire::sys::pw_stream_set_error(
                        stream.as_raw_ptr(),
                        -1,
                        CString::new(e.to_string()).unwrap().as_c_str().as_ptr(),
                    );
                }
                return;
            }
        };

        let mut params = Vec::new();

        let mcursor_obj = pod::object!(
            SpaTypes::ObjectParamMeta,
            ParamType::Meta,
            Property::new(
                SPA_PARAM_META_type,
                pod::Value::Id(spa::utils::Id(SPA_META_Cursor))
            ),
            Property::new(
                SPA_PARAM_META_size,
                pod::Value::Choice(pod::ChoiceValue::Int(spa::utils::Choice::<i32>(
                    ChoiceFlags::empty(),
                    spa::utils::ChoiceEnum::Range {
                        default: cursor_metadata_size!(64, 64),
                        min: cursor_metadata_size!(1, 1),
                        max: cursor_metadata_size!(512, 512)
                    }
                )))
            )
        );
        let mcursor_values = serialize_pod_object(mcursor_obj).unwrap();
        params.push(pod::Pod::from_bytes(&mcursor_values).unwrap());

        let mheader_obj = pod::object!(
            SpaTypes::ObjectParamMeta,
            ParamType::Meta,
            Property::new(
                SPA_PARAM_META_type,
                pod::Value::Id(spa::utils::Id(SPA_META_Header))
            ),
            Property::new(
                SPA_PARAM_META_size,
                pod::Value::Int(size_of::<spa::sys::spa_meta_header>() as i32)
            ),
        );
        let mheader_values = serialize_pod_object(mheader_obj).unwrap();
        params.push(pod::Pod::from_bytes(&mheader_values).unwrap());

        if let Err(e) = stream.update_params(&mut params) {
            unsafe {
                pipewire::sys::pw_stream_set_error(
                    stream.as_raw_ptr(),
                    -1,
                    CString::new(e.to_string()).unwrap().as_c_str().as_ptr(),
                );
            }
            return;
        }

        ud.format.parse(param).unwrap();
        println!(
            // DEBUGGING
            "Got pixel format: {} ({:?})",
            ud.format.format().as_raw(),
            ud.format.format()
        );

        ud.format_negotiating.replace(false);
    }

    fn process(stream: &StreamRef, ud: &mut WaylandCapturerUD) {
        let raw_buffer = unsafe { stream.dequeue_raw_buffer() };
        if raw_buffer.is_null() {
            unsafe { stream.queue_raw_buffer(raw_buffer) };
            return;
        }

        let buffer = unsafe { (*raw_buffer).buffer };
        if buffer.is_null() {
            unsafe { stream.queue_raw_buffer(raw_buffer) };
            return;
        }

        let (metas, datas) = unsafe { (PwMetas::from_raw(&buffer), PwDatas::from_raw(&buffer)) };
        if let (Some(header), Some(data)) = (metas.header, datas.first()) {
            if ud.start_time == 0 {
                ud.start_time = header.pts;
            }

            if metas.cursor_bitmap.is_some() {
                ud.cursor_bitmap = metas.cursor_bitmap;
            }

            // Very expensive
            // let mut pixel_data = data.data.to_vec();
            // 'out: {
            //     if ud.show_cursor_as_metadata {
            //         if let (Some(cursor), Some(bitmap)) = (metas.cursor, ud.cursor_bitmap.as_ref())
            //         {
            //             if cursor.position.y < 0
            //                 || cursor.position.x < 0
            //                 || bitmap.format == ud.format.format() {
            //                 break 'out;
            //             }

            //             TODO: Use cursor.hotspot
            //             let mut bmap_iter = bitmap.data.iter();
            //             let mut h = cursor.position.y as usize;
            //             let mut height_max = std::cmp::min(
            //                 cursor.position.y as usize + bitmap.height as usize,
            //                 ud.format.size().height as usize,
            //             );
            //             let mut width_max = std::cmp::min(
            //                 cursor.position.x as usize + bitmap.width as usize,
            //                 ud.format.size().width as usize,
            //             );
            //             let stride = 4 * ud.format.size().width as usize;
            //             while h < height_max {
            //                 let mut w = cursor.position.x as usize;
            //                 while w < width_max {
            //                     if bitmap.bytes_per_pixel != 4 {
            //                         continue;
            //                     }
            //                     let pix_index = h as usize * stride + w as usize * 4;
            //                     let b = *bmap_iter.next().unwrap();
            //                     let g = *bmap_iter.next().unwrap();
            //                     let r = *bmap_iter.next().unwrap();
            //                     let a = *bmap_iter.next().unwrap();
            //                     if a > 0 {
            //                         pixel_data[pix_index + 0] = b;
            //                         pixel_data[pix_index + 1] = g;
            //                         pixel_data[pix_index + 2] = r;
            //                         pixel_data[pix_index + 3] = a;
            //                     }
            //                     w += 1;
            //                 }

            //                 h += 1;
            //             }
            //         }
            //     }
            // }

            let frame = WaylandVideoFrame {
                size: crate::prelude::Size {
                    width: ud.format.size().width as f64,
                    height: ud.format.size().height as f64,
                },
                id: header.seq,
                captured: std::time::Instant::now(),
                pts: std::time::Duration::from_nanos((header.pts - ud.start_time) as u64),
                format: ud.format,
                data: data.data.as_ptr() as *const c_void,
            };

            (*ud.callback)(Ok(StreamEvent::Video(VideoFrame {
                impl_video_frame: frame,
            })));
        }

        unsafe { stream.queue_raw_buffer(raw_buffer) };
    }

    fn capture_main(
        capture_config: CaptureConfig,
        callback: Box<impl FnMut(Result<StreamEvent, StreamError>) + Send + 'static>,
        should_run: Arc<AtomicBool>,
        init_tx: &Sender<Result<(), StreamCreateError>>,
    ) -> Result<(), StreamCreateError> {
        pipewire::init();
        unsafe {
            // DEBUGGING
            pipewire::sys::pw_log_set_level(SPA_LOG_LEVEL_TRACE);
        }

        let mainloop = MainLoop::new(None).map_err(|e| StreamCreateError::Other(e.to_string()))?;
        let context =
            Context::new(&mainloop).map_err(|e| StreamCreateError::Other(e.to_string()))?;
        let core = context
            .connect(None)
            .map_err(|e| StreamCreateError::Other(e.to_string()))?;

        use pipewire::keys;
        let stream = Stream::new(
            &core,
            "crabgrab",
            pipewire::properties::properties! {
                *keys::MEDIA_TYPE => "Video",
                *keys::MEDIA_CATEGORY => "Capture",
                *keys::MEDIA_ROLE => "Screen",
            },
        )
        .map_err(|e| StreamCreateError::Other(e.to_string()))?;

        let format_negotiating = Rc::new(RefCell::new(true));
        let user_data = WaylandCapturerUD {
            format: Default::default(),
            format_negotiating: format_negotiating.clone(),
            show_cursor_as_metadata: capture_config.show_cursor
                && match &capture_config.target {
                    crate::prelude::Capturable::Window(w) => {
                        w.impl_capturable_window.cursor_as_metadata
                    }
                    crate::prelude::Capturable::Display(d) => {
                        d.impl_capturable_display.cursor_as_metadata
                    }
                },
            start_time: 0,
            callback,
            should_run: Arc::clone(&should_run),
            cursor_bitmap: None,
        };

        let _listener = stream
            .add_local_listener_with_user_data(user_data)
            .state_changed(Self::state_changed)
            .param_changed(Self::param_changed)
            .process(Self::process)
            .register()
            .map_err(|e| StreamCreateError::Other(e.to_string()))?;

        // TODO: Accept DMA buffers
        let stream_param_obj = pod::object!(
            spa::utils::SpaTypes::ObjectParamFormat,
            param::ParamType::EnumFormat,
            pod::property!(
                format::FormatProperties::MediaType,
                Id,
                format::MediaType::Video
            ),
            pod::property!(
                format::FormatProperties::MediaSubtype,
                Id,
                format::MediaSubtype::Raw
            ),
            Self::pod_supported_pixel_formats(),
            Self::pod_supported_resolutions(),
            Self::pod_supported_framerates(),
        );

        let param_obj_values = serialize_pod_object(stream_param_obj)?;
        let mut params = [pod::Pod::from_bytes(&param_obj_values).unwrap()];

        stream
            .connect(
                Direction::Input,
                Some(match capture_config.target {
                    crate::prelude::Capturable::Window(w) => w.impl_capturable_window.pw_node_id,
                    crate::prelude::Capturable::Display(d) => d.impl_capturable_display.pw_node_id,
                }),
                StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS,
                &mut params,
            )
            .map_err(|e| StreamCreateError::Other(e.to_string()))?;

        let loop_ = mainloop.loop_();

        // Iterate the stream and check for errors while negotiating pixel format
        while *(*format_negotiating).borrow() {
            loop_.iterate(Duration::from_millis(100));
        }

        if !should_run.load(Ordering::Acquire) {
            return Err(StreamCreateError::UnsupportedPixelFormat);
        }

        init_tx.send(Ok(())).unwrap();

        while should_run.load(Ordering::Acquire) {
            loop_.iterate(Duration::from_millis(100));
        }

        let _ = stream.disconnect();

        Ok(())
    }

    pub fn new(
        _token: WaylandCaptureAccessToken,
        capture_config: CaptureConfig,
        callback: Box<impl FnMut(Result<StreamEvent, StreamError>) + Send + 'static>,
    ) -> Result<Self, StreamCreateError> {
        let should_run = Arc::new(AtomicBool::new(true));
        let should_run_clone = Arc::clone(&should_run);
        let (init_tx, init_rx) = std::sync::mpsc::channel::<Result<(), StreamCreateError>>();
        let handle = std::thread::spawn(move || {
            if let Err(e) = Self::capture_main(capture_config, callback, should_run_clone, &init_tx)
            {
                init_tx.send(Err(e)).unwrap();
            }
        });

        init_rx.recv().unwrap()?;

        Ok(Self {
            handle: Some(handle),
            should_run,
        })
    }

    pub(crate) fn stop(&mut self) -> Result<(), StreamStopError> {
        if self.should_run.load(Ordering::Acquire) {
            self.should_run.store(false, Ordering::SeqCst);
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        Ok(())
    }
}

impl Drop for WaylandCaptureStream {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[derive(Clone, Debug)]
pub struct WaylandCaptureConfig {}

impl WaylandCaptureConfig {
    pub fn new() -> Self {
        Self {}
    }
}

#[allow(dead_code)]
pub struct WaylandPixelFormat {}

#[derive(Clone, Copy, Debug)]
pub struct WaylandCaptureAccessToken(());

impl WaylandCaptureAccessToken {
    pub(crate) fn allows_borderless(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::c_void;

    use pipewire::spa::sys::spa_meta;

    use super::*;

    #[test]
    fn buffer_metas_extraction_header() {
        let mut meta_header_data = spa_meta_header {
            flags: 1,
            offset: 2,
            pts: 3,
            dts_offset: 4,
            seq: 5,
        };
        let mut metas = [spa_meta {
            type_: SPA_META_Header,
            size: size_of_val(&meta_header_data) as u32,
            data: std::ptr::addr_of_mut!(meta_header_data) as *mut c_void,
        }];
        let mut buffer = spa_buffer {
            n_metas: metas.len() as u32,
            n_datas: 0,
            metas: std::ptr::addr_of_mut!(metas) as *mut spa_meta,
            datas: std::ptr::null_mut(),
        };
        let buffer_addr = std::ptr::addr_of_mut!(buffer);
        let extracted_metas = unsafe { PwMetas::from_raw(&buffer_addr) };
        assert_eq!(
            extracted_metas,
            PwMetas {
                header: Some(&meta_header_data),
                cursor: None,
                cursor_bitmap: None
            }
        );
    }

    #[test]
    fn buffer_metas_extraction_header_cursor() {
        let mut meta_header_data = spa_meta_header {
            flags: 1,
            offset: 2,
            pts: 3,
            dts_offset: 4,
            seq: 5,
        };
        let mut meta_cursor_data = spa_meta_cursor {
            id: 0,
            flags: 123,
            position: spa::sys::spa_point { x: 10, y: 12 },
            hotspot: spa::sys::spa_point { x: 20, y: 22 },
            bitmap_offset: 321,
        };
        let mut metas = [
            spa_meta {
                type_: SPA_META_Header,
                size: size_of_val(&meta_header_data) as u32,
                data: std::ptr::addr_of_mut!(meta_header_data) as *mut c_void,
            },
            spa_meta {
                type_: SPA_META_Cursor,
                size: size_of_val(&meta_cursor_data) as u32,
                data: std::ptr::addr_of_mut!(meta_cursor_data) as *mut c_void,
            },
        ];
        let mut buffer = spa_buffer {
            n_metas: metas.len() as u32,
            n_datas: 0,
            metas: std::ptr::addr_of_mut!(metas) as *mut spa_meta,
            datas: std::ptr::null_mut(),
        };
        let buffer_addr = std::ptr::addr_of_mut!(buffer);
        let extracted_metas = unsafe { PwMetas::from_raw(&buffer_addr) };
        assert_eq!(
            extracted_metas,
            PwMetas {
                header: Some(&meta_header_data),
                cursor: Some(&meta_cursor_data),
                cursor_bitmap: None
            }
        );
    }
}

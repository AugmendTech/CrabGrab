use std::{cell::{Ref, RefCell}, marker::PhantomData, sync::Arc, time::{Duration, Instant}};

use objc::runtime::Object;

use crate::{frame::{AudioCaptureFrame, VideoCaptureFrame}, prelude::{AudioBufferError, AudioChannelCount, AudioChannelData, AudioChannelDataSamples, AudioSampleRate}, util::{Rect, Size}};

use super::objc_wrap::{kAudioFormatFlagIsBigEndian, kAudioFormatFlagIsPacked, kAudioFormatFlagsCanonical, kAudioFormatNativeEndian, AVAudioFormat, AVAudioPCMBuffer, AudioBufferList, AudioStreamBasicDescription, CFDictionary, CGRect, CGRectMakeWithDictionaryRepresentation, CMBlockBuffer, CMSampleBuffer, IOSurface, NSDictionary, NSNumber, NSScreen, SCStreamFrameInfoScaleFactor, SCStreamFrameInfoScreenRect};

pub(crate) struct MacosSCStreamVideoFrame {
    pub(crate) sample_buffer: CMSampleBuffer,
    pub(crate) capture_time: Instant,
    pub(crate) dictionary: RefCell<Option<CFDictionary>>,
    pub(crate) frame_id: u64,
    #[cfg(feature = "metal")]
    pub(crate) metal_device: Option<metal::Device>,
    #[cfg(feature = "wgpu")]
    pub(crate) wgpu_device: Option<Arc<dyn AsRef<wgpu::Device> + Send + Sync + 'static>>,
}

pub(crate) struct MacosCGDisplayStreamVideoFrame {
    pub(crate) io_surface: IOSurface,
    pub(crate) duration: Duration,
    pub(crate) capture_time: Duration,
    pub(crate) capture_timestamp: Instant,
    pub(crate) frame_id: u64,
    pub(crate) source_rect: Rect,
    pub(crate) dest_size: Size,
    #[cfg(feature = "metal")]
    pub(crate) metal_device: metal::Device,
    #[cfg(feature = "wgpu")]
    pub(crate) wgpu_device: Option<Arc<dyn AsRef<wgpu::Device> + Send + Sync + 'static>>,
}

impl MacosSCStreamVideoFrame {
    fn get_info_dict(&self) -> Ref<'_, CFDictionary> {
        let needs_dict = { self.dictionary.borrow().is_none() };
        if needs_dict {
            let mut dict_opt_mut = self.dictionary.borrow_mut();
            *dict_opt_mut = Some(self.sample_buffer.get_sample_attachment_array()[0].clone());
        }
        Ref::map(self.dictionary.borrow(), |x| x.as_ref().unwrap())
    }
}

pub(crate) enum MacosVideoFrame {
    SCStream(MacosSCStreamVideoFrame),
    CGDisplayStream(MacosCGDisplayStreamVideoFrame),
}

impl VideoCaptureFrame for MacosVideoFrame {
    fn size(&self) -> Size {
        match self {
            MacosVideoFrame::SCStream(sc_frame) => {
                sc_frame.sample_buffer.get_image_buffer().map(|image_buffer| {
                    Size {
                        width: image_buffer.get_width() as f64,
                        height: image_buffer.get_height() as f64,
                    }
                }).unwrap_or(Size { width: 0.0, height: 0.0})
            }
            MacosVideoFrame::CGDisplayStream(cgd_frame) => cgd_frame.dest_size
        }
    }

    fn dpi(&self) -> f64 {
        match self {
            MacosVideoFrame::SCStream(sc_frame) => {
                let info_dict = sc_frame.get_info_dict();
                let scale_factor_ptr = unsafe { info_dict.get_value(SCStreamFrameInfoScaleFactor) };
                let scale_factor = unsafe { NSNumber::from_id_unretained(scale_factor_ptr as *mut Object).as_f64() };
                let screen_rect_ptr = unsafe { info_dict.get_value(SCStreamFrameInfoScreenRect) };
                let screen_rect_dict = unsafe { NSDictionary::from_id_unretained(screen_rect_ptr as *mut Object) };
                let frame_screen_rect = unsafe { CGRect::create_from_dictionary_representation(&screen_rect_dict) };
                let mut dpi = 72.0f64;
                for screen in NSScreen::screens() {
                    let screen_rect = screen.frame();
                    if screen_rect.contains(frame_screen_rect.origin) {
                        dpi = screen.dpi();
                        break;
                    }
                }
                dpi
            },
            MacosVideoFrame::CGDisplayStream(cgd_frame) => todo!()
        }
    }

    fn duration(&self) -> Duration {
        match self {
            MacosVideoFrame::SCStream(sc_frame) => std::time::Duration::from_secs_f64(sc_frame.sample_buffer.get_duration().seconds_f64()),
            MacosVideoFrame::CGDisplayStream(cgd_frame) => cgd_frame.duration
        }
    }

    fn origin_time(&self) -> Duration {
        match self {
            MacosVideoFrame::SCStream(sc_frame) => std::time::Duration::from_secs_f64(sc_frame.sample_buffer.get_presentation_timestamp().seconds_f64()),
            MacosVideoFrame::CGDisplayStream(cgd_frame) => cgd_frame.capture_time
        }
    }

    fn capture_time(&self) -> Instant {
        match self {
            MacosVideoFrame::SCStream(sc_frame) => sc_frame.capture_time,
            MacosVideoFrame::CGDisplayStream(cgd_frame) => cgd_frame.capture_timestamp
        }
    }

    fn frame_id(&self) -> u64 {
        match self {
            MacosVideoFrame::SCStream(sc_frame) => sc_frame.frame_id,
            MacosVideoFrame::CGDisplayStream(cgd_frame) => cgd_frame.frame_id
        }
    }
}

pub struct MacosAudioFrame {
    pub(crate) sample_buffer: CMSampleBuffer,
    pub(crate) audio_format_description: AudioStreamBasicDescription,
    pub(crate) pcm_audio_buffer: Option<AVAudioPCMBuffer>,
    pub(crate) block_buffer: Option<CMBlockBuffer>,
    pub(crate) buffer_list: Option<AudioBufferList>,
    pub(crate) capture_time: Instant,
    pub(crate) frame_id: u64,
}

impl AudioCaptureFrame for MacosAudioFrame {
    fn sample_rate(&self) -> crate::prelude::AudioSampleRate {
        if self.audio_format_description.sample_rate >= 15500.0 && self.audio_format_description.sample_rate <= 16500.0 {
            AudioSampleRate::Hz16000
        } else if self.audio_format_description.sample_rate >= 23500.0 && self.audio_format_description.sample_rate <= 24500.0 {
            AudioSampleRate::Hz24000
        } else if self.audio_format_description.sample_rate >= 47500.0 && self.audio_format_description.sample_rate <= 48500.0 {
            AudioSampleRate::Hz48000
        } else {
            AudioSampleRate::Hz8000
        }
    }

    fn channel_count(&self) -> crate::prelude::AudioChannelCount {
        if self.audio_format_description.channels_per_frame == 1 {
            AudioChannelCount::Mono
        } else {
            AudioChannelCount::Stereo
        }
    }

    fn audio_channel_buffer(&mut self, channel: usize) -> Result<AudioChannelData<'_>, AudioBufferError> {
        let pcm_audio_buffer_ref = if self.pcm_audio_buffer.is_some() {
            self.pcm_audio_buffer.as_ref().unwrap()
        } else {
            if self.audio_format_description.format_flags == kAudioFormatFlagsCanonical {
                let (audio_buffer_list, block_buffer) = match unsafe { self.sample_buffer.get_audio_buffer_list_with_block_buffer() } {
                    Ok(x) => x,
                    Err(()) => return Err(AudioBufferError::Other("CMSampleBuffer::get_audio_buffer_list_with_block_buffer() failed".into()))
                };
                self.buffer_list = Some(audio_buffer_list);
                self.block_buffer = Some(block_buffer);
                let audio_buffer_list = self.buffer_list.as_ref().unwrap();
                let av_audio_format = AVAudioFormat::new_with_standard_format_sample_rate_channels(self.audio_format_description.sample_rate, self.audio_format_description.channels_per_frame);
                if let Ok(pcm_audio_buffer) = AVAudioPCMBuffer::new_with_format_buffer_list_no_copy_deallocator(av_audio_format, audio_buffer_list as *const _) {
                    self.pcm_audio_buffer = Some(pcm_audio_buffer);
                    self.pcm_audio_buffer.as_ref().unwrap()
                } else {
                    return Err(AudioBufferError::Other("Failed to build PCM audio buffer".into()));
                }
            } else {
                return Err(AudioBufferError::UnsupportedFormat);
            }
        };
        if channel >= pcm_audio_buffer_ref.channel_count() {
            return Err(AudioBufferError::InvalidChannel);
        }
        let stride = pcm_audio_buffer_ref.stride();
        if let Some(f32_ptr) = pcm_audio_buffer_ref.f32_buffer(channel) {
            let data_samples = AudioChannelDataSamples {
                data: f32_ptr as *const u8,
                stride,
                length: pcm_audio_buffer_ref.frame_capacity(),
                phantom_lifetime: PhantomData
            };
            return Ok(AudioChannelData::F32(data_samples));
        }
        return Err(AudioBufferError::Other("Failed to get audio buffer".into()))
    }

    fn duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs_f64(self.sample_buffer.get_duration().seconds_f64())
    }

    fn origin_time(&self) -> std::time::Duration {
        std::time::Duration::from_secs_f64(self.sample_buffer.get_presentation_timestamp().seconds_f64())
    }

    fn frame_id(&self) -> u64 {
        self.frame_id
    }
}

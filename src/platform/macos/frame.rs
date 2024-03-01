use std::{cell::{Ref, RefCell}, marker::PhantomData, time::{Duration, Instant}};

use crate::{frame::{AudioCaptureFrame, VideoCaptureFrame}, prelude::{AudioBufferError, AudioChannelCount, AudioChannelData, AudioSampleRate}, util::{Rect, Size}};

use super::objc_wrap::{kAudioFormatFlagIsBigEndian, kAudioFormatFlagIsPacked, kAudioFormatFlagsCanonical, kAudioFormatNativeEndian, AVAudioFormat, AVAudioPCMBuffer, AudioBufferList, AudioStreamBasicDescription, CFDictionary, CMBlockBuffer, CMSampleBuffer, IOSurface};

pub(crate) struct MacosSCStreamVideoFrame {
    pub(crate) sample_buffer: CMSampleBuffer,
    pub(crate) capture_time: Instant,
    pub(crate) dictionary: RefCell<Option<CFDictionary>>,
    pub(crate) frame_id: u64,
}

pub(crate) struct MacosCGDisplayStreamVideoFrame {
    pub(crate) io_surface: IOSurface,
    pub(crate) duration: Duration,
    pub(crate) capture_time: Duration,
    pub(crate) capture_timestamp: Instant,
    pub(crate) frame_id: u64,
    pub(crate) source_rect: Rect,
    pub(crate) dest_size: Size,
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

pub enum MacosVideoFrame {
    SCStream(MacosSCStreamVideoFrame),
    CGDisplayStream(MacosCGDisplayStreamVideoFrame),
}

impl VideoCaptureFrame for MacosVideoFrame {
    fn logical_frame(&self) -> Rect {
        match self {
            MacosVideoFrame::SCStream(sc_frame) => todo!(),
            MacosVideoFrame::CGDisplayStream(cgd_frame) => cgd_frame.source_rect.scaled_2d((cgd_frame.source_rect.size.width / cgd_frame.dest_size.width, cgd_frame.source_rect.size.height / cgd_frame.dest_size.height))
        }
    }

    fn physical_frame(&self) -> Rect {
        match self {
            MacosVideoFrame::SCStream(sc_frame) => todo!(),
            MacosVideoFrame::CGDisplayStream(cgd_frame) => cgd_frame.source_rect
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
            return Ok(AudioChannelData::F32(f32_ptr, stride, PhantomData));
        }
        return Err(AudioBufferError::Other("Failed to get audio buffer".into()))
    }

    fn duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs_f64(self.sample_buffer.get_duration().seconds_f64())
    }

    fn origin_time(&self) -> std::time::Duration {
        std::time::Duration::from_secs_f64(self.sample_buffer.get_presentation_timestamp().seconds_f64())
    }

    fn capture_time(&self) -> std::time::Instant {
        self.capture_time
    }

    fn frame_id(&self) -> u64 {
        self.frame_id
    }
}

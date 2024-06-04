use std::{marker::PhantomData, sync::Arc, time::Duration};

use windows::{Graphics::{Capture::Direct3D11CaptureFrame, DirectX::DirectXPixelFormat, SizeInt32}, Win32::Graphics::Direct3D11::ID3D11Device};

use crate::{prelude::{AudioBufferError, AudioCaptureFrame, AudioChannelCount, AudioChannelDataSamples, AudioSampleRate, Point, Rect, VideoCaptureFrame}, util::Size};

pub struct WindowsVideoFrame {
    pub(crate) device       : ID3D11Device,
    pub(crate) frame        : Direct3D11CaptureFrame,
    pub(crate) frame_size   : (usize, usize),
    pub(crate) pixel_format : DirectXPixelFormat,
    pub(crate) frame_id     : u64,
    pub(crate) dpi          : u32,
    pub(crate) t_capture    : std::time::Instant,
    pub(crate) t_origin     : std::time::Duration,
    pub(crate) duration     : std::time::Duration,
    #[cfg(feature = "wgpu")]
    pub(crate) wgpu_device  : Option<Arc<dyn AsRef<wgpu::Device> + Send + Sync + 'static>>,
}

impl VideoCaptureFrame for WindowsVideoFrame {
    fn size(&self) -> Size {
        let size = self.frame.ContentSize().unwrap_or(SizeInt32::default());
        Size {
            width: size.Width as f64,
            height: size.Height as f64,
        }
    }

    fn dpi(&self) -> f64 {
        self.dpi as f64
    }

    fn duration(&self) -> std::time::Duration {
        self.duration
    }

    fn origin_time(&self) -> std::time::Duration {
        self.t_origin
    }

    fn capture_time(&self) -> std::time::Instant {
        self.t_capture
    }

    fn frame_id(&self) -> u64 {
        self.frame_id
    }

    fn content_rect(&self) -> Rect {
        Rect {
            origin: Point::ZERO,
            size: self.size()
        }
    }
}

impl Drop for WindowsVideoFrame {
    fn drop(&mut self) {
        _ = self.frame.Close();
    }
}

pub struct WindowsAudioFrame {
    pub(crate) data: Box<[i16]>,
    pub(crate) channel_count: AudioChannelCount,
    pub(crate) sample_rate: AudioSampleRate,
    pub(crate) duration: Duration,
    pub(crate) origin_time: Duration,
    pub(crate) frame_id: u64,
}

impl AudioCaptureFrame for WindowsAudioFrame {
    fn sample_rate(&self) -> crate::prelude::AudioSampleRate {
        self.sample_rate
    }

    fn channel_count(&self) -> crate::prelude::AudioChannelCount {
        self.channel_count
    }

    fn audio_channel_buffer(&mut self, channel: usize) -> Result<crate::prelude::AudioChannelData<'_>, crate::prelude::AudioBufferError> {
        let element_stride = match self.channel_count {
            AudioChannelCount::Mono => {
                if channel != 0 {
                    return Err(AudioBufferError::InvalidChannel)
                }
                0
            },
            AudioChannelCount::Stereo => {
                if channel > 1 {
                    return Err(AudioBufferError::InvalidChannel)
                }
                channel
            },
        };
        let data = &self.data[element_stride] as *const i16 as *const u8;
        Ok(crate::prelude::AudioChannelData::I16(AudioChannelDataSamples {
            data,
            stride: element_stride / 2,
            length: self.data.len() / element_stride,
            phantom_lifetime: PhantomData
        }))
    }

    fn duration(&self) -> std::time::Duration {
        self.duration
    }

    fn origin_time(&self) -> std::time::Duration {
        self.origin_time
    }

    fn frame_id(&self) -> u64 {
        self.frame_id
    }
}


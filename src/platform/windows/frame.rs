use windows::{Graphics::{Capture::Direct3D11CaptureFrame, DirectX::DirectXPixelFormat, SizeInt32}, Win32::Graphics::Direct3D11::ID3D11Device};

use crate::{prelude::{AudioCaptureFrame, VideoCaptureFrame}, util::{Point, Rect, Size}};

pub struct WindowsVideoFrame {
    pub(crate) device: ID3D11Device,
    pub(crate) frame: Direct3D11CaptureFrame,
    pub(crate) frame_size: (usize, usize),
    pub(crate) pixel_format: DirectXPixelFormat,
    pub(crate) frame_id: u64,
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
        todo!()
    }

    fn duration(&self) -> std::time::Duration {
        todo!()
    }

    fn origin_time(&self) -> std::time::Duration {
        todo!()
    }

    fn capture_time(&self) -> std::time::Instant {
        todo!()
    }

    fn frame_id(&self) -> u64 {
        self.frame_id
    }
}

pub struct WindowsAudioFrame {

}

impl AudioCaptureFrame for WindowsAudioFrame {
    fn sample_rate(&self) -> crate::prelude::AudioSampleRate {
        todo!()
    }

    fn channel_count(&self) -> crate::prelude::AudioChannelCount {
        todo!()
    }

    fn audio_channel_buffer(&mut self, channel: usize) -> Result<crate::prelude::AudioChannelData<'_>, crate::prelude::AudioBufferError> {
        todo!()
    }

    fn duration(&self) -> std::time::Duration {
        todo!()
    }

    fn origin_time(&self) -> std::time::Duration {
        todo!()
    }

    fn capture_time(&self) -> std::time::Instant {
        todo!()
    }

    fn frame_id(&self) -> u64 {
        todo!()
    }
}


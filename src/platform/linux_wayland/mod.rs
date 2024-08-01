mod capture_content;
mod capture_stream;
mod frame;

pub(crate) use capture_stream::WaylandCaptureAccessToken as ImplCaptureAccessToken;
pub(crate) use capture_stream::WaylandCaptureConfig as ImplCaptureConfig;
pub(crate) use capture_stream::WaylandCaptureStream as ImplCaptureStream;
#[allow(unused_imports)]
pub(crate) use capture_stream::WaylandPixelFormat as ImplPixelFormat;

pub(crate) use frame::WaylandVideoFrame as ImplVideoFrame;

pub(crate) use capture_content::WaylandCapturableApplication as ImplCapturableApplication;
pub(crate) use capture_content::WaylandCapturableContent as ImplCapturableContent;
pub(crate) use capture_content::WaylandCapturableContentFilter as ImplCapturableContentFilter;
pub(crate) use capture_content::WaylandCapturableDisplay as ImplCapturableDisplay;
pub(crate) use capture_content::WaylandCapturableWindow as ImplCapturableWindow;

#[derive(Clone, Debug)]
pub(crate) struct ImplAudioCaptureConfig {}

impl ImplAudioCaptureConfig {
    pub fn new() -> Self {
        Self {}
    }
}

use crate::prelude::AudioCaptureFrame;

pub(crate) struct ImplAudioFrame;

impl AudioCaptureFrame for ImplAudioFrame {
    fn sample_rate(&self) -> crate::prelude::AudioSampleRate {
        todo!()
    }

    fn channel_count(&self) -> crate::prelude::AudioChannelCount {
        todo!()
    }

    fn audio_channel_buffer(
        &mut self,
        _channel: usize,
    ) -> Result<crate::prelude::AudioChannelData<'_>, crate::prelude::AudioBufferError> {
        todo!()
    }

    fn duration(&self) -> std::time::Duration {
        todo!()
    }

    fn origin_time(&self) -> std::time::Duration {
        todo!()
    }

    fn frame_id(&self) -> u64 {
        todo!()
    }
}

use metal::Texture;
use crate::prelude::VideoFrame;

pub trait MetalVideoFrame {
    fn get_texture(&self) -> Texture;
}

#[cfg(feature="metal")]
impl MetalVideoFrame for VideoFrame {
    fn get_texture(&self) -> metal::Texture {
        todo!()
    }
}

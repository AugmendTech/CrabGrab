use metal::Texture;

pub trait MetalVideoFrame {
    fn get_texture(&self) -> Texture;
}

#[cfg(feature="metal")]
impl MetalVideoFrame for VideoCaptureFrame {
    fn get_texture(&self) -> metal::Texture {
        
    }
}

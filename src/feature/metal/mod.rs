use metal::Texture;

pub trait MetalVideoFrame {
    fn get_texture(&self) -> Texture;
}

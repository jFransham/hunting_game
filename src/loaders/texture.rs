use gfx::tex::AaMode;
use image::{
    self,
    RgbaImage,
    ImageFormat,
};
use amethyst::context::asset_manager::{
    Assets,
    AssetLoader,
    AssetLoaderRaw,
    Texture,
    RawTextureData,
    Kind,
};

// TODO: Make this generic
pub struct PngTextureLoader(u32, u32, Vec<u8>);

impl AssetLoaderRaw for PngTextureLoader {
    fn from_raw(_assets: &Assets, data: &[u8]) -> Option<Self> {
        let o_img = image::load_from_memory_with_format(data, ImageFormat::PNG)
            .map(|dyn| dyn.to_rgba());

        let image: RgbaImage =
            if let Ok(img) = o_img {
                img
            } else {
                return None;
            };

        let (image_w, image_h) = image.dimensions();

        let pixels: Vec<u8> = image.into_vec();

        PngTextureLoader(image_w, image_h, pixels).into()
    }
}

impl AssetLoader<Texture> for PngTextureLoader {
    fn from_data(assets: &mut Assets, data: Self) -> Option<Texture> {
        let tex = RawTextureData {
            kind: Kind::D2(data.0 as _, data.1 as _, AaMode::Single),
            raw: &data.2,
        };

        AssetLoader::from_data(assets, tex).expect("Texture load failed").into()
    }
}

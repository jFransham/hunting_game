use tobj::{parse_obj, Model};
use amethyst::context::asset_manager::{
    Assets,
    AssetLoader,
    AssetLoaderRaw,
    Kind,
};

// TODO: Make this generic
pub struct ObjLoader((Vec<Model>, Vec<String>));

impl AssetLoaderRaw for ObjLoader {
    fn from_raw(_assets: &Assets, mut data: &[u8]) -> Option<Self> {
        parse_obj(&mut data).map(ObjLoader).ok()
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

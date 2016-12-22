use std::collections::HashMap;
use tobj::{parse_obj, parse_mtl, Model, Material};
use amethyst::context::asset_manager::{
    Asset,
    Assets,
    AssetManager,
    AssetLoader,
    AssetLoaderRaw,
    Mesh,
    FactoryImpl,
    MeshImpl,
};
use amethyst::processors::rendering::Renderable;
use amethyst::renderer::VertexPosNormal;

pub type MtlLib = HashMap<String, Material>;

pub struct MtlLoader(MtlLib);

impl AssetLoaderRaw for MtlLoader {
    fn from_raw(_assets: &Assets, mut data: &[u8]) -> Option<Self> {
        parse_mtl(&mut data).map(MtlLoader).ok()
    }
}

impl AssetLoader<MtlLib> for MtlLoader {
    fn from_data(
        _assets: &mut Assets,
        data: Self,
    ) -> Option<HashMap<String, Material>> {
        Some(data.0)
    }
}

// `Model` is _really_ low-level (it uses flattened vectors). Don't just pipe
// it into OpenGL raw unless you want to have a Really Bad Time.
pub struct ObjLoader((Vec<Model>, Vec<String>));

impl AssetLoaderRaw for ObjLoader {
    fn from_raw(_assets: &Assets, mut data: &[u8]) -> Option<Self> {
        parse_obj(&mut data).map(ObjLoader).ok()
    }
}

impl AssetLoader<Vec<Renderable>> for ObjLoader {
    fn from_data(
        _assets: &mut Assets,
        _data: Self,
    ) -> Option<Vec<Renderable>> {
        // TODO: Use mtl and textures here, and only preload them in
        //       load_from_data
        // TODO: Should load_from_data be replaced with a function that returns
        //       a list of assets to preload?
        None
    }

    fn load_from_data(
        assets: &mut AssetManager,
        mut data: Self,
    ) -> Option<Vec<Renderable>> {
        let mut lib_ids = (data.0).1.drain(..).filter_map(
            |name| {
                let mut split = name.rsplitn(2, '.');

                let o_ext  = split.next();
                let o_name = split.next();

                let (name, ext) = match (o_name, o_ext) {
                    (None, Some(st))     => (st, "mtl"),
                    (Some(nm), Some(ex)) => (nm, ex),
                    _                    => return None,
                };

                assets.load_asset::<MtlLib>(name, ext)
            }
        ).collect::<Vec<_>>();

        let materials: MtlLib = {
            let assets_store = assets.read_assets();

            lib_ids.drain(..).fold(
                HashMap::new(),
                |mut last, cur| {
                    if let Some(asset) =
                        assets_store.get(cur)
                    {
                        let asset: &Asset<MtlLib> = asset;
                        last.extend(
                            asset.0.iter().map(|(a, b)| (a.clone(), b.clone()))
                        );
                    }

                    last
                }
            )
        };

        (data.0).0.drain(..).map(|model| {
            let mesh = {
                let factory_impl = assets.get_loader_mut::<FactoryImpl>()
                    .expect("Unable to retrieve factory");

                let verts = verts_from_model(&model).collect::<Vec<_>>();

                new_mesh(factory_impl, &verts, &model.mesh.indices)
            };

            assets.add_asset(&model.name, mesh);

            let mat = model.mesh.material
                .and_then(|m| materials.get(&m))
                .map(|m| &m.diffuse_texture)
                .and_then(|tex| tex.rsplitn(2, '.').skip(1).next())
                .map(|m| m as &str)
                .unwrap_or("default");

            Renderable::new(
                &model.name,
                mat,
                mat,
            )
        }).collect::<Vec<_>>().into()
    }
}

fn verts_from_model<'a>(model: &'a Model) ->
    impl Iterator<Item=VertexPosNormal> + 'a
{
    model.mesh.positions.chunks(3)
        .zip(model.mesh.normals.chunks(3))
        .zip(model.mesh.texcoords.chunks(2))
        .map(
            |((pos, norms), coords)|
            VertexPosNormal {
                pos: [pos[0], pos[1], pos[2]],
                normal: [norms[0], norms[1], norms[2]],
                tex_coord: [coords[0], coords[1]],
            }
        )
}

impl AssetLoader<Mesh> for ObjLoader {
    fn from_data(assets: &mut Assets, mut data: Self) -> Option<Mesh> {
        let mut out_verts   = vec![];
        let mut out_indices = vec![];

        // TODO: Should I collapse textures together? The alternative, I
        //       guess, is to make this return a Renderable.

        for model in (data.0).0.drain(..) {
            let vertices = verts_from_model(&model);

            let offset = out_verts.len() as u32;
            out_indices.extend(
                model.mesh.indices.iter().map(|i| i + offset)
            );
            out_verts.extend(vertices);

        }

        let factory_impl = assets.get_loader_mut::<FactoryImpl>()
            .expect("Unable to retrieve factory");

        Some(new_mesh(factory_impl, &out_verts, &out_indices))
    }
}

fn new_mesh<F: ::std::ops::DerefMut<Target=FactoryImpl>>(
    mut factory_impl: F,
    buf: &[VertexPosNormal],
    slc: &[u32],
) -> Mesh {
    use gfx::traits::FactoryExt;

    match *factory_impl {
        FactoryImpl::OpenGL { ref mut factory } => {
            let (buffer, slice) =
                factory.create_vertex_buffer_with_slice(
                    buf,
                    slc,
                );

            Mesh {
                mesh_impl: MeshImpl::OpenGL {
                    buffer: buffer,
                    slice: slice,
                }
            }
        }
        #[cfg(windows)]
        FactoryImpl::Direct3D {} => unimplemented!(),
        FactoryImpl::Null => Mesh { mesh_impl: MeshImpl::Null },
    }
}

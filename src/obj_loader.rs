use tobj::{parse_obj, Model};
use amethyst::context::asset_manager::{
    Assets,
    AssetLoader,
    AssetLoaderRaw,
    Mesh,
    FactoryImpl,
    MeshImpl,
};
use amethyst::renderer::VertexPosNormal;

// `Model` is _really_ low-level (it uses flattened vectors). Don't just pipe
// it into OpenGL raw unless you want to have a Really Bad Time.
pub struct ObjLoader((Vec<Model>, Vec<String>));

impl AssetLoaderRaw for ObjLoader {
    fn from_raw(_assets: &Assets, mut data: &[u8]) -> Option<Self> {
        parse_obj(&mut data).map(ObjLoader).ok()
    }
}

impl AssetLoader<Mesh> for ObjLoader {
    fn from_data(assets: &mut Assets, mut data: Self) -> Option<Mesh> {
        use gfx::traits::FactoryExt;

        let factory_impl = assets.get_loader_mut::<FactoryImpl>()
            .expect("Unable to retrieve factory");
        match *factory_impl {
            FactoryImpl::OpenGL { ref mut factory } => {
                (data.0).0.drain(..).next().map(|model| {
                    // TODO: Should I collapse models together if they're from
                    // the same .obj? Should I collapse textures together?
                    // Should I make this return a Renderable? Vec<Renderable>,
                    // maybe?

                    let vertices = model.mesh.positions.chunks(3)
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
                        .collect::<Vec<_>>();

                    let indices: &[u32] = &model.mesh.indices;

                    let (buffer, slice) =
                        factory.create_vertex_buffer_with_slice(
                            &vertices, indices
                        );

                    Mesh {
                        mesh_impl: MeshImpl::OpenGL {
                            buffer: buffer,
                            slice: slice,
                        }
                    }
                })
            }
            #[cfg(windows)]
            FactoryImpl::Direct3D {} => unimplemented!(),
            FactoryImpl::Null => Some(Mesh { mesh_impl: MeshImpl::Null }),
        }
    }
}

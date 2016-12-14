extern crate gfx;
extern crate image;
extern crate amethyst;
extern crate nalgebra;
extern crate ncollide;
extern crate nphysics2d;
extern crate tobj;

use std::sync::{Arc, Mutex};

use amethyst::context::{
    Context,
    ContextConfig,
};
use amethyst::context::asset_manager::{
    Mesh,
    Texture,
    DirectoryStore,
};
use amethyst::engine::{
    Application,
    State,
    Trans,
};
use amethyst::processors::rendering::*;
use amethyst::processors::transform::*;
use amethyst::ecs::{
    World,
    Join,
    RunArg,
    Processor,
};
use ncollide::shape::Cuboid;
use nphysics2d::math::Vector;
use nphysics2d::object::RigidBody;

mod texture_loader;
mod obj_loader;
mod systems;

use systems::physics::*;
use texture_loader::PngTextureLoader;

struct ImpulseProcessor;

impl Processor<Arc<Mutex<Context>>> for ImpulseProcessor {
    fn run(&mut self, arg: RunArg, context: Arc<Mutex<Context>>) {
        use amethyst::ecs::Join;
        use amethyst::context::event::{
            ElementState,
            EngineEvent,
            Event,
            VirtualKeyCode,
        };

        let mut l_impulses = arg.fetch(|w| w.write::<ImpulseComponent>());
        let context = context.lock().unwrap();

        let engine_events = context.broadcaster.read::<EngineEvent>();
        let mut impulse = None;
        for engine_event in engine_events.iter() {
            match engine_event.payload {
                Event::KeyboardInput(
                    ElementState::Pressed,
                    _,
                    Some(VirtualKeyCode::Up),
                ) => {
                    impulse = Some(Vector::new(0., -0.5));
                    break;
                },
                Event::KeyboardInput(
                    ElementState::Pressed,
                    _,
                    Some(VirtualKeyCode::Down),
                ) => {
                    impulse = Some(Vector::new(0., 0.5));
                    break;
                },
                Event::KeyboardInput(
                    ElementState::Pressed,
                    _,
                    Some(VirtualKeyCode::Left),
                ) => {
                    impulse = Some(Vector::new(-0.5, 0.));
                    break;
                },
                Event::KeyboardInput(
                    ElementState::Pressed,
                    _,
                    Some(VirtualKeyCode::Right),
                ) => {
                    impulse = Some(Vector::new(0.5, 0.));
                    break;
                },
                _ => (),
            }
        }

        if let Some(im) = impulse {
            for i in (&mut l_impulses).iter() {
                i.linear = Some(im);
            }
        }
    }
}

struct HelloWorld;

impl State for HelloWorld {
    fn on_start(&mut self, ctx: &mut Context, world: &mut World) {
        let (w, h) = ctx.renderer.get_dimensions().unwrap();
        let aspect = w as f32 / h as f32;
        let eye    = [0., 0., 0.1];
        let target = [0., 0., 0.];
        let up     = [0., -1., 0.];

        // Get an Orthographic projection
        let projection = Projection::Orthographic {
            left:    1.0 * aspect,
            right:  -1.0 * aspect,
            bottom: -1.0,
            top:     1.0,
            near:    0.0,
            far:     1.0,
        };

        world.add_resource(projection);

        // Create a camera entity
        let mut camera = Camera::new(projection, eye, target, up);
        camera.activate();
        world.create_now()
            .with(camera)
            .build();

        // Generate a square mesh
        ctx.asset_manager.load_asset::<Texture>("baby", "png");
        ctx.asset_manager.gen_rectangle("square", 1.0, 1.0);

        let square = Renderable::new("square", "baby", "baby");
        let phys_box = Cuboid::new(Vector::new(0.463, 0.463));
        let l_trans = LocalTransform::default();
        let trans = Transform::default();

        world.create_now()
            .with(square.clone())
            .with(l_trans)
            .with(trans)
            .with(ImpulseComponent::default())
            .with(
                PhysicsComponent::new(
                    RigidBody::new_dynamic(
                        phys_box.clone(),
                        0.5,
                        0.5,
                        0.9,
                    )
                )
            )
            .build();

        let mut l_trans = LocalTransform::default();
        l_trans.translation[1] = 1.5;
        let trans = Transform::default();

        world.create_now()
            .with(square.clone())
            .with(l_trans)
            .with(trans)
            .with(
                PhysicsComponent::new(
                    RigidBody::new_static(
                        phys_box.clone(),
                        0.5,
                        0.5,
                    )
                )
            )
            .build();

        let mut l_trans = LocalTransform::default();
        l_trans.translation[0] = 0.8;
        l_trans.translation[1] = 1.2;
        let trans = Transform::default();

        world.create_now()
            .with(square.clone())
            .with(l_trans)
            .with(trans)
            .with(
                PhysicsComponent::new(
                    RigidBody::new_static(
                        phys_box.clone(),
                        0.5,
                        0.5,
                    )
                )
            )
            .build();
    }

    fn update(
        &mut self,
        ctx: &mut Context,
        _world: &mut World,
    ) -> Trans {
        // Exit if user hits Escape or closes the window
        use amethyst::context::event::{EngineEvent, Event, VirtualKeyCode};
        let engine_events = ctx.broadcaster.read::<EngineEvent>();

        for engine_event in engine_events.iter() {
            match engine_event.payload {
                Event::Closed |
                Event::KeyboardInput(_, _, Some(VirtualKeyCode::Escape)) =>
                    return Trans::Quit,
                _ => (),
            }
        }

        Trans::None
    }
}

fn main() {
    let mut config = ContextConfig::default();
    config.display_config.backend = "OpenGL".into();
    config.display_config.title   = "Hunting game".into();
    let mut context = Context::new(config);

    context.asset_manager.register_asset::<Mesh>();
    context.asset_manager.register_asset::<Texture>();

    context.asset_manager.register_loader::<Texture, PngTextureLoader>("png");

    let path = format!("{}/resources/assets/", env!("CARGO_MANIFEST_DIR"));

    context.asset_manager.register_store(
        DirectoryStore::new(&path)
    );

    let render_prcs = RenderingProcessor::new(
        Default::default(),
        &mut context,
    );

    let phys_process = PhysicsProcessor::new();

    let mut game = Application::build(HelloWorld, context)
        .with(render_prcs, "Rendering processor", 0)
        .register::<Renderable>()
        .register::<Light>()
        .register::<Camera>()
        .with(
            TransformProcessor::new(),
            "Transform processor",
            2,
        )
        .register::<LocalTransform>()
        .register::<Transform>()
        .register::<Child>()
        .register::<Init>()
        .with(phys_process, "Physics processor", 1)
        .register::<PhysicsComponent>()
        .register::<ImpulseComponent>()
        .with(ImpulseProcessor, "Impulse processor", 2)
        .done();

    game.run();
}

use std::sync::{Arc, Mutex};

use nalgebra::{Matrix2, Quaternion};
use nphysics2d::math::{Orientation, Vector, Matrix};
use nphysics2d::world::{
    World,
    RigidBodyId,
};
use nphysics2d::object::{
    WorldObject,
    RigidBody,
    ActivationState,
};

use amethyst::context::Context;
use amethyst::ecs::{
    RunArg,
    Processor,
    Component,
    VecStorage
};
use amethyst::processors::transform::LocalTransform;

type Precision = f32;

#[derive(Default, Clone)]
pub struct ImpulseComponent {
    pub angular: Option<Orientation<Precision>>,
    pub linear:  Option<Vector<Precision>>,
}

impl Component for ImpulseComponent {
    type Storage = VecStorage<ImpulseComponent>;
}

pub struct PhysicsComponent {
    handle: Result<RigidBodyId, RigidBody<Precision>>,
}

impl PhysicsComponent {
    pub fn new(rgd: RigidBody<Precision>) -> Self {
        PhysicsComponent {
            handle: Err(rgd),
        }
    }
}

// TODO: Is this ever safe?
unsafe impl Send for PhysicsComponent {}
unsafe impl Sync for PhysicsComponent {}

impl Component for PhysicsComponent {
    type Storage = VecStorage<PhysicsComponent>;
}

pub struct PhysicsProcessor(World<Precision>);

// Shamelessly nicked from http://www.euclideanspace.com/maths/geometry/rotations/conversions/matrixToQuaternion/index.htm
// and converted to work on 2D rotation matrices. The output quaternion is 3D
// but we can ignore i and j
fn matrix_to_quaternion(m: &Matrix2<Precision>) -> Quaternion<Precision> {
    let trace = 1. + m.m11 + m.m22;

    if trace > 0. {
        let s = (trace + 1.).sqrt() / 2.;

        Quaternion {
            w: s,
            i: 0.,
            j: 0.,
            k: (m.m21 - m.m12) / (4. * s),
        }
    } else if ((m.m11 > m.m22) && (m.m11 > 1.)) || (m.m22 > 1.) {
        Quaternion {
            w: 0.,
            i: 0.,
            j: 0.,
            k: 0.,
        }
    } else {
        let s = (2. - m.m22 - m.m11).sqrt() / 2.;

        Quaternion {
            w: (m.m21 - m.m12) / (4. * s),
            i: 0.,
            j: 0.,
            k: s,
        }
    }
}

impl PhysicsProcessor {
    pub fn new() -> Self {
        let mut world = World::new();
        world.set_gravity(Vector::new(0., 9.81));

        PhysicsProcessor(world)
    }
}

// TODO: This is only safe if we never keep handles longer than the lifetime of
//       the run function, and even then is still unsafe if we run that function
//       twice concurrently. At best, this is a stop-gap before converting
//       nphysics to be truly thread-safe (not a small task, transforming
//       Rc<RefCell<T>> to Arc<RwLock<T>> is likely to bring a non-negligible
//       performance penalty).
unsafe impl Send for PhysicsProcessor {}

impl Processor<Arc<Mutex<Context>>> for PhysicsProcessor {
    fn run(&mut self, arg: RunArg, context: Arc<Mutex<Context>>) {
        use amethyst::ecs::Join;

        let (mut l_physc, mut l_trans, mut l_impulses) = arg.fetch(
            |w| (
                w.write::<PhysicsComponent>(),
                w.write::<LocalTransform>(),
                w.write::<ImpulseComponent>(),
            )
        );
        let ref mut wrld = self.0;

        for (phys, trans) in (&mut l_physc, &mut l_trans).iter() {
            let uid = 
                match phys.handle {
                    Ok(ref uid) => {
                        if let Some(handle) = wrld.get_rigid_body_by_uid(uid) {
                            let &Matrix {
                                translation: Vector { x, y },
                                rotation: rot,
                            } = handle.borrow().position();

                            let quat = matrix_to_quaternion(rot.submatrix());

                            trans.translation = [x, y, 0.];
                            trans.rotation = [quat.w, quat.i, quat.j, quat.k];
                        }

                        continue;
                    }
                    Err(ref rgd) => {
                        let mut rigid_body = rgd.clone();

                        rigid_body.append_translation(
                            &Vector::new(
                                trans.translation[0],
                                trans.translation[1],
                            )
                        );

                        WorldObject::rigid_body_uid(
                            &wrld.add_rigid_body(rigid_body)
                        )
                    },
                };

            phys.handle = Ok(uid);
        }

        for (phys, impls) in (&mut l_physc, &mut l_impulses).iter() {
            if let Some(handle) = phys.handle.as_ref().ok()
                .and_then(|uid| wrld.get_rigid_body_by_uid(&uid))
            {
                let mut handle = handle.borrow_mut();

                if let Some(ng) = impls.angular.take() {
                    if &ActivationState::Inactive == handle.activation_state() {
                        handle.activate(1.);
                    }
                    handle.apply_angular_momentum(ng);
                }

                if let Some(lin) = impls.linear.take() {
                    // TODO: Move this into apply_*
                    if &ActivationState::Inactive == handle.activation_state() {
                        handle.activate(1.);
                    }
                    handle.apply_central_impulse(lin);
                }
            }
        }

        let dt = context.lock().unwrap().delta_time;
        let dt_secs =
            dt.as_secs() as Precision +
            (dt.subsec_nanos() as Precision * 1.0e-9);

        wrld.step(dt_secs);
    }
}

use cgmath::prelude::*;
use itertools::Itertools;
use stable_vec::StableVec;

#[derive(Clone)]
struct PlayerController {
    pending_events: Vec<()>,
    controlled_object: usize,
}

#[derive(Clone)]
enum Controller {
    PlayerController(PlayerController),
}

#[derive(Clone)]
pub enum Object {
    Static {
        pos: cgmath::Point2<f64>,
        size: cgmath::Vector2<f64>,
    },
    Movable {
        pos: cgmath::Point2<f64>,
        size: cgmath::Vector2<f64>,
        velocity: cgmath::Vector2<f64>,
    },
}

impl Object {
    pub fn get_pos(&self) -> &cgmath::Point2<f64> {
        match self {
            Object::Static { pos, .. } => pos,
            Object::Movable { pos, .. } => pos,
        }
    }
    pub fn get_size(&self) -> &cgmath::Vector2<f64> {
        match self {
            Object::Static { size, .. } => size,
            Object::Movable { size, .. } => size,
        }
    }

    fn apply_event(&mut self, event: ()) {
        if let Object::Movable { velocity, .. } = self {
            *velocity += cgmath::vec2(0.0, 5.0);
        }
    }
}

#[derive(Clone)]
pub struct GameState {
    controllers: Vec<Controller>,
    pub objects: StableVec<Object>,
    pub view_object: usize,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            controllers: vec![Controller::PlayerController(PlayerController {
                pending_events: vec![],
                controlled_object: 0,
            })],
            objects: [
                Object::Movable {
                    pos: cgmath::point2(-0.55, -0.5),
                    size: cgmath::vec2(1.0, 1.0),
                    velocity: cgmath::vec2(0.0, 0.0),
                },
                Object::Static {
                    pos: cgmath::point2(-25.0, -25.0),
                    size: cgmath::vec2(50.0, 7.5),
                },
            ]
            .into(),
            view_object: 0,
        }
    }
    pub fn update(&mut self, dt: f64) {
        for controller in &mut self.controllers {
            if let Controller::PlayerController(controller) = controller {
                let controlled = controller.controlled_object;
                let object = self.objects.get_mut(controlled);
                if let Some(object) = object {
                    for event in controller.pending_events.drain(..) {
                        object.apply_event(event);
                    }
                }
            }
        }
        for (_, object) in &mut self.objects {
            if let Object::Movable { velocity, pos, .. } = object {
                *velocity -= cgmath::vec2(0.0, 9.8) * dt;
                *pos += *velocity * dt;
            }
        }

        self.collision_detection();
    }
    pub fn submit_player_event(&mut self, event: ()) {
        for controller in &mut self.controllers {
            if let Controller::PlayerController(controller) = controller {
                controller.pending_events.push(event);
            }
        }
    }
    fn collision_detection(&mut self) {
        // lifetime issues without the collect, clippy is just going to have to deal with it
        #[allow(clippy::needless_collect)]
        let indexes: Vec<_> = self.objects.indices().collect();
        for (object1, object2) in indexes.into_iter().tuple_combinations() {
            self.handle_collision(object1, object2);
        }
    }

    fn handle_collision(&mut self, object1: usize, object2: usize) {
        if object1 == object2 {
            return; //impossible, but just in case, since it would otherwise cause chaos
        }
        //it is now gauranteed that this is safe
        if let (Some(object1), Some(object2)) = unsafe {
            (
                (*(self as *mut Self)).objects.get_mut(object1),
                (*(self as *mut Self)).objects.get_mut(object2),
            )
        } {
            match (object1, object2) {
                (Object::Static { .. }, Object::Static { .. }) => {}
                (
                    Object::Static {
                        pos: pos1,
                        size: size1,
                    },
                    Object::Movable {
                        pos: pos2,
                        size: size2,
                        velocity: velocity2,
                    },
                ) => {
                    if let Some(offset) = check_collision(pos1, size1, pos2, size2) {
                        if offset.x != 0.0 {
                            velocity2.x = 0.0;
                        }
                        if offset.y != 0.0 {
                            velocity2.y = 0.0;
                        }
                        *pos2 -= offset;
                    }
                }
                (
                    Object::Movable {
                        pos: pos1,
                        size: size1,
                        velocity: velocity1,
                    },
                    Object::Static {
                        pos: pos2,
                        size: size2,
                    },
                ) => {
                    if let Some(offset) = check_collision(pos1, size1, pos2, size2) {
                        if offset.x != 0.0 {
                            velocity1.x = 0.0;
                        }
                        if offset.y != 0.0 {
                            velocity1.y = 0.0;
                        }
                        *pos1 += offset;
                    }
                }
                (
                    Object::Movable {
                        pos: pos1,
                        size: size1,
                        velocity: velocity1,
                    },
                    Object::Movable {
                        pos: pos2,
                        size: size2,
                        velocity: velocity2,
                    },
                ) => {
                    if let Some(offset) = check_collision(pos1, size1, pos2, size2) {
                        if offset.x != 0.0 {
                            velocity1.x = 0.0;
                            velocity2.x = 0.0;
                        }
                        if offset.y != 0.0 {
                            velocity1.y = 0.0;
                            velocity2.y = 0.0;
                        }
                        let offset = offset / 2.0;
                        *pos1 += offset;
                        *pos2 -= offset;
                    }
                }
            }
        }
    }
}

fn check_collision(
    pos1: &cgmath::Point2<f64>,
    size1: &cgmath::Vector2<f64>,
    pos2: &cgmath::Point2<f64>,
    size2: &cgmath::Vector2<f64>,
) -> Option<cgmath::Vector2<f64>> {
    if pos1.x < pos2.x + size2.x
        && pos1.x + size1.x > pos2.x
        && pos1.y < pos2.y + size2.y
        && pos1.y + size1.y > pos2.y
    {
        let center1 = pos1 + size1 / 2.0;
        let center2 = pos2 + size2 / 2.0;
        let mut offset_x = if center1.x > center2.x {
            pos2.x + size2.x - pos1.x
        } else if center1.x < center2.x {
            pos2.x - (pos1.x + size1.x)
        } else {
            0.0
        };
        let mut offset_y = if center1.y > center2.y {
            pos2.y + size2.y - pos1.y
        } else if center1.y < center2.y {
            pos2.y - (pos1.y + size1.y)
        } else {
            0.0
        };
        if offset_x.abs() > offset_y.abs() {
            offset_x = 0.0;
        } else {
            offset_y = 0.0;
        }
        Some(cgmath::vec2(offset_x, offset_y))
    } else {
        None
    }
}

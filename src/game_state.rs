use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
};

use cgmath::prelude::*;
use itertools::Itertools;
use stable_vec::StableVec;
use winit::event::ElementState;

#[derive(Clone)]
struct PlayerController {
    pending_events: Vec<Event>,
    controlled_object: usize,
    key_states: HashMap<Direction, ElementState>,
    last_touch_velocity: cgmath::Vector2<f64>,
    top_speed: f64,
    acceleration_speed: f64,
}

impl PlayerController {
    fn update(&mut self, objects: &StableVec<RefCell<Object>>, dt: f64) {
        let mut do_jump = false;
        for event in self.pending_events.drain(..) {
            match event {
                Event::Keyboard { button, state } => {
                    self.key_states.insert(button, state);
                    if let (Direction::Up, ElementState::Pressed) = (button, state) {
                        do_jump = true;
                    }
                }
            }
        }
        let controlled = self.controlled_object;
        let object = objects.get(controlled);
        if let Some(object) = object {
            let mut object = object.borrow_mut();
            if let Object {
                ty: ObjectType::Movable { velocity, .. },
                touching,
                ..
            } = &mut *object
            {
                let touching_sides = touching.iter().fold(HashSet::new(), |mut acc, x| {
                    acc.insert(*x.1);
                    acc
                });
                let average_touch_velocity = if !touching.is_empty() {
                    (|| {
                        let mut weights = 0.0;
                        let mut sum = cgmath::vec2(0.0, 0.0);
                        for index in touching.keys() {
                            let other = &objects[*index].borrow();
                            let contribution = other.surface_friction;
                            if contribution == 0.0 {
                                //fucking glue or smth
                                return other.get_velocity();
                            }
                            let contribution = 1.0 / contribution;
                            sum += other.get_velocity() * contribution;
                            weights += contribution;
                        }
                        sum / weights
                    })()
                } else {
                    self.last_touch_velocity
                };
                self.last_touch_velocity = average_touch_velocity;

                let (left_state, right_state) = (
                    self.key_states
                        .get(&Direction::Left)
                        .unwrap_or(&ElementState::Released),
                    self.key_states
                        .get(&Direction::Right)
                        .unwrap_or(&ElementState::Released),
                );
                if left_state != right_state {
                    if *left_state == ElementState::Pressed {
                        velocity.x += -self.acceleration_speed * dt;
                        if velocity.x < average_touch_velocity.x - self.top_speed {
                            velocity.x = average_touch_velocity.x - self.top_speed;
                        }
                    } else {
                        velocity.x += self.acceleration_speed * dt;
                        if velocity.x > average_touch_velocity.x + self.top_speed {
                            velocity.x = average_touch_velocity.x + self.top_speed;
                        }
                    }
                } else {
                    let target = average_touch_velocity.x - velocity.x;
                    let mut difference = self.acceleration_speed * dt;
                    if difference > target.abs() {
                        difference = target.abs()
                    }
                    velocity.x += difference * target.signum();
                }
                if do_jump && !touching.is_empty() {
                    let mut velocity_offset = cgmath::vec2(0.0, 10.0);
                    if touching_sides.contains(&Direction::Left) {
                        velocity_offset.x += 10.0;
                    } else if touching_sides.contains(&Direction::Right) {
                        velocity_offset.x -= 10.0;
                    }
                    *velocity += velocity_offset;
                }
                if touching_sides.contains(&Direction::Down) {
                    velocity.y += 15.0 * dt;
                }
            }
        }
    }
}

#[derive(Clone)]
enum Controller {
    PlayerController(PlayerController),
}

impl Controller {
    fn update(&mut self, objects: &StableVec<RefCell<Object>>, dt: f64) {
        match self {
            Self::PlayerController(c) => c.update(objects, dt),
        }
    }
}

#[derive(Clone)]
pub enum ObjectType {
    Static,
    Movable {
        velocity: cgmath::Vector2<f64>,
        mass: f64,
    },
    Treadmill {
        fake_velocity: cgmath::Vector2<f64>,
    },
}

#[derive(Clone)]
pub struct Object {
    ty: ObjectType,
    pos: cgmath::Point2<f64>,
    size: cgmath::Vector2<f64>,
    surface_friction: f64,
    touching: HashMap<usize, Direction>,
}

impl Object {
    pub fn get_pos(&self) -> &cgmath::Point2<f64> {
        &self.pos
    }
    pub fn get_size(&self) -> &cgmath::Vector2<f64> {
        &self.size
    }
    fn reset_velocity_components(&mut self, (x, y): (bool, bool)) {
        match &mut self.ty {
            ObjectType::Static { .. } => {}
            ObjectType::Movable { velocity, .. } => {
                if x {
                    velocity.x = 0.0;
                }
                if y {
                    velocity.y = 0.0;
                }
            }
            ObjectType::Treadmill { .. } => {}
        }
    }

    fn apply_push(&mut self, push: cgmath::Vector2<f64>) {
        match &mut self.ty {
            ObjectType::Movable { velocity, .. } => *velocity += push,
            _ => {}
        }
    }

    fn get_velocity(&self) -> cgmath::Vector2<f64> {
        match &self.ty {
            ObjectType::Static => cgmath::vec2(0.0, 0.0),
            ObjectType::Movable { velocity, .. } => *velocity,
            ObjectType::Treadmill { fake_velocity } => *fake_velocity,
        }
    }

    fn can_be_pushed(&self) -> Option<f64> {
        match self.ty {
            ObjectType::Static => None,
            ObjectType::Movable { mass, .. } => Some(mass),
            ObjectType::Treadmill { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    fn invert(&self) -> Self {
        match self {
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
        }
    }
    fn from_vector(vec: &cgmath::Vector2<f64>) -> Self {
        if vec.x.abs() > vec.y.abs() {
            if vec.x > 0.0 {
                Direction::Right
            } else {
                Direction::Left
            }
        } else if vec.y > 0.0 {
            Direction::Up
        } else {
            Direction::Down
        }
    }
}

#[derive(Clone, Copy)]
pub enum Event {
    Keyboard {
        button: Direction,
        state: ElementState,
    },
}

#[derive(Clone)]
pub struct GameState {
    controllers: Vec<Controller>,
    pub objects: StableVec<RefCell<Object>>,
    pub view_object: usize,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            controllers: vec![Controller::PlayerController(PlayerController {
                pending_events: vec![],
                controlled_object: 0,
                key_states: HashMap::new(),
                last_touch_velocity: cgmath::vec2(0.0, 0.0),
                top_speed: 10.0,
                acceleration_speed: 60.0,
            })],
            objects: [
                RefCell::new(Object {
                    pos: cgmath::point2(-0.5, 0.5),
                    size: cgmath::vec2(1.0, 1.0),
                    ty: ObjectType::Movable {
                        velocity: cgmath::vec2(0.0, 0.0),
                        mass: 1.0,
                    },
                    surface_friction: 1.0,
                    touching: HashMap::new(),
                }),
                RefCell::new(Object {
                    pos: cgmath::point2(-25.0, -25.0),
                    size: cgmath::vec2(50.0, 7.5),
                    ty: ObjectType::Static,
                    surface_friction: 1.0,
                    touching: HashMap::new(),
                }),
                RefCell::new(Object {
                    pos: cgmath::point2(17.5, -25.0),
                    size: cgmath::vec2(7.5, 50.0),
                    ty: ObjectType::Static,
                    surface_friction: 1.0,
                    touching: HashMap::new(),
                }),
                RefCell::new(Object {
                    pos: cgmath::point2(-15.0, -19.5),
                    size: cgmath::vec2(10.0, 4.0),
                    ty: ObjectType::Treadmill {
                        fake_velocity: cgmath::vec2(-4.0, 0.0),
                    },
                    surface_friction: 0.5,
                    touching: HashMap::new(),
                }),
            ]
            .into(),
            view_object: 0,
        }
    }
    pub fn update(&mut self, dt: f64) {
        for controller in &mut self.controllers {
            controller.update(&self.objects, dt);
        }
        for (_, object) in &self.objects {
            let mut object = object.borrow_mut();
            let object = &mut *object;
            if let ObjectType::Movable { velocity, .. } = &mut object.ty {
                *velocity -= cgmath::vec2(0.0, 15.0) * dt;
                object.pos += *velocity * dt;
            }
        }

        self.check_whats_still_touching();

        self.collision_detection();
    }
    pub fn submit_player_event(&mut self, event: Event) {
        for controller in &mut self.controllers {
            if let Controller::PlayerController(controller) = controller {
                controller.pending_events.push(event);
            }
        }
    }
    fn collision_detection(&self) {
        for (object1, object2) in self.objects.indices().tuple_combinations() {
            self.handle_collision(object1, object2);
        }
    }

    fn handle_collision(&self, object1_index: usize, object2_index: usize) {
        if object1_index == object2_index {
            return; //shouldn't happen, but just in case, since it would otherwise cause a panic
        }
        if let (Some(object1), Some(object2)) = (
            self.objects.get(object1_index),
            self.objects.get(object2_index),
        ) {
            let mut object1 = object1.borrow_mut();
            let mut object2 = object2.borrow_mut();
            if object1.can_be_pushed().is_some() || object2.can_be_pushed().is_some() {
                let offset = check_collision(
                    object1.get_pos(),
                    object1.get_size(),
                    object2.get_pos(),
                    object2.get_size(),
                );
                if let Some(offset) = offset {
                    let direction = Direction::from_vector(&offset);
                    object1.touching.insert(object2_index, direction.invert());
                    object2.touching.insert(object1_index, direction);
                    object1.reset_velocity_components((offset.x != 0.0, offset.y != 0.0));
                    object2.reset_velocity_components((offset.x != 0.0, offset.y != 0.0));
                    let total = object1.surface_friction * object2.surface_friction;
                    let velocity_offset = if offset.x == 0.0 {
                        cgmath::vec2(
                            (object1.get_velocity().x - object2.get_velocity().x) / total,
                            0.0,
                        )
                    } else if offset.y == 0.0 {
                        cgmath::vec2(
                            0.0,
                            (object1.get_velocity().y - object2.get_velocity().y) / total,
                        )
                    } else {
                        cgmath::vec2(0.0, 0.0)
                    };
                    match (object1.can_be_pushed(), object2.can_be_pushed()) {
                        (Some(mass1), Some(mass2)) => {
                            let ratio = mass1 / (mass1 + mass2);
                            let offset1 = offset * ratio;
                            object1.pos += offset1;
                            object2.pos -= offset - offset1;
                            object1.apply_push(-velocity_offset * ratio);
                            object2.apply_push(velocity_offset * (1.0 - ratio));
                        }
                        (Some(_), None) => {
                            object1.pos += offset;
                            object1.apply_push(-velocity_offset);
                        }
                        (None, Some(_)) => {
                            object2.pos -= offset;
                            object2.apply_push(velocity_offset);
                        }
                        (None, None) => unreachable!(),
                    }
                }
            }
        }
    }

    fn check_whats_still_touching(&mut self) {
        for (index, object) in &self.objects {
            let mut object = object.borrow_mut();
            let touching = object.touching.clone();
            object.touching.clear();
            for (other_index, _) in touching {
                if index == other_index {
                    continue;
                }
                let other_object = self.objects.get(other_index);
                if let Some(other) = other_object {
                    let other = other.borrow();
                    const CHECK_SIZE: f64 = 0.01;
                    let effective_pos = other.pos.map(|a| a - CHECK_SIZE);
                    let effective_size = other.size.map(|a| a + CHECK_SIZE * 2.0);
                    if let Some(offset) =
                        check_collision(&object.pos, &object.size, &effective_pos, &effective_size)
                    {
                        let direction = Direction::from_vector(&offset);
                        object.touching.insert(other_index, direction.invert());
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
        if offset_x == 0.0 || (offset_x.abs() > offset_y.abs() && offset_y != 0.0) {
            offset_x = 0.0;
        } else {
            offset_y = 0.0;
        }
        Some(cgmath::vec2(offset_x, offset_y))
    } else {
        None
    }
}

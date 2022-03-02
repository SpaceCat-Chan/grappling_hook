use cgmath::prelude::*;

#[derive(Clone)]
pub struct GameState {
    pub center: cgmath::Vector2<f64>,
    pub current_angle: cgmath::Rad<f64>,
    pub arm_length: f64,
    time_counter: f64,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            center: cgmath::vec2(0.5, 0.5),
            current_angle: cgmath::Rad(0.0),
            arm_length: 0.1,
            time_counter: 0.0,
        }
    }
    pub fn update(&mut self, dt: f64) {
        self.time_counter += dt;
        self.center.x += self.time_counter.sin() * dt * 0.25;
        self.current_angle += Into::<cgmath::Rad<f64>>::into(cgmath::Deg(45.0)) * dt;
    }
}

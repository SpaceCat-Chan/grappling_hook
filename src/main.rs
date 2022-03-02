mod game_state;
mod render;

use color_eyre::Result;
use std::time::Instant;
use winit::{
    event::{Event, WindowEvent},
    event_loop::ControlFlow,
};

fn main() -> Result<()> {
    const TICK_RATE: f64 = 1.0 / 60.0;

    let event_loop = winit::event_loop::EventLoop::new();

    let window = winit::window::WindowBuilder::new()
        .with_title("Grappling Hook")
        .with_inner_size(winit::dpi::PhysicalSize {
            width: 960,
            height: 960,
        })
        .with_resizable(false)
        .build(&event_loop)?;

    let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);

    let mut state = game_state::GameState::new();
    let mut last_state = state.clone();
    let mut render_state = render::RenderState::new(instance, &window)?;

    let mut accum = 0.0;
    let mut last_time = Instant::now();
    event_loop.run(move |event, _window, control_flow| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event:
                    WindowEvent::MouseInput {
                        state: winit::event::ElementState::Released,
                        button: winit::event::MouseButton::Left,
                        ..
                    },
                ..
            } => {
                state.submit_player_event(());
            }
            Event::MainEventsCleared => {
                let now = Instant::now();
                accum += (now - last_time).as_secs_f64();

                while accum >= TICK_RATE {
                    accum -= TICK_RATE;
                    if accum < TICK_RATE {
                        // last update before render, save previos iteration for interpolation/extrapolation
                        // NOTE: if the state gets too large, it might be worth it to stop doing interpolation to save a bit of time here
                        last_state = state.clone();
                    }
                    state.update(TICK_RATE);
                }

                let render_result = render_state.render(accum / TICK_RATE, &state, &last_state);
                if let Err(e) = render_result {
                    eprintln!("WARNING, Render error occured! {}", e);
                }

                last_time = now;
            }
            _ => {}
        }
    });
}

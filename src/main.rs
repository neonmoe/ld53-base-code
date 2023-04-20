use std::error::Error;
use std::fmt::Display;

use sdl2::event::Event;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::EventPump;

#[cfg(target_family = "wasm")]
mod emscripten_h;

fn main() -> anyhow::Result<()> {
    let sdl_context = sdl2::init().map_err(SdlErr)?;
    let video_subsystem = sdl_context.video().map_err(SdlErr)?;
    let window = video_subsystem
        .window(env!("CARGO_PKG_NAME"), 948, 533)
        .resizable()
        .build()?;
    let canvas = window.into_canvas().build()?;
    let event_pump = sdl_context.event_pump().map_err(SdlErr)?;
    unsafe { STATE = Some(State::new(canvas, event_pump)) };

    #[cfg(target_family = "wasm")]
    emscripten_h::set_main_loop(run_frame);
    #[cfg(not(target_family = "wasm"))]
    loop {
        run_frame()
    }
}

static mut STATE: Option<State> = None;

struct State {
    canvas: Canvas<Window>,
    event_pump: EventPump,
    mouse_position: Point,
}

impl State {
    pub fn new(canvas: Canvas<Window>, event_pump: EventPump) -> State {
        State {
            canvas,
            event_pump,
            mouse_position: Point::new(0, 0),
        }
    }
}

extern "C" fn run_frame() {
    let State {
        canvas,
        event_pump,
        mouse_position,
    } = unsafe { &mut STATE }.as_mut().unwrap();

    for event in event_pump.poll_iter() {
        match event {
            Event::Quit { .. } => std::process::exit(0),
            Event::Window { win_event, .. } => match win_event {
                _ => {}
            },
            Event::MouseMotion { x, y, .. } => *mouse_position = Point::new(x, y),
            _ => {}
        }
    }
    canvas.set_draw_color(Color::RGB(0xFF, 0xFF, 0));
    canvas.clear();
    canvas.set_draw_color(Color::RGB(0xFF, 0, 0));
    canvas
        .fill_rect(Rect::from_center(*mouse_position, 16, 16))
        .unwrap();
    canvas.present();
}

#[derive(Debug)]
pub struct SdlErr(String);
impl Display for SdlErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sdl error: {}", self.0)
    }
}
impl Error for SdlErr {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

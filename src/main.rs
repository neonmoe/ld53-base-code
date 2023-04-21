use std::error::Error;
use std::ffi::{c_int, c_void};
use std::fmt::Display;
use std::ptr;

use sdl2::event::{Event, WindowEvent};
use sdl2::rect::Point;
use sdl2::sys::{SDL_Event, SDL_EventType, SDL_KeyCode};
use sdl2::video::{GLProfile, Window};
use sdl2::{EventPump, VideoSubsystem};

#[cfg(target_family = "wasm")]
mod emscripten_h;
mod renderer;

use renderer::Renderer;

fn main() -> anyhow::Result<()> {
    let sdl_context = sdl2::init().map_err(SdlErr)?;
    let video_subsystem = sdl_context.video().map_err(SdlErr)?;
    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_context_profile(GLProfile::GLES);
    gl_attr.set_context_version(3, 0);
    // Linear->SRGB conversion is done in shader, thanks to lacking WebGL support.
    gl_attr.set_framebuffer_srgb_compatible(false);
    let window = video_subsystem
        .window(env!("CARGO_PKG_NAME"), 948, 533)
        .resizable()
        .opengl()
        .build()?;
    let _gl_context = window.gl_create_context().map_err(SdlErr)?;
    let event_pump = sdl_context.event_pump().map_err(SdlErr)?;

    // Set up an event filter to avoid too eager preventDefault()s on
    // emscripten.
    extern "C" fn event_filter(_: *mut c_void, event: *mut SDL_Event) -> c_int {
        const DROPPED: c_int = 0;
        const ACCEPTED: c_int = 1;
        if let Some(event) = unsafe { event.as_ref() } {
            const KEYDOWN: u32 = SDL_EventType::SDL_KEYDOWN as u32;
            const KEYUP: u32 = SDL_EventType::SDL_KEYUP as u32;
            match unsafe { event.type_ } {
                KEYDOWN | KEYUP => {
                    let key_event = unsafe { event.key };
                    let keycode = key_event.keysym.sym;
                    // Here, we specifically "unignore"
                    if keycode == SDL_KeyCode::SDLK_SPACE as i32 {
                        ACCEPTED
                    } else {
                        DROPPED
                    }
                }
                _ => ACCEPTED,
            }
        } else {
            ACCEPTED
        }
    }
    unsafe { sdl2::sys::SDL_SetEventFilter(Some(event_filter), ptr::null_mut()) };

    unsafe { STATE = Some(State::new(video_subsystem, window, event_pump)) };

    #[cfg(target_family = "wasm")]
    emscripten_h::set_main_loop(run_frame);
    #[cfg(not(target_family = "wasm"))]
    loop {
        run_frame()
    }
}

static mut STATE: Option<State> = None;

struct State {
    window: Window,
    event_pump: EventPump,
    mouse_position: Point,
    renderer: Renderer,
}

impl State {
    pub fn new(video: VideoSubsystem, window: Window, event_pump: EventPump) -> State {
        State {
            renderer: Renderer::new(&video, &window),
            window,
            event_pump,
            mouse_position: Point::new(0, 0),
        }
    }
}

extern "C" fn run_frame() {
    let State {
        event_pump,
        mouse_position,
        renderer,
        window,
        ..
    } = unsafe { &mut STATE }.as_mut().unwrap();

    for event in event_pump.poll_iter() {
        match event {
            Event::Quit { .. } => std::process::exit(0),
            Event::Window { win_event, .. } => match win_event {
                WindowEvent::Resized(w, h) => renderer.resize(w, h),
                _ => {}
            },
            Event::MouseMotion { x, y, .. } => *mouse_position = Point::new(x, y),
            Event::KeyDown { keycode, .. } => println!("Pressed {keycode:?}!"),
            _ => {}
        }
    }

    renderer.render();
    window.gl_swap_window();
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

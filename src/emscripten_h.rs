use core::ffi::{c_int};

pub fn set_main_loop(func: EmCallbackFunc) -> ! {
    unsafe { emscripten_set_main_loop(func, 0, 1) };
    // emscripten_set_main_loop with simulate_infinite_loop set to true will
    // throw an exception to stop execution of the caller, i.e. we never end up
    // here. The loop {} here just reflects the actual "return value" of
    // emscripten_set_main_loop, which would in this case be "!".
    loop {}
}

pub type EmCallbackFunc = extern "C" fn();
extern "C" {
    /// https://emscripten.org/docs/api_reference/emscripten.h.html#c.emscripten_set_main_loop
    pub fn emscripten_set_main_loop(
        func: EmCallbackFunc,
        fps: c_int,
        simulate_infinite_loop: c_int,
    );
}

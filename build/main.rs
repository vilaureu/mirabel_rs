//! Generate bindings.

mod mirabel;

#[cfg(feature = "skia")]
mod gl;

fn main() {
    mirabel::bindings();
    #[cfg(feature = "skia")]
    gl::generate();
}

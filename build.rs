//! Generate bindings.

fn main() {
    #[cfg(feature = "skia")]
    generate_gl();
}

/// Generate _OpenGL_ bindings which match the usage in _mirabel_.
#[cfg(feature = "skia")]
fn generate_gl() {
    use gl_generator::{Api, Fallbacks, Profile, Registry, StaticGenerator};
    use std::env;
    use std::fs::File;
    use std::path::Path;

    let dest = env::var("OUT_DIR").unwrap();
    let mut file = File::create(&Path::new(&dest).join("gl.rs")).unwrap();

    // This needs to follow client.cpp in mirabel.
    Registry::new(Api::Gl, (3, 0), Profile::Core, Fallbacks::All, [])
        .write_bindings(StaticGenerator, &mut file)
        .unwrap();
}

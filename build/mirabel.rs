//! This invokes _bindgen_ to generate _mirabel_ bindings.

use std::{
    env,
    fmt::Write,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

/// Generate bindings for _surena_ and/or _mirabel_.
pub(crate) fn bindings() {
    #[allow(unused_mut)]
    let mut headers = vec!["surena/game_plugin.h"];
    #[allow(unused_mut)]
    let mut allowed_project = vec![
        "lib/surena/includes/surena/game.h",
        "lib/surena/includes/surena/util/serialization.h",
    ];
    #[allow(unused_mut)]
    let mut allowed_system = vec![];

    #[cfg(feature = "mirabel")]
    {
        headers.extend_from_slice(&[
            "mirabel/frontend_plugin.h",
            "mirabel/imgui_c_thin.h",
            "mirabel/log.h",
        ]);
        allowed_project.extend_from_slice(&[
            "includes/mirabel/frontend.h",
            "includes/mirabel/event.h",
            "includes/mirabel/event_queue.h",
            "includes/mirabel/imgui_c_thin.h",
            "includes/mirabel/log.h",
        ]);
        allowed_system.extend_from_slice(&[
            "SDL2/SDL_events.h",
            "SDL2/SDL_video.h",
            "SDL2/SDL_mouse.h",
        ]);
    }

    let mut builder = bindgen::Builder::default()
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .derive_default(true)
        .derive_eq(true);

    let mut contents = String::new();
    for header in headers {
        writeln!(contents, r#"#include "{header}""#).unwrap();
    }
    builder = builder.header_contents("wrapper.h", &contents);
    for allow in finalize_mirabel_headers(allowed_project) {
        // prevent compilation errors from compiling libc headers
        builder = builder.allowlist_file(regex::escape(&allow));
    }
    for allow in allowed_system {
        builder = builder.allowlist_file(format!(r#"(?:^|.*/){}"#, regex::escape(allow)));
    }
    for include in mirabel_includes() {
        builder = builder.clang_arg(format!("-Imirabel/{include}"));
    }
    // Block variables which break because of https://github.com/rust-lang/rust-bindgen/issues/753
    let builder = builder.blocklist_item(regex::escape("LS_ERR"));
    let builder = builder.blocklist_item(regex::escape("MOVE_NONE"));

    let bindings = builder.generate().expect("unable to generate bindings");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("mirabel.rs"))
        .expect("failed to write bindings");
}

/// Deduplicate headers and prepend `mirabel/`.
fn finalize_mirabel_headers(mut headers: Vec<&str>) -> impl Iterator<Item = String> + '_ {
    headers.sort();
    headers.dedup();
    headers.into_iter().map(|h| format!("mirabel/{h}"))
}

/// Returns a list of the includes in the _mirabel_ `CMakeLists.txt`.
fn mirabel_includes() -> Vec<String> {
    const CMAKE_LISTS: &str = "mirabel/CMakeLists.txt";

    println!("cargo:rerun-if-changed={CMAKE_LISTS}");
    let file = File::open(CMAKE_LISTS).expect("failed to open mirabel's CMakeLists.txt");
    let mut reader = BufReader::new(file);

    let mut includes = vec![];
    let mut inside_includes = false;
    let mut line = String::new();
    loop {
        let length = reader
            .read_line(&mut line)
            .expect("reading CMakeLists.txt failed");
        if length == 0 {
            panic!("unexpected end of CMakeLists.txt");
        }
        let text = line.trim();

        if !inside_includes && text.eq_ignore_ascii_case("set(INCLUDES") {
            inside_includes = true;
        } else if inside_includes && text == ")" {
            break;
        } else if inside_includes && !text.is_empty() {
            includes.push(text.to_string());
        }

        line.clear();
    }

    includes
}

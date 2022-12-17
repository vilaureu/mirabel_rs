//! An example of how to use the _mirabel_ frontend wrapper.

use std::ptr::addr_of;

use mirabel::{
    frontend::{
        skia::{Color4f, Font, Paint, Point, Rect, TextBlob},
        *,
    },
    mirabel_sys::cstr_to_rust_unchecked,
    *,
};

/// Runtime data of the frontend.
struct Frontend {
    game_name: String,
    button_pressed: u64,
    checkbox: bool,
    text: String,
    slider: f32,
    scalar: i8,
    mouse_location: Option<Point>,
    highlight_area: Option<Rect>,
    click_location: Option<Point>,
}

impl Frontend {
    const DEFAULT_GAME_NAME: &str = "No game loaded!";
}

impl FrontendMethods for Frontend {
    /// Pre-create options. In this case a [`bool`].
    type Options = bool;

    /// Creates an instance of the frontend.
    fn create(_options: Option<&Self::Options>) -> Result<Self> {
        Ok(Self::default())
    }

    /// Displays the runtime options using _ImGui_.
    fn runtime_opts_display(mut frontend: Wrapped<Self>) -> Result<()> {
        if !frontend.options.unwrap() {
            return Ok(());
        }

        if imgui::button(cstr("Press Me!\0")) {
            frontend.button_pressed = frontend.button_pressed.saturating_add(1);
        }
        imgui::text(&format!(
            "Button was pressed {} times.",
            frontend.button_pressed
        ));
        imgui::check_box(cstr("Checkbox\0"), &mut frontend.checkbox);

        imgui::begin_disabled(frontend.checkbox);
        imgui::input_text(cstr("Text Input\0"), &mut frontend.text, 15);
        imgui::slider_scalar(cstr("Slider\0"), &mut frontend.slider, 0f32, 42f32);
        imgui::input_scalar(cstr("Scalar Input\0"), &mut frontend.scalar);
        imgui::end_disabled();

        Ok(())
    }

    /// Process _mirabel_ events.
    fn process_event(mut frontend: Wrapped<Self>, event: EventAny) -> Result<()> {
        match event.to_rust() {
            EventEnum::GameLoadMethods(e) => {
                frontend.game_name = format!("Loaded game: {}", unsafe {
                    cstr_to_rust_unchecked(*addr_of!((*e.methods).game_name))
                })
            }
            EventEnum::GameUnload(_) => frontend.game_name = Self::DEFAULT_GAME_NAME.to_string(),
            _ => {}
        }

        Ok(())
    }

    /// Process _SDL_ events.
    fn process_input(mut frontend: Wrapped<Self>, event: SDLEventEnum) -> Result<()> {
        match event {
            SDLEventEnum::MouseMotion(event) => {
                frontend.mouse_location = Some((event.x, event.y).into());
            }
            SDLEventEnum::MouseButtonUp(event) => {
                frontend.click_location = Some((event.x, event.y).into());
            }
            _ => (),
        };

        Ok(())
    }

    /// Update the internal state.
    fn update(mut frontend: Wrapped<Self>) -> Result<()> {
        let Some(mouse) = frontend.mouse_location else {
             return Ok(());
        };

        let width = frontend.display_data.w;
        let half = width / 2.;
        let p0 = Point::new(half, frontend.display_data.h);

        let p1x = if mouse.x < half { 0. } else { width };

        frontend.highlight_area = Some(Rect::new(p0.x, p0.y, p1x, 0.));

        Ok(())
    }

    /// Render the background using _Skia_.
    fn render(mut frontend: Wrapped<Self>) -> Result<()> {
        let display_data = frontend.display_data;
        let highlight_area = frontend.highlight_area;
        let click_location = frontend.click_location;
        let c = frontend.canvas.get();

        c.clear(Color4f::new(1., 1., 1., 1.));
        if let Some(area) = highlight_area {
            c.draw_rect(area, &Paint::new(Color4f::new(1., 0.8, 0.8, 1.), None));
        }
        c.draw_circle((0, 0), 50., &Paint::new(Color4f::new(0., 0., 0., 1.), None));
        c.draw_circle(
            (display_data.w, 0.),
            50.,
            &Paint::new(Color4f::new(1., 0., 0., 1.), None),
        );
        c.draw_circle(
            (display_data.w, display_data.h),
            50.,
            &Paint::new(Color4f::new(0., 1., 0., 1.), None),
        );
        c.draw_circle(
            (0., display_data.h),
            50.,
            &Paint::new(Color4f::new(0., 0., 1., 1.), None),
        );
        c.draw_text_blob(
            TextBlob::new("Hello World", &Font::default()).expect("text error"),
            (50, 50),
            &Paint::new(Color4f::new(0., 0., 0., 1.), None),
        );
        c.draw_text_blob(
            TextBlob::new(&frontend.frontend.game_name, &Font::default()).expect("text error"),
            (50, 100),
            &Paint::new(Color4f::new(0., 0., 0., 1.), None),
        );

        if let Some(location) = click_location {
            let mut color = Paint::new(Color4f::new(0., 0., 0., 0.5), None);
            color.set_stroke(true);
            c.draw_circle(location, 5., &color);
        }

        Ok(())
    }

    /// Determine whether we are compatible to the game or not.
    fn is_game_compatible(game: GameInfo) -> CodeResult<()> {
        if !game.game_name.eq_ignore_ascii_case("chess") {
            Ok(())
        } else {
            Err(ErrorCode::FeatureUnsupported)
        }
    }

    /// Create the pre-create options.
    fn opts_create() -> CodeResult<Self::Options> {
        Ok(true)
    }

    /// Display pre-create options using _ImGui_.
    fn opts_display(options_struct: &mut Self::Options) -> CodeResult<()> {
        imgui::check_box(cstr("Show Runtime Options?\0"), options_struct);

        Ok(())
    }
}

impl Default for Frontend {
    fn default() -> Self {
        Self {
            game_name: Self::DEFAULT_GAME_NAME.to_string(),
            button_pressed: Default::default(),
            checkbox: Default::default(),
            text: "prefilled...".to_string(),
            slider: Default::default(),
            scalar: Default::default(),
            mouse_location: Default::default(),
            highlight_area: Default::default(),
            click_location: Default::default(),
        }
    }
}

/// Create the [`frontend_methods`] for this frontend.
fn example_frontend_methods() -> frontend_methods {
    let mut features = frontend_feature_flags::default();
    features.set_options(true);

    create_frontend_methods::<Frontend>(Metadata {
        frontend_name: cstr("Example\0"),
        version: semver {
            major: 0,
            minor: 1,
            patch: 0,
        },
        features,
    })
}

// Generate the exported functions for _mirabel_.
plugin_get_frontend_methods!(example_frontend_methods());

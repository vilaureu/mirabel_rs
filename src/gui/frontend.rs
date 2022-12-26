//! A plugin wrapper for _mirabel_ frontends in (mostly) safe Rust.

use std::{
    marker::PhantomData,
    os::raw::{c_char, c_void},
    ptr::{addr_of, addr_of_mut, null_mut},
};

use crate::sdl_event::SDLEventEnum;
use crate::CodeResult;
use crate::{
    cstr_to_rust,
    error::*,
    event::*,
    game::game_feature_flags,
    sys::{
        self, error_code, event_any, event_queue, frontend_display_data, frontend_methods, semver,
        ERR_ERR_OK,
    },
    ValidCStr,
};

#[cfg(feature = "skia")]
use super::skia_helper;

#[cfg(feature = "skia")]
pub use skia_safe as skia;

pub use crate::sys::frontend_feature_flags;

/// This macro creates the `plugin_get_frontend_methods` function.
///
/// Is must be supplied with all frontend structs and their [`Metadata`]
/// structures so that they can be exported.
/// This macro will internally call [`create_frontend_methods()`] to guarantee
/// safe usage.
/// This method can only be called once but with multiple methods.
/// It also creates the `plugin_init_frontend`,
/// `plugin_get_frontend_capi_version`, and `plugin_cleanup_frontend` functions
/// for you.
///
/// # Example
/// ```ignore
/// fn generate_metadata() -> Metadata {
///     /* ... */
/// }
/// plugin_get_frontend_methods!(MyFrontend{generate_metadata()});
/// ```
#[macro_export]
macro_rules! plugin_get_frontend_methods {
    ( $( $f:ty{$m:expr} ),* ) => {
        static mut PLUGIN_FRONTEND_METHODS: ::std::mem::MaybeUninit<
            [$crate::sys::frontend_methods; $crate::count!($($f),*)]
        > = ::std::mem::MaybeUninit::uninit();

        #[no_mangle]
        unsafe extern "C" fn plugin_init_frontend() {
            ::std::mem::MaybeUninit::write(&mut self::PLUGIN_FRONTEND_METHODS,
                [$(create_frontend_methods::<$f>($m)),*]
            );
        }

        #[no_mangle]
        unsafe extern "C" fn plugin_get_frontend_methods(
            count: *mut u32,
            methods: *mut *const $crate::sys::frontend_methods,
        ) {
            count.write($crate::count!($($f),*));
            if methods.is_null() {
                return;
            }

            let src = ::std::mem::MaybeUninit::assume_init_ref(
                &self::PLUGIN_FRONTEND_METHODS
            );
            for i in 0..$crate::count!($($f),*) {
                methods.add(i).write(&src[i]);
            }
        }

        #[no_mangle]
        unsafe extern "C" fn plugin_cleanup_frontend() {
            // The static array of C structs does not need cleanup.
        }

        #[no_mangle]
        extern "C" fn plugin_get_frontend_capi_version() -> u64 {
            $crate::sys::MIRABEL_FRONTEND_API_VERSION
        }
    };
}

macro_rules! mirabel_try {
    ( $game: expr, $result:expr ) => {
        match $result {
            Ok(v) => v,
            Err(error) => {
                Aux::<F>::get($game).set_error(error.message);
                return error.code.into();
            }
        }
    };
}

/// This is the trait providing the plugin wrapper.
///
/// A plugin can be created by filling in the required methods.
/// Optional features can be implemented by filling in the associated provided
/// methods.
///
/// For documentation on the expected behavior of the individual functions
/// see `mirabel/includes/mirabel/frontend.h`.
///
/// # Example
/// See the `example` crate in the project root.
pub trait FrontendMethods: Sized {
    /// The associated type for storing the pre-create options.
    type Options;

    fn create(options: Option<&Self::Options>) -> Result<Self>;
    fn runtime_opts_display(&mut self, ctx: Context<Self>) -> Result<()>;
    fn process_event(&mut self, ctx: Context<Self>, event: EventAny) -> Result<()>;
    fn process_input(&mut self, ctx: Context<Self>, event: SDLEventEnum) -> Result<()>;
    fn update(&mut self, ctx: Context<Self>) -> Result<()>;
    fn render(&mut self, ctx: Context<Self>) -> Result<()>;
    fn is_game_compatible(game: GameInfo) -> CodeResult<()>;

    fn opts_create() -> CodeResult<Self::Options> {
        unimplemented!("opts_create")
    }

    #[allow(unused_variables)]
    fn opts_display(options_struct: &mut Self::Options) -> CodeResult<()> {
        unimplemented!("opts_display")
    }
}

unsafe extern "C" fn opts_create_wrapped<F: FrontendMethods>(
    options_struct: *mut *mut c_void,
) -> error_code {
    options_struct.write(null_mut());
    match F::opts_create() {
        Ok(options) => {
            *options_struct = Box::into_raw(Box::new(options)).cast::<c_void>();
            ERR_ERR_OK
        }
        Err(code) => code.into(),
    }
}

unsafe extern "C" fn opts_display_wrapped<F: FrontendMethods>(
    options_struct: *mut c_void,
) -> error_code {
    match F::opts_display(&mut *options_struct.cast::<F::Options>()) {
        Ok(()) => ERR_ERR_OK,
        Err(code) => code.into(),
    }
}

unsafe extern "C" fn opts_destroy_wrapped<F: FrontendMethods>(
    options_struct: *mut c_void,
) -> error_code {
    drop(Box::from_raw(options_struct.cast::<F::Options>()));
    ERR_ERR_OK
}

unsafe extern "C" fn get_last_error_wrapped<F: FrontendMethods>(
    frontend: *mut sys::frontend,
) -> *const c_char {
    (&Aux::<F>::get(frontend).error).into()
}

unsafe extern "C" fn create_wrapped<F: FrontendMethods>(
    frontend: *mut sys::frontend,
    display_data: *mut frontend_display_data,
    options_struct: *mut c_void,
) -> error_code {
    let options_struct = options_struct.cast::<F::Options>();

    // Initialize data1 to zero in case creation fails.
    let data1: *mut *mut c_void = addr_of_mut!((*frontend).data1);
    data1.write(null_mut());
    Aux::<F>::init(frontend, display_data, options_struct);

    // TODO: maybe supply display_data to create

    let data = mirabel_try!(frontend, F::create(options_struct.as_ref()));
    // data1 is already initialized.
    *data1 = Box::into_raw(Box::<F>::new(data)).cast::<c_void>();

    sys::ERR_ERR_FEATURE_UNSUPPORTED
}

unsafe extern "C" fn destroy_wrapped<F: FrontendMethods>(
    frontend: *mut sys::frontend,
) -> error_code {
    let data: &mut *mut c_void = &mut *addr_of_mut!((*frontend).data1);
    if !data.is_null() {
        drop(Box::from_raw(data.cast::<F>()));
        // Leave as null pointer to catch use-after-free errors.
        *data = null_mut();
    }
    Aux::<F>::free(frontend);

    ERR_ERR_OK
}

unsafe extern "C" fn runtime_opts_display_wrapped<F: FrontendMethods>(
    frontend: *mut sys::frontend,
) -> error_code {
    mirabel_try!(
        frontend,
        F::runtime_opts_display(get_self(frontend), Context::new(frontend))
    );

    ERR_ERR_OK
}

unsafe extern "C" fn process_event_wrapped<F: FrontendMethods>(
    frontend: *mut sys::frontend,
    event: event_any,
) -> error_code {
    let event = EventAny::new(event);

    mirabel_try!(
        frontend,
        F::process_event(get_self(frontend), Context::new(frontend), event)
    );

    ERR_ERR_OK
}

unsafe extern "C" fn process_input_wrapped<F: FrontendMethods>(
    frontend: *mut sys::frontend,
    event: sys::SDL_Event,
) -> error_code {
    let event = SDLEventEnum::new(event);
    #[cfg(feature = "skia")]
    if let SDLEventEnum::WindowEvent(event) = event {
        use crate::sys::SDL_WindowEventID_SDL_WINDOWEVENT_SIZE_CHANGED;
        if u32::from(event.event) == SDL_WindowEventID_SDL_WINDOWEVENT_SIZE_CHANGED {
            Aux::<F>::get(frontend).surface = None;
        }
    }

    mirabel_try!(
        frontend,
        F::process_input(get_self(frontend), Context::new(frontend), event)
    );

    ERR_ERR_OK
}

unsafe extern "C" fn update_wrapped<F: FrontendMethods>(
    frontend: *mut sys::frontend,
) -> error_code {
    mirabel_try!(
        frontend,
        F::update(get_self(frontend), Context::new(frontend))
    );

    ERR_ERR_OK
}

unsafe extern "C" fn render_wrapped<F: FrontendMethods>(
    frontend: *mut sys::frontend,
) -> error_code {
    mirabel_try!(
        frontend,
        F::render(get_self(frontend), Context::new(frontend))
    );
    #[cfg(feature = "skia")]
    if let Some(surface) = &mut Aux::<F>::get(frontend).surface {
        surface.flush();
    }

    ERR_ERR_OK
}

unsafe extern "C" fn is_game_compatible_wrapped<F: FrontendMethods>(
    compat_game: *const sys::game_methods,
) -> error_code {
    let game = GameInfo::new(compat_game);
    match F::is_game_compatible(game) {
        Ok(()) => ERR_ERR_OK,
        Err(code) => code.into(),
    }
}

/// Extract the `self` of a frontend from [`data1`](sys::frontend::data1) with
/// arbitrary lifetime.
#[inline]
unsafe fn get_self<'l, F>(frontend: *mut sys::frontend) -> &'l mut F {
    let data: *mut c_void = *addr_of_mut!((*frontend).data1);
    &mut *data.cast::<F>()
}

/// This provides access to context information and additional tools.
pub struct Context<'l, F: FrontendMethods> {
    /// A read-only reference to the pre-create options.
    pub options: Option<&'l F::Options>,
    /// Some additional information provided by _mirabel_.
    pub display_data: &'l frontend_display_data,
    /// A helper for sending events to the _mirabel_ core.
    pub outbox: QueueManager<'l>,
    /// A _Skia_ canvas for drawing the frontend.
    #[cfg(feature = "skia")]
    pub canvas: CanvasManager<'l>,
}

impl<'l, F: FrontendMethods + 'l> Context<'l, F> {
    #[inline]
    unsafe fn new(frontend: *mut sys::frontend) -> Self {
        let aux = Aux::<F>::get(frontend);
        let display_data = &*aux.display_data;
        Self {
            // It is ok to use a reference here for options and display_data
            // because 'l does not outlive the wrapper function.
            options: aux.options.as_ref(),
            display_data,
            outbox: QueueManager {
                outbox: display_data.outbox,
                phantom: Default::default(),
            },
            #[cfg(feature = "skia")]
            canvas: CanvasManager {
                surface: &mut aux.surface,
                display_data,
            },
        }
    }
}

/// A wrapper around [`event_queue`] for safely sending events.
pub struct QueueManager<'l> {
    outbox: *mut event_queue,
    phantom: PhantomData<&'l mut event_queue>,
}

impl<'l> QueueManager<'l> {
    /// Copy an event to the outbox.
    #[inline]
    pub fn push(&mut self, event: &mut EventAny) {
        unsafe {
            sys::event_queue_push(self.outbox, &mut **event);
        }
    }
}

/// A wrapper around [`skia::Surface`] for lazy creation of a [`skia::Canvas`].
#[cfg(feature = "skia")]
pub struct CanvasManager<'l> {
    surface: &'l mut Option<skia::Surface>,
    pub display_data: &'l frontend_display_data,
}

#[cfg(feature = "skia")]
impl<'l> CanvasManager<'l> {
    /// Create a new [`skia::Canvas`] for drawing on it.
    ///
    /// This also adjusts the origin to the visible area.
    #[must_use]
    pub fn get(&mut self) -> &mut skia::Canvas {
        self.surface
            .get_or_insert_with(|| {
                skia_helper::create_surface(
                    self.display_data.fbw as i32,
                    self.display_data.fbh as i32,
                )
            })
            .canvas()
    }

    /// Returns a translation matrix.
    ///
    /// The returned matrix sets the origin of the frame to the top left of the
    /// main drawing area.
    /// Use in [`skia::Canvas::set_matrix()`].
    #[must_use]
    #[inline]
    pub fn matrix(&self) -> skia::Matrix {
        skia::Matrix::translate((self.display_data.x, self.display_data.y))
    }
}

/// Basic information about a game.
///
/// This is derived from the [`game_methods`](sys::game_methods).
pub struct GameInfo<'l> {
    pub game_name: &'l str,
    pub variant_name: &'l str,
    pub impl_name: &'l str,
    pub version: semver,
    pub features: game_feature_flags,
}

impl<'l> GameInfo<'l> {
    #[inline]
    unsafe fn new(methods: *const sys::game_methods) -> Self {
        Self {
            game_name: cstr_to_rust(*addr_of!((*methods).game_name)).unwrap_unchecked(),
            variant_name: cstr_to_rust(*addr_of!((*methods).variant_name)).unwrap_unchecked(),
            impl_name: cstr_to_rust(*addr_of!((*methods).impl_name)).unwrap_unchecked(),
            version: *addr_of!((*methods).version),
            features: *addr_of!((*methods).features),
        }
    }
}

/// Non-function members for [`frontend_methods`].
///
/// # Example
/// ```
/// # use mirabel::{cstr, sys::semver, frontend::*};
/// use std::ffi::CStr;
///
/// let mut features = frontend_feature_flags::default();
/// features.set_options(true);
///
/// let metadata = Metadata {
///     frontend_name: cstr("Example\0"),
///     version: semver {
///         major: 0,
///         minor: 1,
///         patch: 0,
///     },
///     features,
/// };
/// ```
pub struct Metadata {
    pub frontend_name: ValidCStr<'static>,
    pub version: semver,
    pub features: frontend_feature_flags,
}

/// Create _mirabel_ [`frontend_methods`] from frontend struct `F` and
/// `metadata`.
///
/// If feature flags are disabled, corresponding function pointers will be set
/// to zero.
///
/// # Example
/// ```ignore
/// create_frontend_methods::<MyFrontend>(metadata);
/// ```
pub fn create_frontend_methods<F: FrontendMethods>(metadata: Metadata) -> frontend_methods {
    frontend_methods {
        frontend_name: metadata.frontend_name.into(),
        version: metadata.version,
        features: metadata.features,
        opts_create: if metadata.features.options() {
            Some(opts_create_wrapped::<F>)
        } else {
            None
        },
        opts_display: if metadata.features.options() {
            Some(opts_display_wrapped::<F>)
        } else {
            None
        },
        opts_destroy: if metadata.features.options() {
            Some(opts_destroy_wrapped::<F>)
        } else {
            None
        },
        get_last_error: Some(get_last_error_wrapped::<F>),
        create: Some(create_wrapped::<F>),
        destroy: Some(destroy_wrapped::<F>),
        runtime_opts_display: Some(runtime_opts_display_wrapped::<F>),
        process_event: Some(process_event_wrapped::<F>),
        process_input: Some(process_input_wrapped::<F>),
        update: Some(update_wrapped::<F>),
        render: Some(render_wrapped::<F>),
        is_game_compatible: Some(is_game_compatible_wrapped::<F>),
        ..Default::default()
    }
}

struct Aux<'l, F: FrontendMethods> {
    error: ErrorString,
    /// Up-to-date metadata required for displaying and communicating.
    ///
    /// This data will get mutated by _mirabel_.
    /// Hence, we store a pointer and not a reference here.
    display_data: *mut frontend_display_data,
    /// General options for this frontend.
    ///
    /// The options might get mutated by [`FrontendMethods::opts_display()`].
    /// Hence, we store a pointer and not a reference here.
    options: *const F::Options,
    #[cfg(feature = "skia")]
    surface: Option<skia::Surface>,
    phantom: PhantomData<(&'l mut frontend_display_data, &'l F::Options)>,
}

impl<'l, F: FrontendMethods> Aux<'l, F>
where
    F::Options: 'l,
{
    unsafe fn init(
        frontend: *mut sys::frontend,
        display_data: *mut frontend_display_data,
        options: *const F::Options,
    ) {
        // Initialize data2 to zero in case creation fails.
        let data2: *mut *mut c_void = addr_of_mut!((*frontend).data2);
        data2.write(null_mut());
        let aux = Box::into_raw(Box::<Self>::new(Self {
            error: Default::default(),
            display_data,
            options,
            #[cfg(feature = "skia")]
            surface: Default::default(),
            phantom: Default::default(),
        }));
        *data2 = aux.cast();
    }

    #[inline]
    #[must_use]
    unsafe fn get<'a>(frontend: *mut sys::frontend) -> &'a mut Self {
        let data2: *mut *mut c_void = addr_of_mut!((*frontend).data2);
        &mut *(*data2).cast::<Self>()
    }

    unsafe fn free(frontend: *mut sys::frontend) {
        let aux: &mut *mut c_void = &mut *addr_of_mut!((*frontend).data2);
        if !aux.is_null() {
            drop(Box::from_raw(aux.cast::<Self>()));
            // Leave as null pointer to catch use-after-free errors.
            *aux = null_mut();
        }
    }

    #[inline]
    fn set_error(&mut self, error: ErrorString) {
        self.error = error;
    }
}

//! This is a wrapper library for the game API of the game engine.

pub use crate::sys::{
    buf_sizer, game_feature_flags, move_code, player_id, semver, MOVE_NONE, PLAYER_NONE,
    PLAYER_RAND,
};

use crate::{
    cstr_to_rust, cstr_to_rust_unchecked,
    error::{ErrorString, Result},
    game_init::GameInit,
    ptr_vec::PtrVec,
    sys::{self, game_methods},
    ValidCStr,
};

use std::{
    ffi::c_void,
    num::NonZeroU8,
    os::raw::c_char,
    ptr::{addr_of, addr_of_mut, null_mut},
};

/// This macro creates the `plugin_get_game_methods` function.
///
/// Is must be supplied with all game structs and their [`Metadata`] structures
/// so that they can be exported.
/// This macro will internally call [`create_game_methods()`] to guarantee safe
/// usage.
/// This method can only be called once but with multiple methods.
/// It also exports the `plugin_init_game`, `plugin_get_game_capi_version`, and
/// `plugin_cleanup_game` functions for you.
///
/// # Example
/// ```ignore
/// fn generate_metadata() -> Metadata {
///     /* ... */
/// }
/// plugin_get_game_methods!(MyGame{generate_metadata()});
/// ```
#[macro_export]
macro_rules! plugin_get_game_methods {
    ( $( $g:ty{$m:expr} ),* ) => {
        static mut PLUGIN_GAME_METHODS: ::std::mem::MaybeUninit<
            [$crate::sys::game_methods; $crate::count!($($g),*)]
        > = ::std::mem::MaybeUninit::uninit();

        #[no_mangle]
        unsafe extern "C" fn plugin_init_game() {
            ::std::mem::MaybeUninit::write(&mut self::PLUGIN_GAME_METHODS,
                [$(create_game_methods::<$g>($m)),*]
            );
        }

        #[no_mangle]
        pub unsafe extern "C" fn plugin_get_game_methods(
            count: *mut u32,
            methods: *mut *const $crate::sys::game_methods,
        ) {
            count.write($crate::count!($($g),*));
            if methods.is_null() {
                return;
            }

            let src = ::std::mem::MaybeUninit::assume_init_ref(
                &self::PLUGIN_GAME_METHODS
            );
            for i in 0..$crate::count!($($g),*) {
                methods.add(i).write(&src[i]);
            }
        }

        #[no_mangle]
        unsafe extern "C" fn plugin_cleanup_game() {
            // The static array of C structs does not need cleanup.
        }

        /// This exports the game API version to the outside.
        #[no_mangle]
        pub extern "C" fn plugin_get_game_capi_version() -> u64 {
            $crate::sys::SURENA_GAME_API_VERSION
        }
    };
}

macro_rules! surena_try {
    ( $game: expr, $result:expr ) => {
        match $result {
            Ok(v) => v,
            Err(error) => {
                Aux::get($game).set_error(error.message);
                return error.code.into();
            }
        }
    };
}

/// A [`PtrVec`] for writing and returning C strings.
pub type StrBuf<'b> = PtrVec<'b, NonZeroU8>;

/// Main trait which needs to be implemented by your game struct.
///
/// See `game.h` @ _surena_ for API documentation.
/// You should **not implement `[...]_wrapped`** methods.
///
/// Games need to implement [`Drop`] for custom `destroy` handling.
/// `clone` is handled by the [`Clone`] implementation and `compare` by [`Eq`].
/// The [`Send`] bound is required by the surena API.
///
/// # Example
/// See the `./example` crate in the project root.
pub trait GameMethods: Sized + Clone + Eq + Send {
    fn create(init_info: &GameInit) -> Result<(Self, buf_sizer)>;
    fn copy_from(&mut self, other: &mut Self) -> Result<()>;
    fn import_state(&mut self, string: Option<&str>) -> Result<()>;
    fn export_state(&mut self, str_buf: &mut StrBuf) -> Result<()>;
    fn players_to_move(&mut self, players: &mut PtrVec<player_id>) -> Result<()>;
    fn get_concrete_moves(
        &mut self,
        player: player_id,
        moves: &mut PtrVec<move_code>,
    ) -> Result<()>;
    fn get_move_code(&mut self, player: player_id, string: &str) -> Result<move_code>;
    fn get_move_str(
        &mut self,
        player: player_id,
        mov: move_code,
        str_buf: &mut StrBuf,
    ) -> Result<()>;
    fn make_move(&mut self, player: player_id, mov: move_code) -> Result<()>;
    fn get_results(&mut self, players: &mut PtrVec<player_id>) -> Result<()>;
    /// Sync counters are currently not supported.
    #[allow(clippy::wrong_self_convention)]
    fn is_legal_move(&mut self, player: player_id, mov: move_code) -> Result<()>;

    /// Must be implemented when the [`game_feature_flags::options`] is enabled.
    #[allow(unused_variables)]
    fn export_options(&mut self, str_buf: &mut StrBuf) -> Result<()> {
        unimplemented!("export_options")
    }
    /// Must be implemented when the [`game_feature_flags::print`] is enabled.
    #[allow(unused_variables)]
    fn print(&mut self, str_buf: &mut StrBuf) -> Result<()> {
        unimplemented!("print")
    }

    #[doc(hidden)]
    unsafe extern "C" fn get_last_error_wrapped(game: *mut sys::game) -> *const c_char {
        (&Aux::get(game).error).into()
    }

    #[doc(hidden)]
    unsafe extern "C" fn create_wrapped(
        game: *mut sys::game,
        init_info: *mut sys::game_init,
    ) -> sys::error_code {
        // Initialize data1 to zero in case creation fails.
        let data1: *mut *mut c_void = addr_of_mut!((*game).data1);
        data1.write(null_mut());
        Aux::init(game);

        let (data, sizer) = surena_try!(game, Self::create(&GameInit::new(&*init_info)));
        check_sizer(&sizer, get_features(game));
        addr_of_mut!((*game).sizer).write(sizer);
        // data1 is already initialized.
        *data1 = Box::into_raw(Box::new(data)).cast();

        sys::ERR_ERR_OK
    }

    #[doc(hidden)]
    unsafe extern "C" fn export_options_wrapped(
        game: *mut sys::game,
        ret_size: *mut usize,
        str_buf: *mut c_char,
    ) -> sys::error_code {
        let mut ptr_vec = StrBuf::from_c_char(str_buf, ret_size, get_sizer(game).options_str);
        surena_try!(game, get_data::<Self>(game).export_options(&mut ptr_vec));
        str_buf.add(*ret_size).write(0);

        sys::ERR_ERR_OK
    }

    #[doc(hidden)]
    unsafe extern "C" fn destroy_wrapped(game: *mut sys::game) -> sys::error_code {
        let data: &mut *mut c_void = &mut *addr_of_mut!((*game).data1);
        if !data.is_null() {
            drop(Box::from_raw(data.cast::<Self>()));
            // Leave as null pointer to catch use-after-free errors.
            *data = null_mut();
        }
        Aux::free(game);

        sys::ERR_ERR_OK
    }

    #[doc(hidden)]
    unsafe extern "C" fn clone_wrapped(
        game: *mut sys::game,
        clone_target: *mut sys::game,
    ) -> sys::error_code {
        clone_target.copy_from_nonoverlapping(game, 1);

        // Initialize data1 to zero in case clone fails.
        let data1: *mut *mut c_void = addr_of_mut!((*clone_target).data1);
        data1.write(null_mut());
        Aux::init(clone_target);

        let data = get_data::<Self>(game).clone();
        // data1 is already initialized.
        *data1 = Box::into_raw(Box::new(data)).cast();

        sys::ERR_ERR_OK
    }

    #[doc(hidden)]
    unsafe extern "C" fn copy_from_wrapped(
        game: *mut sys::game,
        other: *mut sys::game,
    ) -> sys::error_code {
        let other = get_data::<Self>(other);
        surena_try!(game, get_data::<Self>(game).copy_from(other));

        sys::ERR_ERR_OK
    }

    #[doc(hidden)]
    unsafe extern "C" fn compare_wrapped(
        game: *mut sys::game,
        other: *mut sys::game,
        ret_equal: *mut bool,
    ) -> sys::error_code {
        let other = get_data::<Self>(other);
        ret_equal.write(get_data::<Self>(game).eq(&other));

        sys::ERR_ERR_OK
    }

    #[doc(hidden)]
    unsafe extern "C" fn import_state_wrapped(
        game: *mut sys::game,
        string: *const c_char,
    ) -> sys::error_code {
        let string = cstr_to_rust(string);
        surena_try!(game, get_data::<Self>(game).import_state(string));

        sys::ERR_ERR_OK
    }

    #[doc(hidden)]
    unsafe extern "C" fn export_state_wrapped(
        game: *mut sys::game,
        ret_size: *mut usize,
        str_buf: *mut c_char,
    ) -> sys::error_code {
        let mut ptr_vec = StrBuf::from_c_char(str_buf, ret_size, get_sizer(game).state_str);
        surena_try!(game, get_data::<Self>(game).export_state(&mut ptr_vec));
        str_buf.add(*ret_size).write(0);

        sys::ERR_ERR_OK
    }

    #[doc(hidden)]
    unsafe extern "C" fn players_to_move_wrapped(
        game: *mut sys::game,
        ret_count: *mut u8,
        players: *mut player_id,
    ) -> sys::error_code {
        let mut len = 0;
        let mut players = PtrVec::new(
            players,
            &mut len,
            get_sizer(game).max_players_to_move.into(),
        );
        surena_try!(game, get_data::<Self>(game).players_to_move(&mut players));
        ret_count.write(len as u8);

        sys::ERR_ERR_OK
    }

    #[doc(hidden)]
    unsafe extern "C" fn get_concrete_moves_wrapped(
        game: *mut sys::game,
        player: player_id,
        ret_count: *mut u32,
        moves: *mut move_code,
    ) -> sys::error_code {
        let mut len = 0;
        let mut moves = PtrVec::new(moves, &mut len, get_sizer(game).max_moves as usize);
        surena_try!(
            game,
            get_data::<Self>(game).get_concrete_moves(player, &mut moves)
        );
        ret_count.write(len as u32);

        sys::ERR_ERR_OK
    }

    #[doc(hidden)]
    unsafe extern "C" fn is_legal_move_wrapped(
        game: *mut sys::game,
        player: player_id,
        mov: move_code,
    ) -> sys::error_code {
        surena_try!(game, get_data::<Self>(game).is_legal_move(player, mov));

        sys::ERR_ERR_OK
    }

    #[doc(hidden)]
    unsafe extern "C" fn make_move_wrapped(
        game: *mut sys::game,
        player: player_id,
        mov: move_code,
    ) -> sys::error_code {
        surena_try!(game, get_data::<Self>(game).make_move(player, mov));

        sys::ERR_ERR_OK
    }

    #[doc(hidden)]
    unsafe extern "C" fn get_results_wrapped(
        game: *mut sys::game,
        ret_count: *mut u8,
        players: *mut player_id,
    ) -> sys::error_code {
        let mut len = 0;
        let mut players = PtrVec::new(players, &mut len, get_sizer(game).max_results.into());
        surena_try!(game, get_data::<Self>(game).get_results(&mut players));
        ret_count.write(len as u8);

        sys::ERR_ERR_OK
    }

    #[doc(hidden)]
    unsafe extern "C" fn get_move_code_wrapped(
        game: *mut sys::game,
        player: player_id,
        string: *const c_char,
        ret_move: *mut move_code,
    ) -> sys::error_code {
        let string = cstr_to_rust_unchecked(string);
        let result = surena_try!(game, get_data::<Self>(game).get_move_code(player, string));
        ret_move.write(result);

        sys::ERR_ERR_OK
    }

    #[doc(hidden)]
    unsafe extern "C" fn get_move_str_wrapped(
        game: *mut sys::game,
        player: player_id,
        mov: move_code,
        ret_size: *mut usize,
        str_buf: *mut c_char,
    ) -> sys::error_code {
        let mut ptr_vec = StrBuf::from_c_char(str_buf, ret_size, get_sizer(game).move_str);
        surena_try!(
            game,
            get_data::<Self>(game).get_move_str(player, mov, &mut ptr_vec)
        );
        str_buf.add(*ret_size).write(0);

        sys::ERR_ERR_OK
    }

    #[doc(hidden)]
    unsafe extern "C" fn print_wrapped(
        game: *mut sys::game,
        ret_size: *mut usize,
        str_buf: *mut c_char,
    ) -> sys::error_code {
        let mut ptr_vec = StrBuf::from_c_char(str_buf, ret_size, get_sizer(game).print_str);
        surena_try!(game, get_data::<Self>(game).print(&mut ptr_vec));
        str_buf.add(*ret_size).write(0);

        sys::ERR_ERR_OK
    }
}

/// Non-function members for [`game_methods`].
///
/// # Example
/// ```
/// # use mirabel::{cstr, game::*};
///
/// let mut features = game_feature_flags::default();
/// features.set_print(true);
///
/// let metadata = Metadata {
///     game_name: cstr("Example\0"),
///     variant_name: cstr("Standard\0"),
///     impl_name: cstr("mirabel_rs\0"),
///     version: semver {
///         major: 0,
///         minor: 1,
///         patch: 0,
///     },
///     features,
/// };
/// ```
pub struct Metadata {
    pub game_name: ValidCStr<'static>,
    pub variant_name: ValidCStr<'static>,
    pub impl_name: ValidCStr<'static>,
    pub version: semver,
    pub features: game_feature_flags,
}

/// Create _surena_ [`game_methods`] from game struct `G` and `metadata`.
///
/// If feature flags are disabled, corresponding function pointers will be set
/// to zero.
///
/// # Example
/// ```ignore
/// create_game_methods::<MyGame>(metadata);
/// ```
pub fn create_game_methods<G: GameMethods>(metadata: Metadata) -> game_methods {
    game_methods {
        game_name: metadata.game_name.into(),
        variant_name: metadata.variant_name.into(),
        impl_name: metadata.impl_name.into(),
        version: metadata.version,
        features: metadata.features,
        get_last_error: Some(G::get_last_error_wrapped),
        create: Some(G::create_wrapped),
        export_options: if metadata.features.options() {
            Some(G::export_options_wrapped)
        } else {
            None
        },
        destroy: Some(G::destroy_wrapped),
        clone: Some(G::clone_wrapped),
        copy_from: Some(G::copy_from_wrapped),
        compare: Some(G::compare_wrapped),
        import_state: Some(G::import_state_wrapped),
        export_state: Some(G::export_state_wrapped),
        players_to_move: Some(G::players_to_move_wrapped),
        get_concrete_moves: Some(G::get_concrete_moves_wrapped),
        is_legal_move: Some(G::is_legal_move_wrapped),
        make_move: Some(G::make_move_wrapped),
        get_results: Some(G::get_results_wrapped),
        get_move_code: Some(G::get_move_code_wrapped),
        get_move_str: Some(G::get_move_str_wrapped),
        print: if metadata.features.print() {
            Some(G::print_wrapped)
        } else {
            None
        },
        ..Default::default()
    }
}

#[derive(Default)]
struct Aux {
    error: ErrorString,
}

impl Aux {
    unsafe fn init(game: *mut sys::game) {
        // Initialize data2 to zero in case creation fails.
        let data2: *mut *mut c_void = addr_of_mut!((*game).data2);
        data2.write(null_mut());
        let aux = Box::into_raw(Box::<Self>::new(Self::default()));
        *data2 = aux.cast();
    }

    #[inline]
    unsafe fn get<'l>(game: *mut sys::game) -> &'l mut Self {
        let data2: *mut *mut c_void = addr_of_mut!((*game).data2);
        &mut *(*data2).cast::<Self>()
    }

    unsafe fn free(game: *mut sys::game) {
        let aux: &mut *mut c_void = &mut *addr_of_mut!((*game).data2);
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

#[inline]
unsafe fn get_data<'l, G>(game: *mut sys::game) -> &'l mut G {
    let data1: *mut *mut c_void = addr_of_mut!((*game).data1);
    &mut *(*data1).cast::<G>()
}

#[inline]
unsafe fn get_features(game: *mut sys::game) -> game_feature_flags {
    // The methods struct is created by create_game_methods and should be fully
    // initialized.
    (**addr_of_mut!((*game).methods)).features
}

#[inline]
unsafe fn get_sizer<'l>(game: *mut sys::game) -> &'l buf_sizer {
    &*addr_of!((*game).sizer)
}

fn check_sizer(sizer: &buf_sizer, features: game_feature_flags) {
    const FAILURE: &str = "string buffer length must not be 0";

    if features.options() {
        assert!(sizer.options_str > 0, "{FAILURE}");
    }
    assert!(sizer.state_str > 0, "{FAILURE}");
    // This can only happen on <32bit platforms:
    let _: usize = sizer
        .max_moves
        .try_into()
        .expect("max_moves does not fit usize");
    assert!(sizer.move_str > 0, "{FAILURE}");
    if features.print() {
        assert!(sizer.print_str > 0, "{FAILURE}");
    }
}

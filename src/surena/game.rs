//! This is a wrapper library for the game API of the game engine.

pub use crate::sys::{move_code, player_id, semver, MOVE_NONE, PLAYER_NONE, PLAYER_RAND};

use crate::{
    cstr_to_rust, cstr_to_rust_unchecked,
    error::{ErrorString, Result},
    game_init::GameInit,
    sys::{
        self, game_feature_flags, game_methods, move_data,
        move_data_s__bindgen_ty_1 as move_data_cl, move_data_sync,
    },
    MoveDataSync, ValidCStr, ValidCString,
};

use std::{
    ffi::c_void,
    ops::Deref,
    os::raw::c_char,
    ptr::{addr_of, addr_of_mut, null_mut},
    slice::{from_raw_parts, from_raw_parts_mut},
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
                [$($crate::game::create_game_methods::<$g>($m)),*]
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
    ( $aux:expr, $result:expr ) => {
        match $result {
            Ok(v) => v,
            Err(error) => {
                $aux.error = error.message;
                return error.code.into();
            }
        }
    };
}

/// Main trait which needs to be implemented by your game struct.
///
/// See `./mirabel/lib/surena/includes/surena/game.h` for API documentation.
///
/// Games need to implement [`Drop`] for custom `destroy` handling.
/// `clone` is handled by the [`Clone`] implementation and `compare` by [`Eq`].
/// The [`Send`] bound is required by the surena API.
///
/// # Example
/// See the `example` crate in the project root.
pub trait GameMethods: Sized + Clone + Eq + Send {
    /// Use [`MoveCode`] or [`BigMove`] here, depending on your move type.
    type Move: MoveData;

    fn create(init_info: &GameInit) -> Result<Self>;
    fn copy_from(&mut self, other: &mut Self) -> Result<()>;
    fn player_count(&mut self) -> Result<u8>;
    fn import_state(&mut self, string: Option<&str>) -> Result<()>;
    fn export_state(&mut self, player: player_id, str_buf: &mut ValidCString) -> Result<()>;
    fn players_to_move(&mut self, players: &mut Vec<player_id>) -> Result<()>;
    fn get_concrete_moves(&mut self, player: player_id, moves: &mut Vec<Self::Move>) -> Result<()>;
    fn get_move_data(
        &mut self,
        player: player_id,
        string: &str,
    ) -> Result<<<Self::Move as MoveData>::Rust as ToOwned>::Owned>;
    fn get_move_str(
        &mut self,
        player: player_id,
        mov: MoveDataSync<&<Self::Move as MoveData>::Rust>,
        str_buf: &mut ValidCString,
    ) -> Result<()>;
    fn make_move(
        &mut self,
        player: player_id,
        mov: MoveDataSync<&<Self::Move as MoveData>::Rust>,
    ) -> Result<()>;
    fn get_results(&mut self, players: &mut Vec<player_id>) -> Result<()>;
    #[allow(clippy::wrong_self_convention)]
    fn is_legal_move(
        &mut self,
        player: player_id,
        mov: MoveDataSync<&<Self::Move as MoveData>::Rust>,
    ) -> Result<()>;

    /// Must be implemented when [`GameFeatures::options`] is enabled.
    #[allow(unused_variables)]
    fn export_options(&mut self, player: player_id, str_buf: &mut ValidCString) -> Result<()> {
        unimplemented!("export_options")
    }
    /// Must be implemented when [`GameFeatures::print`] is enabled.
    #[allow(unused_variables)]
    fn print(&mut self, player: player_id, str_buf: &mut ValidCString) -> Result<()> {
        unimplemented!("print")
    }
}

unsafe extern "C" fn get_last_error_wrapped<G: GameMethods>(game: *mut sys::game) -> *const c_char {
    (&Aux::<G>::get(game).error).into()
}

unsafe extern "C" fn create_wrapped<G: GameMethods>(
    game: *mut sys::game,
    init_info: *mut sys::game_init,
) -> sys::error_code {
    // Initialize data1 to zero in case creation fails.
    let data1: *mut *mut c_void = addr_of_mut!((*game).data1);
    data1.write(null_mut());
    Aux::<G>::init(game);

    let data = surena_try!(Aux::<G>::get(game), G::create(&GameInit::new(&*init_info)));
    // data1 is already initialized.
    *data1 = Box::into_raw(Box::new(data)).cast();

    sys::ERR_ERR_OK
}

unsafe extern "C" fn export_options_wrapped<G: GameMethods>(
    game: *mut sys::game,
    player: player_id,
    ret_size: *mut usize,
    ret_str: *mut *const c_char,
) -> sys::error_code {
    let (aux, game) = get_both::<G>(game);
    let str_buf = &mut aux.str_buf;
    *str_buf = Default::default();
    surena_try!(aux, game.export_options(player, str_buf));

    ret_str.write(str_buf.as_ptr());
    ret_size.write(str_buf.as_bytes().len());
    sys::ERR_ERR_OK
}

unsafe extern "C" fn destroy_wrapped<G: GameMethods>(game: *mut sys::game) -> sys::error_code {
    let data: &mut *mut c_void = &mut *addr_of_mut!((*game).data1);
    if !data.is_null() {
        drop(Box::from_raw(data.cast::<G>()));
        // Leave as null pointer to catch use-after-free errors.
        *data = null_mut();
    }
    Aux::<G>::free(game);

    sys::ERR_ERR_OK
}

unsafe extern "C" fn clone_wrapped<G: GameMethods>(
    game: *mut sys::game,
    clone_target: *mut sys::game,
) -> sys::error_code {
    clone_target.copy_from_nonoverlapping(game, 1);

    // Initialize data1 to zero in case clone fails.
    let data1: *mut *mut c_void = addr_of_mut!((*clone_target).data1);
    data1.write(null_mut());
    Aux::<G>::init(clone_target);

    let data = get_data::<G>(game).clone();
    // data1 is already initialized.
    *data1 = Box::into_raw(Box::new(data)).cast();

    sys::ERR_ERR_OK
}

unsafe extern "C" fn copy_from_wrapped<G: GameMethods>(
    game: *mut sys::game,
    other: *mut sys::game,
) -> sys::error_code {
    let other = get_data::<G>(other);
    let (aux, game) = get_both::<G>(game);
    surena_try!(aux, game.copy_from(other));

    sys::ERR_ERR_OK
}

unsafe extern "C" fn compare_wrapped<G: GameMethods>(
    game: *mut sys::game,
    other: *mut sys::game,
    ret_equal: *mut bool,
) -> sys::error_code {
    let other = get_data::<G>(other);
    ret_equal.write(get_data::<G>(game).eq(&other));

    sys::ERR_ERR_OK
}

unsafe extern "C" fn player_count_wrapped<G: GameMethods>(
    game: *mut sys::game,
    ret_count: *mut u8,
) -> sys::error_code {
    let (aux, game) = get_both::<G>(game);
    let count = surena_try!(aux, game.player_count());

    ret_count.write(count);
    sys::ERR_ERR_OK
}

unsafe extern "C" fn import_state_wrapped<G: GameMethods>(
    game: *mut sys::game,
    string: *const c_char,
) -> sys::error_code {
    let string = cstr_to_rust(string);
    let (aux, game) = get_both::<G>(game);
    surena_try!(aux, game.import_state(string));

    sys::ERR_ERR_OK
}
unsafe extern "C" fn export_state_wrapped<G: GameMethods>(
    game: *mut sys::game,
    player: player_id,
    ret_size: *mut usize,
    ret_str: *mut *const c_char,
) -> sys::error_code {
    let (aux, game) = get_both::<G>(game);
    let str_buf = &mut aux.str_buf;
    *str_buf = Default::default();
    surena_try!(aux, game.export_state(player, str_buf));

    ret_str.write(str_buf.as_ptr());
    ret_size.write(str_buf.as_bytes().len());
    sys::ERR_ERR_OK
}

unsafe extern "C" fn players_to_move_wrapped<G: GameMethods>(
    game: *mut sys::game,
    ret_count: *mut u8,
    players: *mut *const player_id,
) -> sys::error_code {
    let (aux, game) = get_both::<G>(game);
    let player_buf = &mut aux.player_buf;
    player_buf.clear();
    surena_try!(aux, game.players_to_move(player_buf));

    players.write(player_buf.as_ptr());
    ret_count.write(
        player_buf
            .len()
            .try_into()
            .expect("player buffer too large"),
    );
    sys::ERR_ERR_OK
}

unsafe extern "C" fn get_concrete_moves_wrapped<G: GameMethods>(
    game: *mut sys::game,
    player: player_id,
    ret_count: *mut u32,
    moves: *mut *const move_data,
) -> sys::error_code {
    let (aux, game) = get_both::<G>(game);
    let move_buf = &mut aux.move_buf;
    move_buf.clear();
    surena_try!(aux, game.get_concrete_moves(player, move_buf));

    let ptr: *const G::Move = move_buf.as_ptr();
    moves.write(ptr.cast::<move_data>());
    ret_count.write(move_buf.len().try_into().expect("move buffer too long"));
    sys::ERR_ERR_OK
}

unsafe extern "C" fn is_legal_move_wrapped<G: GameMethods>(
    game: *mut sys::game,
    player: player_id,
    mov: move_data_sync,
) -> sys::error_code {
    let (aux, game) = get_both::<G>(game);
    surena_try!(aux, game.is_legal_move(player, new_sync::<G::Move>(&mov)));

    sys::ERR_ERR_OK
}

unsafe extern "C" fn make_move_wrapped<G: GameMethods>(
    game: *mut sys::game,
    player: player_id,
    mov: move_data_sync,
) -> sys::error_code {
    let (aux, game) = get_both::<G>(game);
    surena_try!(aux, game.make_move(player, new_sync::<G::Move>(&mov)));

    sys::ERR_ERR_OK
}

unsafe extern "C" fn get_results_wrapped<G: GameMethods>(
    game: *mut sys::game,
    ret_count: *mut u8,
    players: *mut *const player_id,
) -> sys::error_code {
    let (aux, game) = get_both::<G>(game);
    let player_buf = &mut aux.player_buf;
    player_buf.clear();
    surena_try!(aux, game.get_results(player_buf));

    players.write(player_buf.as_ptr());
    ret_count.write(
        player_buf
            .len()
            .try_into()
            .expect("player buffer too large"),
    );
    sys::ERR_ERR_OK
}

unsafe extern "C" fn get_move_data_wrapped<G: GameMethods>(
    game: *mut sys::game,
    player: player_id,
    string: *const c_char,
    ret_move: *mut *mut move_data_sync,
) -> sys::error_code {
    let (aux, game_data) = get_both::<G>(game);
    let string = cstr_to_rust_unchecked(string);
    let result = surena_try!(aux, game_data.get_move_data(player, string));
    aux.sync_buf = MoveDataSync {
        md: result.into(),
        sync_ctr: *addr_of!((*game).sync_ctr),
    };
    ret_move.write(&mut aux.sync_buf as *mut MoveDataSync<G::Move> as *mut move_data_sync);

    sys::ERR_ERR_OK
}

unsafe extern "C" fn get_move_str_wrapped<G: GameMethods>(
    game: *mut sys::game,
    player: player_id,
    mov: move_data_sync,
    ret_size: *mut usize,
    ret_str: *mut *const c_char,
) -> sys::error_code {
    let (aux, game) = get_both::<G>(game);
    let str_buf = &mut aux.str_buf;
    *str_buf = Default::default();
    surena_try!(
        aux,
        game.get_move_str(player, new_sync::<G::Move>(&mov), str_buf)
    );

    ret_str.write(str_buf.as_ptr());
    ret_size.write(str_buf.as_bytes().len());
    sys::ERR_ERR_OK
}

unsafe extern "C" fn print_wrapped<G: GameMethods>(
    game: *mut sys::game,
    player: player_id,
    ret_size: *mut usize,
    ret_str: *mut *const c_char,
) -> sys::error_code {
    let (aux, game) = get_both::<G>(game);
    let str_buf = &mut aux.str_buf;
    *str_buf = Default::default();
    surena_try!(aux, game.print(player, str_buf));

    ret_str.write(str_buf.as_ptr());
    ret_size.write(str_buf.as_bytes().len());
    sys::ERR_ERR_OK
}

/// Trait for wrappers of owned [`move_data`].
///
/// # Safety
/// Implementors must be a `repr(transparent)` wrapper for [`move_data`].
pub unsafe trait MoveData:
    AsRef<Self::Rust>
    + Deref<Target = move_data>
    + From<<Self::Rust as ToOwned>::Owned>
    + Default
    + 'static
{
    /// Borrowed Rust-equivalent of the wrapped [`move_data`].
    type Rust: ?Sized + ToOwned;
    /// Corresponds to [`game_feature_flags::big_moves`].
    const IS_BIG: bool;

    /// Create a new, borrowed [`Self`] by wrapping the supplied move.
    ///
    /// # Safety
    /// The mov must be valid and also represent a [`Self`].
    unsafe fn from_ref(mov: &move_data) -> &Self;
}

/// [`move_data`] which is known to represent an owned move code.
#[repr(transparent)]
pub struct MoveCode(move_data);

unsafe impl MoveData for MoveCode {
    type Rust = move_code;
    const IS_BIG: bool = false;

    #[inline]
    unsafe fn from_ref(mov: &move_data) -> &Self {
        // Normal moves must have data==NULL.
        debug_assert!(mov.data.is_null());
        &*(mov as *const move_data as *const Self)
    }
}

impl AsRef<move_code> for MoveCode {
    #[inline]
    fn as_ref(&self) -> &move_code {
        unsafe { &self.0.cl.code }
    }
}

impl Deref for MoveCode {
    type Target = move_data;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<move_code> for MoveCode {
    #[inline]
    fn from(value: move_code) -> Self {
        Self(move_data {
            cl: move_data_cl { code: value },
            data: null_mut(),
        })
    }
}

impl Default for MoveCode {
    fn default() -> Self {
        0.into()
    }
}

/// [`move_data`] which is known to represent an owned big move.
#[repr(transparent)]
pub struct BigMove(move_data);

unsafe impl MoveData for BigMove {
    type Rust = [u8];
    const IS_BIG: bool = true;

    #[inline]
    unsafe fn from_ref(mov: &move_data) -> &Self {
        // Big moves must have data!=NULL.
        debug_assert!(!mov.data.is_null());
        &*(mov as *const move_data as *const Self)
    }
}
impl AsRef<[u8]> for BigMove {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        unsafe {
            // len==0 for empty big moves.
            if self.0.cl.len == 0 {
                &[]
            } else {
                from_raw_parts(self.0.data, self.0.cl.len)
            }
        }
    }
}

impl Deref for BigMove {
    type Target = move_data;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Vec<u8>> for BigMove {
    fn from(value: Vec<u8>) -> Self {
        // Empty big moves must have data!=NULL, which is the case for slice
        // pointers.
        let slice = Box::leak(value.into_boxed_slice());
        Self(move_data {
            cl: move_data_cl { len: slice.len() },
            data: slice.as_mut_ptr(),
        })
    }
}

impl Default for BigMove {
    fn default() -> Self {
        vec![].into()
    }
}

impl Drop for BigMove {
    fn drop(&mut self) {
        let boxed: Box<[u8]> = unsafe { Box::from_raw(from_raw_parts_mut(self.data, self.cl.len)) };
        drop(boxed);
    }
}

/// Create a new, borrowed [`MoveDataSync`] from a [`move_data_sync`].
///
/// This only shallow-copies the [`sync_ctr`](move_data_sync::sync_ctr) and the
/// [`md`](move_data_sync::md).
/// It reuses the buffer of a big move.
#[inline]
fn new_sync<M: MoveData>(mov: &move_data_sync) -> MoveDataSync<&M::Rust> {
    MoveDataSync {
        md: unsafe { M::from_ref(&mov.md).as_ref() },
        sync_ctr: mov.sync_ctr,
    }
}

/// Non-function members for [`game_methods`].
///
/// # Example
/// ```
/// # use mirabel::{cstr, game::*};
///
/// let mut features = GameFeatures {
///     print: true,
///     ..Default::default()
/// };
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
    pub features: GameFeatures,
}

/// Optional game features which are supported by this wrapper.
///
/// Subset of [`game_feature_flags`].
#[derive(Default)]
pub struct GameFeatures {
    pub options: bool,
    pub print: bool,
}

impl GameFeatures {
    #[inline]
    fn feature_flags(&self) -> game_feature_flags {
        let mut flags = game_feature_flags::default();
        flags.set_options(self.options);
        flags.set_print(self.print);
        flags
    }
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
    let mut features = metadata.features.feature_flags();
    features.set_error_strings(true);
    features.set_big_moves(G::Move::IS_BIG);

    game_methods {
        game_name: metadata.game_name.into(),
        variant_name: metadata.variant_name.into(),
        impl_name: metadata.impl_name.into(),
        version: metadata.version,
        features,
        get_last_error: Some(get_last_error_wrapped::<G>),
        create: Some(create_wrapped::<G>),
        export_options: if metadata.features.options {
            Some(export_options_wrapped::<G>)
        } else {
            None
        },
        destroy: Some(destroy_wrapped::<G>),
        clone: Some(clone_wrapped::<G>),
        copy_from: Some(copy_from_wrapped::<G>),
        compare: Some(compare_wrapped::<G>),
        player_count: Some(player_count_wrapped::<G>),
        import_state: Some(import_state_wrapped::<G>),
        export_state: Some(export_state_wrapped::<G>),
        players_to_move: Some(players_to_move_wrapped::<G>),
        get_concrete_moves: Some(get_concrete_moves_wrapped::<G>),
        is_legal_move: Some(is_legal_move_wrapped::<G>),
        make_move: Some(make_move_wrapped::<G>),
        get_results: Some(get_results_wrapped::<G>),
        get_move_data: Some(get_move_data_wrapped::<G>),
        get_move_str: Some(get_move_str_wrapped::<G>),
        print: if metadata.features.print {
            Some(print_wrapped::<G>)
        } else {
            None
        },
        ..Default::default()
    }
}

struct Aux<G: GameMethods> {
    str_buf: ValidCString,
    player_buf: Vec<player_id>,
    move_buf: Vec<G::Move>,
    /// Might get modified from the outside.
    sync_buf: MoveDataSync<G::Move>,
    error: ErrorString,
}

impl<G: GameMethods> Aux<G> {
    unsafe fn init(game: *mut sys::game) {
        // Initialize data2 to zero in case creation fails.
        let data2: *mut *mut c_void = addr_of_mut!((*game).data2);
        data2.write(null_mut());
        let aux = Box::into_raw(Box::<Self>::default());
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
}

impl<G: GameMethods> Default for Aux<G> {
    fn default() -> Self {
        Self {
            str_buf: Default::default(),
            player_buf: Default::default(),
            move_buf: Default::default(),
            sync_buf: Default::default(),
            error: Default::default(),
        }
    }
}

#[inline]
unsafe fn get_data<'l, G>(game: *mut sys::game) -> &'l mut G {
    let data1: *mut *mut c_void = addr_of_mut!((*game).data1);
    &mut *(*data1).cast::<G>()
}

#[inline]
unsafe fn get_both<'l, G: GameMethods>(game: *mut sys::game) -> (&'l mut Aux<G>, &'l mut G) {
    (Aux::get(game), get_data(game))
}

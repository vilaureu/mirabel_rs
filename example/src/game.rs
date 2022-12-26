//! Example (misère) _Nim_ game for showing how to use the wrapper library.

use mirabel::{error::*, game::*, game_init::GameInit, ptr_vec::PtrVec, *};

use std::fmt::Write;

type Counter = u16;

const DEFAULT_COUNTER: Counter = 21;
const DEFAULT_MAX_SUB: Counter = 3;

/// This struct contains the game data.
///
/// It acts as the `Self` for the surena API calls.
#[derive(Copy, Clone, PartialEq, Eq)]
struct Nim {
    counter: Counter,
    max_sub: Counter,
    initial_counter: Counter,
    turn: bool,
}

impl Nim {
    fn new(counter: Counter, max_sub: Counter) -> Self {
        Self {
            counter,
            max_sub,
            initial_counter: counter,
            turn: false,
        }
    }

    fn from_options(opts: &str) -> Result<Self> {
        // eg. "21 3"
        let mut split = opts.split_whitespace();

        let counter = match split.next() {
            None => {
                // Remember to include a trailing NUL byte for static errors!
                return Err(Error::new_static(
                    ErrorCode::InvalidInput,
                    "missing starting counter\0",
                ));
            }
            Some(c) => c,
        };
        let counter = counter.parse().map_err(|e| {
            // Errors can be nicely handled using new_dynamic and format!().
            Error::new_dynamic(
                ErrorCode::InvalidInput,
                format!("counter parsing error: {e}"),
            )
        })?;

        let max_sub = match split.next() {
            None => {
                return Err(Error::new_static(
                    ErrorCode::InvalidInput,
                    "missing maximum subtrahend\0",
                ))
            }
            Some(s) => s,
        };
        let max_sub = max_sub.parse().map_err(|e| {
            Error::new_dynamic(
                ErrorCode::InvalidInput,
                format!("subtrahend parsing error: {e}"),
            )
        })?;
        if max_sub == 0 {
            return Err(Error::new_static(
                ErrorCode::InvalidOptions,
                "maximum subtrahend is zero\0",
            ));
        }

        Ok(Nim::new(counter, max_sub))
    }

    /// This calculates the [`buf_sizer`] according to the description in the
    /// `game.h`.
    ///
    /// Especially, remember that string lengths also include the NUL byte.
    fn calc_sizer(&self) -> buf_sizer {
        // eg. "A 42\0"
        let state_str = digits(self.counter) + 3;
        buf_sizer {
            options_str: digits(self.counter) + digits(self.max_sub) + 2,
            state_str,
            player_count: 2,
            max_players_to_move: 1,
            max_moves: self.max_sub.into(),
            max_results: 1,
            move_str: digits(self.max_sub) + 1,
            print_str: state_str + 1,
            ..Default::default()
        }
    }

    /// Importing the default options should reset the game state.
    fn reset(&mut self) {
        self.counter = self.initial_counter;
        self.turn = false;
    }

    fn player_id(&self) -> player_id {
        match self.turn {
            false => 1,
            true => 2,
        }
    }

    fn player_char(&self) -> char {
        match self.turn {
            false => 'A',
            true => 'B',
        }
    }
}

impl Default for Nim {
    fn default() -> Self {
        Self::new(DEFAULT_COUNTER, DEFAULT_MAX_SUB)
    }
}

impl GameMethods for Nim {
    /// Create a new instance of the game data.
    ///
    /// The game can be configured by parsing the `init_info`'s `opts` and
    /// `state`.
    /// Be careful, the options might be user input!
    ///
    /// See also [`Nim::calc_sizer`].
    fn create(init_info: &GameInit) -> Result<(Self, buf_sizer)> {
        let game = match init_info {
            GameInit::Default => Nim::default(),
            GameInit::Standard {
                opts,
                legacy,
                state,
            } => {
                if legacy.is_some() {
                    return Err(Error::new_static(
                        ErrorCode::InvalidLegacy,
                        "legacy not supported",
                    ));
                }
                let mut g = opts
                    .map(Self::from_options)
                    .transpose()?
                    .unwrap_or_default();
                g.import_state(*state)?;
                g
            }
            GameInit::Serialized(_) => {
                return Err(Error::new_static(
                    ErrorCode::FeatureUnsupported,
                    "initialization via serialized state unsupported",
                ))
            }
        };

        let sizer = game.calc_sizer();
        Ok((game, sizer))
    }

    /// Export the original game settings used to create the game.
    ///
    /// An [`StrBuf`] can be written to by simply using [`write!()`].
    /// The written length must not exceed [`buf_sizer::options_str`]` - 1`.
    fn export_options(&mut self, str_buf: &mut StrBuf) -> Result<()> {
        write!(str_buf, "{} {}", self.initial_counter, self.max_sub)
            .expect("failed to write options buffer");
        Ok(())
    }

    /// Simply copy the data from `other` to `self`.
    ///
    /// The idea is to reuse eg., allocated buffers as much as possible.
    fn copy_from(&mut self, other: &mut Self) -> Result<()> {
        *self = *other;
        Ok(())
    }

    /// Set the internal state according to the input `string`.
    ///
    /// Load default options when `string` is [`None`].
    fn import_state(&mut self, string: Option<&str>) -> Result<()> {
        let string = match string {
            None => {
                self.reset();
                return Ok(());
            }
            Some(s) => s,
        };

        let mut split = string.split_whitespace();
        let player = match split.next() {
            None => {
                self.reset();
                return Ok(());
            }
            Some(s) => s,
        };
        let counter = match split.next() {
            None => {
                return Err(Error::new_static(
                    ErrorCode::InvalidInput,
                    "missing counter value\0",
                ))
            }
            Some(c) => c,
        };

        self.turn = match player {
            "a" | "A" => false,
            "b" | "B" => true,
            _ => {
                return Err(Error::new_static(
                    ErrorCode::InvalidInput,
                    "invalid player code\0",
                ))
            }
        };
        self.counter = counter.parse().map_err(|e| {
            Error::new_dynamic(
                ErrorCode::InvalidInput,
                format!("counter parsing error: {e}"),
            )
        })?;

        Ok(())
    }

    fn export_state(&mut self, str_buf: &mut StrBuf) -> Result<()> {
        write!(str_buf, "{} {}", self.player_char(), self.counter)
            .expect("failed to write state buffer");
        Ok(())
    }

    /// A `PtrVec` is like a very simply [`Vec`] but with a fixed
    /// [`PtrVec::capacity()`].
    ///
    /// Hence, the players which are to move can be simply
    /// [`push()`](PtrVec::push())ed into `players` as long as
    /// [`buf_sizer::max_players_to_move`] is not exceeded.
    ///
    /// Alternatively, players can be assembled in another array and then
    /// copied:
    /// ```ignore
    /// let local = [1, 3, 4];
    /// players.extend_from_slice(&local);
    /// ```
    fn players_to_move(&mut self, players: &mut PtrVec<player_id>) -> Result<()> {
        if self.counter > 0 {
            players.push(self.player_id());
        }
        Ok(())
    }

    fn get_concrete_moves(
        &mut self,
        player: player_id,
        moves: &mut PtrVec<move_code>,
    ) -> Result<()> {
        if player != self.player_id() {
            return Ok(());
        }

        for mov in 1..=self.max_sub.min(self.counter) {
            moves.push(mov.into());
        }
        Ok(())
    }

    fn is_legal_move(&mut self, player: player_id, mov: move_code) -> Result<()> {
        if self.counter == 0 {
            return Err(Error::new_static(
                ErrorCode::InvalidInput,
                "game already over\0",
            ));
        }
        if mov == 0 {
            return Err(Error::new_static(
                ErrorCode::InvalidInput,
                "need to subtract at least one\0",
            ));
        }
        if player != self.player_id() {
            return Err(Error::new_static(
                ErrorCode::InvalidInput,
                "this player is not to move\0",
            ));
        }
        sub_too_large(mov as Counter, self.counter)?;
        Ok(())
    }

    fn make_move(&mut self, _player: player_id, mov: move_code) -> Result<()> {
        self.counter -= mov as Counter;
        self.turn = !self.turn;
        Ok(())
    }

    fn get_results(&mut self, players: &mut PtrVec<player_id>) -> Result<()> {
        if self.counter == 0 {
            players.push(self.player_id());
        }
        Ok(())
    }

    fn get_move_code(&mut self, _player: player_id, string: &str) -> Result<move_code> {
        let mov: Counter = string.parse().map_err(|e| {
            Error::new_dynamic(ErrorCode::InvalidInput, format!("move parsing error: {e}"))
        })?;
        sub_too_large(mov, self.max_sub)?;
        Ok(mov.into())
    }

    fn get_move_str(
        &mut self,
        _player: player_id,
        mov: move_code,
        str_buf: &mut StrBuf,
    ) -> Result<()> {
        write!(str_buf, "{}", mov).expect("failed to write move buffer");
        Ok(())
    }

    fn print(&mut self, str_buf: &mut StrBuf) -> Result<()> {
        self.export_state(str_buf)?;
        writeln!(str_buf).expect("failed to write print buffer");
        Ok(())
    }
}

/// This function creates the [`Metadata`] struct for describing _Nim_.
///
/// [`game_feature_flags`] need to be set via the `set_` functions.
/// Remember to add the trailing NUL byte to the `_name`s (see [`cstr()`]).
fn example_metadata() -> Metadata {
    let mut features = game_feature_flags::default();
    features.set_print(true);
    features.set_options(true);

    Metadata {
        game_name: cstr("Nim\0"),
        variant_name: cstr("Standard\0"),
        impl_name: cstr("mirabel_rs\0"),
        version: semver {
            major: 0,
            minor: 1,
            patch: 0,
        },
        features,
    }
}

fn sub_too_large(mov: Counter, max: Counter) -> Result<()> {
    if mov > max {
        Err(Error::new_dynamic(
            ErrorCode::InvalidInput,
            format!("can subtract at most {max}"),
        ))
    } else {
        Ok(())
    }
}

/// Calculates the number of digits needed to print `n`.
const fn digits(mut n: Counter) -> usize {
    let mut digits = 1;
    loop {
        n /= 10;
        if n == 0 {
            return digits;
        }
        digits += 1;
    }
}

// Finally, this macro creates the required plugin_get_game_methods function,
// which exports all provided game_methods structs to surena.
plugin_get_game_methods!(Nim{example_metadata()});

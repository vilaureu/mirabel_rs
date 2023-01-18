//! Example (misère) _Nim_ game for showing how to use the wrapper library.

use mirabel::{error::*, game::*, game_init::GameInit, *};

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
    /// We need to specify whether we want to use move codes or big moves.
    type Move = MoveCode;

    /// Create a new instance of the game data.
    ///
    /// The game can be configured by parsing the `init_info`'s `opts` and
    /// `state`.
    /// Be careful, the options might be user input!
    fn create(init_info: &GameInit) -> Result<Self> {
        Ok(match init_info {
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
        })
    }

    /// Export the original game settings used to create the game.
    ///
    /// A [`ValidCString`] can be written to by simply using [`write!()`].
    fn export_options(&mut self, _player: player_id, str_buf: &mut ValidCString) -> Result<()> {
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

    fn player_count(&mut self) -> Result<u8> {
        Ok(2)
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

    fn export_state(&mut self, _player: player_id, str_buf: &mut ValidCString) -> Result<()> {
        write!(str_buf, "{} {}", self.player_char(), self.counter)
            .expect("failed to write state buffer");
        Ok(())
    }

    /// The players which are to move can be simply [`push()`](Vec::push())ed
    /// into `players` as long as [`u8::MAX`] is not exceeded.
    ///
    /// Alternatively, players can be assembled in another array and then
    /// copied:
    /// ```ignore
    /// let local = [1, 3, 4];
    /// players.extend_from_slice(&local);
    /// ```
    fn players_to_move(&mut self, players: &mut Vec<player_id>) -> Result<()> {
        if self.counter > 0 {
            players.push(self.player_id());
        }
        Ok(())
    }

    /// The available moves can be simply [`push()`](Vec::push())ed into
    /// `moves`.
    /// The type of `moves` depends on [`Self::Move`].
    fn get_concrete_moves(&mut self, player: player_id, moves: &mut Vec<MoveCode>) -> Result<()> {
        if player != self.player_id() {
            return Ok(());
        }

        for mov in 1..=self.max_sub.min(self.counter) {
            moves.push(move_code::from(mov).into());
        }
        Ok(())
    }

    /// The type of `mov` depends on [`Self::Move`].
    fn is_legal_move(&mut self, player: player_id, mov: MoveDataSync<&u64>) -> Result<()> {
        if self.counter == 0 {
            return Err(Error::new_static(
                ErrorCode::InvalidInput,
                "game already over\0",
            ));
        }
        if *mov.md == 0 {
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
        sub_too_large(*mov.md as Counter, self.counter)?;
        Ok(())
    }

    fn make_move(&mut self, _player: player_id, mov: MoveDataSync<&u64>) -> Result<()> {
        self.counter -= *mov.md as Counter;
        self.turn = !self.turn;
        Ok(())
    }

    fn get_results(&mut self, players: &mut Vec<player_id>) -> Result<()> {
        if self.counter == 0 {
            players.push(self.player_id());
        }
        Ok(())
    }

    fn get_move_data(&mut self, _player: player_id, string: &str) -> Result<u64> {
        let mov: Counter = string.parse().map_err(|e| {
            Error::new_dynamic(ErrorCode::InvalidInput, format!("move parsing error: {e}"))
        })?;
        sub_too_large(mov, self.max_sub)?;
        Ok(mov.into())
    }

    fn get_move_str(
        &mut self,
        _player: player_id,
        mov: MoveDataSync<&u64>,
        str_buf: &mut ValidCString,
    ) -> Result<()> {
        write!(str_buf, "{}", mov.md).expect("failed to write move buffer");
        Ok(())
    }

    fn print(&mut self, player: player_id, str_buf: &mut ValidCString) -> Result<()> {
        self.export_state(player, str_buf)?;
        writeln!(str_buf).expect("failed to write print buffer");
        Ok(())
    }
}

/// This function creates the [`Metadata`] struct for describing _Nim_.
///
/// Remember to add the trailing NUL byte to the `_name`s (see [`cstr()`]).
fn example_metadata() -> Metadata {
    Metadata {
        game_name: cstr("Nim\0"),
        variant_name: cstr("Standard\0"),
        impl_name: cstr("mirabel_rs\0"),
        version: semver {
            major: 0,
            minor: 1,
            patch: 0,
        },
        features: GameFeatures {
            options: true,
            print: true,
        },
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

// Finally, this macro creates the required plugin_get_game_methods function,
// which exports all provided game_methods structs to surena.
plugin_get_game_methods!(Nim{example_metadata()});

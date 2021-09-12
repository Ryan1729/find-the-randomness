pub trait ClearableStorage<A> {
    fn clear(&mut self);

    fn push(&mut self, a: A);
}

/// This type is meant to only contain values in the range [0, 1]
pub type Proportion = f32;

pub type Seed = [u8; 16];

type Xs = [core::num::Wrapping<u32>; 4];

fn xorshift(xs: &mut Xs) -> u32 {
    let mut t = xs[3];

    xs[3] = xs[2];
    xs[2] = xs[1];
    xs[1] = xs[0];

    t ^= t << 11;
    t ^= t >> 8;
    xs[0] = t ^ xs[0] ^ (xs[0] >> 19);

    xs[0].0
}

#[allow(unused)]
fn xs_u32(xs: &mut Xs, min: u32, one_past_max: u32) -> u32 {
    (xorshift(xs) % (one_past_max - min)) + min
}

#[allow(unused)]
fn xs_shuffle<A>(rng: &mut Xs, slice: &mut [A]) {
    for i in 1..slice.len() as u32 {
        // This only shuffles the first u32::MAX_VALUE - 1 elements.
        let r = xs_u32(rng, 0, i + 1) as usize;
        let i = i as usize;
        slice.swap(i, r);
    }
}

#[allow(unused)]
fn new_seed(rng: &mut Xs) -> Seed {
    let s0 = xorshift(rng).to_le_bytes();
    let s1 = xorshift(rng).to_le_bytes();
    let s2 = xorshift(rng).to_le_bytes();
    let s3 = xorshift(rng).to_le_bytes();

    [
        s0[0], s0[1], s0[2], s0[3],
        s1[0], s1[1], s1[2], s1[3],
        s2[0], s2[1], s2[2], s2[3],
        s3[0], s3[1], s3[2], s3[3],
    ]
}

fn xs_from_seed(mut seed: Seed) -> Xs {
    // 0 doesn't work as a seed, so use this one instead.
    if seed == [0; 16] {
        seed = 0xBAD_5EED_u128.to_le_bytes();
    }

    macro_rules! wrap {
        ($i0: literal, $i1: literal, $i2: literal, $i3: literal) => {
            core::num::Wrapping(
                u32::from_le_bytes([
                    seed[$i0],
                    seed[$i1],
                    seed[$i2],
                    seed[$i3],
                ])
            )
        }
    }

    [
        wrap!( 0,  1,  2,  3),
        wrap!( 4,  5,  6,  7),
        wrap!( 8,  9, 10, 11),
        wrap!(12, 13, 14, 15),
    ]
}

mod checked {
    pub trait AddOne: Sized {
        fn checked_add_one(&self) -> Option<Self>;
    }

    pub trait SubOne: Sized {
        fn checked_sub_one(&self) -> Option<Self>;
    }
}
use checked::{AddOne, SubOne};

pub mod tile {
    use crate::{Xs, xs_u32, Proportion, AddOne, SubOne};

    pub type Count = u8;
    
    macro_rules! coord_def {
        (
            ($zero_variant: ident => $zero_number: literal),
            $( ($wrap_variants: ident => $wrap_number: literal) ),+ $(,)?
        ) => {
            #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
            #[repr(u8)]
            pub enum Coord {
                $zero_variant,
                $($wrap_variants,)+
            }

            impl core::convert::TryFrom<u8> for Coord {
                type Error = ();

                fn try_from(byte: u8) -> Result<Self, Self::Error> {
                    Self::const_try_from(byte)
                }
            }

            impl Coord {
                fn proportion(&self) -> Proportion {
                    (u8::from(*self) as Proportion) / (Self::COUNT as Proportion)
                }

                pub const COUNT: Count = {
                    let mut count = 0;
                    
                    count += 1; // $zero_number
                    $(
                        // Some reference to the vars is needed to use 
                        // the repetitions.
                        let _ = $wrap_number;

                        count += 1;
                    )+

                    count
                };

                pub const ALL: [Coord; Self::COUNT as usize] = [
                    Coord::$zero_variant,
                    $(Coord::$wrap_variants,)+
                ];

                const fn const_try_from(byte: u8) -> Result<Self, ()> {
                    match byte {
                        $zero_number => Ok(Coord::$zero_variant),
                        $($wrap_number => Ok(Coord::$wrap_variants),)+
                        Self::COUNT..=u8::MAX => Err(()),
                    }
                }

                const ZERO: Coord = Coord::ALL[0];

                // Currently there are an even amount of Coords, so there is no true center.
                const CENTER_INDEX: usize = Coord::ALL.len() / 2;
        
                const CENTER: Coord = Coord::ALL[Self::CENTER_INDEX];

                const MAX: Coord = Coord::ALL[Coord::ALL.len() - 1];
                pub const MAX_INDEX: Count = Self::COUNT - 1;

                #[allow(unused)] // desired in tests
                pub fn from_rng(rng: &mut Xs) -> Self {
                    Self::ALL[xs_u32(rng, 0, Self::ALL.len() as u32) as usize]
                }
            }

            impl AddOne for Coord {
                fn checked_add_one(&self) -> Option<Self> {
                    self.const_checked_add_one()
                }
            }

            impl Coord {
                const fn const_checked_add_one(&self) -> Option<Self> {
                    match (*self as u8).checked_add(1) {
                        Some(byte) => match Self::const_try_from(byte) {
                            Ok(x) => Some(x),
                            Err(_) => None,
                        },
                        None => None,
                    }
                }
            }

            impl SubOne for Coord {
                fn checked_sub_one(&self) -> Option<Self> {
                    use core::convert::TryInto;
                    (*self as u8).checked_sub(1)
                        .and_then(|byte| byte.try_into().ok())
                }
            }

            impl Default for Coord {
                fn default() -> Self {
                    Self::$zero_variant
                }
            }

            impl From<Coord> for u8 {
                fn from(coord: Coord) -> u8 {
                    coord.const_to_count()
                }
            }

            impl Coord {
                const fn const_to_count(self) -> Count {
                    match self {
                        Coord::$zero_variant => $zero_number,
                        $(Coord::$wrap_variants => $wrap_number,)+
                    }
                }
            }

            impl From<Coord> for usize {
                fn from(coord: Coord) -> Self {
                    Self::from(u8::from(coord))
                }
            }
        }
    }

    coord_def!{
        (C0 => 0),
        (C1 => 1),
        (C2 => 2),
        (C3 => 3),
        (C4 => 4),
        (C5 => 5),
        (C6 => 6),
        (C7 => 7),
    }

    macro_rules! tuple_new_type {
        ($struct_name: ident) => {
            #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
            pub struct $struct_name(Coord);

            impl AddOne for $struct_name {
                fn checked_add_one(&self) -> Option<Self> {
                    self.0.checked_add_one().map($struct_name)
                }
            }
        
            impl SubOne for $struct_name {
                fn checked_sub_one(&self) -> Option<Self> {
                    self.0.checked_sub_one().map($struct_name)
                }
            }

            impl $struct_name {
                pub fn proportion(&self) -> Proportion {
                    self.0.proportion()
                }

                #[allow(unused)]
                pub(crate) const ZERO: $struct_name = $struct_name(Coord::ZERO);
                #[allow(unused)]
                pub(crate) const CENTER: $struct_name = $struct_name(Coord::CENTER);
                #[allow(unused)]
                pub(crate) const MAX: $struct_name = $struct_name(Coord::MAX);
                
                pub(crate) const COUNT: Count = Coord::COUNT;

                #[allow(unused)]
                pub (crate) const ALL: [$struct_name; Self::COUNT as usize] = {
                    let mut all = [$struct_name(Coord::ZERO); Self::COUNT as usize];

                    let mut coord = Coord::ALL[0];
                    while let Some(c) = coord.const_checked_add_one() {
                        all[Coord::const_to_count(c) as usize] = $struct_name(c);

                        coord = c;
                    }

                    all
                };

                #[allow(unused)] // desired in tests
                pub fn from_rng(rng: &mut Xs) -> Self {
                    $struct_name(Coord::from_rng(rng))
                }
            }

            impl From<$struct_name> for usize {
                fn from(thing: $struct_name) -> Self {
                    Self::from(thing.0)
                }
            }
        }
    }

    tuple_new_type!{X}
    tuple_new_type!{Y}

    #[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
    pub struct XY {
        pub x: X,
        pub y: Y,
    }

    impl XY {
        // It would probably be possible to make this a constant with more coord_def
        // macro-trickery, but I'm not sure whether there would be a benefit to 
        // doing so, given that then two `Coord::COUNT * Coord::COUNT` arrays would
        // need to be in the cache at the same time.
        pub fn all() -> impl Iterator<Item = XY> {
            Coord::ALL.iter()
                .flat_map(|&yc|
                    Coord::ALL.iter()
                        .map(move |&xc| (Y(yc), X(xc)))
                )
                .map(|(y, x)| Self {
                    x,
                    y,
                })
        }
    }

    #[derive(Clone, Copy, Debug)]
    pub enum State {
        Unlit,
        Lit,
    }

    impl Default for State {
        fn default() -> Self {
            Self::DEFAULT
        }
    }

    impl State {
        const DEFAULT: Self = Self::Unlit;

        pub fn from_rng(rng: &mut Xs) -> Self {
            use State::*;
            match xs_u32(rng, 0, 2) {
                0 => Unlit,
                _ => Lit,
            }
        }
    }

    pub fn xy_to_i(xy: XY) -> usize {
        xy_to_i_usize((usize::from(xy.x.0), usize::from(xy.y.0)))
    }

    pub fn xy_to_i_usize((x, y): (usize, usize)) -> usize {
        y * Coord::COUNT as usize + x
    }
}

pub const TILES_LENGTH: usize = tile::Coord::COUNT as usize * tile::Coord::COUNT as usize;

#[derive(Clone, Copy, Debug, Default)]
struct Tile {
    state: tile::State,
}

impl Tile {
    pub fn from_rng(rng: &mut Xs) -> Tile {
        Self {
            state: tile::State::from_rng(rng),
        }
    }
}

type Tiles = [Tile; TILES_LENGTH];

fn tiles_from_rng(rng: &mut Xs) -> Tiles {
    let mut output = [Tile::default(); TILES_LENGTH];

    for i in 0..TILES_LENGTH {
        output[i] = Tile::from_rng(rng);
    }

    output
}

type UiPos = tile::XY;

#[derive(Debug)]
struct Board {
    tiles: Tiles,
    rng: Xs,
    ui_pos: UiPos
}

impl Default for Board {
    fn default() -> Self {
        Self {
            tiles: [Tile::default(); TILES_LENGTH],
            rng: <_>::default(),
            ui_pos: <_>::default(),
        }
    }
}

impl Board {
    pub fn from_seed(seed: Seed) -> Self {
        let mut rng = xs_from_seed(seed);

        let tiles = tiles_from_rng(&mut rng);

        Self {
            tiles,
            rng,
            ..<_>::default()
        }
    }
}

pub mod draw;

pub use draw::{
    DrawLength,
    DrawX,
    DrawY, 
    DrawXY,
    DrawW,
    DrawH,
    DrawWH,
    TileState
};

#[derive(Debug, Default)]
pub struct State {
    sizes: draw::Sizes,
    board: Board,
}

impl State {
    pub fn from_seed(seed: Seed) -> Self {
        Self {
            board: Board::from_seed(seed),
            ..<_>::default()
        }
    }
}

pub fn sizes(state: &State) -> draw::Sizes {
    state.sizes.clone()
}

pub type InputFlags = u16;

pub const INPUT_UP_PRESSED: InputFlags              = 0b0000_0000_0000_0001;
pub const INPUT_DOWN_PRESSED: InputFlags            = 0b0000_0000_0000_0010;
pub const INPUT_LEFT_PRESSED: InputFlags            = 0b0000_0000_0000_0100;
pub const INPUT_RIGHT_PRESSED: InputFlags           = 0b0000_0000_0000_1000;

pub const INPUT_UP_DOWN: InputFlags                 = 0b0000_0000_0001_0000;
pub const INPUT_DOWN_DOWN: InputFlags               = 0b0000_0000_0010_0000;
pub const INPUT_LEFT_DOWN: InputFlags               = 0b0000_0000_0100_0000;
pub const INPUT_RIGHT_DOWN: InputFlags              = 0b0000_0000_1000_0000;

pub const INPUT_INTERACT_PRESSED: InputFlags        = 0b0000_0001_0000_0000;

enum Input {
    NoChange,
    Up,
    Down,
    Left,
    Right,
    Interact,
}

impl Input {
    fn from_flags(flags: InputFlags) -> Self {
        use Input::*;

        if INPUT_INTERACT_PRESSED & flags != 0 {
            Interact
        } else if INPUT_UP_PRESSED & flags != 0 {
            Up
        } else if INPUT_DOWN_PRESSED & flags != 0 {
            Down
        } else if INPUT_LEFT_PRESSED & flags != 0 {
            Left
        } else if INPUT_RIGHT_PRESSED & flags != 0 {
            Right
        } else {
            NoChange
        }
    }
}

pub fn update(
    state: &mut State,
    commands: &mut dyn ClearableStorage<draw::Command>,
    input_flags: InputFlags,
    draw_wh: DrawWH,
) {
    use Input::*;
    let input = Input::from_flags(input_flags);

    match (input, &mut state.board.ui_pos) {
        (NoChange, _) => {},
        (Up, xy) => {
            if let Some(new_y) = xy.y.checked_sub_one() {
                xy.y = new_y;
            }
        },
        (Down, xy) => {
            if let Some(new_y) = xy.y.checked_add_one() {
                xy.y = new_y;
            }
        },
        (Left, xy) => {
            if let Some(new_x) = xy.x.checked_sub_one() {
                xy.x = new_x;
            }
        },
        (Right, xy) => {
            if let Some(new_x) = xy.x.checked_add_one() {
                xy.x = new_x;
            }
        },
        (Interact, _xy) => {
            
        },
    }

    if draw_wh != state.sizes.draw_wh {
        state.sizes = draw::fresh_sizes(draw_wh);
    }

    commands.clear();

    for xy in tile::XY::all() {
        let tile = state.board.tiles[tile::xy_to_i(xy)];

        commands.push(
            draw::Command::Tile(
                draw::TileSpec {
                    xy: draw::tile_xy_to_draw(&state.sizes, xy),
                    state: tile.state,
                }
            )
        );
    }

    commands.push(
        draw::Command::Selectrum(
            draw::tile_xy_to_draw(&state.sizes, state.board.ui_pos)
        )
    );
}
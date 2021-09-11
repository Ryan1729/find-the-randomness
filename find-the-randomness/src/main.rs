//#![deny(unused)]

extern crate alloc;
use alloc::vec::Vec;

struct Storage<A>(Vec<A>);

impl <A> game::ClearableStorage<A> for Storage<A> {
    fn clear(&mut self) {
        self.0.clear();
    }

    fn push(&mut self, a: A) {
        self.0.push(a);
    }
}

use raylib::prelude::{
    *,
    KeyboardKey::*,
    core::{
        logging
    }
};

fn draw_wh(rl: &RaylibHandle) -> game::DrawWH {
    game::DrawWH {
        w: rl.get_screen_width() as game::DrawW,
        h: rl.get_screen_height() as game::DrawH,
    }
}

fn main() {
    let (mut rl, thread) = raylib::init()
    .size(0, 0)
    .resizable()
    .title("Find the Randomness")
    .build();

    if cfg!(debug_assertions) {
        logging::set_trace_log_exit(TraceLogType::LOG_WARNING);
    }

    rl.set_target_fps(60);
    rl.toggle_fullscreen();

    let seed: u128 = {
        use std::time::SystemTime;

        let duration = match 
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
        {
            Ok(d) => d,
            Err(err) => err.duration(),
        };

        duration.as_nanos()
    };
    println!("{}", seed);

    let mut state = game::State::from_seed(seed.to_le_bytes());
    let mut commands = Storage(Vec::with_capacity(1024));

    // generate the commands for the first frame
    game::update(&mut state, &mut commands, 0, draw_wh(&rl));

    const BACKGROUND: Color = Color{ r: 0x22, g: 0x22, b: 0x22, a: 255 };
    const BLACK: Color = Color{ r: 0, g: 0, b: 0, a: 255 };
    const WHITE: Color = Color{ r: 0xee, g: 0xee, b: 0xee, a: 255 };

    while !rl.window_should_close() {
        if rl.is_key_pressed(KEY_F11) {
            rl.toggle_fullscreen();
        }

        let mut input_flags = 0;

        if rl.is_key_pressed(KEY_SPACE) || rl.is_key_pressed(KEY_ENTER) {
            input_flags |= game::INPUT_INTERACT_PRESSED;
        }

        if rl.is_key_down(KEY_UP) || rl.is_key_down(KEY_W) {
            input_flags |= game::INPUT_UP_DOWN;
        }

        if rl.is_key_down(KEY_DOWN) || rl.is_key_down(KEY_S) {
            input_flags |= game::INPUT_DOWN_DOWN;
        }

        if rl.is_key_down(KEY_LEFT) || rl.is_key_down(KEY_A) {
            input_flags |= game::INPUT_LEFT_DOWN;
        }

        if rl.is_key_down(KEY_RIGHT) || rl.is_key_down(KEY_D) {
            input_flags |= game::INPUT_RIGHT_DOWN;
        }

        if rl.is_key_pressed(KEY_UP) || rl.is_key_pressed(KEY_W) {
            input_flags |= game::INPUT_UP_PRESSED;
        }
        
        if rl.is_key_pressed(KEY_DOWN) || rl.is_key_pressed(KEY_S) {
            input_flags |= game::INPUT_DOWN_PRESSED;
        }

        if rl.is_key_pressed(KEY_LEFT) || rl.is_key_pressed(KEY_A) {
            input_flags |= game::INPUT_LEFT_PRESSED;
        }

        if rl.is_key_pressed(KEY_RIGHT) || rl.is_key_pressed(KEY_D) {
            input_flags |= game::INPUT_RIGHT_PRESSED;
        }

        game::update(
            &mut state,
            &mut commands,
            input_flags,
            draw_wh(&rl)
        );

        let screen_render_rect = Rectangle {
            x: 0.,
            y: 0.,
            width: rl.get_screen_width() as _,
            height: rl.get_screen_height() as _
        };

        let sizes = game::sizes(&state);

        let mut d = rl.begin_drawing(&thread);

        d.clear_background(BACKGROUND);

        // the -1 and +2 business makes the border lie just outside the actual
        // play area
        d.draw_rectangle_lines(
            sizes.play_xywh.x as i32 - 1,
            sizes.play_xywh.y as i32 - 1,
            sizes.play_xywh.w as i32 + 2,
            sizes.play_xywh.h as i32 + 2,
            WHITE
        );

        let tile_base_render_rect = Rectangle {
            x: 0.,
            y: 0.,
            width: sizes.tile_side_length,
            height: sizes.tile_side_length,
        };

        for cmd in commands.0.iter() {
            use game::draw::Command::*;
            match cmd {
                Tile(tile_spec) => {
                    let origin = Vector2 {
                        x: (tile_base_render_rect.width / 2.).round(),
                        y: (tile_base_render_rect.height / 2.).round(),
                    };

                    let tile_rect = Rectangle {
                        x: tile_spec.xy.x + origin.x,
                        y: tile_spec.xy.y + origin.y,
                        ..tile_base_render_rect
                    };

                    d.draw_rectangle(
                        tile_rect.x as i32,
                        tile_rect.y as i32,
                        tile_rect.width as i32,
                        tile_rect.height as i32,
                        match tile_spec.state {
                            game::TileState::Unlit => BLACK,
                            game::TileState::Lit => WHITE,
                        }
                    );
                }
            }
        }
    }
}
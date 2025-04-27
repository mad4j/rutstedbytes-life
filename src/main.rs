/*!
 * RustedBytes Game of Life
 * A simple implementation of Conway's Game of Life using OpenCL for parallel computation.
 * This program creates a window where you can interact with the simulation using the mouse and keyboard.
 * You can click to toggle cells, use the spacebar to reset the grid, and adjust the frames per second (FPS) using the up and down arrow keys.
 * The simulation runs in a loop, updating the grid based on the rules of the Game of Life.
 * The OpenCL kernel is used to compute the next generation of cells based on their neighbors.
 * The program uses the minifb crate for window management and rendering, and the ocl crate for OpenCL bindings.
 * 
 * References:
 * - Game of Life rules: https://en.wikipedia.org/wiki/Conway%27s_Game_of_Life
 * - Palette generator: https://coolors.co/
 * - Rust OCL bindings: https://crates.io/crates/ocl
 */


// no shell window
#![windows_subsystem = "windows"]

use clap::Parser;
use std::str::FromStr;

use minifb::{Icon, Key, MouseButton, Scale, Window, WindowOptions};
use ocl::ProQue;
use rand::Rng;

const WIDTH: usize = 480;
const HEIGHT: usize = 360;
const ALIVE_COLOR: u32 = 0x6A66A3; // Foreground
const DEAD_COLOR: u32 = 0xDDD8B8; // Background

#[cfg(target_os = "windows")]
const ICO_FILE: &[u8] = include_bytes!("../resources/app.ico");

/// Command-line arguments for the Game of Life application
#[derive(Parser, Debug)]
#[command(
    name = env!("CARGO_PKG_NAME"),
    about = env!("CARGO_PKG_DESCRIPTION"),
    long_about = env!("CARGO_PKG_DESCRIPTION"),
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS")
)]
struct Args {
    /// Initial frames per second (FPS)
    #[arg(short, long, default_value_t = 60)]
    fps: usize,
}

fn get_title(fps: usize) -> String {
    format!("RustedBytes Game of Life ({}fps)", fps)
}

fn main() {
    // Parse command-line arguments
    let args = Args::parse();
    let mut current_fps = args.fps; // Use the FPS value from the command-line arguments

    // OpenCL kernel source code
    // This kernel implements the Game of Life rules
    // Each cell is represented as a byte (0 for dead, 1 for alive)
    // The kernel processes the grid in parallel
    // Each thread computes the next state of a cell based on its neighbors
    // The grid is wrapped around (toroidal array)
    let kernel_source = r#"
        __kernel void game_of_life(__global uchar* grid, __global uchar* new_grid, int width, int height) {
            int x = get_global_id(0);
            int y = get_global_id(1);
            int idx = y * width + x;

            int alive_neighbors = 0;
            for (int dy = -1; dy <= 1; dy++) {
                for (int dx = -1; dx <= 1; dx++) {
                    if (dx == 0 && dy == 0) continue;
                    int nx = (x + dx + width) % width;
                    int ny = (y + dy + height) % height;
                    int n_idx = ny * width + nx;
                    alive_neighbors += grid[n_idx];
                }
            }

            if (grid[idx] == 1) {
                new_grid[idx] = (alive_neighbors == 2 || alive_neighbors == 3) ? 1 : 0;
            } else {
                new_grid[idx] = (alive_neighbors == 3) ? 1 : 0;
            }
        }
    "#;

    // Initialize OpenCL
    let pro_que = ProQue::builder()
        .src(kernel_source)
        .dims((WIDTH, HEIGHT))
        .build()
        .unwrap();

    let mut grid: Vec<u8> = (0..WIDTH * HEIGHT)
        .map(|_| if rand::rng().random_bool(0.2) { 1 } else { 0 })
        .collect();
    let buffer_grid = pro_que.create_buffer::<u8>().unwrap();
    let buffer_new_grid = pro_que.create_buffer::<u8>().unwrap();

    buffer_grid.write(&grid).enq().unwrap();

    let kernel = pro_que
        .kernel_builder("game_of_life")
        .arg(&buffer_grid)
        .arg(&buffer_new_grid)
        .arg(WIDTH as i32)
        .arg(HEIGHT as i32)
        .build()
        .unwrap();

    let mut window = Window::new(
        &get_title(current_fps),
        WIDTH,
        HEIGHT,
        WindowOptions {
            scale: Scale::X2,
            ..WindowOptions::default()
        },
    )
    .unwrap();

    window.set_target_fps(current_fps);

    #[cfg(target_os = "windows")]
    {
        let temp_file = temp_file::with_contents(ICO_FILE);
        window.set_icon(Icon::from_str(temp_file.path().to_str().unwrap()).unwrap());
    }

    let mut frame_buffer = vec![0u32; WIDTH * HEIGHT];

    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Check for space key to reset the grid
        if window.is_key_down(Key::Space) || window.get_mouse_down(MouseButton::Right) {
            grid = (0..WIDTH * HEIGHT)
                .map(|_| if rand::rng().random_bool(0.2) { 1 } else { 0 })
                .collect();
            buffer_grid.write(&grid).enq().unwrap();
        }

        // Adjust frame rate with 'Up' and 'Down' keys
        if window.is_key_down(Key::Up) {
            current_fps = (current_fps + 5).min(250);
            window.set_target_fps(current_fps);
            window.set_title(&get_title(current_fps));
        }
        if window.is_key_down(Key::Down) {
            current_fps = (current_fps - 5).max(5);
            window.set_target_fps(current_fps);
            window.set_title(&get_title(current_fps));
        }

        // Check for mouse click to set cells to alive
        if let Some((mouse_x, mouse_y)) = window.get_mouse_pos(minifb::MouseMode::Clamp) {
            if window.get_mouse_down(MouseButton::Left) {
                let x = mouse_x as usize;
                let y = mouse_y as usize;
                if x < WIDTH && y < HEIGHT {
                    buffer_grid.read(&mut grid).enq().unwrap();
                    grid[y * WIDTH + x] = 1;
                    buffer_grid.write(&grid).enq().unwrap();
                }
            }
        }

        // Execute kernel
        unsafe {
            kernel.enq().unwrap();
        }

        // Read back the new grid
        let mut new_grid = vec![0u8; WIDTH * HEIGHT];
        buffer_new_grid.read(&mut new_grid).enq().unwrap();

        // Update the frame buffer with the new grid
        // Convert the grid to colors for rendering
        for (i, &cell) in new_grid.iter().enumerate() {
            frame_buffer[i] = if cell == 1 { ALIVE_COLOR } else { DEAD_COLOR };
        }

        // update the buffer with the new grid
        window
            .update_with_buffer(&frame_buffer, WIDTH, HEIGHT)
            .unwrap();

        // Display the new grid in the window
        buffer_grid.write(&new_grid).enq().unwrap();
    }
}

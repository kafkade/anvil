//! Animated version banner for Anvil CLI
//!
//! Displays the forge lettermark logo with a molten gradient animation
//! when `anvil --version` is invoked.

use crossterm::{
    cursor,
    style::{self, Attribute, Color, SetAttribute, SetForegroundColor},
    ExecutableCommand,
};
use std::io::{self, Write};
use std::{thread, time::Duration};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const LOGO: &[&str] = &[
    "          ██          ",
    "         ████         ",
    "        ██  ██        ",
    "       ██    ██       ",
    "      ██ ████ ██      ",
    "     ██  ████  ██     ",
    "    ██   ████   ██    ",
    "   ████████████████   ",
];

/// Gradient from white-hot (top) to deep red (bottom)
fn gradient_color(line_index: usize) -> Color {
    match line_index {
        0 => Color::Rgb { r: 254, g: 243, b: 199 },
        1 => Color::Rgb { r: 253, g: 224, b: 71 },
        2 => Color::Rgb { r: 251, g: 191, b: 36 },
        3 => Color::Rgb { r: 245, g: 158, b: 11 },
        4 => Color::Rgb { r: 249, g: 115, b: 22 },
        5 => Color::Rgb { r: 239, g: 68, b: 68 },
        6 => Color::Rgb { r: 220, g: 38, b: 38 },
        7 => Color::Rgb { r: 185, g: 28, b: 28 },
        _ => Color::White,
    }
}

const CROSSBAR_COLOR: Color = Color::Rgb { r: 127, g: 29, b: 29 };

/// Print a single logo line with gradient color and crossbar highlighting
fn print_logo_line(stdout: &mut io::Stdout, index: usize) -> io::Result<()> {
    let outer = gradient_color(index);

    match index {
        4 => {
            stdout.execute(SetForegroundColor(outer))?;
            print!("      ██ ");
            stdout.execute(SetForegroundColor(CROSSBAR_COLOR))?;
            print!("████");
            stdout.execute(SetForegroundColor(outer))?;
            println!(" ██      ");
        }
        5 => {
            stdout.execute(SetForegroundColor(outer))?;
            print!("     ██  ");
            stdout.execute(SetForegroundColor(CROSSBAR_COLOR))?;
            print!("████");
            stdout.execute(SetForegroundColor(outer))?;
            println!("  ██     ");
        }
        6 => {
            stdout.execute(SetForegroundColor(outer))?;
            print!("    ██   ");
            stdout.execute(SetForegroundColor(CROSSBAR_COLOR))?;
            print!("████");
            stdout.execute(SetForegroundColor(outer))?;
            println!("   ██    ");
        }
        _ => {
            stdout.execute(SetForegroundColor(outer))?;
            println!("{}", LOGO[index]);
        }
    }
    stdout.flush()
}

/// Show the animated version banner. Falls back to plain text if the terminal
/// doesn't support the animation (e.g., piped output).
pub fn show_version() {
    if atty::is(atty::Stream::Stdout) {
        if animate_version().is_err() {
            print_plain_version();
        }
    } else {
        print_plain_version();
    }
}

fn print_plain_version() {
    println!("anvil {VERSION}");
}

fn animate_version() -> io::Result<()> {
    let mut stdout = io::stdout();
    let frame_delay = Duration::from_millis(55);

    // Hide cursor during animation
    stdout.execute(cursor::Hide)?;

    // Reveal logo line by line
    for i in 0..LOGO.len() {
        print_logo_line(&mut stdout, i)?;
        thread::sleep(frame_delay);
    }

    // Brief pause, then version info
    thread::sleep(Duration::from_millis(80));

    println!();
    stdout.execute(SetForegroundColor(Color::Rgb { r: 251, g: 191, b: 36 }))?;
    stdout.execute(SetAttribute(Attribute::Bold))?;
    print!("   anvil ");
    stdout.execute(SetAttribute(Attribute::Reset))?;
    stdout.execute(SetForegroundColor(Color::Rgb { r: 148, g: 163, b: 184 }))?;
    println!("v{VERSION}");

    // Reset colors and show cursor
    stdout.execute(style::ResetColor)?;
    stdout.execute(cursor::Show)?;
    stdout.flush()
}

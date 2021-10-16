use std::{
    env::args,
    error::Error,
    io::{Stdout, Write},
    sync::mpsc::{self, Sender},
    thread,
    time::{Duration, Instant},
};

use ansi_term::{Colour, Style};
use crossterm::{cursor::MoveTo, style::Print, QueueableCommand};
use figglebit::{cleanup, init, parse, Renderer};

type Tx = Sender<AppEvent>;

enum AppEvent {
    Tick,
    Quit,
}

fn events(tx: Tx) {
    use crossterm::event::{self, Event as CEvent, KeyCode, KeyEvent, KeyModifiers as KeyMods};

    thread::spawn(move || loop {
        if let Ok(CEvent::Key(KeyEvent { code, modifiers })) = event::read() {
            match code {
                KeyCode::Esc => drop(tx.send(AppEvent::Quit)),
                KeyCode::Char('c') if modifiers.contains(KeyMods::CONTROL) => drop(tx.send(AppEvent::Quit)),
                _ => {}
            }
        }
    });
}

fn tick_timer(tx: Tx) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(1));
        let _ = tx.send(AppEvent::Tick);
    });
}

fn format_time(mut total_sec: i32, show_zeroes: bool) -> String {
    let is_less_than_zero = total_sec < 0;
    if is_less_than_zero {
        total_sec *= -1;
    }
    let hours = total_sec / 60 / 60;
    let minutes = total_sec / 60 - (hours * 60);
    let seconds = total_sec - minutes * 60 - hours * 60 * 60;

    format!(
        "{}{}{}{:0>2}",
        if is_less_than_zero { "-" } else { "" },
        if hours.eq(&0) && !show_zeroes { "".to_string() } else { format!("{:0>2}:", hours) },
        if hours.eq(&0) && minutes.eq(&0) && !show_zeroes { "".to_string() } else { format!("{:0>2}:", minutes) },
        seconds
    )
}

struct AfkConfig {
    allow_negative: bool,
    hours: i32,
    minutes: i32,
    seconds: i32,
    style: Style,
    words: String,
    blink_timer: Instant,
    blink_rate: u64, // in ms
    is_blinking: bool,
    show_zeroes: bool,
    use_font: bool,
}

impl Default for AfkConfig {
    fn default() -> Self {
        Self {
            allow_negative: false,
            hours: 0,
            minutes: 0,
            seconds: 0,
            style: Style::new().fg(Colour::White),
            words: "".to_string(),
            blink_timer: Instant::now(),
            blink_rate: 500,
            is_blinking: false,
            show_zeroes: true,
            use_font: false,
        }
    }
}

impl AfkConfig {
    fn flip_blinker(&mut self) {
        if self.blink_timer.elapsed() >= Duration::from_millis(self.blink_rate) {
            self.is_blinking = !self.is_blinking;
            self.blink_timer = Instant::now();
        }
    }
}

fn show_help() {
    let help = include_str!("../README.md");

    println!("{}", Style::new().fg(Colour::Blue).bold().paint(help));
}

macro_rules! show_error {
    ($error:expr) => {{
        println!("{}", Style::new().fg(Colour::Red).bold().paint($error));
        return None;
    }};
}

fn parse_args(args: &[String]) -> Option<AfkConfig> {
    if args.is_empty() {
        return None;
    }

    let mut config = AfkConfig::default();

    let mut args = args.iter();

    while let Some(arg) = args.next() {
        match arg.to_lowercase().as_ref() {
            "--help" => return None,
            "-k" => config.allow_negative = true,
            "-h" | "-m" | "-s" => match args.next() {
                Some(t) => match t.parse() {
                    Ok(t) => match arg.to_lowercase().as_ref() {
                        "-h" => config.hours = t,
                        "-m" => config.minutes = t,
                        "-s" => config.seconds = t,
                        _ => {}
                    },
                    Err(_) => show_error!(&format!("Cannout parse number after {}.", arg)),
                },
                None => show_error!(&format!("Missing number after {}.", arg)),
            },
            "-c" => {
                config.style = match args.next() {
                    Some(c) => match parse_color(c) {
                        Some(c) => Style::new().fg(c).bold(),
                        None => show_error!(&format!("Unknown color after {}.", arg)),
                    },
                    None => show_error!(&format!("Missing color after {}.", arg)),
                }
            }
            "-0" => config.show_zeroes = false,
            "-f" => config.use_font = true,
            _ => {
                // takes the first unquoted word or "quoted string of words" ignoring any words, strings, or invalid commands after
                if config.words.is_empty() {
                    config.words = arg.to_string();
                }
            }
        }
    }

    // prefer some time to act against, unless allow_negative, which is basically just a stopwatch
    if config.hours.eq(&0) && config.minutes.eq(&0) && config.seconds.eq(&0) && !config.allow_negative {
        show_error!("Please specifiy some time or -k for stopwatch.");
    }

    Some(config)
}

fn parse_color(color: &str) -> Option<Colour> {
    let color = match color.to_lowercase().as_ref() {
        "black" => Colour::Black,
        "red" => Colour::Red,
        "green" => Colour::Green,
        "yellow" => Colour::Yellow,
        "blue" => Colour::Blue,
        "purple" => Colour::Purple,
        "cyan" => Colour::Cyan,
        "white" => Colour::White,
        _ => {
            // Check for RGB color value formatted as 42,42,42 or "42 42 42"
            let rgb = color.contains(&[',', ' '][..]).then(|| {
                color.split(&[',', ' '][..]).map(str::parse::<u8>).filter_map(Result::ok).collect::<Vec<u8>>()
            })?;

            if rgb.len() != 3 {
                show_error!("RGB values should have 3 numbers separated by commas.");
            }

            Colour::RGB(rgb[0], rgb[1], rgb[2])
        }
    };

    Some(color)
}

// this returns the y offset for the fig font numbers to start printing from
// a single line message will always be 1(since it prints on 0)
// a fig font message will be > 1 unless something is borked with the font
fn print_words(out: &mut Stdout, renderer: &Renderer, config: &AfkConfig) -> Result<u16, Box<dyn Error>> {
    if config.words.is_empty() {
        return Ok(1);
    }

    let words = if config.use_font {
        let mut buf = Vec::with_capacity(config.words.len() * 8);
        renderer.render(&config.words, &mut buf)?;
        String::from_utf8(buf)?
    } else {
        config.words.clone()
    };

    let words: String = words.lines().filter(|l| !l.trim_end().is_empty()).map(|l| format!("{}\r\n", l)).collect();

    out.queue(Print(config.style.paint(&words)))?;

    let offset = words.lines().count() as u16;

    match offset {
        1.. => Ok(offset),
        0 => Ok(1),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = args().skip(1).collect::<Vec<_>>();

    let mut config = if let Some(config) = parse_args(&args) {
        config
    } else {
        // display a helper, so they know how to use it
        show_help();
        return Ok(());
    };

    let num_font = parse(include_str!("../resources/Ghost.flf").to_string()).expect("Failed to parse font: Ghost.flf");
    let words_font = parse(include_str!("../resources/Big.flf").to_string()).expect("Failed to parse font: Big.flf");

    let mut stdout = init().expect("Failed to acquire stdout.");

    let mut total_seconds = config.hours * 60 * 60 + config.minutes * 60 + config.seconds;
    let mut old_lines: Vec<String> = Vec::new();

    let (tx, rx) = mpsc::channel();
    events(tx.clone());
    tick_timer(tx);

    stdout.queue(MoveTo(0, 0))?;

    // print the message one time. resizing the window too small will erase whatever goes past the window edge
    // cast now, so we don't cast muiltiple later
    let offset_y = print_words(&mut stdout, &Renderer::new(words_font), &config)? as i32;

    let renderer = Renderer::new(num_font);

    loop {
        if total_seconds == 0 && !config.allow_negative {
            config.flip_blinker();
        }

        let mut buf = Vec::new();
        if !config.is_blinking {
            let text = format_time(total_seconds, config.show_zeroes);
            renderer.render(&text, &mut buf)?;
        }

        if let Ok(txt) = String::from_utf8(buf) {
            let lines = txt.lines().map(ToString::to_string).collect::<Vec<_>>();

            for (i, line) in old_lines.drain(..).enumerate() {
                let i = i as i32;
                stdout.queue(MoveTo(0, (offset_y + i) as u16))?;
                let line = line.to_string();
                stdout.queue(Print(" ".repeat(line.len())))?;
            }

            let mut num_y_offset = 0;

            for (i, line) in lines.iter().enumerate() {
                let i = i as i32;
                if line.trim().is_empty() {
                    num_y_offset += 1;
                    continue;
                }
                stdout.queue(MoveTo(0, (offset_y - num_y_offset + i) as u16))?;
                stdout.queue(Print(config.style.paint(line)))?;
            }

            old_lines = lines;
            stdout.flush()?;
        }

        if let Ok(app_event) = rx.try_recv() {
            match app_event {
                AppEvent::Tick => {
                    if total_seconds > 0 || config.allow_negative {
                        total_seconds -= 1;
                    }
                }
                AppEvent::Quit => break,
            }
        }

        thread::sleep(Duration::from_millis(100));
    }

    cleanup(&mut stdout);

    Ok(())
}

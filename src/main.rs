use std::{
    env::args,
    error::Error,
    io::Write,
    sync::mpsc::{self, Sender},
    thread,
    time::{Duration, Instant},
};

use ansi_term::{Colour, Style};
use crossterm::{cursor::MoveTo, event, style::Print, QueueableCommand};
use figglebit::{cleanup, init, parse, Renderer};

type Tx = Sender<Event>;

enum Event {
    Tick,
    Quit,
}

fn events(tx: Tx) {
    thread::spawn(move || loop {
        if let Ok(ev) = event::read() {
            match ev {
                event::Event::Key(event::KeyEvent { code: event::KeyCode::Esc, .. }) => {
                    let _ = tx.send(Event::Quit);
                }
                event::Event::Key(event::KeyEvent {
                    code: event::KeyCode::Char('c'),
                    modifiers: event::KeyModifiers::CONTROL,
                }) => {
                    let _ = tx.send(Event::Quit);
                }
                _ => {}
            }
        }
    });
}

fn tick_timer(tx: Tx) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(1));
        let _ = tx.send(Event::Tick);
    });
}

fn format_time(mut total_sec: i128, show_zeroes: bool) -> String {
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
    hours: i128,
    minutes: i128,
    seconds: i128,
    color: Colour,
    words: String,
    blink_timer: Instant,
    blink_rate: u64, // in ms
    is_blinking: bool,
    show_zeroes: bool,
}

impl Default for AfkConfig {
    fn default() -> Self {
        Self {
            allow_negative: false,
            hours: 0,
            minutes: 0,
            seconds: 0,
            color: Colour::White,
            words: "".to_string(),
            blink_timer: Instant::now(),
            blink_rate: 500,
            is_blinking: false,
            show_zeroes: true,
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

// TODO: format this with nice colors and stuff
fn show_help() {
    let help = r#"
Usage: afk "some text to show" -h # -m # -s # -k -c blue

Text to display can be empty, a single word, or a "quoted string" of words.

-h #  Number of hours to count down
-m #  Number of minutes to count down
-s #  Number of seconds to count down
You can enter time in any combination of hms or just one.
The application will adjust it. Ex: -s 90 will translate to 1m 30s.

-c color  colors the text with a bold foreground color.
Colors: Black, Red, Green, Yellow, Blue, Purple, Cyan, White
Color can be an comma separated RGB value: 42,42,42

-k Allow countdown to go negative / Stopwatch mode

-0 Hide hour or minutes when zero

--help  shows this help
"#;

    println!("{}", Style::new().fg(Colour::Blue).bold().paint(help));
}

fn show_error(error: &str) {
    println!("{}", Style::new().fg(Colour::Red).bold().paint(error));
}

fn parse_args(args: Vec<String>) -> Option<AfkConfig> {
    let mut config = AfkConfig::default();

    let mut args = args.iter();

    while let Some(arg) = args.next() {
        match arg.to_lowercase().as_ref() {
            "--help" => return None,
            "-k" => config.allow_negative = true,
            "-h" | "-m" | "-s" => {
                if let Some(t) = args.next() {
                    if let Ok(t) = t.parse() {
                        match arg.to_lowercase().as_ref() {
                            "-h" => config.hours = t,
                            "-m" => config.minutes = t,
                            "-s" => config.seconds = t,
                            _ => {}
                        }
                    } else {
                        show_error(&format!("Cannout parse number after {}.", arg));
                        return None;
                    }
                } else {
                    show_error(&format!("Missing number after {}.", arg));
                    return None;
                }
            }
            "-c" => {
                config.color = {
                    if let Some(c) = args.next() {
                        if let Some(c) = parse_color(c) {
                            c
                        } else {
                            show_error(&format!("Unknown color after {}.", arg));
                            return None;
                        }
                    } else {
                        show_error(&format!("Missing color after {}.", arg));
                        return None;
                    }
                }
            }
            "-0" => {
                config.show_zeroes = false;
            }
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
        show_error(&format!("Please specifiy some time or -k for stopwatch."));
        return None;
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
            // Check for RGB color value formatted as 42,42,42
            if color.contains(',') {
                let rgb_str: Vec<&str> = color.split(',').collect();

                if rgb_str.len().eq(&3) {
                    let mut rgb: Vec<u8> = Vec::with_capacity(3);

                    for maybe_u8 in rgb_str {
                        if let Ok(u) = maybe_u8.parse::<u8>() {
                            rgb.push(u);
                        } else {
                            show_error("RGB values should be a number from 0 to 255");
                            return None;
                        }
                    }

                    Colour::RGB(rgb[0], rgb[1], rgb[2])
                } else {
                    show_error("RGB values should have 3 numbers separated by commas.");
                    return None;
                }
            } else {
                return None;
            }
        }
    };

    Some(color)
}

fn main() -> Result<(), Box<dyn Error>> {
    let arg = args().skip(1).collect::<Vec<_>>();

    // display a helper, so they know how to use it
    if arg.is_empty() {
        show_help();
        return Ok(());
    }

    let mut config = match parse_args(arg) {
        Some(w) => w,
        None => {
            show_help();
            return Ok(());
        }
    };

    let font_data = include_str!("../resources/Ghost.flf").to_owned();
    let font = parse(font_data).unwrap();
    let mut stdout = init().unwrap();
    let renderer = Renderer::new(font);

    let mut total_seconds = config.hours * 60 * 60 + config.minutes * 60 + config.seconds;
    let mut old_lines: Vec<String> = Vec::new();

    let (tx, rx) = mpsc::channel();
    events(tx.clone());
    tick_timer(tx);

    let paint = Style::new().fg(config.color).bold();

    let offset_y = 3;
    stdout.queue(MoveTo(2, offset_y - 1))?;
    stdout.queue(Print(paint.paint(&config.words)))?;

    loop {
        #[allow(unused_assignments)]
        let mut text = String::new();
        let mut buf = Vec::new();

        if total_seconds == 0 && !config.allow_negative {
            config.flip_blinker();
        }

        if !config.is_blinking {
            text = format_time(total_seconds, config.show_zeroes);
            renderer.render(&text, &mut buf)?;
        }

        if let Ok(txt) = String::from_utf8(buf) {
            let lines = txt.lines().map(|l| l.to_string()).collect::<Vec<_>>();

            for (i, line) in old_lines.drain(..).enumerate() {
                stdout.queue(MoveTo(0, offset_y + i as u16))?;
                let line = line.to_string();
                stdout.queue(Print(" ".repeat(line.len())))?;
            }

            for (i, line) in lines.iter().enumerate() {
                stdout.queue(MoveTo(0, offset_y + i as u16))?;
                stdout.queue(Print(paint.paint(line)))?;
            }

            old_lines = lines;
            stdout.flush()?;
        }

        if let Ok(ev) = rx.try_recv() {
            match ev {
                Event::Tick => {
                    if total_seconds > 0 || config.allow_negative {
                        total_seconds -= 1;
                    }
                }
                Event::Quit => break,
            }
        }

        thread::sleep(Duration::from_millis(100));
    }

    cleanup(&mut stdout);

    Ok(())
}

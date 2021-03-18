use std::{
    env::args,
    error::Error,
    io::Write,
    sync::mpsc::{self, Sender},
    thread,
    time::Duration,
};

use ansi_term::{Colour, Style};

use crossterm::{cursor::MoveTo, event, style::Print, QueueableCommand};

use figglebit::{cleanup, init, parse, Renderer};

type Tx = Sender<Event>;

enum Event {
    CatpeasanTick,
    Quit,
}

fn events(tx: Tx) {
    thread::spawn(move || loop {
        if let Ok(ev) = event::read() {
            match ev {
                event::Event::Key(event::KeyEvent {
                    code: event::KeyCode::Esc,
                    ..
                }) => {
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
        let _ = tx.send(Event::CatpeasanTick);
        thread::sleep(Duration::from_secs(1));
    });
}

fn format_time(mut total_sec: i128) -> String {
    let is_less_than_zero = total_sec < 0;
    if is_less_than_zero {
        total_sec *= -1;
    }
    let hours = total_sec / 60 / 60;
    let minutes = total_sec / 60 - (hours * 60);
    let seconds = total_sec - minutes * 60 - hours * 60 * 60;

    format!(
        "{}{:0>2}:{:0>2}:{:0>2}",
        if is_less_than_zero { "-" } else { "" },
        hours,
        minutes,
        seconds
    )
}

// TODO:
// add a commandline help -h --h --help -? --? /? etc
// apply some color to the clock output

// -k keep running negative
// -h hours
// -m minutes
// -s seconds
// -c foreground color
// "words" or word

struct MountainDew {
    allow_negative: bool,
    hours: i128,
    minutes: i128,
    seconds: i128,
    color: Colour,
    words: String,
}

impl Default for MountainDew {
    fn default() -> Self {
        Self {
            allow_negative: true,
            hours: 0,
            minutes: 0,
            seconds: 0,
            color: Colour::Red,
            words: "".to_string(),
        }
    }
}

fn show_help() {
    println!("Usage: ask \"Some string or a single unquoted word\" -h #HOURS -m #MINUTES -s #SECONDS -c \"#FFFFFF\"\n-c : color is optional. hex format.\n-h -m -s : input by hours, minutes, seconds or their cumulative parts as one type.")
}

fn show_error(error: &str) {
    // println!("{}", Colour::Red.paint(error));
    println!("{}", Style::new().fg(Colour::Red).bold().paint(error));
}

fn parse_args(args: Vec<String>) -> Option<MountainDew> {
    let mut water_bottle = MountainDew::default();

    let mut args = args.iter();

    while let Some(arg) = args.next() {
        match arg.to_lowercase().as_ref() {
            "-k" => water_bottle.allow_negative = true,
            // FIXME: handle so we can fall back to help display
            "-h" | "-m" | "-s" => {
                if let Some(t) = args.next() {
                    if let Ok(t) = t.parse() {
                        match arg.to_lowercase().as_ref() {
                            "-h" => water_bottle.hours = t,
                            "-m" => water_bottle.minutes = t,
                            "-s" => water_bottle.seconds = t,
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
                water_bottle.color = {
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
            _ => {
                // takes the first unquoted word or "quoted string of words" ignoring any words, strings, or invalid commands after
                if water_bottle.words.is_empty() {
                    water_bottle.words = arg.to_string();
                }
            }
        }
    }

    Some(water_bottle)
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
        // Fixed(u8),
        // RGB(u8, u8, u8),
        _ => return None,
    };

    Some(color)
}

fn main() -> Result<(), Box<dyn Error>> {
    let arg = args().skip(1).collect::<Vec<_>>();

    // display a helper, so they know how to use it
    if arg.is_empty()  {
        show_error("You need some help, and eventually will get it.");
        return Ok(());
    }

    let water_bottle = match parse_args(arg) {
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

    let mut total_seconds =
        water_bottle.hours * 60 * 60 + water_bottle.minutes * 60 + water_bottle.seconds;
    let mut old_lines: Vec<String> = Vec::new();

    let (tx, rx) = mpsc::channel();
    events(tx.clone());
    tick_timer(tx);

    let offset_y = 3;
    stdout.queue(MoveTo(2, offset_y - 1))?;
    stdout.queue(Print(water_bottle.words))?;

    loop {
        let text = &format_time(total_seconds);
        let mut buf = Vec::new();
        renderer.render(&text, &mut buf)?;

        if let Ok(txt) = String::from_utf8(buf) {
            let lines = txt.lines().map(|l| l.to_string()).collect::<Vec<_>>();

            for (i, line) in old_lines.drain(..).enumerate() {
                stdout.queue(MoveTo(0, offset_y + i as u16))?;
                let line = line.to_string();
                stdout.queue(Print(" ".repeat(line.len())))?;
            }

            for (i, line) in lines.iter().enumerate() {
                stdout.queue(MoveTo(0, offset_y + i as u16))?;
                stdout.queue(Print(&line))?;
            }

            old_lines = lines;
            stdout.flush()?;
        }

        if let Ok(ev) = rx.recv() {
            match ev {
                Event::CatpeasanTick => {
                    if total_seconds > 0 || water_bottle.allow_negative {
                        total_seconds -= 1;
                    }
                    thread::sleep(Duration::from_secs(1));
                }
                Event::Quit => break,
            }
        }
    }

    cleanup(&mut stdout);

    Ok(())
}

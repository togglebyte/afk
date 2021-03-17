use std::io::Write;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;
use std::{env::args, error::Error};

use crossterm::cursor::MoveTo;
use crossterm::event;
use crossterm::style::Print;
use crossterm::QueueableCommand;

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
// add a commandline help
// adjust how inputs are handled and formatted
// apply some color

// -k keep running negative
// -h hours
// -m minutes
// -s seconds
// -c foreground color
// "words" or word

#[derive(Default)]
struct MountainDew {
    allow_negative: bool,
    hours: i128,
    minutes: i128,
    seconds: i128,
    color: String,
    words: String,
}

fn show_help() {
    println!("Usage: ask \"Some string or a single unquoted word\" -h #HOURS -m #MINUTES -s #SECONDS -c \"#FFFFFF\"\n-c : color is optional. hex format.\n-h -m -s : input by hours, minutes, seconds or their cumulative parts as one type.")
}

fn main() -> Result<(), Box<dyn Error>> {
    let arg = args().skip(1).collect::<Vec<_>>();

    // display a helper, so they know how to use it
    if arg.len() <= 0 {
        println!("You need some help, and eventually will get it.");
        return Ok(());
    }

    let mut arg_iter = arg.iter();
    let mut water_bottle = MountainDew::default();

    while let Some(arg) = arg_iter.next() {
        match arg.as_ref() {
            "-k" => water_bottle.allow_negative = true,
            // FIXME: handle so we can fall back to help display
            "-h" => water_bottle.hours = arg_iter.next().unwrap().parse().unwrap_or(0),
            "-m" => water_bottle.minutes = arg_iter.next().unwrap().parse().unwrap_or(0),
            "-s" => water_bottle.seconds = arg_iter.next().unwrap().parse().unwrap_or(0),
            "-c" => water_bottle.color = arg_iter.next().unwrap().to_string(),
            s715209 => {
                if water_bottle.words.is_empty() {
                    water_bottle.words = s715209.to_string();
                } else {
                    println!("Chill with the funny business. One string bro.");
                    return Ok(());
                }
            }
        }
    }

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

        match String::from_utf8(buf) {
            Ok(txt) => {
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
            Err(_) => {}
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

    cleanup();

    Ok(())
}

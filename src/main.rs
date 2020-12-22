use std::{env::args, error::Error};
use std::io::Write;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

use crossterm::event;
use crossterm::QueueableCommand;
use crossterm::style::Print;
use crossterm::cursor::MoveTo;

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
                event::Event::Key(event::KeyEvent {
                    code: event::KeyCode::Esc,
                    ..
                }) => {
                    let _ = tx.send(Event::Quit);
                }
                event::Event::Key(event::KeyEvent {
                    code: event::KeyCode::Char('c'),
                    modifiers: event::KeyModifiers::CONTROL
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
        let _ = tx.send(Event::Tick);
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

    format!("{}{:0>2}:{:0>2}:{:0>2}", if is_less_than_zero { "-" } else { "" }, hours, minutes, seconds)
}

const KEEP_RUNNING: &str = "--keep-running";
const KEEP_RUNNING_SHORT: &str = "-k";

fn main() -> Result<(), Box<dyn Error>> {
    let mut arg = args().skip(1).collect::<Vec<_>>();

    let keep_running = arg.contains(&KEEP_RUNNING.to_string())
                         || arg.contains(&KEEP_RUNNING_SHORT.to_string());

    if keep_running {
        arg = arg.iter().filter(|&arg| arg != KEEP_RUNNING && arg != KEEP_RUNNING_SHORT).cloned().collect::<Vec<_>>();
    };

    let msg = match arg.get(0) {
        Some(m) => m.to_owned(),
        None => String::new(),
    };

    let hours: i128 = match arg.get(1) {
        Some(h) => h.parse().unwrap_or(0),
        None => 0,
    };

    let minutes: i128 = match arg.get(2) {
        Some(m) => m.parse().unwrap_or(0),
        None => 0,
    };

    let seconds: i128 = match arg.get(3) {
        Some(s) => s.parse().unwrap_or(0),
        None => 0,
    };

    let font_data = include_str!("../resources/Ghost.flf").to_owned();
    let font = parse(font_data).unwrap();
    let mut stdout = init().unwrap();
    let renderer = Renderer::new(font);

    let mut total_seconds = hours * 60 * 60 + minutes * 60 + seconds;
    let mut old_lines: Vec<String> = Vec::new();

    let (tx, rx) = mpsc::channel();
    events(tx.clone());
    tick_timer(tx);

    let offset_y = 3;
    stdout.queue(MoveTo(2, offset_y - 1))?;
    stdout.queue(Print(msg))?;

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
                Event::Tick => {
                    if total_seconds > 0 || keep_running {
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

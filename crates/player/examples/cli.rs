//! Plays the whole lazer library from a terminal.
//!
//! `cargo run -p player --release --example cli -- [fade seconds]` after `dotnet build helper/Helper -c Release`.
//! Commands on stdin: pause, next, prev, shuffle, repeat, vol 0.5, seek 30, quit.

use std::io::BufRead;
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;

use library::helper::{Event as HelperEvent, Helper};
use library::protocol::Message;
use player::{Event, Player, Repeat};

fn main() {
    let fade: f64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(3.0);
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../helper/Helper/bin/Release/net10.0");
    let exe = root.join(if cfg!(windows) {
        "listenosu-helper.exe"
    } else {
        "listenosu-helper"
    });
    let data_dir = library::discover::detect().expect("a lazer folder");

    let helper = Helper::start(exe, data_dir.clone());
    let songs = loop {
        match helper.events().recv_timeout(Duration::from_secs(30)) {
            Ok(HelperEvent::Message(Message::Snapshot { songs, .. })) => break songs,
            Ok(HelperEvent::Failed(why)) => panic!("helper failed: {why}"),
            Ok(_) => {}
            Err(_) => panic!("no snapshot within 30 s"),
        }
    };
    println!("{} songs", songs.len());

    let dir = data_dir.clone();
    let mut player =
        Player::new(move |song: &library::Song| song.audio_path(&dir)).expect("audio device");
    player.set_crossfade(Duration::from_secs_f64(fade));
    player.set_volume(0.3);
    let titles: Vec<String> = songs
        .iter()
        .map(|s| format!("{} - {}", s.artist(false), s.title(false)))
        .collect();
    player.play(songs, 0);

    let (tx, commands) = channel();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });

    let mut repeat = Repeat::Off;
    let mut shuffle = false;
    loop {
        for event in player.tick() {
            match event {
                Event::Started(i) => println!("playing {}", titles[i]),
                Event::Skipped(i) => println!("skipped (missing or unreadable): {}", titles[i]),
                Event::Finished => return println!("queue finished"),
            }
        }
        while let Ok(line) = commands.try_recv() {
            let words: Vec<&str> = line.split_whitespace().collect();
            let arg = words.get(1).and_then(|w| w.parse::<f64>().ok());
            match words.first().copied().unwrap_or("") {
                "pause" | "" => player.toggle(),
                "next" => player.next(),
                "prev" => player.previous(),
                "shuffle" => {
                    shuffle = !shuffle;
                    player.set_shuffle(shuffle);
                    println!("shuffle {shuffle}");
                }
                "repeat" => {
                    repeat = match repeat {
                        Repeat::Off => Repeat::All,
                        Repeat::All => Repeat::One,
                        Repeat::One => Repeat::Off,
                    };
                    player.set_repeat(repeat);
                    println!("repeat {repeat:?}");
                }
                "vol" => player.set_volume(arg.unwrap_or(0.3) as f32),
                "seek" => player.seek(arg.unwrap_or(0.0)),
                "quit" => return,
                other => println!("unknown command {other}"),
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

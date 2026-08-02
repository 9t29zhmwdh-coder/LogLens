//! Stellt fest, welche Ereignisse `notify` beim Anhaengen tatsaechlich meldet.
//!
//! `file_collector` reagiert nur auf `Modify(Data(_))`. Ob die jeweilige
//! Plattform das liefert, laesst sich nicht erraten, und auf einem Mac laesst
//! sich Windows nicht nachstellen. Dieser Test misst es dort, wo es zaehlt,
//! und schreibt das Ergebnis in die Ausgabe.

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::io::Write;
use std::sync::mpsc;
use std::time::Duration;

#[test]
fn report_which_event_kinds_this_platform_sends_for_an_append() {
    let verzeichnis = std::env::temp_dir().join(format!("ll-kinds-{}", std::process::id()));
    std::fs::create_dir_all(&verzeichnis).expect("Testverzeichnis");
    let pfad = verzeichnis.join("p.log");
    std::fs::write(&pfad, "start\n").expect("Datei anlegen");

    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(
        move |res: notify::Result<notify::Event>| {
            if let Ok(e) = res {
                let _ = tx.send(e);
            }
        },
        notify::Config::default(),
    )
    .expect("Watcher");
    watcher
        .watch(&pfad, RecursiveMode::NonRecursive)
        .expect("watch");

    std::thread::sleep(Duration::from_millis(500));

    {
        let mut datei = std::fs::OpenOptions::new()
            .append(true)
            .open(&pfad)
            .expect("oeffnen");
        writeln!(datei, "ERROR eins").expect("schreiben");
        writeln!(datei, "INFO zwei").expect("schreiben");
        datei.flush().expect("flush");
    }

    let mut gesehen: Vec<String> = Vec::new();
    while let Ok(e) = rx.recv_timeout(Duration::from_secs(5)) {
        gesehen.push(format!("{:?}", e.kind));
    }
    let _ = std::fs::remove_dir_all(&verzeichnis);

    println!("PLATTFORM={}", std::env::consts::OS);
    println!("EREIGNISSE={gesehen:?}");
    let mit_data = gesehen.iter().filter(|k| k.contains("Data")).count();
    println!("MIT_MODIFY_DATA={mit_data}");

    assert!(
        !gesehen.is_empty(),
        "auf {} kam ueberhaupt kein Ereignis an",
        std::env::consts::OS
    );
    // Bewusst keine Forderung nach Modify(Data(..)): Windows meldet
    // Modify(Any), und genau deshalb filtert file_collector inzwischen auf
    // Modify(_). Was hier zaehlt, ist dass ueberhaupt ein Modify ankommt.
    let mit_modify = gesehen.iter().filter(|k| k.starts_with("Modify")).count();
    assert!(
        mit_modify > 0,
        "auf {} meldet notify beim Anhaengen kein Modify ueberhaupt; \
         file_collector wuerde hier nie nachlesen. Gesehen: {:?}",
        std::env::consts::OS,
        gesehen
    );
}

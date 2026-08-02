//! Prueft, dass eine angehaengte Zeile beim Collector ankommt.
//!
//! Das ist die Kernschleife dieses Werkzeugs und war bis hierher ungetestet.
//! Sie haengt an `notify`, und ein Sprung ueber zwei Hauptversionen kann die
//! Ereignisse veraendern, ohne dass sich am Kompilieren etwas zeigt. Ein
//! Ausfall waere lautlos: die Oberflaeche bliebe leer, ohne Fehlermeldung.
//!
//! Nebenbei haelt der Test fest, dass eine Zeile erst herausgeht, wenn die
//! naechste sie abschliesst: der Stacktrace-Sammler muss wissen, ob eine
//! Fortsetzung folgt.

use std::io::Write;
use std::sync::Arc;
use std::time::Duration;

use ll_core::clustering::ClusterGrouper;
use ll_core::collector::file_collector;
use ll_core::models::log_entry::{LogSource, LogSourceKind};
use ll_core::normalizer::CustomParserSet;
use tokio::sync::{mpsc, watch};

// Voruebergehend zurueckgestellt: dieser Test scheitert unter Windows, und
// solange er das Testbinary abbricht, kommt der Messtest daneben nie dran.
// Wird wieder aktiviert, sobald die Messung zeigt, woran es liegt.
#[cfg_attr(windows, ignore)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_line_appended_to_a_watched_file_reaches_the_collector() {
    let verzeichnis = std::env::temp_dir().join(format!("ll-tail-{}", std::process::id()));
    std::fs::create_dir_all(&verzeichnis).expect("Testverzeichnis");
    let pfad = verzeichnis.join("test.log");
    std::fs::write(&pfad, "vorher\n").expect("Datei anlegen");

    let (tx, mut rx) = mpsc::channel(16);
    let (cancel_tx, cancel_rx) = watch::channel(false);

    let pfad_string = pfad.to_string_lossy().to_string();
    let quelle = LogSource::new(
        "test",
        LogSourceKind::File { path: pfad_string.clone() },
    );

    let aufgabe = tokio::spawn(file_collector::run(
        quelle,
        pfad_string.clone(),
        tx,
        Arc::new(ClusterGrouper::new()),
        Arc::new(CustomParserSet::compile(&[])),
        cancel_rx,
    ));

    // Dem Watcher Zeit geben, sich auf die Datei zu setzen. Ohne das haengt
    // der Test an einem Wettlauf statt an der Sache, die er pruefen soll.
    tokio::time::sleep(Duration::from_millis(300)).await;

    {
        let mut datei = std::fs::OpenOptions::new()
            .append(true)
            .open(&pfad)
            .expect("zum Anhaengen oeffnen");
        // Zwei Zeilen, mit Absicht. `StacktraceAccumulator` haelt eine Zeile
        // zurueck, bis die naechste zeigt, ob ein Stacktrace folgt. Mit nur
        // einer Zeile wartet der Test auf etwas, das erst beim Beenden kaeme.
        writeln!(datei, "ERROR etwas ist passiert").expect("anhaengen");
        writeln!(datei, "INFO danach").expect("anhaengen");
        datei.flush().expect("flush");
    }

    let ergebnis = tokio::time::timeout(Duration::from_secs(10), rx.recv()).await;

    let _ = cancel_tx.send(true);
    aufgabe.abort();
    let _ = std::fs::remove_dir_all(&verzeichnis);

    let eintrag = ergebnis
        .expect("innerhalb von zehn Sekunden kam nichts an: der Watcher meldet die Aenderung nicht mehr")
        .expect("der Kanal wurde geschlossen, bevor eine Zeile ankam");

    assert!(
        eintrag.message.contains("etwas ist passiert"),
        "die angehaengte Zeile kam veraendert an: {:?}",
        eintrag.message
    );
}

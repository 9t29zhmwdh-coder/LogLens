//! Prueft den Syslog-Empfaenger ueber einen echten Socket.
//!
//! Der Empfaenger ist die einzige Quelle, die nicht liest, sondern
//! entgegennimmt. Ein Bindungsfehler oder ein falsch zerlegtes Datagramm
//! faellt beim Kompilieren nicht auf, sondern erst dann, wenn ein Geraet im
//! Netz sendet und in der Oberflaeche nichts erscheint. Deshalb geht dieser
//! Test wirklich ueber UDP und TCP statt die Zerlegung direkt aufzurufen.

use std::sync::Arc;
use std::time::Duration;

use ll_core::clustering::ClusterGrouper;
use ll_core::collector::syslog_collector;
use ll_core::models::log_entry::{LogSource, LogSourceKind, LogLevel, SyslogTransport};
use tokio::net::{TcpStream, UdpSocket};
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, watch};

/// Laesst das Betriebssystem einen freien Port waehlen, damit parallele
/// Testlaeufe sich nicht gegenseitig den Port wegnehmen.
async fn freier_port() -> u16 {
    let socket = UdpSocket::bind("127.0.0.1:0").await.expect("Port");
    socket.local_addr().expect("Adresse").port()
}

fn quelle(port: u16, protokoll: SyslogTransport) -> LogSource {
    LogSource::new(
        "Testempfaenger",
        LogSourceKind::Syslog { bind: format!("127.0.0.1:{port}"), protocol: protokoll },
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn eine_unifi_zeile_per_udp_wird_klassifiziert() {
    let port = freier_port().await;
    let (tx, mut rx) = mpsc::channel(16);
    let (abbruch_tx, abbruch_rx) = watch::channel(false);

    tokio::spawn(syslog_collector::run(
        quelle(port, SyslogTransport::Udp),
        format!("127.0.0.1:{port}"),
        SyslogTransport::Udp,
        tx,
        Arc::new(ClusterGrouper::new()),
        abbruch_rx,
    ));
    tokio::time::sleep(Duration::from_millis(150)).await;

    let sender = UdpSocket::bind("127.0.0.1:0").await.expect("Sendesocket");
    let zeile = "<132>Aug 27 10:15:00 U7-Pro hostapd: ath0: STA aa:bb:cc:dd:ee:ff WPA: invalid MIC in msg 2/4 of 4-Way Handshake";
    sender.send_to(zeile.as_bytes(), format!("127.0.0.1:{port}")).await.expect("senden");

    let eintrag = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("Zeitueberschreitung, es kam nichts an")
        .expect("Kanal geschlossen");

    assert_eq!(eintrag.level, LogLevel::Warn);
    assert_eq!(eintrag.service.as_deref(), Some("hostapd"));
    assert_eq!(eintrag.fields["event_type"], "wifi_auth_failure");
    assert_eq!(eintrag.fields["client_mac"], "aa:bb:cc:dd:ee:ff");
    assert_eq!(eintrag.fields["vendor"], "unifi");
    assert!(!eintrag.fingerprint.is_empty(), "Gruppierung muss gelaufen sein");

    let _ = abbruch_tx.send(true);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mehrere_zeilen_in_einem_datagramm_werden_einzeln_gemeldet() {
    let port = freier_port().await;
    let (tx, mut rx) = mpsc::channel(16);
    let (abbruch_tx, abbruch_rx) = watch::channel(false);

    tokio::spawn(syslog_collector::run(
        quelle(port, SyslogTransport::Udp),
        format!("127.0.0.1:{port}"),
        SyslogTransport::Udp,
        tx,
        Arc::new(ClusterGrouper::new()),
        abbruch_rx,
    ));
    tokio::time::sleep(Duration::from_millis(150)).await;

    let sender = UdpSocket::bind("127.0.0.1:0").await.expect("Sendesocket");
    let buendel = "<134>Aug 27 10:15:00 UXG dnsmasq-dhcp[1]: DHCPACK(br0) 192.168.1.50 aa:bb:cc:dd:ee:ff eins\n\
                   <134>Aug 27 10:15:01 UXG dnsmasq-dhcp[1]: DHCPACK(br0) 192.168.1.51 aa:bb:cc:dd:ee:01 zwei";
    sender.send_to(buendel.as_bytes(), format!("127.0.0.1:{port}")).await.expect("senden");

    let mut adressen = Vec::new();
    for _ in 0..2 {
        let eintrag = tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("Zeitueberschreitung")
            .expect("Kanal geschlossen");
        adressen.push(eintrag.fields["src_ip"].as_str().unwrap_or_default().to_string());
    }
    adressen.sort();
    assert_eq!(adressen, vec!["192.168.1.50", "192.168.1.51"]);

    let _ = abbruch_tx.send(true);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn eine_zeile_per_tcp_kommt_ebenfalls_an() {
    let port = freier_port().await;
    let (tx, mut rx) = mpsc::channel(16);
    let (abbruch_tx, abbruch_rx) = watch::channel(false);

    tokio::spawn(syslog_collector::run(
        quelle(port, SyslogTransport::Tcp),
        format!("127.0.0.1:{port}"),
        SyslogTransport::Tcp,
        tx,
        Arc::new(ClusterGrouper::new()),
        abbruch_rx,
    ));
    tokio::time::sleep(Duration::from_millis(150)).await;

    let mut strom = TcpStream::connect(format!("127.0.0.1:{port}")).await.expect("verbinden");
    let zeile = "<134>Aug 27 10:15:00 fw filterlog: 5,,,1000000103,igb0,match,block,in,4,0x0,,64,1,0,DF,6,tcp,60,192.0.2.10,198.51.100.20,54321,443\n";
    strom.write_all(zeile.as_bytes()).await.expect("senden");

    let eintrag = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("Zeitueberschreitung")
        .expect("Kanal geschlossen");

    assert_eq!(eintrag.fields["event_type"], "firewall_blocked");
    assert_eq!(eintrag.fields["vendor"], "pfsense");
    assert_eq!(eintrag.fields["dst_port"], "443");

    let _ = abbruch_tx.send(true);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn eine_zeile_ohne_syslog_form_wird_verworfen_statt_geraten() {
    let port = freier_port().await;
    let (tx, mut rx) = mpsc::channel(16);
    let (abbruch_tx, abbruch_rx) = watch::channel(false);

    tokio::spawn(syslog_collector::run(
        quelle(port, SyslogTransport::Udp),
        format!("127.0.0.1:{port}"),
        SyslogTransport::Udp,
        tx,
        Arc::new(ClusterGrouper::new()),
        abbruch_rx,
    ));
    tokio::time::sleep(Duration::from_millis(150)).await;

    let sender = UdpSocket::bind("127.0.0.1:0").await.expect("Sendesocket");
    sender.send_to(b"kein Syslog, nur Text", format!("127.0.0.1:{port}")).await.expect("senden");
    // Danach eine gueltige Zeile: kommt die an, war der Empfaenger die ganze
    // Zeit ansprechbar und hat die erste bewusst verworfen.
    let gueltig = "<134>Aug 27 10:15:00 UXG dnsmasq-dhcp[1]: DHCPACK(br0) 192.168.1.50 aa:bb:cc:dd:ee:ff geraet";
    sender.send_to(gueltig.as_bytes(), format!("127.0.0.1:{port}")).await.expect("senden");

    let eintrag = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("Zeitueberschreitung")
        .expect("Kanal geschlossen");
    assert_eq!(eintrag.fields["event_type"], "dhcp_lease_granted");

    let _ = abbruch_tx.send(true);
}

//! Checks that a stored secret survives the process that wrote it.
//!
//! `keyring` without a platform feature compiles and passes a store-then-read
//! test inside one process, because it falls back to a store held in process
//! memory. Nothing persists and no error is raised, so a same-process test is
//! blind to exactly the defect this guards against.
//!
//! CleanFlow ships for macOS, Windows and Linux, and each has its own backend,
//! so the check runs on all three. It distinguishes three outcomes:
//!
//! - writing fails: a real backend is compiled in but its service is missing,
//!   which is the normal state of a Linux CI runner with no D-Bus secret
//!   service. Not the defect, so the test passes and says so.
//! - writing succeeds and a second process reads the value back: a real,
//!   persistent store.
//! - writing succeeds and the second process finds nothing: the in-memory
//!   fallback, which is the defect.
//!
//! The second process is this same binary re-run, not `/usr/bin/security`. The
//! keychain grants read access per application, so a different binary asking
//! for an item it did not create raises an authorisation dialog that blocks CI
//! and interrupts whoever is at the keyboard.

use std::process::Command;

const SERVICE: &str = "cf-keychain-test";
const CHILD_ENV: &str = "CF_KEYCHAIN_TEST_CHILD";
const SECRET: &str = "correct horse battery staple";

#[test]
fn a_stored_secret_survives_the_writing_process() {
    if let Ok(account) = std::env::var(CHILD_ENV) {
        let Ok(entry) = keyring::Entry::new(SERVICE, &account) else {
            std::process::exit(3);
        };
        match entry.get_password() {
            Ok(v) if v == SECRET => std::process::exit(0),
            Ok(_) => std::process::exit(2),
            Err(_) => std::process::exit(3),
        }
    }

    let account = format!("probe-{}", std::process::id());

    // Where the connection to the store is made differs by version: keyring 3
    // opens it on the first write, keyring 4 already on Entry::new. Either
    // failure means a real backend is compiled in and only its service is
    // missing, so both are treated the same way.
    let entry = match keyring::Entry::new(SERVICE, &account) {
        Ok(e) => e,
        Err(e) => {
            println!("backend present, service unavailable, persistence not exercised: {e}");
            return;
        }
    };

    if let Err(e) = entry.set_password(SECRET) {
        // The in-memory fallback never fails to write, so an error here proves a
        // real backend is compiled in. Its service simply is not running.
        println!("backend present, service unavailable, persistence not exercised: {e}");
        return;
    }

    let status = Command::new(std::env::current_exe().expect("own path"))
        .args(["--exact", "a_stored_secret_survives_the_writing_process", "--nocapture"])
        .env(CHILD_ENV, &account)
        .status()
        .expect("run child");

    let _ = entry.delete_credential();

    match status.code() {
        Some(0) => {}
        Some(2) => panic!("the store returned a different value"),
        Some(3) => panic!(
            "nothing persisted: keyring fell back to an in-memory store, \
             which means no platform feature is enabled for this target"
        ),
        other => panic!("child exited unexpectedly: {other:?}"),
    }
}

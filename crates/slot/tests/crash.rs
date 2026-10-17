use std::os::unix::process::ExitStatusExt;
use std::process::Command;

const CHILD: &str = "SLOT_CRASH_CHILD";

#[test]
fn a_crash_says_where_it_happened_and_still_dies_by_its_signal() {
    if std::env::var_os(CHILD).is_some() {
        slot::crash::install();
        std::thread::Builder::new()
            .name("slot-emu".into())
            .spawn(|| unsafe { std::ptr::read_volatile(8 as *const u8) })
            .unwrap()
            .join()
            .ok();
        return;
    }
    let out = Command::new(std::env::current_exe().unwrap())
        .args([
            "a_crash_says_where_it_happened_and_still_dies_by_its_signal",
            "--exact",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .output()
        .unwrap();
    let log = String::from_utf8_lossy(&out.stderr);
    let line = log
        .lines()
        .find(|l| l.starts_with("slot: crash: "))
        .unwrap_or_else(|| panic!("no crash line in the log:\n{log}"));
    assert!(
        line.contains("SIGSEGV") || line.contains("SIGBUS"),
        "{line}"
    );
    assert!(line.contains("thread slot-emu"), "{line}");
    assert!(line.contains("addr 0x8 "), "{line}");
    let exe = std::env::current_exe().unwrap();
    let name = exe.file_name().unwrap().to_string_lossy();
    assert!(
        line.contains(&*name),
        "the crash does not name the code it was in: {line}"
    );
    assert!(
        matches!(out.status.signal(), Some(11) | Some(10)),
        "the process did not die by its signal, so no core is kept: {:?}",
        out.status
    );
}

#[test]
fn a_crash_signal_sent_from_outside_is_reported_and_still_kills() {
    if std::env::var_os(CHILD).is_some() {
        slot::crash::install();
        let me = std::process::id().to_string();
        std::thread::Builder::new()
            .name("slot-kit".into())
            .spawn(move || {
                Command::new("kill").args(["-SEGV", &me]).status().unwrap();
                std::thread::sleep(std::time::Duration::from_secs(10));
            })
            .unwrap()
            .join()
            .ok();
        return;
    }
    let out = Command::new(std::env::current_exe().unwrap())
        .args([
            "a_crash_signal_sent_from_outside_is_reported_and_still_kills",
            "--exact",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .output()
        .unwrap();
    let log = String::from_utf8_lossy(&out.stderr);
    let line = log
        .lines()
        .find(|l| l.starts_with("slot: crash: SIGSEGV"))
        .unwrap_or_else(|| panic!("no crash line in the log:\n{log}"));
    assert!(line.contains(" pc "), "{line}");
    assert_eq!(
        out.status.signal(),
        Some(11),
        "a crash sent by kill -SEGV (as the device crash test does) did not end the process by SIGSEGV: {:?}",
        out.status
    );
}

#[test]
fn a_crash_on_the_main_thread_is_reported_as_slot() {
    if std::env::var_os(CHILD).is_some() {
        slot::crash::install();
        // Run with one test thread, so this thread and the main thread are the only ones.
        // Block the signal here so the kernel hands it to the main thread, as on the
        // device, where the main thread is named after ld-linux.
        unsafe {
            let mut set: libc::sigset_t = std::mem::zeroed();
            libc::sigemptyset(&mut set);
            libc::sigaddset(&mut set, libc::SIGSEGV);
            libc::pthread_sigmask(libc::SIG_BLOCK, &set, std::ptr::null_mut());
            libc::kill(libc::getpid(), libc::SIGSEGV);
        }
        std::thread::sleep(std::time::Duration::from_secs(10));
        return;
    }
    let out = Command::new(std::env::current_exe().unwrap())
        .args([
            "a_crash_on_the_main_thread_is_reported_as_slot",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD, "1")
        .output()
        .unwrap();
    let log = String::from_utf8_lossy(&out.stderr);
    let line = log
        .lines()
        .find(|l| l.starts_with("slot: crash: SIGSEGV"))
        .unwrap_or_else(|| panic!("no crash line in the log:\n{log}"));
    assert!(
        line.contains("in thread slot addr"),
        "the main thread is not reported as slot: {line}"
    );
    assert_eq!(out.status.signal(), Some(11), "{:?}", out.status);
}

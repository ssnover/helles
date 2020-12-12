use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::TryRecvError;
use std::sync::Arc;
use std::time::Duration;

use json_command_server::Server;

fn main() -> std::io::Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .unwrap();

    let (server, rx) = Server::new("/tmp/test.sock")?;
    let server_context = Server::start(server, running.clone());

    while running.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_secs(1));
        match rx.try_recv() {
            Ok(cmd) => println!("Server got command: {}", cmd),
            Err(err) => {
                match err {
                    TryRecvError::Disconnected => { running.store(false, Ordering::SeqCst); },
                    _ => {},
                }
            }
        }
    }

    server_context.join().expect("Failed to join server thread");

    Ok(())
}

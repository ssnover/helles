# helles

helles is a library providing a Unix-socket based client and server for sending JSON-based messages to a daemon application.

### Usage

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::TryRecvError;
use std::sync::Arc;
use std::time::Duration;

use helles::Server;

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
            Err(err) => match err {
                TryRecvError::Disconnected => {
                    running.store(false, Ordering::SeqCst);
                }
                _ => {}
            },
        }
    }

    server_context.join().expect("Failed to join server thread");

    Ok(())
}
```

### License

Licensed under either of
 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be dual licensed as above, without any
additional terms or conditions.
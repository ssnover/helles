#![allow(irrefutable_let_patterns)]

use std::io::prelude::*;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

//type Handler = Box<dyn FnOnce(String) + Send + 'static>;

pub struct Server {
    socket: UnixListenerWrapper,
}

impl Server {
    pub fn new(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let wrapper = match UnixListenerWrapper::bind(path) {
            Ok(sock) => sock,
            Err(err) => {
                eprintln!("json-cmd-srv: Bind to socket failed: {}", err);
                return Err(err);
            }
        };

        Ok(Server { socket: wrapper })
    }

    pub fn run(&self, keep_running: Arc<AtomicBool>) {
        let listener = &self.socket.listener;
        listener.set_nonblocking(true).unwrap();

        let mut buffer = [0 as u8; 1024];

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    match self.handle_client(stream, &mut buffer) {
                        Ok(()) => (),
                        Err(err) => {
                            eprintln!("json-cmd-srv: Failure to handle client: {}", err);
                        }
                    };
                }
                Err(ref err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(100));
                    if !keep_running.load(Ordering::SeqCst) {
                        break;
                    }
                }
                Err(err) => {
                    eprintln!("json-cmd-srv: Connection failed: {}", err);
                    break;
                }
            }
        }
    }

    pub fn start(server: Server, keep_running: Arc<AtomicBool>) -> JoinHandle<()> {
        std::thread::spawn(move || server.run(keep_running))
    }

    fn handle_client(&self, mut stream: UnixStream, buffer: &mut [u8]) -> std::io::Result<()> {
        stream.set_nonblocking(false)?;
        stream.set_read_timeout(Some(Duration::from_millis(100)))?;

        let mut left_braces = 0 as u32;
        let mut right_braces = 0 as u32;

        let mut byte_counter = 0 as usize;
        // Should probably check this error, since timeout is intended to be valid error case to handle
        while let bytes_read = stream.read(buffer)? {
            let total_bytes = byte_counter + bytes_read;
            if total_bytes > buffer.len() {
                eprintln!(
                    "json-cmd-srv: Received a message that is too long: {}",
                    total_bytes + bytes_read
                );
                break;
            }

            let mut end_brace_idx = None;
            // Iterate over the new bytes to check for curly braces
            for (idx, ch) in buffer[byte_counter..total_bytes].iter().enumerate() {
                match ch {
                    b'{' => {
                        left_braces += 1;
                    }
                    b'}' => {
                        right_braces += 1;
                        if left_braces == right_braces {
                            end_brace_idx = Some(idx);
                        }
                    }
                    _ => {}
                };
            }

            if let Some(idx) = end_brace_idx {
                self.handle_command(String::from_utf8_lossy(&buffer[..idx]).to_string());
                break;
            } else {
                byte_counter += bytes_read;
            }
        }
        Ok(())
    }

    fn handle_command(&self, cmd: String) {
        println!("json-cmd-srv: Got a command: {}", cmd);
    }
}

struct UnixListenerWrapper {
    path: PathBuf,
    listener: UnixListener,
}

impl UnixListenerWrapper {
    fn bind(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref().to_owned();
        UnixListener::bind(&path).map(|listener| UnixListenerWrapper { path, listener })
    }
}

impl Drop for UnixListenerWrapper {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path).unwrap();
    }
}

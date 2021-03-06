// Copyright (c) 2020 Shane Snover <ssnover95@gmail.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::io::prelude::*;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

pub struct CommandClient {
    socket_path: PathBuf,
}

impl CommandClient {
    pub fn new(path: impl AsRef<Path>) -> Self {
        CommandClient {
            socket_path: path.as_ref().to_owned(),
        }
    }
}

impl Write for CommandClient {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut stream = UnixStream::connect(&self.socket_path)?;
        stream.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub struct Server {
    socket: UnixListenerWrapper,
    channel_tx: Sender<String>,
}

impl Server {
    pub fn new(path: impl AsRef<Path>) -> std::io::Result<(Self, Receiver<String>)> {
        let (tx, rx) = channel();
        let wrapper = match UnixListenerWrapper::bind(path) {
            Ok(sock) => sock,
            Err(err) => {
                eprintln!("json-cmd-srv: Bind to socket failed: {}", err);
                return Err(err);
            }
        };

        Ok((
            Server {
                socket: wrapper,
                channel_tx: tx,
            },
            rx,
        ))
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
        stream.set_read_timeout(Some(Duration::from_millis(500)))?;

        let mut left_braces = 0 as u32;
        let mut right_braces = 0 as u32;

        let mut byte_counter = 0 as usize;
        loop {
            let bytes_read = stream.read(buffer)?;
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
                            break;
                        }
                    }
                    _ => {}
                };
            }

            if let Some(idx) = end_brace_idx {
                self.handle_command(String::from_utf8_lossy(&buffer[..idx + 1]).to_string());
                break;
            } else {
                byte_counter += bytes_read;
            }
        }
        Ok(())
    }

    fn handle_command(&self, cmd: String) {
        match self.channel_tx.send(cmd) {
            Ok(_) => {}
            Err(err) => eprintln!("json-cmd-srv: Error sending command: {}", err),
        }
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

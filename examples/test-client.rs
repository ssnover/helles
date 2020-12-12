use std::io::prelude::*;
use std::os::unix::net::UnixStream;

fn main() -> std::io::Result<()> {
    let mut stream = UnixStream::connect("/tmp/test.sock")?;

    stream.write_all(br#"{"cmd":1}{"cmd":2}"#)?;

    Ok(())
}

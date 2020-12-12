use std::io::prelude::*;
use std::os::unix::net::UnixStream;

fn main() -> std::io::Result<()> {
    let mut stream = UnixStream::connect("/tmp/test.sock")?;
    print!("> ");
    let mut buffer = String::new();
    let mut stdin = std::io::stdin();
    //stdin.read_to_string(&mut buffer)?;

    //stream.write_all(&buffer.into_bytes()[..])?;
    stream.write_all(br#"{"cmd":1"#)?;

    Ok(())
}
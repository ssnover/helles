use std::io::prelude::*;

fn main() -> std::io::Result<()> {
    let mut client = helles::CommandClient::new("/tmp/test.sock");
    client.write(br#"{"cmd":1}{"cmd":2}"#)?;

    Ok(())
}

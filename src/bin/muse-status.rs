use muse_status::{client, errors::MuseStatusError};

fn main() -> Result<(), MuseStatusError> {
    // now isn't this simple :)
    let client = client::Client::new()?;
    client.act()
}

use muse_status::errors::MuseStatusError;
use muse_status::client;

fn main() -> Result<(), MuseStatusError> {
    // now isn't this simple :)
    let client = client::Client::new()?;
    client.act()
}

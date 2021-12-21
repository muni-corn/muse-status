use muse_status::client;
use muse_status::errors::MuseStatusError;

fn main() -> Result<(), MuseStatusError> {
    // now isn't this simple :)
    let client = client::Client::new()?;
    client.act()
}

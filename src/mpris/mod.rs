use crate::errors::*;
use crate::format::blocks::output::{BlockOutput, BlockText};
use crate::format::blocks::{Block, BlockOutputMsg, NextUpdate};
use crate::format::Attention;
use mpris as mpris_lib;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

/// A block that displays information about any media currently playing on the device.
pub struct MprisBlock {
    playing_icon: char,
    paused_icon: char,

    status: PlayerStatus,
    title: Option<String>,
    artist: Option<String>,
}

impl Default for MprisBlock {
    fn default() -> Self {
        MprisBlock {
            playing_icon: '\u{F0F74}',
            paused_icon: '\u{F03E4}',

            status: PlayerStatus::Stopped,
            title: None,
            artist: None,
        }
    }
}

impl MprisBlock {
    /// Returns a new MprisBlock.
    pub fn new() -> Self {
        Default::default()
    }

    fn get_icon(&self) -> char {
        match self.status {
            PlayerStatus::Playing => self.playing_icon,
            PlayerStatus::Paused => self.paused_icon,
            PlayerStatus::Stopped => self.paused_icon,
        }
    }

    fn set_metadata(&mut self, metadata: mpris::Metadata) {
        self.title = metadata.title().map(String::from);

        self.artist = if let Some(av) = metadata.album_artists() {
            av.first().map(|first_artist| first_artist.to_string())
        } else {
            None
        };
    }

    fn main_iteration(
        mutex: Arc<Mutex<Box<Self>>>,
        block_sender: Sender<BlockOutputMsg>,
    ) -> Result<(), MuseStatusError> {
        let mut player = mpris_lib::PlayerFinder::new()
            .map_err(|e| UpdateError {
                block_name: "mpris".to_string(),
                message: format!("couldn't create PlayerFinder: {e}"),
            })?
            .find_active()
            .map_err(|e| UpdateError {
                block_name: "mpris".to_string(),
                message: format!("couldn't find active player: {e}"),
            })?;

        // allow a timeout of 10s
        player.set_dbus_timeout_ms(10000);

        {
            let mut block = mutex.lock().unwrap();
            let metadata = player.get_metadata().map_err(|e| UpdateError {
                block_name: block.name().to_owned(),
                message: format!("{}", e),
            })?;
            block.set_metadata(metadata);
            block_sender
                .send(BlockOutputMsg::new(block.name(), block.output()))
                .unwrap();
        }

        match player.events() {
            Ok(mut events) => {
                while let Some(Ok(e)) = events.next() {
                    // update the player data, then send the update
                    let mut block = mutex.lock().unwrap();

                    match e {
                        // update the block depending on the Event
                        mpris_lib::Event::Playing => block.status = PlayerStatus::Playing,
                        mpris_lib::Event::Paused => block.status = PlayerStatus::Paused,
                        mpris_lib::Event::Stopped | mpris_lib::Event::PlayerShutDown => {
                            block.status = PlayerStatus::Stopped
                        }
                        mpris_lib::Event::TrackChanged(m) => block.set_metadata(m),
                        _ => (),
                    }

                    block_sender
                        .send(BlockOutputMsg::new(block.name(), block.output()))
                        .unwrap();
                }
            }
            Err(e) => eprintln!("error getting player events: {}", e),
        }

        {
            let mut block = mutex.lock().unwrap();
            block.status = PlayerStatus::Stopped;
            block_sender
                .send(BlockOutputMsg::new(block.name(), block.output()))
                .unwrap();
        }

        Ok(())
    }
}

impl Block for MprisBlock {
    fn run(
        self: Box<Self>,
        block_sender: Sender<BlockOutputMsg>,
    ) -> (Vec<JoinHandle<()>>, Sender<()>) {
        // This might seem dumb, but MprisBlock updates are dependent on updates from the mpris
        // client, so it will not listen to any "notify" requests
        let (notify_tx, _) = std::sync::mpsc::channel::<()>();

        let mutex = Arc::new(Mutex::new(self));
        let player_listen_handle = thread::Builder::new()
            .name(String::from("mpris player listener"))
            .spawn(move || loop {
                if let Err(e) = Self::main_iteration(mutex.clone(), block_sender.clone()) {
                    eprintln!("error in main mpris block loop: {e}");
                }

                // sleep after every iteration to prevent spamming
                thread::sleep(std::time::Duration::from_secs(5));
            })
            .unwrap();

        (vec![player_listen_handle], notify_tx)
    }

    fn update(&mut self) -> Result<(), UpdateError> {
        Ok(())
    }

    fn name(&self) -> &str {
        "mpris"
    }

    fn next_update(&self) -> Option<NextUpdate> {
        Some(NextUpdate::In(chrono::Duration::seconds(5)))
    }

    fn output(&self) -> Option<BlockOutput> {
        match self.status {
            PlayerStatus::Stopped => None,
            _ => {
                let text = if let Some(title) = &self.title {
                    if let Some(artist) = &self.artist {
                        // title and artist exist, so we can do a pair!
                        BlockText::Pair(title.to_owned(), artist.to_owned())
                    } else {
                        // title exists, but no artist
                        BlockText::Single(title.to_owned())
                    }
                } else {
                    // no title (and we'll exclude the artist too, even if it's something)
                    // use some generic default string
                    BlockText::Single(String::from("Media is playing"))
                };
                Some(BlockOutput::new(
                    self.name(),
                    Some(self.get_icon()),
                    text,
                    Attention::Normal,
                ))
            }
        }
    }
}

/// Represents the playing, paused, or stopped state of a player.
pub enum PlayerStatus {
    /// The player is playing. The play icon is shown.
    Playing,

    /// The player is paused. The pause icon is shown.
    Paused,

    /// The player is stopped. The block is hidden from the status bar.
    Stopped,
}

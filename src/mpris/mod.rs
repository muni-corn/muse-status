use crate::errors::*;
use crate::format::blocks::output::{BlockOutput, BlockOutputContent, NiceOutput};
use crate::format::blocks::Block;
use crate::format::Attention;
use mpris as mpris_lib;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

/// A block that displays information about any media currently playing on the device.
pub struct MprisBlock {
    next_update_time: chrono::DateTime<chrono::Local>,

    playing_icon: char,
    paused_icon: char,

    status: PlayerStatus,
    title: Option<String>,
    artist: Option<String>,
}

impl Default for MprisBlock {
    fn default() -> Self {
        MprisBlock {
            next_update_time: chrono::Local::now() + chrono::Duration::seconds(5),
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
        self.title = if let Some(t) = metadata.title() {
            Some(String::from(t))
        } else {
            None
        };

        self.artist = if let Some(av) = metadata.album_artists() {
            Some(av[0].clone())
        } else {
            None
        };
    }

    fn main_loop(
        mutex: Arc<Mutex<Box<Self>>>,
        block_sender: Sender<BlockOutput>,
    ) -> Result<(), MuseStatusError> {
        let player = loop {
            thread::sleep(std::time::Duration::from_secs(5));

            if let Ok(player_finder) = mpris_lib::PlayerFinder::new() {
                if let Ok(player) = player_finder.find_active() {
                    break player;
                }
            }
        };

        {
            let mut block = mutex.lock().unwrap();
            let metadata = player.get_metadata().map_err(|e| UpdateError {
                block_name: block.name().to_owned(),
                message: format!("{}", e),
            })?;
            block.set_metadata(metadata);
            block_sender.send(BlockOutput::new(block.name(), block.output())).unwrap();
        }

        if let Ok(mut events) = player.events() {
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

                block_sender.send(BlockOutput::new(block.name(), block.output())).unwrap();
            }
        }

        {
            let mut block = mutex.lock().unwrap();
            block.status = PlayerStatus::Stopped;
            block_sender.send(BlockOutput::new(block.name(), block.output())).unwrap();
        }

        Ok(())
    }
}

impl Block for MprisBlock {
    fn run(
        self: Box<Self>,
        block_sender: Sender<BlockOutput>,
    ) -> (Vec<JoinHandle<()>>, Sender<String>) {
        // This might seem dumb, but MprisBlock updates are dependent on updates from the mpris
        // client, so it will not listen to any "notify" requests
        let (notify_tx, _) = std::sync::mpsc::channel::<String>();

        let mutex = Arc::new(Mutex::new(self));
        let player_listen_handle = thread::Builder::new()
            .name(String::from("mpris player listener"))
            .spawn(move || loop {
                let _ = Self::main_loop(mutex.clone(), block_sender.clone());
            })
            .unwrap();

        (vec![player_listen_handle], notify_tx)
    }

    fn update(&mut self) -> Result<(), UpdateError> {
        Ok(())
    }

    fn name(&self) -> &str {
        "playerctl"
    }

    fn next_update_time(&self) -> Option<chrono::DateTime<chrono::Local>> {
        Some(self.next_update_time)
    }

    fn output(&self) -> Option<BlockOutputContent> {
        match self.status {
            PlayerStatus::Stopped => None,
            _ => Some(BlockOutputContent::from(NiceOutput {
                primary_text: self.title.clone().unwrap_or_else(String::new),
                secondary_text: self.artist.clone(),
                icon: self.get_icon(),
                attention: Attention::Normal,
            })),
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

use crate::format::blocks::Block;
use crate::format::blocks::output::{BlockOutput, BlockOutputBody};
use std::thread::JoinHandle;
use std::thread;
use std::sync::mpsc::Sender;
use crate::errors::*;

const PLAYING_ICON: char = '\u{f387}';
const PAUSED_ICON: char = '\u{f3e4}';

pub struct MprisBlock {
    next_update_time: chrono::DateTime<chrono::Local>
}

impl Default for MprisBlock {
    fn default() -> Self {
        MprisBlock {
            next_update_time: chrono::Local::now() + chrono::Duration::seconds(5)
        }
    }
}

impl MprisBlock {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Block for MprisBlock {
    fn run(mut self: Box<Self>, block_sender: Sender<BlockOutput>) -> (Vec<JoinHandle<()>>, Sender<String>) {
        let (notify_tx, notify_rx) = std::sync::mpsc::channel::<String>();

        let find_players_handle = thread::spawn(move || loop {
        });

        let sender_clone = block_sender.clone();
        let listen_notify_handle = thread::spawn(move || {
            while let Ok(s) = notify_rx.recv() {
                if s == self.name() {
                    self.update().unwrap();
                    sender_clone.send(BlockOutput::new(self.name(), self.output()));
                }
            }
        });

        (vec![find_players_handle, listen_notify_handle], notify_tx)
    }

    fn update(&mut self) -> Result<(), UpdateError> {
        unimplemented!()
    }

    fn name(&self) -> &str {
        "playerctl"
    }

    fn next_update_time(&self) -> Option<chrono::DateTime<chrono::Local>> {
        Some(self.next_update_time)
    }

    fn output(&self) -> Option<BlockOutputBody> {
        unimplemented!()
    }
}

pub enum PlayerStatus {
    Playing,
    Paused,
    Stopped,
}

/// A module for block outputs.
pub mod output;

use crate::{errors::UpdateError, format};
use chrono::{DateTime, Duration, Local};
use serde::{Deserialize, Serialize};
use std::{
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time,
};

pub use output::BlockOutput;

/// Represents when or in how much time the next update of a block should occur.
pub enum NextUpdate {
    /// The next update occurs after the specified duration.
    In(Duration),

    /// The next update occurs at a specified time.
    At(DateTime<Local>),
}

/// A type to represent the block output that is sent over MPSC channels.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BlockOutputMsg {
    /// The name of the block.
    name: String,

    /// The output of the block. If None, the block is (temporarily) removed from the status bar
    data: Option<BlockOutput>
}

impl BlockOutputMsg {
    pub fn new(name: &str, data: Option<BlockOutput>) -> Self {
        Self {
            name: name.to_string(),
            data,
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn data(&self) -> Option<BlockOutput> {
        self.data.to_owned()
    }
}

/// Block is a piece of data in the status bar.
pub trait Block: Send + Sync {
    /// Runs the block asynchronously. The tuple returns (1) a `Vec` of `JoinHandle`s to any threads
    /// started asynchronously and (2) a `Sender` that will send notification query to force an
    /// update on blocks (via `muse-status notify <block-name>`).
    ///
    /// About the returned `Sender`: If a request to notify blocks is sent, the `Sender` sends the
    /// block name specified (or whatever string is sent through). The `Block`, which should be
    /// listening with a partnered `Receiver` in a different thread, can handle this data as it
    /// pleases.
    fn run(self: Box<Self>, block_sender: Sender<BlockOutputMsg>) -> (Vec<JoinHandle<()>>, Sender<()>)
    where
        Self: 'static,
    {
        let (notify_tx, notify_rx) = mpsc::channel::<()>();

        // make arcs and mutexes
        let loop_thread_name = format!("{} update loop", self.name());
        let notify_listener_thread_name = format!("{} notify listening thread", self.name());
        let block_arc_mutex = Arc::new(Mutex::new(self));
        let arc_clone = block_arc_mutex.clone();

        // clone the sender
        let output_sender_clone = block_sender.clone();

        // start block auto-updating loop
        let loop_handle = thread::Builder::new()
            .name(loop_thread_name)
            .spawn(move || loop {
                // update block and return next update
                let next_update_opt = {
                    let mut block = block_arc_mutex.lock().unwrap();

                    // update block and then update the bar
                    if let Err(e) = block.update() {
                        println!("{}", e)
                    }
                    let _ = block_sender.send(BlockOutputMsg::new(block.name(),block.output()));

                    block.next_update()
                };

                // sleep until next update
                if let Some(next_update) = next_update_opt {
                    let chrono_duration = match next_update {
                        NextUpdate::At(date_time) => {
                            let now = Local::now();
                            date_time - now
                        }
                        NextUpdate::In(duration) => duration,
                    };

                    let std_duration = chrono_duration
                        .to_std()
                        .unwrap_or(time::Duration::from_secs(5));
                    thread::sleep(std_duration);
                } else {
                    break;
                }
            })
            .unwrap();

        // listen for update requests
        let notify_listen_handle = thread::Builder::new()
            .name(notify_listener_thread_name)
            .spawn(move || {
                while notify_rx.recv().is_ok() {
                    let mut block = arc_clone.lock().unwrap();
                    let _ = block.update();
                    output_sender_clone
                        .send(BlockOutputMsg::new(block.name(), block.output()))
                        .unwrap();
                }
            })
            .unwrap();

        (vec![loop_handle, notify_listen_handle], notify_tx)
    }

    /// Sets the banner sender.
    fn set_banner_sender(&mut self, _banner_sender: Sender<format::Banner>) {}

    /// Updates the block, returning an error if the update fails.
    fn update(&mut self) -> Result<(), UpdateError>;

    /// The next time at which the block will update, if any. If None, the block will immediately
    /// stop polling/updating automatically.
    fn next_update(&self) -> Option<NextUpdate>;

    /// Output returns Some BlockOutputBody, or None. If None, the Block is hidden from the status
    /// bar. If Some, the block is updated in the status bar.
    fn output(&self) -> Option<BlockOutput>;

    /// Returns the name of the block, which is used as a sort of key in the status bar. It's used
    /// to update blocks in the status bar.
    fn name(&self) -> &str;
}

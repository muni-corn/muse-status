/// A module for block outputs.
pub mod output;

use crate::errors::UpdateError;
use crate::format;
pub use output::{BlockOutput, BlockOutputContent};
use std::sync::mpsc;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

/// Block is a piece of data in the status bar.
pub trait Block: Send + Sync {
    /// Runs the block asynchronously. The tuple returns (1) a Vec of JoinHandles to any threads
    /// started asynchronously and (2) a Sender that will send notification query to force an
    /// update on blocks (via `muse-status notify <block-name>`).
    ///
    /// About the returned Sender: If a request to notify blocks is sent, the Sender sends the
    /// block name specified (or whatever string is sent through). The Block, which should be
    /// listening with a partnered Receiver in a different thread, can handle this data as it
    /// pleases.
    fn run(self: Box<Self>, block_sender: Sender<BlockOutput>) -> (Vec<JoinHandle<()>>, Sender<()>)
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

        let loop_handle = thread::Builder::new()
            .name(loop_thread_name)
            .spawn(move || loop {
                let next_update_time = {
                    let mut block = block_arc_mutex.lock().unwrap();

                    // update and update the bar
                    if let Err(e) = block.update() {
                        println!("{}", e)
                    }
                    let _ = block_sender.send(BlockOutput::new(block.name(), block.output()));

                    block.next_update_time()
                };

                let now = chrono::Local::now();
                if let Some(d) = next_update_time {
                    let duration = (d - now).to_std().unwrap();
                    thread::sleep(duration);
                } else {
                    break;
                }
            })
            .unwrap();

        let notify_listen_handle = thread::Builder::new()
            .name(notify_listener_thread_name)
            .spawn(move || {
                while notify_rx.recv().is_ok() {
                    let mut block = arc_clone.lock().unwrap();
                    let _ = block.update();
                    output_sender_clone
                        .send(BlockOutput::new(block.name(), block.output()))
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
    fn next_update_time(&self) -> Option<chrono::DateTime<chrono::Local>>;

    /// Output returns Some BlockOutputBody, or None. If None, the Block is hidden from the status
    /// bar. If Some, the block is updated in the status bar.
    fn output(&self) -> Option<BlockOutputContent>;

    /// Returns the name of the block, which is used as a sort of key in the status bar. It's used
    /// to update blocks in the status bar.
    fn name(&self) -> &str;
}

pub mod node;
pub mod tracker;
pub mod wallet;

use actix::prelude::*;
use consensus::Entry;
use futures::{future::try_join_all, prelude::*};

pub use node::*;
pub use tracker::*;
pub use wallet::*;

pub struct SystemAddrs {
    pub node_addrs: Vec<Addr<Node>>,
    pub tracker_addr: Addr<Tracker>,
    pub wallet_addr: Addr<Wallet>,
}

impl SystemAddrs {
    pub async fn get_all_entries(&self) -> Result<Vec<Entry>, actix::MailboxError> {
        try_join_all(self.node_addrs.iter().map(|node| node.send(EntryRequest)))
            .await
            .map(|res| res.into_iter().filter_map(|res| res.ok()).collect())
    }

    pub async fn get_all_distances(&self) -> Result<Vec<u32>, actix::MailboxError> {
        let entries = self.get_all_entries().await?;

        let n_entries = entries.len();
        let mut distances = Vec::with_capacity(n_entries * n_entries / 2);
        for i in 0..entries.len() {
            for j in 0..i {
                let dist = entries[i]
                    .oddsketch
                    .iter()
                    .zip(entries[j].oddsketch.iter())
                    .fold(0, |total, (byte_a, byte_b)| {
                        total + (byte_a ^ byte_b).count_ones()
                    });
                distances.push(dist);
            }
        }
        Ok(distances)
    }
}

pub fn start_simulation(
    nodes: Vec<Node>,
    wallet_fan: usize,
    broadcast_interval_ms: u64,
) -> SystemAddrs {
    // Start nodes
    let node_addrs: Vec<_> = nodes.into_iter().map(move |node| node.start()).collect();

    // Start trackers
    let tracker_addr = Tracker::new(node_addrs.clone()).start();

    // Start wallet
    let wallet_addr = Wallet::new(node_addrs.clone(), wallet_fan, broadcast_interval_ms).start();

    SystemAddrs {
        node_addrs,
        tracker_addr,
        wallet_addr,
    }
}

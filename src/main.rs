pub mod node;
pub mod tracker;
pub mod wallet;

use actix::prelude::*;

use node::*;
use tracker::*;
use wallet::*;

fn start_simulation(nodes: Vec<Node>, wallet_fan: usize, broadcast_interval_ms: u64) {
    let sys = System::new("experiment");

    // Start nodes
    let addresses: Vec<_> = nodes.into_iter().map(move |node| node.start()).collect();
    
    // Start trackers
    Tracker::new(addresses.clone()).start();

    // Start wallet
    Wallet::new(addresses, wallet_fan, broadcast_interval_ms).start();

    // Run experiment
    sys.run();
}

fn main() {
    // Start nodes and collect addresses
    let n_nodes = 30;
    let heartbeat_ms = 1_000;

    let mut nodes: Vec<_> = (0..n_nodes)
        .map(|_| {
            Node::new(1, heartbeat_ms, 1, 4)
        })
        .collect();
    nodes.push(Node::new(2, heartbeat_ms, 1, 4));
    
    start_simulation(nodes, 10, 100);
}

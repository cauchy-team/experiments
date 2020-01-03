pub mod node;
pub mod tracker;
pub mod wallet;

use actix::prelude::*;

use node::*;
use tracker::*;
use wallet::*;

fn main() {
    let sys = System::new("experiment");

    // Start nodes and collect addresses
    let n_nodes = 30;
    let heartbeat_ms = 1_000;

    let mut addresses: Vec<_> = (0..n_nodes)
        .map(|_| {
            let node = Node::new(1, heartbeat_ms, 1, 4);
            node.start()
        })
        .collect();
    let addr = Node::new(2, heartbeat_ms, 1, 4).start();
    addresses.push(addr);
    let tracker = Tracker::new(addresses.clone());
    tracker.start();

    // Wallet
    let wallet_fan = 12;
    let wallet = Wallet::new(addresses, wallet_fan, 10);
    wallet.start();

    // let mut node = Node::new(10, 1000);
    // node.start();
    sys.run();
}

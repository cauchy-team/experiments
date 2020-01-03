use std::time::Duration;

use actix::prelude::*;
use rand::{seq::SliceRandom, Rng};

use crate::node::*;

pub struct Wallet {
    nodes: Vec<Addr<Node>>,
    sample_size: usize,
    broadcast_interval: u64,
}

impl Wallet {
    pub fn new(nodes: Vec<Addr<Node>>, sample_size: usize, broadcast_interval: u64) -> Self {
        Wallet {
            nodes,
            sample_size,
            broadcast_interval,
        }
    }
}

impl Wallet {
    /// Broadcast a tx to random wallets
    fn broadcast(&mut self, _: &mut Context<Self>) {
        let mut rng = &mut rand::thread_rng();
        let sample_set: Vec<_> = self
            .nodes
            .choose_multiple(&mut rng, self.sample_size)
            .cloned()
            .collect();

        let new_tx: usize = rng.gen();
        for node in sample_set {
            node.do_send(Transaction(new_tx));
        }
    }
}

impl Actor for Wallet {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        // Send transactions randomly to nodes
        ctx.run_interval(Duration::from_millis(self.broadcast_interval), Self::broadcast);
    }
}

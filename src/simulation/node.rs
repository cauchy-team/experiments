use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use actix::prelude::*;
use consensus::*;
use futures::{future, prelude::*};
use rand::{seq::SliceRandom, Rng};

pub struct Node {
    entry: Arc<RwLock<Entry>>,
    hash_rate: u64,
    peers: Vec<Addr<Self>>,
    heartbeat: Duration,
    fault_rate: u8,
    sample_size: usize,
}

impl Node {
    pub fn new(hash_rate: u64, heartbeat_ms: u64, fault_rate: u8, sample_size: usize) -> Self {
        let heartbeat = Duration::from_millis(heartbeat_ms);
        let entry = Arc::new(RwLock::new(Entry {
            oddsketch: [0; ODDSKETCH_LEN],
            mass: 0,
        }));
        Node {
            entry,
            hash_rate,
            peers: vec![],
            heartbeat,
            fault_rate,
            sample_size,
        }
    }

    fn work(&self) -> u32 {
        let mut rng = rand::thread_rng();
        let record = (0..self.hash_rate)
            .map(|_| rng.gen_range(0, 2_000_000))
            .max()
            .unwrap();
        record
    }

    fn new_tx(&mut self, index: usize) {
        let mut entry_guard = self.entry.write().unwrap();
        entry_guard.oddsketch[index / 8] ^= 1 << (index % 8);
        entry_guard.mass = self.work();
    }

    fn reconcile(&mut self, ctx: &mut Context<Self>) {
        // Poll random sample of peers
        let mut rng = &mut rand::thread_rng();
        let sample_set: Vec<_> = self
            .peers
            .choose_multiple(&mut rng, self.sample_size)
            .cloned()
            .collect();
        let sampling = sample_set
            .into_iter()
            .map(move |sample| sample.send(EntryRequest).map(move |res| res.ok()));

        // Filter failures
        let responses = future::join_all(sampling).map(move |results| {
            results
                .into_iter()
                .filter_map(move |res| res)
                .filter_map(move |res| res.ok())
        });

        // Find winner
        let entry_inner = self.entry.clone();
        let winner = responses.map(move |responses| {
            let mut entries: Vec<_> = responses.collect();
            entries.push(entry_inner.read().unwrap().clone());
            let winner = calculate_winner_par(&entries).unwrap();
            entries[winner].clone()
        });

        // Reconcile
        let entry_handle = self.entry.clone();
        let reconcile = winner.map(move |entry| *entry_handle.write().unwrap() = entry);
        reconcile.into_actor(self).wait(ctx);
    }
}

impl Actor for Node {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        // Reconcile periodically
        ctx.run_interval(self.heartbeat, Self::reconcile);
    }
}

pub struct EntryRequest;
pub struct ResponseError;

impl Message for EntryRequest {
    type Result = Result<Entry, ResponseError>;
}

impl Handler<EntryRequest> for Node {
    type Result = <EntryRequest as Message>::Result;
    fn handle(&mut self, _: EntryRequest, _: &mut Context<Self>) -> Self::Result {
        // TODO: Random failure
        Ok(self.entry.read().unwrap().clone())
    }
}
pub struct NewPeer(Addr<Node>);

impl Message for NewPeer {
    type Result = ();
}

impl Handler<NewPeer> for Node {
    type Result = ();

    fn handle(&mut self, msg: NewPeer, _: &mut Context<Self>) {
        self.peers.push(msg.0)
    }
}

pub struct NewPeerBatch(pub Vec<Addr<Node>>);

impl Message for NewPeerBatch {
    type Result = ();
}

impl Handler<NewPeerBatch> for Node {
    type Result = ();

    fn handle(&mut self, msg: NewPeerBatch, _: &mut Context<Self>) {
        self.peers.extend(msg.0)
    }
}

pub struct Transaction(pub usize);

impl Message for Transaction {
    type Result = ();
}

impl Handler<Transaction> for Node {
    type Result = ();

    fn handle(&mut self, msg: Transaction, _: &mut Context<Self>) {
        let mut rng = rand::thread_rng();
        let i = rng.gen_range(0, ODDSKETCH_LEN * 8);
        self.new_tx(i);
    }
}

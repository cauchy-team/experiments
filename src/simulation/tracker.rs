use actix::prelude::*;

use super::node::*;

pub struct Tracker {
    nodes: Vec<Addr<Node>>,
}

impl Tracker {
    pub fn new(nodes: Vec<Addr<Node>>) -> Self {
        Tracker { nodes }
    }
}

impl Actor for Tracker {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        // Send addresses to every node
        for i in 0..self.nodes.len() {
            let mut nodes = self.nodes.clone();
            nodes.remove(i);
            self.nodes[i].do_send(NewPeerBatch(nodes));
        }
    }
}

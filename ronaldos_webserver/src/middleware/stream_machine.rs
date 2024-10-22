use actix_web::dev::Url;
use std::{
    collections::{BTreeMap, HashMap},
    net::IpAddr,
};
use uuid::Uuid;

pub struct SinkInfo {
    address: Url,
    max_bandwidth_kb: usize,
    client_size_kb: usize,
}

/// The main goal of this struct is to spread load between sinks. Clients get connected to the node
/// that has the biggest capacity left.
pub struct StreamMachine {
    nodes: HashMap<Uuid, SinkInfo>,
    // a max heap showing the capacity that is left for the given nodes. when nodes get registered
    // a budget is passed for each client connection. For each connected client this value is
    // subtracted from the capacity of the node.
    capacity: BTreeMap<Uuid, usize>,
    clients: HashMap<IpAddr, Uuid>,
}

impl StreamMachine {
    pub fn sink_added(&mut self, info: SinkInfo) -> Uuid {
        let uuid = Uuid::new_v4();
        self.nodes.insert(uuid, info);
        uuid
    }

    pub fn sink_removed(&mut self, address: IpAddr) -> bool {
        todo!()
    }

    pub fn stream_requested(&mut self, client: IpAddr) -> Option<Url> {}

    pub fn client_disconnected(&mut self, client: IpAddr) {
        todo!()
    }

    pub fn connected_clients(&self) -> usize {
        self.clients.len()
    }

    pub fn capacity_left_kb(&self) -> usize {
        self.capacity.values().sum()
    }
}

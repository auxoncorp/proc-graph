use std::{
    collections::HashMap,
    sync::mpsc::{Receiver, Sender},
};

pub struct Network<T> {
    pub(crate) network: HashMap<String, Process<T>>,
}

pub(crate) struct Process<T> {
    pub(crate) name: String,
    pub(crate) senders: HashMap<String, Sender<(String, T)>>,
    pub(crate) receivers: Vec<Receiver<(String, T)>>,
}

impl<T> Process<T> {
    fn run<F>(&self, f: F)
    where
        F: Fn(HashMap<String, T>) -> HashMap<String, T>,
    {
        println!("running process {}", self.name);
        let incoming = HashMap::new();
        for r in self.receivers.iter() {
            let (id, data) = r.recv().expect("shouldn't ever hit a closed-channel case");
            println!("process {} received from {}", self.name, id);
            incoming.insert(id, data);
        }
        let outgoing = f(incoming);
        for (sname, s) in self.senders.iter() {
            let data = outgoing
                .remove(sname)
                .expect("cannot continue, outgoing message has no recipient");
            let _ = s.send((self.name.clone(), data));
        }
    }
}

impl<T> Network<T> {
    pub fn run(self) {
        let handles = Vec::new();
        for (_, proc) in self.network {
            handles.push(std::thread::spawn(move || proc.run()));
        }
        for h in handles.iter() {
            h.join();
        }
    }
}

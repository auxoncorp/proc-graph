use std::{
    collections::HashMap,
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
    time::Duration,
};

use log::debug;

pub struct Network<T, S> {
    procs: HashMap<String, Process<T, S>>,
}

impl<T, S> Network<T, S>
where
    S: Send + 'static,
    T: Default + Send + 'static,
{
    pub fn new() -> Self {
        Network {
            procs: HashMap::new(),
        }
    }

    pub fn add_process<'a, F>(
        &'a mut self,
        name: &'static str,
        adj: Vec<&'static str>,
        initial_state: S,
        body: F,
    ) where
        F: Fn(HashMap<String, T>, S) -> (HashMap<String, T>, S) + Send + 'static,
    {
        let mut proc = {
            if let Some(mut p) = self.procs.remove(name) {
                p.adj = adj.into_iter().map(|a| a.to_string()).collect();
                p.initial_state = Some(initial_state);
                p.body = Some(Box::new(body));
                p
            } else {
                Process {
                    name: name.to_string(),
                    adj: adj.into_iter().map(|a| a.to_string()).collect(),
                    senders: Vec::new(),
                    receivers: Vec::new(),
                    initial_state: Some(initial_state),
                    body: Some(Box::new(body)),
                }
            }
        };
        for adj in proc.adj.iter() {
            let (s, r) = mpsc::channel();
            proc.senders.push(s);
            let recv_proc = {
                if let Some(mut p) = self.procs.remove(adj) {
                    p.receivers.push(r);
                    p
                } else {
                    Process {
                        name: adj.to_string(),
                        adj: Vec::new(),
                        senders: Vec::new(),
                        receivers: vec![r],
                        initial_state: None,
                        body: None,
                    }
                }
            };
            self.procs.insert(adj.clone(), recv_proc);
        }
        self.procs.insert(name.to_string(), proc);
    }

    pub fn start(self) {
        let mut handles = Vec::new();
        for (_, proc) in self.procs.into_iter() {
            handles.push(proc.run());
        }
        for handle in handles {
            handle.join().expect("joining a forever-running thread");
        }
    }
}

pub struct Process<T, S> {
    name: String,
    adj: Vec<String>,
    initial_state: Option<S>,
    body: Option<Box<dyn Fn(HashMap<String, T>, S) -> (HashMap<String, T>, S) + Send + 'static>>,
    senders: Vec<Sender<(String, T)>>,
    receivers: Vec<Receiver<(String, T)>>,
}

impl<T, S> Process<T, S>
where
    S: Send + 'static,
    T: Default + Send + 'static,
{
    pub fn run(self) -> JoinHandle<(HashMap<String, T>, S)> {
        thread::spawn(move || {
            let mut state = self
                .initial_state
                .expect("a graph should be complete before running its processes");
            let body = self
                .body
                .expect("a graph should be complete before running its processes");
            loop {
                debug!("starting the loop for proc {}", self.name);
                thread::sleep(Duration::from_secs(5));
                let mut input = HashMap::new();
                for r in self.receivers.iter() {
                    let (sender, data) = r.recv().expect("should never encounter a closed channel");
                    debug!("{} received from {}", self.name, sender);
                    input.insert(sender, data);
                }
                let (mut output, output_state) = body(input, state);
                state = output_state;
                for (s, adj_name) in self.senders.iter().zip(self.adj.iter()) {
                    debug!("{} is sending to {}", self.name, adj_name);
                    match output.remove(adj_name) {
                        Some(data) => s
                            .send((self.name.clone(), data))
                            .expect("should never encounter a closed channel"),
                        None => s
                            .send((self.name.clone(), T::default()))
                            .expect("should never encounter a closed channel"),
                    };
                }
            }
        })
    }
}

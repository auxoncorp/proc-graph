use std::{
    collections::HashMap,
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

pub struct Network<T> {
    procs: HashMap<String, Process<T>>,
}

impl<T> Network<T>
where
    T: Default + Send + 'static,
{
    pub fn new() -> Self {
        Network {
            procs: HashMap::new(),
        }
    }

    pub fn add_process<'a, F>(&'a mut self, name: &'static str, adj: Vec<&'static str>, body: F)
    where
        F: Fn(HashMap<String, Sender<(String, T)>>, Receiver<(String, T)>) + Send + 'static,
    {
        let mut proc = {
            if let Some(mut p) = self.procs.remove(name) {
                p.adj = adj.into_iter().map(|a| a.to_string()).collect();
                p.body = Some(Box::new(body));
                p
            } else {
                let (s, r) = mpsc::channel();
                Process {
                    name: name.to_string(),
                    adj: adj.into_iter().map(|a| a.to_string()).collect(),
                    senders: HashMap::new(),
                    self_sender: s,
                    receiver: r,
                    body: Some(Box::new(body)),
                }
            }
        };
        for adj in proc.adj.iter() {
            let recv_proc = {
                if let Some(p) = self.procs.remove(adj) {
                    p
                } else {
                    let (s, r) = mpsc::channel();
                    Process {
                        name: adj.to_string(),
                        adj: Vec::new(),
                        senders: HashMap::new(),
                        self_sender: s,
                        receiver: r,
                        body: None,
                    }
                }
            };
            proc.senders
                .insert(adj.to_string(), recv_proc.self_sender.clone());
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
            handle.join().expect("joining running threads");
        }
    }
}

pub struct Process<T> {
    name: String,
    adj: Vec<String>,
    body: Option<
        Box<dyn Fn(HashMap<String, Sender<(String, T)>>, Receiver<(String, T)>) + Send + 'static>,
    >,
    senders: HashMap<String, Sender<(String, T)>>,
    self_sender: Sender<(String, T)>,
    receiver: Receiver<(String, T)>,
}

impl<T> Process<T>
where
    T: Default + Send + 'static,
{
    pub fn run(self) -> JoinHandle<()> {
        thread::spawn(move || {
            let body = self
                .body
                .expect("a graph should be complete before running its processes");
            body(self.senders, self.receiver);
        })
    }
}

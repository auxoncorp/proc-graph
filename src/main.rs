mod graph {
    use std::{
        collections::{HashMap, HashSet},
        fs::File,
        io::Read,
        path::Path,
        sync::mpsc::Receiver,
    };

    use serde::Deserialize;

    #[derive(Deserialize)]
    pub(crate) struct Vertex {
        pub(crate) name: String,
        pub(crate) adj: Vec<String>,
    }

    #[derive(Deserialize)]
    pub(crate) struct Graph {
        vertices: HashMap<String, Vertex>,
        root: String,
    }

    #[derive(Debug)]
    pub(crate) struct Error;

    impl From<std::io::Error> for Error {
        fn from(_: std::io::Error) -> Self {
            Error
        }
    }

    impl From<serde_yaml::Error> for Error {
        fn from(_: serde_yaml::Error) -> Self {
            Error
        }
    }

    impl Graph {
        pub(crate) fn new(root: String, vertices: Vec<Vertex>) -> Result<Self, Error> {
            let vs: HashMap<String, Vertex> =
                vertices.into_iter().map(|v| (v.name.clone(), v)).collect();
            let graph = Graph { root, vertices: vs };
            if graph.vertices.get(&graph.root).is_none() || graph.is_cyclic() {
                return Err(Error);
            }
            Ok(graph)
        }

        pub(crate) fn from_yaml(path: &Path) -> Result<Self, Error> {
            let mut f = File::open(path)?;
            let mut yam_str = String::new();
            f.read_to_string(&mut yam_str)?;
            let graph: Graph = serde_yaml::from_str(&yam_str)?;
            if graph.vertices.get(&graph.root).is_none() || graph.is_cyclic() {
                return Err(Error);
            }
            Ok(graph)
        }

        fn is_cyclic(&self) -> bool {
            is_cyclic_from(
                &self.vertices,
                &mut HashSet::new(),
                self.vertices
                    .get(&self.root)
                    .expect("already checked that the root vertex is present"),
            )
        }

        pub(crate) fn to_network(self) -> super::process::Network {
            let net = super::process::Network {
                network: HashMap::new(),
                finishers: Vec::new(),
            };
            let root = self
                .vertices
                .get(&self.root)
                .expect("already checked that the root vertex is present");
            to_network(&self.vertices, root, net, None)
        }
    }

    fn is_cyclic_from(
        graph: &HashMap<String, Vertex>,
        visited: &mut HashSet<String>,
        vert: &Vertex,
    ) -> bool {
        if visited.contains(&vert.name) {
            return true;
        }
        visited.insert(vert.name.clone());
        for a in vert.adj.iter() {
            let next = graph.get(a).expect("incomplete graph");
            if is_cyclic_from(graph, visited, next) {
                return true;
            }
        }
        visited.remove(&vert.name);
        false
    }

    fn to_network(
        graph: &HashMap<String, Vertex>,
        vert: &Vertex,
        mut procs: super::process::Network,
        rxs: Option<Receiver<String>>,
    ) -> super::process::Network {
        {
            let _ = procs
                .network
                .entry(vert.name.clone())
                .or_insert(super::process::Process {
                    name: vert.name.clone(),
                    receivers: Vec::new(),
                    senders: Vec::new(),
                });
        }
        let mut proc = procs
            .network
            .remove(&vert.name)
            .expect("we put this in right above");
        if let Some(r) = rxs {
            proc.receivers.push(r);
        }
        if vert.adj.is_empty() {
            let (s, r) = std::sync::mpsc::channel();
            proc.senders.push(s);
            procs.finishers.push(r);
        } else {
            for a in vert.adj.iter() {
                let (s, r) = std::sync::mpsc::channel();
                proc.senders.push(s);
                let next = graph.get(a).expect("incomplete graph");
                procs = to_network(graph, &next, procs, Some(r));
            }
        }
        procs.network.insert(vert.name.clone(), proc);
        procs
    }
}

mod process {
    use std::{
        collections::HashMap,
        sync::mpsc::{Receiver, Sender},
    };

    pub(crate) struct Network {
        pub(crate) network: HashMap<String, Process>,
        pub(crate) finishers: Vec<Receiver<String>>,
    }

    pub(crate) struct Process {
        pub(crate) name: String,
        pub(crate) senders: Vec<Sender<String>>,
        pub(crate) receivers: Vec<Receiver<String>>,
    }

    impl Process {
        fn run(&self) {
            println!("running process {}", self.name);
            for r in self.receivers.iter() {
                let from = r.recv().expect("shouldn't ever hit a closed-channel case");
                println!("process {} received from {}", self.name, from);
            }
            for s in self.senders.iter() {
                let _ = s.send(self.name.clone());
            }
        }
    }

    impl Network {
        pub(crate) fn run(self) {
            for (_, proc) in self.network {
                std::thread::spawn(move || proc.run());
            }
            for r in self.finishers {
                let _ = r.recv().expect("should never encounter a closed channel");
            }
        }
    }
}

fn main() {
    use graph::*;
    let g = Graph::new(
        "a".into(),
        vec![
            Vertex {
                name: "a".into(),
                adj: vec!["b".into(), "c".into()],
            },
            Vertex {
                name: "b".into(),
                adj: vec!["d".into()],
            },
            Vertex {
                name: "c".into(),
                adj: vec!["d".into()],
            },
            Vertex {
                name: "d".into(),
                adj: vec![],
            },
        ],
    )
    .expect("graph isn't broken or cyclic");
    g.to_network().run();
}

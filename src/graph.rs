use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::Read,
    path::Path,
    sync::mpsc::{self, Receiver},
};

use serde::Deserialize;

use super::{Network, Process};

#[derive(Clone, Deserialize, PartialEq)]
pub struct Vertex {
    pub(crate) name: String,
    pub(crate) adj: Vec<String>,
}

/// A descriptive directed acyclic graph (DAG) that can be converted
/// into a network of communicating processes.
#[derive(Deserialize)]
pub struct Graph {
    roots: Vec<Vertex>,
    vertices: HashMap<String, Vertex>,
}

/// Errors returned when working with graphs.
#[derive(Debug)]
pub enum Error {
    /// Returned in `Graph::from_yaml` if the YAML file cannot be
    /// found or opened.
    IoError(std::io::Error),
    /// Returned in `Graph::from_yaml` if a graph cannot be
    /// deserialized from YAML.
    YamlError(serde_yaml::Error),
    /// Returned if the graph is incomplete or cyclic.
    BadGraph,
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::IoError(e)
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(e: serde_yaml::Error) -> Self {
        Error::YamlError(e)
    }
}

impl Graph {
    /// Construct a graph from a list of vertices.
    pub fn new(vs: Vec<Vertex>) -> Result<Self, Error> {
        let mut roots = vs.clone();
        let vertices: HashMap<String, Vertex> =
            vs.into_iter().map(|v| (v.name.clone(), v)).collect();
        let mut reachable = HashSet::new();
        for v in vertices.values() {
            if is_cyclic_or_incomplete_from(&vertices, &mut HashSet::new(), &mut reachable, v) {
                return Err(Error::BadGraph);
            }
        }

        // Take the set difference between all and `reachable`,
        // leaving us with our roots.
        roots.drain_filter(|v| reachable.iter().fold(false, |t, r| t || &v.name == r));
        Ok(Graph { roots, vertices })
    }

    /// Construct a graph from a YAML list of vertices.
    pub fn from_yaml(path: &Path) -> Result<Self, Error> {
        let mut f = File::open(path)?;
        let mut yam_str = String::new();
        f.read_to_string(&mut yam_str)?;
        let vertices: Vec<Vertex> = serde_yaml::from_str(&yam_str)?;
        Graph::new(vertices)
    }

    /// Convert the descriptive graph to a network of processes.
    pub fn to_network(self) -> Network {
        let mut net = Network {
            network: HashMap::new(),
        };
        for (name, vert) in self.vertices.iter() {
            for adj in vert.adj.iter() {
                let (s, r) = mpsc::channel();
                // Senders (this vertex).
                if let Some(mut proc) = net.network.remove(name) {
                    proc.senders.push(s);
                    net.network.insert(name.clone(), proc);
                } else {
                    net.network.insert(
                        name.clone(),
                        Process {
                            name: name.clone(),
                            senders: vec![s],
                            receivers: Vec::new(),
                        },
                    );
                }

                // Receivers (the adjacent vertex).
                if let Some(mut a_proc) = net.network.remove(adj) {
                    a_proc.receivers.push(r);
                    net.network.insert(adj.clone(), a_proc);
                } else {
                    net.network.insert(
                        adj.clone(),
                        Process {
                            name: adj.clone(),
                            senders: Vec::new(),
                            receivers: vec![r],
                        },
                    );
                }
            }
        }
        net
    }

    fn is_cyclic_or_incomplete(&self) -> bool {
        let mut reachable = HashSet::new();
        for v in self.vertices.values() {
            if is_cyclic_or_incomplete_from(&self.vertices, &mut HashSet::new(), &mut reachable, v)
            {
                return true;
            }
        }
        false
    }
}

// This does a DFS over the graph from the given vertex. If, while
// discovering a path, the search returns to an already visited
// vertex, we've found a cycle.
fn is_cyclic_or_incomplete_from(
    graph: &HashMap<String, Vertex>,
    visited: &mut HashSet<String>,
    reachable: &mut HashSet<String>,
    vert: &Vertex,
) -> bool {
    if visited.contains(&vert.name) {
        return true;
    }
    visited.insert(vert.name.clone());
    reachable.insert(vert.name.clone());
    for a in vert.adj.iter() {
        if let Some(next) = graph.get(a) {
            if is_cyclic_or_incomplete_from(graph, visited, reachable, next) {
                return true;
            }
        } else {
            return false;
        }
    }
    visited.remove(&vert.name);
    false
}

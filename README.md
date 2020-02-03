# proc-graph

## Overview

A small library which converts a process graph into a set of
communicating processes.

## Getting Started

```toml
# Cargo.toml

[dependencies]
proc-graph = "0.1"
```

## Usage

```rust
use std::{thread, time::Duration};

use proc_graph::Network;

fn main() {
    env_logger::init();

    let mut net = Network::new();

    net.add_process("a", vec!["b", "c"], |senders, _| loop {
        thread::sleep(Duration::from_secs(1));
        for (adj, s) in senders.iter() {
            println!("a is sending to {}", adj);
            s.send(("a".to_string(), ()))
                .expect("shouldn't encounter a closed channel");
        }
    });

    net.add_process("b", vec!["d"], |senders, receiver| loop {
        thread::sleep(Duration::from_secs(1));
        let (sender, _) = receiver
            .recv()
            .expect("shouldn't encounter a closed channel");
        println!("b received from {}", sender);
        for s in senders.values() {
            s.send(("b".to_string(), ()))
                .expect("shouldn't encounter a closed channel");
        }
    });

    net.add_process("c", vec!["d"], |senders, receiver| loop {
        thread::sleep(Duration::from_secs(1));
        let (sender, _) = receiver
            .recv()
            .expect("shouldn't encounter a closed channel");
        println!("c received from {}", sender);
        for s in senders.values() {
            s.send(("c".to_string(), ()))
                .expect("shouldn't encounter a closed channel");
        }
    });

    net.add_process("d", vec![], |_, receiver| loop {
        thread::sleep(Duration::from_secs(1));
        let (sender, _) = receiver
            .recv()
            .expect("shouldn't encounter a closed channel");
        println!("d received from {}", sender);
    });

    net.start();
}
```

## License

`proc-graph` is licensed under the MIT License (MIT) unless otherwise
noted. Please see [LICENSE](./LICENSE) for more details.


use std::collections::HashMap;

use log::info;

use imdb::Network;

fn main() {
    env_logger::init();

    let mut net = Network::new();

    net.add_process("a", vec!["b", "c"], (), |_: HashMap<_, ()>, _| {
        info!("hello from inside the body of a");
        let mut outgoing = HashMap::new();
        outgoing.insert("b".to_string(), ());
        outgoing.insert("c".to_string(), ());
        (outgoing, ())
    });

    net.add_process("b", vec!["d"], (), |_, _| {
        info!("hello from inside the body of b");
        let mut outgoing = HashMap::new();
        outgoing.insert("d".to_string(), ());
        (outgoing, ())
    });

    net.add_process("c", vec!["d"], (), |_, _| {
        info!("hello from inside the body of c");
        let mut outgoing = HashMap::new();
        outgoing.insert("d".to_string(), ());
        (outgoing, ())
    });

    net.add_process("d", Vec::new(), (), |_, _| {
        info!("hello from inside the body of d");
        (HashMap::new(), ())
    });

    net.start();
}

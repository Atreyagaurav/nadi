use std::process::exit;

mod network;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 {
        eprintln!("Usage: {} connections_file", args[0]);
        exit(1);
    }
    let mut net = network::Network::from_file(&args[1]);
    if args.len() == 3 {
        net.set_node_template(&args[2]);
    }
    net.graph_print();
    // net.graph_print_dot();
}

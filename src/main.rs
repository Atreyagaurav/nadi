use std::process::exit;

mod network;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 {
        eprintln!("Usage: {} connections_file", args[0]);
        exit(1);
    }
    let net = network::Network::from_file(&args[1]);
    net.graph_print_dot();
}

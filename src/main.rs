mod network;

fn main() {
    let net = network::Network::from_file("example/jpt.txt");
    net.graph_print();
}

mod network;

fn main() {
    let net = network::Network::from_file("example/ohio.txt");
    net.graph_print();
}

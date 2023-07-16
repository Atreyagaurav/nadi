use anyhow;
use std::collections::VecDeque;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

#[derive(Clone)]
pub enum NodeAttr {
    String(String),
    Number(usize),
    Vec(Vec<usize>),
    Value(f32),
    Timeseries(Vec<f32>),
}

impl fmt::Display for NodeAttr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NodeAttr::String(s) => write!(f, "{}", s),
            NodeAttr::Number(n) => write!(f, "{}", n),
            NodeAttr::Vec(v) => write!(f, "{:?}", v),
            NodeAttr::Value(v) => write!(f, "{:.2}", v),
            NodeAttr::Timeseries(t) => write!(f, "{:?}", t),
        }
    }
}

impl NodeAttr {
    pub fn string(val: impl ToString) -> Self {
        Self::String(val.to_string())
    }

    pub fn number(val: impl Into<usize>) -> Self {
        Self::Number(val.into())
    }

    pub fn vec(val: impl Into<Vec<usize>>) -> Self {
        Self::Vec(val.into())
    }

    pub fn value(val: impl Into<f32>) -> Self {
        Self::Value(val.into())
    }

    pub fn timseries(val: impl Into<Vec<f32>>) -> Self {
        Self::Timeseries(val.into())
    }

    pub fn read_string(&self) -> Option<&str> {
        if let Self::String(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn read_number(&self) -> Option<&usize> {
        if let Self::Number(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn read_vec(&self) -> Option<&Vec<usize>> {
        if let Self::Vec(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn read_value(&self) -> Option<&f32> {
        if let Self::Value(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn read_timeseries(&self) -> Option<&Vec<f32>> {
        if let Self::Timeseries(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

pub enum NodeTemplate {
    Attr(String),
    Lit(String),
}

#[derive(Default)]
struct GraphNode {
    pre: usize,
    post: usize,
    merge: bool,
    text: String,
}

#[derive(Clone)]
pub struct Node {
    index: usize,
    name: String,
    inputs: Vec<usize>,
    output: Option<usize>,
    attrs: HashMap<String, NodeAttr>,
}

impl Node {
    pub fn new(index: usize, name: String, inputs: Vec<usize>, output: Option<usize>) -> Self {
        let mut node = Self {
            index,
            name: name.clone(),
            inputs: inputs.clone(),
            output,
            attrs: HashMap::new(),
        };
        node.set_attr("name", NodeAttr::string(name));
        node.set_attr("index", NodeAttr::number(index));
        node.set_attr("inputs", NodeAttr::vec(inputs));
        node
    }

    pub fn set_inputs(&mut self, inputs: Vec<usize>) {
        self.inputs = inputs.clone();
        self.set_attr("inputs", NodeAttr::vec(inputs));
    }

    pub fn set_output(&mut self, output: usize) {
        self.output = Some(output);
        self.set_attr("output", NodeAttr::number(output));
    }

    pub fn set_index(&mut self, index: usize) {
        self.index = index;
        self.set_attr("index", NodeAttr::number(index));
    }

    pub fn get_index(&self) -> usize {
        self.index
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_attr(&self, key: &str) -> Option<&NodeAttr> {
        self.attrs.get(key)
    }

    pub fn load_attrs_from_file(&mut self, filename: PathBuf) -> anyhow::Result<()> {
        let file = File::open(&filename)?;
        let reader_lines = BufReader::new(file).lines();
        for line in reader_lines {
            let line = line?.trim().to_string();
            if line.starts_with("#") || line == "" {
                continue;
            }
            if let Some((key, val)) = line.split_once("=") {
                let val = val.trim();
                if let Ok(n) = val.parse::<usize>() {
                    self.set_attr(key.trim(), NodeAttr::number(n));
                } else if let Ok(n) = val.parse::<f32>() {
                    self.set_attr(key.trim(), NodeAttr::value(n));
                } else {
                    self.set_attr(key.trim(), NodeAttr::string(val.trim()));
                }
            }
        }
        Ok(())
    }

    pub fn get_attr_repr(&self, key: &str) -> String {
        self.attrs
            .get(key)
            .map(|a| a.to_string())
            .unwrap_or("".to_string())
    }

    pub fn set_attr(&mut self, key: &str, val: NodeAttr) {
        self.attrs.insert(key.to_string(), val);
    }

    pub fn format(&self, template: &Vec<NodeTemplate>) -> String {
        let mut repr = String::new();
        for tmpl in template {
            match tmpl {
                NodeTemplate::Lit(s) => repr.push_str(&s),
                NodeTemplate::Attr(s) => repr.push_str(&self.get_attr_repr(&s)),
            }
        }
        repr
    }
}

pub struct Network {
    pub indices: HashMap<String, usize>,
    pub nodes: Vec<Node>,
    pub node_template: Vec<NodeTemplate>,
}

impl Network {
    pub fn from_file(filename: &str) -> Self {
        let mut indices: HashMap<String, usize> = HashMap::new();
        let mut inputs: Vec<Vec<usize>> = Vec::new();
        let mut output_map: HashMap<usize, usize> = HashMap::new();
        let filename = PathBuf::from(filename);
        let file = File::open(&filename).unwrap();
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line.unwrap().trim().to_string();
            if line == "" || line.starts_with("#") {
                continue;
            }
            if let Some((inp, out)) = line.split_once("->") {
                let inp = inp.trim();
                let out = out.trim();
                // eprintln!("{} {}", inp, out);
                if !indices.contains_key(inp) {
                    indices.insert(inp.to_string(), indices.len());
                    inputs.push(Vec::new());
                }
                if !indices.contains_key(out) {
                    indices.insert(out.to_string(), indices.len());
                    inputs.push(Vec::new());
                }
                output_map.insert(indices[inp], indices[out]);
                inputs[indices[out]].push(indices[inp])
            } else {
                if !indices.contains_key(&line) {
                    indices.insert(line.to_string(), indices.len());
                    inputs.push(Vec::new());
                }
            }
        }

        let names: HashMap<usize, String> =
            indices.clone().into_iter().map(|(k, v)| (v, k)).collect();
        let nodes_attrs_dir = filename
            .parent()
            .unwrap_or(&PathBuf::from("."))
            .join("nodes/");
        let nodes: Vec<Node> = inputs
            .into_iter()
            .enumerate()
            .map(|(i, input)| {
                let mut n = Node::new(i, names[&i].clone(), input, output_map.get(&i).copied());
                n.load_attrs_from_file(nodes_attrs_dir.join(format!("{}.txt", n.name)))
                    .ok();
                n
            })
            .collect::<Vec<Node>>();
        let node_template = vec![
            // NodeTemplate::Attr("stn".to_string()),
            // NodeTemplate::Lit("[".to_string()),
            NodeTemplate::Attr("index".to_string()),
            // NodeTemplate::Lit(":".to_string()),
            // NodeTemplate::Attr("order".to_string()),
            // NodeTemplate::Lit(".".to_string()),
            // NodeTemplate::Attr("level".to_string()),
            // NodeTemplate::Lit("] ".to_string()),
            // NodeTemplate::Attr("name".to_string()),
            // NodeTemplate::Lit(" (".to_string()),
            // NodeTemplate::Attr("obs_7q10".to_string()),
            // NodeTemplate::Lit(" cfs; ".to_string()),
            // NodeTemplate::Attr("nat_7q10".to_string()),
            // NodeTemplate::Lit(" cfs) ".to_string()),
            // NodeTemplate::Attr("inputs".to_string()),
        ];
        let mut net = Self {
            indices,
            nodes,
            node_template,
        };
        net.order();
        net.reindex();
        net
    }

    pub fn order(&mut self) {
        let mut all_nodes: HashSet<usize> = (0..self.nodes.len()).collect();
        let mut order_queue: Vec<usize> = Vec::with_capacity(self.nodes.len());
        loop {
            if all_nodes.len() == 0 && order_queue.len() == 0 {
                break;
            }

            if order_queue.len() == 0 {
                let elem = all_nodes.iter().next().unwrap().clone();
                order_queue.push(elem);
                all_nodes.remove(&elem);
            }

            let n = order_queue.pop().unwrap();
            let node: &Node = &self.nodes[n];
            if node.inputs.len() == 0 {
                self.nodes[n].set_attr("order", NodeAttr::Number(1));
            } else {
                let uncalc_inputs: Vec<&usize> = node
                    .inputs
                    .iter()
                    .filter(|i| all_nodes.contains(&i))
                    .collect();
                if uncalc_inputs.len() > 0 {
                    order_queue.push(n);
                    uncalc_inputs.iter().for_each(|i| {
                        order_queue.push(**i);
                        all_nodes.remove(&i);
                    });
                } else {
                    let ord: usize = node
                        .inputs
                        .iter()
                        .map(|n| {
                            self.nodes[*n]
                                .get_attr("order")
                                .unwrap()
                                .read_number()
                                .unwrap()
                        })
                        .sum();
                    self.nodes[n].set_attr("order", NodeAttr::number(ord + 1));
                }
            }
        }
    }

    pub fn reindex(&mut self) {
        if self.nodes.is_empty() {
            return;
        }
        // find the most downstream point
        let mut output = 0;
        loop {
            if let Some(out) = self.nodes[output].output {
                output = out
            } else {
                break;
            }
        }

        let mut nodes: Vec<(usize, usize)> = Vec::new();
        let mut all_nodes: HashSet<usize> = (0..self.nodes.len()).collect();
        let mut curr_nodes: VecDeque<(usize, usize)> = VecDeque::from([(output, 0)]);
        loop {
            if curr_nodes.len() == 0 {
                if all_nodes.len() == 0 {
                    break;
                } else {
                    let elem = all_nodes.iter().next().unwrap().clone();
                    curr_nodes.push_back((elem, 0));
                    all_nodes.remove(&elem);
                }
            }
            let (n, level): (usize, usize) = curr_nodes.pop_front().unwrap();
            nodes.push((n, level));
            all_nodes.remove(&n);
            if !self.nodes[n].inputs.is_empty() {
                let orders: Vec<usize> = self
                    .nodes
                    .iter()
                    .map(|n| *n.get_attr("order").unwrap().read_number().unwrap())
                    .collect();
                self.nodes[n]
                    .inputs
                    .sort_by(|n1, n2| orders[*n1].cmp(&orders[*n2]));
                self.nodes[n].inputs.reverse();
                for &inp in self.nodes[n].inputs.iter() {
                    let level = if inp == self.nodes[n].inputs[0] {
                        level
                    } else {
                        level + 1
                    };
                    curr_nodes.push_back((inp, level));
                    all_nodes.remove(&inp);
                }
            }
        }

        let inputs_map: HashMap<usize, usize> =
            nodes.iter().enumerate().map(|(i, n)| (n.0, i)).collect();
        let mut new_nodes: Vec<Node> = nodes.iter().map(|n| self.nodes[n.0].clone()).collect();
        new_nodes.iter_mut().enumerate().for_each(|(i, n)| {
            n.set_index(i);
            n.set_inputs(n.inputs.iter().map(|i| inputs_map[i]).collect());
            if let Some(out) = n.output {
                n.set_output(inputs_map[&out]);
            }
            n.set_attr("level", NodeAttr::number(nodes[i].1))
        });
        let new_indices = new_nodes
            .iter()
            .map(|n| (n.name.clone(), n.index))
            .collect();
        self.indices = new_indices;
        self.nodes = new_nodes;
    }

    pub fn simple_print(&self) {
        for node in &self.nodes {
            println!("{}", node.format(&self.node_template));
        }
    }

    pub fn graph_print(&self) {
        if self.nodes.len() == 0 {
            return;
        }

        let mut graph_nodes: Vec<GraphNode> = Vec::new();
        let mut all_nodes: HashSet<usize> = (1..self.nodes.len()).collect();
        let mut curr_nodes: Vec<usize> = vec![0];
        loop {
            if curr_nodes.len() == 0 {
                if all_nodes.len() == 0 {
                    break;
                } else {
                    eprint!("Error");
                    let elem = all_nodes.iter().next().unwrap().clone();
                    curr_nodes.push(elem);
                    all_nodes.remove(&elem);
                }
            }
            let mut gnd = GraphNode::default();
            let n = curr_nodes.pop().unwrap();
            let node = &self.nodes[n];
            gnd.text = node.format(&self.node_template);

            let level = *node.get_attr("level").unwrap().read_number().unwrap();
            let par_level = *self.nodes[node.output.unwrap_or(node.index)]
                .get_attr("level")
                .unwrap()
                .read_number()
                .unwrap();
            gnd.pre = level;
            gnd.post = 0;
            gnd.merge = level != par_level;
            graph_nodes.push(gnd);

            // println!("{} {}", prefix, node.format(&self.node_template));
            // println!("{} {}", prefix, node.format(&self.node_template));
            for &inp in node.inputs.iter() {
                if all_nodes.contains(&inp) {
                    curr_nodes.push(inp);
                    all_nodes.remove(&inp);
                }
            }
        }
        let graph_text: Vec<String> = graph_nodes
            .iter()
            .rev()
            .map(|gnd| {
                let mut graph_cmps = String::new();
                for _ in 0..gnd.pre {
                    graph_cmps.push_str(" |");
                }
                if gnd.merge {
                    graph_cmps.pop();
                    graph_cmps.push('+');
                }
                graph_cmps.push_str(if gnd.merge { "-*" } else { " *" });
                for _ in 0..gnd.post {
                    graph_cmps.push_str(" |");
                }
                graph_cmps
            })
            .collect();
        let max_width = graph_text.iter().map(|gt| gt.len()).max().unwrap_or(10);
        graph_text
            .iter()
            .zip(graph_nodes.iter().rev())
            .for_each(|(pre, gnd)| {
                println!("{1:0$}  {2}", max_width, pre, gnd.text);
                for _ in 0..(gnd.pre + if gnd.merge { 0 } else { 1 }) {
                    print!(" |");
                }
                for _ in 0..gnd.post {
                    print!(" |");
                }
                println!("");
            })
    }

    pub fn graph_print_dot(&self) {
        if self.nodes.len() == 0 {
            return;
        }

        // Node index, x and y
        let mut graph_nodes: Vec<(usize, usize, usize)> = Vec::new();
        let mut all_nodes: HashSet<usize> = (1..self.nodes.len()).collect();
        let mut curr_nodes: Vec<usize> = vec![0];
        loop {
            if curr_nodes.len() == 0 {
                if all_nodes.len() == 0 {
                    break;
                } else {
                    eprint!("Error");
                    let elem = all_nodes.iter().next().unwrap().clone();
                    curr_nodes.push(elem);
                    all_nodes.remove(&elem);
                }
            }
            let n = curr_nodes.pop().unwrap();
            let node = &self.nodes[n];
            let level = *node.get_attr("level").unwrap().read_number().unwrap();
            graph_nodes.push((n, level, graph_nodes.len()));

            for &inp in node.inputs.iter() {
                if all_nodes.contains(&inp) {
                    curr_nodes.push(inp);
                    all_nodes.remove(&inp);
                }
            }
        }
        let max_x = graph_nodes.iter().map(|(_, x, _)| x).max().unwrap();

        println!("digraph network {{");
        println!(" overlap=true;");
        println!(" node [shape=circle,fixedsize=false];");

        for (n, x, y) in &graph_nodes {
            let node = &self.nodes[*n];
            let par = node.output.map(|o| self.nodes[o].index);
            let text = node.format(&self.node_template);
            println!(
                "{} [pos=\"{},{}!\", size=30, fixedsize=true]",
                node.index, x, y
            );
            println!(
                "l{} [shape=plain,pos=\"{},{}!\", label=\"{}\",fontsize=42]",
                node.index,
                max_x + 1,
                y,
                text
            );
            println!("{0} -> l{0} [color=none]", node.index);
            if let Some(par) = par {
                println!("{} -> {}", node.index, par);
            }
        }
        println!("}}");
    }
}

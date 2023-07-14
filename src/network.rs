use std::fs::File;
use std::io::{BufRead, BufReader};
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug)]
pub enum NodeAttr {
    String(String),
    Number(usize),
    Value(f32),
    Timeseries(Vec<f32>),
}

pub enum NodeTemplate {
    Attr(String),
    Lit(String),
}

pub struct Node {
    index: usize,
    pub name: String,
    attrs: HashMap<String, NodeAttr>,
}

impl Node {
    pub fn new(index: usize, name: String) -> Self {
        let mut node = Self {
            index,
            name: name.clone(),
            attrs: HashMap::new(),
        };
        node.set_attr("name", NodeAttr::String(name));
        node.set_attr("index", NodeAttr::Number(index));
        node
    }

    pub fn get_attr(&self, key: &str) -> Option<&NodeAttr> {
        self.attrs.get(key)
    }

    pub fn set_attr(&mut self, key: &str, val: NodeAttr) {
        self.attrs.insert(key.to_string(), val);
    }

    pub fn format(&self, template: &Vec<NodeTemplate>) -> String {
        let mut repr = String::new();
        for tmpl in template {
            match tmpl {
                NodeTemplate::Lit(s) => repr.push_str(&s),
                NodeTemplate::Attr(s) => repr.push_str(
                    &self
                        .get_attr(&s)
                        .map(|a| format!("{:?}", a))
                        .unwrap_or("Err".to_string()),
                ),
            }
        }
        repr
    }
}

pub struct Network {
    indices: HashMap<String, usize>,
    nodes: Vec<Node>,
    node_template: Vec<NodeTemplate>,
}

impl Network {
    pub fn from_file(filename: &str) -> Self {
        let mut indices: HashMap<String, usize> = HashMap::new();
        let mut inputs: Vec<Vec<usize>> = Vec::new();
        let mut output_map: HashMap<usize, usize> = HashMap::new();

        let file = File::open(PathBuf::from(filename)).unwrap();
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
                    eprintln!("{}: {}", indices.len(), inp);
                    indices.insert(inp.to_string(), indices.len());
                    inputs.push(Vec::new());
                }
                if !indices.contains_key(out) {
                    eprintln!("{}: {}", indices.len(), out);
                    indices.insert(out.to_string(), indices.len());
                    inputs.push(Vec::new());
                }
                output_map.insert(indices[inp], indices[out]);
                inputs[indices[out]].push(indices[inp])
            } else {
                if !indices.contains_key(&line) {
                    eprintln!("{}: {}", indices.len(), line);
                    indices.insert(line.to_string(), indices.len());
                    inputs.push(Vec::new());
                }
            }
        }

        let names: HashMap<usize, String> =
            indices.clone().into_iter().map(|(k, v)| (v, k)).collect();
        let nodes: Vec<Node> = (0..indices.len())
            .map(|i| Node::new(i, names[&i].clone()))
            .collect::<Vec<Node>>();
        let node_template = vec![
            NodeTemplate::Lit("Node [".to_string()),
            NodeTemplate::Attr("index".to_string()),
            NodeTemplate::Lit("] : ".to_string()),
            NodeTemplate::Attr("name".to_string()),
        ];
        Self {
            indices,
            nodes,
            node_template,
        }
    }

    pub fn simple_print(&self) {
        for node in &self.nodes {
            println!("{}", node.format(&self.node_template));
        }
    }
}

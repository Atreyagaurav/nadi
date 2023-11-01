use anyhow::{Context, Error};
use clap::{Args, ValueEnum};
use std::collections::VecDeque;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};
use string_template_plus::{Render, RenderOptions, Template};

use crate::cliargs::CliAction;

#[derive(Args)]
pub struct CliArgs {
    /// graphviz format
    #[arg(short, long, action)]
    graphviz: bool,
    /// Direction to move while making the graph
    #[arg(
        short,
        long,
        rename_all = "lower",
        default_value = "b",
        value_enum,
        requires = "graphviz"
    )]
    direction: GraphVizDirection,
    /// Shape of the node
    #[arg(short = 'S', long, requires = "graphviz", default_value = "circle")]
    node_shape: String,
    /// Shape of the label
    #[arg(short = 'O', long, requires = "graphviz", default_value = "1")]
    node_offset: f64,
    /// Shape of the label
    #[arg(short = 'A', long, requires = "graphviz", default_value = "plain")]
    label_shape: String,
    /// Shape of the label
    #[arg(short = 'o', long, requires = "graphviz", default_value = "1")]
    label_offset: f64,
    /// size of the node
    #[arg(short = 'N', long, requires = "graphviz", default_value = "30")]
    node_size: usize,
    /// Template for the text inside the circle of nodes
    #[arg(short, long, requires = "graphviz", default_value = "{index}", value_parser=Template::parse_template)]
    node_template: Template,
    /// URL Template for Node URL
    #[arg(short, long, default_value = "", value_parser=Template::parse_template)]
    url_template: Template,
    /// Template for Node Label
    #[arg(short, long, default_value = "{index}", value_parser=Template::parse_template)]
    label_template: Template,
    /// Cumulate the attribute (as float) based on the connections
    #[arg(short, long, value_delimiter = ',')]
    cumulate: Vec<String>,
    /// Latex table header and template
    #[arg(short = 'L', long, conflicts_with = "graphviz", value_parser=parse_latex_table, value_delimiter=';')]
    latex_table: Vec<(String, Template)>,
    /// Simply print the node and attributes from the template
    #[arg(short = 'D', long, conflicts_with = "graphviz")]
    debug_print: bool,
    /// Sort by this attribute
    #[arg(short, long)]
    sort_by: Option<String>,
    /// Connection file
    connection_file: PathBuf,
}

fn parse_latex_table(arg: &str) -> Result<(String, Template), Error> {
    let (head, templ) = arg
        .split_once(':')
        .context("Header should have a template followed")?;
    Ok((head.to_string(), Template::parse_template(templ)?))
}
// TODO make HashMap CLI args with graph attr, node_attr, label_attr,
// edge_attr etc that can be looped through and then used for the dot
// generation. It will be more flexible and easier to make than adding
// each option one by one. (We can also remove the label attr one
// honestly, remove the label totally.)

// Also make anek link type on emacs, that I can use for other stuff
// as well. The link type will use the anek template to open the
// links. I can make it easy to change link template so the same link
// can work to open multiple files for me.
pub struct GraphVizSettings<'a> {
    direction: &'a GraphVizDirection,
    sort_by: &'a Option<String>,
    node_shape: &'a str,
    node_offset: f64,
    label_shape: &'a str,
    label_offset: f64,
    node_size: usize,
    templates: Templates<'a>,
}

impl<'a> GraphVizSettings<'a> {
    fn new(args: &'a CliArgs, templates: Templates<'a>) -> Self {
        Self {
            direction: &args.direction,
            sort_by: &args.sort_by,
            node_shape: &args.node_shape,
            node_offset: args.node_offset,
            label_shape: &args.label_shape,
            label_offset: args.label_offset,
            node_size: args.node_size,
            templates,
        }
    }
}

#[derive(Clone)]
struct Templates<'a> {
    node: &'a Template,
    label: &'a Template,
    url: &'a Template,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum GraphVizDirection {
    #[value(alias = "tb", alias = "b")]
    TopToBottom,
    #[value(alias = "bt", alias = "t")]
    BottomToTop,
    #[value(alias = "rl", alias = "l")]
    RightToLeft,
    #[value(alias = "rl", alias = "r")]
    LeftToRight,
}

impl CliAction for CliArgs {
    fn run(self) -> anyhow::Result<()> {
        let templ = Templates {
            node: &self.node_template,
            label: &self.label_template,
            url: &self.url_template,
        };
        let mut net = Network::from_file(&self.connection_file);
        net.cumulate(&self.cumulate)?;
        if self.debug_print {
            net.simple_print(&templ.label);
        } else if self.graphviz {
            let settings = GraphVizSettings::new(&self, templ);
            net.graph_print_dot(&settings);
        } else if !self.latex_table.is_empty() {
            net.generate_latex_table(&self.latex_table, &templ.url);
        } else {
            net.graph_print(&templ.label);
        }
        Ok(())
    }
}

#[derive(Clone)]
pub enum NodeAttr {
    String(String),
    Number(usize),
    Vec(Vec<usize>),
    Value(f32),
}

impl fmt::Display for NodeAttr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NodeAttr::String(s) => write!(f, "{}", s),
            NodeAttr::Number(n) => write!(f, "{}", n),
            NodeAttr::Vec(v) => write!(f, "{:?}", v),
            NodeAttr::Value(v) => write!(f, "{}", v),
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

    pub fn read_value(&self) -> Option<f32> {
        match self {
            Self::Value(v) => Some(*v),
            Self::Number(i) => Some(*i as f32),
            _ => None,
        }
    }
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
    render_ops: RenderOptions,
}

impl Node {
    pub fn new(
        index: usize,
        name: String,
        inputs: Vec<usize>,
        output: Option<usize>,
        wd: PathBuf,
    ) -> Self {
        let mut node = Self {
            index,
            name: name.clone(),
            inputs: inputs.clone(),
            output,
            attrs: HashMap::new(),
            render_ops: RenderOptions {
                wd,
                variables: HashMap::new(),
                shell_commands: false,
            },
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
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            if let Some((key, val)) = line.split_once('=') {
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
        self.render_ops
            .variables
            .insert(key.to_string(), val.to_string());
        self.attrs.insert(key.to_string(), val);
    }

    pub fn format(&self, template: &Template) -> String {
        template.render(&self.render_ops).unwrap()
    }
}

#[derive(Clone)]
pub struct Network {
    pub indices: HashMap<String, usize>,
    pub nodes: Vec<Node>,
}

fn insert_ifnot_node(
    indices: &mut HashMap<String, usize>,
    inputs: &mut Vec<Vec<usize>>,
    inp: &str,
) {
    if !indices.contains_key(inp) {
        indices.insert(inp.to_string(), indices.len());
        inputs.push(Vec::new());
    }
}

impl Network {
    pub fn from_file(filename: &PathBuf) -> Self {
        // first read the file contents and fill the node indices,
        // inputs and outputs for those nodes.
        let mut indices: HashMap<String, usize> = HashMap::new();
        let mut inputs: Vec<Vec<usize>> = Vec::new();
        let mut output_map: HashMap<usize, usize> = HashMap::new();
        let file = File::open(filename).unwrap();
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line.unwrap().trim().to_string();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((inp, out)) = line.split_once("->") {
                let inp = inp.trim();
                let out = out.trim();
                // eprintln!("{} {}", inp, out);
                insert_ifnot_node(&mut indices, &mut inputs, inp);
                insert_ifnot_node(&mut indices, &mut inputs, out);
                output_map.insert(indices[inp], indices[out]);
                inputs[indices[out]].push(indices[inp])
            } else {
                insert_ifnot_node(&mut indices, &mut inputs, &line);
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
                let mut n = Node::new(
                    i,
                    names[&i].clone(),
                    input,
                    output_map.get(&i).copied(),
                    filename
                        .parent()
                        .unwrap_or(&PathBuf::from("."))
                        .to_path_buf(),
                );
                n.load_attrs_from_file(nodes_attrs_dir.join(format!("{}.txt", n.name)))
                    .ok();
                n.load_attrs_from_file(nodes_attrs_dir.join(format!("{}", n.name)))
                    .ok();
                n
            })
            .collect::<Vec<Node>>();
        let mut net = Self { indices, nodes };
        net.order();
        net.reindex();
        net
    }

    pub fn order(&mut self) {
        let mut all_nodes: HashSet<usize> = (0..self.nodes.len()).collect();
        let mut order_queue: Vec<usize> = Vec::with_capacity(self.nodes.len());
        loop {
            if all_nodes.is_empty() && order_queue.is_empty() {
                break;
            }

            if order_queue.is_empty() {
                let elem = *all_nodes.iter().next().unwrap();
                order_queue.push(elem);
                all_nodes.remove(&elem);
            }

            let n = order_queue.pop().unwrap();
            let node: &Node = &self.nodes[n];
            if node.inputs.is_empty() {
                self.nodes[n].set_attr("order", NodeAttr::Number(1));
            } else {
                let uncalc_inputs: Vec<&usize> = node
                    .inputs
                    .iter()
                    .filter(|i| all_nodes.contains(i))
                    .collect();
                if !uncalc_inputs.is_empty() {
                    order_queue.push(n);
                    uncalc_inputs.iter().for_each(|i| {
                        order_queue.push(**i);
                        all_nodes.remove(i);
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

    pub fn cumulate(&mut self, variables: &Vec<String>) -> Result<(), Error> {
        if self.nodes.is_empty() {
            return Ok(());
        }
        for var in variables {
            let mut values: HashMap<&str, f32> = HashMap::new();
            let cl = self.clone();
            let (var, safe): (&str, bool) = if var.ends_with('?') {
                (&var[..(var.len() - 1)], true)
            } else {
                (var.as_str(), false)
            };
            get_values(&cl, var, safe, &mut values)?;
            for node in &cl.nodes {
                let val = *values.get(node.get_name()).unwrap();
                let mut out = node.output;
                loop {
                    if let Some(o) = out {
                        let mut v = *values.get(cl.nodes[o].get_name()).unwrap();
                        v += val;
                        values.insert(cl.nodes[o].get_name(), v);
                        out = cl.nodes[o].output;
                    } else {
                        break;
                    }
                }
            }
            set_values(self, var, &values);
        }

        Ok(())
    }

    pub fn reindex(&mut self) {
        if self.nodes.is_empty() {
            return;
        }
        // find the most downstream point
        let mut output = 0;
        while let Some(out) = self.nodes[output].output {
            output = out
        }

        let mut nodes: Vec<(usize, usize)> = Vec::new();
        let mut all_nodes: HashSet<usize> = (0..self.nodes.len()).collect();
        let mut curr_nodes: VecDeque<(usize, usize)> = VecDeque::from([(output, 0)]);
        loop {
            if curr_nodes.is_empty() {
                if all_nodes.is_empty() {
                    break;
                } else {
                    let elem = *all_nodes.iter().next().unwrap();
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
                // self.nodes[n].inputs.reverse();
                for &inp in self.nodes[n].inputs.iter() {
                    let level = if inp == self.nodes[n].inputs[self.nodes[n].inputs.len() - 1] {
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

    pub fn simple_print(&self, template: &Template) {
        for node in &self.nodes {
            println!("{}", node.format(template));
        }
    }

    pub fn graph_print(&self, template: &Template) {
        if self.nodes.is_empty() {
            return;
        }

        let mut graph_nodes: Vec<GraphNode> = Vec::new();
        let mut all_nodes: HashSet<usize> = (1..self.nodes.len()).collect();
        let mut curr_nodes: Vec<usize> = vec![0];
        loop {
            if curr_nodes.is_empty() {
                if all_nodes.is_empty() {
                    break;
                } else {
                    eprint!("Error");
                    let elem = *all_nodes.iter().next().unwrap();
                    curr_nodes.push(elem);
                    all_nodes.remove(&elem);
                }
            }
            let mut gnd = GraphNode::default();
            let n = curr_nodes.pop().unwrap();
            let node = &self.nodes[n];
            gnd.text = node.format(template);

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
                println!();
            })
    }

    pub fn graph_print_dot(&self, settings: &GraphVizSettings) {
        if self.nodes.is_empty() {
            return;
        }

        // Node index, x and y
        let mut graph_nodes: Vec<(usize, f64, f64)> = Vec::new();
        let mut all_nodes: HashSet<usize> = (1..self.nodes.len()).collect();
        let mut curr_nodes: Vec<usize> = vec![0];
        loop {
            if curr_nodes.is_empty() {
                if all_nodes.is_empty() {
                    break;
                } else {
                    eprint!("Error");
                    let elem = *all_nodes.iter().next().unwrap();
                    curr_nodes.push(elem);
                    all_nodes.remove(&elem);
                }
            }
            let n = curr_nodes.pop().unwrap();
            let node = &self.nodes[n];
            let level = *node.get_attr("level").unwrap().read_number().unwrap();
            graph_nodes.push((
                n,
                level as f64 * settings.node_offset,
                graph_nodes.len() as f64 * settings.node_offset,
            ));

            for &inp in node.inputs.iter().rev() {
                if all_nodes.contains(&inp) {
                    curr_nodes.push(inp);
                    all_nodes.remove(&inp);
                }
            }
        }
        if let Some(sb) = &settings.sort_by {
            let mut ind: Vec<usize> = (0..graph_nodes.len()).collect();
            let attrs: Vec<f32> = ind
                .iter()
                .map(|n| {
                    self.nodes[*n]
                        .get_attr(sb)
                        .expect("Attribute should be present")
                        .read_value()
                        .expect("Attribute should have float value")
                })
                .collect();
            ind.sort_by(|n1, n2| attrs[*n1].partial_cmp(&attrs[*n2]).unwrap());
            let y_map: HashMap<usize, f64> = ind
                .into_iter()
                .enumerate()
                .map(|(k, v)| (v, k as f64 * settings.node_offset))
                .collect();
            graph_nodes = graph_nodes
                .into_iter()
                .map(|(n, x, _)| (n, x, y_map[&n]))
                .collect();
        }
        let max_x = graph_nodes
            .iter()
            .map(|(_, x, _)| x)
            .fold(f64::NAN, |a, b| f64::max(a, *b));
        let max_y = graph_nodes
            .iter()
            .map(|(_, _, y)| y)
            .fold(f64::NAN, |a, b| f64::max(a, *b));

        println!("digraph network {{");
        println!(" overlap=true;");
        println!(" node [shape={},fixedsize=false];", settings.node_shape);

        let horizontal = *settings.direction == GraphVizDirection::LeftToRight;
        for (n, mut x, mut y) in &graph_nodes {
            if horizontal {
                (x, y) = (max_y - y, x);
            }
            let node = &self.nodes[*n];
            // let riv_len = node.get_attr("riv_length").map(|l| ())
            let par = node.output.map(|o| self.nodes[o].index);
            let node_txt = node.format(&settings.templates.node);
            let label = node.format(&settings.templates.label);
            let url = node.format(&settings.templates.url);
            print!(
                "{} [pos=\"{},{}!\", size={}, fixedsize=true",
                node.index, x, y, settings.node_size
            );

            print!(",label=\"{}\"", node_txt);
            if !url.is_empty() {
                print!(",URL=\"{}\"", url);
            }
            println!("]");
            print!(
                "l{} [shape={},pos=\"{},{}!\", label=\"{}\",fontsize=42",
                node.index,
                settings.label_shape,
                if horizontal {
                    x
                } else {
                    max_x + settings.label_offset
                },
                if horizontal {
                    max_x + settings.label_offset
                } else {
                    y
                },
                label
            );
            if !url.is_empty() {
                print!(",URL=\"{}\"", url);
            }
            println!("]");
            println!("{0} -> l{0} [color=none]", node.index);
            if let Some(par) = par {
                println!("{} -> {}", node.index, par);
            }
        }
        println!("}}");
    }

    fn generate_latex_table(&self, latex_table: &Vec<(String, Template)>, url_template: &Template) {
        if self.nodes.is_empty() {
            return;
        }
        // Node index, x and y
        let mut graph_nodes: Vec<(usize, usize, usize)> = Vec::new();
        let mut all_nodes: HashSet<usize> = (1..self.nodes.len()).collect();
        let mut curr_nodes: Vec<usize> = vec![0];
        loop {
            if curr_nodes.is_empty() {
                if all_nodes.is_empty() {
                    break;
                } else {
                    eprint!("Error");
                    let elem = *all_nodes.iter().next().unwrap();
                    curr_nodes.push(elem);
                    all_nodes.remove(&elem);
                }
            }
            let n = curr_nodes.pop().unwrap();
            let node = &self.nodes[n];
            let level = *node.get_attr("level").unwrap().read_number().unwrap();
            graph_nodes.push((n, level, graph_nodes.len()));

            for &inp in node.inputs.iter().rev() {
                if all_nodes.contains(&inp) {
                    curr_nodes.push(inp);
                    all_nodes.remove(&inp);
                }
            }
        }
        let table_fmt: String = "l".repeat(latex_table.len() + 1);
        println!(
            r"\documentclass{{standalone}}

\usepackage{{array}}
\usepackage{{booktabs}}
\usepackage{{multirow}}
\usepackage{{graphicx}}
\usepackage[hidelinks]{{hyperref}}
\usepackage{{tikz}}
\usetikzlibrary{{tikzmark}}

\newcommand{{\TikzNode}}[4][0]{{%
  \tikz[overlay,remember picture]{{\draw (#1 / 2 +0.5, 0.1) circle [radius=0.14] node (#2) {{\href{{#4}}{{\tiny #3}}}};}}}}


\begin{{document}}

  \begin{{tabular}}{{{table_fmt}}}
    \toprule"
        );
        print!("Connection");
        for (head, _) in latex_table {
            print!(" & {head}");
        }
        println!(r"\\");
        println!(r"\midrule");
        let mut connections_list: Vec<String> = Vec::new();
        for (n, x, _) in graph_nodes.iter().rev() {
            let node = &self.nodes[*n];
            // let riv_len = node.get_attr("riv_length").map(|l| ())
            let parent = node.output.map(|o| self.nodes[o].index);
            let url = node.format(url_template);
            print!("\\TikzNode[{x}]{{{0}}}{{{0}}}{{{url}}}", node.index);
            for (_, templ) in latex_table {
                let templ = node.format(&templ);
                print!(" & {templ}");
            }
            println!(r"\\");

            if let Some(par) = parent {
                connections_list.push(format!("\\path[->] ({}) edge ({});", node.index, par));
            }
        }
        println!("\\bottomrule");
        println!("\\end{{tabular}}");
        // this causes a small extra space on the right side, couldn't fix it
        println!("\\tikz[overlay,remember picture]{{");
        for conn in connections_list {
            println!("{}", conn);
        }
        println!("}}");
        println!(r"\end{{document}}")
    }
}

fn set_values(network: &mut Network, var: &str, values: &HashMap<&str, f32>) {
    for i in 0..network.nodes.len() {
        let val = *values.get(network.nodes[i].get_name()).unwrap();
        network.nodes[i].set_attr(&format!("cum_{var}"), NodeAttr::value(val));
    }
}

fn get_values<'a>(
    network: &'a Network,
    var: &str,
    safe: bool,
    values: &mut HashMap<&'a str, f32>,
) -> Result<(), Error> {
    for node in &network.nodes {
        let val = if safe {
            node.get_attr(var)
                .and_then(|v| v.read_value())
                .unwrap_or(0.0)
        } else {
            node.get_attr(var)
                .context(format!("Node {} doesn't have attribute {}", node.name, var))?
                .read_value()
                .context(format!(
                    "Node {}, attribute {} is not parsable as float",
                    node.name, var
                ))?
        };
        values.insert(node.get_name(), val);
    }
    Ok(())
}

![nadi icon](./icons/nadi-128.png)

# NADI (Not Available Data Integration)
It is a software package with the intention to deal with data gaps in timeseries data. It is being developed with primary focus for handling data with network configurations like river networks where the data gaps can be estimated using data from connected nodes.

Nadi also means river in Nepali. 

Although the term "graph" is used, here it is used to mean directional graphs as we're focusing in river networks.

# Installation
## Binary
`nadi` binary can be installed using the rust ecosystem, or the `makepkg` command in Arch Linux.

To compile the program, run `cargo build --release`, and then you'll have the `nadi` binary in the `target/release` folder. Copy that to your `PATH`. Also, you'll probably need shared libraries for `gdal`

## QGIS plugin
The python plugin for QGIS is in the `qgis/` directory. Copy `qgis/nadi` to `~/.local/share/QGIS/QGIS3/profiles/default/python/plugins/` to load it into QGIS.

# Planned Features
- [ ] Read graph connection from a file
  - [x] Ignore comments
  - [x] Read Nodes
  - [x] Read Edges/Connections
  - [x] Read Node Attributes from file
  - [ ] Extract node and edges from [DOT language file](https://www.graphviz.org/doc/info/lang.html)
  - [ ] Extract node and edges attributes from [DOT language file](https://www.graphviz.org/doc/info/lang.html)
- [x] Visualization of the graph
  - [x] ASCII Visualization of graph network
  - [x] Graphviz Compatible Visualization of graph network
  - [x] LaTeX code for Visualization of network and attributes
  - [x] Visualization sorted by attributes (?)
  - [x] Attributes Display using a template
- [ ] Data Filling
  - [ ] Forward Fill
  - [ ] Backward Fill
  - [ ] Center Fill
  - [ ] Linear Interpolation
  - [ ] Seasonality Fill
	- [ ] Simple seasonality
	- [ ] Seasonality Kernel (for circular averaging across data)
  - [ ] Correlation Fill
	- [ ] Simple correlation
	- [ ] Correlation Kernel (for circular averaging across data)
  - [ ] Routing Model Fill
	- [ ] Forward Propagation
	- [ ] Backward propagation
	- [ ] Both
- [ ] Data reinterpolation (e.g. to remove leap year and convert to
		365 days for seasonality; to convert monthly data to
		bi-monthly, etc)
  - [ ] Linear interpolation
  - [ ] Median data removal
  - [ ] Nearest neighbour
  - [ ] Aggregate
- [ ] Handle Varying Time information in different nodes
- [ ] Plugin system for custom functions and methods

![nadi icon](./icons/nadi-128.png)

# NADI (Not Available Data Integration)
It is a software package with the intention to deal with data gaps in timeseries data. It is being developed with primary focus for handling data with network configurations like river networks where the data gaps can be estimated using data from connected nodes.

Nadi also means river in Nepali. 

Although the term "graph" is used, here it is used to mean directional graphs as we're focusing in river networks.

# Planned Features
- [ ] Read graph connection from a file
  - [ ] Ignore comments
  - [ ] Read Nodes
  - [ ] Read Edges/Connections
  - [ ] Read Node Attributes from file
  - [ ] Extract node and edges from [DOT language file](https://www.graphviz.org/doc/info/lang.html)
  - [ ] Extract node and edges attributes from [DOT language file](https://www.graphviz.org/doc/info/lang.html)
- [ ] ASCII Visualization of the graph
  - [ ] Visualization of graph network
  - [ ] Visualization sorted by attributes (?)
  - [ ] Attributes Display using a template
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

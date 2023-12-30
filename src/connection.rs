use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use anyhow::Context;
use clap::Args;
use gdal::vector::{FieldValue, Geometry, Layer, LayerAccess, OGRFieldType};
use gdal::{Dataset, Driver, DriverManager, GdalOpenFlags, LayerOptions, Metadata};
use ordered_float::NotNan;

use crate::cliargs::CliAction;

#[derive(Args)]
pub struct CliArgs {
    /// Ignore spatial reference check
    #[arg(short, long, action)]
    ignore_spatial_reference: bool,
    /// Fields to use as id for Points file
    #[arg(short, long)]
    points_field: Option<String>,
    /// Fields to use as id for Streams vector file
    #[arg(short, long)]
    streams_field: Option<String>,
    /// Output driver [default: based on file extension]
    #[arg(short, long)]
    driver: Option<String>,
    /// Output file
    #[arg(short, long, value_parser=parse_new_layer)]
    output: Option<(PathBuf, Option<String>)>,
    /// Connections only on the output file instead of whole streams
    #[arg(short, long)]
    connections_only: bool,
    /// Print progress
    #[arg(short, long)]
    verbose: bool,
    /// Nodes file, if provided save the nodes of the graph as points with nodeid
    #[arg(short, long, value_parser=parse_new_layer)]
    nodes: Option<(PathBuf, Option<String>)>,
    /// Points file with points of interest
    #[arg(value_parser=parse_layer, value_name="POINTS_FILE[:LAYER]")]
    points: (PathBuf, String),
    /// Streams vector file with streams network
    #[arg(value_parser=parse_layer, value_name="STREAMS_FILE[:LAYER]")]
    streams: (PathBuf, String),
}

fn parse_new_layer(arg: &str) -> Result<(PathBuf, Option<String>), anyhow::Error> {
    if let Some((path, layer)) = arg.split_once(':') {
        Ok((PathBuf::from(path), Some(layer.to_string())))
    } else {
        Ok((PathBuf::from(arg), None))
    }
}

fn parse_layer(arg: &str) -> Result<(PathBuf, String), anyhow::Error> {
    if let Some((path, layer)) = arg.split_once(':') {
        let data = Dataset::open(path)?;
        if data.layer_by_name(layer).is_err() {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Layer name {layer} doesn't exist in the file {path}"),
            )
            .into())
        } else {
            Ok((PathBuf::from(path), layer.to_string()))
        }
    } else {
        let data = Dataset::open(arg)?;
        if data.layer_count() == 1 {
            let layer = data.layer(0)?;
            Ok((PathBuf::from(&arg), layer.name()))
        } else {
            eprintln!("Provide a layer name to choose layer \"FILENAME:LAYERNAME\"");
            eprintln!("Available Layers:");
            data.layers().for_each(|l| eprintln!("  {}", l.name()));
            let layer = data.layer(0)?;
            Ok((PathBuf::from(&arg), layer.name()))
        }
    }
}

impl CliAction for CliArgs {
    fn run(self) -> Result<(), anyhow::Error> {
        let points_data = Dataset::open(&self.points.0).unwrap();
        let points = points_data.layer_by_name(&self.points.1).unwrap();

        let streams_data = Dataset::open(&self.streams.0).unwrap();
        let streams = streams_data.layer_by_name(&self.streams.1).unwrap();

        if self.ignore_spatial_reference
            || check_spatial_ref_system_compatibility(&points, &streams).is_ok()
        // TODO streams is line GIS layer
        {
            self.print_connections(points, streams, &self.output)?;
        }

        Ok(())
    }
}

impl CliArgs {
    fn print_connections(
        &self,
        mut points_lyr: Layer,
        mut streams_lyr: Layer,
        output: &Option<(PathBuf, Option<String>)>,
    ) -> Result<(), anyhow::Error> {
        let points = get_geometries(&mut points_lyr, &self.points_field)?;
        let streams = get_geometries(&mut streams_lyr, &self.streams_field)?;
        if points.is_empty() || streams.is_empty() {
            return Ok(());
        }

        let origin = Point2D::new((0.0, 0.0, 0.0));
        let mut points_closest: HashMap<&str, (Point2D, Point2D, f64)> = points
            .iter()
            .map(|(k, _)| (k.as_str(), (origin.clone(), origin.clone(), f64::INFINITY)))
            .collect();

        // node: point to node number
        let mut nodes: HashMap<Point2D, usize> = HashMap::new();
        // node number to geometry index in streams file
        let mut streams_geo_location: HashMap<(usize, usize), usize> = HashMap::new();
        // geometries of the streams
        let mut streams_touched: HashMap<(usize, usize), Geometry> = HashMap::new();
        // edge: node to another node at the end
        let mut edges: HashMap<usize, usize> = HashMap::new();
        let mut branches: HashMap<usize, usize> = HashMap::new();

        let mut progress: usize = 0;
        let total = streams.len();
        for (i, (_name, geom)) in streams.iter().enumerate() {
            let start = Point2D::new(geom.get_point(0));
            let end = Point2D::new(geom.get_point((geom.point_count() - 1) as i32));
            let l = nodes.len();
            let start_ind = *nodes.entry(start.clone()).or_insert(l);
            let l = nodes.len();
            let end_ind = *nodes.entry(end.clone()).or_insert(l);
            streams_geo_location.insert((start_ind, end_ind), i);
            if let Entry::Vacant(e) = edges.entry(start_ind) {
                e.insert(end_ind);
            } else {
                branches.insert(start_ind, end_ind);
            }

            points.iter().for_each(|(k, p)| {
                let dist = distance(p, geom);
                if dist < points_closest[k.as_str()].2 {
                    points_closest.insert(k.as_str(), (start.clone(), end.clone(), dist));
                }
            });
            if self.verbose {
                progress += 1;
                println!("Reading Streams: {}", progress * 100 / total);
            }
        }

        for (_, (start, end, _)) in &points_closest {
            let edge = (nodes[&start], nodes[&end]);
            let i = streams_geo_location[&edge];
            streams_touched.insert(edge, streams[i].1.clone());
        }
        if let Some((filename, lyr)) = &self.nodes {
            let driver = get_driver_by_filename(&filename, &self.driver)?;
            let mut out_data = driver.create_vector_only(&filename)?;
            // let mut txn = out_data.start_transaction()?;
            let mut layer = out_data.create_layer(LayerOptions {
                name: lyr.as_ref().unwrap_or(&"nodes".to_string()),
                srs: streams_lyr.spatial_ref().as_ref(),
                ty: gdal_sys::OGRwkbGeometryType::wkbPoint,
                ..Default::default()
            })?;
            layer.create_defn_fields(&[("id", OGRFieldType::OFTInteger)])?;
            let fields = ["id"];

            for (pt, id) in &nodes {
                let mut edge_geometry = Geometry::empty(gdal_sys::OGRwkbGeometryType::wkbPoint)?;
                edge_geometry.add_point(pt.coord());
                layer.create_feature_fields(
                    edge_geometry,
                    &fields,
                    &[FieldValue::IntegerValue(*id as i32)],
                )?;
            }
            // txn.commit()?;
        }

        let points_nodes: HashMap<usize, &str> = points_closest
            .iter()
            .map(|(&k, (_, v, _))| (nodes[v], k))
            .collect();
        let mut points_edges: HashMap<usize, usize> = HashMap::new();
        let nodes_rev: HashMap<usize, &Point2D> = nodes.iter().map(|(k, &v)| (v, k)).collect();

        progress = 0;
        let total = points_nodes.len();
        for pt in points_nodes.keys() {
            let mut outlet = *pt;
            // eprint!("{}", pt);
            let mut curr_branches: Vec<&usize> = Vec::new();
            let mut final_outlet = None;
            loop {
                if let Some(&o) = edges.get(&outlet) {
                    if let Some(bout) = branches.get(&outlet) {
                        if let Some(&i) = streams_geo_location.get(&(outlet, *bout)) {
                            streams_touched.insert((outlet, i), streams[i].1.clone());
                        }
                        curr_branches.push(bout);
                    }
                    if let Some(&i) = streams_geo_location.get(&(outlet, o)) {
                        streams_touched.insert((outlet, i), streams[i].1.clone());
                    }
                    // eprint!(" -> {}", outlet);
                    outlet = o;
                    if points_nodes.contains_key(&o) {
                        println!("{} -> {}", points_nodes[pt], points_nodes[&outlet]);
                        points_edges.insert(*pt, outlet);
                        final_outlet = Some(outlet);
                        break;
                    }
                } else {
                    eprintln!(
                        "{} {} -> None {}",
                        points_nodes[pt], nodes_rev[pt], nodes_rev[&outlet]
                    );
                    break;
                }
            }

            for b in curr_branches {
                // currently can't detect branches in the branch,
                // maybe we can call it recursively after separating
                // it in a function
                let mut converses = false;
                let mut b = *b;
                while let Some(&co) = edges.get(&b) {
                    if let Some(&i) = streams_geo_location.get(&(b, co)) {
                        streams_touched.insert((outlet, i), streams[i].1.clone());
                    }
                    if Some(co) == final_outlet {
                        converses = true;
                        break;
                    }
                    b = co;
                }
                if final_outlet.is_some() && !converses {
                    eprintln!(
                        "Branch detected from node {} downstream of {}",
                        b, points_nodes[pt]
                    );
                }
            }
            if self.verbose {
                progress += 1;
                println!("Searching Connections: {}", progress * 100 / total);
            }
        }

        if let Some(output) = output {
            save_connections_file(
                &self.driver,
                output,
                &streams_lyr,
                &points,
                &points_nodes,
                &points_edges,
                streams_touched,
                self.connections_only,
            )?;
        }

        Ok(())
    }
}

fn save_connections_file(
    driver: &Option<String>,
    output: &(PathBuf, Option<String>),
    streams_lyr: &Layer,
    points: &Vec<(String, Geometry)>,
    points_nodes: &HashMap<usize, &str>,
    points_edges: &HashMap<usize, usize>,
    streams_touched: HashMap<(usize, usize), Geometry>,
    connections_only: bool,
) -> Result<(), anyhow::Error> {
    let driver = get_driver_by_filename(&output.0, driver)?;
    let mut out_data = driver.create_vector_only(&output.0)?;
    // Not supported in all the formats, so removing it.
    // let mut txn = out_data.start_transaction()?;
    let mut layer = out_data.create_layer(LayerOptions {
        name: output.1.as_ref().unwrap_or(&"network".to_string()),
        srs: streams_lyr.spatial_ref().as_ref(),
        ty: gdal_sys::OGRwkbGeometryType::wkbLineString,
        ..Default::default()
    })?;

    if connections_only {
        layer.create_defn_fields(&[
            ("start", OGRFieldType::OFTString),
            ("end", OGRFieldType::OFTString),
        ])?;
        let fields = ["start", "end"];

        let points_map: HashMap<&str, (f64, f64, f64)> = points
            .iter()
            .map(|(k, g)| (k.as_str(), g.get_point(0)))
            .collect();
        for (start, end) in points_edges {
            let mut edge_geometry = Geometry::empty(gdal_sys::OGRwkbGeometryType::wkbLineString)?;
            edge_geometry.add_point(points_map[points_nodes[start]]);
            edge_geometry.add_point(points_map[points_nodes[end]]);
            layer.create_feature_fields(
                edge_geometry,
                &fields,
                &[
                    FieldValue::StringValue(points_nodes[start].to_string()),
                    FieldValue::StringValue(points_nodes[end].to_string()),
                ],
            )?;
        }
    } else {
        layer.create_defn_fields(&[("start", OGRFieldType::OFTString)])?;
        layer.create_defn_fields(&[("end", OGRFieldType::OFTString)])?;
        let fields = ["start", "end"];
        for ((start, end), geo) in streams_touched {
            layer.create_feature_fields(
                geo,
                &fields,
                &[
                    FieldValue::StringValue(points_nodes.get(&start).unwrap_or(&"").to_string()),
                    FieldValue::StringValue(points_nodes.get(&end).unwrap_or(&"").to_string()),
                ],
            )?;
        }
    }
    // txn.commit()?;
    Ok(())
}

#[derive(Eq, PartialEq, Hash, Clone, Debug)]
struct Point2D {
    x: NotNan<f64>,
    y: NotNan<f64>,
}

impl Point2D {
    fn new(coord: (f64, f64, f64)) -> Self {
        Self {
            x: NotNan::new(coord.0).expect("GIS Coordinate shouldn't be NaN"),
            y: NotNan::new(coord.1).expect("GIS Coordinate shouldn't be NaN"),
        }
    }

    fn coord(&self) -> (f64, f64, f64) {
        (self.x.into_inner(), self.y.into_inner(), 0.0)
    }
}

impl fmt::Display for Point2D {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

fn distance(point: &Geometry, line: &Geometry) -> f64 {
    let (x, y, _) = point.get_point(0);
    let dist: f64 = line
        .get_point_vec()
        .iter()
        .map(|&(sx, sy, _)| (sx - x).powi(2) + (sy - y).powi(2))
        .fold(f64::INFINITY, |a, b| a.min(b));
    dist
}

fn get_geometries(
    layer: &mut Layer,
    field: &Option<String>,
) -> Result<Vec<(String, Geometry)>, anyhow::Error> {
    layer
        .features()
        .enumerate()
        .map(|(i, f)| {
            let geom = match f.geometry() {
                Some(g) => g.clone(),
                None => {
                    // TODO take X,Y possible names as Vec<String>
                    let x = f.field_as_double_by_name("lon")?.unwrap();
                    let y = f.field_as_double_by_name("lat")?.unwrap();
                    let mut pt = Geometry::empty(gdal_sys::OGRwkbGeometryType::wkbPoint)?;
                    pt.add_point((x, y, 0.0));
                    pt
                }
            };
            let name = if let Some(name) = field {
                f.field_as_string_by_name(name)?.unwrap_or("".to_string())
            } else {
                i.to_string()
            };
            Ok((name, geom.to_owned()))
        })
        .collect()
}

fn check_spatial_ref_system_compatibility(points: &Layer, streams: &Layer) -> Result<(), ()> {
    match (
        points.spatial_ref().and_then(|r| r.to_proj4().ok()),
        streams.spatial_ref().and_then(|r| r.to_proj4().ok()),
    ) {
        (Some(p), Some(s)) => {
            if p != s {
                eprintln!("Spatial reference mismatch.");
                eprintln!("{:?} {:?}", p, s);
                // TODO proper error return
                return Err(());
            }
        }
        (Some(_), None) => {
            eprintln!("Streams layer doesn't have spatial reference");
        }
        (None, Some(_)) => {
            eprintln!("Points layer doesn't have spatial reference");
        }
        (None, None) => {
            eprintln!("Streams and Point layers don't have spatial reference");
        }
    }
    Ok(())
}

fn get_driver_by_filename(filename: &PathBuf, driver: &Option<String>) -> anyhow::Result<Driver> {
    let drivers =
        get_drivers_for_filename(filename.to_str().unwrap(), &GdalOpenFlags::GDAL_OF_VECTOR);

    if let Some(driver) = driver {
        drivers
            .into_iter()
            .filter(|d| d.short_name() == *driver)
            .next()
            .context(format!(
                "There is no matching vector driver {driver} for filename {filename:?}"
            ))
    } else {
        if drivers.len() > 1 {
            eprintln!(
                "Multiple drivers are compatible defaulting to the first: {:?}",
                drivers
                    .iter()
                    .map(|d| d.short_name())
                    .collect::<Vec<String>>()
            )
        }
        drivers.into_iter().next().context(format!(
            "Couldn't infer driver based on filename: {filename:?}"
        ))
    }
}

// remove once the gdal has the pull request merged
// https://github.com/georust/gdal/pull/510
fn get_drivers_for_filename(filename: &str, options: &GdalOpenFlags) -> Vec<Driver> {
    let ext = {
        let filename = filename.to_ascii_lowercase();
        let e = match filename.rsplit_once(".") {
            Some(("", _)) => "", // hidden file no ext
            Some((f, "zip")) => {
                // zip files could be zipped shp or gpkg
                if f.ends_with(".shp") {
                    "shp.zip"
                } else if f.ends_with(".gpkg") {
                    "gpkg.zip"
                } else {
                    "zip"
                }
            }
            Some((_, e)) => e, // normal file with ext
            None => "",
        };
        e.to_string()
    };

    let mut drivers: Vec<Driver> = Vec::new();
    for i in 0..DriverManager::count() {
        let d = DriverManager::get_driver(i).expect("Index for this loop should be valid");
        let mut supports = false;
        if (d.metadata_item("DCAP_CREATE", "").is_some()
            || d.metadata_item("DCAP_CREATECOPY", "").is_some())
            && ((options.contains(GdalOpenFlags::GDAL_OF_VECTOR)
                && d.metadata_item("DCAP_VECTOR", "").is_some())
                || (options.contains(GdalOpenFlags::GDAL_OF_RASTER)
                    && d.metadata_item("DCAP_RASTER", "").is_some()))
        {
            supports = true;
        } else if options.contains(GdalOpenFlags::GDAL_OF_VECTOR)
            && d.metadata_item("DCAP_VECTOR_TRANSLATE_FROM", "").is_some()
        {
            supports = true;
        }
        if !supports {
            continue;
        }

        if let Some(e) = &d.metadata_item("DMD_EXTENSION", "") {
            if *e == ext {
                drivers.push(d);
                continue;
            }
        }
        if let Some(e) = d.metadata_item("DMD_EXTENSIONS", "") {
            if e.split(" ").collect::<Vec<&str>>().contains(&ext.as_str()) {
                drivers.push(d);
                continue;
            }
        }

        if let Some(pre) = d.metadata_item("DMD_CONNECTION_PREFIX", "") {
            if filename.starts_with(&pre) {
                drivers.push(d);
            }
        }
    }

    return drivers;
}

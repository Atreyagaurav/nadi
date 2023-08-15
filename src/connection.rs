use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use clap::Args;
use gdal::vector::{FieldValue, Geometry, Layer, LayerAccess, OGRFieldType};
use gdal::{Dataset, DriverManager, LayerOptions};
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
    /// Output driver
    #[arg(short, long, default_value = "GPKG")]
    driver: String,
    /// Output file
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Points file with points of interest
    #[arg(value_parser=parse_layer, value_name="POINTS_FILE[:LAYER]")]
    points: (PathBuf, String),
    /// Streams vector file with streams network
    #[arg(value_parser=parse_layer, value_name="STREAMS_FILE[:LAYER]")]
    streams: (PathBuf, String),
}

fn parse_layer(arg: &str) -> Result<(PathBuf, String), anyhow::Error> {
    if let Some((path, layer)) = arg.split_once(":") {
        let data = Dataset::open(path)?;
        if data.layer_by_name(&layer).is_err() {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Layer name {layer} doesn't exist in the file {path}"),
            )
            .into())
        } else {
            Ok((PathBuf::from(path), layer.to_string()))
        }
    } else {
        let data = Dataset::open(&arg)?;
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

        if (self.ignore_spatial_reference
            || check_spatial_ref_system_compatibility(&points, &streams).is_ok())
            && valid_driver_name(&self.driver)
        // TODO streams is line GIS layer
        {
            self.print_connections(points, streams, &self.output, &self.driver)?;
        }

        Ok(())
    }
}

impl CliArgs {
    fn print_connections(
        &self,
        mut points_lyr: Layer,
        mut streams_lyr: Layer,
        output: &Option<PathBuf>,
        driver: &str,
    ) -> Result<(), anyhow::Error> {
        let points = get_geometries(&mut points_lyr, &self.points_field)?;
        let streams = get_geometries(&mut streams_lyr, &self.streams_field)?;
        if points.is_empty() || streams.is_empty() {
            return Ok(());
        }
        let mut points_closest: HashMap<&str, (Point2D, f64)> = points
            .iter()
            .map(|(k, p)| {
                let dist = distance(p, &streams[0].1);
                let end = Point2D::new(
                    streams[0]
                        .1
                        .get_point((streams[0].1.point_count() - 1) as i32),
                );
                (k.as_str(), (end, dist))
            })
            .collect();

        let mut nodes: HashMap<Point2D, usize> = HashMap::new();
        let mut edges: HashMap<usize, usize> = HashMap::new();
        for (_name, geom) in &streams[1..] {
            let start = Point2D::new(geom.get_point(0));
            let end = Point2D::new(geom.get_point((geom.point_count() - 1) as i32));
            if !nodes.contains_key(&start) {
                nodes.insert(start.clone(), nodes.len());
            }
            if !nodes.contains_key(&end) {
                nodes.insert(end.clone(), nodes.len());
            }
            edges.insert(nodes[&start], nodes[&end]);

            points.iter().for_each(|(k, p)| {
                let dist = distance(p, &geom);
                if dist < points_closest[k.as_str()].1 {
                    points_closest.insert(k.as_str(), (end.clone(), dist));
                }
            });
        }

        let points_nodes: HashMap<usize, &str> = points_closest
            .iter()
            .map(|(&k, (v, _))| (nodes[v], k))
            .collect();
        let mut points_edges: HashMap<usize, usize> = HashMap::new();
        let nodes_rev: HashMap<usize, &Point2D> = nodes.iter().map(|(k, &v)| (v, k)).collect();

        for pt in points_nodes.keys() {
            let mut outlet = *pt;
            loop {
                if let Some(&o) = edges.get(&outlet) {
                    outlet = o;
                    if points_nodes.contains_key(&o) {
                        println!("{} -> {}", points_nodes[pt], points_nodes[&outlet]);
                        points_edges.insert(*pt, outlet);
                        break;
                    }
                } else {
                    eprintln!(
                        "{} {} -> None {}",
                        points_nodes[pt], nodes_rev[&pt], nodes_rev[&outlet]
                    );
                    break;
                }
            }
        }

        if let Some(output) = output {
            let driver = DriverManager::get_driver_by_name(driver)?;
            let mut out_data = driver.create_vector_only(output)?;

            let mut txn = out_data.start_transaction()?;
            let mut layer = txn.create_layer(LayerOptions {
                name: "network",
                srs: streams_lyr.spatial_ref().as_ref(),
                ty: gdal_sys::OGRwkbGeometryType::wkbLineString,
                ..Default::default()
            })?;
            layer.create_defn_fields(&[
                ("start", OGRFieldType::OFTString),
                ("end", OGRFieldType::OFTString),
            ])?;
            let fields = ["start", "end"];

            let points_map: HashMap<&str, (f64, f64, f64)> = points
                .iter()
                .map(|(k, g)| (k.as_str(), g.get_point(0)))
                .collect();
            for (start, end) in &points_edges {
                let mut edge_geometry =
                    Geometry::empty(gdal_sys::OGRwkbGeometryType::wkbLineString)?;
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
            txn.commit()?;
        }

        Ok(())
    }
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
            let geom = f.geometry().unwrap();
            let name = if let Some(name) = field {
                f.field_as_string_by_name(&name)?.unwrap()
            } else {
                i.to_string()
            };
            Ok((name, geom.to_owned()))
        })
        .collect()
}

fn valid_driver_name(name: &str) -> bool {
    match DriverManager::get_driver_by_name(name) {
        Ok(_) => true,
        Err(e) => {
            eprintln!("{:?}", e);
            for i in 0..DriverManager::count() {
                let d = DriverManager::get_driver(i).unwrap();
                eprintln!("{} : {}", d.short_name(), d.long_name());
            }
            false
        }
    }
}

fn check_spatial_ref_system_compatibility(points: &Layer, streams: &Layer) -> Result<(), ()> {
    match (
        points.spatial_ref().and_then(|r| r.to_proj4().ok()),
        streams.spatial_ref().and_then(|r| r.to_proj4().ok()),
    ) {
        (Some(p), Some(s)) => {
            if p != s {
                eprintln!("Spatial reference mismatch.");
                eprintln!("{}", format!("{:?} {:?}", p, s));
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

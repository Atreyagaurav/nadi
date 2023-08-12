use std::path::PathBuf;

use clap::Args;
use gdal::vector::LayerAccess;
use gdal::Dataset;

use crate::cliargs::CliAction;

#[derive(Args)]
pub struct CliArgs {
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
            eprintln!("Please provide a layer \"FILENAME:LAYERNAME\"");
            eprintln!("Available Layers:");
            data.layers().for_each(|l| eprintln!("  {}", l.name()));
            let layer = data.layer(0)?;
            Ok((PathBuf::from(&arg), layer.name()))
        }
    }
}

impl CliAction for CliArgs {
    fn run(self) -> anyhow::Result<()> {
        let data = Dataset::open(self.points.0).unwrap();
        let mut points = data.layer_by_name(&self.points.1).unwrap();
        for feature in points.features() {
            let geom = feature.geometry().unwrap();
            let start = geom.get_point(0);
            let end = geom.get_point((geom.point_count() - 1) as i32);
            println!("{:?} {:?}", start, end);
        }
        Ok(())
    }
}

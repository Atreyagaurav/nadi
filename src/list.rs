use std::path::PathBuf;

use clap::Args;
use gdal::vector::{FieldValue, Layer, LayerAccess};
use gdal::Dataset;

use crate::cliargs::CliAction;

#[derive(Args)]
pub struct CliArgs {
    /// Fields to use as id for file
    #[arg(short, long)]
    primary_key: Option<String>,
    /// GIS file with points of interest
    #[arg(value_parser=parse_layer, value_name="POINTS_FILE[:LAYER]")]
    file: (PathBuf, String),
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
        let file_data = Dataset::open(&self.file.0).unwrap();
        let file = file_data.layer_by_name(&self.file.1).unwrap();
        self.print_attrs(file, &self.primary_key)?;
        Ok(())
    }
}

impl CliArgs {
    fn print_attrs(&self, mut lyr: Layer, field: &Option<String>) -> Result<(), anyhow::Error> {
        for (i, f) in lyr.features().enumerate() {
            let name = if let Some(name) = field {
                f.field_as_string_by_name(name)?.unwrap_or("".to_string())
            } else {
                i.to_string()
            };
            f.fields().for_each(|(s, v)| {
                if let Some(val) = v {
                    match val {
                        FieldValue::Integer64Value(i) => println!("{name}::{s}={i}"),
                        FieldValue::StringValue(i) => println!("{name}::{s}={i}"),
                        FieldValue::RealValue(i) => println!("{name}::{s}={i}"),
                        FieldValue::DateValue(i) => println!("{name}::{s}={i}"),
                        _ => (),
                    }
                }
            });
        }
        Ok(())
    }
}

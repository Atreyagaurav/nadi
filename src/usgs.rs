use anyhow;
use reqwest;
use std::io::Write;
use std::{fs::File, path::PathBuf};

use clap::{Args, ValueEnum, ValueHint};

use crate::cliargs::CliAction;

#[derive(Args)]
pub struct CliArgs {
    /// USGS Site no
    #[arg(short, long, value_delimiter = ',', required = true)]
    site_no: Vec<String>,
    /// Type of data (u/d/t/b)
    ///
    /// [upstream (u), downstream (d), tributories (t), basin (b)]
    #[arg(
        short,
        long,
        rename_all = "lower",
        default_value = "t",
        value_enum,
        hide_possible_values = true
    )]
    data: Vec<GeoInfo>,
    #[arg(short, long, value_hint=ValueHint::DirPath, default_value=".")]
    output_dir: PathBuf,
}

impl CliAction for CliArgs {
    fn run(self) -> anyhow::Result<()> {
        for site in self.site_no {
            for data in &self.data {
                data.download(&site, &self.output_dir);
            }
        }
        Ok(())
    }
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum GeoInfo {
    #[value(alias = "u")]
    Upstream,
    #[value(alias = "d")]
    Downstream,
    #[value(alias = "t")]
    Tributories,
    #[value(alias = "b")]
    Basin,
}

impl GeoInfo {
    pub fn usgs_abbr(&self) -> &str {
        match self {
            Self::Upstream => "navigate/UM",
            Self::Downstream => "navigate/DM",
            Self::Tributories => "navigate/UT",
            Self::Basin => "basin",
        }
    }

    pub fn usgs_url(&self, site_no: &str) -> String {
        let dt = self.usgs_abbr();
        format!("https://labs.waterdata.usgs.gov/api/nldi/linked-data/nwissite/USGS-{site_no}/{dt}?f=json")
    }

    pub fn download(&self, site_no: &str, dir: &PathBuf) {
        let url = self.usgs_url(site_no);
        let bytes = reqwest::blocking::get(url).unwrap().bytes().unwrap();
        let filepath = dir.join(format!(
            "{}_{}.json",
            site_no,
            self.usgs_abbr().split("/").last().unwrap()
        ));
        let mut file = File::create(filepath).unwrap();
        file.write_all(&bytes).unwrap();
    }
}

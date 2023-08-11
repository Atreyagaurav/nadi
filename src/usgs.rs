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
    /// [upstream-main (u), downstream-main (d), upstreamf-tributories (t), basin (b)]
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
                download_usgs(&site, &data, &self.output_dir);
            }
        }
        Ok(())
    }
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum GeoInfo {
    #[value(alias = "u")]
    UpstreamMain,
    #[value(alias = "d")]
    DownstreamMain,
    #[value(alias = "t")]
    UpstreamTributories,
    #[value(alias = "b")]
    Basin,
}

impl GeoInfo {
    pub fn usgs_abbr(&self) -> &str {
        match self {
            Self::UpstreamMain => "UM",
            Self::DownstreamMain => "DM",
            Self::UpstreamTributories => "UT",
            Self::Basin => "BM",
        }
    }

    pub fn usgs_url(&self, site_no: &str) -> String {
        let dt = self.usgs_abbr();
        format!("https://labs.waterdata.usgs.gov/api/nldi/linked-data/nwissite/USGS-{site_no}/navigate/{dt}?f=json")
    }
}

pub fn download_usgs(site_no: &str, info: &GeoInfo, dir: &PathBuf) {
    let url = info.usgs_url(site_no);
    let bytes = reqwest::blocking::get(url).unwrap().bytes().unwrap();
    let filepath = dir.join(format!("{}_{}.json", site_no, info.usgs_abbr()));
    let mut file = File::create(filepath).unwrap();
    file.write_all(&bytes).unwrap();
}

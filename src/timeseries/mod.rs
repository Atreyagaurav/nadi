use anyhow;
use std::collections::HashMap;
use std::path::PathBuf;

use clap::{Args, ValueHint};

use crate::cliargs::CliAction;

#[derive(Args)]
pub struct CliArgs {
    /// property to use for ratio
    #[arg(short, long, required = true)]
    property: String,
    /// File for the timeseries
    #[arg(short,long, value_hint=ValueHint::FilePath)]
    input_file: PathBuf,
    #[arg(short, long, value_hint=ValueHint::FilePath)]
    output_file: Option<PathBuf>,
}

impl CliAction for CliArgs {
    fn run(self) -> anyhow::Result<()> {
        let mut t1 = Timeseries1 {
            values: vec![1.1, 2.3, f64::NAN],
            times: TimeseriesTime::None,
            properties: HashMap::new(),
        };
        let mut t2 = Timeseries1 {
            values: vec![1.0, f64::NAN, 3.0],
            times: TimeseriesTime::None,
            properties: HashMap::new(),
        };
        t1.fill_na();
        println!("{:?}", t1.values);
        t2.cast_na_only_from(&t1);
        println!("{:?}", t2.values);
        Ok(())
    }
}

struct Timeseries1 {
    values: Vec<f64>,
    times: TimeseriesTime,
    properties: HashMap<String, f64>,
}

impl Timeseries1 {
    pub fn from_file(ts_file: &PathBuf) -> Self {
        Self {
            values: vec![1.1, 2.3, 3.4],
            times: TimeseriesTime::None,
            properties: HashMap::new(),
        }
    }
}

enum TimeseriesTime {
    None,
    Regular(usize, usize),
    Irregular(Vec<usize>),
}

pub trait Timeseries {
    fn length(&self) -> usize;
    fn get_val(&self, index: usize) -> f64;
    fn set_val(&mut self, index: usize, val: f64);
    fn set_times(&mut self, time: TimeseriesTime);
    fn get_time(&self, index: usize) -> Option<usize>;
}

impl Timeseries for Timeseries1 {
    fn length(&self) -> usize {
        self.values.len()
    }

    fn get_val(&self, index: usize) -> f64 {
        self.values[index]
    }

    fn set_val(&mut self, index: usize, val: f64) {
        self.values[index] = val;
    }

    fn get_time(&self, index: usize) -> Option<usize> {
        match &self.times {
            TimeseriesTime::None => None,
            TimeseriesTime::Regular(start, increment) => {
                if index < self.values.len() {
                    Some(start + increment * index)
                } else {
                    None
                }
            }
            TimeseriesTime::Irregular(times) => times.get(index).copied(),
        }
    }

    fn set_times(&mut self, time: TimeseriesTime) {
        self.times = time;
    }
}

pub trait Cast: Timeseries {
    fn cast_from(&mut self, other: &Self) {
        (0..self.length()).for_each(|i| self.set_val(i, self.calc_cast_from(&other, i)));
    }
    fn cast_to(&self, other: &mut Self) {
        other.cast_from(&self);
    }
    fn calc_cast_from(&self, other: &Self, index: usize) -> f64 {
        other.get_val(index)
    }
    fn calc_cast_to(&self, other: &Self, index: usize) -> f64 {
        other.calc_cast_from(&self, index)
    }
    fn cast_na_only_from(&mut self, other: &Self) {
        (0..self.length()).for_each(|i| {
            if self.get_val(i).is_nan() {
                self.set_val(i, self.calc_cast_from(&other, i))
            }
        });
    }
    fn cast_to_na_only(&self, other: &mut Self) {
        other.cast_na_only_from(&self);
    }
}

pub trait Fill: Timeseries {
    fn fill_na(&mut self);
    fn fill_na_with(&mut self, default: f64) {
        (0..self.length()).for_each(|i| {
            if self.get_val(i).is_nan() {
                self.set_val(i, default)
            }
        });
    }
}

impl Cast for Timeseries1 {}
impl Fill for Timeseries1 {
    fn fill_na(&mut self) {
        self.fill_na_with(0.0);
    }
}

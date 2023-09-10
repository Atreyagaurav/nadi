use polars::{
    export::chrono::{NaiveDate, ParseError},
    lazy::dsl::{first, when},
    prelude::*,
};

use clap::{Args, ValueEnum, ValueHint};
use std::{fs::File, path::PathBuf, str::FromStr};

use crate::cliargs::CliAction;

#[derive(Args)]
pub struct CliArgs {
    /// Date Range to filter the timeseries by
    #[arg(short, long, default_value = "",value_hint=ValueHint::Other)]
    date_range: DateRange,
    /// column name containing date and/or time in csv
    #[arg(long, default_value = "date",value_hint=ValueHint::Other)]
    datetime_col: String,
    /// column name containing discharges in csv
    #[arg(long, default_value = "flow", value_hint=ValueHint::Other)]
    discharge_col: String,
    /// Print in a abridged format that can't be piped
    #[arg(short, long, conflicts_with = "output")]
    no_pipe: bool,
    /// Print a barplot
    #[arg(short, long, conflicts_with = "output")]
    plot: Option<String>,
    /// output file path
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Action to perform
    #[arg(
        short,
        long,
        rename_all = "lower",
        default_value = "na",
        value_enum,
        hide_possible_values = false
    )]
    command: TsProcess,
    /// extra args for the command
    #[arg(short, long, value_delimiter = ',')]
    args: Vec<String>,
    /// input csv file
    #[arg(required = true)]
    input: PathBuf,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum TsProcess {
    #[value(alias = "e")]
    Echo,
    #[value(alias = "na")]
    NaValues,
    #[value(alias = "nff")]
    NaFillForward,
    #[value(alias = "nfb")]
    NaFillBackward,
    #[value(alias = "nfv")]
    NaFillValue,
    #[value(alias = "sm")]
    MonthlySeasonality,
    #[value(alias = "sd")]
    DailySeasonality,
    #[value(alias = "ay")]
    AggAnnual,
    #[value(alias = "am")]
    AggMonthly,
}

#[derive(Clone)]
pub struct DateRange {
    start: Option<NaiveDate>,
    end: Option<NaiveDate>,
}

impl FromStr for DateRange {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (start, end) = s.split_once(",").unwrap_or((s.trim(), ""));
        Ok(DateRange {
            start: if start.is_empty() {
                None
            } else {
                Some(NaiveDate::parse_from_str(start, "%Y-%m-%d")?)
            },
            end: if end.is_empty() {
                None
            } else {
                Some(NaiveDate::parse_from_str(end, "%Y-%m-%d")?)
            },
        })
    }
}

impl CliAction for CliArgs {
    fn run(self) -> anyhow::Result<()> {
        let mut ts = Discharges::new(&self.input, &self.datetime_col, &self.discharge_col);
        ts.data_table = apply_date_range(&ts, &self);

        match self.command {
            TsProcess::Echo => echo(&ts, &self),
            TsProcess::NaValues => missing_data(&ts, &self),
            TsProcess::MonthlySeasonality => monthly_seasonality(&ts, &self),
            TsProcess::DailySeasonality => daily_seasonality(&ts, &self),
            TsProcess::AggMonthly => monthly_mean(&ts, &self),
            TsProcess::AggAnnual => annual_mean(&ts, &self),
            TsProcess::NaFillForward => na_fill_forward(&ts, &self),
            _ => (),
        }
        Ok(())
    }
}

fn dataframe_output(mut outdf: DataFrame, args: &CliArgs) {
    if let Some(output) = &args.output {
        let file = File::create(output).unwrap();
        CsvWriter::new(file).finish(&mut outdf).unwrap();
    } else if args.no_pipe {
        println!("{}", outdf);
    } else if let Some(plt_col) = &args.plot {
        let outdf = outdf
            .clone()
            .lazy()
            .with_column(
                (col(plt_col) - col(plt_col).min()) / (col(plt_col).max() - col(plt_col).min()),
            )
            .collect()
            .unwrap();

        let nrow = outdf.shape().0;
        let schema: Vec<String> = outdf.schema().iter().map(|s| s.0.to_string()).collect();
        let col_ind = schema
            .iter()
            .enumerate()
            .filter(|(_, c)| c == &plt_col)
            .next()
            .unwrap()
            .0;
        let head_str: Vec<&str> = schema
            .iter()
            .enumerate()
            .filter_map(|(i, v)| if i != col_ind { Some(v.as_str()) } else { None })
            .collect();
        println!("{}", head_str.join(","));
        if outdf.is_empty() {
            return;
        }
        let mut row = outdf.get_row(0).unwrap();
        for i in 0..nrow {
            outdf.get_row_amortized(i, &mut row).unwrap();
            let row_str: Vec<String> = row
                .0
                .iter()
                .enumerate()
                .filter_map(|(i, v)| {
                    if i != col_ind {
                        Some(format!("{}", v))
                    } else {
                        None
                    }
                })
                .collect();
            print!("{}", row_str.join(","));
            let rep: usize = match row.0[col_ind] {
                AnyValue::Float64(f) => (f * 100.0) as usize,
                _ => 0,
            };
            println!("\t {}", "#".repeat(rep));
        }
    } else {
        let nrow = outdf.shape().0;
        let schema: Vec<String> = outdf.schema().iter().map(|s| s.0.to_string()).collect();
        println!("{}", schema.join(","));
        if outdf.is_empty() {
            return;
        }
        let mut row = outdf.get_row(0).unwrap();
        for i in 0..nrow {
            outdf.get_row_amortized(i, &mut row).unwrap();
            let row_str: Vec<String> = row.0.iter().map(|v| format!("{}", v)).collect();
            println!("{}", row_str.join(","));
        }
    }
}

pub struct Discharges<'a> {
    datetime_col: &'a str,
    discharge_col: &'a str,
    data_table: DataFrame,
}

impl<'a> Discharges<'a> {
    pub fn new(filename: &PathBuf, datetime_col: &'a str, discharge_col: &'a str) -> Self {
        let columns = vec![datetime_col.to_string(), discharge_col.to_string()];
        let schema = Schema::from_iter(vec![
            Field::new(datetime_col, DataType::Date),
            Field::new(discharge_col, DataType::Float64),
        ]);
        let data_table = CsvReader::from_path(filename)
            .unwrap()
            .has_header(true)
            .with_columns(Some(columns))
            .with_dtypes(Some(Arc::new(schema)))
            .finish()
            .unwrap();
        Self {
            datetime_col,
            discharge_col,
            data_table,
        }
    }

    pub fn derived(self, df: DataFrame) -> Self {
        Self {
            datetime_col: self.datetime_col,
            discharge_col: self.discharge_col,
            data_table: df,
        }
    }
}

fn apply_date_range(ts: &Discharges, args: &CliArgs) -> DataFrame {
    ts.data_table
        .clone()
        .lazy()
        .filter(
            col(ts.datetime_col)
                .gt_eq(
                    args.date_range
                        .start
                        .map(lit)
                        .unwrap_or(col(ts.datetime_col).first()),
                )
                .and(
                    col(ts.datetime_col).lt_eq(
                        args.date_range
                            .end
                            .map(lit)
                            .unwrap_or(col(ts.datetime_col).last()),
                    ),
                ),
        )
        .collect()
        .unwrap()
}

// fn apply_kernel_ma(df: DataFrame, col_name: &str, kernel: Vec<f64>) -> DataFrame {
//     // df.clone().lazy().with_column(col(col_name).)
//     df
// }

pub fn echo(ts: &Discharges, args: &CliArgs) {
    dataframe_output(ts.data_table.clone(), args);
}

pub fn na_fill_forward(ts: &Discharges, args: &CliArgs) {
    let threshold: Option<u32> = args
        .args
        .get(0)
        .map(|s| s.parse().expect("Threshold needs to be integer"));
    let nafill = ts
        .data_table
        .clone()
        .lazy()
        .with_columns(&[col(ts.discharge_col).forward_fill(threshold)])
        .collect()
        .unwrap();
    dataframe_output(nafill, args);
}

pub fn monthly_seasonality(ts: &Discharges, args: &CliArgs) {
    let seasonality = ts
        .data_table
        .clone()
        .lazy()
        .groupby(&[col(ts.datetime_col).dt().month().alias("month")])
        .agg([col("flow").mean()])
        .sort("month", SortOptions::default())
        .collect()
        .unwrap();
    dataframe_output(seasonality, args);
}

pub fn daily_seasonality(ts: &Discharges, args: &CliArgs) {
    let seasonality = ts
        .data_table
        .clone()
        .lazy()
        .groupby(&[col(ts.datetime_col).dt().ordinal_day().alias("day")])
        .agg([col("flow").mean()])
        .sort("day", SortOptions::default())
        .collect()
        .unwrap();
    dataframe_output(seasonality, args);
}

pub fn annual_mean(ts: &Discharges, args: &CliArgs) {
    let annual = ts
        .data_table
        .clone()
        .lazy()
        .groupby(&[col(ts.datetime_col).dt().year().alias("year")])
        .agg([col("flow").mean(), col("flow").count().alias("count")])
        .sort("year", SortOptions::default())
        .collect()
        .unwrap();
    dataframe_output(annual, args);
}

pub fn monthly_mean(ts: &Discharges, args: &CliArgs) {
    let monthly = ts
        .data_table
        .clone()
        .lazy()
        .groupby_stable(&[
            col(ts.datetime_col).dt().year().alias("year"),
            col(ts.datetime_col).dt().month().alias("month"),
        ])
        .agg([col("flow").mean(), col("flow").count().alias("count")])
        .collect()
        .unwrap();
    dataframe_output(monthly, args);
}

pub fn missing_data(ts: &Discharges, args: &CliArgs) {
    let df = ts
        .data_table
        .clone()
        .lazy()
        .select([
            col(ts.datetime_col).alias("start_date"),
            col(ts.discharge_col).is_null().alias("isna"),
        ])
        .with_column(
            when(col("isna").eq(col("isna").shift(1)))
                .then(lit(0))
                .otherwise(lit(1))
                .cumsum(false)
                .alias("isna_blk"),
        )
        // .filter(col("isna"))
        .groupby([col("isna_blk")])
        .agg([
            col("start_date").first(),
            col("isna_blk").count().alias("count"),
            col("isna").first(),
        ])
        .drop_columns(["isna_blk"])
        .sort("start_date", SortOptions::default())
        .collect()
        .unwrap();
    dataframe_output(df, args);
}

// pub fn run() {
//     let ts = Discharges::new("streamflow.csv", "date", "flow");
//     missing_data(&ts);
//     seasonality(&ts);
//     annual_mean(&ts);
//     monthly_mean(&ts);
// }

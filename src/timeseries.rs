use polars::{lazy::dsl::when, prelude::*};

use clap::{Args, ValueEnum, ValueHint};
use std::{fs::File, path::PathBuf};

use crate::cliargs::CliAction;

#[derive(Args)]
pub struct CliArgs {
    /// column name containing date and/or time in csv
    #[arg(long, default_value = "date")]
    datetime_col: String,
    /// column name containing discharges in csv
    #[arg(long, default_value = "flow")]
    discharge_col: String,
    /// Print in a abridged format that can't be piped
    #[arg(short, long)]
    no_pipe: bool,
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
    #[arg(short, long, default_value = "", value_delimiter = ',')]
    args: Vec<String>,
    /// input csv file
    #[arg(required = true)]
    input: PathBuf,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum TsProcess {
    #[value(alias = "na")]
    NaValues,
    #[value(alias = "nf")]
    NaFill,
    #[value(alias = "s")]
    Seasonality,
    #[value(alias = "ay")]
    AggAnnual,
    #[value(alias = "am")]
    AggMonthly,
}

impl CliAction for CliArgs {
    fn run(self) -> anyhow::Result<()> {
        let ts = Discharges::new(&self.input, &self.datetime_col, &self.discharge_col);
        match self.command {
            TsProcess::NaValues => missing_data(&ts, &self),
            TsProcess::Seasonality => seasonality(&ts, &self),
            TsProcess::AggMonthly => monthly_mean(&ts, &self),
            TsProcess::AggAnnual => annual_mean(&ts, &self),
            _ => (),
        }
        Ok(())
    }
}

fn dataframe_output(mut df: DataFrame, args: &CliArgs) {
    if let Some(output) = &args.output {
        let file = File::create(output).unwrap();
        CsvWriter::new(file).finish(&mut df).unwrap();
    } else if args.no_pipe {
        println!("{}", df);
    } else {
        let nrow = df.shape().0;
        let schema: Vec<String> = df.schema().iter().map(|s| s.0.to_string()).collect();
        println!("{}", schema.join(","));
        if df.is_empty() {
            return;
        }
        let mut row = df.get_row(0).unwrap();
        for i in 0..nrow {
            df.get_row_amortized(i, &mut row).unwrap();
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
}

pub fn seasonality(ts: &Discharges, args: &CliArgs) {
    let seasonality = ts
        .data_table
        .clone()
        .lazy()
        .groupby(&[col(ts.datetime_col).dt().month().alias("month")])
        .agg([col("flow").mean()])
        .sort("month", SortOptions::default())
        .collect()
        .unwrap();
    println!("{:#?}", seasonality);
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

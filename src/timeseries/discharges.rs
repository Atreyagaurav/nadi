use polars::export::chrono::Datelike;
use polars::prelude::*;

pub struct Discharges<'a> {
    datetime_col: &'a str,
    discharge_col: &'a str,
    data_table: DataFrame,
}

impl<'a> Discharges<'a> {
    pub fn new(filename: &str, datetime_col: &'a str, discharge_col: &'a str) -> Self {
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

pub fn seasonality(ts: &Discharges) {
    let seasonality = ts
        .data_table
        .clone()
        .lazy()
        .with_column(
            col(ts.datetime_col)
                .map(
                    |s| {
                        Ok(Some(
                            s.date()
                                .expect("series must contain date")
                                .as_date_iter()
                                .map(|d| d.map(|d| d.month()))
                                .collect(),
                        ))
                    },
                    GetOutput::from_type(DataType::Int32),
                )
                .alias("month"),
        )
        .groupby(&[col("month")])
        .agg([col("flow").mean()])
        .collect()
        .unwrap();
    println!("{:#?}", seasonality);
}

pub fn missing_data(ts: &Discharges) {
    let df = ts
        .data_table
        .clone()
        .lazy()
        .select([
            col(ts.datetime_col).alias("start_date"),
            col(ts.discharge_col).is_null().alias("isna"),
        ])
        .with_column(
            (col("isna").cast(DataType::UInt64) - lit(1))
                .cumsum(false)
                .alias("isna_blk"),
        )
        .filter(col("isna").eq(1))
        .groupby([col("isna_blk")])
        .agg([
            col("start_date").first(),
            col("isna_blk").count().alias("count"),
        ])
        .drop_columns(["isna_blk"])
        .sort("start_date", SortOptions::default())
        .collect()
        .unwrap();
    println!("{}", df);
}

pub fn run() {
    let ts = Discharges::new("streamflow.csv", "date", "flow");
    missing_data(&ts);
    seasonality(&ts);
}

use polars::{lazy::dsl::when, prelude::*};

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
        .groupby(&[col(ts.datetime_col).dt().month().alias("month")])
        .agg([col("flow").mean()])
        .sort("month", SortOptions::default())
        .collect()
        .unwrap();
    println!("{:#?}", seasonality);
}

pub fn annual_mean(ts: &Discharges) {
    let annual = ts
        .data_table
        .clone()
        .lazy()
        .groupby(&[col(ts.datetime_col).dt().year().alias("year")])
        .agg([col("flow").mean(), col("flow").count().alias("count")])
        .sort("year", SortOptions::default())
        .collect()
        .unwrap();
    println!("{:#?}", annual);
}

pub fn monthly_mean(ts: &Discharges) {
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
    println!("{:#?}", monthly);
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
    println!("{}", df);
}

pub fn run() {
    let ts = Discharges::new("streamflow.csv", "date", "flow");
    missing_data(&ts);
    seasonality(&ts);
    annual_mean(&ts);
    monthly_mean(&ts);
}

use std::collections::HashMap;
use std::time::Duration;

use log::{error, info, warn};
use serde::Serialize;
use sqlx::{Column, Error, PgPool, Pool, Postgres, Row, TypeInfo, ValueRef};
use sqlx::postgres::{PgPoolOptions, PgRow};

/// Connect to database
pub async fn connect(db: &str, username: &str, password: &str, host: &str, port: &str, db_name: &str) -> Pool<Postgres> {
    let url = format!("{}://{}:{}@{}:{}/{}", db, username, password, host, port, db_name);

    loop {
        info!("💪Trying to connect to database");

        let start_time = std::time::Instant::now();

        let pool = PgPoolOptions::new().max_connections(5).connect(&url).await;

        match pool {

            // if connected. returns pool
            Ok(pool) => {
                let used_time = start_time.elapsed().as_millis();

                info!("🏎️ Connecting: {}:{} | Database:{} | User:{}\n
⏳ Time used: {} ms", host, port, db_name, username, used_time);

                return pool;
            }

            // if not retry in 30 seconds
            Err(_) => {
                error!("🙍Connecting to database failed, retry in 30 sec...");
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        }
    }
}

/// Store values through enum
/// so SQL result can be in a single HashMap<String, SqlResult>
/// then I don't need to write struct to map the result
/// just use the Vec<HashMap<String, SqlResult>>
#[derive(Debug, Clone, Serialize)]
pub enum SqlResult {
    BOOL(bool),
    String(String),
    StringVec(Vec<String>),
    I32(i32),
    DATE(chrono::NaiveDate),
    TIME(chrono::NaiveTime),
    JSON(serde_json::Value),
    Null(),
    UnknownType(),
}

// impl SqlResult {
//     // if you want to get i32 value from SqlResult::I32 use these functions
//     // e.g. calling SqlResult::I32(12).to_i32().unwrap() returns 12
//
//     /// convert SqlResult:I32 to i32
//     pub fn to_i32(self) -> Result<i32, Error> {
//         match self {
//             SqlResult::I32(val) => { Ok(val) }
//             _ => { Err(Error::new(ErrorKind::InvalidInput, "Can only use this function on SqlResult::I32")) }
//         }
//     }
//
//     /// convert SqlResult::String to String
//     pub fn to_string(self) -> Result<String, Error> {
//         match self {
//             SqlResult::String(val) => { Ok(val) }
//             // if null then return ""
//             SqlResult::Null() => { Ok(String::new()) }
//             _ => { Err(Error::new(ErrorKind::InvalidInput, "Can only use this function on SqlResult::String")) }
//         }
//     }
//
//     /// convert SqlResult:BOOL to bool
//     pub fn to_bool(self) -> Result<bool, Error> {
//         match self {
//             SqlResult::BOOL(val) => { Ok(val) }
//             _ => { Err(Error::new(ErrorKind::InvalidInput, "Can only use this function on SqlResult::BOOL")) }
//         }
//     }
//
//     /// convert SqlResult:DATE to chrono::NaiveDate
//     pub fn to_date(self) -> Result<chrono::NaiveDate, Error> {
//         match self {
//             SqlResult::DATE(val) => { Ok(val) }
//             _ => { Err(Error::new(ErrorKind::InvalidInput, "Can only use this function on SqlResult::DATE")) }
//         }
//     }
//
//     /// convert SqlResult:TIME to chrono::NaiveTime
//     pub fn to_time(self) -> Result<chrono::NaiveTime, Error> {
//         match self {
//             SqlResult::TIME(val) => { Ok(val) }
//             _ => { Err(Error::new(ErrorKind::InvalidInput, "Can only use this function on SqlResult::TIME")) }
//         }
//     }
// }

/// convert PgRow into HashMap<String, SqlResult>
fn into_hashmap(row: PgRow) -> HashMap<String, SqlResult> {
    let mut result: HashMap<String, SqlResult> = HashMap::new();

    for column in row.columns() {
        // print column data types
        // info!("{:16} - {}",column.name(),column.type_info().name());

        let result_value: SqlResult;

        // if the value is null
        if row.try_get_raw(column.name()).unwrap().is_null() {
            result_value = SqlResult::Null();
        } else {
            // if the value is not null
            // use match to get desired type value and put it into hashmap
            // info!("typeinfo: {}" , column.type_info().name());
            match column.type_info().name() {
                "BOOL" => {
                    let value: bool = row.get(column.name());
                    result_value = SqlResult::BOOL(value);
                }
                "TEXT" => {
                    let value: String = row.get(column.name());
                    result_value = SqlResult::String(value);
                }
                "TEXT[]" => {
                    let value: Vec<String> = row.get(column.name());
                    result_value = SqlResult::StringVec(value);
                }
                "INT4" => {
                    let value: i32 = row.get(column.name());
                    result_value = SqlResult::I32(value);
                }
                "DATE" => {
                    let value: chrono::NaiveDate = row.get(column.name());
                    result_value = SqlResult::DATE(value);
                }
                "TIME" => {
                    let value: chrono::NaiveTime = row.get(column.name());
                    result_value = SqlResult::TIME(value);
                }
                "JSON" => {
                    let value: serde_json::Value = row.get(column.name());
                    result_value = SqlResult::JSON(value);
                }
                "StringVec" => {
                    let value: Vec<String> = row.get(column.name());
                    result_value = SqlResult::StringVec(value);
                }
                _ => {
                    info!("Can't convert SQL type {} to Rust type.", column.type_info().name());
                    result_value = SqlResult::UnknownType();
                }
            }
        }
        result.insert(column.name().to_string(), result_value);
    }

    return result;
}

/// Query SQL and return Vec<HashMap<String, SqlResults>>
pub async fn query(pool: &PgPool, sql: &String) -> Result<Vec<HashMap<String, SqlResult>>, sqlx::Error> {
    let start_time = std::time::Instant::now();


    let rows = sqlx::query(sql)
        .fetch_all(pool)
        .await?;

    let mut results = Vec::new();

    for row in rows {
        let result = into_hashmap(row);
        results.push(result);
    }

    let used_time = start_time.elapsed().as_millis();

    info!("\
🔍 Querying: {} \n
🎉 Result: {:?} \n
⌚️ Time used: {} ms ", sql, results, used_time );

    Ok(results)
}
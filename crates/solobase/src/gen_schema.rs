//! Generates the JSON Schema for solobase.json and writes it to stdout.

use schemars::schema_for;
use solobase::app_config::AppConfig;

fn main() {
    let schema = schema_for!(AppConfig);
    let json = serde_json::to_string_pretty(&schema).expect("failed to serialize schema");
    println!("{json}");
}

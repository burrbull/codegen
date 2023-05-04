use crate::cubemx::Db;
use anyhow::Result;
//use once_cell::sync::Lazy;
//use regex::Regex;
use serde::Deserialize;
use std::path::PathBuf;

pub fn load(db: &Db, version: &str) -> Result<Ip> {
    let name = format!("DMA-{}_Modes", version);
    let ip_path: PathBuf = ["IP", &name].iter().collect();
    db.load_mcu(&ip_path)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Ip {
    pub version: String,
    #[serde(rename = "RefMode")]
    pub modes: Vec<RefMode>,
    #[serde(rename = "ModeLogicOperator")]
    pub dmas: Vec<ModeLogicOperator>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RefMode {
    #[serde(rename = "BaseMode")]
    pub basemode: String,
    pub name: String,
    #[serde(rename = "Parameter", default)]
    pub parameters: Vec<Parameter>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Parameter {
    pub name: String,
    #[serde(rename = "PossibleValue", default)]
    pub possible_values: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ModeLogicOperator {
    pub name: String,
    #[serde(rename = "Mode")]
    pub modes: Vec<Mode>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Mode {
    pub name: String,
    #[serde(rename = "ModeLogicOperator", default)]
    pub operators: Vec<ModeLogicOperator>,
}

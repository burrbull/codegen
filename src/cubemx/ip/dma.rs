use crate::cubemx::Db;
use anyhow::Result;
use serde::Deserialize;
use std::{path::PathBuf, fmt::Display, str::FromStr};

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
    pub name: Logic,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Logic {
    Or,
    And,
    Xor
}

impl FromStr for Logic {
    type Err = ();
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "OR" => Ok(Self::Or),
            "AND" => Ok(Self::And),
            "XOR" => Ok(Self::Xor),
            _ => Err(()),
        }
    }
}

impl Display for Logic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Or => f.write_str("OR"),
            Self::And => f.write_str("AND"),
            Self::Xor => f.write_str("XOR"),
        }
    }
}

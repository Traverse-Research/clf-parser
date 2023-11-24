#![doc = include_str!("../README.md")]

pub mod clf;
mod deserialize;

use anyhow::{Context, Result};
use clf::ProcessList;
use std::io::Read;

pub fn load_clf(reader: impl Read) -> Result<ProcessList> {
    let clf: ProcessList = serde_xml_rs::de::from_reader(reader).context("CLF parsing failed.")?;
    clf.validate()?;
    Ok(clf)
}

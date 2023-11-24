use crate::deserialize::deserialize_space_separated;
use anyhow::{bail, ensure, Context, Result};

#[derive(
    Debug,
    serde::Deserialize,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
    PartialEq,
    Clone,
    Copy,
)]
#[archive_attr(derive(bytecheck::CheckBytes))]
pub enum BitDepth {
    #[serde(rename = "8i")]
    I8,
    #[serde(rename = "10i")]
    I10,
    #[serde(rename = "12i")]
    I12,
    #[serde(rename = "16i")]
    I16,
    #[serde(rename = "16f")]
    F16,
    #[serde(rename = "32f")]
    F32,
}

#[derive(
    Debug, serde::Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, PartialEq,
)]
#[archive_attr(derive(bytecheck::CheckBytes))]
#[serde(rename_all = "camelCase")]
pub struct OperatorBitDepth {
    in_bit_depth: BitDepth,
    out_bit_depth: BitDepth,
}

impl OperatorBitDepth {
    fn validate(&self) -> Result<()> {
        ensure!(
            self.in_bit_depth != self.out_bit_depth || self.out_bit_depth != BitDepth::F32,
            "currently only BitDepth::F32 is supported"
        );
        Ok(())
    }
}

#[derive(Debug, serde::Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[archive_attr(derive(bytecheck::CheckBytes))]
pub struct Array {
    #[serde(deserialize_with = "deserialize_space_separated")]
    pub dim: Vec<u32>,
    #[serde(rename = "$value", deserialize_with = "deserialize_space_separated")]
    pub data: Vec<f32>,
}

/// Range attribute scale + offset
///
/// <https://docs.acescentral.com/specifications/clf#range>
/// <https://github.com/AcademySoftwareFoundation/OpenColorIO/blob/9078753990d7f976a0bfcd55cfa63f2e1de3a53b/src/OpenColorIO/ops/range/RangeOpData.cpp#L474>
#[derive(Debug, serde::Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[archive_attr(derive(bytecheck::CheckBytes))]
#[serde(rename_all = "camelCase")]
pub struct Range {
    #[serde(flatten)]
    bit_depth: OperatorBitDepth,
    min_in_value: f32,
    max_in_value: f32,
    min_out_value: f32,
    max_out_value: f32,
}

impl Range {
    pub fn scale(&self) -> f32 {
        (self.max_out_value - self.min_out_value) / (self.max_in_value - self.min_in_value)
    }

    pub fn offset(&self) -> f32 {
        self.min_out_value - self.scale() * self.min_in_value
    }
}

#[derive(Debug, serde::Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[archive_attr(derive(bytecheck::CheckBytes))]
#[serde(rename = "camelCase")]
pub struct Lut1d {
    #[serde(flatten)]
    pub bit_depth: OperatorBitDepth,
    #[serde(rename = "Array")]
    pub array: Array,
}

impl Lut1d {
    pub fn validate(&self) -> Result<()> {
        self.bit_depth.validate()?;

        if self.array.dim.len() != 2 {
            bail!("A dim attribute defined for a Lut1d should have 2 elements.")
        }

        if self.array.dim.iter().product::<u32>() != self.array.data.len() as u32 {
            bail!(
                "expected {} elements, got {}",
                self.array.dim.iter().product::<u32>(),
                self.array.data.len()
            )
        }
        Ok(())
    }
}

#[derive(Debug, serde::Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[archive_attr(derive(bytecheck::CheckBytes))]
#[serde(rename_all = "camelCase")]
pub struct Lut3d {
    #[serde(flatten)]
    pub bit_depth: OperatorBitDepth,
    #[serde(rename = "Array")]
    pub array: Array,
}

impl Lut3d {
    fn validate(&self) -> Result<()> {
        self.bit_depth.validate()?;

        if self.array.dim.len() == 4 {
            if self.array.dim[0] != self.array.dim[1] || self.array.dim[0] != self.array.dim[2] {
                bail!("A Lut3d should have the same dimensions on all three axes.")
            }
            if self.array.dim[3] != 3 {
                bail!("A Lut3d should have 3 color components.")
            }
        } else {
            bail!("A dim attribute defined for a Lut3D should have 4 elements.")
        }

        if self.array.dim.iter().product::<u32>() != self.array.data.len() as u32 {
            bail!(
                "expected {} elements, got {}",
                self.array.dim.iter().product::<u32>(),
                self.array.data.len()
            )
        }
        Ok(())
    }
}

#[derive(Debug, serde::Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[archive_attr(derive(bytecheck::CheckBytes))]
#[serde(rename_all = "camelCase")]
pub struct ProcessList {
    #[serde(rename = "compCLFversion")]
    pub comp_clf_version: u32,
    pub id: String,
    #[serde(rename = "$value")]
    pub operators: Vec<Operator>,
}

impl ProcessList {
    pub fn validate(&self) -> Result<()> {
        if self.comp_clf_version > 3 {
            bail!(
                "CLF versions higher than 3 (currently parsing {}) are not yet supported",
                self.comp_clf_version
            )
        }

        for x in &self.operators {
            x.validate()?
        }

        Ok(())
    }
}

#[derive(Debug, serde::Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
#[archive_attr(derive(bytecheck::CheckBytes))]
#[serde(deny_unknown_fields)]
pub enum Operator {
    Range(Range),
    #[serde(rename = "LUT1D")]
    Lut1D(Lut1d),
    #[serde(rename = "LUT3D")]
    Lut3D(Lut3d),
}

impl Operator {
    fn validate(&self) -> Result<()> {
        match self {
            Self::Range(range) => range
                .bit_depth
                .validate()
                .context("Range operator invalid."),
            Self::Lut1D(lut1d) => lut1d.validate().context("LUT 1D operator invalid."),
            Self::Lut3D(lut3d) => lut3d.validate().context("LUT 3D operator invalid."),
        }
    }
}

use crate::deserialize::deserialize_space_separated;
use anyhow::{bail, ensure, Context, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    archive_attr(derive(bytecheck::CheckBytes))
)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    archive_attr(derive(bytecheck::CheckBytes))
)]
#[serde(rename_all = "camelCase")]
pub struct OperatorBitDepth {
    in_bit_depth: BitDepth,
    out_bit_depth: BitDepth,
}

impl OperatorBitDepth {
    fn validate(&self) -> Result<()> {
        ensure!(
            self.in_bit_depth == self.out_bit_depth && self.out_bit_depth == BitDepth::F32,
            "currently only BitDepth::F32 is supported, found in bit depth: {:?}, out bit depth: {:?}", self.in_bit_depth, self.out_bit_depth
        );
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    archive_attr(derive(bytecheck::CheckBytes))
)]
pub struct Array {
    #[serde(deserialize_with = "deserialize_space_separated")]
    pub dim: Vec<u32>,
    #[serde(rename = "$value", deserialize_with = "deserialize_space_separated")]
    pub data: Vec<f32>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    archive_attr(derive(bytecheck::CheckBytes))
)]
pub enum RangeStyle {
    #[default]
    Clamp,
    #[serde(rename = "noClamp")]
    NoClamp,
}

/// Range attribute scale + offset
///
/// <https://docs.acescentral.com/specifications/clf#range>
/// <https://github.com/AcademySoftwareFoundation/OpenColorIO/blob/9078753990d7f976a0bfcd55cfa63f2e1de3a53b/src/OpenColorIO/ops/range/RangeOpData.cpp#L474>
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    archive_attr(derive(bytecheck::CheckBytes))
)]
#[serde(rename_all = "camelCase")]
pub struct Range {
    #[serde(flatten)]
    bit_depth: OperatorBitDepth,
    #[serde(default)]
    style: RangeStyle,
    min_in_value: Option<f32>,
    max_in_value: Option<f32>,
    min_out_value: Option<f32>,
    max_out_value: Option<f32>,
}

impl Range {
    fn validate(&self) -> Result<()> {
        self.bit_depth.validate()?;
        ensure!(
            self.min_in_value.is_some() == self.min_out_value.is_some(),
            "minInValue and minOutValue must be specified together"
        );
        ensure!(
            self.max_in_value.is_some() == self.max_out_value.is_some(),
            "maxInValue and maxOutValue must be specified together"
        );
        ensure!(
            self.min_in_value.is_some() || self.max_in_value.is_some(),
            "A Range needs minimum values, maximum values, or both"
        );
        if let (Some(min_in), Some(max_in)) = (self.min_in_value, self.max_in_value) {
            ensure!(
                min_in != max_in,
                "minInValue and maxInValue must differ, both are {min_in}"
            );
        }
        if self.style == RangeStyle::NoClamp {
            ensure!(
                self.min_in_value.is_some() && self.max_in_value.is_some(),
                "A noClamp Range needs both minimum and maximum values"
            );
        }
        Ok(())
    }

    pub fn style(&self) -> RangeStyle {
        self.style
    }

    /// A Range with only minimum or only maximum values does not scale.
    pub fn scale(&self) -> f32 {
        match (
            self.min_in_value,
            self.max_in_value,
            self.min_out_value,
            self.max_out_value,
        ) {
            (Some(min_in), Some(max_in), Some(min_out), Some(max_out)) => {
                (max_out - min_out) / (max_in - min_in)
            }
            _ => 1.0,
        }
    }

    pub fn offset(&self) -> f32 {
        match (self.min_in_value, self.min_out_value) {
            (Some(min_in), Some(min_out)) => min_out - self.scale() * min_in,
            _ => self.max_out_value.unwrap_or(0.0) - self.max_in_value.unwrap_or(0.0),
        }
    }

    /// Lower bound of the output, if the Range clamps it.
    pub fn clamp_min(&self) -> Option<f32> {
        self.min_out_value
            .filter(|_| self.style == RangeStyle::Clamp)
    }

    /// Upper bound of the output, if the Range clamps it.
    pub fn clamp_max(&self) -> Option<f32> {
        self.max_out_value
            .filter(|_| self.style == RangeStyle::Clamp)
    }
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    archive_attr(derive(bytecheck::CheckBytes))
)]
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
            bail!(
                "A dim attribute defined for a Lut1d should have 2 elements; {} were found instead.",
                self.array.dim.len()
            )
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

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    archive_attr(derive(bytecheck::CheckBytes))
)]
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
                bail!(
                    "A Lut3d should have the same dimensions on all three axes. Found {:?} instead.",
                    self.array.dim
                )
            }
            if self.array.dim[3] != 3 {
                bail!(
                    "A Lut3d should have 3 color components; {} were found instead.",
                    self.array.dim[3]
                )
            }
        } else {
            bail!("A dim attribute defined for a Lut3D should have 4 elements; {} were found instead.", self.array.dim.len())
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

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    archive_attr(derive(bytecheck::CheckBytes))
)]
#[serde(rename_all = "camelCase")]
pub struct Matrix {
    #[serde(flatten)]
    pub bit_depth: OperatorBitDepth,
    #[serde(rename = "Array")]
    pub array: Array,
}

impl Matrix {
    pub fn validate(&self) -> Result<()> {
        self.bit_depth.validate()?;
        ensure!(
            self.array.dim.len() == 2,
            "The matrix array dimensions needs to have two components."
        );
        ensure!(
            self.array.dim[0] == 3,
            "The CLF specification states that only 3x3 or 3x4 matrices are supported."
        );
        ensure!(
            self.array.dim[1] == 3 || self.array.dim[1] == 4,
            "The CLF specification states that only 3x3 or 3x4 matrices are supported."
        );

        Ok(())
    }
}

/// <https://docs.acescentral.com/specifications/clf#log>
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    archive_attr(derive(bytecheck::CheckBytes))
)]
#[serde(rename_all = "camelCase")]
pub enum LogStyle {
    Log10,
    AntiLog10,
    Log2,
    AntiLog2,
    LinToLog,
    LogToLin,
    CameraLinToLog,
    CameraLogToLin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    archive_attr(derive(bytecheck::CheckBytes))
)]
pub enum Channel {
    R,
    G,
    B,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    archive_attr(derive(bytecheck::CheckBytes))
)]
#[serde(rename_all = "camelCase")]
pub struct LogParams {
    /// Set when the parameters apply to a single channel, which is not supported.
    pub channel: Option<Channel>,
    pub base: Option<f32>,
    pub log_side_slope: Option<f32>,
    pub log_side_offset: Option<f32>,
    pub lin_side_slope: Option<f32>,
    pub lin_side_offset: Option<f32>,
    pub lin_side_break: Option<f32>,
    pub linear_slope: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    archive_attr(derive(bytecheck::CheckBytes))
)]
#[serde(rename_all = "camelCase")]
pub struct Log {
    #[serde(flatten)]
    pub bit_depth: OperatorBitDepth,
    pub style: LogStyle,
    /// Per-channel `LogParams` are not supported; one element applies to all channels.
    #[serde(rename = "LogParams")]
    pub params: Option<LogParams>,
}

/// The parameters of a Log operator in the general form of the CLF specification, with the
/// defaults of its style applied.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedLogParams {
    /// `true` when the operator converts linear to log.
    pub lin_to_log: bool,
    pub base: f32,
    pub log_side_slope: f32,
    pub log_side_offset: f32,
    pub lin_side_slope: f32,
    pub lin_side_offset: f32,
    /// The linear segment below `lin_side_break` of the camera styles.
    pub linear_segment: Option<LinearSegment>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearSegment {
    pub lin_side_break: f32,
    pub linear_slope: f32,
    pub linear_offset: f32,
}

impl Log {
    pub fn validate(&self) -> Result<()> {
        self.bit_depth.validate()?;
        ensure!(
            self.params.and_then(|p| p.channel).is_none(),
            "Per-channel LogParams are not supported"
        );
        if matches!(
            self.style,
            LogStyle::CameraLinToLog | LogStyle::CameraLogToLin
        ) {
            ensure!(
                self.params.is_some_and(|p| p.lin_side_break.is_some()),
                "The camera Log styles require linSideBreak"
            );
        }
        let params = self.resolved_params();
        ensure!(
            params.base > 0.0 && params.base != 1.0,
            "The Log base must be positive and not 1, found {}",
            params.base
        );
        if let Some(segment) = params.linear_segment {
            let inner = params.lin_side_slope * segment.lin_side_break + params.lin_side_offset;
            ensure!(
                inner > 0.0,
                "linSideSlope * linSideBreak + linSideOffset must be positive, found {inner}"
            );
        }
        Ok(())
    }

    pub fn resolved_params(&self) -> ResolvedLogParams {
        let p = self.params.unwrap_or_default();
        let (lin_to_log, base) = match self.style {
            LogStyle::Log10 => (true, 10.0),
            LogStyle::AntiLog10 => (false, 10.0),
            LogStyle::Log2 => (true, 2.0),
            LogStyle::AntiLog2 => (false, 2.0),
            LogStyle::LinToLog | LogStyle::CameraLinToLog => (true, p.base.unwrap_or(2.0)),
            LogStyle::LogToLin | LogStyle::CameraLogToLin => (false, p.base.unwrap_or(2.0)),
        };
        let parametric = !matches!(
            self.style,
            LogStyle::Log10 | LogStyle::AntiLog10 | LogStyle::Log2 | LogStyle::AntiLog2
        );
        let param = |v: Option<f32>, default: f32| {
            if parametric {
                v.unwrap_or(default)
            } else {
                default
            }
        };
        let log_side_slope = param(p.log_side_slope, 1.0);
        let log_side_offset = param(p.log_side_offset, 0.0);
        let lin_side_slope = param(p.lin_side_slope, 1.0);
        let lin_side_offset = param(p.lin_side_offset, 0.0);

        let linear_segment = match self.style {
            LogStyle::CameraLinToLog | LogStyle::CameraLogToLin => {
                let lin_side_break = p.lin_side_break.unwrap_or_default();
                let inner = lin_side_slope * lin_side_break + lin_side_offset;
                let linear_slope = p
                    .linear_slope
                    .unwrap_or(log_side_slope * lin_side_slope / (inner * base.ln()));
                let log_side_break = log_side_slope * inner.log(base) + log_side_offset;
                Some(LinearSegment {
                    lin_side_break,
                    linear_slope,
                    linear_offset: log_side_break - linear_slope * lin_side_break,
                })
            }
            _ => None,
        };

        ResolvedLogParams {
            lin_to_log,
            base,
            log_side_slope,
            log_side_offset,
            lin_side_slope,
            lin_side_offset,
            linear_segment,
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    archive_attr(derive(bytecheck::CheckBytes))
)]
#[serde(from = "RawProcessList")]
pub struct ProcessList {
    pub comp_clf_version: f32,
    pub id: String,
    pub operators: Vec<Operator>,
}

/// The elements of a `ProcessList`, including the metadata elements that carry no operation.
/// Keep the operator variants in sync with [`Operator`].
#[derive(serde::Deserialize)]
enum RawElement {
    Range(Range),
    Matrix(Matrix),
    Log(Log),
    #[serde(rename = "LUT1D")]
    Lut1D(Lut1d),
    #[serde(rename = "LUT3D")]
    Lut3D(Lut3d),
    Description(serde::de::IgnoredAny),
    InputDescriptor(serde::de::IgnoredAny),
    OutputDescriptor(serde::de::IgnoredAny),
    Info(serde::de::IgnoredAny),
}

#[derive(serde::Deserialize)]
struct RawProcessList {
    #[serde(rename = "compCLFversion")]
    comp_clf_version: f32,
    id: String,
    #[serde(rename = "$value")]
    elements: Vec<RawElement>,
}

impl From<RawProcessList> for ProcessList {
    fn from(raw: RawProcessList) -> Self {
        let operators = raw
            .elements
            .into_iter()
            .filter_map(|element| match element {
                RawElement::Range(range) => Some(Operator::Range(range)),
                RawElement::Matrix(matrix) => Some(Operator::Matrix(matrix)),
                RawElement::Log(log) => Some(Operator::Log(log)),
                RawElement::Lut1D(lut1d) => Some(Operator::Lut1D(lut1d)),
                RawElement::Lut3D(lut3d) => Some(Operator::Lut3D(lut3d)),
                RawElement::Description(_)
                | RawElement::InputDescriptor(_)
                | RawElement::OutputDescriptor(_)
                | RawElement::Info(_) => None,
            })
            .collect();
        Self {
            comp_clf_version: raw.comp_clf_version,
            id: raw.id,
            operators,
        }
    }
}

impl ProcessList {
    pub fn validate(&self) -> Result<()> {
        if self.comp_clf_version >= 4.0 {
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

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    archive_attr(derive(bytecheck::CheckBytes))
)]
pub enum Operator {
    Range(Range),
    Matrix(Matrix),
    Log(Log),
    #[serde(rename = "LUT1D")]
    Lut1D(Lut1d),
    #[serde(rename = "LUT3D")]
    Lut3D(Lut3d),
}

impl Operator {
    fn validate(&self) -> Result<()> {
        match self {
            Self::Range(range) => range.validate().context("Range operator invalid."),
            Self::Lut1D(lut1d) => lut1d.validate().context("LUT 1D operator invalid."),
            Self::Lut3D(lut3d) => lut3d.validate().context("LUT 3D operator invalid."),
            Self::Matrix(matrix) => matrix.validate().context("Matrix operator invalid."),
            Self::Log(log) => log.validate().context("Log operator invalid."),
        }
    }
}

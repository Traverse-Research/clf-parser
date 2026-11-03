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

#[cfg(test)]
mod tests {
    use super::*;
    use clf::{LogStyle, Operator, RangeStyle};

    const ACESCCT_LOOK: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ProcessList xmlns="urn:AMPAS:CLF:v3.0" id="look" name="Look" compCLFversion="3.0">
    <Description>A look in ACES2065-1</Description>
    <InputDescriptor>ACES2065-1 AP0</InputDescriptor>
    <OutputDescriptor>ACES2065-1 AP0</OutputDescriptor>
    <Info>
        <Revision>1</Revision>
    </Info>
    <Matrix inBitDepth="32f" outBitDepth="32f">
        <Description>AP0 to AP1</Description>
        <Array dim="3 3">
             1.4514393 -0.2365107 -0.2149286
            -0.0765538  1.1762297 -0.0996759
             0.0083161 -0.0060324  0.9977163
        </Array>
    </Matrix>
    <Range inBitDepth="32f" outBitDepth="32f" style="Clamp">
        <maxInValue>65504.0</maxInValue>
        <maxOutValue>65504.0</maxOutValue>
    </Range>
    <Log inBitDepth="32f" outBitDepth="32f" style="cameraLinToLog">
        <LogParams base="2" logSideSlope="0.05707762557" logSideOffset="0.5547945205" linSideBreak="0.0078125" />
    </Log>
    <Range inBitDepth="32f" outBitDepth="32f">
        <minInValue>0.0</minInValue>
        <maxInValue>1.4679964</maxInValue>
        <minOutValue>0.0</minOutValue>
        <maxOutValue>1.0</maxOutValue>
    </Range>
    <LUT1D inBitDepth="32f" outBitDepth="32f">
        <Array dim="2 1">
            0.0
            1.4679964
        </Array>
    </LUT1D>
    <Log inBitDepth="32f" outBitDepth="32f" style="cameraLogToLin">
        <LogParams base="2" logSideSlope="0.05707762557" logSideOffset="0.5547945205" linSideBreak="0.0078125" />
    </Log>
</ProcessList>"#;

    #[test]
    fn parses_metadata_matrix_log_and_clamp_range() {
        let clf = load_clf(ACESCCT_LOOK.as_bytes()).unwrap();
        assert_eq!(clf.comp_clf_version, 3.0);
        assert_eq!(clf.operators.len(), 6);
        assert!(matches!(clf.operators[0], Operator::Matrix(_)));

        let Operator::Range(clamp) = &clf.operators[1] else {
            panic!("expected a Range");
        };
        assert_eq!(clamp.style(), RangeStyle::Clamp);
        assert_eq!((clamp.scale(), clamp.offset()), (1.0, 0.0));
        assert_eq!(
            (clamp.clamp_min(), clamp.clamp_max()),
            (None, Some(65504.0))
        );

        let Operator::Range(scale) = &clf.operators[3] else {
            panic!("expected a Range");
        };
        assert!((scale.scale() - 1.0 / 1.4679964).abs() < 1e-6);
        assert_eq!(
            (scale.clamp_min(), scale.clamp_max()),
            (Some(0.0), Some(1.0))
        );
    }

    #[test]
    fn camera_log_params_match_acescct() {
        let clf = load_clf(ACESCCT_LOOK.as_bytes()).unwrap();
        let Operator::Log(log) = &clf.operators[2] else {
            panic!("expected a Log");
        };
        assert_eq!(log.style, LogStyle::CameraLinToLog);
        let params = log.resolved_params();
        assert!(params.lin_to_log);
        let segment = params.linear_segment.unwrap();
        // ACEScct: 10.5402377416545 * x + 0.0729055341958355 below the break.
        assert!((segment.linear_slope - 10.540_238).abs() < 1e-4);
        assert!((segment.linear_offset - 0.072_905_53).abs() < 1e-6);

        let Operator::Log(inverse) = &clf.operators[5] else {
            panic!("expected a Log");
        };
        assert!(!inverse.resolved_params().lin_to_log);
    }

    fn load_error(operator: &str) -> String {
        let clf = format!(r#"<ProcessList id="x" compCLFversion="3">{operator}</ProcessList>"#);
        format!("{:#}", load_clf(clf.as_bytes()).unwrap_err())
    }

    #[test]
    fn camera_log_requires_lin_side_break() {
        let error = load_error(
            r#"<Log inBitDepth="32f" outBitDepth="32f" style="cameraLinToLog">
                <LogParams base="2" />
            </Log>"#,
        );
        assert!(error.contains("require linSideBreak"), "{error}");
    }

    #[test]
    fn rejects_per_channel_log_params() {
        let error = load_error(
            r#"<Log inBitDepth="32f" outBitDepth="32f" style="log2">
                <LogParams channel="R" />
            </Log>"#,
        );
        assert!(error.contains("Per-channel LogParams"), "{error}");
    }

    #[test]
    fn rejects_log_base_one() {
        let error = load_error(
            r#"<Log inBitDepth="32f" outBitDepth="32f" style="linToLog">
                <LogParams base="1" />
            </Log>"#,
        );
        assert!(error.contains("Log base"), "{error}");
    }

    #[test]
    fn rejects_camera_log_with_non_positive_break() {
        let error = load_error(
            r#"<Log inBitDepth="32f" outBitDepth="32f" style="cameraLinToLog">
                <LogParams linSideBreak="0.0" />
            </Log>"#,
        );
        assert!(error.contains("must be positive"), "{error}");
    }

    #[test]
    fn rejects_empty_input_range() {
        let error = load_error(
            r#"<Range inBitDepth="32f" outBitDepth="32f">
                <minInValue>0.5</minInValue>
                <maxInValue>0.5</maxInValue>
                <minOutValue>0.0</minOutValue>
                <maxOutValue>1.0</maxOutValue>
            </Range>"#,
        );
        assert!(error.contains("must differ"), "{error}");
    }

    #[test]
    fn rejects_one_sided_no_clamp_range() {
        let error = load_error(
            r#"<Range inBitDepth="32f" outBitDepth="32f" style="noClamp">
                <maxInValue>1.0</maxInValue>
                <maxOutValue>1.0</maxOutValue>
            </Range>"#,
        );
        assert!(error.contains("noClamp Range"), "{error}");
    }

    #[test]
    fn rejects_matrix_with_too_few_values() {
        let error = load_error(
            r#"<Matrix inBitDepth="32f" outBitDepth="32f">
                <Array dim="3 3">1 0 0 0 1 0 0 0</Array>
            </Matrix>"#,
        );
        assert!(error.contains("expected 9 elements, got 8"), "{error}");
    }

    #[test]
    fn rejects_3x4_matrix_with_3x3_values() {
        let error = load_error(
            r#"<Matrix inBitDepth="32f" outBitDepth="32f">
                <Array dim="3 4">1 0 0 0 1 0 0 0 1</Array>
            </Matrix>"#,
        );
        assert!(error.contains("expected 12 elements, got 9"), "{error}");
    }
}

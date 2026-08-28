use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Read};
use std::path::Path;

const HELP: &str = r#"aso-lint

USAGE:
  aso-lint validate --input PATH
  aso-lint experiment --control-conversions N --control-visitors N \
    --variant-conversions N --variant-visitors N [--alpha NUMBER]

Use --input - to read listing JSON from stdin.
"#;

#[derive(Debug, Eq, PartialEq)]
pub struct CliOutcome {
    pub output: String,
    pub exit_code: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Unit {
    Characters,
    Bytes,
}

impl Unit {
    fn as_str(self) -> &'static str {
        match self {
            Self::Characters => "characters",
            Self::Bytes => "bytes",
        }
    }

    fn count(self, value: &str) -> usize {
        match self {
            Self::Characters => value.chars().count(),
            Self::Bytes => value.len(),
        }
    }
}

pub fn run_cli<I>(args: I) -> Result<CliOutcome, String>
where
    I: IntoIterator<Item = String>,
{
    let args: Vec<String> = args.into_iter().collect();
    let Some(command) = args.first().map(String::as_str) else {
        return Ok(success(HELP));
    };
    match command {
        "help" | "--help" | "-h" => Ok(success(HELP)),
        "validate" => validate_command(&args[1..]),
        "experiment" => experiment_command(&args[1..]),
        _ => Err(format!("unknown command {command:?}\n\n{HELP}")),
    }
}

fn validate_command(args: &[String]) -> Result<CliOutcome, String> {
    let options = parse_options(args)?;
    reject_unknown(&options, &["--input"])?;
    let input = required(&options, "--input")?;
    let text = read_input(input)?;
    let listing: Value =
        serde_json::from_str(&text).map_err(|error| format!("invalid listing JSON: {error}"))?;
    let report = validate_listing(&listing)?;
    let valid = report
        .get("valid")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Ok(CliOutcome {
        output: pretty(&report)?,
        exit_code: if valid { 0 } else { 2 },
    })
}

fn experiment_command(args: &[String]) -> Result<CliOutcome, String> {
    let options = parse_options(args)?;
    reject_unknown(
        &options,
        &[
            "--control-conversions",
            "--control-visitors",
            "--variant-conversions",
            "--variant-visitors",
            "--alpha",
        ],
    )?;
    let control_conversions = parse_u64(
        required(&options, "--control-conversions")?,
        "control conversions",
    )?;
    let control_visitors = parse_u64(
        required(&options, "--control-visitors")?,
        "control visitors",
    )?;
    let variant_conversions = parse_u64(
        required(&options, "--variant-conversions")?,
        "variant conversions",
    )?;
    let variant_visitors = parse_u64(
        required(&options, "--variant-visitors")?,
        "variant visitors",
    )?;
    let alpha = options
        .get("--alpha")
        .map_or(Ok(0.05), |value| parse_f64(value, "alpha"))?;
    let report = analyze_experiment(
        control_conversions,
        control_visitors,
        variant_conversions,
        variant_visitors,
        alpha,
    )?;
    Ok(CliOutcome {
        output: pretty(&report)?,
        exit_code: 0,
    })
}

pub fn validate_listing(listing: &Value) -> Result<Value, String> {
    let object = listing
        .as_object()
        .ok_or_else(|| "listing JSON root must be an object".to_owned())?;
    let platform =
        text_field(object, "platform")?.ok_or_else(|| "platform is required".to_owned())?;
    let mut checks = Vec::new();
    let mut warnings = Vec::new();

    match platform {
        "apple" => validate_apple(object, &mut checks, &mut warnings)?,
        "google" => validate_google(object, &mut checks, &mut warnings)?,
        _ => return Err("platform must be apple or google".to_owned()),
    }

    let allowed = allowed_fields(platform);
    for key in object.keys() {
        if !allowed.contains(key.as_str()) {
            warnings.push(format!("unknown field {key:?} was not validated"));
        }
    }

    let valid = checks
        .iter()
        .all(|check| check.get("status").and_then(Value::as_str) != Some("error"));
    Ok(json!({
        "schema_version": 1,
        "platform": platform,
        "locale": object.get("locale").and_then(Value::as_str),
        "valid": valid,
        "checks": checks,
        "warnings": warnings,
        "note": "Structural validation only; platform review and ASO performance are not inferred."
    }))
}

fn validate_apple(
    object: &Map<String, Value>,
    checks: &mut Vec<Value>,
    warnings: &mut Vec<String>,
) -> Result<(), String> {
    check_text(object, checks, "name", true, 30, Unit::Characters)?;
    check_text(object, checks, "subtitle", false, 30, Unit::Characters)?;
    check_text(
        object,
        checks,
        "promotional_text",
        false,
        170,
        Unit::Characters,
    )?;
    check_text(object, checks, "description", true, 4_000, Unit::Characters)?;
    check_text(object, checks, "keywords", true, 100, Unit::Bytes)?;
    check_text(object, checks, "whats_new", false, 4_000, Unit::Characters)?;
    validate_apple_keywords(object, checks, warnings)
}

fn validate_google(
    object: &Map<String, Value>,
    checks: &mut Vec<Value>,
    _warnings: &mut Vec<String>,
) -> Result<(), String> {
    check_text(object, checks, "name", true, 30, Unit::Characters)?;
    check_text(
        object,
        checks,
        "short_description",
        true,
        80,
        Unit::Characters,
    )?;
    check_text(
        object,
        checks,
        "full_description",
        true,
        4_000,
        Unit::Characters,
    )
}

fn check_text(
    object: &Map<String, Value>,
    checks: &mut Vec<Value>,
    field: &str,
    required: bool,
    limit: usize,
    unit: Unit,
) -> Result<(), String> {
    let value = text_field(object, field)?;
    let Some(value) = value else {
        if required {
            checks.push(json!({
                "field": field,
                "status": "error",
                "used": 0,
                "limit": limit,
                "unit": unit.as_str(),
                "message": "required field is missing"
            }));
        }
        return Ok(());
    };
    let used = unit.count(value);
    let status = if value.trim().is_empty() && required || used > limit {
        "error"
    } else {
        "pass"
    };
    let message = if value.trim().is_empty() && required {
        "required field is empty".to_owned()
    } else if used > limit {
        format!("exceeds limit by {} {}", used - limit, unit.as_str())
    } else {
        format!("{} {} remaining", limit - used, unit.as_str())
    };
    checks.push(json!({
        "field": field,
        "status": status,
        "used": used,
        "limit": limit,
        "unit": unit.as_str(),
        "message": message
    }));
    Ok(())
}

fn validate_apple_keywords(
    object: &Map<String, Value>,
    checks: &mut Vec<Value>,
    warnings: &mut Vec<String>,
) -> Result<(), String> {
    let Some(keywords) = text_field(object, "keywords")? else {
        return Ok(());
    };
    if keywords.trim().is_empty() {
        return Ok(());
    }
    let entries: Vec<&str> = keywords.split(',').collect();
    let mut seen = BTreeSet::new();
    let mut short = Vec::new();
    let mut duplicates = Vec::new();
    let mut padded = Vec::new();
    for entry in entries {
        let trimmed = entry.trim();
        if trimmed.chars().count() < 3 {
            short.push(trimmed.to_owned());
        }
        if trimmed != entry {
            padded.push(trimmed.to_owned());
        }
        let normalized = trimmed.to_lowercase();
        if !normalized.is_empty() && !seen.insert(normalized) {
            duplicates.push(trimmed.to_owned());
        }
    }
    checks.push(json!({
        "field": "keywords.entries",
        "status": if short.is_empty() { "pass" } else { "error" },
        "message": if short.is_empty() {
            "all keywords exceed two characters".to_owned()
        } else {
            format!("keywords must exceed two characters: {}", short.join(", "))
        }
    }));
    if !duplicates.is_empty() {
        warnings.push(format!(
            "duplicate keywords waste budget: {}",
            duplicates.join(", ")
        ));
    }
    if !padded.is_empty() {
        warnings.push(format!(
            "spaces around commas waste bytes: {}",
            padded.join(", ")
        ));
    }

    let searchable_names = [
        text_field(object, "name")?,
        text_field(object, "company_name")?,
    ];
    for name in searchable_names.into_iter().flatten() {
        let normalized = name.trim().to_lowercase();
        if seen.contains(&normalized) {
            warnings.push(format!("keyword list duplicates searchable name {name:?}"));
        }
    }
    Ok(())
}

pub fn analyze_experiment(
    control_conversions: u64,
    control_visitors: u64,
    variant_conversions: u64,
    variant_visitors: u64,
    alpha: f64,
) -> Result<Value, String> {
    if control_visitors == 0 || variant_visitors == 0 {
        return Err("visitor counts must be greater than zero".to_owned());
    }
    if control_conversions > control_visitors || variant_conversions > variant_visitors {
        return Err("conversions cannot exceed visitors".to_owned());
    }
    if !alpha.is_finite() || !(0.0..1.0).contains(&alpha) {
        return Err("alpha must be finite and between zero and one".to_owned());
    }

    let control_rate = control_conversions as f64 / control_visitors as f64;
    let variant_rate = variant_conversions as f64 / variant_visitors as f64;
    let difference = variant_rate - control_rate;
    let pooled = (control_conversions + variant_conversions) as f64
        / (control_visitors + variant_visitors) as f64;
    let standard_error =
        (pooled * (1.0 - pooled) * (1.0 / control_visitors as f64 + 1.0 / variant_visitors as f64))
            .sqrt();
    let z_score = if standard_error == 0.0 {
        0.0
    } else {
        difference / standard_error
    };
    let p_value = (2.0 * (1.0 - normal_cdf(z_score.abs()))).clamp(0.0, 1.0);
    let relative_lift = if control_rate == 0.0 {
        Value::Null
    } else {
        json!((difference / control_rate) * 100.0)
    };

    Ok(json!({
        "schema_version": 1,
        "method": "two-proportion-z-test",
        "control": {
            "conversions": control_conversions,
            "visitors": control_visitors,
            "rate": control_rate
        },
        "variant": {
            "conversions": variant_conversions,
            "visitors": variant_visitors,
            "rate": variant_rate
        },
        "absolute_difference": difference,
        "relative_lift_percent": relative_lift,
        "z_score": z_score,
        "p_value_two_sided": p_value,
        "alpha": alpha,
        "significant": p_value < alpha,
        "warning": "Snapshot test only. Check allocation, minimum run condition, sequential peeking, multiple comparisons, seasonality, and store-specific experiment methodology."
    }))
}

fn normal_cdf(value: f64) -> f64 {
    let x = value.abs();
    let t = 1.0 / (1.0 + 0.231_641_9 * x);
    let density = 0.398_942_280_401_432_7 * (-0.5 * x * x).exp();
    let tail = density
        * t
        * (0.319_381_530
            + t * (-0.356_563_782
                + t * (1.781_477_937 + t * (-1.821_255_978 + t * 1.330_274_429))));
    if value >= 0.0 {
        1.0 - tail
    } else {
        tail
    }
}

fn text_field<'a>(object: &'a Map<String, Value>, field: &str) -> Result<Option<&'a str>, String> {
    match object.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value)),
        Some(_) => Err(format!("field {field:?} must be a string")),
    }
}

fn allowed_fields(platform: &str) -> BTreeSet<&'static str> {
    let common = ["platform", "locale", "name", "company_name"];
    let specific: &[&str] = match platform {
        "apple" => &[
            "subtitle",
            "promotional_text",
            "description",
            "keywords",
            "whats_new",
        ],
        "google" => &["short_description", "full_description"],
        _ => &[],
    };
    common.into_iter().chain(specific.iter().copied()).collect()
}

fn read_input(path: &str) -> Result<String, String> {
    if path == "-" {
        let mut text = String::new();
        io::stdin()
            .read_to_string(&mut text)
            .map_err(|error| format!("cannot read stdin: {error}"))?;
        Ok(text)
    } else {
        fs::read_to_string(Path::new(path)).map_err(|error| format!("cannot read {path}: {error}"))
    }
}

fn parse_options(args: &[String]) -> Result<BTreeMap<String, String>, String> {
    if !args.len().is_multiple_of(2) {
        return Err("every option requires one value".to_owned());
    }
    let mut values = BTreeMap::new();
    for pair in args.chunks_exact(2) {
        if !pair[0].starts_with("--") {
            return Err(format!("unexpected positional argument {:?}", pair[0]));
        }
        if values.insert(pair[0].clone(), pair[1].clone()).is_some() {
            return Err(format!("duplicate option {}", pair[0]));
        }
    }
    Ok(values)
}

fn reject_unknown(values: &BTreeMap<String, String>, allowed: &[&str]) -> Result<(), String> {
    for key in values.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("unknown option {key}"));
        }
    }
    Ok(())
}

fn required<'a>(values: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str, String> {
    values
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("missing required option {key}"))
}

fn parse_u64(value: &str, label: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|error| format!("invalid {label}: {error}"))
}

fn parse_f64(value: &str, label: &str) -> Result<f64, String> {
    value
        .parse::<f64>()
        .map_err(|error| format!("invalid {label}: {error}"))
}

fn pretty(value: &Value) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|error| format!("cannot encode report: {error}"))
}

fn success(output: &str) -> CliOutcome {
    CliOutcome {
        output: output.to_owned(),
        exit_code: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::{analyze_experiment, run_cli, validate_listing};
    use serde_json::{json, Value};

    fn valid_apple() -> Value {
        json!({
            "platform": "apple",
            "locale": "en-US",
            "name": "TaskFlow",
            "description": "Plan work with focus.",
            "keywords": "tasks,planner,focus"
        })
    }

    fn valid_google() -> Value {
        json!({
            "platform": "google",
            "locale": "en-US",
            "name": "TaskFlow",
            "short_description": "Plan focused work with your team.",
            "full_description": "Plan focused work with your team."
        })
    }

    fn check<'a>(report: &'a Value, field: &str) -> &'a Value {
        report["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["field"] == field)
            .unwrap()
    }

    #[test]
    fn apple_character_boundaries_pass() {
        let mut listing = valid_apple();
        listing["name"] = json!("a".repeat(30));
        listing["subtitle"] = json!("b".repeat(30));
        listing["promotional_text"] = json!("c".repeat(170));
        listing["description"] = json!("d".repeat(4_000));
        listing["keywords"] = json!("k".repeat(100));
        listing["whats_new"] = json!("w".repeat(4_000));
        let report = validate_listing(&listing).unwrap();
        assert_eq!(report["valid"], true);
    }

    #[test]
    fn apple_keywords_use_utf8_bytes() {
        let mut listing = valid_apple();
        listing["keywords"] = json!("é".repeat(51));
        let report = validate_listing(&listing).unwrap();
        assert_eq!(report["valid"], false);
        let keywords = check(&report, "keywords");
        assert_eq!(keywords["used"], 102);
        assert_eq!(keywords["unit"], "bytes");
    }

    #[test]
    fn google_name_limit_is_thirty_not_fifty() {
        let mut listing = valid_google();
        listing["name"] = json!("x".repeat(31));
        let report = validate_listing(&listing).unwrap();
        assert_eq!(report["valid"], false);
        assert_eq!(check(&report, "name")["limit"], 30);
    }

    #[test]
    fn required_fields_fail_when_missing() {
        let report = validate_listing(&json!({"platform": "google"})).unwrap();
        assert_eq!(report["valid"], false);
        assert_eq!(check(&report, "short_description")["status"], "error");
    }

    #[test]
    fn experiment_detects_clear_lift() {
        let report = analyze_experiment(100, 2_000, 160, 2_000, 0.05).unwrap();
        assert_eq!(report["significant"], true);
        assert!(report["p_value_two_sided"].as_f64().unwrap() < 0.05);
    }

    #[test]
    fn experiment_rejects_invalid_counts() {
        assert!(analyze_experiment(11, 10, 1, 10, 0.05).is_err());
        assert!(analyze_experiment(1, 10, 1, 0, 0.05).is_err());
        assert!(analyze_experiment(1, 10, 1, 10, 1.0).is_err());
    }

    #[test]
    fn cli_help_is_successful() {
        let outcome = run_cli(["--help".to_owned()]).unwrap();
        assert_eq!(outcome.exit_code, 0);
        assert!(outcome.output.contains("aso-lint validate"));
    }
}

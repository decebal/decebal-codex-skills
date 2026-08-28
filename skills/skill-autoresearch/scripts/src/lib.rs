use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const CONFIG_FILE: &str = "config.tsv";
const RESULTS_FILE: &str = "results.tsv";
const STDOUT_FILE: &str = "last-evaluator.stdout";
const STDERR_FILE: &str = "last-evaluator.stderr";
const RESULTS_HEADER: &str = "iteration\tscore\tdelta\tverdict\tcommit\tdescription\n";

const HELP: &str = r#"skill-autoresearch

USAGE:
  skill-autoresearch init STATE --skill-path PATH --metric NAME \
    --direction higher|lower --max-iterations N --timeout-seconds N
  skill-autoresearch baseline STATE --commit REV --description TEXT \
    (--score NUMBER | -- PROGRAM [ARG...])
  skill-autoresearch candidate STATE --commit REV --description TEXT \
    (--score NUMBER | -- PROGRAM [ARG...])
  skill-autoresearch status STATE

Evaluator final non-empty stdout line must be NUMBER or score=NUMBER.
The runner executes PROGRAM directly, never through a shell.
"#;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Direction {
    Higher,
    Lower,
}

impl Direction {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "higher" => Ok(Self::Higher),
            "lower" => Ok(Self::Lower),
            _ => Err(format!(
                "direction must be higher or lower, received {value:?}"
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Higher => "higher",
            Self::Lower => "lower",
        }
    }

    fn improved(self, candidate: f64, incumbent: f64) -> bool {
        match self {
            Self::Higher => candidate > incumbent,
            Self::Lower => candidate < incumbent,
        }
    }
}

#[derive(Debug)]
struct Config {
    skill_path: PathBuf,
    metric: String,
    direction: Direction,
    max_iterations: u32,
    timeout_seconds: u64,
}

impl Config {
    fn load(state: &Path) -> Result<Self, String> {
        let path = state.join(CONFIG_FILE);
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let mut values = BTreeMap::new();
        for (number, line) in text.lines().enumerate() {
            let (key, value) = line.split_once('\t').ok_or_else(|| {
                format!(
                    "{}:{}: expected tab-separated key and value",
                    path.display(),
                    number + 1
                )
            })?;
            if values.insert(key.to_owned(), value.to_owned()).is_some() {
                return Err(format!("{}: duplicate config key {key:?}", path.display()));
            }
        }
        if values.get("version").map(String::as_str) != Some("1") {
            return Err(format!("{}: unsupported config version", path.display()));
        }
        let skill_path = PathBuf::from(required_map(&values, "skill_path")?);
        let metric = required_map(&values, "metric")?.to_owned();
        let direction = Direction::parse(required_map(&values, "direction")?)?;
        let max_iterations = parse_u32(
            required_map(&values, "max_iterations")?,
            "max_iterations",
            1,
            10_000,
        )?;
        let timeout_seconds = parse_u64(
            required_map(&values, "timeout_seconds")?,
            "timeout_seconds",
            1,
            86_400,
        )?;
        Ok(Self {
            skill_path,
            metric,
            direction,
            max_iterations,
            timeout_seconds,
        })
    }

    fn write(&self, state: &Path) -> Result<(), String> {
        let path = state.join(CONFIG_FILE);
        let skill_path = self.skill_path.to_string_lossy();
        validate_field(&skill_path, "skill_path")?;
        validate_field(&self.metric, "metric")?;
        let text = format!(
            "version\t1\nskill_path\t{}\nmetric\t{}\ndirection\t{}\nmax_iterations\t{}\ntimeout_seconds\t{}\n",
            skill_path,
            self.metric,
            self.direction.as_str(),
            self.max_iterations,
            self.timeout_seconds
        );
        fs::write(&path, text).map_err(|error| format!("cannot write {}: {error}", path.display()))
    }
}

#[derive(Debug, Default)]
struct History {
    baseline: Option<f64>,
    incumbent: Option<f64>,
    candidates: u32,
    kept: u32,
    discarded: u32,
    failed: u32,
    last_verdict: Option<String>,
}

impl History {
    fn load(state: &Path) -> Result<Self, String> {
        let path = state.join(RESULTS_FILE);
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let mut lines = text.lines();
        if lines.next() != Some(RESULTS_HEADER.trim_end()) {
            return Err(format!("{}: invalid results header", path.display()));
        }

        let mut history = Self::default();
        for (offset, line) in lines.enumerate() {
            let fields: Vec<&str> = line.split('\t').collect();
            if fields.len() != 6 {
                return Err(format!(
                    "{}:{}: expected six result fields",
                    path.display(),
                    offset + 2
                ));
            }
            let iteration = fields[0].parse::<u32>().map_err(|error| {
                format!(
                    "{}:{}: invalid iteration: {error}",
                    path.display(),
                    offset + 2
                )
            })?;
            let score = if fields[1] == "NA" {
                None
            } else {
                Some(parse_score(fields[1])?)
            };
            let verdict = fields[3];
            match verdict {
                "baseline" => {
                    if iteration != 0 || history.baseline.is_some() {
                        return Err(format!(
                            "{}:{}: invalid baseline",
                            path.display(),
                            offset + 2
                        ));
                    }
                    let score = score.ok_or_else(|| {
                        format!("{}:{}: baseline score missing", path.display(), offset + 2)
                    })?;
                    history.baseline = Some(score);
                    history.incumbent = Some(score);
                }
                "keep" => {
                    history.incumbent = score;
                    history.kept += 1;
                }
                "discard" => history.discarded += 1,
                "error" | "timeout" => history.failed += 1,
                _ => {
                    return Err(format!(
                        "{}:{}: unknown verdict {verdict:?}",
                        path.display(),
                        offset + 2
                    ));
                }
            }
            if iteration > 0 {
                history.candidates = history.candidates.max(iteration);
            }
            history.last_verdict = Some(verdict.to_owned());
        }
        Ok(history)
    }
}

#[derive(Debug)]
struct ParsedOptions {
    values: BTreeMap<String, String>,
    command: Vec<String>,
}

#[derive(Debug)]
enum Evaluation {
    Score(f64),
    Error(String),
    Timeout(String),
}

pub fn run_cli<I>(args: I) -> Result<String, String>
where
    I: IntoIterator<Item = String>,
{
    let args: Vec<String> = args.into_iter().collect();
    let Some(command) = args.first().map(String::as_str) else {
        return Ok(HELP.to_owned());
    };
    match command {
        "help" | "--help" | "-h" => Ok(HELP.to_owned()),
        "init" => init_command(&args[1..]),
        "baseline" => measure_command(&args[1..], true),
        "candidate" => measure_command(&args[1..], false),
        "status" => status_command(&args[1..]),
        _ => Err(format!("unknown command {command:?}\n\n{HELP}")),
    }
}

fn init_command(args: &[String]) -> Result<String, String> {
    let state = state_arg(args)?;
    let parsed = parse_options(&args[1..])?;
    if !parsed.command.is_empty() {
        return Err("init does not accept an evaluator command".to_owned());
    }
    reject_unknown(
        &parsed.values,
        &[
            "--skill-path",
            "--metric",
            "--direction",
            "--max-iterations",
            "--timeout-seconds",
        ],
    )?;

    let skill_path = PathBuf::from(required_option(&parsed.values, "--skill-path")?);
    if !skill_path.join("SKILL.md").is_file() {
        return Err(format!("{} must contain SKILL.md", skill_path.display()));
    }
    let skill_path = fs::canonicalize(&skill_path)
        .map_err(|error| format!("cannot resolve {}: {error}", skill_path.display()))?;
    let metric = required_option(&parsed.values, "--metric")?.to_owned();
    validate_field(&metric, "metric")?;
    let direction = Direction::parse(required_option(&parsed.values, "--direction")?)?;
    let max_iterations = parse_u32(
        required_option(&parsed.values, "--max-iterations")?,
        "max_iterations",
        1,
        10_000,
    )?;
    let timeout_seconds = parse_u64(
        required_option(&parsed.values, "--timeout-seconds")?,
        "timeout_seconds",
        1,
        86_400,
    )?;

    if state.exists()
        && fs::read_dir(&state)
            .map_err(io_error(&state))?
            .next()
            .is_some()
    {
        return Err(format!("{} must be absent or empty", state.display()));
    }
    fs::create_dir_all(&state).map_err(io_error(&state))?;
    let config = Config {
        skill_path,
        metric,
        direction,
        max_iterations,
        timeout_seconds,
    };
    config.write(&state)?;
    fs::write(state.join(RESULTS_FILE), RESULTS_HEADER).map_err(io_error(&state))?;

    Ok(format!(
        "state={}\nmetric={}\ndirection={}\niterations=0/{}\nnext=baseline",
        state.display(),
        config.metric,
        config.direction.as_str(),
        config.max_iterations
    ))
}

fn measure_command(args: &[String], baseline: bool) -> Result<String, String> {
    let state = state_arg(args)?;
    let config = Config::load(&state)?;
    let history = History::load(&state)?;
    if baseline {
        if history.baseline.is_some() || history.candidates > 0 {
            return Err("baseline already recorded or candidate history exists".to_owned());
        }
    } else {
        if history.baseline.is_none() {
            return Err("record baseline before candidates".to_owned());
        }
        if history.candidates >= config.max_iterations {
            return Err(format!(
                "candidate limit reached: {}/{}",
                history.candidates, config.max_iterations
            ));
        }
    }

    let parsed = parse_options(&args[1..])?;
    reject_unknown(&parsed.values, &["--commit", "--description", "--score"])?;
    let commit = required_option(&parsed.values, "--commit")?;
    let description = required_option(&parsed.values, "--description")?;
    validate_field(commit, "commit")?;
    validate_field(description, "description")?;
    let score_option = parsed.values.get("--score");
    if score_option.is_some() == !parsed.command.is_empty() {
        return Err("provide exactly one of --score NUMBER or -- PROGRAM [ARG...]".to_owned());
    }

    let iteration = if baseline { 0 } else { history.candidates + 1 };
    let evaluation = match score_option {
        Some(value) => Evaluation::Score(parse_score(value)?),
        None => run_evaluator(
            &state,
            &parsed.command,
            Duration::from_secs(config.timeout_seconds),
        ),
    };

    match evaluation {
        Evaluation::Score(score) if baseline => {
            append_result(
                &state,
                iteration,
                Some(score),
                Some(0.0),
                "baseline",
                commit,
                description,
            )?;
            Ok(format!(
                "iteration=0\nscore={}\nverdict=baseline\nincumbent={}",
                format_score(score),
                format_score(score)
            ))
        }
        Evaluation::Score(score) => {
            let incumbent = history
                .incumbent
                .ok_or_else(|| "incumbent missing".to_owned())?;
            let delta = score - incumbent;
            let verdict = if config.direction.improved(score, incumbent) {
                "keep"
            } else {
                "discard"
            };
            append_result(
                &state,
                iteration,
                Some(score),
                Some(delta),
                verdict,
                commit,
                description,
            )?;
            let next_incumbent = if verdict == "keep" { score } else { incumbent };
            Ok(format!(
                "iteration={iteration}\nscore={}\ndelta={}\nverdict={verdict}\nincumbent={}",
                format_score(score),
                format_score(delta),
                format_score(next_incumbent)
            ))
        }
        Evaluation::Error(detail) => {
            append_result(&state, iteration, None, None, "error", commit, description)?;
            Err(format!("iteration {iteration} recorded as error: {detail}"))
        }
        Evaluation::Timeout(detail) => {
            append_result(
                &state,
                iteration,
                None,
                None,
                "timeout",
                commit,
                description,
            )?;
            Err(format!(
                "iteration {iteration} recorded as timeout: {detail}"
            ))
        }
    }
}

fn status_command(args: &[String]) -> Result<String, String> {
    if args.len() != 1 {
        return Err("status requires exactly one STATE argument".to_owned());
    }
    let state = PathBuf::from(&args[0]);
    let config = Config::load(&state)?;
    let history = History::load(&state)?;
    Ok(format!(
        "state={}\nskill_path={}\nmetric={}\ndirection={}\niterations={}/{}\nbaseline={}\nincumbent={}\nkept={}\ndiscarded={}\nfailed={}\nlast_verdict={}",
        state.display(),
        config.skill_path.display(),
        config.metric,
        config.direction.as_str(),
        history.candidates,
        config.max_iterations,
        optional_score(history.baseline),
        optional_score(history.incumbent),
        history.kept,
        history.discarded,
        history.failed,
        history.last_verdict.as_deref().unwrap_or("none")
    ))
}

fn state_arg(args: &[String]) -> Result<PathBuf, String> {
    let value = args
        .first()
        .ok_or_else(|| "missing STATE argument".to_owned())?;
    if value.starts_with("--") {
        return Err("STATE must be first positional argument".to_owned());
    }
    Ok(PathBuf::from(value))
}

fn parse_options(args: &[String]) -> Result<ParsedOptions, String> {
    let mut values = BTreeMap::new();
    let mut index = 0;
    while index < args.len() {
        if args[index] == "--" {
            return Ok(ParsedOptions {
                values,
                command: args[index + 1..].to_vec(),
            });
        }
        let key = &args[index];
        if !key.starts_with("--") {
            return Err(format!("unexpected positional argument {key:?}"));
        }
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {key}"))?;
        if value == "--" {
            return Err(format!("missing value for {key}"));
        }
        if values.insert(key.clone(), value.clone()).is_some() {
            return Err(format!("duplicate option {key}"));
        }
        index += 2;
    }
    Ok(ParsedOptions {
        values,
        command: Vec::new(),
    })
}

fn reject_unknown(values: &BTreeMap<String, String>, allowed: &[&str]) -> Result<(), String> {
    for key in values.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!("unknown option {key}"));
        }
    }
    Ok(())
}

fn required_option<'a>(values: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str, String> {
    values
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("missing required option {key}"))
}

fn required_map<'a>(values: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str, String> {
    values
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| format!("missing config key {key:?}"))
}

fn parse_u32(value: &str, label: &str, min: u32, max: u32) -> Result<u32, String> {
    let value = value
        .parse::<u32>()
        .map_err(|error| format!("invalid {label}: {error}"))?;
    if !(min..=max).contains(&value) {
        return Err(format!("{label} must be in {min}..={max}"));
    }
    Ok(value)
}

fn parse_u64(value: &str, label: &str, min: u64, max: u64) -> Result<u64, String> {
    let value = value
        .parse::<u64>()
        .map_err(|error| format!("invalid {label}: {error}"))?;
    if !(min..=max).contains(&value) {
        return Err(format!("{label} must be in {min}..={max}"));
    }
    Ok(value)
}

fn parse_score(value: &str) -> Result<f64, String> {
    let value = value.trim();
    let value = value.strip_prefix("score=").unwrap_or(value).trim();
    let score = value
        .parse::<f64>()
        .map_err(|error| format!("invalid score {value:?}: {error}"))?;
    if !score.is_finite() {
        return Err("score must be finite".to_owned());
    }
    Ok(score)
}

fn parse_evaluator_score(stdout: &str) -> Result<f64, String> {
    let line = stdout
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .ok_or_else(|| "evaluator stdout has no score".to_owned())?;
    parse_score(line)
}

fn run_evaluator(state: &Path, command: &[String], timeout: Duration) -> Evaluation {
    if command.is_empty() {
        return Evaluation::Error("evaluator command is empty".to_owned());
    }
    let stdout_path = state.join(STDOUT_FILE);
    let stderr_path = state.join(STDERR_FILE);
    let stdout = match File::create(&stdout_path) {
        Ok(file) => file,
        Err(error) => {
            return Evaluation::Error(format!("cannot create {}: {error}", stdout_path.display()));
        }
    };
    let stderr = match File::create(&stderr_path) {
        Ok(file) => file,
        Err(error) => {
            return Evaluation::Error(format!("cannot create {}: {error}", stderr_path.display()));
        }
    };
    let mut child = match Command::new(&command[0])
        .args(&command[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
    {
        Ok(child) => child,
        Err(error) => return Evaluation::Error(format!("cannot start evaluator: {error}")),
    };

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() >= timeout => {
                let kill_error = child.kill().err();
                let _ = child.wait();
                let stderr = fs::read_to_string(&stderr_path).unwrap_or_default();
                let mut detail = format!("evaluator exceeded {} second(s)", timeout.as_secs_f64());
                if let Some(error) = kill_error {
                    detail.push_str(&format!("; kill failed: {error}"));
                }
                if !stderr.trim().is_empty() {
                    detail.push_str(&format!("; stderr: {}", tail(&stderr, 240)));
                }
                return Evaluation::Timeout(detail);
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(error) => return Evaluation::Error(format!("cannot poll evaluator: {error}")),
        }
    };

    let stdout = fs::read_to_string(&stdout_path).unwrap_or_default();
    let stderr = fs::read_to_string(&stderr_path).unwrap_or_default();
    if !status.success() {
        let detail = if stderr.trim().is_empty() {
            format!("evaluator exited with {status}")
        } else {
            format!(
                "evaluator exited with {status}; stderr: {}",
                tail(&stderr, 240)
            )
        };
        return Evaluation::Error(detail);
    }
    match parse_evaluator_score(&stdout) {
        Ok(score) => Evaluation::Score(score),
        Err(error) => Evaluation::Error(error),
    }
}

fn append_result(
    state: &Path,
    iteration: u32,
    score: Option<f64>,
    delta: Option<f64>,
    verdict: &str,
    commit: &str,
    description: &str,
) -> Result<(), String> {
    let path = state.join(RESULTS_FILE);
    let mut file = OpenOptions::new()
        .append(true)
        .open(&path)
        .map_err(|error| format!("cannot open {}: {error}", path.display()))?;
    let score = score.map(format_score).unwrap_or_else(|| "NA".to_owned());
    let delta = delta.map(format_score).unwrap_or_else(|| "NA".to_owned());
    writeln!(
        file,
        "{iteration}\t{score}\t{delta}\t{verdict}\t{commit}\t{description}"
    )
    .map_err(|error| format!("cannot append {}: {error}", path.display()))
}

fn validate_field(value: &str, label: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("{label} cannot be empty"));
    }
    if value.contains(['\t', '\n', '\r']) {
        return Err(format!("{label} cannot contain tabs or newlines"));
    }
    if value.chars().count() > 512 {
        return Err(format!("{label} cannot exceed 512 characters"));
    }
    Ok(())
}

fn format_score(value: f64) -> String {
    value.to_string()
}

fn optional_score(value: Option<f64>) -> String {
    value.map(format_score).unwrap_or_else(|| "none".to_owned())
}

fn tail(value: &str, limit: usize) -> String {
    let chars: Vec<char> = value.trim().chars().collect();
    let start = chars.len().saturating_sub(limit);
    chars[start..].iter().collect()
}

fn io_error(path: &Path) -> impl FnOnce(std::io::Error) -> String + '_ {
    move |error| format!("{}: {error}", path.display())
}

#[cfg(test)]
mod tests {
    use super::{parse_evaluator_score, run_cli, run_evaluator, Evaluation};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    struct Fixture {
        root: PathBuf,
        skill: PathBuf,
        state: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let suffix = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "skill-autoresearch-test-{}-{suffix}",
                std::process::id()
            ));
            let skill = root.join("skill");
            let state = root.join("state");
            fs::create_dir_all(&skill).unwrap();
            fs::write(skill.join("SKILL.md"), "---\nname: sample\n---\n").unwrap();
            Self { root, skill, state }
        }

        fn init(&self, direction: &str, max_iterations: &str) -> String {
            cli(&[
                "init",
                path(&self.state),
                "--skill-path",
                path(&self.skill),
                "--metric",
                "test-score",
                "--direction",
                direction,
                "--max-iterations",
                max_iterations,
                "--timeout-seconds",
                "2",
            ])
            .unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn path(value: &Path) -> &str {
        value.to_str().unwrap()
    }

    fn cli(args: &[&str]) -> Result<String, String> {
        run_cli(args.iter().map(|value| (*value).to_owned()))
    }

    #[test]
    fn higher_score_keeps_strict_improvement_and_discards_tie() {
        let fixture = Fixture::new();
        fixture.init("higher", "3");
        cli(&[
            "baseline",
            path(&fixture.state),
            "--commit",
            "base",
            "--description",
            "baseline",
            "--score",
            "10",
        ])
        .unwrap();
        let keep = cli(&[
            "candidate",
            path(&fixture.state),
            "--commit",
            "one",
            "--description",
            "improves",
            "--score",
            "11",
        ])
        .unwrap();
        assert!(keep.contains("verdict=keep"));
        let discard = cli(&[
            "candidate",
            path(&fixture.state),
            "--commit",
            "two",
            "--description",
            "ties",
            "--score",
            "11",
        ])
        .unwrap();
        assert!(discard.contains("verdict=discard"));
        let status = cli(&["status", path(&fixture.state)]).unwrap();
        assert!(status.contains("incumbent=11"));
        assert!(status.contains("kept=1"));
        assert!(status.contains("discarded=1"));
    }

    #[test]
    fn lower_score_and_iteration_cap_work() {
        let fixture = Fixture::new();
        fixture.init("lower", "1");
        cli(&[
            "baseline",
            path(&fixture.state),
            "--commit",
            "base",
            "--description",
            "baseline",
            "--score",
            "5",
        ])
        .unwrap();
        let keep = cli(&[
            "candidate",
            path(&fixture.state),
            "--commit",
            "one",
            "--description",
            "faster",
            "--score",
            "4.5",
        ])
        .unwrap();
        assert!(keep.contains("verdict=keep"));
        let error = cli(&[
            "candidate",
            path(&fixture.state),
            "--commit",
            "two",
            "--description",
            "too-many",
            "--score",
            "4",
        ])
        .unwrap_err();
        assert!(error.contains("candidate limit reached"));
    }

    #[test]
    fn parses_only_final_nonempty_score_line() {
        assert_eq!(
            parse_evaluator_score("detail\nscore=12.25\n\n").unwrap(),
            12.25
        );
        assert!(parse_evaluator_score("score=NaN\n").is_err());
        assert!(parse_evaluator_score("detail only\n").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn evaluator_runs_directly_and_captures_score() {
        let fixture = Fixture::new();
        fs::create_dir_all(&fixture.state).unwrap();
        let command = vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            "printf 'detail\\nscore=7.5\\n'".to_owned(),
        ];
        match run_evaluator(&fixture.state, &command, Duration::from_secs(2)) {
            Evaluation::Score(score) => assert_eq!(score, 7.5),
            other => panic!("unexpected evaluation: {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn evaluator_timeout_is_enforced() {
        let fixture = Fixture::new();
        fs::create_dir_all(&fixture.state).unwrap();
        let command = vec!["/bin/sh".to_owned(), "-c".to_owned(), "sleep 1".to_owned()];
        match run_evaluator(&fixture.state, &command, Duration::from_millis(30)) {
            Evaluation::Timeout(detail) => assert!(detail.contains("exceeded")),
            other => panic!("unexpected evaluation: {other:?}"),
        }
    }
}

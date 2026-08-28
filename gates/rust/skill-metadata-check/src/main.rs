use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = flag(&args, "--skills-dir")
        .map(PathBuf::from)
        .unwrap_or_else(default_skills_dir);
    let plugin = flag(&args, "--plugin-manifest").map(PathBuf::from);

    let (count, mut errors) = match validate_tree(&root) {
        Ok(count) => (count, Vec::new()),
        Err(errors) => (0, errors),
    };
    if let Some(path) = plugin.as_deref() {
        errors.extend(validate_plugin(path));
    }
    if errors.is_empty() {
        println!("skill-metadata-check: {count} skill(s) and plugin metadata valid");
        ExitCode::SUCCESS
    } else {
        for error in errors {
            eprintln!("skill-metadata-check: {error}");
        }
        ExitCode::from(1)
    }
}

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|arg| arg == name)
        .and_then(|index| args.get(index + 1))
        .cloned()
}

fn default_skills_dir() -> PathBuf {
    let local = PathBuf::from("skills");
    if local.is_dir() {
        local
    } else {
        PathBuf::from("../../skills")
    }
}

fn validate_tree(root: &Path) -> Result<usize, Vec<String>> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) => return Err(vec![format!("{}: {error}", root.display())]),
    };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir() && path.join("SKILL.md").is_file())
        .collect();
    dirs.sort();

    let mut errors = Vec::new();
    for dir in &dirs {
        errors.extend(validate_skill(dir));
    }
    if errors.is_empty() {
        Ok(dirs.len())
    } else {
        Err(errors)
    }
}

fn validate_skill(dir: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    let label = dir.display();
    let markdown = match std::fs::read_to_string(dir.join("SKILL.md")) {
        Ok(text) => text,
        Err(error) => return vec![format!("{label}/SKILL.md: {error}")],
    };
    let frontmatter = match frontmatter(&markdown) {
        Some(value) => value,
        None => return vec![format!("{label}/SKILL.md: invalid YAML frontmatter fence")],
    };
    let name = field(frontmatter, "name").unwrap_or_default();
    let description = field(frontmatter, "description").unwrap_or_default();
    let expected = dir
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");

    if name != expected {
        errors.push(format!(
            "{label}/SKILL.md: name {name:?} must equal {expected:?}"
        ));
    }
    if !valid_name(name) {
        errors.push(format!("{label}/SKILL.md: invalid skill name {name:?}"));
    }
    if description.is_empty() || description.len() > 1024 {
        errors.push(format!(
            "{label}/SKILL.md: description length must be 1..=1024"
        ));
    }
    if description.contains('<') || description.contains('>') {
        errors.push(format!(
            "{label}/SKILL.md: description cannot contain angle brackets"
        ));
    }

    let metadata_path = dir.join("agents/openai.yaml");
    let metadata = match std::fs::read_to_string(&metadata_path) {
        Ok(text) => text,
        Err(error) => {
            errors.push(format!("{}: {error}", metadata_path.display()));
            return errors;
        }
    };
    let display_name = field(&metadata, "display_name").unwrap_or_default();
    let short = field(&metadata, "short_description").unwrap_or_default();
    let prompt = field(&metadata, "default_prompt").unwrap_or_default();
    if display_name.is_empty() {
        errors.push(format!("{}: display_name missing", metadata_path.display()));
    }
    if !(25..=64).contains(&short.len()) {
        errors.push(format!(
            "{}: short_description must be 25..=64 characters",
            metadata_path.display()
        ));
    }
    if !prompt.contains(&format!("${name}")) {
        errors.push(format!(
            "{}: default_prompt must mention ${name}",
            metadata_path.display()
        ));
    }
    errors
}

fn frontmatter(markdown: &str) -> Option<&str> {
    let rest = markdown.strip_prefix("---\n")?;
    rest.split_once("\n---\n").map(|(head, _)| head)
}

fn field<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().find_map(|line| {
        let line = line.trim();
        let value = line.strip_prefix(key)?.strip_prefix(':')?.trim();
        let quoted = value
            .strip_prefix('"')
            .and_then(|inner| inner.strip_suffix('"'));
        Some(quoted.unwrap_or(value))
    })
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--")
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn validate_plugin(path: &Path) -> Vec<String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => return vec![format!("{}: {error}", path.display())],
    };
    let value: serde_json::Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => return vec![format!("{}: {error}", path.display())],
    };
    let mut errors = Vec::new();
    for key in ["name", "version", "description", "license", "skills"] {
        if value.get(key).and_then(|entry| entry.as_str()).is_none() {
            errors.push(format!("{}: string field {key:?} missing", path.display()));
        }
    }
    if value.get("skills").and_then(|entry| entry.as_str()) != Some("./skills/") {
        errors.push(format!(
            "{}: skills must point to ./skills/",
            path.display()
        ));
    }
    let interface = value.get("interface").and_then(|entry| entry.as_object());
    for key in ["displayName", "shortDescription", "defaultPrompt"] {
        if interface.and_then(|entry| entry.get(key)).is_none() {
            errors.push(format!(
                "{}: interface field {key:?} missing",
                path.display()
            ));
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::{field, frontmatter, valid_name};

    #[test]
    fn reads_frontmatter_and_quoted_fields() {
        let markdown = "---\nname: sample\ndescription: \"Useful skill\"\n---\n# Body\n";
        let head = frontmatter(markdown).unwrap();
        assert_eq!(field(head, "name"), Some("sample"));
        assert_eq!(field(head, "description"), Some("Useful skill"));
    }

    #[test]
    fn validates_codex_skill_names() {
        assert!(valid_name("codex-beads"));
        assert!(!valid_name("Codex-Beads"));
        assert!(!valid_name("codex--beads"));
    }
}

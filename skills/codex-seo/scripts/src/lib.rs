//! Deterministic, bounded SEO evidence collection for the `codex-seo` skill.

use reqwest::blocking::{Client, Response};
use reqwest::header::{CONTENT_TYPE, LOCATION};
use reqwest::redirect::Policy;
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::io::Read;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};
use std::path::Path;
use std::time::Duration;
use url::{Host, Url};

const MAX_RESPONSE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_REDIRECTS: usize = 5;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);
const USER_AGENT: &str = "decebal-codex-seo/0.1 (+https://github.com/decebal/decebal-codex-skills)";

const HELP: &str = r"codex-seo

USAGE:
  codex-seo audit --input PATH_OR_URL [--base-url URL] [--format json|markdown]
  codex-seo sitemap --input PATH_OR_URL
  codex-seo drift --before AUDIT.json --after AUDIT.json
  codex-seo doctor

Network inputs accept public HTTP(S) only. Local and private addresses are rejected.
";

#[derive(Debug, Eq, PartialEq)]
pub struct CliOutcome {
    pub output: String,
    pub exit_code: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Finding {
    pub id: String,
    pub severity: Severity,
    pub title: String,
    pub evidence: String,
    pub recommendation: String,
    pub verification: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Hreflang {
    pub language: String,
    pub href: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PageEvidence {
    pub title: Option<String>,
    pub meta_description: Option<String>,
    pub canonical: Option<String>,
    pub canonical_count: usize,
    pub robots: Vec<String>,
    pub lang: Option<String>,
    pub viewport: Option<String>,
    pub h1: Vec<String>,
    pub h2_count: usize,
    pub link_count: usize,
    pub internal_links: usize,
    pub external_links: usize,
    pub image_count: usize,
    pub images_without_alt: usize,
    pub word_count: usize,
    pub open_graph_title: Option<String>,
    pub open_graph_description: Option<String>,
    pub twitter_card: Option<String>,
    pub json_ld_blocks: usize,
    pub invalid_json_ld_blocks: usize,
    pub json_ld_types: Vec<String>,
    pub hreflang: Vec<Hreflang>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Coverage {
    pub checks_executed: usize,
    pub checks_passed: usize,
    pub static_html_score: usize,
    pub scope: String,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AuditReport {
    pub schema_version: u8,
    pub source: String,
    pub status: Option<u16>,
    pub final_url: Option<String>,
    pub page: PageEvidence,
    pub coverage: Coverage,
    pub findings: Vec<Finding>,
}

#[derive(Debug)]
struct LoadedInput {
    text: String,
    source: String,
    status: Option<u16>,
    final_url: Option<Url>,
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
        "audit" => audit_command(&args[1..]),
        "sitemap" => sitemap_command(&args[1..]),
        "drift" => drift_command(&args[1..]),
        "doctor" => doctor_command(&args[1..]),
        _ => Err(format!("unknown command {command:?}\n\n{HELP}")),
    }
}

fn audit_command(args: &[String]) -> Result<CliOutcome, String> {
    let options = parse_options(args)?;
    reject_unknown(&options, &["--input", "--base-url", "--format"])?;
    let input = required(&options, "--input")?;
    let loaded = load_input(input, ExpectedContent::Html)?;
    let base_url = match options.get("--base-url") {
        Some(value) => Some(validate_public_url(value)?),
        None => loaded.final_url.clone(),
    };
    let report = audit_html(
        &loaded.text,
        &loaded.source,
        loaded.status,
        base_url.as_ref(),
    );
    let format = options.get("--format").map_or("json", String::as_str);
    let output = match format {
        "json" => pretty(&report)?,
        "markdown" => render_audit_markdown(&report),
        _ => return Err("--format must be json or markdown".to_owned()),
    };
    Ok(CliOutcome {
        output,
        exit_code: u8::from(
            report
                .findings
                .iter()
                .any(|finding| matches!(finding.severity, Severity::High | Severity::Critical)),
        ) * 2,
    })
}

fn sitemap_command(args: &[String]) -> Result<CliOutcome, String> {
    let options = parse_options(args)?;
    reject_unknown(&options, &["--input"])?;
    let input = required(&options, "--input")?;
    let loaded = load_input(input, ExpectedContent::Xml)?;
    let report = inspect_sitemap(&loaded.text, &loaded.source)?;
    let invalid = report["valid"].as_bool() == Some(false);
    Ok(CliOutcome {
        output: pretty(&report)?,
        exit_code: if invalid { 2 } else { 0 },
    })
}

fn drift_command(args: &[String]) -> Result<CliOutcome, String> {
    let options = parse_options(args)?;
    reject_unknown(&options, &["--before", "--after"])?;
    let before = read_json_file(required(&options, "--before")?)?;
    let after = read_json_file(required(&options, "--after")?)?;
    let report = compare_audits(&before, &after)?;
    let changed = report["changed_fields"]
        .as_array()
        .is_some_and(|items| !items.is_empty());
    Ok(CliOutcome {
        output: pretty(&report)?,
        exit_code: if changed { 2 } else { 0 },
    })
}

fn doctor_command(args: &[String]) -> Result<CliOutcome, String> {
    if !args.is_empty() {
        return Err("doctor takes no options".to_owned());
    }
    Ok(success(&pretty(&json!({
        "schema_version": 1,
        "runtime": "rust",
        "capabilities": ["static HTML audit", "public URL fetch", "sitemap inspection", "audit drift"],
        "setup_required": [
            "Google Search Console", "Google Analytics 4", "PageSpeed Insights/CrUX",
            "DataForSEO", "Moz", "Bing Webmaster Tools", "maps/GBP", "rendered browser evidence"
        ],
        "note": "doctor reports capability only; it does not read credentials or claim integration access"
    }))?))
}

#[allow(clippy::too_many_lines)]
pub fn audit_html(
    html: &str,
    source: &str,
    status: Option<u16>,
    base_url: Option<&Url>,
) -> AuditReport {
    let document = Html::parse_document(html);
    let page = extract_page(&document, base_url);
    let mut findings = Vec::new();
    let mut passed = 0;
    let mut executed = 0;

    check_title(&page, &mut findings, &mut executed, &mut passed);
    check_description(&page, &mut findings, &mut executed, &mut passed);
    check_h1(&page, &mut findings, &mut executed, &mut passed);
    check_presence(
        page.canonical_count == 1,
        "SEO-CANONICAL-MISSING",
        "medium",
        "Canonical declaration needs review",
        &format!("Observed {} rel=canonical elements.", page.canonical_count),
        "Add exactly one absolute canonical URL when page should be indexed.",
        "Fetch final HTML and confirm one canonical points at intended index URL.",
        &mut findings,
        &mut executed,
        &mut passed,
    );
    check_presence(
        page.lang.is_some(),
        "SEO-LANG-MISSING",
        "low",
        "Document language missing",
        "html element has no lang attribute.",
        "Set a valid BCP 47 language tag on html.",
        "Inspect rendered html element and validate language tag.",
        &mut findings,
        &mut executed,
        &mut passed,
    );
    check_presence(
        page.viewport.is_some(),
        "SEO-VIEWPORT-MISSING",
        "medium",
        "Viewport metadata missing",
        "No meta viewport found in static HTML.",
        "Add a responsive viewport declaration.",
        "Test rendered page on narrow and wide viewports.",
        &mut findings,
        &mut executed,
        &mut passed,
    );
    executed += 1;
    if page.image_count == 0 || page.images_without_alt == 0 {
        passed += 1;
    } else {
        findings.push(finding(
            "SEO-IMG-ALT",
            Severity::Medium,
            "Images lack alt attributes",
            format!(
                "{} of {} images have no alt attribute.",
                page.images_without_alt, page.image_count
            ),
            "Add useful alt text to informative images and empty alt text to decorative images.",
            "Re-audit static HTML and inspect representative rendered images.",
        ));
    }
    executed += 1;
    if page.robots.iter().any(|value| value == "noindex") {
        findings.push(finding(
            "SEO-NOINDEX",
            Severity::High,
            "Page declares noindex",
            "Meta robots contains noindex.",
            "Confirm exclusion is intentional; remove noindex before indexing if not.",
            "Fetch final HTML and verify indexing directive plus X-Robots-Tag response headers.",
        ));
    } else {
        passed += 1;
    }
    executed += 1;
    if status.is_none_or(|value| (200..400).contains(&value)) {
        passed += 1;
    } else {
        findings.push(finding(
            "SEO-HTTP-STATUS",
            Severity::High,
            "Page returned non-success status",
            format!("Observed HTTP status {}.", status.unwrap_or_default()),
            "Restore successful delivery or use an intentional redirect.",
            "Fetch URL again without browser cache and confirm final response chain.",
        ));
    }
    executed += 1;
    if page.invalid_json_ld_blocks == 0 {
        passed += 1;
    } else {
        findings.push(finding(
            "SEO-JSONLD-INVALID",
            Severity::Medium,
            "JSON-LD block is invalid JSON",
            format!(
                "{} of {} JSON-LD blocks failed JSON parsing.",
                page.invalid_json_ld_blocks, page.json_ld_blocks
            ),
            "Repair JSON syntax, then validate vocabulary and visible-content alignment.",
            "Parse every final HTML JSON-LD block and run relevant official validators.",
        ));
    }

    findings.sort_by_key(|item| std::cmp::Reverse(item.severity));
    let score = (passed * 100).checked_div(executed).unwrap_or(0);
    AuditReport {
        schema_version: 1,
        source: source.to_owned(),
        status,
        final_url: base_url.map(ToString::to_string),
        page,
        coverage: Coverage {
            checks_executed: executed,
            checks_passed: passed,
            static_html_score: score,
            scope: "observed static HTML and response status only".to_owned(),
            limitations: vec![
                "no JavaScript rendering or interaction".to_owned(),
                "no recursive crawl or broken-link requests".to_owned(),
                "no ranking, traffic, backlink, Core Web Vitals, or indexation data".to_owned(),
                "robots.txt and response X-Robots-Tag require separate evidence".to_owned(),
            ],
        },
        findings,
    }
}

fn extract_page(document: &Html, base_url: Option<&Url>) -> PageEvidence {
    let title = first_text(document, "title");
    let h1 = all_text(document, "h1");
    let h2_count = count(document, "h2");
    let body_text = all_text(document, "body").join(" ");
    let word_count = body_text.split_whitespace().count();
    let lang = select(document, "html")
        .next()
        .and_then(|node| attribute(&node, "lang"));

    let mut metadata = BTreeMap::new();
    for node in select(document, "meta") {
        let key = attribute(&node, "name")
            .or_else(|| attribute(&node, "property"))
            .map(|value| value.to_ascii_lowercase());
        if let (Some(key), Some(content)) = (key, attribute(&node, "content")) {
            metadata.entry(key).or_insert(content);
        }
    }

    let canonicals: Vec<ElementRef<'_>> = select(document, "link")
        .filter(|node| rel_contains(node, "canonical"))
        .collect();
    let canonical = canonicals
        .first()
        .and_then(|node| attribute(node, "href"))
        .map(|href| resolve_url(base_url, &href));

    let hreflang = select(document, "link")
        .filter(|node| rel_contains(node, "alternate"))
        .filter_map(|node| {
            Some(Hreflang {
                language: attribute(&node, "hreflang")?,
                href: resolve_url(base_url, &attribute(&node, "href")?),
            })
        })
        .collect();

    let mut robots = BTreeSet::new();
    for key in ["robots", "googlebot"] {
        if let Some(value) = metadata.get(key) {
            for directive in value.split(',') {
                let normalized = directive.trim().to_ascii_lowercase();
                if !normalized.is_empty() {
                    robots.insert(normalized);
                }
            }
        }
    }

    let images: Vec<ElementRef<'_>> = select(document, "img").collect();
    let images_without_alt = images
        .iter()
        .filter(|image| image.value().attr("alt").is_none())
        .count();

    let links: Vec<String> = select(document, "a")
        .filter_map(|node| attribute(&node, "href"))
        .filter(|href| !href.starts_with('#'))
        .map(|href| resolve_url(base_url, &href))
        .collect();
    let (internal_links, external_links) = classify_links(&links, base_url);

    let mut json_ld_types = BTreeSet::new();
    let json_ld_nodes: Vec<ElementRef<'_>> =
        select(document, "script[type='application/ld+json']").collect();
    let mut invalid_json_ld_blocks = 0;
    for node in &json_ld_nodes {
        let raw = node.text().collect::<String>();
        if let Ok(value) = serde_json::from_str::<Value>(&raw) {
            collect_json_ld_types(&value, &mut json_ld_types);
        } else {
            invalid_json_ld_blocks += 1;
        }
    }

    PageEvidence {
        title,
        meta_description: metadata.get("description").cloned(),
        canonical,
        canonical_count: canonicals.len(),
        robots: robots.into_iter().collect(),
        lang,
        viewport: metadata.get("viewport").cloned(),
        h1,
        h2_count,
        link_count: links.len(),
        internal_links,
        external_links,
        image_count: images.len(),
        images_without_alt,
        word_count,
        open_graph_title: metadata.get("og:title").cloned(),
        open_graph_description: metadata.get("og:description").cloned(),
        twitter_card: metadata.get("twitter:card").cloned(),
        json_ld_blocks: json_ld_nodes.len(),
        invalid_json_ld_blocks,
        json_ld_types: json_ld_types.into_iter().collect(),
        hreflang,
    }
}

fn check_title(
    page: &PageEvidence,
    findings: &mut Vec<Finding>,
    executed: &mut usize,
    passed: &mut usize,
) {
    *executed += 1;
    match page.title.as_deref() {
        None | Some("") => findings.push(finding(
            "SEO-TITLE-MISSING",
            Severity::High,
            "Title missing",
            "No non-empty title element found.",
            "Add a concise, unique title matching page intent.",
            "Fetch final HTML and confirm one non-empty title.",
        )),
        Some(value) if !(15..=60).contains(&value.chars().count()) => findings.push(finding(
            "SEO-TITLE-LENGTH",
            Severity::Low,
            "Title length needs review",
            format!(
                "Observed {} characters; 15-60 is a review heuristic, not a platform limit.",
                value.chars().count()
            ),
            "Rewrite for clarity and likely SERP display; do not pad for a score.",
            "Compare rendered search snippets and query intent.",
        )),
        Some(_) => *passed += 1,
    }
}

fn check_description(
    page: &PageEvidence,
    findings: &mut Vec<Finding>,
    executed: &mut usize,
    passed: &mut usize,
) {
    *executed += 1;
    match page.meta_description.as_deref() {
        None | Some("") => findings.push(finding(
            "SEO-DESCRIPTION-MISSING",
            Severity::Medium,
            "Meta description missing",
            "No non-empty meta description found.",
            "Add a distinct summary aligned with search intent and page content.",
            "Fetch final HTML and inspect likely search snippet rendering.",
        )),
        Some(value) if !(50..=160).contains(&value.chars().count()) => findings.push(finding(
            "SEO-DESCRIPTION-LENGTH",
            Severity::Low,
            "Meta description length needs review",
            format!(
                "Observed {} characters; 50-160 is a review heuristic, not a platform limit.",
                value.chars().count()
            ),
            "Prioritize useful summary and differentiation over fixed character targets.",
            "Review query-specific snippets because search engines may rewrite descriptions.",
        )),
        Some(_) => *passed += 1,
    }
}

fn check_h1(
    page: &PageEvidence,
    findings: &mut Vec<Finding>,
    executed: &mut usize,
    passed: &mut usize,
) {
    *executed += 1;
    if page.h1.len() == 1 && !page.h1[0].is_empty() {
        *passed += 1;
    } else {
        findings.push(finding(
            "SEO-H1-COUNT",
            Severity::Medium,
            "Primary heading needs review",
            format!("Observed {} h1 elements.", page.h1.len()),
            "Provide one clear primary heading unless document semantics justify another structure.",
            "Inspect rendered heading hierarchy and accessibility tree.",
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn check_presence(
    present: bool,
    id: &str,
    severity: &str,
    title: &str,
    evidence: &str,
    recommendation: &str,
    verification: &str,
    findings: &mut Vec<Finding>,
    executed: &mut usize,
    passed: &mut usize,
) {
    *executed += 1;
    if present {
        *passed += 1;
    } else {
        findings.push(finding(
            id,
            match severity {
                "low" => Severity::Low,
                "medium" => Severity::Medium,
                "high" => Severity::High,
                "critical" => Severity::Critical,
                _ => Severity::Info,
            },
            title,
            evidence,
            recommendation,
            verification,
        ));
    }
}

fn finding(
    id: &str,
    severity: Severity,
    title: &str,
    evidence: impl Into<String>,
    recommendation: &str,
    verification: &str,
) -> Finding {
    Finding {
        id: id.to_owned(),
        severity,
        title: title.to_owned(),
        evidence: evidence.into(),
        recommendation: recommendation.to_owned(),
        verification: verification.to_owned(),
    }
}

fn inspect_sitemap(xml: &str, source: &str) -> Result<Value, String> {
    let document =
        roxmltree::Document::parse(xml).map_err(|error| format!("invalid sitemap XML: {error}"))?;
    let root = document.root_element().tag_name().name();
    let kind = match root {
        "urlset" => "urlset",
        "sitemapindex" => "sitemapindex",
        _ => "unknown",
    };
    let locations: Vec<String> = document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "loc")
        .filter_map(|node| node.text())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    let mut seen = BTreeSet::new();
    let duplicates: Vec<String> = locations
        .iter()
        .filter(|value| !seen.insert((*value).clone()))
        .cloned()
        .collect();
    let invalid_urls: Vec<String> = locations
        .iter()
        .filter(|value| {
            Url::parse(value).map_or(true, |url| {
                !matches!(url.scheme(), "http" | "https") || url.host().is_none()
            })
        })
        .cloned()
        .collect();
    let count_limit = 50_000;
    let valid = kind != "unknown"
        && !locations.is_empty()
        && locations.len() <= count_limit
        && duplicates.is_empty()
        && invalid_urls.is_empty();
    Ok(json!({
        "schema_version": 1,
        "source": source,
        "kind": kind,
        "valid": valid,
        "location_count": locations.len(),
        "location_limit": count_limit,
        "duplicate_locations": duplicates,
        "invalid_locations": invalid_urls,
        "note": "XML shape and URL syntax only; indexation, lastmod accuracy, robots access, and fetch status require separate evidence"
    }))
}

fn compare_audits(before: &Value, after: &Value) -> Result<Value, String> {
    for (label, value) in [("before", before), ("after", after)] {
        if value["schema_version"].as_u64() != Some(1) || value["page"].as_object().is_none() {
            return Err(format!("{label} is not a codex-seo audit schema version 1"));
        }
    }
    let paths = [
        "/status",
        "/final_url",
        "/page/title",
        "/page/meta_description",
        "/page/canonical",
        "/page/canonical_count",
        "/page/robots",
        "/page/lang",
        "/page/h1",
        "/page/invalid_json_ld_blocks",
        "/page/json_ld_types",
        "/page/hreflang",
    ];
    let changes: Vec<Value> = paths
        .iter()
        .filter_map(|path| {
            let old = before.pointer(path).cloned().unwrap_or(Value::Null);
            let new = after.pointer(path).cloned().unwrap_or(Value::Null);
            (old != new).then(|| json!({"path": path, "before": old, "after": new}))
        })
        .collect();
    Ok(json!({
        "schema_version": 1,
        "changed": !changes.is_empty(),
        "changed_fields": changes,
        "note": "Only high-risk audit fields are compared; investigate deploy context before assigning cause."
    }))
}

#[derive(Clone, Copy)]
enum ExpectedContent {
    Html,
    Xml,
}

fn load_input(input: &str, expected: ExpectedContent) -> Result<LoadedInput, String> {
    if input.starts_with("http://") || input.starts_with("https://") {
        fetch_public(input, expected)
    } else {
        let path = Path::new(input);
        let text = fs::read_to_string(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        if text.len() as u64 > MAX_RESPONSE_BYTES {
            return Err(format!("input exceeds {MAX_RESPONSE_BYTES} byte limit"));
        }
        Ok(LoadedInput {
            text,
            source: path.display().to_string(),
            status: None,
            final_url: None,
        })
    }
}

fn fetch_public(input: &str, expected: ExpectedContent) -> Result<LoadedInput, String> {
    let mut current = validate_public_url(input)?;
    for redirect in 0..=MAX_REDIRECTS {
        let (response, resolved_url) = request_once(&current)?;
        if response.status().is_redirection() {
            if redirect == MAX_REDIRECTS {
                return Err(format!("redirect limit of {MAX_REDIRECTS} exceeded"));
            }
            let location = response
                .headers()
                .get(LOCATION)
                .ok_or_else(|| "redirect response lacks Location header".to_owned())?
                .to_str()
                .map_err(|error| format!("invalid redirect Location header: {error}"))?;
            current = validate_public_url(
                current
                    .join(location)
                    .map_err(|error| format!("invalid redirect URL: {error}"))?
                    .as_str(),
            )?;
            continue;
        }
        validate_content_type(&response, expected)?;
        let status = response.status().as_u16();
        let text = read_limited(response)?;
        return Ok(LoadedInput {
            text,
            source: input.to_owned(),
            status: Some(status),
            final_url: Some(resolved_url),
        });
    }
    Err("unreachable redirect state".to_owned())
}

fn request_once(url: &Url) -> Result<(Response, Url), String> {
    let host = url
        .host_str()
        .ok_or_else(|| "URL must contain a host".to_owned())?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| "URL has no usable port".to_owned())?;
    let addresses = resolve_public(host, port)?;
    let pinned = *addresses
        .first()
        .ok_or_else(|| "host resolved to no addresses".to_owned())?;
    let client = Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .redirect(Policy::none())
        .no_proxy()
        .resolve(host, pinned)
        .build()
        .map_err(|error| format!("cannot build HTTP client: {error}"))?;
    let response = client
        .get(url.clone())
        .header("User-Agent", USER_AGENT)
        .send()
        .map_err(|error| format!("request failed: {error}"))?;
    Ok((response, url.clone()))
}

fn validate_public_url(value: &str) -> Result<Url, String> {
    let url = Url::parse(value).map_err(|error| format!("invalid URL: {error}"))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("URL scheme must be http or https".to_owned());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("URL credentials are not allowed".to_owned());
    }
    if url.host().is_none() {
        return Err("URL must contain a host".to_owned());
    }
    if let Some(host) = url.host() {
        match host {
            Host::Ipv4(address) => ensure_public_ip(IpAddr::V4(address))?,
            Host::Ipv6(address) => ensure_public_ip(IpAddr::V6(address))?,
            Host::Domain(_) => {}
        }
    }
    Ok(url)
}

fn resolve_public(host: &str, port: u16) -> Result<Vec<SocketAddr>, String> {
    let addresses: Vec<SocketAddr> = (host, port)
        .to_socket_addrs()
        .map_err(|error| format!("cannot resolve host: {error}"))?
        .collect();
    if addresses.is_empty() {
        return Err("host resolved to no addresses".to_owned());
    }
    for address in &addresses {
        ensure_public_ip(address.ip())?;
    }
    Ok(addresses)
}

fn ensure_public_ip(ip: IpAddr) -> Result<(), String> {
    let blocked = match ip {
        IpAddr::V4(ip) => blocked_ipv4(ip),
        IpAddr::V6(ip) => blocked_ipv6(ip),
    };
    if blocked {
        Err(format!("local or non-public address is not allowed: {ip}"))
    } else {
        Ok(())
    }
}

fn blocked_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, _, _] = ip.octets();
    ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_multicast()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || a == 0
        || (a == 100 && (64..=127).contains(&b))
        || (a == 192 && b == 0)
        || (a == 192 && b == 0 && ip.octets()[2] == 2)
        || (a == 198 && matches!(b, 18 | 19))
        || (a == 198 && b == 51 && ip.octets()[2] == 100)
        || (a == 203 && b == 0 && ip.octets()[2] == 113)
        || a >= 240
}

fn blocked_ipv6(ip: Ipv6Addr) -> bool {
    let segments = ip.segments();
    let first = segments[0];
    ip.is_loopback()
        || ip.is_unspecified()
        || ip.is_multicast()
        || (first & 0xfe00) == 0xfc00
        || (first & 0xffc0) == 0xfe80
        || (first & 0xffc0) == 0xfec0
        || (first == 0x0100 && segments[1..4].iter().all(|segment| *segment == 0))
        || (first == 0x2001 && segments[1] == 0x0002)
        || (first == 0x2001 && segments[1] == 0x0db8)
        || (first == 0x2001 && (0x0010..=0x002f).contains(&segments[1]))
        || ip.to_ipv4_mapped().is_some_and(blocked_ipv4)
}

fn validate_content_type(response: &Response, expected: ExpectedContent) -> Result<(), String> {
    let Some(value) = response.headers().get(CONTENT_TYPE) else {
        return Ok(());
    };
    let value = value
        .to_str()
        .map_err(|error| format!("invalid Content-Type header: {error}"))?
        .to_ascii_lowercase();
    let accepted = match expected {
        ExpectedContent::Html => {
            value.contains("text/html") || value.contains("application/xhtml+xml")
        }
        ExpectedContent::Xml => value.contains("xml") || value.contains("text/plain"),
    };
    if accepted {
        Ok(())
    } else {
        Err(format!("unexpected Content-Type {value:?}"))
    }
}

fn read_limited(response: Response) -> Result<String, String> {
    let mut bytes = Vec::new();
    response
        .take(MAX_RESPONSE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read response: {error}"))?;
    if bytes.len() as u64 > MAX_RESPONSE_BYTES {
        return Err(format!("response exceeds {MAX_RESPONSE_BYTES} byte limit"));
    }
    String::from_utf8(bytes).map_err(|error| format!("response is not UTF-8: {error}"))
}

fn first_text(document: &Html, selector: &str) -> Option<String> {
    select(document, selector)
        .map(|node| normalized_text(&node))
        .find(|text| !text.is_empty())
}

fn all_text(document: &Html, selector: &str) -> Vec<String> {
    select(document, selector)
        .map(|node| normalized_text(&node))
        .filter(|text| !text.is_empty())
        .collect()
}

fn normalized_text(node: &ElementRef<'_>) -> String {
    node.text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn select<'a>(document: &'a Html, selector: &str) -> std::vec::IntoIter<ElementRef<'a>> {
    let selector = Selector::parse(selector).expect("hard-coded selector must parse");
    document.select(&selector).collect::<Vec<_>>().into_iter()
}

fn count(document: &Html, selector: &str) -> usize {
    select(document, selector).count()
}

fn attribute(node: &ElementRef<'_>, name: &str) -> Option<String> {
    node.value()
        .attr(name)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn rel_contains(node: &ElementRef<'_>, token: &str) -> bool {
    node.value().attr("rel").is_some_and(|value| {
        value
            .split_ascii_whitespace()
            .any(|part| part.eq_ignore_ascii_case(token))
    })
}

fn resolve_url(base: Option<&Url>, value: &str) -> String {
    Url::parse(value)
        .or_else(|_| {
            base.ok_or(url::ParseError::RelativeUrlWithoutBase)?
                .join(value)
        })
        .map_or_else(|_| value.to_owned(), |url| url.to_string())
}

fn classify_links(links: &[String], base: Option<&Url>) -> (usize, usize) {
    let Some(base_host) = base.and_then(Url::host_str) else {
        return (0, links.len());
    };
    links.iter().fold((0, 0), |(internal, external), link| {
        if Url::parse(link)
            .ok()
            .and_then(|url| url.host_str().map(ToOwned::to_owned))
            .as_deref()
            == Some(base_host)
        {
            (internal + 1, external)
        } else {
            (internal, external + 1)
        }
    })
}

fn collect_json_ld_types(value: &Value, types: &mut BTreeSet<String>) {
    match value {
        Value::Object(object) => {
            if let Some(kind) = object.get("@type") {
                match kind {
                    Value::String(value) => {
                        types.insert(value.clone());
                    }
                    Value::Array(values) => {
                        for value in values.iter().filter_map(Value::as_str) {
                            types.insert(value.to_owned());
                        }
                    }
                    _ => {}
                }
            }
            for nested in object.values() {
                collect_json_ld_types(nested, types);
            }
        }
        Value::Array(values) => {
            for nested in values {
                collect_json_ld_types(nested, types);
            }
        }
        _ => {}
    }
}

fn render_audit_markdown(report: &AuditReport) -> String {
    let mut output = format!(
        "# SEO audit\n\n- Source: `{}`\n- Static HTML score: {}/100 ({} of {} checks)\n- Scope: {}\n\n",
        report.source,
        report.coverage.static_html_score,
        report.coverage.checks_passed,
        report.coverage.checks_executed,
        report.coverage.scope
    );
    output.push_str("## Findings\n\n");
    if report.findings.is_empty() {
        output.push_str("No findings from executed checks.\n");
    } else {
        for finding in &report.findings {
            let _ = write!(
                output,
                "### {:?}: {} (`{}`)\n\n{}\n\nFix: {}\n\nVerify: {}\n\n",
                finding.severity,
                finding.title,
                finding.id,
                finding.evidence,
                finding.recommendation,
                finding.verification
            );
        }
    }
    output.push_str("## Limitations\n\n");
    for limitation in &report.coverage.limitations {
        let _ = writeln!(output, "- {limitation}");
    }
    output
}

fn read_json_file(path: &str) -> Result<Value, String> {
    let text = fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("invalid JSON in {path}: {error}"))
}

fn parse_options(args: &[String]) -> Result<BTreeMap<String, String>, String> {
    if !args.len().is_multiple_of(2) {
        return Err("every option requires one value".to_owned());
    }
    let mut values = BTreeMap::new();
    let (pairs, remainder) = args.as_chunks::<2>();
    debug_assert!(remainder.is_empty());
    for pair in pairs {
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

fn pretty<T: Serialize>(value: &T) -> Result<String, String> {
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
    use super::{
        audit_html, blocked_ipv4, blocked_ipv6, compare_audits, inspect_sitemap, run_cli, Severity,
    };
    use serde_json::json;
    use std::net::{Ipv4Addr, Ipv6Addr};
    use url::Url;

    const COMPLETE_HTML: &str = r#"<!doctype html>
<html lang="en"><head><title>Evidence-led SEO audit page</title>
<meta name="description" content="A specific, evidence-led description long enough for deterministic review without pretending it controls search snippets.">
<meta name="viewport" content="width=device-width, initial-scale=1">
<link rel="canonical" href="/guide"><link rel="alternate" hreflang="fr" href="/fr/guide">
<meta property="og:title" content="SEO guide"><meta name="twitter:card" content="summary">
<script type="application/ld+json">{"@context":"https://schema.org","@type":"Article"}</script>
</head><body><h1>SEO guide</h1><h2>Evidence</h2><p>This page contains enough words for parsing.</p>
<a href="/next">Next</a><a href="https://example.net">External</a><img src="hero.jpg" alt="Audit report"></body></html>"#;

    #[test]
    fn complete_html_produces_structured_evidence() {
        let base = Url::parse("https://example.com/guide").unwrap();
        let report = audit_html(COMPLETE_HTML, "fixture", Some(200), Some(&base));
        assert_eq!(
            report.page.title.as_deref(),
            Some("Evidence-led SEO audit page")
        );
        assert_eq!(
            report.page.canonical.as_deref(),
            Some("https://example.com/guide")
        );
        assert_eq!(report.page.internal_links, 1);
        assert_eq!(report.page.external_links, 1);
        assert_eq!(report.page.json_ld_types, vec!["Article"]);
        assert_eq!(report.page.hreflang[0].language, "fr");
        assert!(!report
            .findings
            .iter()
            .any(|item| item.severity == Severity::High));
    }

    #[test]
    fn missing_signals_have_stable_findings() {
        let report = audit_html(
            "<html><body><img src='x'></body></html>",
            "fixture",
            None,
            None,
        );
        let ids: Vec<&str> = report
            .findings
            .iter()
            .map(|item| item.id.as_str())
            .collect();
        assert!(ids.contains(&"SEO-TITLE-MISSING"));
        assert!(ids.contains(&"SEO-DESCRIPTION-MISSING"));
        assert!(ids.contains(&"SEO-H1-COUNT"));
        assert!(ids.contains(&"SEO-IMG-ALT"));
        assert!(report.coverage.static_html_score < 50);
    }

    #[test]
    fn sitemap_reports_duplicates_and_bad_urls() {
        let report = inspect_sitemap(
            "<urlset><url><loc>https://example.com/a</loc></url><url><loc>https://example.com/a</loc></url><url><loc>ftp://example.com/b</loc></url></urlset>",
            "fixture",
        )
        .unwrap();
        assert_eq!(report["valid"], false);
        assert_eq!(report["location_count"], 3);
        assert_eq!(report["duplicate_locations"][0], "https://example.com/a");
        assert_eq!(report["invalid_locations"][0], "ftp://example.com/b");
    }

    #[test]
    fn drift_compares_only_high_risk_fields() {
        let before =
            json!({"schema_version": 1, "status": 200, "page": {"title": "Old", "word_count": 10}});
        let after =
            json!({"schema_version": 1, "status": 200, "page": {"title": "New", "word_count": 99}});
        let report = compare_audits(&before, &after).unwrap();
        assert_eq!(report["changed_fields"].as_array().unwrap().len(), 1);
        assert_eq!(report["changed_fields"][0]["path"], "/page/title");
    }

    #[test]
    fn private_and_special_addresses_are_blocked() {
        for address in [
            Ipv4Addr::LOCALHOST,
            Ipv4Addr::new(10, 0, 0, 1),
            Ipv4Addr::new(169, 254, 1, 1),
            Ipv4Addr::new(100, 64, 0, 1),
            Ipv4Addr::new(203, 0, 113, 1),
        ] {
            assert!(blocked_ipv4(address));
        }
        assert!(!blocked_ipv4(Ipv4Addr::new(1, 1, 1, 1)));
        assert!(blocked_ipv6(Ipv6Addr::LOCALHOST));
        assert!(blocked_ipv6("fc00::1".parse().unwrap()));
        assert!(!blocked_ipv6("2606:4700:4700::1111".parse().unwrap()));
    }

    #[test]
    fn doctor_is_explicit_about_setup() {
        let outcome = run_cli(["doctor".to_owned()]).unwrap();
        assert_eq!(outcome.exit_code, 0);
        assert!(outcome.output.contains("setup_required"));
        assert!(outcome.output.contains("Google Search Console"));
    }

    #[test]
    fn cli_rejects_loopback_url_before_request() {
        let error = run_cli([
            "audit".to_owned(),
            "--input".to_owned(),
            "http://127.0.0.1/secret".to_owned(),
        ])
        .unwrap_err();
        assert!(error.contains("local or non-public address"));
    }
}

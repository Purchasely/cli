//! The checks of `test/docs.test.ts` on the doc files and the site, so they stay after the TS tests go.
//! The two checks that need the command tree are in `tests/binary.rs`.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn read(file: &str) -> String {
    std::fs::read_to_string(root().join(file)).unwrap_or_else(|e| panic!("{file}: {e}"))
}
const DOCS: [&str; 7] = [
    "docs/install.md",
    "docs/commands.md",
    "docs/guardrails.md",
    "docs/exit-codes.md",
    "docs/limits.md",
    "docs/development.md",
    "docs/release.md",
];
/// README.md, AGENTS.md and every `docs/*.md`.
fn md_files() -> Vec<String> {
    let mut docs = std::fs::read_dir(root().join("docs"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|f| f.ends_with(".md"))
        .map(|f| format!("docs/{f}"))
        .collect::<Vec<_>>();
    docs.sort();
    ["README.md".to_owned(), "AGENTS.md".to_owned()]
        .into_iter()
        .chain(docs)
        .collect()
}
/// The targets of the Markdown links of `text`: the TS regex `\]\(([^)\s]+)\)`.
fn link_targets(text: &str) -> Vec<&str> {
    let mut targets = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("](") {
        let after = &rest[at + 2..];
        let end = after
            .find(|c: char| c == ')' || c.is_whitespace())
            .unwrap_or(after.len());
        if end > 0 && after[end..].starts_with(')') {
            targets.push(&after[..end]);
        }
        rest = after;
    }
    targets
}
fn has_dash(text: &str) -> bool {
    text.contains(['\u{2013}', '\u{2014}'])
}

#[test]
fn link_targets_follow_the_ts_pattern() {
    assert_eq!(
        link_targets("[a](x.md) [b](https://y) [c](has space) [d]() [e](z.md#top)"),
        ["x.md", "https://y", "z.md#top"]
    );
}

/// docs.test.ts "links resolve".
#[test]
fn links_resolve() {
    for f in DOCS {
        assert!(root().join(f).exists(), "{f}");
    }
    for f in md_files() {
        let text = read(&f);
        let dir = Path::new(&f).parent().unwrap().to_owned();
        for link in link_targets(&text) {
            let target = link.split('#').next().unwrap();
            let scheme = target
                .split_once(':')
                .is_some_and(|(s, _)| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphabetic()));
            if target.is_empty() || scheme {
                continue;
            }
            assert!(root().join(&dir).join(target).exists(), "{f} -> {link}");
        }
    }
    // The README is the entry point: it links to every reference file.
    let readme = read("README.md");
    for f in DOCS.iter().chain(&["AGENTS.md"]) {
        assert!(readme.contains(&format!("]({f})")), "{f}");
    }
}

/// docs.test.ts "ai section: readmes and landing page".
#[test]
fn ai_section_readmes_and_landing_page() {
    for f in ["README.md", "site/index.html"] {
        let text = read(f);
        let title = if f.starts_with("site/") {
            "The Purchasely AI plugin"
        } else {
            "Use with your AI agent"
        };
        assert!(text.contains(title), "{f}");
        for w in [
            "purchasely ai install",
            "ai status",
            "ai update",
            "ai uninstall",
            "--agent",
            "purchasely-cli",
            "sdk-expert",
            "Claude Code",
            "Codex",
            "Copilot CLI",
            "Gemini CLI",
            "Cursor",
            "OpenCode",
            "SDK API key",
        ] {
            assert!(text.contains(w), "{f}: {w}");
        }
    }
    assert!(read("README.md").split('\n').count() < 100);
    let site = read("site/index.html");
    assert!(site.contains("href=\"#ai\""));
    assert!(site.contains("id=\"ai\""));
}

/// docs.test.ts "landing page: sharing head, reference links, real 404 and robots.txt".
#[test]
fn landing_page_sharing_head_reference_links_404_and_robots() {
    let site = read("site/index.html");
    assert!(site.contains(
        "<title>Purchasely CLI: your account in the terminal, the CI and your AI agent</title>"
    ));
    for tag in [
        "rel=\"canonical\" href=\"https://cli.purchasely.io/\"",
        "property=\"og:image\" content=\"https://cli.purchasely.io/assets/og.png\"",
        "name=\"twitter:card\" content=\"summary_large_image\"",
    ] {
        assert!(site.contains(tag), "{tag}");
    }
    assert!(root().join("site/assets/og.png").exists());
    assert!(site.contains("href=\"/reference\""));
    assert!(site.contains("href=\"https://docs.purchasely.com/docs/management-api\""));
    // The dry-run sample shows the real User-Agent form. The formula lives in this repo, so brew needs the tap URL.
    assert!(site.contains(
        "\"User-Agent\": \"purchasely-cli/&lt;version&gt; (-; -; placements update; tty)\""
    ));
    let lower = site.to_lowercase();
    assert!(!lower.contains("@placement.json"));
    assert!(site.contains("brew tap purchasely/cli https://github.com/Purchasely/cli"));
    assert!(site.contains("brew trust purchasely/cli"));
    assert!(!site.contains("Install with <code>brew install purchasely</code>"));
    let not_found = read("site/404.html");
    assert!(not_found.contains("href=\"/\""));
    // Served at any depth: assets need absolute paths.
    for attr in ["src", "href"] {
        for quote in ["", "\"", "'"] {
            let relative = format!("{attr}={quote}assets/");
            assert!(!not_found.contains(&relative), "{relative}");
        }
    }
    for f in ["site/index.html", "site/404.html"] {
        assert!(!has_dash(&read(f)), "{f}");
    }
    assert_eq!(read("site/robots.txt"), "User-agent: *\nAllow: /\n");
}

/// docs.test.ts "ai section: commands, install, limits, release".
#[test]
fn ai_section_commands_install_limits_release() {
    let doc = read("docs/commands.md");
    for c in ["ai install", "ai status", "ai update", "ai uninstall"] {
        assert!(doc.contains(&format!("purchasely {c}")), "{c}");
    }
    let install = read("docs/install.md");
    for w in ["ai.plugin.json", "daily", "never touch"] {
        assert!(install.contains(w), "{w}");
    }
    let limits = read("docs/limits.md");
    for w in ["gemini extensions update purchasely", "120 s", "restart"] {
        assert!(limits.contains(w), "{w}");
    }
    // The release doc names what the workflow pins.
    let wf = read(".github/workflows/release.yml");
    let rel = read("docs/release.md");
    let at = wf.find("wrangler@").expect("release.yml pins wrangler");
    let pin = &wf[at..];
    let end = "wrangler@".len()
        + pin["wrangler@".len()..]
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(pin.len() - "wrangler@".len());
    assert!(end > "wrangler@".len());
    assert!(rel.contains(&pin[..end]), "{}", &pin[..end]);
    // CI and the release never need more than read access.
    for f in [".github/workflows/ci.yml", ".github/workflows/release.yml"] {
        let lines = read(f).lines().map(str::to_owned).collect::<Vec<_>>();
        assert!(
            lines
                .windows(2)
                .any(|w| w[0] == "permissions:" && w[1] == "  contents: read"),
            "{f}"
        );
    }
    for w in [
        "--branch=main",
        "SHA256SUMS",
        "install.sh",
        "_headers",
        "latest.json",
        "scripts/pack-release.sh",
    ] {
        assert!(rel.contains(w), "{w}");
    }
}

/// docs.test.ts "uninstall and skill count match the code".
#[test]
fn uninstall_and_skill_count_match_the_code() {
    let readme = read("README.md");
    assert!(readme.contains("command -v purchasely"));
    assert!(!readme.contains("~/.local/bin/purchasely"));
    assert!(readme.contains("6 skills from plugin 2.3.0 (5 before)"));
}

/// docs.test.ts "no em-dash or en-dash".
#[test]
fn no_em_dash_or_en_dash() {
    for f in md_files() {
        assert!(!has_dash(&read(&f)), "{f}");
    }
}

//! Pacman update checking — same `pacman -Qu` invocation and line grammar as
//! `SysData.qml`, but parsed once into owned rows instead of a JS array.

use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Update {
    pub name: String,
    pub old_version: String,
    pub new_version: String,
    pub extra: String,
}

/// Parses one `pacman -Qu` line: `pkg 1.0-1 -> 2.0-1` (`extra` keeps any
/// trailing text, matching the QML regex `(\S+)\s+(\S+)\s+->\s+(\S+)(.*)$`).
pub fn parse_line(line: &str) -> Option<Update> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let mut parts = line.split_whitespace();
    let name = parts.next()?;
    // Accept both "a -> b" and fall back to a bare package name.
    let rest: Vec<&str> = parts.collect();
    if let Some(arrow) = rest.iter().position(|p| *p == "->") {
        if arrow >= 1 {
            let old = rest[arrow - 1];
            if let Some(new) = rest.get(arrow + 1) {
                let extra = rest.get(arrow + 2..).unwrap_or(&[]).join(" ");
                return Some(Update {
                    name: name.to_string(),
                    old_version: old.to_string(),
                    new_version: new.to_string(),
                    extra,
                });
            }
        }
    }
    Some(Update {
        name: line.to_string(),
        old_version: String::new(),
        new_version: String::new(),
        extra: String::new(),
    })
}

pub fn parse(output: &str) -> Vec<Update> {
    output.lines().filter_map(parse_line).collect()
}

/// Runs the configured update check (`pacman -Qu`); exit status 1 means
/// "no updates", not an error.
pub fn check(cmd: &str) -> Vec<Update> {
    let out = match Command::new("sh").arg("-c").arg(cmd).output() {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };
    parse(&String::from_utf8_lossy(&out.stdout))
}

/// Launches the configured terminal upgrade command (same shape as QML:
/// `<term> -e sh -c "<upgrade>; echo 'Press enter to exit...'; read"`).
pub fn spawn_upgrade(term: &str, upgrade: &str) -> bool {
    Command::new(term)
        .arg("-e")
        .arg("sh")
        .arg("-c")
        .arg(format!("{upgrade}; echo 'Press enter to exit...'; read"))
        .spawn()
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_version_transition() {
        let u = parse_line("linux 6.12.1-1 -> 6.12.2-1").unwrap();
        assert_eq!(u.name, "linux");
        assert_eq!(u.old_version, "6.12.1-1");
        assert_eq!(u.new_version, "6.12.2-1");
        assert_eq!(u.extra, "");
    }

    #[test]
    fn parses_extra_text() {
        let u = parse_line("foo 1 -> 2 [ignored]").unwrap();
        assert_eq!(u.old_version, "1");
        assert_eq!(u.new_version, "2");
        assert_eq!(u.extra, "[ignored]");
    }

    #[test]
    fn bare_name_falls_back() {
        let u = parse_line("weird-package").unwrap();
        assert_eq!(u.name, "weird-package");
        assert_eq!(u.old_version, "");
    }

    #[test]
    fn empty_output_means_up_to_date() {
        assert!(parse("").is_empty());
        assert!(parse("\n\n").is_empty());
    }

    #[test]
    fn parses_multiple_lines() {
        let out = "a 1 -> 2\nb 3 -> 4\n";
        let v = parse(out);
        assert_eq!(v.len(), 2);
        assert_eq!(v[1].name, "b");
    }
}

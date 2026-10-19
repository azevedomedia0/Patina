//! The `Exec` key: splitting it into arguments and expanding its field codes, per the Desktop
//! Entry Specification ("The Exec key").

use std::path::Path;

/// Splits `Exec` into arguments: spaces separate them, double quotes group them, and inside
/// quotes a backslash escapes `"`, `` ` ``, `$` and `\`.
pub fn split(exec: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut any = false;
    let mut chars = exec.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            '\\' if quoted => {
                if let Some(&n) = chars.peek() {
                    if matches!(n, '"' | '`' | '$' | '\\') {
                        cur.push(n);
                        chars.next();
                        continue;
                    }
                }
                cur.push('\\');
            }
            ' ' | '\t' if !quoted => {
                if any || !cur.is_empty() {
                    args.push(std::mem::take(&mut cur));
                    any = false;
                }
            }
            _ => cur.push(c),
        }
    }
    if any || !cur.is_empty() {
        args.push(cur);
    }
    args
}

/// The arguments to run, with no files or URLs: `%f %F %u %U` (and the deprecated codes) are
/// dropped, `%i` becomes `--icon ICON`, `%c` the name, `%k` the desktop file, `%%` a percent.
pub fn expand(exec: &str, icon: Option<&str>, name: &str, path: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for arg in split(exec) {
        match arg.as_str() {
            "%f" | "%F" | "%u" | "%U" | "%d" | "%D" | "%n" | "%N" | "%v" | "%m" => continue,
            "%i" => {
                if let Some(icon) = icon {
                    out.push("--icon".into());
                    out.push(icon.into());
                }
                continue;
            }
            _ => {}
        }
        let mut s = String::new();
        let mut chars = arg.chars();
        while let Some(c) = chars.next() {
            if c != '%' {
                s.push(c);
                continue;
            }
            match chars.next() {
                Some('%') => s.push('%'),
                Some('c') => s.push_str(name),
                Some('k') => s.push_str(&path.to_string_lossy()),
                // Field codes inside a longer argument (`--file=%f`) expand to nothing.
                Some(_) | None => {}
            }
        }
        out.push(s);
    }
    out
}

/// The program `Exec` runs (its first argument), skipping `env VAR=x` prefixes.
pub fn program(exec: &str) -> Option<String> {
    let args = split(exec);
    let mut it = args.into_iter();
    let mut first = it.next()?;
    if first == "env" {
        first = it.find(|a| !a.contains('='))?;
    }
    Some(first)
}

/// Whether `program` is an absolute path to a file, or is found in `$PATH`.
pub fn in_path(program: &str) -> bool {
    if program.contains('/') {
        return Path::new(program).is_file();
    }
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d.join(program).is_file()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_with_quotes_and_escapes() {
        assert_eq!(split(r#"foo  bar"#), vec!["foo", "bar"]);
        assert_eq!(
            split(r#""/opt/My App/app" --name="a \"b\"" %U"#),
            vec!["/opt/My App/app", r#"--name=a "b""#, "%U"]
        );
        assert_eq!(
            split(r#"sh -c "echo \$HOME""#),
            vec!["sh", "-c", "echo $HOME"]
        );
        assert_eq!(split(r#"x """#), vec!["x", ""]);
    }

    #[test]
    fn expands_field_codes() {
        let p = Path::new("/usr/share/applications/x.desktop");
        assert_eq!(
            expand("app %U --title=%c %i 100%%", Some("app-icon"), "App", p),
            vec!["app", "--title=App", "--icon", "app-icon", "100%"]
        );
        assert_eq!(
            expand("app --file=%f", None, "App", p),
            vec!["app", "--file="]
        );
        assert_eq!(expand("app %i", None, "App", p), vec!["app"]);
    }

    #[test]
    fn finds_the_program() {
        assert_eq!(
            program("env GDK_BACKEND=x11 code %F").as_deref(),
            Some("code")
        );
        assert_eq!(program("/usr/bin/foot").as_deref(), Some("/usr/bin/foot"));
        assert_eq!(program("").as_deref(), None);
    }
}

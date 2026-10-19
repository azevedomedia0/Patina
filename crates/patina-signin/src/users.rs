//! People who can sign in: the accounts in `/etc/passwd` with a login shell and a user id in
//! the range for people (1000 and up, below the `nobody` range).

/// One account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub login: String,
    /// The real name from the account's comment field; may be empty.
    pub real_name: String,
    pub uid: u32,
}

impl User {
    pub fn new(login: &str, real_name: &str, uid: u32) -> Self {
        Self {
            login: login.into(),
            real_name: real_name.into(),
            uid,
        }
    }

    /// The real name, else the login name.
    pub fn display_name(&self) -> String {
        if self.real_name.is_empty() {
            self.login.clone()
        } else {
            self.real_name.clone()
        }
    }

    /// Up to two initials: "Ana Lima" is "AL", "guest" is "G".
    pub fn initials(&self) -> String {
        self.display_name()
            .split_whitespace()
            .take(2)
            .filter_map(|w| w.chars().next())
            .flat_map(char::to_uppercase)
            .collect()
    }

    /// The line under the name in the switch-user list.
    pub fn sub_label(&self) -> String {
        if self.real_name.is_empty() {
            "Local account".into()
        } else {
            self.login.clone()
        }
    }
}

/// Parses `/etc/passwd` text into the people who can sign in, by user id.
pub fn parse_passwd(text: &str) -> Vec<User> {
    let mut users: Vec<User> = text
        .lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.split(':').collect();
            if f.len() < 7 || line.starts_with('#') {
                return None;
            }
            let uid: u32 = f[2].parse().ok()?;
            let shell = f[6];
            let can_log_in = !shell.ends_with("nologin") && !shell.ends_with("/false");
            ((1000..60000).contains(&uid) && can_log_in).then(|| User {
                login: f[0].into(),
                // The comment field: "Full Name,room,phone,..."; the name comes first.
                real_name: f[4].split(',').next().unwrap_or("").trim().into(),
                uid,
            })
        })
        .collect();
    users.sort_by_key(|u| u.uid);
    users
}

pub fn all() -> Vec<User> {
    std::fs::read_to_string("/etc/passwd")
        .map(|t| parse_passwd(&t))
        .unwrap_or_default()
}

/// The user running this program.
pub fn current() -> Option<User> {
    let uid = std::fs::read_to_string("/proc/self/status")
        .ok()?
        .lines()
        .find_map(|l| l.strip_prefix("Uid:"))?
        .split_whitespace()
        .next()?
        .parse::<u32>()
        .ok()?;
    let text = std::fs::read_to_string("/etc/passwd").ok()?;
    text.lines().find_map(|line| {
        let f: Vec<&str> = line.split(':').collect();
        (f.len() >= 5 && f[2].parse::<u32>().ok()? == uid).then(|| User {
            login: f[0].into(),
            real_name: f[4].split(',').next().unwrap_or("").trim().into(),
            uid,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASSWD: &str = "root:x:0:0:root:/root:/bin/bash\n\
        daemon:x:1:1:daemon:/usr/sbin:/usr/sbin/nologin\n\
        ana:x:1000:1000:Ana Lima,,,:/home/ana:/bin/bash\n\
        svc:x:1001:1001::/var/svc:/usr/sbin/nologin\n\
        guest:x:1002:1002::/home/guest:/bin/sh\n\
        nobody:x:65534:65534:nobody:/nonexistent:/usr/sbin/nologin\n";

    #[test]
    fn people_with_login_shells_only() {
        let u = parse_passwd(PASSWD);
        let logins: Vec<_> = u.iter().map(|u| u.login.as_str()).collect();
        assert_eq!(logins, vec!["ana", "guest"]);
        assert_eq!(u[0].real_name, "Ana Lima");
    }

    #[test]
    fn initials_and_names() {
        let ana = User::new("ana", "Ana Lima", 1000);
        assert_eq!(ana.initials(), "AL");
        assert_eq!(ana.display_name(), "Ana Lima");
        let g = User::new("guest", "", 1002);
        assert_eq!(g.initials(), "G");
        assert_eq!(g.display_name(), "guest");
        assert_eq!(User::new("x", "Ana Maria Lima", 1).initials(), "AM");
    }
}

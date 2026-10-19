use anyhow::bail;

use crate::users::DisplayName;

const ISSUE_SESSION: &str = "issue-session";
const DISPLAY_NAME_FLAG: &str = "--display-name";
const USAGE: &str = "usage: api [issue-session --display-name <name>]";

pub enum Command {
    Serve,
    /// Human-only, on the VPS (plan step 17): prints a new session's cookie value.
    IssueSession(DisplayName),
}

impl Command {
    /// # Errors
    /// Fails on anything but no arguments or `issue-session --display-name <name>`.
    pub fn parse(arguments: impl IntoIterator<Item = String>) -> anyhow::Result<Self> {
        let arguments: Vec<String> = arguments.into_iter().collect();
        match arguments.as_slice() {
            [] => Ok(Self::Serve),
            [command, flag, name] if command == ISSUE_SESSION && flag == DISPLAY_NAME_FLAG => {
                Ok(Self::IssueSession(DisplayName::try_from(name.clone())?))
            }
            _ => bail!(USAGE),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(arguments: &[&str]) -> anyhow::Result<Command> {
        Command::parse(arguments.iter().map(|argument| (*argument).to_owned()))
    }

    #[test]
    fn no_arguments_serves() {
        assert!(matches!(parse(&[]), Ok(Command::Serve)));
    }

    #[test]
    fn parses_issue_session_with_a_display_name() {
        assert!(matches!(
            parse(&["issue-session", "--display-name", "Maintainer"]),
            Ok(Command::IssueSession(_))
        ));
    }

    #[test]
    fn rejects_anything_else() {
        for arguments in [
            &["serve"][..],
            &["issue-session"],
            &["issue-session", "--display-name"],
            &["issue-session", "--name", "Maintainer"],
            &["issue-session", "--display-name", " "],
            &["issue-session", "--display-name", "Maintainer", "extra"],
        ] {
            assert!(parse(arguments).is_err(), "{arguments:?}");
        }
    }
}

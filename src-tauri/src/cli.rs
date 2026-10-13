use crate::window::Trigger;

pub const HELP: &str = "\
Usage: keymint [OPTION]

  --toggle        Show the palette, or hide it if it is open
  --show          Show the palette
  --run <ID>      Show the palette and run a command, e.g. ai.fix-grammar
  --settings      Open settings

Without options Keymint starts in the system tray.";

/// Parse arguments (without the program name). Unknown arguments are ignored
/// so a second launch with odd flags never breaks the running instance.
pub fn parse(args: &[String]) -> Option<Trigger> {
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--toggle" => return Some(Trigger::Toggle),
            "--show" => return Some(Trigger::Show),
            "--settings" => return Some(Trigger::Settings),
            "--run" => return it.next().map(|id| Trigger::Run(id.clone())),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str]) -> Option<Trigger> {
        parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn parses_flags() {
        assert_eq!(p(&["--toggle"]), Some(Trigger::Toggle));
        assert_eq!(p(&["--run", "ai.summarize"]), Some(Trigger::Run("ai.summarize".into())));
        assert_eq!(p(&["--run"]), None);
        assert_eq!(p(&["--weird", "--settings"]), Some(Trigger::Settings));
        assert_eq!(p(&[]), None);
    }
}

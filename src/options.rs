use std::{net::SocketAddr, path::PathBuf};
#[derive(Default)]
pub struct Options {
    pub controller: crate::controller::Controller,
    pub mock: bool,
    pub diagnose: bool,
    pub visible: bool,
    pub led_test: bool,
    pub test: bool,
    pub help: bool,
    pub g1: Option<SocketAddr>,
    pub patch_dir: Option<PathBuf>,
    pub state_file: Option<PathBuf>,
}
impl Options {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut result = Self::default();
        let mut args = args.into_iter();
        let mut no_persist = false;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--controller" => result.controller = match args.next().as_deref() {
                    Some("maschine") => crate::controller::Controller::Maschine,
                    Some("komplete") => crate::controller::Controller::Komplete,
                    _ => return Err("--controller needs maschine or komplete (S49 Mk2)".into()),
                },
                "--g1" => result.g1 = Some("127.0.0.1:47410".parse().unwrap()),
                "--patch-dir" => {
                    let path = args
                        .next()
                        .filter(|p| !p.starts_with("--"))
                        .ok_or("--patch-dir needs a folder")?;
                    let path = std::fs::canonicalize(path)
                        .map_err(|e| format!("Invalid patch folder: {e}"))?;
                    if !path.is_dir() {
                        return Err("--patch-dir must be a folder".into());
                    }
                    result.patch_dir = Some(path);
                }
                "--g1-address" => {
                    let address = args
                        .next()
                        .ok_or("--g1-address needs loopback IP:port")?
                        .parse::<SocketAddr>()
                        .map_err(|_| "Invalid G1 address")?;
                    if !address.ip().is_loopback() || address.port() < 1024 {
                        return Err("G1 bridge must use a loopback IP and port >=1024".into());
                    }
                    result.g1 = Some(address);
                }
                "--mock" => result.mock = true,
                "--diagnose" => result.diagnose = true,
                "--visible" => result.visible = true,
                "--led-test" => result.led_test = true,
                "--hardware-test" => result.test = true,
                "--help" | "-h" => result.help = true,
                "--no-persist" => no_persist = true,
                "--state-file" => {
                    let path = args
                        .next()
                        .filter(|p| !p.starts_with("--"))
                        .ok_or("--state-file needs a .json path")?;
                    result.state_file = Some(path.into());
                }
                _ => return Err(format!("Unknown argument {arg}; use --help")),
            }
        }
        if result.mock && (result.diagnose || result.led_test) {
            return Err("--mock cannot combine with --diagnose or --led-test".into());
        }
        if result.patch_dir.is_some() && result.g1.is_none() {
            return Err("--patch-dir requires --g1 or --g1-address".into());
        }
        if result.diagnose && result.test {
            return Err("--diagnose cannot combine with --hardware-test".into());
        }
        if result.g1.is_some() && (result.test || result.diagnose || result.state_file.is_some()) {
            return Err(
                "--g1 cannot combine with diagnostics, hardware-test or mock fixture persistence"
                    .into(),
            );
        }
        if no_persist && result.state_file.is_some() {
            return Err("--state-file cannot combine with --no-persist".into());
        }
        if no_persist || result.test || result.diagnose {
            result.state_file = None;
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(args: &[&str]) -> Result<Options, String> {
        Options::parse(args.iter().map(|arg| arg.to_string()))
    }
    #[test]
    fn controller_selection_is_explicit_and_model_bounded() {
        assert_eq!(parse(&[]).unwrap().controller, crate::controller::Controller::Komplete);
        assert_eq!(parse(&["--controller","komplete"]).unwrap().controller, crate::controller::Controller::Komplete);
        assert!(parse(&["--controller","mk3-keyboard"]).is_err());
        assert!(parse(&["--controller"]).is_err());
    }
    #[test]
    fn real_g1_is_loopback_only_and_cannot_restore_mock_projects() {
        assert!(parse(&["--mock", "--g1"]).unwrap().g1.is_some());
        assert_eq!(
            parse(&["--g1-address", "127.0.0.1:47411"])
                .unwrap()
                .g1
                .unwrap()
                .port(),
            47411
        );
        for args in [
            &["--g1-address", "192.0.2.1:47410"][..],
            &["--g1", "--hardware-test"],
            &["--g1", "--state-file", "fixture.json"],
        ] {
            assert!(parse(args).is_err());
        }
    }
    #[test]
    fn persistence_is_explicitly_disabled_for_tests_and_flags_are_checked() {
        assert!(parse(&["--mock", "--no-persist"])
            .unwrap()
            .state_file
            .is_none());
        assert!(parse(&["--hardware-test"]).unwrap().state_file.is_none());
        assert_eq!(
            parse(&["--state-file", "my project.json"])
                .unwrap()
                .state_file
                .unwrap(),
            PathBuf::from("my project.json")
        );
        for args in [
            &["--editor"][..],
            &["--no-persist", "--state-file", "a.json"],
            &["--state-file"],
            &["--mock", "--diagnose"],
        ] {
            assert!(parse(args).is_err());
        }
    }
}

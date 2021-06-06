use crate::{Config, Coordinates, VersionCheck};
use clap::{
    App,
    AppSettings::{
        AllowNegativeNumbers, ArgRequiredElseHelp, ColoredHelp, DeriveDisplayOrder,
        UnifiedHelpMessage,
    },
    Arg,
};
use semver::VersionReq;
use std::{fmt::Display, str::FromStr};

#[derive(Debug)]
#[cfg_attr(test, derive(Default))]
pub(crate) struct Opts {
    version_checks: Vec<VersionCheck>,
    include_pre_releases: bool,
}

impl Opts {
    pub(crate) fn new() -> Self {
        Self::from_matches(Self::app().get_matches())
    }

    #[cfg(test)]
    fn of(args: &[&str]) -> Result<Self, clap::Error> {
        let args = args.to_vec();
        let matches = Self::app()
            .setting(clap::AppSettings::NoBinaryName)
            .try_get_matches_from(args)?;
        Ok(Self::from_matches(matches))
    }

    pub(crate) fn config(&self) -> Config {
        Config {
            include_pre_releases: self.include_pre_releases,
        }
    }

    pub(crate) fn into_version_checks(self) -> Vec<VersionCheck> {
        self.version_checks
    }

    fn app() -> App<'static> {
        App::new(env!("CARGO_BIN_NAME"))
            .about(env!("CARGO_PKG_DESCRIPTION"))
            .version(env!("CARGO_PKG_VERSION"))
            .setting(AllowNegativeNumbers)
            .setting(ArgRequiredElseHelp)
            .setting(ColoredHelp)
            .setting(DeriveDisplayOrder)
            .setting(UnifiedHelpMessage)
            .arg(
                Arg::new("include-pre-releases")
                    .about("Also consider pre releases")
                    .short('i')
                    .long("include-pre-releases"),
            ).arg(
                Arg::new("version-checks")
                    .takes_value(true)
                    .multiple(true)
                    .min_values(1)
                    .validator(parse_coordinates)
                    .about("The maven coordinates to check for. Can be specified multiple times")
                    .long_about(r#"
The maven coordinates to check for. Can be specified multiple times.

These arguments take the form of `{groupId}:{artifactId}[:{version}]*`.
The versions are treated as requirement qualifiers.
Every matching version will be collected into the same bucket per requirement.
The latest version per bucket is then shown.
The value for a requirement follow the semver range specification from https://www.npmjs.com/package/semver#advanced-range-syntax

Multiple checks will be run concurrently and may be printed out of order."#)
                    )
    }

    fn from_matches(matches: clap::ArgMatches) -> Self {
        Opts {
            version_checks: matches
                .values_of("version-checks")
                .map(|v| v.map(|s| parse_coordinates(s).unwrap()).collect())
                .unwrap_or_else(Vec::new),
            include_pre_releases: matches.is_present("include-pre-releases"),
        }
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub(crate) enum Error {
    Missing(&'static str, String),
    InvalidRange(String, semver::Error),
}

impl FromStr for VersionCheck {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_coordinates(s)
    }
}

fn parse_coordinates(input: &str) -> Result<VersionCheck, Error> {
    let mut segments = input.split(':').map(str::trim).peekable();

    let coordinates = match segments.next() {
        Some("maven") => {
            let group_id = match segments.next() {
                Some(group_id) if !group_id.is_empty() => group_id,
                _ => return Err(Error::Missing("group_id", input.into())),
            };

            let artifact_id = match segments.next() {
                Some(artifact_id) if !artifact_id.is_empty() => artifact_id,
                _ => return Err(Error::Missing("artifact_id", input.into())),
            };

            Coordinates::Maven {
                group_id: group_id.into(),
                artifact_id: artifact_id.into(),
            }
        }
        Some("cargo") => {
            let package = match segments.next() {
                Some(package) if !package.is_empty() => package,
                Some(_) => match segments.next() {
                    Some(package) if !package.is_empty() => package,
                    _ => return Err(Error::Missing("package", input.into())),
                },
                _ => return Err(Error::Missing("package", input.into())),
            };

            Coordinates::Cargo(package.into())
        }
        Some("npm") => match segments.next() {
            Some(scope_or_package) if !scope_or_package.is_empty() => {
                if scope_or_package.starts_with('@') {
                    let scope = scope_or_package.trim_start_matches('@');
                    if scope.is_empty() {
                        return Err(Error::Missing("scope", input.into()));
                    }
                    if let Some((scope, package)) = scope.split_once('/') {
                        if package.is_empty() {
                            return Err(Error::Missing("package", input.into()));
                        }
                        Coordinates::Npm {
                            scope: Some(scope.into()),
                            package: package.into(),
                        }
                    } else {
                        let package = match segments.next() {
                            Some(package) if !package.is_empty() => package,
                            _ => return Err(Error::Missing("package", input.into())),
                        };
                        Coordinates::Npm {
                            scope: Some(scope.into()),
                            package: package.into(),
                        }
                    }
                } else {
                    if let Some((scope, package)) = scope_or_package.split_once('/') {
                        if scope.is_empty() {
                            return Err(Error::Missing("scope", input.into()));
                        }
                        if package.is_empty() {
                            return Err(Error::Missing("package", input.into()));
                        }
                        Coordinates::Npm {
                            scope: Some(scope.into()),
                            package: package.into(),
                        }
                    } else {
                        match segments.peek() {
                            Some(package)
                                if !package.is_empty() && VersionReq::parse(package).is_err() =>
                            {
                                let coords = Coordinates::Npm {
                                    scope: Some(scope_or_package.into()),
                                    package: (*package).into(),
                                };
                                let _ = segments.next();
                                coords
                            }
                            _ => Coordinates::Npm {
                                scope: None,
                                package: scope_or_package.into(),
                            },
                        }
                    }
                }
            }
            _ => return Err(Error::Missing("package", input.into())),
        },
        Some("go") => match segments.next() {
            Some(gomod) if !gomod.is_empty() && gomod.contains('/') => {
                Coordinates::AnyGo(gomod.into())
            }
            Some(user) if !user.is_empty() => match segments.next() {
                Some(gomod) if !gomod.is_empty() => Coordinates::Go {
                    user: user.into(),
                    module: gomod.into(),
                },
                _ => return Err(Error::Missing("module", input.into())),
            },
            _ => return Err(Error::Missing("user", input.into())),
        },
        Some("github.com") => {
            let user = match segments.next() {
                Some(user) if !user.is_empty() => user,
                _ => return Err(Error::Missing("user", input.into())),
            };

            let module = match segments.next() {
                Some(module) if !module.is_empty() => module,
                _ => return Err(Error::Missing("module", input.into())),
            };

            Coordinates::Go {
                user: user.into(),
                module: module.into(),
            }
        }
        Some(group_id) if !group_id.is_empty() => {
            let artifact_id = match segments.next() {
                Some(artifact_id) if !artifact_id.is_empty() => artifact_id,
                _ => return Err(Error::Missing("artifact_id", input.into())),
            };

            Coordinates::Maven {
                group_id: group_id.into(),
                artifact_id: artifact_id.into(),
            }
        }
        _ => return Err(Error::Missing("group_id", input.into())),
    };

    let versions = segments.map(parse_version).collect::<Result<Vec<_>, _>>()?;

    Ok(VersionCheck {
        coordinates,
        versions,
    })
}

fn parse_version(version: &str) -> Result<VersionReq, Error> {
    if version.is_empty() {
        Ok(VersionReq::STAR)
    } else {
        VersionReq::parse(version).map_err(|e| Error::InvalidRange(version.into(), e))
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Missing(what, input) => write!(f, "Missing {} in {}", what, input),
            Error::InvalidRange(input, _) => write!(
                f,
                "Could not parse {} into a semantic version range. Please provide a valid range according to {}",
                console::style(input).red().bold(),
                console::style("https://www.npmjs.com/package/semver#advanced-range-syntax").cyan().underlined(),
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Error::InvalidRange(_, src) = self {
            Some(src)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::ErrorKind;
    use test_case::test_case;

    #[test]
    fn empty_args_shows_help() {
        let err = Opts::of(&[]).unwrap_err();
        assert_eq!(err.kind, ErrorKind::MissingArgumentOrSubcommand);
    }

    #[test]
    fn test_empty_version_arg() {
        let err = Opts::of(&[""]).unwrap_err();
        assert_eq!(err.kind, ErrorKind::EmptyValue);
        assert_eq!(err.info, vec![String::from("<version-checks>...")]);
    }

    #[test_case("foo:bar", "foo", "bar"; "case1")]
    #[test_case("foo.bar:baz", "foo.bar", "baz"; "case2")]
    #[test_case("foo:bar.baz", "foo", "bar.baz"; "case3")]
    #[test_case("foo.bar:baz.qux", "foo.bar", "baz.qux"; "case4")]
    #[test_case("42:1337", "42", "1337"; "case5")]
    #[test_case(" 42 :  1337  ", "42", "1337"; "case6")]
    fn test_version_arg_coords(arg: &str, expected_group_id: &str, expected_artifact_id: &str) {
        let opts = Opts::of(&[arg]).unwrap();
        let mut checks = opts.version_checks.into_iter();
        let check = checks.next().unwrap();

        match check.coordinates {
            Coordinates::Maven {
                group_id,
                artifact_id,
            } => {
                assert_eq!(group_id, expected_group_id);
                assert_eq!(artifact_id, expected_artifact_id);
            }
            wrong => assert!(false, "{:?}", wrong),
        }
        assert_eq!(checks.next(), None);
    }

    #[test_case(":foo", Error::Missing("group_id", ":foo".into()); "empty_group_id_1")]
    #[test_case(":foo:", Error::Missing("group_id", ":foo:".into()); "empty_group_id_2")]
    #[test_case("", Error::Missing("group_id", "".into()); "empty_group_id_3")]
    #[test_case(":", Error::Missing("group_id", ":".into()); "empty_group_id_4")]
    #[test_case("::", Error::Missing("group_id", "::".into()); "empty_group_id_5")]
    #[test_case("  ", Error::Missing("group_id", "  ".into()); "empty_group_id_6")]
    #[test_case("  :", Error::Missing("group_id", "  :".into()); "empty_group_id_7")]
    #[test_case("foo:", Error::Missing("artifact_id", "foo:".into()); "empty_artifact_1")]
    #[test_case("foo::", Error::Missing("artifact_id", "foo::".into()); "empty_artifact_2")]
    #[test_case("foo: ", Error::Missing("artifact_id", "foo: ".into()); "empty_artifact_3")]
    #[test_case("foo: :", Error::Missing("artifact_id", "foo: :".into()); "empty_artifact_4")]
    #[test_case("foo", Error::Missing("artifact_id", "foo".into()); "missing_artifact")]
    fn test_invalid_coords(arg: &str, expected: Error) {
        match (parse_coordinates(arg).unwrap_err(), expected) {
            (Error::Missing(lhs1, lhs2), Error::Missing(rhs1, rhs2)) => {
                assert_eq!(lhs1, rhs1);
                assert_eq!(lhs2, rhs2);
            }
            (lhs, rhs) => panic!("Different errors: left == {} right == {}", lhs, rhs),
        }
    }

    #[test_case(":foo", "Missing group_id in :foo"; "empty_group_id_1")]
    #[test_case(":foo:", "Missing group_id in :foo:"; "empty_group_id_2")]
    #[test_case(":", "Missing group_id in :"; "empty_group_id_4")]
    #[test_case("::", "Missing group_id in ::"; "empty_group_id_5")]
    #[test_case("  ", "Missing group_id in   "; "empty_group_id_6")]
    #[test_case("  :", "Missing group_id in   :"; "empty_group_id_7")]
    #[test_case("foo:", "Missing artifact_id in foo:"; "empty_artifact_1")]
    #[test_case("foo::", "Missing artifact_id in foo::"; "empty_artifact_2")]
    #[test_case("foo: ", "Missing artifact_id in foo: "; "empty_artifact_3")]
    #[test_case("foo: :", "Missing artifact_id in foo: :"; "empty_artifact_4")]
    #[test_case("foo", "Missing artifact_id in foo"; "missing_artifact")]
    fn test_version_arg_invalid_coords(arg: &str, msg: &str) {
        console::set_colors_enabled(false);
        let err = Opts::of(&[arg]).unwrap_err();
        assert_eq!(err.kind, ErrorKind::ValueValidation);
        assert_eq!(
            err.to_string(),
            format!("error: Invalid value for '<version-checks>...': {}\n\nFor more information try --help\n", msg)
        );
    }

    #[test_case("foo:bar:1", vec!["1"]; "version 1")]
    #[test_case("foo:bar:0", vec!["0"]; "version 0")]
    #[test_case("foo:bar:*", vec!["*"]; "any version")]
    #[test_case("foo:bar:", vec!["*"]; "empty version")]
    #[test_case("foo:bar", vec![]; "no version")]
    #[test_case("foo:bar:1.0", vec!["1.0"]; "version 1.0")]
    #[test_case("foo:bar:1.x", vec!["1.x"]; "version 1.x")]
    #[test_case("foo:bar:1.*", vec!["1.*"]; "version 1.*")]
    #[test_case("foo:bar:=1.2.3", vec!["=1.2.3"]; "exact version")]
    #[test_case("foo:bar:<1.2.3", vec!["<1.2.3"]; "lt version")]
    #[test_case("foo:bar:>1.2.3", vec![">1.2.3"]; "gt version")]
    #[test_case("foo:bar:<=1.2.3", vec!["<=1.2.3"]; "lte version")]
    #[test_case("foo:bar:>=1.2.3", vec![">=1.2.3"]; "gte version")]
    #[test_case("foo:bar:1.2.3, 2", vec!["1.2.3, 2"]; "multi range with space")]
    #[test_case("foo:bar:1.2.3,2", vec!["1.2.3,2"]; "multi range with comma")]
    #[test_case("foo:bar:1.2.3:2", vec!["1.2.3", "2"]; "multiple ranges")]
    fn test_version_arg_range(arg: &str, ranges: Vec<&str>) {
        let ranges = ranges
            .into_iter()
            .map(VersionReq::parse)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let opts = Opts::of(&[arg]).unwrap();
        let mut checks = opts.version_checks.into_iter();
        let check = checks.next().unwrap();
        assert_eq!(check.versions, ranges);
        assert_eq!(checks.next(), None);
    }

    #[test_case("foo:bar:01", "01"; "major with leading 0")]
    #[test_case("foo:bar:1.02", "1.02"; "minor with leading 0")]
    #[test_case("foo:bar:.", "."; "missing major")]
    #[test_case("foo:bar:1.", "1."; "trailing period before minor")]
    #[test_case("foo:bar:1..", "1.."; "two trailing periods")]
    #[test_case("foo:bar:1.2.", "1.2."; "trailing period before path")]
    #[test_case("foo:bar:qux", "qux"; "non numeric major")]
    #[test_case("foo:bar:1.qux", "1.qux"; "non numeric minor")]
    #[test_case("foo:bar:-42", "-42"; "negative major")]
    #[test_case("foo:bar:*42", "*42"; "mixed star and version")]
    #[test_case("foo:bar:1.3.3.7", "1.3.3.7"; "4 segments")]
    #[test_case("foo:bar:1:foo", "foo"; "second version fails")]
    #[test_case("foo:bar:1.2.3 2", "1.2.3 2"; "multi range with space separator")]
    fn test_version_arg_invalid_range(arg: &str, spec: &str) {
        console::set_colors_enabled(false);
        let err = Opts::of(&[arg]).unwrap_err();
        assert_eq!(err.kind, ErrorKind::ValueValidation);
        assert_eq!(
            err.to_string(),
            format!("error: Invalid value for '<version-checks>...': Could not parse {} into a semantic version range. Please provide a valid range according to https://www.npmjs.com/package/semver#advanced-range-syntax\n\nFor more information try --help\n", spec)
        );
    }

    #[test]
    fn test_default_pre_release_flag() {
        let opts = Opts::default();
        assert_eq!(opts.include_pre_releases, false);
        assert_eq!(opts.config().include_pre_releases, false);
    }

    #[test_case("-i"; "short flag")]
    #[test_case("--include-pre-releases"; "long flag")]
    fn test_pre_release_flag(flag: &str) {
        let opts = Opts::of(&[flag]).unwrap();
        assert_eq!(opts.include_pre_releases, true);
        assert_eq!(opts.config().include_pre_releases, true);
    }
}

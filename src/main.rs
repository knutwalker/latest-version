/*!
Check deps.dev for the latest version of artifacts.

# Building

## Prerequisites

This tool is build with Rust so you need to have a rust toolchain and cargo installed.
If you don't, please visit [https://rustup.rs/](https://rustup.rs/) and follow their instructions.

## Building

The preferred way is to run:

```
make install
```
If you do not have a fairly recent make (on macOS, homebrew can install a newer version),
or don't want to use make, you can also run `cargo install --path .`.

# Usage

Run `latest-version --help` for an overview of all available options.

The main usage is by providing maven coordinates in the form of `groupId:artifact`, followed by multiple `:version` qualifiers.
These version qualifier are [Semantic Version Ranges](https://www.npmjs.com/package/semver#advanced-range-syntax).
For each of the provided versions, the latest available version on maven central is printed.

### Default version

The version ranges can be left out, in which case the latest overall version is printed.

### Multiple Version ranges

You can also enter multiple coordinates, each with their own versions to check against.
The result are printed as they arrive, so they might be out of order.

### Pre Release Versions

Pre-releases can be included with the `--include-pre-releases` flag (or `-i` for short).

### Version overrides

The versions are matched in order and a single version can only be matched by one qualifier.
Previous matches will – depending on the range – consume all versions that would have also been matched by later qualifiers.
Try to define the qualifiers in the order from most restrictive to least.

# Examples

Matching against minor-compatible releases.

    $ latest-version org.neo4j.gds:proc:~1.1:~1.3:1
    Latest version for maven:org.neo4j.gds:proc matching >=1.1.0, <1.2.0: 1.1.6
    Latest version for maven:org.neo4j.gds:proc matching >=1.3.0, <1.4.0: 1.3.5
    Latest version for maven:org.neo4j.gds:proc matching >=1.0.0, <2.0.0: 1.6.0


Matching against major compatible releases. Note that `1.3` does not produce any match, as it is already covered by `1.1`.

    $ latest-version org.neo4j.gds:proc:^1.1:^1.3:^1
    Latest version for maven:org.neo4j.gds:proc matching >=1.1.0, <2.0.0: 1.6.0
    No version for maven:org.neo4j.gds:proc matching >=1.3.0, <2.0.0
    Latest version for maven:org.neo4j.gds:proc matching >=1.0.0, <2.0.0: 1.0.0


Inclusion of pre releases.

    $ latest-version org.neo4j.gds:proc:~1.1:~1.3:1 --include-pre-releases
    Latest version(s) for org.neo4j.gds:proc:
    Latest version for maven:org.neo4j.gds:proc matching >=1.1.0, <1.2.0: 1.1.6
    Latest version for maven:org.neo4j.gds:proc matching >=1.3.0, <1.4.0: 1.3.5
    Latest version for maven:org.neo4j.gds:proc matching >=1.0.0, <2.0.0: 1.4.0-alpha02


Default version.

    $ latest-version org.neo4j.gds:proc
    Latest version for maven:org.neo4j.gds:proc matching >=0.0.0: 1.6.0

    $ latest-version org.neo4j.gds:proc --include-pre-releases
    Latest version for maven:org.neo4j.gds:proc matching *: 1.4.0-alpha02


Multiple checks.

    $ latest-version org.neo4j.gds:proc org.neo4j:neo4j
    Latest version for maven:org.neo4j.gds:proc matching >=0.0.0: 1.6.0
    Latest version for maven:org.neo4j:neo4j matching >=0.0.0: 4.2.6

*/

use color_eyre::eyre::Result;
use console::style;
use reqwest::{Client, Url};
use semver::VersionReq;
use serde_json::Value;
use std::{borrow::Cow, fmt::Write, sync::Arc, time::Duration};
use tokio::io::{self, AsyncWriteExt};
use versions::Versions;

mod opts;
mod versions;

fn main() -> Result<()> {
    if console::colors_enabled() {
        color_eyre::config::HookBuilder::default()
            .display_env_section(false)
            .install()?
    }

    let opts = opts::Opts::new();
    let config = opts.config();
    let checks = opts.into_version_checks();

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move { run(config, checks).await })
}

async fn run(config: Config, checks: Vec<VersionCheck>) -> Result<()> {
    static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

    let client = Client::builder()
        .user_agent(APP_USER_AGENT)
        .gzip(true)
        .timeout(Duration::from_secs(10))
        .tcp_keepalive(Some(Duration::from_secs(60)))
        .use_rustls_tls()
        .build()?;
    let client = Arc::new(client);

    let tasks = checks
        .into_iter()
        .map({
            |check| {
                let client = Arc::clone(&client);
                tokio::spawn(async move { run_check_and_report(client, config, check).await })
            }
        })
        .collect::<Vec<_>>();

    for task in tasks {
        task.await??;
    }
    Ok(())
}

async fn run_check_and_report(
    client: Arc<Client>,
    config: Config,
    check: VersionCheck,
) -> Result<()> {
    let url = check_url(&check);
    let versions = query_versions(&*client, url.clone()).await?;
    let versions = versions.latest_versions(config.include_pre_releases, check.versions);

    let mut stdout = io::stdout();
    let mut msg = String::with_capacity(64);
    let coordinates = check.coordinates;
    let pkg = coordinates.package_slug();

    for (req, latest) in versions {
        msg.clear();
        let _ = match latest {
            Some(latest) => writeln!(
                msg,
                "Latest version for {}:{} matching {}: {}",
                style(coordinates.system_slug()).magenta(),
                style(&pkg).blue(),
                style(req).cyan().bold(),
                style(latest).green().bold()
            ),
            None => writeln!(
                msg,
                "No version for {}:{} matching {}",
                style(coordinates.system_slug()).magenta(),
                style(&pkg).blue(),
                style(req).yellow().bold()
            ),
        };

        stdout.write_all(msg.as_bytes()).await?;
    }

    Ok(())
}

fn check_url(check: &VersionCheck) -> Url {
    let mut url = Url::parse("https://deps.dev/_/s").expect("this is a valid url");
    url.path_segments_mut()
        .expect("url can be a base")
        .extend(std::array::IntoIter::new([
            check.coordinates.system_slug(),
            "p",
            &check.coordinates.package_slug(),
            "versions",
        ]));
    url
}

async fn query_versions(client: &Client, url: Url) -> Result<Versions> {
    let response = client.get(url).send().await?.json::<Value>().await?;
    let versions = response
        .get("versions")
        .and_then(|v| v.as_array())
        .map_or_else(
            || Versions::default(),
            |v| {
                v.iter()
                    .filter_map(|v| v.get("version"))
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect()
            },
        );
    Ok(versions)
}

#[derive(Debug, Clone, Copy)]
struct Config {
    include_pre_releases: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct VersionCheck {
    coordinates: Coordinates,
    versions: Vec<VersionReq>,
}

#[derive(Debug, Clone, PartialEq)]
enum Coordinates {
    Maven {
        group_id: String,
        artifact_id: String,
    },
    Cargo(String),
    Npm {
        scope: Option<String>,
        package: String,
    },
    Go {
        user: String,
        module: String,
    },
    AnyGo(String),
}

impl Coordinates {
    fn system_slug(&self) -> &str {
        match self {
            Coordinates::Maven { .. } => "maven",
            Coordinates::Cargo(_) => "cargo",
            Coordinates::Npm { .. } => "npm",
            Coordinates::Go { .. } | Coordinates::AnyGo(_) => "go",
        }
    }

    fn package_slug(&self) -> Cow<str> {
        match self {
            Coordinates::Maven {
                group_id,
                artifact_id,
            } => Cow::Owned(format!("{}:{}", group_id, artifact_id)),
            Coordinates::Cargo(package) => Cow::Borrowed(package),
            Coordinates::Npm {
                scope: Some(scope),
                package,
            } => Cow::Owned(format!("@{}/{}", scope, package)),
            Coordinates::Npm {
                scope: None,
                package,
            } => Cow::Borrowed(package),
            Coordinates::Go { user, module } => {
                Cow::Owned(format!("github.com/{}/{}", user, module))
            }
            Coordinates::AnyGo(go) => Cow::Borrowed(go),
        }
    }
}

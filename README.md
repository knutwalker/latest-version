<h1 align="center">latest-version</h1>

<p align="center">
  <a href="https://github.com/knutwalker/latest-version/actions">
    <img src="https://img.shields.io/github/workflow/status/knutwalker/latest-version/checks/main?label=workflow&style=for-the-badge"
         alt="GitHub Actions workflow status" />
  </a>
  
  <a href="https://crates.io/crates/latest-version">
    <img src="https://img.shields.io/crates/v/latest-version?style=for-the-badge"
         alt="Latest version on crates.io" />
  </a>
  <a href="https://github.com/knutwalker/latest-version/releases">
    <img src="https://img.shields.io/github/v/release/knutwalker/latest-version?sort=semver&style=for-the-badge"
         alt="Latest release on Github" />
  </a>
  <a href="https://choosealicense.com/licenses/mit/">
    <img src="https://img.shields.io/crates/l/latest-version?style=for-the-badge"
         alt="License: MIT/Apache-2.0" />
  </a>

  <br />

  <a href="https://github.com/knutwalker/latest-version/search?l=rust&type=code">
    <img src="https://img.shields.io/github/languages/top/knutwalker/latest-version?color=orange&label=awesome&style=for-the-badge"
         alt="GitHub most-used language" />
  </a>
  <a href="https://github.com/knutwalker/latest-version/search?type=code">
    <img src="https://img.shields.io/tokei/lines/github/knutwalker/latest-version?label=power%20level&style=for-the-badge"
         alt="Total number of source code lines" />
  </a>
</p>

<p align="center"><em>Check deps.dev for the latest version of any artifact</em></p>

# Installation

Pre-build binaries for the main architectures can be pulled from [Github releases](https://github.com/knutwalker/latest-version/releases).

This project is published to crates.io, if you have a rust toolchain installed, you can also install via cargo:

```
cargo install latest-version
```

Alternatively, you can build from source.

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

The main usage is by providing artifact coordinates in the form of `[system:]groupId:artifact`, followed by multiple `:version` qualifiers.
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


# Artifact Coordinates

The default specifier searches for maven packages and uses the `groupId:artifactId` scheme.
These specifiers can be prefixed with one of the available systems on deps.dev.
At the time this is:

 * `maven`
 * `cargo`
 * `npm`
 * `go`

The presence of a system selector changes the way that the remaining specifier is understood.

## Maven packages

`[maven:]$groupId:$artifactId`

Maven packages require two components, the `groupId` and the `artifactId`.
This is also the default system.

The following calls are identical

    $ latest-version org.neo4j:neo4j maven:org.neo4j:neo4j
    Latest version for maven:org.neo4j:neo4j matching >=0.0.0: 4.2.6
    Latest version for maven:org.neo4j:neo4j matching >=0.0.0: 4.2.6

The explicit `maven:` can be used to search for artifacts that contain a group id that is also a system identifier, such as
[`cargo:cargo`](https://search.maven.org/artifact/cargo/cargo/0.6/jar).

Using `cargo:cargo` will search the `cargo` system for a `cargo` crate.
Using `maven:cargo:cargo` will search the `maven` system for a `cargo` groupId and a `cargo` artifactId.

## Cargo crates

`cargo:$crate`

Cargo crates require an explicit `cargo:` system identifier, followed by a **single** crate name.

    $ latest-version cargo:lenient_semver
    Latest version for cargo:lenient_semver matching >=0.0.0: 0.3.0

## NPM packages

`npm:[$scope:]$package`

NPM packages require an explicit `npm:` system identifier, followed by an optional scope, followed by the package.

In the easiest form, npm requires only a single package specifier

    $ latest-version npm:neo4j-driver
    Latest version for npm:neo4j-driver matching >=0.0.0: 4.3.0

In order to search for scoped packages, the scope needs be before the package name.
The scope can be separated via `/` or `:` and the leading `@` is optional.
All of these are identical

    $ latest-version npm:@types/neo4j npm:types/neo4j npm:@types:neo4j npm:types:neo4j
    Latest version for npm:@types/neo4j matching >=0.0.0: 2.0.2
    Latest version for npm:@types/neo4j matching >=0.0.0: 2.0.2
    Latest version for npm:@types/neo4j matching >=0.0.0: 2.0.2
    Latest version for npm:@types/neo4j matching >=0.0.0: 2.0.2


Searching for scoped packages where the package could be parsed as a version requirement,
such as [`@euler/1`](https://www.npmjs.com/package/@euler/1/v/0.0.5)
requires the usage of either `@` or `/` to disambiguate from just searching for `euler` and the version requirement `1`.

    $ latest-version npm:euler:1 npm:@euler:1 npm:euler/1 npm:@euler/1
    No version for npm:euler matching >=1.0.0, <2.0.0   #  <- `npm:euler:1` -> unscoped `euler`, version 1
    Latest version for npm:@euler/1 matching >=0.0.0: 0.0.5  # using `@` to disambiguate
    Latest version for npm:@euler/1 matching >=0.0.0: 0.0.5  # using `/`  disambiguate
    Latest version for npm:@euler/1 matching >=0.0.0: 0.0.5  # using `@` and `/` to disambiguate

## Go modules

`go:$user:$module`

Go modules require an explicit `go:` system identifier, followed by two more identifiers.
Those will be searched as the full module path `github.com/$user/$module`.
To align the syntax with the `:` as separator, `github.com` can also be used instead of `go` to specify a module from the github.com repository.
To use a different repository, you can specify the full path as a single argument.

The following are identical

    $ latest-version go:neo4j:neo4j-go-driver go:github.com/neo4j/neo4j-go-driver github.com:neo4j:neo4j-go-driver
    Latest version for go:github.com/neo4j/neo4j-go-driver matching >=0.0.0: v1.8.3
    Latest version for go:github.com/neo4j/neo4j-go-driver matching >=0.0.0: v1.8.3
    Latest version for go:github.com/neo4j/neo4j-go-driver matching >=0.0.0: v1.8.3

Note that using a single identifier with `/` does require a repository:

    $ latest-version go:neo4j/neo4j-go-driver
    No version for go:neo4j/neo4j-go-driver matching >=0.0.0

# About the data

`latest-version` uses [Open Source Insights (deps.dev)](https://deps.dev/about) to provide the data.
Package information is usually up to date within the hour.

See [What packages does Insights cover?](https://deps.dev/faq#what-packages-does-insights-cover)
and [How fresh is the information?](https://deps.dev/faq#how-fresh-is-the-information)
for more information.

License: MIT OR Apache-2.0

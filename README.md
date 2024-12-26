# fsidx

[![MIT licensed][mit-badge]][mit-url]

[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/tokio-rs/tokio/blob/master/LICENSE

The **`fsidx`** program provides fast pathname searching using database files. *fsidx* is the appriviation for 'File System Index'.

## Overview

Main features are:

- Creating database files with file names and sizes
- Searching file names in the database files
- Searching unmounted file systems
- Own shell with history and completions
- Opening query results with default applications
- Glob patterns
- Plain text search
- Case-sensitive and case-insensitive search
- Same order or any order search
- Whole path or last element search
- Smart space matching
- Word boundary matching
- TOML configuration file

The [fsidx(1)] man page contains a detailed description of all features.

[fsidx(1)]:https://github.com/jerbs/fsidx/blob/master/doc/fsidx.1.md

Using **`fsidx`** is a 3 step process: (1) Configure, (1) Update, (3) Locate.

### 1. Configure

You need to manually create a configuration file, ideally at **`~/.fsidx/fsidx.toml`**. The format of the configuration file is described in the [fsidx.toml(5)] man page. The [TOML] configuration file format is used. The only mandatory configuration is a folder list in table index:

```toml
[index]
folder = [
    "~/Documents",
    "/Volumes/Music"
]
```

[fsidx.toml(5)]:https://github.com/jerbs/fsidx/blob/master/doc/fsidx.toml.5.md
[TOML]:https://toml.io/

### 2. Update

For each configured folder a database file is created by running the **`update`** subcommand:

```shell
$ fsidx update
```

The database files store all pathname and file sizes below the configured folders. If a folder disappears, e.g. when a folder is not mounted anymore, then the update command will keep the previously created database file as it is. 

### 3. Locate

The **`locate`** subcommand quieries the database files for all configured folders. Images from the last hiking trip may for example be located with:

```shell
$ fsidx locate 2023 hiking "*.jpg"
```

Using the shell mode of **`fsidx`** avoids the necessity to quote some query patterns which would have been expanded by the normal shell of the OS. Using **`fsidx shell`** also simplifies opening the located files with the respective default applications:

```shell
$ fsidx shell
```

At the shell prompt you can directly enter the query:

```shell
> 2023 hiking *.jpg
```

This prints a list of indexed query results. The open the first 10 files of the last query results:

```shell
> 1.-10.
```

All files of the query result are opened with:

```shell
> \o *
```

A glob pattern can also be used to open selected files from the last query results:

```shell
> \o **
```

## Installation

The program supports Linux and MacOS. 

To install `fsidx` into the home directory:

```shell
$ make install prefix=$HOME
```

To install `fsidx` into `/usr/local`:

```shell
$ INSTALL="sudo install" make install
```

Setting the environment variable `INSTALL` to `sudo install` still executes all build and test tools with the normal user and only the `install` command with root permissions.

## Alternative Tools

- The [find(1)] utility recursively descends the directory tree and prints pathnames matching the search criteria. *find* can not only search for matching file names, but for any file system level metadata. For repeated queries *find* is slower than just searching in the database files. *fsidx* queries are also more intuitive. *find* usually only provides glob patterns. Searching files on media curently not mounted is also not possible.

- The [locate(1)] program also searches a database for all pathnames which match a specified pattern. In that sense *locate* is a very similar program. *fsidx* provides additional features and more intuitve queries.

[find(1)]:https://linux.die.net/man/1/find
[locate(1)]:https://linux.die.net/man/1/locate

## Changelog

[view changelog](https://github.com/jerbs/fsidx/blob/master/CHANGELOG.md)

## License

This project is licensed under the [MIT license].

[MIT license]: https://github.com/tokio-rs/tokio/blob/master/LICENSE

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in fsidx by you, shall be licensed as MIT, without any additional terms or conditions.

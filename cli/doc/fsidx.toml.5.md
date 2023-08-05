% FSIDX.TOML(5) fsidx.toml 0.1.0
% Joachim Erbs
% August 5, 2023

# NAME
fsidx.toml -- find filenames quickly

# DESCRIPTION
The **fsidx.toml** file configures the **fsidx** tool. The user or administrator needs to manually create this file. It is mandatory to define a list of top level folders for which pathname databases are created. Optionally, the file may define alternative defaults for the **locate** subcommand.

**TOML** is a file format for configuration files. The name **TOML** is an acronym for "**Tom's Obvious, Minimal Language**". A specification is available at *https://toml.io/en/v1.0.0*.

The **fsidx.toml** file may contain 2 tables with key value pairs.

## index
The index table defines the folders for which database files are created and where the database files are stored.

**folder**
:   The folder key is mandatory. The value is an array of folders. **fsidx update** scans each folder and creates a database file with a pathname index.

**dbpath**
:   The dbpath key is optional. Database files are stored in this folder. By default, the database files are stored in the same folder as fsidx.toml.

## locate
The locate table is optional and may define alternative defaults for the **fsidx locate** command.

**case_sensitive**
:   Allowed values are **true** and **false** (default).

**order**
:   Allowed values are **"any_order"** (default) and **"same_order"**.

**what**
:   Allowed values are **"whole_path"** (default) and **"last_element"**.

**smart_spaces**
:   Allowed values are **true** (default) and **false**.

**word_boundaries**
:   Allowed values are **true** and **false** (default).

**literal_separator**
:   Allowed values are **true** and **false** (default).

**mode**
:   Allowed values are **auto** (default), **plain** and **glob**.

Refer to the **fsidx(1)** man page for a detailed description of the locate options.

# EXAMPLE

**fsidx.toml** with default locate options:

    [index]
    folder = [
        "~/Music",
        "/Volumes/Music"
    ]

    [locate]
    case_sensitive = false
    order = "any_order"
    what = "whole_path"
    smart_spaces = true
    word_boundaries = false
    literal_separator = false
    mode = "auto"

# SEE ALSO
fsidx(1)

# COPYRIGHT
Copyright ©  2023 Joachim Erbs

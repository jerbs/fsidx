% FSIDX(1) fsidx 0.1.0
% Joachim Erbs
% August 5, 2023

# NAME
fsidx - find filenames quickly

# SYNOPSIS
**fsidx** [*MAIN-OPTIONS*] [*subcommand*] [*SUBCOMMAND-OPTIONS*]\
**fsidx update**\
**fsidx locate** [*pattern*]\
**fsidx shell**

# DESCRIPTION
**fsidx** allows fast path name searching with the help of database files. In a first step a configured set of file system trees is scanned to sore path names and file sizes in a database. In a second much faster step these database files are searched using various patterns. Beside glob patterns other more intuitive search queries are available.

**fsidx** has the following main command options:

**`-c`**, **`--config-file`**
:   Specify the configuration file to use.  (See **fsidx.toml**(5).)

**`-h`**, **`--help`**
:   Display available options and subcommands. Using the short option multiple times displays different documentation: Usage information (**`-h`**), fsidx man page (**`-hh`** or **`--help`**), fsidx.toml man page (**`-hhh`**) describing the configuration file format.

**`-v`**, **`--verbose`**
:   Verbose mode.q

**`-V`**, **`--version`**
:   Display the software version.

**fsidx** has subcommands:

## UPDATE
The **update** subcommand scans folders defined in the configuration file and stores path names and file sizes in database files. If the top level folder does not exist, then an already existing database file is not modified. This is useful to create indices for removable media.

## LOCATE
The **locate** subcommand uses a search query to find matching path names in the database files created by the **update** subcommand. A search query is an arbitrarily long sequence of plain text, glob patterns and options in any order. Options have an impact on all subsequent elements of a query.

**plain text**
:   Plain text must occur somewhere in the path name (default) or in the last path element. Options may add restrictions, like case-sensitivity or order requirements. 

**glob pattern**
:   Glob patterns are either applied to the complete path (default) or to the last path element. Multiple glob patterns are accumulative. A search result must match with a single glob pattern only.

    Standard Unix-style glob syntax is supported:

    - **`?`** matches any single character. (If the literal_separator option is enabled, then **`?`** can never match a path separator.)

    - **`*`** matches zero or more characters. (If the literal_separator option is enabled, then **`*`** can never match a path separator.)

    - **`**`** recursively matches directories but are only legal in three situations. First, if the glob starts with **`**/`**, then it matches all directories. For example, **`**/foo`** matches **`foo`** and **`bar/foo`** but not **`foo/bar`**. Secondly, if the glob ends with **`/**`**, then it matches all sub-entries. For example, **`foo/**`** matches **`foo/a`** and **`foo/a/b`**, but not **`foo`**. Thirdly, if the glob contains **`/**/`** anywhere within the pattern, then it matches zero or more directories. Using **`**`** anywhere else is illegal (N.B. the glob **`**`** is allowed and means “match everything”).

    - **`{`**a**`,`**b**`}`** matches a or b where a and b are arbitrary glob patterns. (N.B. Nesting **`{`**...**`}`** is not currently allowed.)

    - **`[`**ab**`]`** matches a or b where a and b are characters. Use **`[!`**ab**`]`** to match any character except for a and b.

    - Metacharacters such as **`*`** and **`?`** can be escaped with character class notation. e.g., **`[*]`** matches **`*`**.

    - When backslash escapes are enabled, a backslash (**`\`**) will escape all meta characters in a glob. If it precedes a non-meta character, then the slash is ignored. A **`\\`** will match a literal **`\`**.

**Options**
:   Single character short options start with a single leading dash. Long options start with two leading dashs. Short options with a single leading slash can be combined. 

**locate** supports the following options:

**`-c`**, **`--case-sensitive`**
:   Case-sensitive matching for plain text and glob patterns.

**`-i`**, **`-case-insensitive`** (default)
:   Case-insensitive matching for plain text and glob patterns.

**`-a`**, **`--any-order`** (default)
:   Searching for subsequent plain text elements always starts at the beginning. Esentially, this means that plain text elements may appear in any order in the path name.

**`-o`**, **`--same-order`**
:   Searching for subsequent plain text elements always starts after the previous match. Esentially, this means that plain text elements must occurr in the same order in the path name.

**`-w`**, **`--whole-path`** (default)
:   Plain text and glob patterns are applied on the while path name starting from root.

**`-l`**, **`--last-element`**
:   Plain text and glob patterns are applied on the last element of the path name only, i.e. on the file name or directory name without any parent directory names.

**`-s`**. **`--smart-spaces`** (default) 
:   Spaces in quoted plain text do match with any white space, minus characters, underscore characters or with no character at all. Instead of quoted text it is also possible to use CamelCase to create an equivalent search query.

**`-S`**, **`--no-smart-spaces`**
:   Spaces in quoted plain text are handled as every other character. Also no special handling for CamelCase query text.

**`-b`**, **`--word-boundary`**
:   Enables that tokens must start and end on word boundaries. Letters and numbers are evaluated. If a token starts/ends with a letter the preceeding/succeeding character in a match must not be a letter. The same is true for numbers. A number next to a letter is considered as a word boundary. An upper case letter following on a lower case letter is considered as a word boundary.

**`-B`**, **`--no-word-boundary`**
:   Disables matching on word boundaries only.

**`--ls`**, **`--literal-separator`**
:   In glob patterns wildcards (*) do not match path separators (/).

**`--nls`**, **`--no_literal-separator`** (default)
:   In glob patterns wildcards (*) match path separators (/).

**`-0`**, **`--auto`** (default)
:   Autodetection if an element is a plain text or a glob pattern. 

**`-1`**, **`--plain`**
:   All none option elements are handled as plain text.

**`-2`**, **`--glob`**
:   All none option elements are handled as glob patterns.


## SHELL

The **shell** subcommand enters the interactive mode which provides an own shell prompt. Entering search queries in the applications own shell avoids the necessity to quote globs in order to avoid expansion by the Unix shell used to invoke **fsidx**.

Nevertheless the **fsidx** shell also provides quoting and escaping to support entering special characters. Quoting is done with a pair of double quotation marks (\"...\"). Within the quotes escape sequences are supported to enter special characters: tab (\\t), new line (\\n), carriage return (\\r), double quotes (\\") and backslash (\\\\). Outside of quotes a backslash has no special meaning. Without quotes tokens (plain text,glob patterns, options)  are separated by white spaces. Quotes allow to enter tokens containing white spaces.

Most text entered at the **fsidx** shell prompt is handled in the same way as parameters which are passed to the **locate** subcommand. Read the **LOCATE** section for detailed information about how to enter search queries.

In addition to search queries the **fsidx** shell accepts backslash commands:

**`\q`**
:    The **quit** command terminates the application.

**`\o`**
:    The **open** command opens files and directories related to the last search query findings with the respective default applications. See below for more details.

**`\u`**
:    The **update** command scans folders defined in the configuration file and updates the database files. It is the same as the **UPDATE** subcommand.

**`\h`**
:    The **help** command prints a cheatsheet with commands available in the **fsidx** shell. 

The open command **`\o`** accepts the following arguments:

**`nnn.`**
:   nnn is any of the indicees printed with the last query results. An arbitrary number of indicees can be references. The referenced file or directory is opened with the default application.

**`nnn.-mmm.`**
:   nnn and mmm are are indicees printed with the last query results. All files and directories in the range are opened with their default applications.

**`glob`**
:   glob is any glob pattern. The glob pattern is applied on the results of the last query. All matching files and directories are opened with their default applications.

    In an open glob the asterisk (*) always matches with path separators. The glob is also always case-insensitive.

**`nnn./path/glob`**
:   glob is any glob pattern. The glob pattern is prefixed with the pathname of the nnn-th result of the last search query. In addition a relative path can also be defined. The resulting path is normalized, i.e. for every **`..`** the corresponding path is removed. The resulting glob pattern is applied on the results of the last query. All matching files and directories are opened with their default applications.

For all variants of the open command, except **`glob`**, the `\o` can be omitted. For the glob only variant the `\o` is required to distinguish it from a locate query.

For long options completions (tab) and hints (right cursor) are provided.

## HELP

The **help** subcommand displays available options and subcommands.

# ENVIRONMENT

- HOME
- FSIDX_CONFIG_FILE

# FILES
**fsidx.toml**
:   The search order for the fsidx.toml configuration file is:

    `1.` Value of the **`--config_file`** command line option.\
    `2.` **`$FSIDX_CONFIG_FILE`** environment variable\
    `3.` **`$HOME/.fsidx/fsidx.toml`**\
    `4.` **`/etc/fsidx/fsidx.toml`**

**`*.fsdb`**
:   For each folder specified in the **fsidx.toml** file a database file is created. The base name of the file is derived from the folder path by replacing the path separator characters with underscores.

    By default the database files are stored in the same folder the configuration file was read from. The configuration file can specify an alternative folder.

# EXAMPLES

Some **fsidx** shell command examples:

## LOCATE COMMAND EXAMPLES

**`Anne Miller`**
:   This locates all path names which contain both character sequences in any order and in any case. "Mike MILLER and SuzannA Harris" would also be included in the results.

**`-cls Anne Miller`**
:   Here three options are set: case_sensitive (-c), last_element (-l) and same_order (-s). Now Anna no longer matches Suzanna. Nevertheless, the following examples would still match: "Anne Scott and Mike Miller" or "Anne-Maria Miller".

**`"Anne Miller"`**
:   Quoting the name with double quotes removes some unexpected results. Smart spaces are enabled by default. When enabled spaces do also match with other characters commonly used instead of spaces in file names. The locate results would include: "Anne Miller", "Anne-Miller", "Anne_Miller", "AnneMiller" and also other case variants. Nevertheless, the results may again contain "Suzanna Miller" for example, since matching is case-insensitive by default.

**`-b "Anne Miller"`**
:   Here the word boundary feature is enabled in addition. Now the first and the last letter of token "Anna Miller" must be at a word boundary. No further letter is allowed in front of the 'A' and after the 'r'. This excludes "Suzanna Miller" from the results.

**`AnneMiller`**
:   When smart spaces are enabled, then this is equivalent to `"Anne Miller"`. Camel case query texts result in smart space queries.

**`time -l out -c -w /Jazz/ *.flac`**
:   This query searches for flac-files in or below a 'Jazz' folder. The character sequence 'time' must occur somewhere in the pathname. The character sequence 'out' must occur in the file name, because the option last_element (-l) is activated. With option whole_path (-w) this is disabled again and '/Jazz/' is searched in the complete pathname.

**`summer sun *.jpg *.mp4`**
:   All jpg- and mp4-files having the character sequences 'summer' and 'sun' in the pathname are located.

**`*summer*sun*{jpg,mp4}`**
:   This locates jpg- and mp4-files with 'summer' and 'sun' in the pathname. Here 'summer' must appear before 'sun'.

**`--ls /**/Downloads/**/*.mp4`**
:   In this example the glob pattern option literal_separator (--ls) is enabled. '*' no longer matches the path separator '/'. '**' matches directories recursively, including the current directory (.). This query locates any mp4-file in or below any Downloads folder.

**`*20[0-9][0-9]*`**
:   This locates all pathnames containing any number in the range from 2000 to 2099.

Pitfalls:

**`"`**
:   This results in a "Missing closing quote" error. Use **`"\""`** to find all pathnames containing double quotes.

**`[0]`**
:   With default options this will never return any results. The expression is detected as a glob pattern due to the brackets. Glob pattern must match the whole path by default. Since all pathnames are absolute, i.e. starting with a backslash, nothing will match. Explicitly switch to plain text mode in order to find all pathnames containing brackets. E.g **`--plain [0]`**.

**`?`**
:   With default options this will never return any results. The expression is again detected as a glob pattern. In this case due to the question mark. It will never match an absolute pathname starting with a slash. **`-l ?`** finds all single character file names. **`--plain ?`** finds all pahnames containing a question mark.

**`*`**
:   By default the asterisk is detected as a glob expression. As a result all database entries are printed. Use **`--plain *`** to get all pathnames containing the asterisk character.

## OPEN COMMAND EXAMPLES

**`\o *`**
:   Opens all files of the last search query with the respective default application.

**`\o 23.`**
:   Opens the 23th result of the last search query.

**`23.`**
:   Everything that starts with a number followed by a dot is interpreted as an open command. The `\o` is optional. This also opens the 23th result of the last search query.

**`4.-7. 9.`**
:   Opens the 4th, 5th, 6th, 7th and 9th result.

**`1./*.jpg`**
:   Assuming that the first result is a directory, this opens all jpg-files in that directory, which where part of the last query result.

**`\o 2./../*.jpg`**
:   Assuming that the second result is a file, this opens all jpg-files in the same directory, which where part of the last query result.

**`\o *.jpg *.flac`**
:   Opens all jpg- and flac-files part of the last search result.

# EXIT VALUES

**0**
:   Success

**1**
:   Invalid option

# SEE ALSO
fsidx.toml(5), locate(1)

# COPYRIGHT
Copyright ©  2023 Joachim Erbs

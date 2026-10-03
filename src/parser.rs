use std::{fs, io, path::PathBuf};

use globset::GlobSetBuilder;

use crate::config::ParserConfig;

pub(crate) struct ParsedTitles {
    pub(crate) title: String,
    pub(crate) files: Vec<PathBuf>,
}

pub(crate) struct ParseOutput {
    pub(crate) titles: Vec<ParsedTitles>,
    pub(crate) errors: Vec<Box<dyn std::error::Error>>,
}

pub(crate) fn parse(parser: &ParserConfig) -> Result<ParseOutput, Box<dyn std::error::Error>> {
    if parser.directories.is_empty() {
        return Err("parser has no directories configured".into());
    }

    let mut builder = GlobSetBuilder::new();
    for pattern in &parser.globs {
        builder.add(globset::Glob::new(pattern)?);
    }
    let globs = builder.build()?;

    let mut output = ParseOutput {
        titles: Vec::new(),
        errors: Vec::new(),
    };

    for directory in &parser.directories {
        match fs::metadata(directory) {
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => {
                output.errors.push(
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("'{}' is not a directory", directory.display()),
                    )
                    .into(),
                );
                continue;
            }
            Err(error) => {
                output.errors.push(
                    io::Error::new(
                        error.kind(),
                        format!("could not inspect '{}': {error}", directory.display()),
                    )
                    .into(),
                );
                continue;
            }
        }

        for title_entry in walkdir::WalkDir::new(directory)
            .min_depth(1)
            .max_depth(1)
            .follow_links(true)
        {
            let title_entry = match title_entry {
                Ok(entry) => entry,
                Err(error) => {
                    output.errors.push(error.into());
                    continue;
                }
            };

            if title_entry.file_type().is_dir() {
                let mut parsed_title = ParsedTitles {
                    title: title_entry.file_name().to_string_lossy().into_owned(),
                    files: Vec::new(),
                };

                for file_entry in walkdir::WalkDir::new(title_entry.path())
                    .max_depth(100)
                    .follow_links(true)
                {
                    let file_entry = match file_entry {
                        Ok(file_entry) => file_entry,
                        Err(error) => {
                            output.errors.push(error.into());
                            continue;
                        }
                    };

                    if file_entry.file_type().is_file()
                        && file_entry
                            .path()
                            .file_name()
                            .is_some_and(|name| globs.is_match(std::path::Path::new(name)))
                    {
                        parsed_title.files.push(file_entry.into_path());
                    }
                }

                if !parsed_title.files.is_empty() {
                    output.titles.push(parsed_title);
                }
            }
        }
    }

    Ok(output)
}

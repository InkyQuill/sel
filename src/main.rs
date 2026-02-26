//! `sel` — Select Slices from Text Files

use clap::Parser;
use sel::cli::Cli;
use sel::selector::Selector;
use std::process;

fn main() {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Validate arguments
    if let Err(e) = cli.validate() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }

    // Run the main logic
    if let Err(e) = run(cli) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn run(cli: Cli) -> sel::Result<()> {
    use sel::selector::Selector;

    // Get files from CLI
    let files = cli.get_files();
    if files.is_empty() {
        return Err(sel::SelError::Message("No input files specified".to_string()));
    }

    // Parse selector if provided (not in regex mode)
    let selector = if cli.regex.is_some() {
        sel::selector::Selector::All
    } else {
        match cli.get_selector() {
            Some(s) => Selector::parse(&s)?,
            None => Selector::All,
        }
    };

    let color_mode = cli.color_mode();
    let show_filename = cli.with_filename || files.len() > 1;

    // Process each file
    for file_path in &files {
        // For single file or non-regex mode, show filename only if requested
        let filename = if show_filename {
            Some(file_path.to_string_lossy().to_string())
        } else {
            None
        };

        // Open and process the file
        let file = sel::reader::open_file(file_path)?;
        let mut reader = sel::reader::LineReader::new(file);

        // Create output formatter
        let mut formatter = sel::output::OutputFormatter::new(
            std::io::stdout(),
            !cli.no_line_numbers,
            show_filename,
            filename,
            color_mode,
        );

        // Process based on mode
        if let Some(pattern) = &cli.regex {
            // Regex mode
            process_regex(&mut reader, &mut formatter, pattern, &cli)?;
        } else {
            // Selector mode
            process_selector(&mut reader, &mut formatter, &selector, &cli)?;
        }

        formatter.flush()?;
    }

    Ok(())
}

/// Process file in selector mode.
fn process_selector<R: std::io::Read, W: std::io::Write>(
    reader: &mut sel::reader::LineReader<R>,
    formatter: &mut sel::output::OutputFormatter<W>,
    selector: &sel::selector::Selector,
    cli: &Cli,
) -> sel::Result<()> {

    match selector {
        Selector::All => {
            // Output all lines with line numbers
            while let Some((line_no, line)) = reader.read_line()? {
                formatter.write_line(line_no, &line)?;
            }
        }
        Selector::LineNumbers(specs) => {
            // Normalize selector (merge ranges, remove duplicates)
            let normalized = selector.normalize();
            let normalized_specs = match &normalized {
                Selector::LineNumbers(s) => s,
                _ => specs,
            };

            if let Some(context) = cli.context {
                process_with_context(reader, formatter, normalized_specs, context)?;
            } else {
                process_simple(reader, formatter, normalized_specs)?;
            }
        }
        Selector::Positions(positions) => {
            if cli.context.is_some() || cli.char_context.is_some() {
                process_positions_with_context(reader, formatter, positions, cli)?;
            } else {
                // For positions without context, just output the full lines
                process_positions_simple(reader, formatter, positions)?;
            }
        }
    }

    Ok(())
}

/// Process file in regex mode.
fn process_regex<R: std::io::Read, W: std::io::Write>(
    reader: &mut sel::reader::LineReader<R>,
    formatter: &mut sel::output::OutputFormatter<W>,
    pattern: &str,
    cli: &Cli,
) -> sel::Result<()> {
    use regex::Regex;

    let regex = Regex::new(pattern).map_err(|e| sel::SelError::InvalidRegex(e.to_string()))?;

    // Check if we need character context (fragments with pointers)
    let use_char_context = cli.char_context.is_some();
    let char_context = cli.char_context.unwrap_or(0);

    // Check if we need line context
    let use_line_context = cli.context.is_some();
    let line_context = cli.context.unwrap_or(0);

    if use_char_context {
        // Process with character context (fragments with pointers)
        process_regex_char_context(reader, formatter, &regex, char_context, cli)?;
    } else if use_line_context {
        // Process with line context
        process_regex_line_context(reader, formatter, &regex, line_context, cli)?;
    } else {
        // Simple processing with match highlighting
        process_regex_simple(reader, formatter, &regex, cli)?;
    }

    Ok(())
}

/// Process regex with simple output (just matching lines).
fn process_regex_simple<R: std::io::Read, W: std::io::Write>(
    reader: &mut sel::reader::LineReader<R>,
    formatter: &mut sel::output::OutputFormatter<W>,
    regex: &regex::Regex,
    _cli: &Cli,
) -> sel::Result<()> {
    while let Some((line_no, line)) = reader.read_line()? {
        if let Some(matches) = find_matches(regex, &line)
            && !matches.is_empty()
        {
            formatter.write_line_with_matches(line_no, &line, &matches)?;
        }
    }
    Ok(())
}

/// Process regex with line context (show surrounding lines).
fn process_regex_line_context<R: std::io::Read, W: std::io::Write>(
    reader: &mut sel::reader::LineReader<R>,
    formatter: &mut sel::output::OutputFormatter<W>,
    regex: &regex::Regex,
    context: usize,
    _cli: &Cli,
) -> sel::Result<()> {
    use std::collections::BTreeSet;

    // Read all lines and find matching ones
    let mut all_lines: Vec<(usize, String)> = Vec::new();
    let mut target_lines: BTreeSet<usize> = BTreeSet::new();

    while let Some((line_no, line)) = reader.read_line()? {
        if let Some(matches) = find_matches(regex, &line)
            && !matches.is_empty()
        {
            target_lines.insert(line_no);
        }
        all_lines.push((line_no, line));
    }

    // Output lines with context
    for (line_no, line) in &all_lines {
        let is_target = target_lines.contains(line_no);

        // Check if this line should be shown (target or within context)
        let should_show = is_target
            || target_lines.iter().any(|&target| {
                line_no.abs_diff(target) <= context
            });

        if should_show {
            if is_target {
                if let Some(matches) = find_matches(regex, line) {
                    formatter.write_target_line_with_matches(*line_no, line, &matches)?;
                }
            } else {
                formatter.write_context_line(*line_no, line)?;
            }
        }
    }

    Ok(())
}

/// Process regex with character context (fragments with pointers).
fn process_regex_char_context<R: std::io::Read, W: std::io::Write>(
    reader: &mut sel::reader::LineReader<R>,
    formatter: &mut sel::output::OutputFormatter<W>,
    regex: &regex::Regex,
    char_context: usize,
    cli: &Cli,
) -> sel::Result<()> {
    let line_context = cli.context.unwrap_or(0);

    if line_context > 0 {
        // With line context + char context
        process_regex_both_contexts(reader, formatter, regex, char_context, line_context, cli)?;
    } else {
        // Just char context (fragments with pointers)
        while let Some((line_no, line)) = reader.read_line()? {
            if let Some(matches) = find_matches(regex, &line)
                && !matches.is_empty()
            {
                // Output each match as a fragment
                for m in matches {
                    // Create fragment centered on the match
                    let match_center = m.start + (m.end - m.start) / 2;
                    let fragment = sel::output::Fragment::new(&line, match_center + 1, char_context);

                    // Calculate where the match is within the fragment
                    let fragment_start = fragment.start_column.saturating_sub(1);
                    let match_start_in_fragment = m.start.saturating_sub(fragment_start);
                    let match_end_in_fragment = m.end.saturating_sub(fragment_start);

                    // Clamp to fragment bounds
                    let frag_len = fragment.content.len();
                    let start = match_start_in_fragment.min(frag_len);
                    let end = match_end_in_fragment.min(frag_len);

                    formatter.write_fragment_with_match(
                        line_no,
                        &fragment.content,
                        start..end,
                    )?;
                }
            }
        }
    }

    Ok(())
}

/// Process regex with both line and character context.
fn process_regex_both_contexts<R: std::io::Read, W: std::io::Write>(
    reader: &mut sel::reader::LineReader<R>,
    formatter: &mut sel::output::OutputFormatter<W>,
    regex: &regex::Regex,
    char_context: usize,
    line_context: usize,
    _cli: &Cli,
) -> sel::Result<()> {
    use std::collections::BTreeSet;

    // Read all lines and find matching ones
    let mut all_lines: Vec<(usize, String)> = Vec::new();
    let mut target_lines: BTreeSet<usize> = BTreeSet::new();
    let mut line_matches: std::collections::HashMap<usize, Vec<std::ops::Range<usize>>> =
        std::collections::HashMap::new();

    while let Some((line_no, line)) = reader.read_line()? {
        if let Some(matches) = find_matches(regex, &line)
            && !matches.is_empty()
        {
            target_lines.insert(line_no);
            line_matches.insert(line_no, matches);
        }
        all_lines.push((line_no, line));
    }

    // Output lines with context
    for (line_no, line) in &all_lines {
        let is_target = target_lines.contains(line_no);

        let should_show = is_target
            || target_lines.iter().any(|&target| {
                line_no.abs_diff(target) <= line_context
            });

        if should_show {
            if is_target {
                // For target lines, show fragments for each match
                if let Some(matches) = line_matches.get(line_no) {
                    for m in matches {
                        let match_center = m.start + (m.end - m.start) / 2;
                        let fragment =
                            sel::output::Fragment::new(line, match_center + 1, char_context);

                        let fragment_start = fragment.start_column.saturating_sub(1);
                        let match_start_in_fragment = m.start.saturating_sub(fragment_start);
                        let match_end_in_fragment = m.end.saturating_sub(fragment_start);

                        let frag_len = fragment.content.len();
                        let start = match_start_in_fragment.min(frag_len);
                        let end = match_end_in_fragment.min(frag_len);

                        formatter.write_fragment_with_match(
                            *line_no,
                            &fragment.content,
                            start..end,
                        )?;
                    }
                }
            } else {
                // Context lines - show as is
                formatter.write_context_line(*line_no, line)?;
            }
        }
    }

    Ok(())
}

/// Find all matches of a regex in a string.
fn find_matches(regex: &regex::Regex, text: &str) -> Option<Vec<std::ops::Range<usize>>> {
    let matches: Vec<std::ops::Range<usize>> = regex
        .find_iter(text)
        .map(|m| m.start()..m.end())
        .collect();

    if matches.is_empty() {
        None
    } else {
        Some(matches)
    }
}

/// Simple processing without context.
/// Optimized to skip lines outside any range using binary search.
fn process_simple<R: std::io::Read, W: std::io::Write>(
    reader: &mut sel::reader::LineReader<R>,
    formatter: &mut sel::output::OutputFormatter<W>,
    specs: &[sel::selector::LineSpec],
) -> sel::Result<()> {
    // Specs are already normalized (sorted, merged) from selector.normalize()
    // Use binary search to quickly find if a line is in any spec
    while let Some((line_no, line)) = reader.read_line()? {
        // Binary search for the first spec that could contain this line
        let idx = specs.partition_point(|spec| spec.start() < line_no);

        // Check if this spec or any previous spec contains the line
        let matches = (idx < specs.len() && specs[idx].contains(line_no))
            || (idx > 0 && specs[idx - 1].contains(line_no));

        if matches {
            formatter.write_line(line_no, &line)?;
        }
    }

    Ok(())
}

/// Process with line context.
fn process_with_context<R: std::io::Read, W: std::io::Write>(
    reader: &mut sel::reader::LineReader<R>,
    formatter: &mut sel::output::OutputFormatter<W>,
    specs: &[sel::selector::LineSpec],
    context: usize,
) -> sel::Result<()> {
    use sel::reader::{ContextBuffer, ContextRange, merge_context_ranges};
    use std::collections::BTreeSet;

    // Collect all target line numbers
    let mut target_lines = BTreeSet::new();
    for spec in specs {
        match spec {
            sel::selector::LineSpec::Single(n) => {
                target_lines.insert(*n);
            }
            sel::selector::LineSpec::Range(start, end) => {
                for n in *start..=*end {
                    target_lines.insert(n);
                }
            }
        }
    }

    // Build context ranges around each target line
    let ranges: Vec<ContextRange> = target_lines
        .iter()
        .map(|&line| ContextRange::around(line, context))
        .collect();

    // Merge overlapping ranges
    let merged_ranges = merge_context_ranges(ranges);

    // Track lines that are targets (for marking with >)
    let targets: BTreeSet<_> = target_lines.into_iter().collect();

    // Process file in a single pass
    let mut before_buffer = ContextBuffer::new(context);
    let mut range_idx = 0;
    let mut in_context_range = false;

    while let Some((line_no, line)) = reader.read_line()? {
        // Check if we're in or entering a context range
        let is_in_range = if range_idx < merged_ranges.len() {
            let range = &merged_ranges[range_idx];
            line_no >= range.start && line_no <= range.end
        } else {
            false
        };

        // Move to next range if we've passed the current one
        while range_idx < merged_ranges.len() && line_no > merged_ranges[range_idx].end {
            range_idx += 1;
        }

        if is_in_range {
            // We're in a context range
            if !in_context_range {
                // Entering a new range - flush only buffer entries within range
                let current_range = &merged_ranges[range_idx];
                for (buf_line_no, buf_line) in before_buffer.drain() {
                    if buf_line_no >= current_range.start {
                        formatter.write_context_line(buf_line_no, &buf_line)?;
                    }
                }
                in_context_range = true;
            }

            // Check if this line is a target
            let is_target = targets.contains(&line_no);

            if is_target {
                formatter.write_target_line(line_no, &line)?;
            } else {
                formatter.write_context_line(line_no, &line)?;
            }
        } else {
            // Not in a range - just add to the "before" buffer
            before_buffer.push(line_no, line);
            in_context_range = false;
        }
    }

    Ok(())
}

/// Process positions (simple - just output the lines).
fn process_positions_simple<R: std::io::Read, W: std::io::Write>(
    reader: &mut sel::reader::LineReader<R>,
    formatter: &mut sel::output::OutputFormatter<W>,
    positions: &[sel::selector::Position],
) -> sel::Result<()> {
    use std::collections::BTreeSet;

    // Build a set of target lines for O(log n) lookup
    let target_lines: BTreeSet<usize> = positions.iter().map(|p| p.line).collect();

    while let Some((line_no, line)) = reader.read_line()? {
        if target_lines.contains(&line_no) {
            formatter.write_line(line_no, &line)?;
        }
    }

    Ok(())
}

/// Process positions with character context and/or line context.
fn process_positions_with_context<R: std::io::Read, W: std::io::Write>(
    reader: &mut sel::reader::LineReader<R>,
    formatter: &mut sel::output::OutputFormatter<W>,
    positions: &[sel::selector::Position],
    cli: &Cli,
) -> sel::Result<()> {
    use sel::reader::{ContextBuffer, ContextRange, merge_context_ranges};
    use std::collections::{BTreeMap, BTreeSet};

    let char_context = cli.char_context.unwrap_or(0);
    let line_context = cli.context.unwrap_or(0);

    // Build a map from line number to positions for O(log n) lookup
    let mut positions_by_line: BTreeMap<usize, Vec<&sel::selector::Position>> = BTreeMap::new();
    for pos in positions {
        positions_by_line
            .entry(pos.line)
            .or_default()
            .push(pos);
    }

    if line_context > 0 {
        // Process with line context
        let target_lines: BTreeSet<_> = positions_by_line.keys().cloned().collect();

        // Build context ranges around each target line
        let ranges: Vec<ContextRange> = target_lines
            .iter()
            .map(|&line| ContextRange::around(line, line_context))
            .collect();

        // Merge overlapping ranges
        let merged_ranges = merge_context_ranges(ranges);

        // Process file in a single pass
        let mut before_buffer = ContextBuffer::new(line_context);
        let mut range_idx = 0;
        let mut in_context_range = false;

        while let Some((line_no, line)) = reader.read_line()? {
            // Check if we're in or entering a context range
            let is_in_range = if range_idx < merged_ranges.len() {
                let range = &merged_ranges[range_idx];
                line_no >= range.start && line_no <= range.end
            } else {
                false
            };

            // Move to next range if we've passed the current one
            while range_idx < merged_ranges.len() && line_no > merged_ranges[range_idx].end {
                range_idx += 1;
            }

            if is_in_range {
                // We're in a context range
                if !in_context_range {
                    // Entering a new range - flush only buffer entries within range
                    let current_range = &merged_ranges[range_idx];
                    for (buf_line_no, buf_line) in before_buffer.drain() {
                        if buf_line_no >= current_range.start {
                            formatter.write_context_line(buf_line_no, &buf_line)?;
                        }
                    }
                    in_context_range = true;
                }

                // Check if this line is a target
                let is_target = target_lines.contains(&line_no);

                if is_target {
                    if char_context > 0 {
                        // Show as fragments for each position
                        if let Some(pos_list) = positions_by_line.get(&line_no) {
                            for pos in pos_list {
                                let fragment = sel::output::Fragment::new(&line, pos.column, char_context);
                                formatter.write_fragment(line_no, &fragment.content, fragment.pointer_offset())?;
                            }
                        }
                    } else {
                        formatter.write_target_line(line_no, &line)?;
                    }
                } else {
                    formatter.write_context_line(line_no, &line)?;
                }
            } else {
                // Not in a range - just add to the "before" buffer
                before_buffer.push(line_no, line);
                in_context_range = false;
            }
        }
    } else {
        // No line context - just show target lines with char context
        while let Some((line_no, line)) = reader.read_line()? {
            if let Some(pos_list) = positions_by_line.get(&line_no) {
                if char_context > 0 {
                    // Create fragment for each position on this line
                    for pos in pos_list {
                        let fragment = sel::output::Fragment::new(&line, pos.column, char_context);
                        formatter.write_fragment(line_no, &fragment.content, fragment.pointer_offset())?;
                    }
                } else {
                    formatter.write_target_line(line_no, &line)?;
                }
            }
        }
    }

    Ok(())
}

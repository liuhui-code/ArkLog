use std::ops::Range;

use crate::TextInputView;

pub(crate) fn level_suggestion(input: TextInputView<'_>) -> Option<&'static str> {
    if input.text.is_empty()
        || input.cursor != input.text.len()
        || input.selection.is_some()
        || !(input.text.starts_with("level:") || "level:".starts_with(input.text))
    {
        return None;
    }
    [
        "level:WARN",
        "level:ERROR",
        "level:INFO",
        "level:DEBUG",
        "level:VERBOSE",
        "level:FATAL",
        "level:ASSERT",
    ]
    .into_iter()
    .find_map(|candidate| {
        let prefix = candidate.get(..input.text.len())?;
        prefix
            .eq_ignore_ascii_case(input.text)
            .then_some(&candidate[input.text.len()..])
            .filter(|suffix| !suffix.is_empty())
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum LogLevel {
    Verbose,
    Debug,
    Info,
    Warning,
    Error,
    Fatal,
}

impl LogLevel {
    fn parse(field: &str) -> Option<Self> {
        match field {
            "V" => Some(Self::Verbose),
            "D" => Some(Self::Debug),
            "I" => Some(Self::Info),
            "W" => Some(Self::Warning),
            "E" => Some(Self::Error),
            "F" => Some(Self::Fatal),
            _ => None,
        }
    }
}

pub(crate) fn split_level_prefix(query: &str) -> Result<(Option<LogLevel>, &str), String> {
    let Some(rest) = query.strip_prefix("level:") else {
        return Ok((None, query));
    };
    let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let level = match rest[..end].to_ascii_uppercase().as_str() {
        "VERBOSE" => LogLevel::Verbose,
        "DEBUG" => LogLevel::Debug,
        "INFO" => LogLevel::Info,
        "WARN" => LogLevel::Warning,
        "ERROR" => LogLevel::Error,
        "FATAL" | "ASSERT" => LogLevel::Fatal,
        _ => {
            return Err(
                "Invalid log level; use VERBOSE, DEBUG, INFO, WARN, ERROR, FATAL or ASSERT".into(),
            )
        }
    };
    Ok((Some(level), rest[end..].trim_start()))
}

pub(crate) fn log_level_field(raw: &str) -> Option<(Range<usize>, LogLevel)> {
    let fields = raw
        .split_whitespace()
        .take(6)
        .map(|field| {
            let start = field.as_ptr() as usize - raw.as_ptr() as usize;
            (start..start + field.len(), field)
        })
        .collect::<Vec<_>>();
    let standard_start = usize::from(
        fields.len() >= 6
            && fields[0].1.parse::<u64>().is_ok()
            && fields[1].1.contains('-')
            && fields[2].1.contains(':'),
    );
    let candidate = if fields.len() >= standard_start + 5
        && fields[standard_start].1.contains('-')
        && fields[standard_start + 1].1.contains(':')
        && fields[standard_start + 2].1.parse::<u64>().is_ok()
        && fields[standard_start + 3].1.parse::<u64>().is_ok()
    {
        fields.get(standard_start + 4)
    } else if fields.len() >= 4
        && crate::agent_query::parse_epoch_millis(fields[0].1).is_some()
        && fields[1].1.parse::<u64>().is_ok()
        && fields[2].1.parse::<u64>().is_ok()
    {
        fields.get(3)
    } else if fields
        .first()
        .is_some_and(|(_, field)| field.parse::<u64>().is_ok())
    {
        fields.get(1)
    } else {
        None
    }?;
    LogLevel::parse(candidate.1).map(|level| (candidate.0.clone(), level))
}

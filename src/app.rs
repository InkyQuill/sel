//! Typed builder for a ready-to-run pipeline.
//!
//! Encodes the invariant that positional selectors cannot be paired with stdin.

use crate::context::Expander;
use crate::format::Formatter;
use crate::matcher::Matcher;
use crate::sink::Sink;
use crate::source::Source;

/// Type-level marker: any source.
pub trait SourceKind {}
/// Type-level marker: sources that allow positional selectors.
pub trait Seekable: SourceKind {}

pub struct Seek;
pub struct NonSeek;

impl SourceKind for Seek {}
impl SourceKind for NonSeek {}
impl Seekable for Seek {}

pub struct App<K: SourceKind> {
    pub source: Box<dyn Source>,
    pub matcher: Box<dyn Matcher>,
    pub expander: Box<dyn Expander>,
    pub formatter: Box<dyn Formatter>,
    pub sink: Box<dyn Sink>,
    _k: std::marker::PhantomData<K>,
}

/// Stage 1: pick a source.
pub struct Stage1;

impl Stage1 {
    pub fn with_seekable_source(source: Box<dyn Source>) -> Stage2<Seek> {
        Stage2 {
            source,
            _k: std::marker::PhantomData,
        }
    }
    pub fn with_nonseekable_source(source: Box<dyn Source>) -> Stage2<NonSeek> {
        Stage2 {
            source,
            _k: std::marker::PhantomData,
        }
    }
}

/// Stage 2: pick a matcher. Positional only allowed on `Seek`.
pub struct Stage2<K: SourceKind> {
    source: Box<dyn Source>,
    _k: std::marker::PhantomData<K>,
}

impl<K: SourceKind> Stage2<K> {
    pub fn with_matcher(self, matcher: Box<dyn Matcher>) -> Stage3<K> {
        Stage3 {
            source: self.source,
            matcher,
            _k: std::marker::PhantomData,
        }
    }
}

impl Stage2<Seek> {
    /// Positional matcher — only available on seekable sources.
    pub fn with_position_matcher(self, matcher: crate::matcher::PositionMatcher) -> Stage3<Seek> {
        Stage3 {
            source: self.source,
            matcher: Box::new(matcher),
            _k: std::marker::PhantomData,
        }
    }
}

pub struct Stage3<K: SourceKind> {
    source: Box<dyn Source>,
    matcher: Box<dyn Matcher>,
    _k: std::marker::PhantomData<K>,
}

impl<K: SourceKind> Stage3<K> {
    pub fn with_expander(self, expander: Box<dyn Expander>) -> Stage4<K> {
        Stage4 {
            source: self.source,
            matcher: self.matcher,
            expander,
            _k: std::marker::PhantomData,
        }
    }
}

pub struct Stage4<K: SourceKind> {
    source: Box<dyn Source>,
    matcher: Box<dyn Matcher>,
    expander: Box<dyn Expander>,
    _k: std::marker::PhantomData<K>,
}

impl<K: SourceKind> Stage4<K> {
    pub fn with_formatter(self, formatter: Box<dyn Formatter>) -> Stage5<K> {
        Stage5 {
            source: self.source,
            matcher: self.matcher,
            expander: self.expander,
            formatter,
            _k: std::marker::PhantomData,
        }
    }
}

pub struct Stage5<K: SourceKind> {
    source: Box<dyn Source>,
    matcher: Box<dyn Matcher>,
    expander: Box<dyn Expander>,
    formatter: Box<dyn Formatter>,
    _k: std::marker::PhantomData<K>,
}

impl<K: SourceKind> Stage5<K> {
    pub fn with_sink(self, sink: Box<dyn Sink>) -> App<K> {
        App {
            source: self.source,
            matcher: self.matcher,
            expander: self.expander,
            formatter: self.formatter,
            sink,
            _k: std::marker::PhantomData,
        }
    }
}

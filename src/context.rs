//! Context expander — turns hits into an emit plan, optionally including neighbors.

use crate::{Emit, Line, MatchInfo, Role};
use std::collections::VecDeque;

/// An expander consumes `(Line, MatchInfo)` pairs and produces `Emit`s.
///
/// The expander owns the line and match info until it emits them, because
/// it may need to buffer lines as context.
pub trait Expander {
    /// Feed the next line/match pair. Call `drain()` after EOF to flush remaining context.
    fn push(&mut self, line: Line, info: MatchInfo, out: &mut dyn FnMut(EmitOwned));

    /// Called once at EOF to flush any buffered trailing context.
    fn drain(&mut self, out: &mut dyn FnMut(EmitOwned));
}

/// Owned form of `Emit` — the expander hands these to the caller.
#[derive(Debug, Clone)]
pub struct EmitOwned {
    pub line: Line,
    pub role: Role,
    pub match_info: MatchInfo,
}

impl EmitOwned {
    pub fn borrow(&self) -> Emit<'_> {
        Emit {
            line: &self.line,
            role: self.role,
            match_info: &self.match_info,
        }
    }
}

/// Emits only hits, nothing else.
pub struct NoContext;

impl Expander for NoContext {
    fn push(&mut self, line: Line, info: MatchInfo, out: &mut dyn FnMut(EmitOwned)) {
        if info.hit {
            out(EmitOwned {
                line,
                role: Role::Target,
                match_info: info,
            });
        }
    }

    fn drain(&mut self, _out: &mut dyn FnMut(EmitOwned)) {}
}

/// Emits each hit plus `n` lines before and after, merging overlapping windows.
pub struct LineContext {
    n: usize,
    /// Ring buffer of the last `n` lines (oldest at front).
    before: VecDeque<(Line, MatchInfo)>,
    /// Lines remaining to emit as trailing context for a recent hit.
    trailing: usize,
    /// Highest line number already emitted (avoids duplicates on overlap).
    last_emitted: u64,
}

impl LineContext {
    pub fn new(n: usize) -> Self {
        Self {
            n,
            before: VecDeque::with_capacity(n),
            trailing: 0,
            last_emitted: 0,
        }
    }

    fn emit(&mut self, line: Line, info: MatchInfo, role: Role, out: &mut dyn FnMut(EmitOwned)) {
        if line.no <= self.last_emitted {
            return;
        }
        self.last_emitted = line.no;
        out(EmitOwned {
            line,
            role,
            match_info: info,
        });
    }
}

impl Expander for LineContext {
    fn push(&mut self, line: Line, info: MatchInfo, out: &mut dyn FnMut(EmitOwned)) {
        if info.hit {
            // Flush stored "before" lines as context.
            let buffered: Vec<_> = self.before.drain(..).collect();
            for (bl, bi) in buffered {
                self.emit(bl, bi, Role::Context, out);
            }
            let hit_line = line;
            let hit_info = info;
            self.emit(hit_line, hit_info, Role::Target, out);
            self.trailing = self.n;
        } else if self.trailing > 0 {
            self.trailing -= 1;
            self.emit(line, info, Role::Context, out);
        } else {
            // Record as potential "before" context.
            if self.n > 0 {
                if self.before.len() == self.n {
                    self.before.pop_front();
                }
                self.before.push_back((line, info));
            }
        }
    }

    fn drain(&mut self, _out: &mut dyn FnMut(EmitOwned)) {
        // Trailing lines were already emitted as they came. "Before" buffer is just dropped.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(n: u64) -> (Line, MatchInfo) {
        (
            Line::new(n, format!("line{n}").into_bytes()),
            MatchInfo {
                hit: true,
                ..Default::default()
            },
        )
    }
    fn miss(n: u64) -> (Line, MatchInfo) {
        (
            Line::new(n, format!("line{n}").into_bytes()),
            MatchInfo::default(),
        )
    }

    fn collect<E: Expander>(mut e: E, inputs: Vec<(Line, MatchInfo)>) -> Vec<(u64, Role)> {
        let mut out: Vec<(u64, Role)> = Vec::new();
        {
            let mut f = |emit: EmitOwned| out.push((emit.line.no, emit.role));
            for (l, i) in inputs {
                e.push(l, i, &mut f);
            }
            e.drain(&mut f);
        }
        out
    }

    #[test]
    fn no_context_emits_only_hits() {
        let out = collect(NoContext, vec![miss(1), hit(2), miss(3), hit(4)]);
        assert_eq!(out, vec![(2, Role::Target), (4, Role::Target)]);
    }

    #[test]
    fn line_context_emits_around_hit() {
        let out = collect(
            LineContext::new(1),
            vec![miss(1), miss(2), hit(3), miss(4), miss(5)],
        );
        assert_eq!(
            out,
            vec![(2, Role::Context), (3, Role::Target), (4, Role::Context),]
        );
    }

    #[test]
    fn overlapping_contexts_do_not_duplicate() {
        let out = collect(LineContext::new(1), vec![miss(1), hit(2), hit(3), miss(4)]);
        assert_eq!(
            out,
            vec![
                (1, Role::Context),
                (2, Role::Target),
                (3, Role::Target),
                (4, Role::Context),
            ]
        );
    }
}

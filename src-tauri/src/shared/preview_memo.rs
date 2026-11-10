//! The last Preview, remembered for the Run that confirms it. Written by the wizard's preview
//! command, read by the run command.

use crate::shared::domain::{Counts, Preview, SavedSync, SyncMode};
use crate::shared::error_messages::{self, Explanation};
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::sync::{Mutex, MutexGuard, PoisonError};

/// What a run needs from the preview it confirms.
struct Remembered {
    token: String,
    source: String,
    destination: String,
    mode: SyncMode,
    counts: Counts,
}

#[derive(Default)]
struct Memo {
    /// Previews started so far, in the order they were asked: the number of the newest one. Tokens
    /// differ even within the same process.
    started: u64,
    last: Option<Remembered>,
}

/// The last preview asked for, with the token the UI gets for it. A run takes its counts from
/// here, never from what the UI sends back, and is refused unless it brings the token of that
/// preview, for the same A, B and mode (and, when it would delete from B, the user's consent).
///
/// "Last" is the order the previews were asked in, not the order they finish in: a slow preview
/// of a share that ends after a newer one must not take its place.
#[derive(Default)]
pub struct PreviewMemo(Mutex<Memo>);

impl PreviewMemo {
    /// A preview is about to start: it is now the only one that can confirm a run, so any earlier
    /// token goes stale at once, whether this preview ends well, blocked or in an error. Returns
    /// its number, for `remember`.
    pub fn begin(&self) -> u64 {
        let mut memo = self.memo();
        memo.started += 1;
        memo.last = None;
        memo.started
    }

    /// Keeps the finished `preview` (number `started`, from `begin`) as the one a run can confirm
    /// and returns its token. `None`: a blocked preview confirms nothing, and one that a newer
    /// preview has overtaken is ignored (it changes nothing).
    pub fn remember(
        &self,
        started: u64,
        source: &str,
        destination: &str,
        mode: SyncMode,
        preview: &Preview,
    ) -> Option<String> {
        let mut memo = self.memo();
        if started != memo.started || preview.blocked.is_some() {
            return None;
        }
        let token = format!(
            "{started:x}-{:016x}",
            RandomState::new().build_hasher().finish()
        );
        memo.last = Some(Remembered {
            token: token.clone(),
            source: source.into(),
            destination: destination.into(),
            mode,
            counts: preview.counts(),
        });
        Some(token)
    }

    /// Counts of the preview this run confirms; each preview confirms one run, also when the run
    /// is then refused for what it asks (another sync, no consent). A token that is not the
    /// last preview's changes nothing.
    pub fn confirm(
        &self,
        token: Option<&str>,
        sync: &SavedSync,
        consent: bool,
    ) -> Result<Counts, Explanation> {
        let token = token
            .filter(|token| !token.is_empty())
            .ok_or_else(error_messages::run_without_preview)?;
        let remembered = self
            .memo()
            .last
            .take_if(|remembered| remembered.token == token)
            .ok_or_else(error_messages::run_with_stale_preview)?;
        if remembered.source != sync.source
            || remembered.destination != sync.destination
            || remembered.mode != sync.mode
        {
            return Err(error_messages::run_with_other_preview());
        }
        // The UI asks for the consent only when the preview shows deletions.
        if remembered.mode == SyncMode::Mirror && remembered.counts.deleted > 0 && !consent {
            return Err(error_messages::mirror_without_consent());
        }
        Ok(remembered.counts)
    }

    fn memo(&self) -> MutexGuard<'_, Memo> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::domain::{FileGroup, counts, sample_sync};

    const A: &str = r"D:\Foto";
    const B: &str = r"E:\Backup";

    fn group(count: usize) -> FileGroup {
        FileGroup {
            count,
            files: Vec::new(),
        }
    }

    fn preview_of(new: usize, updated: usize, deleted: usize) -> Preview {
        Preview {
            new: group(new),
            updated: group(updated),
            deleted: group(deleted),
            ..Preview::default()
        }
    }

    fn sync(source: &str, destination: &str, mode: SyncMode) -> SavedSync {
        SavedSync {
            source: source.into(),
            destination: destination.into(),
            mode,
            ..sample_sync()
        }
    }

    /// A memo holding a preview of A to B in `mode`, and the token of that preview.
    fn memo_with(mode: SyncMode, preview: &Preview) -> (PreviewMemo, String) {
        let memo = PreviewMemo::default();
        let started = memo.begin();
        let token = memo.remember(started, A, B, mode, preview).unwrap();
        (memo, token)
    }

    #[test]
    fn a_run_takes_its_counts_from_the_preview_its_token_belongs_to_once() {
        let (memo, token) = memo_with(SyncMode::Mirror, &preview_of(1, 0, 5));
        let mirror = sync(A, B, SyncMode::Mirror);

        assert_eq!(
            memo.confirm(Some(&token), &mirror, true),
            Ok(counts(1, 0, 5))
        );
        assert_eq!(
            memo.confirm(Some(&token), &mirror, true),
            Err(error_messages::run_with_stale_preview()),
            "one preview confirms one run"
        );

        let (memo, token) = memo_with(SyncMode::Copy, &preview_of(2, 1, 0));
        assert_eq!(
            memo.confirm(Some(&token), &sync(A, B, SyncMode::Copy), false),
            Ok(counts(2, 1, 0)),
            "copy needs no consent"
        );
    }

    #[test]
    fn a_run_without_a_token_is_refused_in_both_modes_and_keeps_the_preview() {
        for mode in [SyncMode::Copy, SyncMode::Mirror] {
            let (memo, token) = memo_with(mode, &preview_of(1, 0, 0));
            for missing in [None, Some("")] {
                assert_eq!(
                    memo.confirm(missing, &sync(A, B, mode), true),
                    Err(error_messages::run_without_preview())
                );
            }
            assert!(memo.confirm(Some(&token), &sync(A, B, mode), true).is_ok());
        }
    }

    #[test]
    fn a_run_with_a_token_nobody_gave_is_refused_and_keeps_the_preview() {
        for mode in [SyncMode::Copy, SyncMode::Mirror] {
            let (memo, token) = memo_with(mode, &preview_of(1, 0, 0));
            assert_eq!(
                memo.confirm(Some("forged"), &sync(A, B, mode), true),
                Err(error_messages::run_with_stale_preview())
            );
            assert!(memo.confirm(Some(&token), &sync(A, B, mode), true).is_ok());
        }
        let empty = PreviewMemo::default();
        assert_eq!(
            empty.confirm(Some("1-0"), &sync(A, B, SyncMode::Copy), true),
            Err(error_messages::run_with_stale_preview()),
            "before any preview"
        );
    }

    #[test]
    fn the_token_of_a_replaced_preview_is_stale_even_for_the_same_folders() {
        for mode in [SyncMode::Copy, SyncMode::Mirror] {
            let memo = PreviewMemo::default();
            let first = memo.begin();
            let old = memo
                .remember(first, A, B, mode, &preview_of(9, 9, 0))
                .unwrap();
            let second = memo.begin();
            let new = memo
                .remember(second, A, B, mode, &preview_of(1, 0, 0))
                .unwrap();
            assert_ne!(old, new);

            assert_eq!(
                memo.confirm(Some(&old), &sync(A, B, mode), true),
                Err(error_messages::run_with_stale_preview())
            );
            assert_eq!(
                memo.confirm(Some(&new), &sync(A, B, mode), true),
                Ok(counts(1, 0, 0)),
                "the newer preview is still good"
            );
        }
    }

    #[test]
    fn a_blocked_preview_has_no_token_and_makes_the_earlier_one_stale() {
        let (memo, token) = memo_with(SyncMode::Mirror, &preview_of(1, 0, 0));
        let started = memo.begin();
        let blocked = Preview {
            blocked: Some("no".into()),
            ..preview_of(1, 0, 5)
        };

        assert_eq!(
            memo.remember(started, A, B, SyncMode::Mirror, &blocked),
            None
        );
        assert_eq!(
            memo.confirm(Some(&token), &sync(A, B, SyncMode::Mirror), true),
            Err(error_messages::run_with_stale_preview())
        );
    }

    #[test]
    fn a_preview_that_fails_makes_the_earlier_token_stale() {
        let (memo, token) = memo_with(SyncMode::Mirror, &preview_of(1, 0, 0));

        // The next preview starts and then fails: `remember` is never called for it.
        memo.begin();

        assert_eq!(
            memo.confirm(Some(&token), &sync(A, B, SyncMode::Mirror), true),
            Err(error_messages::run_with_stale_preview())
        );
    }

    #[test]
    fn a_slow_preview_that_ends_after_a_newer_one_changes_nothing() {
        for mode in [SyncMode::Copy, SyncMode::Mirror] {
            let memo = PreviewMemo::default();
            let slow = memo.begin();
            let newer = memo.begin();
            let token = memo
                .remember(newer, A, B, mode, &preview_of(1, 0, 0))
                .unwrap();

            let late = memo.remember(slow, A, B, mode, &preview_of(9, 9, 0));
            let late_blocked = memo.remember(
                slow,
                A,
                B,
                mode,
                &Preview {
                    blocked: Some("no".into()),
                    ..preview_of(0, 0, 0)
                },
            );

            assert_eq!((late, late_blocked), (None, None));
            assert_eq!(
                memo.confirm(Some(&token), &sync(A, B, mode), true),
                Ok(counts(1, 0, 0)),
                "the newer preview keeps its token"
            );
        }
    }

    #[test]
    fn a_slow_preview_that_ends_before_the_newer_one_does_is_ignored_too() {
        let memo = PreviewMemo::default();
        let slow = memo.begin();
        let newer = memo.begin();

        assert_eq!(
            memo.remember(slow, A, B, SyncMode::Copy, &preview_of(9, 9, 0)),
            None
        );
        assert!(
            memo.remember(newer, A, B, SyncMode::Copy, &preview_of(1, 0, 0))
                .is_some()
        );
    }

    #[test]
    fn a_run_of_other_folders_or_another_mode_than_its_preview_is_refused_and_uses_it_up() {
        for mode in [SyncMode::Copy, SyncMode::Mirror] {
            let other = if mode == SyncMode::Copy {
                SyncMode::Mirror
            } else {
                SyncMode::Copy
            };
            for asked in [
                sync(r"D:\Altro", B, mode),
                sync(A, r"E:\Altro", mode),
                sync(A, B, other),
            ] {
                let (memo, token) = memo_with(mode, &preview_of(1, 0, 0));
                assert_eq!(
                    memo.confirm(Some(&token), &asked, true),
                    Err(error_messages::run_with_other_preview()),
                    "{} {} {:?}",
                    asked.source,
                    asked.destination,
                    asked.mode
                );
                assert_eq!(
                    memo.confirm(Some(&token), &sync(A, B, mode), true),
                    Err(error_messages::run_with_stale_preview()),
                    "refused is used up"
                );
            }
        }
    }

    #[test]
    fn a_mirror_run_that_deletes_needs_the_consent_and_one_that_deletes_nothing_does_not() {
        let mirror = sync(A, B, SyncMode::Mirror);

        let (memo, token) = memo_with(SyncMode::Mirror, &preview_of(2, 0, 1));
        assert_eq!(
            memo.confirm(Some(&token), &mirror, false),
            Err(error_messages::mirror_without_consent())
        );
        assert_eq!(
            memo.confirm(Some(&token), &mirror, true),
            Err(error_messages::run_with_stale_preview()),
            "refused is used up: the consent goes with a new preview"
        );

        let (memo, token) = memo_with(SyncMode::Mirror, &preview_of(2, 0, 1));
        assert_eq!(
            memo.confirm(Some(&token), &mirror, true),
            Ok(counts(2, 0, 1))
        );

        let (memo, token) = memo_with(SyncMode::Mirror, &preview_of(2, 1, 0));
        assert_eq!(
            memo.confirm(Some(&token), &mirror, false),
            Ok(counts(2, 1, 0)),
            "nothing to delete: nothing to consent to"
        );
    }

    #[test]
    fn a_copy_run_without_a_matching_preview_is_refused() {
        let memo = PreviewMemo::default();
        assert_eq!(
            memo.confirm(None, &sync(A, B, SyncMode::Copy), true),
            Err(error_messages::run_without_preview())
        );
    }
}

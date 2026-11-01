# Tasks

- [x] Route only the foreign `/48` of a pair that includes this house
- [x] Remove that route when the caller no longer presents the pair
- [x] Do not route a stock prefix, the local `/48`, or a pair for another house
- [x] Test the pair, the empty set, and the stock prefix
- [ ] Fresh advise after preparation
- [x] Owed by advise 2026-11-01: retain failed route removals for retry, distinguish observation failure from confirmed absence, and expose reconciliation failure; verify failed deletion followed by successful cleanup on the next sync (review finding 1).
- [x] Owed by advise 2026-11-01: bind sync's local guild to the open TUN session and exclude its own `/48`; verify an A session cannot route its own prefix via an A/B pair presented as B or accept an unrelated B/C pair, while valid foreign routing still works (review finding 2).

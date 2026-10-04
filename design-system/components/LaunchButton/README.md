The primary verb for a game; the lifecycle in one control. Rust: `reclaw_ui::deck::{launch_verb, LaunchButton}`; the table is `deck/launch.rs` and `lifecycle.ui.LaunchButton` in the contract.

Desktop swaps Play for Stop while the app runs. Deck shows Resume (A) with Stop beside it, because Resume is almost always what you want and Stop must not be one mis-press away. Stop sends a graceful quit to the whole process group; **pressing it again while Stopping force-kills** (the button then reads Force quit). A failed run shows Retry and a plain-words reason, never a bare red state.

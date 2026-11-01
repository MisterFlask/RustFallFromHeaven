//! What the game runners share (`ffh-stress`, `ffh-play soak`; std only):
//!
//! * [`check`]: cross-checks of the rules' redundant bookkeeping, run every few turns;
//! * [`panic`]: a panic hook that records the message, location and workspace frames of a panic,
//!   and [`panic::catch`] around a game;
//! * [`child`]: one child process with a timeout, its output collected;
//! * [`pool`]: N jobs over a list of games, results in list order.

pub mod check;
pub mod child;
pub mod panic;
pub mod pool;

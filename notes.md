# Notes
* `cargo run` worked out of the box
* Really cool UI aesthetic, feels like looking at a retro command center or something!
* Why do you need both `CLAUDE.md` and `AGENTS.md`?

# UI / UX
* Took a while to figure out how to load the satellites. I wish there were some loaded when the program starts up.
* Different satellite groups should have different symbols / colors to distinguish them. Becomes a mess when more than one is loaded.
* Load all option would be cool.
* I don't know how to load SpaceX data, README.md could be clearer about how to use other data sources.
* What's the difference between the "1x" and "Now" buttons?
* Distance color gradient is barely perceptible. 

# Rust Feedback
* Try turning on pedantic clippy linting. Add `#![warn(clippy::all, clippy::pedantic)]` to `src/main.rs`.
* Empty integration tests directory?
* Dependencies:
    * Consider adding `cargo-sort` as part of your formatting workflow.
    * Unused dependencies. Use `cargo-shear` to detect and fix, it's a nice tool.
* Really long compile times, I wonder if you could optimize this? 
    * The rust perf book has useful tips: https://nnethercote.github.io/perf-book/compile-times.html
    * Good related blog post that covers lots of topics: https://corrode.dev/blog/tips-for-faster-rust-compile-times/
    * Put tests in separate compilation units: https://matklad.github.io/2021/02/27/delete-cargo-integration-tests.html

# Bugs
* Release build is broken
  ```shell
  ❯ ./target/release/bevyearth
  dyld[44530]: Library not loaded: @rpath/libstd-80c57efd13e5c60f.dylib
    Referenced from: <8A17C2ED-BA92-32E9-B2B3-E8008F9366B7> /Users/jhelland/Documents/repos/bevyearth/target/release/bevyearth
    Reason: no LC_RPATH's found
  fish: Job 1, './target/release/bevyearth' terminated by signal SIGABRT (Abort)
  ```
* UI top bar disappears when you click "Time" button with no way to bring it back.

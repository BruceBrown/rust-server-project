# component_config

## The Motivation
Building upon merged_config we're going to look at configuring components. In an ideal world,
you'd be able to specify an enum where each variant represents a component and its structure.
```rust
pub struct EchoService {
    pub server: Server,
    pub max_sessions: usize,
}
pub enum ComponentConfig {
    EchoService(EchoService)
}
```
Sadly, we can't to this. Instead, we can extract the from the config and create a
`HashMap<String, ComponentConfig>` and work with that. This is how we're going to
implement component configuration.

## Test Driven Experiments
There won't be nearly as much unit testing. We'll process a set of configs and dump them out, and
ensure that features are being accumulated. Beyond that there are tests which run main providing
the environment.

try:
```
    carog run
    RUN_ENV=Development cargo run
    RUN_ENV=Testing cargo run
```


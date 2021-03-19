# merged_config

## The Motivation
Switching gears from Machine creation and instruction pipelines, we're going to take a look
at server configuration. We're going to use the config crate, coupled with serde and
serde_with to consume and merge various config files into a final configuration.

Since we will run in multiple environments, we'll want to have a config of defaults, we'll
then layer environment config, think of this as configuring for development, test, production,
stage, etc. Then, because we want to build a single server which can perform many tasks, we'll
have a server_flavor, which specifies a sub-folder, which again contains defaults and
environment configs. Finally, we'll want to override the settings with additional environment
variables. We also want to allow the config to be a JSON or TOML file.

As a last thought, config uses a replacement policy. Where a value in a config overrides a
previous value. For our server, we'd like some fields to be merged. For example, we're going
to use feature toggles to enable behavior, and those will likely be scattered across a few,
if not all of the config files.

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


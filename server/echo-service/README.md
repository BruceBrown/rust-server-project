# echo-service

## The Motivation
Building upon async_framework and component_config, we're going to introduce
networking and services.

## Instruction Sets
The NetCmd instruction set forms the interconnect between services and the network.

## Traits
The `ServerServices` trait provides a common API for managing services. While the
`ServiceStateTransiion` trait provides notification of state service state transitions.

## Services
The Echo service implements a service which handles a connection. Anything received is
sent back to the sender.

## Test Driven Experiments
We've reached the point where test driven experiments have a diminised return, as we're
running services which have a longer duration for testing.

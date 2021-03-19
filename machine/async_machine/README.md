# async_machine

## The Motivation
Building upon async_channel, we're going to introduce some traits that describe
a machine. We'll also introduce a means of building a machine from a model.
Finally, we'll hide the async sending of messages to the channel.

## Traits
The `Machine` trait has 3 methods:
* connected(), which is called once, when the machine is created;
* disconnected(), which is called once, when the machine is terminated;
* receive(), which is called whenever a message is received for the machine

## Construction
The `Machine` is constructed from a model of a machine, along with its
instruction set. For this experiment, a single instruction set will be
used: TestMessage.

Construction consists of calling the connect() or connect_unbounded()
function, which returns a tuple consisting of a machine and sender for
that machine.

See machine_adapter tests for an example.

## Test Driven Experiments
There are a bunch of unit tests. However, the most interesting are found
in daidy_chain. There a small and daisy-chain is run. This mirrors the
daisy chain which is run in the asycn_channel experiment. Chaos-Monkey
has been added. Each machine has the senders to all of the machines,
including themself; they're the  Monkeys. A message is sent to a random
Monkey, the value is incremented and sent to the next random monkey until
it reaches an inflection point, at which it starts decrementing. At 0,
a notification is sent. This is closer to similating a pipeline architecture
where you have many parallel paths and a fixed number of hops between
a message arriving at the server and it departing the server.

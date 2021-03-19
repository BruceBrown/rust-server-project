# async_machine

## The Motivation
Continuing along in the series, we're going to add generics into the machine_adapter structs. We'll also
introduce a derive macro, `MachineImpl` to simplify `Machine` construction. We'll derive it for enums,
such as our old friend TestMessage, transorming the enum into a `Machine's Instruction Set`.

There are two key point here:
* A derive macro tags an enum as an instuction set;
* A `Machine` can support mulitple instruction sets.

We're also turning this into a library, so that it can be consumed as a framework. When complete, we have
a foundation for building any machine, which can send instructions to any other machine. Each machine may
unerstant 1 or more instruction sets.

At this point, the foundation has been created. Any number of instruction sets can be created. Any number
of different machine models can be described. Each can be turned into one or more machines that can
communicate with other machines. This gives us a good jumping off point for create a pipeline server, where
machines are organized and configured. The next set of experiments will build upon this framework,
extending it to towards the goal of building a configurable server.

## Traits
A new trait will be introduced to allow the machines to send instructions to other machines. While we had
this in async_machine, we're going to extend it to support multiple instruction sets. Additionally, we'll
need to introduce an async trait to send the instructions. The downside here is that we have to do some
boxing, and that has a significant impact upon performance.

## Queues
Each `Machine` has a sender and receiver associated with it. The queue size can be unbounded, bounded
by a default, or bounded by a specified capacity.

## Test Driven Experiments
Again, there are a bunch of unit tests, there's also our old firends daisy_chain and chaos_monkey. We're
going to introduce a new instruction set. StateTable, and our friend Alice will implement it, as well as
implementing the TestMessage instruction set.


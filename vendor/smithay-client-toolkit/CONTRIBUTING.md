# Contributing

Smithay's Client ToolKit (SCTK) is open to contributions from anyone.

## Coordination

Most discussion about features and their implementations takes place on github.
If you have questions, suggestions, ideas, you can open an issue to discuss it, or add your message
in an already existing issue if it fits its scope.

If you want a more realtime discussion there is a a Matrix room dedicated to the Smithay project:
[#smithay:matrix.org](https://matrix.to/#/#smithay:matrix.org). If you don't want to use matrix, this room is
also bridged to gitter: https://gitter.im/smithay/Lobby.

## Scope & Structure

SCTK aims to provide generic building blocks to write wayland clients, abstracting away the boilerplate of the
wayland protocol while allowing direct control when wanted. As such, it is composed of several loosely-coupled
modules, which can be used independenly of each other. This given, if you want to contribute a new feature to
SCTK, please consider these design points:

- The feature should be designed it is most general form, allowing it to be used by other projects, probaby
  different from the exact use-case you have in mind.
- This new feature should not heavily depend on the other parts of SCTK if it can avoid it. As much as
  possible, SCTK users should be able to use your feature alone.

## Pull requests & commits organisation

The development branch is the `master` branch, and it should be the target of your pull requests.

In general, single-purpose pull requests are prefered. If you have two independent contributions to make,
please open two different pull requests.

On the other hand, if you have changes that could technically be separated, but really belong together (for
example a new feature, that first require some refactoring before being introduced), it is okay to ship them
in the same pull request. However, to simplify the review work (and future reference to the commit history),
these changes should be separated in different commits. This will allow the reviewers to review each commit
independently, reducing the cognitive load.

At merge time, pull requests consisting of a single commit or of a few well-scoped commits will be rebased on
master. Pull requests which have accumulated several review-addressing commits will be squashed.


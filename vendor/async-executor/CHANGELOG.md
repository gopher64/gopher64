# Version 1.6.0

- Remove the thread-local queue optimization, as it caused a number of bugs in production use cases. (#61)

# Version 1.5.4

- Fix a panic that could happen when two concurrent `run()` calls are made and the thread local task slot is left as `None`. (#55)

# Version 1.5.3

- Fix an accidental breaking change in v1.5.2, where `ex.run()` was no longer `Send`. (#50)
- Remove the unused `memchr` dependency. (#51)

# Version 1.5.2

- Add thread-local task queue optimizations, allowing new tasks to avoid using the global queue. (#37)
- Update `fastrand` to v2. (#45)

# Version 1.5.1

- Implement a better form of debug output for Executor and LocalExecutor. (#33) 

# Version 1.5.0

- Remove the dependency on the `once_cell` crate to restore the MSRV. (#29)
- Update `concurrent-queue` to v2.

# Version 1.4.1

- Remove dependency on deprecated `vec-arena`. (#23)

# Version 1.4.0

- Add `Executor::is_empty()` and `LocalExecutor::is_empty()`.

# Version 1.3.0

- Parametrize executors over a lifetime to allow spawning non-`static` futures.

# Version 1.2.0

- Update `async-task` to v4.

# Version 1.1.1

- Replace `AtomicU64` with `AtomicUsize`.

# Version 1.1.0

- Use atomics to make `Executor::run()` and `Executor::tick()` futures `Send + Sync`.

# Version 1.0.0

- Stabilize.

# Version 0.2.1

- Add `try_tick()` and `tick()` methods.

# Version 0.2.0

- Redesign the whole API.

# Version 0.1.2

- Add the `Spawner` API.

# Version 0.1.1

- Initial version

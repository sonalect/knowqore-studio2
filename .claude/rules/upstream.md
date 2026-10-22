# Upstream: the store component in Knowqore

Studio is a caller of the store component of Knowqore
(`github.com/sonalect/knowqore`, usually cloned as `../knowqore`). Its two
crates live in Knowqore's Cargo workspace:

- `knowqore-stores` (`rust/stores`): the contract, its records and calls;
- `knowqore-store-scheda` (`rust/store-scheda`): its implementation over a
  Scheda Vault.

Studio takes both by one Cargo git dependency on the Knowqore repository,
pinned to one `rev` (a tag, once Knowqore tags its releases); Cargo finds
each crate in the workspace by its name. Never the
proto (Knowqore's `design-principles.md`). The separate repositories
`knowqore-stores` and `knowqore-store-scheda` are closed.

## A change Studio needs upstream

A call or a field Studio needs and the contract lacks is made there, never
worked around here:

1. Write down what Studio needs, in the words of the contract (the
   request, the reply, the error codes), and agree it with the owner.
2. The change is made in a session in Knowqore, under its rules: the
   contract first, then the implementation, each with its own version and
   CHANGELOG.
3. Only then does Studio take the new `rev` and use the call.

Do not edit Knowqore from this session unless the owner asks.
`readiness.md` § "A plan that crosses repositories" applies: before a plan
that leans on an upstream change is called complete, write Studio's real
calls against the real API once.

## Taking a new version

Move the `rev` of both crates together: they come from one commit. Read
`rust/stores/CHANGELOG.md` and `rust/store-scheda/CHANGELOG.md` between the
two commits, and note in `CHANGELOG.md` what changes for the person who
runs Studio. Crates.io crates the store brings follow Knowqore's
`Cargo.lock` (`dependency-quarantine.md`); Cargo cannot pick another
version of a crate Knowqore pins with `=`.
